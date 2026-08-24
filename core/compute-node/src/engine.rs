use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use auki_p2p::{Multiaddr, Protocol};
use compute_runner_api::{
    ArtifactSink, ControlPlane, InputSource, LeaseEnvelope, P2pDataset, Runner, TaskCtx,
};
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

use crate::relay_booking::{
    NodeRelayReservationBackend, RelayBookingCoordinator, RelayCoordinatorConfig,
    RelayCoordinatorHealth, RelayPollingGate,
};
use crate::{
    auth::token_manager::{TokenProvider, AUTH_FAILURE_BACKOFF},
    config::{NodeConfig, RelayMode, RobotNodeConfig},
    dds::p2p::{ProcessP2p, RobotP2pTokenProvider},
    dms::{
        client::DmsClient,
        relay::{RelayBookingClient, RelayIdempotencyKey},
    },
    heartbeat::{progress_channel, ProgressReceiver, ProgressSender},
    p2p_dataset::{DatasetRoutePolicy, P2pDatasetAdapter, P2pDatasetServer},
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
        self.run_for_lease_with_p2p(lease, input, output, ctrl, access_token, None)
            .await
    }

    pub async fn run_for_lease_with_p2p(
        &self,
        lease: &LeaseEnvelope,
        input: &dyn InputSource,
        output: &dyn ArtifactSink,
        ctrl: &dyn ControlPlane,
        access_token: &dyn compute_runner_api::runner::AccessTokenProvider,
        p2p_dataset: Option<&dyn P2pDataset>,
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
            p2p_dataset,
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
    signal_task.abort();
    let _ = signal_task.await;

    result
}

pub async fn run_node_with_shutdown(
    cfg: crate::config::NodeConfig,
    runners: RunnerRegistry,
    shutdown: CancellationToken,
) -> Result<()> {
    let process_p2p = start_process_p2p(&cfg, DatasetRoutePolicy::DirectOnly).await?;
    let peer_binding = process_p2p
        .as_ref()
        .map(|runtime| runtime.process.binding_client());
    let siwe =
        match crate::auth::SiweAfterRegistration::from_config_with_peer_binding(&cfg, peer_binding)
        {
            Ok(siwe) => siwe,
            Err(error) => {
                shutdown_process_p2p(process_p2p).await;
                return Err(error);
            }
        };
    info!("DDS SIWE authentication configured; waiting for DDS registration");
    let siwe_handle = tokio::select! {
        result = siwe.start() => match result {
            Ok(handle) => handle,
            Err(error) => {
                siwe.shutdown().await;
                shutdown_process_p2p(process_p2p).await;
                return Err(error);
            }
        },
        _ = shutdown.cancelled() => {
            siwe.shutdown().await;
            shutdown_process_p2p(process_p2p).await;
            info!("Shutdown signal received before SIWE authentication completed");
            return Ok(());
        }
    };
    info!("DDS SIWE token manager started");

    let auth: Arc<dyn TokenProvider> = Arc::new(siwe_handle.clone());
    let dataset = process_p2p
        .as_ref()
        .map(|runtime| Arc::clone(&runtime.dataset));
    let result = run_authenticated_node_loop(
        &cfg,
        &runners,
        auth,
        shutdown,
        "SIWE",
        cfg.auki_p2p_enabled,
        NodeLoopP2p {
            dataset,
            install_task_credentials: cfg.auki_p2p_enabled,
            relay_polling_gate: None,
            forced_shutdown: None,
        },
    )
    .await;

    siwe_handle.shutdown().await;
    shutdown_process_p2p(process_p2p).await;
    info!("Shutdown signal received; exiting run_node loop");

    result
}

/// Run a robot-authenticated node until interrupted.
pub async fn run_robot_node(cfg: RobotNodeConfig, runners: RunnerRegistry) -> Result<()> {
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

    let result =
        run_robot_node_with_shutdowns(cfg, runners, shutdown.clone(), forced_shutdown.clone())
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
    runners: RunnerRegistry,
    shutdown: CancellationToken,
) -> Result<()> {
    run_robot_node_with_shutdowns(cfg, runners, shutdown, CancellationToken::new()).await
}

