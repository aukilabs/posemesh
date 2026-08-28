use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use auki_p2p::{Identity, Multiaddr, PeerIdentityProof, Protocol};
use auki_p2p_dataset::{
    DatasetRoutePolicy, DatasetService, P2pDataset, P2pDatasetAdapter, P2pDatasetServer,
};
use auki_sdk::{
    AukiPeer, AukiPeerConfig, AukiPeerStatus, AukiRelayConfig, AukiRelayMode,
    ExternalAuthorityControl,
};
use compute_runner_api::{ArtifactSink, ControlPlane, InputSource, LeaseEnvelope, Runner, TaskCtx};
use parking_lot::RwLock as SyncRwLock;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    auth::token_manager::{TokenProvider, AUTH_FAILURE_BACKOFF},
    config::{NodeConfig, RelayMode, RobotNodeConfig},
    dds::p2p::{DdsP2pClient, PeerBindingClient, RobotP2pAuthorityDriver, RobotP2pAuthoritySource},
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
        let runner_lease = lease.without_p2p_credentials();
        let ctx = TaskCtx {
            lease: &runner_lease,
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

/// Typed process-level dependencies available while constructing runners.
///
/// A runner takes the protocol handles it needs in its constructor. They are
/// deliberately absent from [`TaskCtx`], whose fields vary for every task.
/// Adding another protocol means adding another explicit typed handle here,
/// rather than inserting it into an untyped service locator.
#[derive(Clone, Default)]
struct RunnerDependencies {
    dataset: Option<DatasetService>,
}

impl RunnerDependencies {
    fn require_dataset(&self) -> Result<DatasetService> {
        self.dataset
            .clone()
            .context("the authenticated P2P dataset protocol is unavailable")
    }
}

/// One typed, fail-closed protocol slot shared with process-constructed
/// runners. Compute activates it only for the current task peer; Robot keeps
/// one activation for its fixed-Domain peer lifetime.
#[derive(Clone, Default)]
struct TaskDatasetSlot {
    state: Arc<SyncRwLock<TaskDatasetSlotState>>,
}

#[derive(Default)]
struct TaskDatasetSlotState {
    generation: u64,
    active: Option<Arc<dyn P2pDataset>>,
}

struct TaskDatasetActivation {
    slot: TaskDatasetSlot,
    generation: u64,
}

#[derive(Clone, Copy)]
enum PeerRuntimeKind {
    Robot,
    Compute,
}

impl PeerRuntimeKind {
    fn app_id(self) -> &'static str {
        match self {
            Self::Robot => "posemesh-robot-node",
            Self::Compute => "posemesh-compute-node",
        }
    }

    fn storage_directory(self) -> &'static str {
        match self {
            Self::Robot => "robot",
            Self::Compute => "compute",
        }
    }
}

fn peer_facade_config(
    cfg: &NodeConfig,
    kind: PeerRuntimeKind,
    relay: Option<crate::config::RelayBookingConfig>,
) -> Result<AukiPeerConfig> {
    let peer_id = cfg
        .p2p_peer_id()
        .ok_or_else(|| anyhow!("a persisted P2P identity is required for the Auki peer facade"))?;
    let listen_addresses = parse_p2p_multiaddrs(
        &cfg.auki_p2p_listen_multiaddrs,
        "AUKI_P2P_LISTEN_MULTIADDRS",
    )?;
    let direct_routes = parse_p2p_multiaddrs(
        &cfg.auki_p2p_advertised_multiaddrs,
        "AUKI_P2P_ADVERTISED_MULTIADDRS",
    )?;
    validate_advertised_p2p_multiaddrs(&direct_routes)?;
    let storage_root = std::env::temp_dir()
        .join("posemesh-auki-peer")
        .join(peer_id.to_string())
        .join(kind.storage_directory());
    let config = AukiPeerConfig::new(cfg.dms_base_url.as_str(), kind.app_id(), storage_root)?
        .with_listen_addresses(listen_addresses)?
        .with_advertised_direct_routes(direct_routes)?;

    let Some(relay) = relay.filter(|relay| relay.mode().is_enabled()) else {
        return Ok(config.direct_only());
    };
    let relay = AukiRelayConfig::new(
        match relay.booking_mode() {
            crate::config::RelayBookingMode::Public => AukiRelayMode::Public,
            crate::config::RelayBookingMode::Dedicated => AukiRelayMode::Dedicated,
        },
        relay.relay_count(),
        StdDuration::from_secs(relay.requested_duration_seconds()),
        relay.status_poll_interval(),
    )?;
    Ok(config.with_relay(relay)?)
}

struct PreparedPeerIdentity {
    identity: Identity,
    proof: PeerIdentityProof,
    dds: DdsP2pClient,
    binding: PeerBindingClient,
}

fn prepare_peer_identity(cfg: &NodeConfig) -> Result<Option<PreparedPeerIdentity>> {
    if !cfg.auki_p2p_enabled {
        return Ok(None);
    }
    let identity = cfg
        .auki_p2p_private_key
        .as_ref()
        .ok_or_else(|| {
            anyhow!(
                "AUKI_P2P_PRIVATE_KEY_FILE or AUKI_P2P_PRIVATE_KEY required when P2P is enabled"
            )
        })?
        .identity()?;
    let dds_base_url = cfg
        .dds_base_url
        .clone()
        .ok_or_else(|| anyhow!("DDS_BASE_URL required when AUKI_P2P_ENABLED=true"))?;
    let dds = DdsP2pClient::new(
        dds_base_url,
        StdDuration::from_secs(cfg.request_timeout_secs.max(1)),
    )?;
    let proof = identity.proof();
    let binding = PeerBindingClient::new(dds.clone(), proof.clone());
    Ok(Some(PreparedPeerIdentity {
        identity,
        proof,
        dds,
        binding,
    }))
}

#[derive(Clone)]
struct ComputeP2pHost {
    identity: Identity,
    proof: PeerIdentityProof,
    dds: DdsP2pClient,
    config: AukiPeerConfig,
    dataset_slot: TaskDatasetSlot,
}

#[derive(Clone)]
#[doc(hidden)]
pub struct ComputeAuthorityUpdater {
    domain_id: Uuid,
    proof: PeerIdentityProof,
    dds: DdsP2pClient,
    control: Arc<ExternalAuthorityControl>,
}

struct ComputeTaskPeer {
    peer: Option<AukiPeer>,
    dataset_activation: Option<TaskDatasetActivation>,
    authority: ComputeAuthorityUpdater,
}

