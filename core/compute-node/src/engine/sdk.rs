//! Compatibility adapters. The SDK owns machine, lease, data and peer lifetimes.
use super::*;
use crate::poller::{jittered_delay_ms, PollerConfig};
use async_trait::async_trait;
use auki_auth::SecretString;
use auki_sdk::{
    AukiComputeCredential, AukiDmsTasks, AukiRobotCredential, AukiTaskPeerConfig, ComputeConfig,
    MachineCredential, RobotConfig, TaskContext, TaskError, TaskHandler, TaskPeerContext,
    TaskResult, TasksConfig,
};
use serde_json::{json, Value};
use std::time::Duration as StdDuration;

fn task_config(cfg: &NodeConfig) -> TasksConfig {
    TasksConfig {
        request_timeout: StdDuration::from_secs(cfg.request_timeout_secs),
        poll_interval: StdDuration::from_millis(cfg.poll_backoff_ms_min.clamp(10, 300_000)),
        ..TasksConfig::default()
    }
}

pub(super) fn peer_config(
    cfg: &NodeConfig,
    relay: Option<AukiRelayConfig>,
) -> Result<Option<Arc<AukiTaskPeerConfig>>> {
    if !cfg.auki_p2p_enabled {
        return Ok(None);
    }
    let identity = cfg
        .auki_p2p_private_key
        .as_ref()
        .context("AUKI_P2P_PRIVATE_KEY_FILE or AUKI_P2P_PRIVATE_KEY required when P2P is enabled")?
        .identity()?;
    let dds = cfg
        .dds_base_url
        .as_ref()
        .context("DDS_BASE_URL is required")?;
    Ok(Some(Arc::new(AukiTaskPeerConfig::new(
        identity,
        dds.as_str(),
        peer_facade_config(cfg, relay)?,
    )?)))
}

fn compose(
    runners: RunnerComposition,
    peer: bool,
) -> Result<(RunnerRegistry, Option<AukiProtocolsHandle>)> {
    let protocols = peer.then(AukiProtocolsHandle::default);
    let runners = runners.compose(RunnerDependencies {
        protocols: protocols.clone(),
    })?;
    Ok((runners, protocols))
}

fn runtime(
    machine: impl Into<MachineCredential>,
    runners: &RunnerRegistry,
    cfg: &NodeConfig,
    peer: Option<Arc<AukiTaskPeerConfig>>,
) -> Result<AukiDmsTasks> {
    Ok(match peer {
        Some(peer) => {
            AukiDmsTasks::new_with_peer(machine, runners.capabilities(), task_config(cfg), peer)?
        }
        None => AukiDmsTasks::new(machine, runners.capabilities(), task_config(cfg))?,
    })
}

pub(super) async fn run_compute(
    cfg: NodeConfig,
    runners: RunnerComposition,
    shutdown: CancellationToken,
) -> Result<()> {
    if shutdown.is_cancelled() {
        return Ok(());
    }
    let peer = peer_config(&cfg, None)?;
    let (runners, protocols) = compose(runners, peer.is_some())?;
    let mut machine = ComputeConfig::new(
        cfg.dds_base_url
            .as_ref()
            .context("DDS_BASE_URL is required")?
            .as_str(),
        cfg.dms_base_url.as_str(),
        SecretString::new(
            cfg.reg_secret
                .as_deref()
                .context("REG_SECRET is required")?,
        ),
        SecretString::new(
            cfg.secp256k1_privhex
                .as_deref()
                .context("SECP256K1_PRIVHEX is required")?,
        ),
        &cfg.node_version,
        &crate::storage::client::env_client_id(),
    )?;
    machine.request_timeout = StdDuration::from_secs(cfg.request_timeout_secs);
    machine.registration_interval =
        StdDuration::from_secs(cfg.register_interval_secs.unwrap_or(120));
    machine.peer_identity = peer.as_ref().map(|peer| peer.identity_proof());
    let tasks = runtime(AukiComputeCredential::new(machine)?, &runners, &cfg, peer)?;
    run_host(
        &tasks,
        &cfg,
        &runners,
        protocols,
        false,
        shutdown,
        CancellationToken::new(),
    )
    .await
}