/// Run a robot-authenticated node with separate graceful and forced shutdown
/// signals.
///
/// The first token stops task polling and begins the bounded dataset-reference
/// drain. The second token interrupts that drain and tears down P2P authority.
pub async fn run_robot_node_with_shutdowns(
    cfg: RobotNodeConfig,
    runners: RunnerRegistry,
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
    // External shutdown stops task polling first. Dataset serving, endpoint
    // credentials, booking renewal, and reservations live on this separate
    // token until the graceful reference drain is complete.
    let lifecycle = CancellationToken::new();
    if shutdown.is_cancelled() {
        return Ok(());
    }
    let process_p2p = tokio::select! {
        biased;
        result = start_process_p2p(&runtime_cfg, route_policy) => result?,
        _ = shutdown.cancelled() => {
            info!("Shutdown signal received before process P2P startup completed");
            return Ok(());
        }
    };
    if shutdown.is_cancelled() {
        shutdown_process_p2p(process_p2p).await;
        return Ok(());
    }
    let peer_binding = process_p2p
        .as_ref()
        .map(|runtime| runtime.process.binding_client());
    let robot = match crate::auth::RobotMachineAuth::from_config_with_peer_binding(
        &cfg,
        runners.capabilities(),
        peer_binding,
    ) {
        Ok(robot) => robot,
        Err(error) => {
            shutdown_process_p2p(process_p2p).await;
            return Err(error);
        }
    };
    info!("DDS robot authentication configured");
    let robot_handle = tokio::select! {
        result = robot.start() => match result {
            Ok(handle) => handle,
            Err(error) => {
                robot.shutdown().await;
                shutdown_process_p2p(process_p2p).await;
                return Err(error);
            }
        },
        _ = shutdown.cancelled() => {
            robot.shutdown().await;
            shutdown_process_p2p(process_p2p).await;
            info!("Shutdown signal received before robot authentication completed");
            return Ok(());
        }
    };
    info!("DDS robot token manager started");

    let auth: Arc<dyn TokenProvider> = Arc::new(robot_handle.clone());
    let mut robot_p2p = if let Some(runtime) = process_p2p.as_ref() {
        let start = RobotP2pTokenProvider::start(
            runtime.process.dds_client(),
            Arc::clone(&auth),
            runtime.process.credentials(),
            &lifecycle,
        );
        let provider = tokio::select! {
            result = start => match result {
                Ok(provider) => provider,
                Err(error) => {
                    robot_handle.shutdown().await;
                    shutdown_process_p2p(process_p2p).await;
                    return Err(error.into());
                }
            },
            _ = shutdown.cancelled() => {
                robot_handle.shutdown().await;
                shutdown_process_p2p(process_p2p).await;
                return Ok(());
            }
        };
        Some(provider)
    } else {
        None
    };
    let dataset_server =
        if let (Some(runtime), Some(provider)) = (process_p2p.as_ref(), robot_p2p.as_ref()) {
            let start = runtime
                .dataset
                .start_serving(provider.domain_id(), &lifecycle);
            let server = tokio::select! {
                result = start => match result {
                    Ok(server) => server,
                    Err(error) => {
                        if let Some(provider) = robot_p2p.take() {
                            provider.shutdown().await;
                        }
                        robot_handle.shutdown().await;
                        shutdown_process_p2p(process_p2p).await;
                        return Err(error.into());
                    }
                },
                _ = shutdown.cancelled() => {
                    if let Some(provider) = robot_p2p.take() {
                        provider.shutdown().await;
                    }
                    robot_handle.shutdown().await;
                    shutdown_process_p2p(process_p2p).await;
                    return Ok(());
                }
            };
            Some(server)
        } else {
            None
        };
    let dataset = process_p2p
        .as_ref()
        .map(|runtime| Arc::clone(&runtime.dataset));
    let relay_coordinator_result: Result<Option<RelayBookingCoordinator>> = async {
        if !relay_config.mode().is_enabled() {
            return Ok(None);
        }
        let runtime = process_p2p
            .as_ref()
            .ok_or_else(|| anyhow!("relay booking requires an active P2P runtime"))?;
        let dataset = Arc::clone(&runtime.dataset);
        let api: Arc<dyn crate::dms::relay::RelayBookingApi> = Arc::new(RelayBookingClient::new(
            runtime_cfg.dms_base_url.clone(),
            Arc::clone(&auth),
        )?);
        let backend: Arc<dyn crate::relay_booking::RelayReservationBackend> =
            Arc::new(NodeRelayReservationBackend::new(runtime.process.node()));
        let routes: Arc<dyn crate::relay_booking::RelayRouteRegistry> = dataset;
        let coordinator_config = RelayCoordinatorConfig {
            idempotency_key: RelayIdempotencyKey::new(format!("robot-relay-{}", Uuid::new_v4()))?,
            mode: relay_config.booking_mode(),
            requested_duration_seconds: relay_config.requested_duration_seconds(),
            relay_count: relay_config.relay_count(),
            status_poll_interval: relay_config.status_poll_interval(),
            reservation_retry_budget: relay_config.reservation_retry_budget(),
            retry_min: relay_config.retry_jitter_min(),
            retry_max: relay_config.retry_jitter_max(),
            http_timeout: relay_config.http_timeout(),
            authority_safety_margin: relay_config.authority_deadline_safety_margin(),
            gate_task_polling: relay_polling_required(
                relay_config.mode(),
                runtime_cfg.auki_p2p_advertised_multiaddrs.is_empty(),
            ),
        };
        start_relay_coordinator(api, backend, routes, coordinator_config, &shutdown)
            .await
            .map(Some)
    }
    .await;
    let mut relay_coordinator = match relay_coordinator_result {
        Ok(coordinator) => coordinator,
        Err(error) => {
            lifecycle.cancel();
            shutdown_dataset_server(dataset_server).await;
            if let Some(provider) = robot_p2p.take() {
                provider.shutdown().await;
            }
            robot_handle.shutdown().await;
            shutdown_process_p2p(process_p2p).await;
            if shutdown.is_cancelled() {
                info!("Shutdown signal received during relay booking startup");
                return Ok(());
            }
            return Err(error);
        }
    };
    let relay_polling_gate = relay_coordinator
        .as_ref()
        .map(RelayBookingCoordinator::polling_gate);
    let mut relay_health = relay_coordinator
        .as_ref()
        .map(RelayBookingCoordinator::health);
    let registration_stop_task = dataset.as_ref().map(|dataset| {
        let dataset = Arc::clone(dataset);
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            shutdown.cancelled().await;
            if let Err(error) = dataset.stop_registrations() {
                warn!(error = %error, "failed to close dataset registration at drain start");
            }
        })
    });
    let loop_shutdown = shutdown.child_token();
    let node_loop = run_authenticated_node_loop(
        &runtime_cfg,
        &runners,
        auth,
        loop_shutdown.clone(),
        "robot",
        true,
        NodeLoopP2p {
            dataset: dataset.clone(),
            install_task_credentials: false,
            relay_polling_gate,
            forced_shutdown: Some(forced_shutdown.clone()),
        },
    );
    tokio::pin!(node_loop);
    let (mut result, mut coordinator_failed) = if let Some(health) = relay_health.as_mut() {
        tokio::select! {
            result = &mut node_loop => (result, false),
            _ = health.failed() => {
                if let Some(dataset) = dataset.as_ref() {
                    let _ = dataset.stop_registrations();
                }
                forced_shutdown.cancel();
                loop_shutdown.cancel();
                let _ = (&mut node_loop).await;
                (
                    Err(anyhow!("relay booking coordinator stopped unexpectedly")),
                    true,
                )
            },
        }
    } else {
        (node_loop.await, false)
    };
    if let Some(task) = registration_stop_task {
        task.abort();
        let _ = task.await;
    }

    if !coordinator_failed
        && relay_health
            .as_ref()
            .is_some_and(RelayCoordinatorHealth::is_failed)
    {
        coordinator_failed = true;
        result = Err(anyhow!("relay booking coordinator stopped unexpectedly"));
    }
    if !coordinator_failed {
        if let Some(dataset) = dataset.as_ref() {
            let mut drain_health_failed = false;
            let drain_result = if let Some(health) = relay_health.as_mut() {
                tokio::select! {
                    biased;
                    _ = health.failed() => {
                        drain_health_failed = true;
                        Err(anyhow!("relay booking coordinator stopped during dataset drain"))
                    }
                    result = drain_dataset_references(dataset, &forced_shutdown) => result,
                }
            } else {
                drain_dataset_references(dataset, &forced_shutdown).await
            };
            if drain_health_failed {
                coordinator_failed = true;
                result = Err(anyhow!(
                    "relay booking coordinator stopped during dataset drain"
                ));
            } else if let Err(error) = drain_result {
                warn!(error = %error, "forced Robot shutdown interrupted the dataset drain; published references may be lost");
                lifecycle.cancel();
                if result.is_ok() {
                    result = Err(error
                        .context("forced Robot shutdown interrupted the dataset reference drain"));
                }
            }
        }
    }
    if coordinator_failed {
        warn!("forcing Robot P2P shutdown after relay coordinator failure; published references may be lost");
        lifecycle.cancel();
    }
    if let Some(coordinator) = relay_coordinator.take() {
        if let Err(error) = coordinator.shutdown(true).await {
            warn!(error = %error, "exact relay booking shutdown failed");
            if result.is_ok() {
                result = Err(anyhow!("exact relay booking shutdown failed: {error}"));
            }
        }
    }
    lifecycle.cancel();
    shutdown_dataset_server(dataset_server).await;
    if let Some(provider) = robot_p2p {
        provider.shutdown().await;
    }
    robot_handle.shutdown().await;
    shutdown_process_p2p(process_p2p).await;
    info!("Shutdown signal received; exiting robot node loop");

    result
}