impl ComputeP2pHost {
    async fn start_task(&self, lease: &LeaseEnvelope) -> Result<Option<ComputeTaskPeer>> {
        let Some((token, expires_at)) = complete_p2p_credential(
            lease.p2p_access_token.as_deref(),
            lease.p2p_access_token_expires_at,
        )?
        else {
            return Ok(None);
        };
        let domain_id = lease
            .domain_id
            .ok_or_else(|| anyhow!("P2P task lease is missing its Domain"))?;
        let update = self
            .dds
            .external_authority_update(&self.proof, domain_id, token, expires_at)
            .await
            .context("prepare task peer authority")?;
        let (peer, control) =
            AukiPeer::start_external(self.identity.clone(), update, self.config.clone())
                .await
                .context("start task-scoped Auki peer")?;
        let dataset =
            match P2pDatasetAdapter::new(peer.protocol_context(), DatasetRoutePolicy::DirectOnly) {
                Ok(dataset) => Arc::new(dataset),
                Err(error) => {
                    let _ = peer.shutdown().await;
                    return Err(error.into());
                }
            };
        let active_dataset: Arc<dyn P2pDataset> = dataset;
        let dataset_activation = match self.dataset_slot.activate(active_dataset) {
            Ok(activation) => activation,
            Err(error) => {
                let _ = peer.shutdown().await;
                return Err(error);
            }
        };
        Ok(Some(ComputeTaskPeer {
            peer: Some(peer),
            dataset_activation: Some(dataset_activation),
            authority: ComputeAuthorityUpdater {
                domain_id,
                proof: self.proof.clone(),
                dds: self.dds.clone(),
                control: Arc::new(control),
            },
        }))
    }
}

impl ComputeAuthorityUpdater {
    async fn apply(
        &self,
        domain_id: Option<Uuid>,
        token: Option<&str>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<()> {
        if domain_id.is_some_and(|domain_id| domain_id != self.domain_id) {
            return Err(anyhow!("heartbeat moved the task peer to another Domain"));
        }
        let Some((token, expires_at)) = complete_p2p_credential(token, expires_at)? else {
            return Ok(());
        };
        let update = self
            .dds
            .external_authority_update(&self.proof, self.domain_id, token, expires_at)
            .await
            .context("prepare heartbeat peer authority")?;
        self.control
            .replace(update)
            .await
            .context("replace heartbeat peer authority")?;
        Ok(())
    }
}

impl ComputeTaskPeer {
    fn authority(&self) -> ComputeAuthorityUpdater {
        self.authority.clone()
    }

    fn subscribe_status(&self) -> tokio::sync::watch::Receiver<AukiPeerStatus> {
        self.peer
            .as_ref()
            .expect("live task peer")
            .subscribe_status()
    }

    async fn shutdown(mut self) -> Result<()> {
        drop(self.dataset_activation.take());
        if let Some(peer) = self.peer.take() {
            peer.shutdown().await.context("shut down task Auki peer")?;
        }
        Ok(())
    }
}

async fn shutdown_task_peer(task_peer: &mut Option<ComputeTaskPeer>) -> Result<()> {
    match task_peer.take() {
        Some(peer) => peer.shutdown().await,
        None => Ok(()),
    }
}

fn complete_p2p_credential(
    token: Option<&str>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<Option<(&str, chrono::DateTime<chrono::Utc>)>> {
    match (token, expires_at) {
        (Some(token), Some(expires_at)) => Ok(Some((token, expires_at))),
        (None, None) => Ok(None),
        _ => Err(anyhow!(
            "P2P access token and expiration must be supplied together"
        )),
    }
}

impl TaskDatasetSlot {
    fn activate(&self, dataset: Arc<dyn P2pDataset>) -> Result<TaskDatasetActivation> {
        let mut state = self.state.write();
        if state.active.is_some() {
            return Err(anyhow!("the task dataset protocol is already active"));
        }
        state.generation = state
            .generation
            .checked_add(1)
            .ok_or_else(|| anyhow!("the task dataset activation generation is exhausted"))?;
        state.active = Some(dataset);
        Ok(TaskDatasetActivation {
            slot: self.clone(),
            generation: state.generation,
        })
    }

    fn current(&self) -> Result<Arc<dyn P2pDataset>> {
        self.state
            .read()
            .active
            .clone()
            .ok_or_else(|| anyhow!("the task dataset protocol is not active"))
    }
}

impl Drop for TaskDatasetActivation {
    fn drop(&mut self) {
        let mut state = self.slot.state.write();
        if state.generation == self.generation {
            state.active = None;
        }
    }
}

#[async_trait]
impl P2pDataset for TaskDatasetSlot {
    async fn register(
        &self,
        registration: auki_p2p_dataset::P2pDatasetRegistration,
    ) -> anyhow::Result<auki_p2p_dataset::P2pDatasetReference> {
        self.current()?.register(registration).await
    }

    async fn fetch(
        &self,
        reference: &auki_p2p_dataset::P2pDatasetReference,
        destination: &std::path::Path,
    ) -> anyhow::Result<()> {
        self.current()?.fetch(reference, destination).await
    }
}

/// Builds the runner registry after process-level protocols have started.
///
/// Plain registries convert into a fixed composition automatically. Dataset-
/// aware applications use [`RunnerComposition::with_dataset`] and construct
/// each runner with that explicit dependency.
pub struct RunnerComposition {
    build: Box<dyn FnOnce(RunnerDependencies) -> Result<RunnerRegistry> + Send>,
}

impl RunnerComposition {
    fn new<F>(build: F) -> Self
    where
        F: FnOnce(RunnerDependencies) -> Result<RunnerRegistry> + Send + 'static,
    {
        Self {
            build: Box::new(build),
        }
    }

    /// Construct runners that explicitly require the dataset protocol.
    pub fn with_dataset<F>(build: F) -> Self
    where
        F: FnOnce(DatasetService) -> RunnerRegistry + Send + 'static,
    {
        Self::new(move |dependencies| Ok(build(dependencies.require_dataset()?)))
    }

    fn compose(self, dependencies: RunnerDependencies) -> Result<RunnerRegistry> {
        (self.build)(dependencies)
    }
}

impl From<RunnerRegistry> for RunnerComposition {
    fn from(runners: RunnerRegistry) -> Self {
        Self::new(move |_| Ok(runners))
    }
}

/// Run the node main loop. Networking and storage are wired in later prompts.
pub async fn run_node(
    cfg: crate::config::NodeConfig,
    runners: impl Into<RunnerComposition> + Send,
) -> Result<()> {
    let shutdown = CancellationToken::new();
    let signal_token = shutdown.clone();
    let signal_task = tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            signal_token.cancel();
        }
    });

    let result = run_node_with_shutdown(cfg, runners.into(), shutdown.clone()).await;

    shutdown.cancel();
    signal_task.abort();
    let _ = signal_task.await;

    result
}

