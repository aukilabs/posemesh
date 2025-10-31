use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use compute_runner_api::{ArtifactSink, ControlPlane, InputSource, LeaseEnvelope, Runner, TaskCtx};
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    dms::client::DmsClient,
    heartbeat::{progress_channel, ProgressReceiver, ProgressSender},
    poller::{jittered_delay_ms, PollerConfig},
    session::{CapabilitySelector, HeartbeatPolicy, SessionManager},
};

/// Registry mapping capability strings to runner instances.
#[derive(Default)]
pub struct RunnerRegistry {
    runners: HashMap<String, Arc<dyn Runner>>,
}

impl RunnerRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            runners: HashMap::new(),
        }
    }

    /// Register a runner by its capability. Last registration wins on duplicates.
    pub fn register<R: Runner + 'static>(mut self, runner: R) -> Self {
        let key = runner.capability().to_string();
        self.runners.insert(key, Arc::new(runner));
        self
    }

    /// Retrieve a runner by capability.
    pub fn get(&self, capability: &str) -> Option<Arc<dyn Runner>> {
        self.runners.get(capability).cloned()
    }

    /// Snapshot of registered capability strings.
    pub fn capabilities(&self) -> Vec<String> {
        let mut caps: Vec<_> = self.runners.keys().cloned().collect();
        caps.sort();
        caps
    }

    /// Dispatch task to the appropriate runner based on `lease.task.capability`.
    pub async fn run_for_lease(
        &self,
        lease: &LeaseEnvelope,
        input: &dyn InputSource,
        output: &dyn ArtifactSink,
        ctrl: &dyn ControlPlane,
        access_token: &dyn compute_runner_api::runner::AccessTokenProvider,
    ) -> std::result::Result<(), crate::errors::ExecutorError> {
        let cap = lease.task.capability.as_str();
        let runner = self
            .get(cap)
            .ok_or_else(|| crate::errors::ExecutorError::NoRunner(cap.to_string()))?;
        let ctx = TaskCtx {
            lease,
            input,
            output,
            ctrl,
            access_token,
        };
        runner
            .run(ctx)
            .await
            .map_err(|e| crate::errors::ExecutorError::Runner(e.to_string()))
    }
}

/// Run the node main loop. Networking and storage are wired in later prompts.
pub async fn run_node(cfg: crate::config::NodeConfig, runners: RunnerRegistry) -> Result<()> {
    let shutdown = CancellationToken::new();
    let signal_token = shutdown.clone();
    let signal_task = tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            signal_token.cancel();
        }
    });

    let result = run_node_with_shutdown(cfg, runners, shutdown.clone()).await;

    shutdown.cancel();
    let _ = signal_task.await;

    result
}

pub async fn run_node_with_shutdown(
    cfg: crate::config::NodeConfig,
    runners: RunnerRegistry,
    shutdown: CancellationToken,
) -> Result<()> {
    let siwe = crate::auth::SiweAfterRegistration::from_config(&cfg)?;
    info!("DDS SIWE authentication configured; waiting for DDS registration callback");
    let siwe_handle = siwe.start().await?;
    info!("DDS SIWE token manager started");

    let poll_cfg = PollerConfig {
        backoff_ms_min: cfg.poll_backoff_ms_min,
        backoff_ms_max: cfg.poll_backoff_ms_max,
    };

    loop {
        if shutdown.is_cancelled() {
            break;
        }

        // Ensure SIWE token is available before attempting DMS operations
        if let Err(err) = siwe_handle.bearer().await {
            warn!(error = %err, "Failed to obtain SIWE bearer token; backing off");
            let delay_ms = jittered_delay_ms(poll_cfg);
            tokio::select! {
                _ = shutdown.cancelled() => break,
                _ = sleep(StdDuration::from_millis(delay_ms)) => continue,
            }
        }

        let timeout = StdDuration::from_secs(cfg.request_timeout_secs);
        let dms_client = match crate::dms::client::DmsClient::new(
            cfg.dms_base_url.clone(),
            timeout,
            std::sync::Arc::new(siwe_handle.clone()),
        ) {
            Ok(client) => client,
            Err(err) => {
                warn!(error = %err, "Failed to create DMS client; backing off");
                let delay_ms = jittered_delay_ms(poll_cfg);
                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    _ = sleep(StdDuration::from_millis(delay_ms)) => continue,
                }
            }
        };

        match run_cycle_with_dms(&cfg, &dms_client, &runners).await {
            Ok(true) => {
                // Successful task execution; immediately attempt next poll.
                continue;
            }
            Ok(false) => {
                let delay_ms = jittered_delay_ms(poll_cfg);
                info!(delay_ms, "No lease available; backing off before next poll");
                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    _ = sleep(StdDuration::from_millis(delay_ms)) => {}
                }
            }
            Err(err) => {
                warn!(error = %err, "DMS cycle failed; backing off");
                let delay_ms = jittered_delay_ms(poll_cfg);
                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    _ = sleep(StdDuration::from_millis(delay_ms)) => {}
                }
            }
        }
    }

    siwe_handle.shutdown().await;
    info!("Shutdown signal received; exiting run_node loop");

    Ok(())
}