struct ProcessP2pRuntime {
    process: ProcessP2p,
    dataset: Arc<P2pDatasetAdapter>,
}

struct NodeLoopP2p {
    dataset: Option<Arc<P2pDatasetAdapter>>,
    install_task_credentials: bool,
    relay_polling_gate: Option<RelayPollingGate>,
    forced_shutdown: Option<CancellationToken>,
}

async fn start_process_p2p(
    cfg: &NodeConfig,
    route_policy: DatasetRoutePolicy,
) -> Result<Option<ProcessP2pRuntime>> {
    if !cfg.auki_p2p_enabled {
        return Ok(None);
    }
    let dds_base_url = cfg
        .dds_base_url
        .clone()
        .ok_or_else(|| anyhow!("DDS_BASE_URL required when AUKI_P2P_ENABLED=true"))?;
    let listen_addresses = parse_p2p_multiaddrs(
        &cfg.auki_p2p_listen_multiaddrs,
        "AUKI_P2P_LISTEN_MULTIADDRS",
    )?;
    let advertised_addresses = parse_p2p_multiaddrs(
        &cfg.auki_p2p_advertised_multiaddrs,
        "AUKI_P2P_ADVERTISED_MULTIADDRS",
    )?;
    validate_advertised_p2p_multiaddrs(&advertised_addresses)?;
    let process = ProcessP2p::start_with_listen_addresses(
        dds_base_url,
        StdDuration::from_secs(cfg.request_timeout_secs.max(1)),
        listen_addresses,
    )
    .await
    .context("start process P2P identity")?;
    info!(peer_id = %process.binding_client().peer_id(), "Auki P2P identity started");
    let dataset = Arc::new(P2pDatasetAdapter::new_with_route_policy(
        process.node(),
        process.credentials(),
        advertised_addresses,
        route_policy,
    )?);
    Ok(Some(ProcessP2pRuntime { process, dataset }))
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

fn relay_polling_required(mode: RelayMode, direct_routes_empty: bool) -> bool {
    mode == RelayMode::Always || (mode == RelayMode::Auto && direct_routes_empty)
}

async fn shutdown_dataset_server(server: Option<P2pDatasetServer>) {
    if let Some(server) = server {
        if let Err(error) = server.shutdown().await {
            warn!(error = %error, "Auki P2P dataset server shutdown failed");
        }
    }
}

async fn shutdown_process_p2p(runtime: Option<ProcessP2pRuntime>) {
    if let Some(runtime) = runtime {
        if let Err(error) = runtime.process.shutdown().await {
            warn!(error = %error, "Auki P2P shutdown failed");
        }
    }
}

async fn start_relay_coordinator(
    api: Arc<dyn crate::dms::relay::RelayBookingApi>,
    backend: Arc<dyn crate::relay_booking::RelayReservationBackend>,
    routes: Arc<dyn crate::relay_booking::RelayRouteRegistry>,
    config: RelayCoordinatorConfig,
    shutdown: &CancellationToken,
) -> Result<RelayBookingCoordinator> {
    loop {
        let result = tokio::select! {
            biased;
            result = RelayBookingCoordinator::start(
                Arc::clone(&api),
                Arc::clone(&backend),
                Arc::clone(&routes),
                config.clone(),
            ) => result,
            _ = shutdown.cancelled() => {
                return Err(anyhow!("shutdown interrupted relay booking startup"));
            }
        };
        match result {
            Ok(coordinator) => return Ok(coordinator),
            Err(error) => {
                let Some(delay) = error.startup_retry_after(config.status_poll_interval) else {
                    return Err(error.into());
                };
                warn!(error = %error, ?delay, "relay booking startup is fenced; waiting for prior authority");
                tokio::select! {
                    _ = shutdown.cancelled() => return Err(error.into()),
                    _ = sleep(delay) => {}
                }
            }
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
        dataset,
        install_task_credentials,
        mut relay_polling_gate,
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

        if let Some(gate) = relay_polling_gate.as_mut() {
            if gate.wait(&shutdown).await.is_err() {
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
                        dataset.clone(),
                        install_task_credentials,
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
                        dataset.clone(),
                        install_task_credentials,
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

pub async fn apply_p2p_credential_update(
    credentials: Option<&crate::dds::p2p::P2pCredentialStore>,
    token: Option<&str>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<()> {
    match credentials {
        Some(credentials) => {
            credentials
                .install_optional(token, expires_at)
                .await
                .context("install DDS P2P task credential")?;
            Ok(())
        }
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
    run_leased_cycle_with_dms(cfg, dms, reg, acquired, None, false).await
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
    p2p_dataset: Option<Arc<P2pDatasetAdapter>>,
    install_task_p2p_credentials: bool,
) -> Result<bool> {
    let credentials = if install_task_p2p_credentials {
        p2p_dataset.as_ref().map(|dataset| dataset.credentials())
    } else {
        None
    };
    if let Some(credentials) = &credentials {
        credentials.clear().await;
    }
    let result = run_leased_cycle_inner(
        cfg,
        dms,
        reg,
        acquired,
        p2p_dataset.as_deref(),
        credentials.clone(),
    )
    .await;
    if let Some(credentials) = credentials {
        credentials.clear().await;
    }
    result
}

async fn run_leased_cycle_with_abort(
    cfg: &crate::config::NodeConfig,
    dms: &DmsClient,
    reg: &RunnerRegistry,
    acquired: AcquiredLease,
    p2p_dataset: Option<Arc<P2pDatasetAdapter>>,
    install_task_p2p_credentials: bool,
    forced_shutdown: Option<&CancellationToken>,
) -> Result<bool> {
    let cycle = run_leased_cycle_with_dms(
        cfg,
        dms,
        reg,
        acquired,
        p2p_dataset,
        install_task_p2p_credentials,
    );
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
    p2p_dataset: Option<&P2pDatasetAdapter>,
    p2p_credentials: Option<crate::dds::p2p::P2pCredentialStore>,
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

    if let Err(error) = apply_p2p_credential_update(
        p2p_credentials.as_ref(),
        lease.p2p_access_token.as_deref(),
        lease.p2p_access_token_expires_at,
    )
    .await
    {
        if let Err(fail_error) = report_setup_failure("p2p_lease_token", &error).await {
            warn!(error = %fail_error, task_id = %task_id, "failed to report P2P setup failure");
            return Err(error);
        }
        return Ok(true);
    }

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
    if let Err(error) = apply_p2p_credential_update(
        p2p_credentials.as_ref(),
        heartbeat_initial.p2p_access_token.as_deref(),
        heartbeat_initial.p2p_access_token_expires_at,
    )
    .await
    {
        if let Err(fail_error) = report_setup_failure("p2p_heartbeat_token", &error).await {
            warn!(error = %fail_error, task_id = %task_id, "failed to report P2P heartbeat setup failure");
            return Err(error);
        }
        return Ok(true);
    }
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

    let ports = match crate::storage::build_ports(&lease, token_ref.clone()) {
        Ok(ports) => ports,
        Err(err) => {
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
            p2p_credentials: p2p_credentials.clone(),
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

    let run_res = reg
        .run_for_lease_with_p2p(
            &lease,
            &*ports.input,
            &*ports.output,
            &ctrl,
            &token_ref,
            p2p_dataset.map(|dataset| dataset as &dyn P2pDataset),
        )
        .await;

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
    pub p2p_credentials: Option<crate::dds::p2p::P2pCredentialStore>,
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
    p2p_credentials: Option<crate::dds::p2p::P2pCredentialStore>,
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
            p2p_credentials: args.p2p_credentials,
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
                if let Err(error) = apply_p2p_credential_update(
                    self.p2p_credentials.as_ref(),
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
    use crate::config::{RelayBookingConfig, RelayBookingMode, RobotNodeConfig};

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

    #[test]
    fn robot_p2p_address_requirements_are_mode_sensitive() {
        let mut robot = RobotNodeConfig::new(
            "https://dds.example.test".parse().unwrap(),
            "https://dms.example.test/v1".parse().unwrap(),
            "opaque-registration-credential",
        )
        .unwrap();
        robot.auki_p2p_enabled = true;

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
    fn relay_polling_gate_matches_mode_and_direct_route_policy() {
        assert!(!relay_polling_required(RelayMode::Disabled, true));
        assert!(!relay_polling_required(RelayMode::Auto, false));
        assert!(relay_polling_required(RelayMode::Auto, true));
        assert!(relay_polling_required(RelayMode::Always, false));
        assert!(relay_polling_required(RelayMode::Always, true));
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