pub async fn run_node_with_shutdown(
    cfg: crate::config::NodeConfig,
    runners: impl Into<RunnerComposition> + Send,
    shutdown: CancellationToken,
) -> Result<()> {
    let prepared_peer = prepare_peer_identity(&cfg)?;
    let compute_peer_config = prepared_peer
        .as_ref()
        .map(|_| peer_facade_config(&cfg, PeerRuntimeKind::Compute, None))
        .transpose()?;
    let dataset_slot = prepared_peer.as_ref().map(|_| TaskDatasetSlot::default());
    let runners = runners
        .into()
        .compose(runner_dependencies(dataset_slot.as_ref()))
        .context("construct task runners")?;
    let peer_binding = prepared_peer
        .as_ref()
        .map(|prepared| prepared.binding.clone());
    let siwe =
        match crate::auth::SiweAfterRegistration::from_config_with_peer_binding(&cfg, peer_binding)
        {
            Ok(siwe) => siwe,
            Err(error) => return Err(error),
        };
    info!("DDS SIWE authentication configured; waiting for DDS registration");
    let siwe_handle = tokio::select! {
        result = siwe.start() => match result {
            Ok(handle) => handle,
            Err(error) => {
                siwe.shutdown().await;
                return Err(error);
            }
        },
        _ = shutdown.cancelled() => {
            siwe.shutdown().await;
            info!("Shutdown signal received before SIWE authentication completed");
            return Ok(());
        }
    };
    info!("DDS SIWE token manager started");

    let auth: Arc<dyn TokenProvider> = Arc::new(siwe_handle.clone());
    let compute_peer = match (prepared_peer, dataset_slot) {
        (Some(prepared), Some(dataset_slot)) => Some(ComputeP2pHost {
            config: compute_peer_config.expect("P2P identity creates a peer config"),
            identity: prepared.identity,
            proof: prepared.proof,
            dds: prepared.dds,
            dataset_slot,
        }),
        (None, None) => None,
        _ => unreachable!("peer identity and dataset slot are created together"),
    };
    let result = run_authenticated_node_loop(
        &cfg,
        &runners,
        auth,
        shutdown,
        "SIWE",
        cfg.auki_p2p_enabled,
        NodeLoopP2p {
            compute_peer,
            fixed_peer_status: None,
            forced_shutdown: None,
        },
    )
    .await;

    siwe_handle.shutdown().await;
    info!("Shutdown signal received; exiting run_node loop");

    result
}

/// Run a robot-authenticated node until interrupted.
pub async fn run_robot_node(
    cfg: RobotNodeConfig,
    runners: impl Into<RunnerComposition> + Send,
) -> Result<()> {
    let shutdown = CancellationToken::new();
    let forced_shutdown = CancellationToken::new();
    let signal_token = shutdown.clone();
    let force_token = forced_shutdown.clone();
    let signal_task = tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            signal_token.cancel();
        }
        if tokio::signal::ctrl_c().await.is_ok() {
            force_token.cancel();
        }
    });

    let result = run_robot_node_with_shutdowns(
        cfg,
        runners.into(),
        shutdown.clone(),
        forced_shutdown.clone(),
    )
    .await;

    shutdown.cancel();
    forced_shutdown.cancel();
    signal_task.abort();
    let _ = signal_task.await;

    result
}

/// Run a robot-authenticated node until the supplied cancellation token fires.
pub async fn run_robot_node_with_shutdown(
    cfg: RobotNodeConfig,
    runners: impl Into<RunnerComposition> + Send,
    shutdown: CancellationToken,
) -> Result<()> {
    run_robot_node_with_shutdowns(cfg, runners.into(), shutdown, CancellationToken::new()).await
}