/// Build storage ports (input/output) for a given lease by constructing a TokenRef
/// from the lease's access token and delegating to storage::build_ports.
pub fn build_storage_for_lease(lease: &LeaseEnvelope) -> Result<crate::storage::Ports> {
    let token = crate::storage::TokenRef::new(lease.access_token.clone().unwrap_or_default());
    crate::storage::build_ports(lease, token)
}

/// Apply heartbeat token refresh: if HeartbeatResponse carries a new access token,
/// swap it into the provided TokenRef so subsequent storage requests use it.
pub fn apply_heartbeat_token_update(
    token: &crate::storage::TokenRef,
    hb: &crate::dms::types::HeartbeatResponse,
) {
    if let Some(new) = hb.access_token.clone() {
        token.swap(new);
    }
}

/// Merge fields from a heartbeat response into the cached lease.
pub fn merge_heartbeat_into_lease(
    lease: &mut LeaseEnvelope,
    hb: &crate::dms::types::HeartbeatResponse,
) {
    if let Some(token) = hb.access_token.clone() {
        lease.access_token = Some(token);
    }
    if let Some(expiry) = hb.access_token_expires_at {
        lease.access_token_expires_at = Some(expiry);
    }
    if let Some(expiry) = hb.lease_expires_at {
        lease.lease_expires_at = Some(expiry);
    }
    if let Some(cancel) = hb.cancel {
        lease.cancel = cancel;
    }
    if let Some(status) = hb.status.clone() {
        lease.status = Some(status);
    }
    if let Some(domain_id) = hb.domain_id {
        lease.domain_id = Some(domain_id);
    }
    if let Some(url) = hb.domain_server_url.clone() {
        lease.domain_server_url = Some(url);
    }
    if let Some(task) = hb.task.clone() {
        lease.task = task;
    } else {
        if let Some(task_id) = hb.task_id {
            lease.task.id = task_id;
        }
        if let Some(job_id) = hb.job_id {
            lease.task.job_id = Some(job_id);
        }
        if let Some(attempts) = hb.attempts {
            lease.task.attempts = Some(attempts);
        }
        if let Some(max_attempts) = hb.max_attempts {
            lease.task.max_attempts = Some(max_attempts);
        }
        if let Some(deps_remaining) = hb.deps_remaining {
            lease.task.deps_remaining = Some(deps_remaining);
        }
    }
}

