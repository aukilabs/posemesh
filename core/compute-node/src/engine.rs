use crate::{
    config::{NodeConfig, RobotNodeConfig},
    dms::client::DmsClient,
};
use anyhow::{anyhow, Context, Result};
use auki_p2p::{Multiaddr, Protocol};
use auki_sdk::{AukiPeerConfig, AukiPeerProtocolContext, AukiPeerStatus, AukiRelayConfig};
use compute_runner_api::{ArtifactSink, ControlPlane, InputSource, LeaseEnvelope, Runner, TaskCtx};
use parking_lot::RwLock as SyncRwLock;
use std::{collections::HashMap, sync::Arc};
use tokio_util::sync::CancellationToken;
mod sdk;

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
#[derive(Clone, Default)]
struct RunnerDependencies {
    protocols: Option<AukiProtocolsHandle>,
}

impl RunnerDependencies {
    fn require_protocols(&self) -> Result<AukiProtocolsHandle> {
        self.protocols
            .clone()
            .context("the authenticated P2P protocol surface is unavailable")
    }
}

/// Lazily populated handle to the host's authenticated peer protocol context.
///
/// Runners receive this handle through [`RunnerComposition::with_protocols`]
/// and mount their application protocols on the same peer identity. The context
/// provides protocol registration/opening, published routes, Peer ID and Domain.
///
/// Robot populates the handle after its fixed-Domain peer starts. Compute
/// populates it before dispatching a task with P2P authority and clears it when
/// that task ends, including cancellation and errors. Call `get()` inside the
/// runner for each task; a context retained from an earlier task is stopped.
#[derive(Clone, Default)]
pub struct AukiProtocolsHandle {
    state: Arc<SyncRwLock<Option<AukiPeerProtocolContext>>>,
}

impl AukiProtocolsHandle {
    fn activate(&self, context: AukiPeerProtocolContext) {
        *self.state.write() = Some(context);
    }

    fn activate_task(&self, context: AukiPeerProtocolContext) -> Result<TaskProtocolActivation> {
        let mut state = self.state.write();
        if state.is_some() {
            return Err(anyhow!("the task peer protocol surface is already active"));
        }
        *state = Some(context);
        Ok(TaskProtocolActivation(self.clone()))
    }

    /// Build an already activated handle from a real or test-fixture peer context.
    pub fn for_testing(context: AukiPeerProtocolContext) -> Self {
        let handle = Self::default();
        handle.activate(context);
        handle
    }

    /// Obtain the current peer context. Compute has no context between tasks
    /// or for a task whose lease has no P2P authority.
    pub fn get(&self) -> Result<AukiPeerProtocolContext> {
        self.state
            .read()
            .clone()
            .context("the authenticated P2P protocol surface is unavailable")
    }
}

struct TaskProtocolActivation(AukiProtocolsHandle);

impl Drop for TaskProtocolActivation {
    fn drop(&mut self) {
        self.0.state.write().take();
    }
}

fn peer_facade_config(cfg: &NodeConfig, relay: Option<AukiRelayConfig>) -> Result<AukiPeerConfig> {
    let listen_addresses = parse_p2p_multiaddrs(
        &cfg.auki_p2p_listen_multiaddrs,
        "AUKI_P2P_LISTEN_MULTIADDRS",
    )?;
    let direct_routes = parse_p2p_multiaddrs(
        &cfg.auki_p2p_advertised_multiaddrs,
        "AUKI_P2P_ADVERTISED_MULTIADDRS",
    )?;
    let config =
        AukiPeerConfig::new(cfg.dms_base_url.as_str())?.with_listen_addresses(listen_addresses)?;
    let config = match relay {
        Some(relay) => config.with_relay(relay)?,
        None => config.direct_only(),
    };
    Ok(config.with_advertised_direct_routes(direct_routes)?)
}