/// Run a robot-authenticated node with separate graceful and forced shutdown
/// signals.
///
/// The first token stops task polling and begins the bounded dataset-reference
/// drain. The second token interrupts that drain and tears down P2P authority.
pub async fn run_robot_node_with_shutdowns(
    cfg: RobotNodeConfig,
    runners: impl Into<RunnerComposition> + Send,
    shutdown: CancellationToken,
    forced_shutdown: CancellationToken,
) -> Result<()> {
    cfg.validate_relay_booking_config()?;
    let runtime_cfg = cfg.runtime_config();
    let relay_config = cfg.relay_booking_config();
    validate_robot_p2p_config(&runtime_cfg, relay_config.mode())?;
    if runtime_cfg.auki_p2p_enabled && relay_config.mode() == RelayMode::Disabled {
        warn!("Robot relay booking is disabled; direct-only immutable dataset references cannot be repaired if the advertised route is unreachable");
    }
    let route_policy = if relay_config.mode().is_enabled() {
        DatasetRoutePolicy::RelayRequired
    } else {
        DatasetRoutePolicy::DirectOnly
    };
    if shutdown.is_cancelled() {
        return Ok(());
    }
    let prepared_peer = prepare_peer_identity(&runtime_cfg)?;
    let peer_config = prepared_peer
        .as_ref()
        .map(|_| peer_facade_config(&runtime_cfg, PeerRuntimeKind::Robot, Some(relay_config)))
        .transpose()?;
    let dataset_slot = prepared_peer.as_ref().map(|_| TaskDatasetSlot::default());
    let runners = runners
        .into()
        .compose(runner_dependencies(dataset_slot.as_ref()))
        .context("construct task runners")?;
    let peer_binding = prepared_peer
        .as_ref()
        .map(|prepared| prepared.binding.clone());
    let robot = match crate::auth::RobotMachineAuth::from_config_with_peer_binding(
        &cfg,
        runners.capabilities(),
        peer_binding,
    ) {
        Ok(robot) => robot,
        Err(error) => return Err(error),
    };
    info!("DDS robot authentication configured");
    let robot_handle = tokio::select! {
        result = robot.start() => match result {
            Ok(handle) => handle,
            Err(error) => {
                robot.shutdown().await;
                return Err(error);
            }
        },
        _ = shutdown.cancelled() => {
            robot.shutdown().await;
            info!("Shutdown signal received before robot authentication completed");
            return Ok(());
        }
    };
    info!("DDS robot token manager started");

    let auth: Arc<dyn TokenProvider> = Arc::new(robot_handle.clone());
    let Some(prepared_peer) = prepared_peer else {
        let result = run_authenticated_node_loop(
            &runtime_cfg,
            &runners,
            auth,
            shutdown,
            "robot",
            true,
            NodeLoopP2p {
                compute_peer: None,
                fixed_peer_status: None,
                forced_shutdown: Some(forced_shutdown),
            },
        )
        .await;
        robot_handle.shutdown().await;
        return result;
    };
    let dataset_slot = dataset_slot.expect("P2P identity creates a dataset slot");
    let authority_source = RobotP2pAuthoritySource::new(
        prepared_peer.dds.clone(),
        Arc::clone(&auth),
        prepared_peer.proof.clone(),
    );
    let prepared_authority = tokio::select! {
        result = authority_source.prepare() => match result {
            Ok(authority) => authority,
            Err(error) => {
                robot_handle.shutdown().await;
                return Err(anyhow::Error::new(error).context("prepare Robot P2P authority"));
            }
        },
        _ = shutdown.cancelled() => {
            robot_handle.shutdown().await;
            return Ok(());
        }
    };
    let domain_id = prepared_authority.domain_id();
    let authority_expires_at = prepared_authority.expires_at();
    let peer_config = peer_config.expect("P2P identity creates a peer config");
    let (peer, authority_control) = tokio::select! {
        result = AukiPeer::start_external(
            prepared_peer.identity,
            prepared_authority.into_update(),
            peer_config,
        ) => match result {
            Ok(peer) => peer,
            Err(error) => {
                robot_handle.shutdown().await;
                return Err(error).context("start Robot Auki peer");
            }
        },
        _ = shutdown.cancelled() => {
            robot_handle.shutdown().await;
            return Ok(());
        }
    };
    info!(peer_id = %peer.peer_id(), %domain_id, "Robot Auki peer is ready");
    let authority_lifecycle = CancellationToken::new();
    let authority_driver = match RobotP2pAuthorityDriver::start(
        authority_source,
        domain_id,
        authority_expires_at,
        authority_control,
        &authority_lifecycle,
    ) {
        Ok(driver) => driver,
        Err(error) => {
            let _ = peer.shutdown().await;
            authority_lifecycle.cancel();
            robot_handle.shutdown().await;
            return Err(anyhow::Error::new(error).context("start Robot P2P authority driver"));
        }
    };
    let dataset = match P2pDatasetAdapter::new(peer.protocol_context(), route_policy) {
        Ok(dataset) => Arc::new(dataset),
        Err(error) => {
            let _ = peer.shutdown().await;
            authority_lifecycle.cancel();
            authority_driver.shutdown().await;
            robot_handle.shutdown().await;
            return Err(error.into());
        }
    };
    let active_dataset: Arc<dyn P2pDataset> = dataset.clone();
    let dataset_activation = match dataset_slot.activate(active_dataset) {
        Ok(activation) => activation,
        Err(error) => {
            let _ = peer.shutdown().await;
            authority_lifecycle.cancel();
            authority_driver.shutdown().await;
            robot_handle.shutdown().await;
            return Err(error);
        }
    };
    let dataset_server = match dataset.start_serving().await {
        Ok(server) => server,
        Err(error) => {
            drop(dataset_activation);
            let _ = peer.shutdown().await;
            authority_lifecycle.cancel();
            authority_driver.shutdown().await;
            robot_handle.shutdown().await;
            return Err(error.into());
        }
    };
    let peer_status = peer.subscribe_status();
    let mut result = run_authenticated_node_loop(
        &runtime_cfg,
        &runners,
        auth,
        shutdown,
        "robot",
        true,
        NodeLoopP2p {
            compute_peer: None,
            fixed_peer_status: Some(peer_status),
            forced_shutdown: Some(forced_shutdown.clone()),
        },
    )
    .await;

    if let Err(error) = drain_dataset_references(&dataset, &forced_shutdown).await {
        warn!(error = %error, "forced Robot shutdown interrupted the dataset drain; published references may be lost");
        if result.is_ok() {
            result =
                Err(error.context("forced Robot shutdown interrupted the dataset reference drain"));
        }
    }
    shutdown_dataset_server(Some(dataset_server)).await;
    drop(dataset_activation);
    if let Err(error) = peer.shutdown().await {
        warn!(error = %error, "Robot Auki peer shutdown failed");
        if result.is_ok() {
            result = Err(error.into());
        }
    }
    authority_lifecycle.cancel();
    authority_driver.shutdown().await;
    robot_handle.shutdown().await;
    info!("Shutdown signal received; exiting robot node loop");

    result
}

struct NodeLoopP2p {
    compute_peer: Option<ComputeP2pHost>,
    fixed_peer_status: Option<tokio::sync::watch::Receiver<AukiPeerStatus>>,
    forced_shutdown: Option<CancellationToken>,
}

fn runner_dependencies(dataset_slot: Option<&TaskDatasetSlot>) -> RunnerDependencies {
    let dataset = dataset_slot.map(|slot| {
        let dataset: Arc<dyn P2pDataset> = Arc::new(slot.clone());
        DatasetService::new(dataset)
    });
    RunnerDependencies { dataset }
}

fn parse_p2p_multiaddrs(values: &[String], setting: &'static str) -> Result<Vec<Multiaddr>> {
    values
        .iter()
        .map(|value| {
            let address = value
                .parse::<Multiaddr>()
                .with_context(|| format!("invalid TCP multiaddr in {setting}"))?;
            if !address
                .iter()
                .any(|protocol| matches!(protocol, Protocol::Tcp(_)))
            {
                return Err(anyhow!("non-TCP multiaddr in {setting}"));
            }
            Ok(address)
        })
        .collect()
}

fn validate_advertised_p2p_multiaddrs(addresses: &[Multiaddr]) -> Result<()> {
    for address in addresses {
        if address
            .iter()
            .any(|protocol| matches!(protocol, Protocol::Tcp(0)))
        {
            return Err(anyhow!(
                "AUKI_P2P_ADVERTISED_MULTIADDRS must not contain tcp/0"
            ));
        }
        if address.iter().any(|protocol| match protocol {
            Protocol::Ip4(ip) => ip.is_unspecified(),
            Protocol::Ip6(ip) => ip.is_unspecified(),
            _ => false,
        }) {
            return Err(anyhow!(
                "AUKI_P2P_ADVERTISED_MULTIADDRS must contain reachable addresses"
            ));
        }
    }
    Ok(())
}

fn validate_robot_p2p_config(cfg: &NodeConfig, relay_mode: RelayMode) -> Result<()> {
    if cfg.auki_p2p_enabled && cfg.auki_p2p_private_key.is_none() {
        return Err(anyhow!(
            "AUKI_P2P_PRIVATE_KEY_FILE or AUKI_P2P_PRIVATE_KEY required when P2P is enabled"
        ));
    }
    if cfg.auki_p2p_enabled
        && relay_mode == RelayMode::Disabled
        && cfg.auki_p2p_listen_multiaddrs.is_empty()
    {
        return Err(anyhow!(
            "AUKI_P2P_LISTEN_MULTIADDRS required for Robot P2P serving"
        ));
    }
    if cfg.auki_p2p_enabled
        && relay_mode == RelayMode::Disabled
        && cfg.auki_p2p_advertised_multiaddrs.is_empty()
    {
        return Err(anyhow!(
            "AUKI_P2P_ADVERTISED_MULTIADDRS required for Robot P2P serving"
        ));
    }
    Ok(())
}