/// Run a single poll→run→complete/fail cycle using DMS client and the runner registry.
/// This is a minimal integration used by tests; `run_node` wiring remains separate.
pub async fn run_cycle_with_dms(
    _cfg: &crate::config::NodeConfig,
    dms: &DmsClient,
    reg: &RunnerRegistry,
) -> Result<bool> {
    use crate::dms::types::{CompleteTaskRequest, FailTaskRequest, HeartbeatRequest};
    use serde_json::json;

    let capabilities = reg.capabilities();
    let capability = capabilities
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("no runners registered"))?;

    // Lease a task from DMS
    let mut lease = match dms.lease_by_capability(&capability).await? {
        Some(lease) => lease,
        None => {
            return Ok(false);
        }
    };
    if lease.access_token.is_none() {
        tracing::warn!(
            "Lease missing access token; storage client will fall back to legacy token flow"
        );
    }

    // Initialise session state for heartbeats and token rotation.
    let selector = CapabilitySelector::new(capabilities.clone());
    let session = SessionManager::new(selector);
    let policy = HeartbeatPolicy::default_policy();
    let mut rng = StdRng::from_entropy();
    let snapshot = session
        .start_session(&lease, Instant::now(), &policy, &mut rng)
        .await
        .map_err(|err| anyhow!("failed to initialise session: {err}"))?;
    if snapshot.cancel() {
        warn!(
            task_id = %snapshot.task_id(),
            "Lease already marked as cancelled; skipping execution"
        );
        return Ok(true);
    }

    let token_ref = crate::storage::TokenRef::new(lease.access_token.clone().unwrap_or_default());

    let heartbeat_initial = dms
        .heartbeat(
            lease.task.id,
            &HeartbeatRequest {
                progress: json!({}),
                events: json!({}),
            },
        )
        .await?;
    apply_heartbeat_token_update(&token_ref, &heartbeat_initial);
    merge_heartbeat_into_lease(&mut lease, &heartbeat_initial);
    session
        .apply_heartbeat(
            &heartbeat_initial,
            Some(json!({})),
            Instant::now(),
            &policy,
            &mut rng,
        )
        .await
        .map_err(|err| anyhow!("failed to refresh session after heartbeat: {err}"))?;

    let ports = crate::storage::build_ports(&lease, token_ref.clone())?;

    let (progress_tx, progress_rx) = progress_channel();
    let control_state = Arc::new(Mutex::new(ControlState::default()));
    {
        let mut guard = control_state.lock().await;
        guard.progress = json!({});
        guard.events = json!({});
    }

    let runner_cancel = CancellationToken::new();
    let heartbeat_shutdown = CancellationToken::new();

    let ctrl = EngineControlPlane::new(
        runner_cancel.clone(),
        progress_tx.clone(),
        control_state.clone(),
    );

    // Trigger an immediate heartbeat once the loop starts to refresh tokens.
    progress_tx.update(json!({}), json!({}));

    let heartbeat_driver = HeartbeatDriver::new(
        dms.clone(),
        HeartbeatDriverArgs {
            session: session.clone(),
            policy,
            rng,
            progress_rx,
            state: control_state.clone(),
            token_ref: token_ref.clone(),
            runner_cancel: runner_cancel.clone(),
            shutdown: heartbeat_shutdown.clone(),
            task_id: lease.task.id,
        },
    );
    let heartbeat_handle = tokio::spawn(async move { heartbeat_driver.run().await });

    let run_res = reg
        .run_for_lease(&lease, &*ports.input, &*ports.output, &ctrl, &token_ref)
        .await;

    heartbeat_shutdown.cancel();
    let heartbeat_result = match heartbeat_handle.await {
        Ok(result) => result,
        Err(err) => {
            warn!(error = %err, "heartbeat loop task failed");
            HeartbeatLoopResult::Completed
        }
    };

    match heartbeat_result {
        HeartbeatLoopResult::Completed => {}
        HeartbeatLoopResult::Cancelled => {
            info!(
                task_id = %lease.task.id,
                "Lease cancelled during execution; skipping completion"
            );
            runner_cancel.cancel();
            return Ok(true);
        }
        HeartbeatLoopResult::LostLease(err) => {
            warn!(
                task_id = %lease.task.id,
                error = %err,
                "Lease lost during heartbeat; abandoning task"
            );
            runner_cancel.cancel();
            return Ok(true);
        }
    }

    let uploaded_artifacts = ports.uploaded_artifacts();
    let artifacts_json: Vec<Value> = uploaded_artifacts
        .iter()
        .map(|artifact| {
            json!({
                "logical_path": artifact.logical_path,
                "name": artifact.name,
                "data_type": artifact.data_type,
                "id": artifact.id,
            })
        })
        .collect();
    let job_info = json!({
        "task_id": lease.task.id,
        "job_id": lease.task.job_id,
        "domain_id": lease.domain_id,
        "capability": lease.task.capability,
    });

    // Complete or fail the task depending on runner outcome.
    match run_res {
        Ok(()) => {
            let body = CompleteTaskRequest {
                outputs_index: json!({ "artifacts": artifacts_json.clone() }),
                result: json!({
                    "job": job_info,
                    "artifacts": artifacts_json,
                }),
            };
            dms.complete(lease.task.id, &body).await?;
        }
        Err(err) => {
            error!(
                task_id = %lease.task.id,
                job_id = ?lease.task.job_id,
                capability = %lease.task.capability,
                error = %err,
                debug = ?err,
                "Runner execution failed; reporting failure to DMS"
            );
            let body = FailTaskRequest {
                reason: err.to_string(),
                details: json!({
                    "job": job_info,
                    "artifacts": artifacts_json,
                }),
            };
            dms.fail(lease.task.id, &body)
                .await
                .with_context(|| format!("report fail for task {} to DMS", lease.task.id))?;
        }
    }

    Ok(true)
}