/// Builds the runner registry with the host's protocol handles.
///
/// Plain registries convert into a fixed composition automatically. Applications
/// that mount their own protocols use [`RunnerComposition::with_protocols`].
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

    /// Construct runners that require the general authenticated peer
    /// protocol surface to mount protocols or open streams on the host's peer.
    pub fn with_protocols<F>(build: F) -> Self
    where
        F: FnOnce(AukiProtocolsHandle) -> RunnerRegistry + Send + 'static,
    {
        Self::new(move |dependencies| Ok(build(dependencies.require_protocols()?)))
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

/// Run a compute host with DDS authentication, DMS tasks, and lease-backed storage.
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
    cfg: NodeConfig,
    runners: impl Into<RunnerComposition> + Send,
    shutdown: CancellationToken,
) -> Result<()> {
    sdk::run_compute(cfg, runners.into(), shutdown).await
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
/// The first token stops task polling and lets the active task finish.
/// The second token interrupts an active task before peer shutdown.
pub async fn run_robot_node_with_shutdowns(
    cfg: RobotNodeConfig,
    runners: impl Into<RunnerComposition> + Send,
    shutdown: CancellationToken,
    forced_shutdown: CancellationToken,
) -> Result<()> {
    sdk::run_robot(cfg, runners.into(), shutdown, forced_shutdown).await
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

fn validate_robot_p2p_config(cfg: &NodeConfig, relay_enabled: bool) -> Result<()> {
    if cfg.auki_p2p_enabled && cfg.auki_p2p_private_key.is_none() {
        return Err(anyhow!(
            "AUKI_P2P_PRIVATE_KEY_FILE or AUKI_P2P_PRIVATE_KEY required when P2P is enabled"
        ));
    }
    if cfg.auki_p2p_enabled && !relay_enabled && cfg.auki_p2p_listen_multiaddrs.is_empty() {
        return Err(anyhow!(
            "AUKI_P2P_LISTEN_MULTIADDRS required for Robot P2P serving"
        ));
    }
    if cfg.auki_p2p_enabled && !relay_enabled && cfg.auki_p2p_advertised_multiaddrs.is_empty() {
        return Err(anyhow!(
            "AUKI_P2P_ADVERTISED_MULTIADDRS required for Robot P2P serving"
        ));
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
            AukiPeerStatus::AuthorityUnavailable | AukiPeerStatus::RelayUnavailable => {}
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

/// Run one task through the managed SDK lifecycle with host-owned authentication.
pub async fn run_cycle_with_dms(
    cfg: &NodeConfig,
    dms: &DmsClient,
    runners: &RunnerRegistry,
) -> Result<bool> {
    sdk::run_cycle(cfg, dms, runners).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{P2pPrivateKey, RobotNodeConfig};
    use auki_p2p::Identity;
    use auki_sdk::AukiRelayMode;
    use std::time::Duration as StdDuration;
    #[test]
    fn protocol_runner_composition_fails_without_p2p_runtime() {
        let result = RunnerComposition::with_protocols(|_| RunnerRegistry::new())
            .compose(RunnerDependencies::default());
        let error = result.err().expect("missing P2P must fail composition");

        assert!(error
            .to_string()
            .contains("authenticated P2P protocol surface is unavailable"));
    }

    #[test]
    fn p2p_listener_multiaddrs_are_explicit_tcp() {
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

        let error = sdk::peer_config(&runtime, None)
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
        assert!(validate_robot_p2p_config(&disabled, false).is_err());

        let relay = AukiRelayConfig::new(
            AukiRelayMode::Public,
            1,
            StdDuration::from_secs(300),
            StdDuration::from_secs(5),
        )
        .unwrap();
        robot.set_relay_config(Some(relay)).unwrap();
        let runtime = robot.runtime_config();
        assert!(runtime.auki_p2p_enabled);
        assert!(validate_robot_p2p_config(&runtime, true).is_ok());
    }

    #[test]
    fn facade_config_maps_relay_selection_to_sdk() {
        let mut robot = RobotNodeConfig::new(
            "https://dds.example.test".parse().unwrap(),
            "https://dms.example.test/v1".parse().unwrap(),
            "opaque-registration-credential",
        )
        .unwrap();
        robot.auki_p2p_enabled = true;
        let relay = AukiRelayConfig::new(
            AukiRelayMode::Dedicated,
            2,
            StdDuration::from_secs(600),
            StdDuration::from_secs(7),
        )
        .unwrap();
        robot.set_relay_config(Some(relay)).unwrap();
        let runtime = robot.runtime_config();

        let direct = peer_facade_config(&runtime, None).unwrap();
        assert!(!direct.relay_required());

        let relayed = peer_facade_config(&runtime, Some(relay)).unwrap();
        assert_eq!(relayed.relay(), Some(relay));
    }

    #[test]
    fn facade_config_delegates_advertised_route_validation_to_sdk() {
        let mut robot = RobotNodeConfig::new(
            "https://dds.example.test".parse().unwrap(),
            "https://dms.example.test/v1".parse().unwrap(),
            "opaque-registration-credential",
        )
        .unwrap();
        robot.auki_p2p_enabled = true;
        robot.auki_p2p_advertised_multiaddrs = vec!["/ip4/0.0.0.0/tcp/41001".into()];

        let error = peer_facade_config(&robot.runtime_config(), None)
            .expect_err("the SDK must reject an unspecified advertised address");
        assert!(error
            .to_string()
            .contains("advertised direct route is invalid"));
    }

    #[test]
    fn facade_config_selects_direct_only_before_applying_full_route_set() {
        let mut robot = RobotNodeConfig::new(
            "https://dds.example.test".parse().unwrap(),
            "https://dms.example.test/v1".parse().unwrap(),
            "opaque-registration-credential",
        )
        .unwrap();
        robot.auki_p2p_enabled = true;
        robot.auki_p2p_advertised_multiaddrs = (1..=16)
            .map(|last_octet| format!("/ip4/192.0.2.{last_octet}/tcp/41001"))
            .collect();

        let config = peer_facade_config(&robot.runtime_config(), None).unwrap();
        assert!(!config.relay_required());
        assert_eq!(config.advertised_direct_routes().len(), 16);
    }

    #[test]
    fn facade_config_accepts_the_sdk_route_capacity_boundary() {
        let mut robot = RobotNodeConfig::new(
            "https://dds.example.test".parse().unwrap(),
            "https://dms.example.test/v1".parse().unwrap(),
            "opaque-registration-credential",
        )
        .unwrap();
        robot.auki_p2p_enabled = true;
        robot.auki_p2p_advertised_multiaddrs = (1..=13)
            .map(|last_octet| format!("/ip4/192.0.2.{last_octet}/tcp/41001"))
            .collect();
        let relay = AukiRelayConfig::new(
            AukiRelayMode::Public,
            3,
            StdDuration::from_secs(300),
            StdDuration::from_secs(5),
        )
        .unwrap();

        let config = peer_facade_config(&robot.runtime_config(), Some(relay)).unwrap();
        assert_eq!(config.advertised_direct_routes().len(), 13);
        assert_eq!(config.relay(), Some(relay));
    }
}