async fn shutdown_dataset_server(server: Option<P2pDatasetServer>) {
    if let Some(server) = server {
        if let Err(error) = server.shutdown().await {
            warn!(error = %error, "Auki P2P dataset server shutdown failed");
        }
    }
}

async fn drain_dataset_references(
    dataset: &P2pDatasetAdapter,
    forced_shutdown: &CancellationToken,
) -> Result<()> {
    let mut status = dataset.subscribe_serving_status();
    dataset.stop_registrations()?;
    loop {
        let current = dataset.serving_status()?;
        let next_deadline = [
            current.max_available_until,
            current.max_active_transfer_deadline,
        ]
        .into_iter()
        .flatten()
        .max();
        if next_deadline.is_none() && current.active_transfer_count == 0 {
            return Ok(());
        }
        let delay = next_deadline
            .map(|deadline| {
                deadline
                    .signed_duration_since(chrono::Utc::now())
                    .to_std()
                    .unwrap_or_default()
            })
            .unwrap_or(StdDuration::from_secs(1));
        tokio::select! {
            _ = forced_shutdown.cancelled() => return Err(anyhow!("forced shutdown interrupted the dataset drain")),
            changed = status.changed() => {
                changed.map_err(|_| anyhow!("dataset serving status channel closed"))?;
            }
            _ = sleep(delay.max(StdDuration::from_millis(1))) => {}
        }
    }
}