pub(super) async fn run_robot(
    cfg: RobotNodeConfig,
    runners: RunnerComposition,
    shutdown: CancellationToken,
    forced: CancellationToken,
) -> Result<()> {
    if shutdown.is_cancelled() || forced.is_cancelled() {
        return Ok(());
    }
    cfg.validate_relay_config()?;
    let runtime_cfg = cfg.runtime_config();
    validate_robot_p2p_config(&runtime_cfg, cfg.relay_config().is_some())?;
    let peer = peer_config(&runtime_cfg, cfg.relay_config())?;
    let (runners, protocols) = compose(runners, peer.is_some())?;
    let mut machine = RobotConfig::new(
        cfg.dds_base_url.as_str(),
        cfg.dms_base_url.as_str(),
        SecretString::new(cfg.registration_credentials()),
        &cfg.node_version,
        &crate::storage::client::env_client_id(),
        cfg.audience().context(
            "set DDS_ROBOT_AUDIENCE or RobotNodeConfig::set_audience to the deployment's exclusive robot audience",
        )?,
        runners.capabilities(),
    )?;
    machine.request_timeout = StdDuration::from_secs(cfg.request_timeout_secs);
    machine.peer_identity = peer.as_ref().map(|peer| peer.identity_proof());
    let tasks = runtime(
        AukiRobotCredential::new(machine)?,
        &runners,
        &runtime_cfg,
        peer,
    )?;
    run_host(
        &tasks,
        &runtime_cfg,
        &runners,
        protocols,
        true,
        shutdown,
        forced,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_host(
    tasks: &AukiDmsTasks,
    cfg: &NodeConfig,
    runners: &RunnerRegistry,
    protocols: Option<AukiProtocolsHandle>,
    robot: bool,
    graceful: CancellationToken,
    forced: CancellationToken,
) -> Result<()> {
    let handler = RunnerAdapter {
        runners,
        protocols: protocols.clone(),
        robot,
    };
    let result = run_host_loop(tasks, cfg, &handler, &graceful, &forced).await;
    let cleanup = tasks.close().await;
    if let Some(protocols) = protocols {
        protocols.state.write().take();
    }
    cleanup?;
    result
}

async fn run_host_loop(
    tasks: &AukiDmsTasks,
    cfg: &NodeConfig,
    handler: &RunnerAdapter<'_>,
    graceful: &CancellationToken,
    forced: &CancellationToken,
) -> Result<()> {
    tokio::select! { biased;
        _ = graceful.cancelled() => return Ok(()),
        _ = forced.cancelled() => return Ok(()),
        result = tasks.start(graceful) => result?,
    }
    if let (Some(handle), Some(peer)) = (&handler.protocols, tasks.peer()) {
        handle.activate((*peer).clone());
    }
    loop {
        let claimed = async {
            if let Some(peer) = tasks.peer() {
                let ready = wait_until_peer_ready(&mut peer.subscribe_status(), graceful)
                    .await
                    .map_err(|_| TaskError::Authority("robot peer stopped"))?;
                if !ready {
                    return Ok(None);
                }
            }
            tasks.claim_any(graceful).await
        };
        let lease = tokio::select! { biased;
            _ = graceful.cancelled() => break,
            _ = forced.cancelled() => break,
            result = claimed => result,
        };
        match lease {
            Ok(Some(lease)) => match lease.execute_with_optional_peer(handler, forced).await {
                Ok(()) | Err(TaskError::Handler | TaskError::LeaseLost) => continue,
                Err(TaskError::Cancelled) if !forced.is_cancelled() => continue,
                Err(TaskError::Cancelled) => break,
                Err(
                    error
                    @ (TaskError::Closed | TaskError::Authentication | TaskError::PeerCleanup),
                ) => return Err(error.into()),
                Err(error) => tracing::warn!(%error, "Task ended; backing off before polling"),
            },
            Ok(None) => {}
            Err(
                error @ (TaskError::Closed
                | TaskError::Authentication
                | TaskError::Authority(_)
                | TaskError::Configuration(_)
                | TaskError::PeerCleanup),
            ) => return Err(error.into()),
            Err(error) => tracing::warn!(%error, "Task claim failed; backing off"),
        }
        let delay = jittered_delay_ms(PollerConfig {
            backoff_ms_min: cfg.poll_backoff_ms_min,
            backoff_ms_max: cfg.poll_backoff_ms_max,
        });
        tokio::select! {
            _ = graceful.cancelled() => break,
            _ = forced.cancelled() => break,
            _ = tokio::time::sleep(StdDuration::from_millis(delay)) => {},
        }
    }
    Ok(())
}

pub(super) async fn run_cycle(
    cfg: &NodeConfig,
    dms: &DmsClient,
    runners: &RunnerRegistry,
) -> Result<bool> {
    let tasks = AukiDmsTasks::from_client(
        dms.clone(),
        crate::storage::client::env_client_id(),
        runners.capabilities(),
        task_config(cfg),
    )?;
    let handler = RunnerAdapter {
        runners,
        protocols: None,
        robot: false,
    };
    let result = async {
        let lease = match tasks.claim_any(&CancellationToken::new()).await {
            Ok(Some(lease)) => lease,
            Ok(None) | Err(TaskError::Busy) => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        match lease.execute(&handler, &CancellationToken::new()).await {
            Ok(()) | Err(TaskError::Handler) => Ok(true),
            Err(error) => Err(anyhow::Error::new(error)),
        }
    }
    .await;
    tasks.close().await?;
    result
}

struct RunnerAdapter<'a> {
    runners: &'a RunnerRegistry,
    protocols: Option<AukiProtocolsHandle>,
    robot: bool,
}

#[async_trait]
impl TaskHandler for RunnerAdapter<'_> {
    async fn run(&self, task: TaskContext) -> std::result::Result<TaskResult, TaskError> {
        let lease = task.credential.lease_snapshot()?;
        let ports = crate::storage::build_task_ports(&task).map_err(|_| TaskError::Data)?;
        let peer = task.peer();
        let _activation = match (&self.protocols, &peer) {
            (Some(handle), Some(peer)) if !self.robot => Some(
                handle
                    .activate_task((**peer).clone())
                    .map_err(|_| TaskError::Handler)?,
            ),
            _ => None,
        };
        let ctrl = SdkControlPlane(task.clone());
        let token = crate::storage::TokenRef::from_task(task.access_token.clone());
        let run = self
            .runners
            .run_for_lease(&lease, &*ports.input, &*ports.output, &ctrl, &token);
        tokio::pin!(run);
        let result = match peer {
            Some(peer) => {
                let mut status = peer.subscribe_status();
                tokio::select! {
                    result = &mut run => result,
                    _ = wait_for_peer_loss(&mut status) => {
                        task.cancellation().cancel();
                        let _ = run.await;
                        ports.close().await;
                        return Err(TaskError::Cancelled);
                    }
                }
            }
            None => run.await,
        };
        ports.close().await;
        let mut artifacts = ports.uploaded_artifacts();
        artifacts.sort_by(|a, b| a.logical_path.cmp(&b.logical_path));
        let output_cids = artifacts.iter().filter_map(|a| a.id.clone()).collect();
        let receipt = json!({
            "job": {"task_id":lease.task.id,"job_id":lease.task.job_id,
                "domain_id":lease.domain_id,"capability":lease.task.capability},
            "artifacts": artifacts,
        });
        match result {
            Ok(()) => Ok(TaskResult {
                output_cids,
                meta: receipt,
            }),
            Err(error) => {
                task.set_failure(error.to_string(), receipt)?;
                Err(TaskError::Handler)
            }
        }
    }
}

struct SdkControlPlane(TaskContext);
#[async_trait]
impl ControlPlane for SdkControlPlane {
    async fn is_cancelled(&self) -> bool {
        self.0.is_cancelled()
    }
    async fn progress(&self, value: Value) -> Result<()> {
        Ok(self.0.progress(value)?)
    }
    async fn log_event(&self, value: Value) -> Result<()> {
        Ok(self.0.log_event(value)?)
    }
}