#[derive(Default)]
pub struct ControlState {
    progress: Value,
    events: Value,
}

struct EngineControlPlane {
    cancel: CancellationToken,
    progress_tx: ProgressSender,
    state: Arc<Mutex<ControlState>>,
}

impl EngineControlPlane {
    pub fn new(
        cancel: CancellationToken,
        progress_tx: ProgressSender,
        state: Arc<Mutex<ControlState>>,
    ) -> Self {
        Self {
            cancel,
            progress_tx,
            state,
        }
    }
}

#[async_trait]
impl ControlPlane for EngineControlPlane {
    async fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    async fn progress(&self, value: Value) -> Result<()> {
        let events = {
            let mut state = self.state.lock().await;
            state.progress = value.clone();
            state.events.clone()
        };
        self.progress_tx.update(value, events);
        Ok(())
    }

    async fn log_event(&self, fields: Value) -> Result<()> {
        let progress = {
            let mut state = self.state.lock().await;
            state.events = fields.clone();
            state.progress.clone()
        };
        self.progress_tx.update(progress, fields);
        Ok(())
    }
}

pub enum HeartbeatLoopResult {
    Completed,
    Cancelled,
    LostLease(anyhow::Error),
}

#[async_trait]
pub trait HeartbeatTransport: Send + Sync + Clone + 'static {
    async fn post_heartbeat(
        &self,
        task_id: Uuid,
        body: &crate::dms::types::HeartbeatRequest,
    ) -> Result<crate::dms::types::HeartbeatResponse>;
}

#[async_trait]
impl HeartbeatTransport for DmsClient {
    async fn post_heartbeat(
        &self,
        task_id: Uuid,
        body: &crate::dms::types::HeartbeatRequest,
    ) -> Result<crate::dms::types::HeartbeatResponse> {
        self.heartbeat(task_id, body).await
    }
}

pub struct HeartbeatDriverArgs {
    pub session: SessionManager,
    pub policy: HeartbeatPolicy,
    pub rng: StdRng,
    pub progress_rx: ProgressReceiver,
    pub state: Arc<Mutex<ControlState>>,
    pub token_ref: crate::storage::TokenRef,
    pub runner_cancel: CancellationToken,
    pub shutdown: CancellationToken,
    pub task_id: Uuid,
}

pub struct HeartbeatDriver<T>
where
    T: HeartbeatTransport,
{
    transport: T,
    session: SessionManager,
    policy: HeartbeatPolicy,
    rng: StdRng,
    progress_rx: ProgressReceiver,
    state: Arc<Mutex<ControlState>>,
    token_ref: crate::storage::TokenRef,
    runner_cancel: CancellationToken,
    shutdown: CancellationToken,
    task_id: Uuid,
    last_progress: Value,
}