async fn run_authenticated_node_loop(
    cfg: &NodeConfig,
    runners: &RunnerRegistry,
    auth: Arc<dyn TokenProvider>,
    shutdown: CancellationToken,
    auth_kind: &'static str,
    interrupt_on_shutdown: bool,
    p2p: NodeLoopP2p,
) -> Result<()> {
    let NodeLoopP2p {
        compute_peer,
        mut fixed_peer_status,
        forced_shutdown,
    } = p2p;
    let poll_cfg = PollerConfig {
        backoff_ms_min: cfg.poll_backoff_ms_min,
        backoff_ms_max: cfg.poll_backoff_ms_max,
    };

    loop {
        if shutdown.is_cancelled() {
            break;
        }

        if let Some(status) = fixed_peer_status.as_mut() {
            if !wait_until_peer_ready(status, &shutdown).await? {
                break;
            }
        }

        // Ensure a token is available before attempting DMS operations.
        let bearer = if interrupt_on_shutdown {
            tokio::select! {
                result = auth.bearer() => result,
                _ = shutdown.cancelled() => break,
            }
        } else {
            auth.bearer().await
        };
        if let Err(err) = bearer {
            warn!(auth_kind, error = %err, "Failed to obtain bearer token; backing off");
            let delay_ms = jittered_delay_ms(poll_cfg);
            let delay = StdDuration::from_millis(delay_ms);
            let delay = if interrupt_on_shutdown {
                delay.max(AUTH_FAILURE_BACKOFF)
            } else {
                delay
            };
            tokio::select! {
                _ = shutdown.cancelled() => break,
                _ = sleep(delay) => continue,
            }
        }

        let timeout = StdDuration::from_secs(cfg.request_timeout_secs);
        let dms_client = match crate::dms::client::DmsClient::new(
            cfg.dms_base_url.clone(),
            timeout,
            auth.clone(),
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

        let cycle = if interrupt_on_shutdown {
            let acquired = tokio::select! {
                biased;
                result = acquire_lease_with_dms(&dms_client, runners) => result,
                _ = shutdown.cancelled() => break,
            };
            match acquired {
                Ok(Some(acquired)) => {
                    run_leased_cycle_with_abort(
                        cfg,
                        &dms_client,
                        runners,
                        acquired,
                        compute_peer.clone(),
                        fixed_peer_status.clone(),
                        forced_shutdown.as_ref(),
                    )
                    .await
                }
                Ok(None) => Ok(false),
                Err(err) => Err(err),
            }
        } else {
            match acquire_lease_with_dms(&dms_client, runners).await {
                Ok(Some(acquired)) => {
                    run_leased_cycle_with_abort(
                        cfg,
                        &dms_client,
                        runners,
                        acquired,
                        compute_peer.clone(),
                        fixed_peer_status.clone(),
                        forced_shutdown.as_ref(),
                    )
                    .await
                }
                Ok(None) => Ok(false),
                Err(error) => Err(error),
            }
        };

        match cycle {
            Ok(true) => {
                // Successful task execution; immediately attempt next poll.
                continue;
            }
            Ok(false) => {
                let delay_ms = jittered_delay_ms(poll_cfg);
                debug!(delay_ms, "No lease available; backing off before next poll");
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

    Ok(())
}

async fn wait_until_peer_ready(
    status: &mut tokio::sync::watch::Receiver<AukiPeerStatus>,
    shutdown: &CancellationToken,
) -> Result<bool> {
    loop {
        match *status.borrow_and_update() {
            AukiPeerStatus::Ready => return Ok(true),
            AukiPeerStatus::Failed(failure) => {
                return Err(anyhow!("Auki peer failed: {failure:?}"));
            }
            AukiPeerStatus::Stopping | AukiPeerStatus::Stopped => {
                return Err(anyhow!("Auki peer stopped unexpectedly"));
            }
            AukiPeerStatus::Starting
            | AukiPeerStatus::AuthorityUnavailable
            | AukiPeerStatus::RelayUnavailable => {}
        }
        tokio::select! {
            _ = shutdown.cancelled() => return Ok(false),
            changed = status.changed() => {
                changed.map_err(|_| anyhow!("Auki peer status channel closed"))?;
            }
        }
    }
}

async fn wait_for_peer_loss(
    status: &mut tokio::sync::watch::Receiver<AukiPeerStatus>,
) -> anyhow::Error {
    loop {
        match *status.borrow_and_update() {
            AukiPeerStatus::Ready => {}
            AukiPeerStatus::Starting => {
                return anyhow!("Auki peer unexpectedly returned to startup");
            }
            AukiPeerStatus::AuthorityUnavailable => {
                return anyhow!("Auki peer authority became unavailable");
            }
            AukiPeerStatus::RelayUnavailable => {
                return anyhow!("Auki peer relay became unavailable");
            }
            AukiPeerStatus::Failed(failure) => {
                return anyhow!("Auki peer failed: {failure:?}");
            }
            AukiPeerStatus::Stopping | AukiPeerStatus::Stopped => {
                return anyhow!("Auki peer stopped during task execution");
            }
        }
        if status.changed().await.is_err() {
            return anyhow!("Auki peer status channel closed during task execution");
        }
    }
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

async fn apply_compute_authority_update(
    authority: Option<&ComputeAuthorityUpdater>,
    domain_id: Option<Uuid>,
    token: Option<&str>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<()> {
    match authority {
        Some(authority) => authority.apply(domain_id, token, expires_at).await,
        None if token.is_none() && expires_at.is_none() => Ok(()),
        None => Err(anyhow!(
            "DMS returned a P2P credential to a node without a P2P runtime"
        )),
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
    if let Some(token) = hb.p2p_access_token.clone() {
        lease.p2p_access_token = Some(token);
    }
    if let Some(expiry) = hb.p2p_access_token_expires_at {
        lease.p2p_access_token_expires_at = Some(expiry);
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
    cfg: &crate::config::NodeConfig,
    dms: &DmsClient,
    reg: &RunnerRegistry,
) -> Result<bool> {
    let Some(acquired) = acquire_lease_with_dms(dms, reg).await? else {
        return Ok(false);
    };
    run_leased_cycle_with_dms(cfg, dms, reg, acquired, None, None).await
}

struct AcquiredLease {
    capabilities: Vec<String>,
    lease: LeaseEnvelope,
}

struct LeaseExecutionGuard {
    runner_cancel: CancellationToken,
    heartbeat_shutdown: CancellationToken,
    heartbeat_abort: tokio::task::AbortHandle,
}

impl Drop for LeaseExecutionGuard {
    fn drop(&mut self) {
        self.runner_cancel.cancel();
        self.heartbeat_shutdown.cancel();
        self.heartbeat_abort.abort();
    }
}

async fn acquire_lease_with_dms(
    dms: &DmsClient,
    reg: &RunnerRegistry,
) -> Result<Option<AcquiredLease>> {
    let capabilities = reg.capabilities();
    let capability = capabilities
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("no runners registered"))?;

    let Some(lease) = dms.lease_by_capability(&capability).await? else {
        return Ok(None);
    };

    Ok(Some(AcquiredLease {
        capabilities,
        lease,
    }))
}

async fn run_leased_cycle_with_dms(
    cfg: &crate::config::NodeConfig,
    dms: &DmsClient,
    reg: &RunnerRegistry,
    acquired: AcquiredLease,
    compute_peer: Option<ComputeP2pHost>,
    fixed_peer_status: Option<tokio::sync::watch::Receiver<AukiPeerStatus>>,
) -> Result<bool> {
    run_leased_cycle_inner(cfg, dms, reg, acquired, compute_peer, fixed_peer_status).await
}

async fn run_leased_cycle_with_abort(
    cfg: &crate::config::NodeConfig,
    dms: &DmsClient,
    reg: &RunnerRegistry,
    acquired: AcquiredLease,
    compute_peer: Option<ComputeP2pHost>,
    fixed_peer_status: Option<tokio::sync::watch::Receiver<AukiPeerStatus>>,
    forced_shutdown: Option<&CancellationToken>,
) -> Result<bool> {
    let cycle = run_leased_cycle_with_dms(cfg, dms, reg, acquired, compute_peer, fixed_peer_status);
    match forced_shutdown {
        Some(forced_shutdown) => {
            tokio::select! {
                biased;
                _ = forced_shutdown.cancelled() => {
                    Err(anyhow!("forced shutdown interrupted the active Robot task"))
                }
                result = cycle => result,
            }
        }
        None => cycle.await,
    }
}

async fn run_leased_cycle_inner(
    cfg: &crate::config::NodeConfig,
    dms: &DmsClient,
    reg: &RunnerRegistry,
    acquired: AcquiredLease,
    compute_peer: Option<ComputeP2pHost>,
    fixed_peer_status: Option<tokio::sync::watch::Receiver<AukiPeerStatus>>,
) -> Result<bool> {
    use crate::dms::types::{CompleteTaskRequest, FailTaskRequest, HeartbeatRequest};
    use serde_json::json;

    let AcquiredLease {
        capabilities,
        mut lease,
    } = acquired;
    if lease.access_token.is_none() {
        tracing::warn!(
            "Lease missing access token; storage client will fall back to legacy token flow"
        );
    }

    // Initialise session state for heartbeats and token rotation.
    let selector = CapabilitySelector::new(capabilities.clone());
    let session = SessionManager::new(selector);
    let policy = HeartbeatPolicy::new(cfg.heartbeat_min_ratio, cfg.heartbeat_max_ratio);
    let mut rng = StdRng::from_entropy();
    let task_id = lease.task.id;
    let report_setup_failure = |stage: &'static str, err: &anyhow::Error| {
        let details = json!({
            "stage": stage,
            "error": err.to_string(),
        });
        async move {
            let body = FailTaskRequest {
                reason: "node_setup_failed".into(),
                details,
            };
            dms.fail(task_id, &body).await
        }
    };

    let snapshot = match session
        .start_session(&lease, Instant::now(), &policy, &mut rng)
        .await
    {
        Ok(snapshot) => snapshot,
        Err(err) => {
            let original = anyhow!("failed to initialise session: {err}");
            if let Err(fail_err) = report_setup_failure("start_session", &original).await {
                warn!(
                    error = %fail_err,
                    task_id = %task_id,
                    "failed to report setup failure"
                );
                return Err(original);
            }
            return Ok(true);
        }
    };
    if snapshot.cancel() {
        warn!(
            task_id = %snapshot.task_id(),
            "Lease already marked as cancelled; skipping execution"
        );
        return Ok(true);
    }

    let token_ref = crate::storage::TokenRef::new(lease.access_token.clone().unwrap_or_default());

    let heartbeat_initial = match dms
        .heartbeat(
            lease.task.id,
            &HeartbeatRequest {
                progress: json!({}),
                events: Vec::new(),
            },
        )
        .await
    {
        Ok(response) => response,
        Err(err) => {
            if let Err(fail_err) = report_setup_failure("initial_heartbeat", &err).await {
                warn!(
                    error = %fail_err,
                    task_id = %task_id,
                    "failed to report setup failure"
                );
                return Err(err);
            }
            return Ok(true);
        }
    };
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

    let mut task_peer = match compute_peer.as_ref() {
        Some(host) => match host.start_task(&lease).await {
            Ok(peer) => peer,
            Err(error) => {
                if let Err(fail_error) = report_setup_failure("start_task_peer", &error).await {
                    warn!(error = %fail_error, task_id = %task_id, "failed to report P2P setup failure");
                    return Err(error);
                }
                return Ok(true);
            }
        },
        None => {
            if let Err(error) = apply_compute_authority_update(
                None,
                lease.domain_id,
                lease.p2p_access_token.as_deref(),
                lease.p2p_access_token_expires_at,
            )
            .await
            {
                if let Err(fail_error) = report_setup_failure("unexpected_task_peer", &error).await
                {
                    warn!(error = %fail_error, task_id = %task_id, "failed to report P2P setup failure");
                    return Err(error);
                }
                return Ok(true);
            }
            None
        }
    };
    let task_authority = task_peer.as_ref().map(ComputeTaskPeer::authority);
    let task_peer_status = task_peer
        .as_ref()
        .map(ComputeTaskPeer::subscribe_status)
        .or(fixed_peer_status);

    let ports = match crate::storage::build_ports(&lease, token_ref.clone()) {
        Ok(ports) => ports,
        Err(err) => {
            if let Err(shutdown_error) = shutdown_task_peer(&mut task_peer).await {
                warn!(
                    error = %shutdown_error,
                    task_id = %task_id,
                    "failed to shut down task peer after storage setup failure"
                );
            }
            if let Err(fail_err) = report_setup_failure("build_ports", &err).await {
                warn!(
                    error = %fail_err,
                    task_id = %task_id,
                    "failed to report setup failure"
                );
                return Err(err);
            }
            return Ok(true);
        }
    };

    let (progress_tx, progress_rx) = progress_channel();
    let control_state = Arc::new(Mutex::new(ControlState::default()));
    {
        let mut guard = control_state.lock().await;
        guard.progress = json!({});
        guard.events = Vec::new();
    }

    let runner_cancel = CancellationToken::new();
    let heartbeat_shutdown = CancellationToken::new();

    let ctrl = EngineControlPlane::new(
        runner_cancel.clone(),
        progress_tx.clone(),
        control_state.clone(),
    );

    // Trigger an immediate heartbeat once the loop starts to refresh tokens.
    progress_tx.update(json!({}), Vec::new());

    let heartbeat_driver = HeartbeatDriver::new(
        dms.clone(),
        HeartbeatDriverArgs {
            session: session.clone(),
            policy,
            rng,
            progress_rx,
            state: control_state.clone(),
            token_ref: token_ref.clone(),
            p2p_authority: task_authority,
            runner_cancel: runner_cancel.clone(),
            shutdown: heartbeat_shutdown.clone(),
            task_id: lease.task.id,
        },
    );
    let heartbeat_handle = tokio::spawn(async move { heartbeat_driver.run().await });
    let _execution_guard = LeaseExecutionGuard {
        runner_cancel: runner_cancel.clone(),
        heartbeat_shutdown: heartbeat_shutdown.clone(),
        heartbeat_abort: heartbeat_handle.abort_handle(),
    };

    let runner = reg.run_for_lease(&lease, &*ports.input, &*ports.output, &ctrl, &token_ref);
    tokio::pin!(runner);
    let mut run_res = if let Some(mut peer_status) = task_peer_status {
        tokio::select! {
            result = &mut runner => result,
            error = wait_for_peer_loss(&mut peer_status) => {
                runner_cancel.cancel();
                Err(crate::errors::ExecutorError::Runner(error.to_string()))
            }
        }
    } else {
        runner.await
    };

    // Re-broadcast the latest progress/events so the heartbeat loop can flush
    // them before shutdown. Without this, very short tasks may complete before
    // the final heartbeat is delivered, leaving stale progress in DMS.
    {
        let state = control_state.lock().await;
        progress_tx.update(state.progress.clone(), state.events.clone());
    }
    sleep(StdDuration::from_millis(200)).await;

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
            if let Err(error) = shutdown_task_peer(&mut task_peer).await {
                warn!(error = %error, task_id = %task_id, "failed to shut down cancelled task peer");
            }
            return Ok(true);
        }
        HeartbeatLoopResult::LostLease(err) => {
            warn!(
                task_id = %lease.task.id,
                error = %err,
                "Lease lost during heartbeat; abandoning task"
            );
            runner_cancel.cancel();
            if let Err(shutdown_error) = shutdown_task_peer(&mut task_peer).await {
                warn!(error = %shutdown_error, task_id = %task_id, "failed to shut down abandoned task peer");
            }
            return Ok(true);
        }
    }

    if let Err(error) = shutdown_task_peer(&mut task_peer).await {
        if run_res.is_ok() {
            run_res = Err(crate::errors::ExecutorError::Runner(format!(
                "task peer shutdown failed: {error:#}"
            )));
        } else {
            warn!(error = %error, task_id = %task_id, "task peer shutdown also failed");
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
    let output_cids: Vec<String> = uploaded_artifacts
        .iter()
        .filter_map(|artifact| artifact.id.clone())
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
                output_cids,
                meta: json!({
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
    events: Vec<Value>,
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
        let (progress, events) = {
            let mut state = self.state.lock().await;
            state.events.push(fields.clone());
            (state.progress.clone(), state.events.clone())
        };
        self.progress_tx.update(progress, events);
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
    pub p2p_authority: Option<ComputeAuthorityUpdater>,
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
    p2p_authority: Option<ComputeAuthorityUpdater>,
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
            p2p_authority: args.p2p_authority,
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
        let (progress, events) = self.snapshot_state().await;
        self.send_and_update(progress, events).await
    }

    async fn handle_ttl(&mut self) -> Option<HeartbeatLoopResult> {
        let (progress, events) = self.snapshot_state().await;
        self.send_and_update(progress, events).await
    }

    async fn snapshot_state(&self) -> (Value, Vec<Value>) {
        let state = self.state.lock().await;
        (state.progress.clone(), state.events.clone())
    }

    async fn send_and_update(
        &mut self,
        progress: Value,
        events: Vec<Value>,
    ) -> Option<HeartbeatLoopResult> {
        let request = crate::dms::types::HeartbeatRequest {
            progress: progress.clone(),
            events: events.clone(),
        };

        match self.transport.post_heartbeat(self.task_id, &request).await {
            Ok(update) => {
                if !events.is_empty() {
                    let mut state = self.state.lock().await;
                    if state.events.len() >= events.len()
                        && state.events[..events.len()] == events[..]
                    {
                        state.events.drain(0..events.len());
                    }
                }
                apply_heartbeat_token_update(&self.token_ref, &update);
                if let Err(error) = apply_compute_authority_update(
                    self.p2p_authority.as_ref(),
                    update.domain_id,
                    update.p2p_access_token.as_deref(),
                    update.p2p_access_token_expires_at,
                )
                .await
                {
                    self.runner_cancel.cancel();
                    return Some(HeartbeatLoopResult::LostLease(error));
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{P2pPrivateKey, RelayBookingConfig, RelayBookingMode, RobotNodeConfig};
    use auki_p2p::Identity;
    use auki_p2p_dataset::{P2pDatasetReference, P2pDatasetRegistration};

    struct SuccessfulDataset;

    #[async_trait]
    impl P2pDataset for SuccessfulDataset {
        async fn register(
            &self,
            _registration: P2pDatasetRegistration,
        ) -> anyhow::Result<P2pDatasetReference> {
            Err(anyhow!("registration is unused by this test"))
        }

        async fn fetch(
            &self,
            _reference: &P2pDatasetReference,
            _destination: &std::path::Path,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn dataset_runner_composition_fails_without_dataset_runtime() {
        let result = RunnerComposition::with_dataset(|_| RunnerRegistry::new())
            .compose(RunnerDependencies::default());
        let error = result.err().expect("missing dataset must fail composition");

        assert!(error
            .to_string()
            .contains("authenticated P2P dataset protocol is unavailable"));
    }

    #[tokio::test]
    async fn task_dataset_slot_is_active_for_exactly_one_runtime() {
        let slot = TaskDatasetSlot::default();
        let service = DatasetService::new(Arc::new(slot.clone()));
        let reference = P2pDatasetReference {
            schema: "test".into(),
            dataset_id: "test".into(),
            domain_id: Uuid::nil(),
            name: "test".into(),
            peer_id: "test".into(),
            multiaddrs: vec![],
            size_bytes: 1,
            sha256: "test".into(),
            available_until: chrono::Utc::now() + chrono::Duration::minutes(1),
        };

        assert!(service
            .fetch(&reference, std::path::Path::new("unused"))
            .await
            .is_err());
        let activation = slot.activate(Arc::new(SuccessfulDataset)).unwrap();
        assert!(slot.activate(Arc::new(SuccessfulDataset)).is_err());
        service
            .fetch(&reference, std::path::Path::new("unused"))
            .await
            .unwrap();

        drop(activation);
        assert!(service
            .fetch(&reference, std::path::Path::new("unused"))
            .await
            .is_err());
    }

    #[test]
    fn p2p_multiaddrs_are_explicit_tcp_and_advertised_addresses_are_dialable() {
        let listen = parse_p2p_multiaddrs(
            &["/ip4/127.0.0.1/tcp/0".into()],
            "AUKI_P2P_LISTEN_MULTIADDRS",
        )
        .unwrap();
        assert_eq!(listen.len(), 1);
        assert!(parse_p2p_multiaddrs(
            &["/ip4/127.0.0.1/udp/41001".into()],
            "AUKI_P2P_LISTEN_MULTIADDRS"
        )
        .is_err());

        let reachable = ["/ip4/192.0.2.10/tcp/41001".parse::<Multiaddr>().unwrap()];
        assert!(validate_advertised_p2p_multiaddrs(&reachable).is_ok());
        let ephemeral = ["/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap()];
        assert!(validate_advertised_p2p_multiaddrs(&ephemeral).is_err());
        let unspecified = ["/ip4/0.0.0.0/tcp/41001".parse::<Multiaddr>().unwrap()];
        assert!(validate_advertised_p2p_multiaddrs(&unspecified).is_err());
    }

    #[tokio::test]
    async fn production_p2p_start_rejects_a_missing_identity_before_network_io() {
        let mut robot = RobotNodeConfig::new(
            "https://dds.example.test".parse().unwrap(),
            "https://dms.example.test/v1".parse().unwrap(),
            "opaque-registration-credential",
        )
        .unwrap();
        robot.auki_p2p_enabled = true;
        let runtime = robot.runtime_config();

        let error = prepare_peer_identity(&runtime)
            .err()
            .expect("missing production identity must fail before DDS access");
        assert!(error.to_string().contains("P2P_PRIVATE_KEY"));
    }

    #[test]
    fn robot_p2p_address_requirements_are_mode_sensitive() {
        let mut robot = RobotNodeConfig::new(
            "https://dds.example.test".parse().unwrap(),
            "https://dms.example.test/v1".parse().unwrap(),
            "opaque-registration-credential",
        )
        .unwrap();
        robot.auki_p2p_enabled = true;
        let identity = Identity::from_ed25519_seed(&[0x53; 32]);
        robot.set_p2p_private_key(Some(
            P2pPrivateKey::from_protobuf_encoding(identity.to_protobuf_encoding().unwrap())
                .unwrap(),
        ));

        let disabled = robot.runtime_config();
        assert!(validate_robot_p2p_config(&disabled, RelayMode::Disabled).is_err());

        for mode in [RelayMode::Auto, RelayMode::Always] {
            robot
                .set_relay_booking_config(
                    RelayBookingConfig::new(
                        mode,
                        RelayBookingMode::Public,
                        300,
                        1,
                        StdDuration::from_secs(5),
                    )
                    .unwrap(),
                )
                .unwrap();
            let runtime = robot.runtime_config();
            assert!(runtime.auki_p2p_enabled);
            assert!(validate_robot_p2p_config(&runtime, mode).is_ok());
        }
    }

    #[test]
    fn facade_config_maps_compute_to_direct_only_and_robot_to_sdk_relay() {
        let mut robot = RobotNodeConfig::new(
            "https://dds.example.test".parse().unwrap(),
            "https://dms.example.test/v1".parse().unwrap(),
            "opaque-registration-credential",
        )
        .unwrap();
        robot.auki_p2p_enabled = true;
        let identity = Identity::from_ed25519_seed(&[0x47; 32]);
        robot.set_p2p_private_key(Some(
            P2pPrivateKey::from_protobuf_encoding(identity.to_protobuf_encoding().unwrap())
                .unwrap(),
        ));
        let relay = RelayBookingConfig::new(
            RelayMode::Auto,
            RelayBookingMode::Dedicated,
            600,
            2,
            StdDuration::from_secs(7),
        )
        .unwrap();
        robot.set_relay_booking_config(relay).unwrap();
        let runtime = robot.runtime_config();

        let compute = peer_facade_config(&runtime, PeerRuntimeKind::Compute, None).unwrap();
        assert_eq!(compute.app_id(), "posemesh-compute-node");
        assert!(!compute.relay_required());

        let robot = peer_facade_config(&runtime, PeerRuntimeKind::Robot, Some(relay)).unwrap();
        assert_eq!(robot.app_id(), "posemesh-robot-node");
        assert_eq!(
            robot.relay(),
            Some(
                AukiRelayConfig::new(
                    AukiRelayMode::Dedicated,
                    2,
                    StdDuration::from_secs(600),
                    StdDuration::from_secs(7),
                )
                .unwrap()
            )
        );
    }

    #[tokio::test]
    async fn dropping_a_lease_execution_aborts_its_heartbeat_task() {
        let runner_cancel = CancellationToken::new();
        let heartbeat_shutdown = CancellationToken::new();
        let heartbeat = tokio::spawn(std::future::pending::<()>());
        let guard = LeaseExecutionGuard {
            runner_cancel: runner_cancel.clone(),
            heartbeat_shutdown: heartbeat_shutdown.clone(),
            heartbeat_abort: heartbeat.abort_handle(),
        };

        drop(guard);

        assert!(runner_cancel.is_cancelled());
        assert!(heartbeat_shutdown.is_cancelled());
        let error = tokio::time::timeout(StdDuration::from_secs(1), heartbeat)
            .await
            .expect("heartbeat abort is bounded")
            .expect_err("heartbeat task was aborted");
        assert!(error.is_cancelled());
    }
}