impl<T> HeartbeatDriver<T>
where
    T: HeartbeatTransport,
{
    pub fn new(transport: T, args: HeartbeatDriverArgs) -> Self {
        Self {
            transport,
            session: args.session,
            policy: args.policy,
            rng: args.rng,
            progress_rx: args.progress_rx,
            state: args.state,
            token_ref: args.token_ref,
            runner_cancel: args.runner_cancel,
            shutdown: args.shutdown,
            task_id: args.task_id,
            last_progress: Value::default(),
        }
    }

    pub async fn run(mut self) -> HeartbeatLoopResult {
        loop {
            if self.shutdown.is_cancelled() || self.runner_cancel.is_cancelled() {
                return HeartbeatLoopResult::Completed;
            }

            let snapshot = match self.session.snapshot().await {
                Some(s) => s,
                None => return HeartbeatLoopResult::Completed,
            };

            let ttl_delay = snapshot
                .next_heartbeat_due()
                .map(|due| due.saturating_duration_since(Instant::now()));

            if let Some(delay) = ttl_delay {
                tokio::select! {
                    _ = self.shutdown.cancelled() => return HeartbeatLoopResult::Completed,
                    progress = self.progress_rx.recv() => {
                        if let Some(data) = progress {
                            if let Some(outcome) = self.handle_progress(data).await {
                                return outcome;
                            }
                        } else {
                            return HeartbeatLoopResult::Completed;
                        }
                    }
                    _ = tokio::time::sleep(delay) => {
                        if let Some(outcome) = self.handle_ttl().await {
                            return outcome;
                        }
                    }
                }
            } else {
                tokio::select! {
                    _ = self.shutdown.cancelled() => return HeartbeatLoopResult::Completed,
                    progress = self.progress_rx.recv() => {
                        if let Some(data) = progress {
                            if let Some(outcome) = self.handle_progress(data).await {
                                return outcome;
                            }
                        } else {
                            return HeartbeatLoopResult::Completed;
                        }
                    }
                }
            }
        }
    }

    async fn handle_progress(
        &mut self,
        data: crate::heartbeat::HeartbeatData,
    ) -> Option<HeartbeatLoopResult> {
        self.last_progress = data.progress.clone();
        self.send_and_update(data.progress, data.events).await
    }

    async fn handle_ttl(&mut self) -> Option<HeartbeatLoopResult> {
        let (progress, events) = self.snapshot_state().await;
        self.send_and_update(progress, events).await
    }

    async fn snapshot_state(&self) -> (Value, Value) {
        let state = self.state.lock().await;
        (state.progress.clone(), state.events.clone())
    }

    async fn send_and_update(
        &mut self,
        progress: Value,
        events: Value,
    ) -> Option<HeartbeatLoopResult> {
        let request = crate::dms::types::HeartbeatRequest {
            progress: progress.clone(),
            events: events.clone(),
        };

        match self.transport.post_heartbeat(self.task_id, &request).await {
            Ok(update) => {
                apply_heartbeat_token_update(&self.token_ref, &update);
                if let Some(task) = &update.task {
                    self.task_id = task.id;
                } else if let Some(task_id) = update.task_id {
                    self.task_id = task_id;
                }
                if let Err(err) = self
                    .session
                    .apply_heartbeat(
                        &update,
                        Some(progress.clone()),
                        Instant::now(),
                        &self.policy,
                        &mut self.rng,
                    )
                    .await
                {
                    return Some(HeartbeatLoopResult::LostLease(anyhow::Error::new(err)));
                }
                if update.cancel.unwrap_or(false) {
                    self.runner_cancel.cancel();
                    return Some(HeartbeatLoopResult::Cancelled);
                }
                None
            }
            Err(err) => {
                self.runner_cancel.cancel();
                Some(HeartbeatLoopResult::LostLease(err))
            }
        }
    }
}
