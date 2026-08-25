use std::{collections::HashMap, future::Future, sync::Arc, time::Duration};

use async_trait::async_trait;
use auki_p2p::{
    ExpectedRelayLimits, Multiaddr, Node, PeerId, RelayConfirmationRejection, RelayProvider,
    RelayReservationHandle, RelayReservationSnapshot, RelayTransportEvent,
};
use rand::Rng;
use tokio::sync::broadcast;
use tokio::{
    sync::{mpsc, oneshot, watch},
    task::JoinHandle,
    time::Instant,
};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use uuid::Uuid;

use crate::dms::relay::{
    CreateRelayBookingRequest, RelayBookingApi, RelayBookingClientError, RelayBookingMode,
    RelayBookingSnapshot, RelayBookingState, RelayErrorCode, RelayIdempotencyKey, RelayOperation,
    RelaySlotState, ReservationFailedRequest, ReservationFailureReason,
};

/// The complete local fence for one child reservation attempt.
///
/// DMS owns the first three components. The process-local generation makes a
/// late result from a canceled attempt harmless even when DMS legitimately
/// returns to the same assignment and epoch later.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct LocalRelayFence {
    pub(crate) slot_id: Uuid,
    pub(crate) assignment_id: Uuid,
    pub(crate) reservation_epoch: Uuid,
    pub(crate) local_generation: u64,
}

#[derive(Debug)]
pub(crate) struct ReservationAttemptFailure {
    pub(crate) handle: Option<RelayReservationHandle>,
    pub(crate) reason: ReservationFailureReason,
    pub(crate) retryable: bool,
}

#[async_trait]
pub(crate) trait RelayReservationBackend: Send + Sync {
    /// Start must be cancellation-safe: dropping this future before a handle
    /// is delivered must roll back any generation minted by the backend.
    async fn start(
        &self,
        provider: RelayProvider,
    ) -> Result<RelayReservationHandle, ReservationAttemptFailure>;

    async fn wait(
        &self,
        handle: RelayReservationHandle,
    ) -> Result<RelayReservationSnapshot, ReservationAttemptFailure>;

    async fn cancel(&self, handle: RelayReservationHandle) -> Result<(), auki_p2p::Error>;

    fn subscribe(&self) -> broadcast::Receiver<RelayTransportEvent>;
}

#[derive(Clone)]
pub(crate) struct NodeRelayReservationBackend {
    node: Node,
}

impl NodeRelayReservationBackend {
    pub(crate) fn new(node: Node) -> Self {
        Self { node }
    }
}

#[async_trait]
impl RelayReservationBackend for NodeRelayReservationBackend {
    async fn start(
        &self,
        provider: RelayProvider,
    ) -> Result<RelayReservationHandle, ReservationAttemptFailure> {
        self.node
            .start_relay_reservation(provider)
            .await
            .map_err(|error| ReservationAttemptFailure {
                handle: None,
                reason: reservation_failure_reason(&error, false),
                retryable: reservation_failure_is_retryable(&error),
            })
    }

    async fn wait(
        &self,
        handle: RelayReservationHandle,
    ) -> Result<RelayReservationSnapshot, ReservationAttemptFailure> {
        self.node
            .wait_relay_reservation(handle)
            .await
            .map_err(|error| ReservationAttemptFailure {
                handle: Some(handle),
                reason: reservation_failure_reason(&error, false),
                retryable: reservation_failure_is_retryable(&error),
            })
    }

    async fn cancel(&self, handle: RelayReservationHandle) -> Result<(), auki_p2p::Error> {
        self.node.cancel_relay_reservation(handle).await
    }

    fn subscribe(&self) -> broadcast::Receiver<RelayTransportEvent> {
        self.node.subscribe_relay_events()
    }
}

fn reservation_failure_reason(
    error: &auki_p2p::Error,
    was_publishable: bool,
) -> ReservationFailureReason {
    use auki_p2p::Error;

    match error {
        Error::RelayDirectConnectionMismatch { .. }
        | Error::InvalidRemoteAddress { .. }
        | Error::InvalidRelayRoute { .. }
        | Error::RelayReservation(
            auki_p2p::RelayReservationError::InvalidBase { .. }
            | auki_p2p::RelayReservationError::BasePeerMismatch { .. }
            | auki_p2p::RelayReservationError::DuplicateBase(_)
            | auki_p2p::RelayReservationError::EmptyBases,
        ) => ReservationFailureReason::AddressMismatch,
        Error::RelayConfirmationRejected(
            RelayConfirmationRejection::MissingLimits
            | RelayConfirmationRejection::IncompleteLimits { .. }
            | RelayConfirmationRejection::LimitMismatch { .. },
        ) => ReservationFailureReason::LimitMismatch,
        Error::RelayReservationClosed(_) if was_publishable => {
            ReservationFailureReason::ReservationLost
        }
        Error::RelayReservationClosed(_) | Error::RelayConfirmationRejected(_) => {
            ReservationFailureReason::ReservationDenied
        }
        Error::Dns(_) | Error::Dial(_) => ReservationFailureReason::DialFailed,
        _ if was_publishable => ReservationFailureReason::ReservationLost,
        _ => ReservationFailureReason::DialFailed,
    }
}

fn reservation_failure_is_retryable(error: &auki_p2p::Error) -> bool {
    matches!(
        error,
        auki_p2p::Error::Dns(_)
            | auki_p2p::Error::Dial(_)
            | auki_p2p::Error::RelayReservationClosed(_)
            | auki_p2p::Error::SwarmStopped
            | auki_p2p::Error::Io(_)
    )
}

pub(crate) fn relay_provider(
    peer_id: &str,
    bases: &[String],
    duration_seconds: u32,
    data_bytes_per_direction: u64,
) -> Result<RelayProvider, auki_p2p::RelayReservationError> {
    let peer_id =
        peer_id
            .parse()
            .map_err(|error| auki_p2p::RelayReservationError::InvalidBase {
                address: peer_id.to_string(),
                reason: format!("invalid provider Peer ID: {error}"),
            })?;
    let limits = ExpectedRelayLimits::new(
        Duration::from_secs(u64::from(duration_seconds)),
        data_bytes_per_direction,
    )?;
    RelayProvider::new(peer_id, bases, limits)
}

pub(crate) fn confirmed_route(snapshot: &RelayReservationSnapshot) -> Option<Multiaddr> {
    snapshot.publishable_route().cloned()
}

pub(crate) type SharedReservationBackend = Arc<dyn RelayReservationBackend>;

#[derive(Clone, Debug)]
pub(crate) struct PublishedRelayRoute {
    pub(crate) fence: LocalRelayFence,
    pub(crate) route: Multiaddr,
    pub(crate) limits: ExpectedRelayLimits,
    pub(crate) authorized_until: chrono::DateTime<chrono::Utc>,
    pub(crate) relay_peer_id: PeerId,
}

#[async_trait]
pub(crate) trait RelayRouteRegistry: Send + Sync {
    async fn publish(&self, route: PublishedRelayRoute) -> Result<(), String>;
    async fn refresh_authority(
        &self,
        fence: LocalRelayFence,
        authorized_until: chrono::DateTime<chrono::Utc>,
    ) -> Result<bool, String>;
    async fn tombstone(&self, fence: LocalRelayFence) -> Result<bool, String>;
}

pub(crate) type SharedRouteRegistry = Arc<dyn RelayRouteRegistry>;

fn route_fence(fence: LocalRelayFence) -> auki_p2p::RouteFence {
    auki_p2p::RouteFence {
        route_id: fence.slot_id,
        authority_id: fence.assignment_id,
        authority_epoch: fence.reservation_epoch,
        local_generation: fence.local_generation,
    }
}

#[async_trait]
impl RelayRouteRegistry for auki_p2p::RouteCatalog {
    async fn publish(&self, route: PublishedRelayRoute) -> Result<(), String> {
        self.publish_confirmed(auki_p2p::ConfirmedRoute {
            fence: route_fence(route.fence),
            relay_peer_id: route.relay_peer_id,
            route: route.route,
            limits: route.limits,
            authorized_until: route.authorized_until,
        })
        .map(|_| ())
        .map_err(|error| error.to_string())
    }

    async fn refresh_authority(
        &self,
        fence: LocalRelayFence,
        authorized_until: chrono::DateTime<chrono::Utc>,
    ) -> Result<bool, String> {
        match self.refresh_authorization(route_fence(fence), authorized_until) {
            Ok(_) => Ok(true),
            Err(
                auki_p2p::RouteCatalogError::RouteNotFound
                | auki_p2p::RouteCatalogError::StaleRouteFence,
            ) => Ok(false),
            Err(error) => Err(error.to_string()),
        }
    }

    async fn tombstone(&self, fence: LocalRelayFence) -> Result<bool, String> {
        match self.tombstone(route_fence(fence)) {
            Ok(_) => Ok(true),
            Err(
                auki_p2p::RouteCatalogError::RouteNotFound
                | auki_p2p::RouteCatalogError::StaleRouteFence,
            ) => Ok(false),
            Err(error) => Err(error.to_string()),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RelayCoordinatorConfig {
    pub(crate) idempotency_key: RelayIdempotencyKey,
    pub(crate) mode: RelayBookingMode,
    pub(crate) requested_duration_seconds: u64,
    pub(crate) relay_count: u8,
    pub(crate) status_poll_interval: Duration,
    pub(crate) reservation_retry_budget: Duration,
    pub(crate) retry_min: Duration,
    pub(crate) retry_max: Duration,
    pub(crate) http_timeout: Duration,
    pub(crate) authority_safety_margin: Duration,
    pub(crate) gate_task_polling: bool,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RelayCoordinatorError {
    #[error("relay booking request failed")]
    Dms(#[from] RelayBookingClientError),
    #[error("the active relay booking does not match this process configuration")]
    ActiveBookingMismatch,
    #[error("relay booking authority ended")]
    AuthorityEnded,
    #[error("relay coordinator task stopped")]
    Stopped,
    #[error("relay route registry rejected a fenced update: {0}")]
    RouteRegistry(String),
    #[error("relay reservation cleanup did not complete: {0}")]
    ReservationCleanup(String),
    #[error("relay reservation event stream lagged by {0} events")]
    RelayEventLagged(u64),
}

impl RelayCoordinatorError {
    pub(crate) fn startup_retry_after(&self, fallback: Duration) -> Option<Duration> {
        let Self::Dms(error) = self else {
            return None;
        };
        if error.is_retryable()
            || matches!(
                error.http_code(),
                Some(
                    RelayErrorCode::StaleRobotPrincipal
                        | RelayErrorCode::ActiveBookingConflict
                        | RelayErrorCode::TargetPeerConflict
                )
            )
        {
            return Some(error.retry_after().unwrap_or(fallback));
        }
        None
    }
}

pub(crate) struct RelayBookingCoordinator {
    commands: mpsc::Sender<CoordinatorCommand>,
    availability: watch::Receiver<usize>,
    health: watch::Receiver<bool>,
    gate_task_polling: bool,
    task: Option<JoinHandle<Result<(), RelayCoordinatorError>>>,
}

impl RelayBookingCoordinator {
    pub(crate) async fn start(
        api: Arc<dyn RelayBookingApi>,
        backend: SharedReservationBackend,
        routes: SharedRouteRegistry,
        config: RelayCoordinatorConfig,
    ) -> Result<Self, RelayCoordinatorError> {
        let active =
            match bounded_control_call(config.http_timeout, RelayOperation::Active, api.active())
                .await
            {
                Ok(active) => active,
                Err(error) if error.http_code() == Some(RelayErrorCode::StaleRobotPrincipal) => {
                    // Create is the expiry-on-access path for a new process Peer
                    // ID and returns the authoritative conflict Retry-After while
                    // the old Robot authority is still live.
                    None
                }
                Err(error) => return Err(error.into()),
            };
        let snapshot = match active {
            Some(snapshot) => {
                validate_booking_matches(&snapshot, &config)?;
                snapshot
            }
            None => {
                let request = CreateRelayBookingRequest::new(
                    config.mode,
                    config.requested_duration_seconds,
                    config.relay_count,
                )?;
                let response = bounded_control_call(
                    config.http_timeout,
                    RelayOperation::Create,
                    api.create(&config.idempotency_key, &request),
                )
                .await?;
                if response.snapshot.state == RelayBookingState::Active {
                    response.snapshot
                } else {
                    let replacement_key =
                        RelayIdempotencyKey::new(format!("robot-relay-{}", Uuid::new_v4()))?;
                    bounded_control_call(
                        config.http_timeout,
                        RelayOperation::Create,
                        api.create(&replacement_key, &request),
                    )
                    .await?
                    .snapshot
                }
            }
        };

        validate_booking_matches(&snapshot, &config)?;
        let (commands, command_rx) = mpsc::channel(32);
        let (events, event_rx) = mpsc::channel(64);
        let (availability_tx, availability) = watch::channel(0_usize);
        let (health_tx, health) = watch::channel(true);
        let relay_events = backend.subscribe();
        let mut actor = CoordinatorActor {
            api,
            backend,
            routes,
            config: config.clone(),
            snapshot,
            slots: HashMap::new(),
            retiring: HashMap::new(),
            pending_relay_failures: HashMap::new(),
            next_generation: 1,
            command_rx,
            events: events.clone(),
            event_rx,
            relay_events,
            availability: availability_tx,
            next_poll: Instant::now(),
            next_renew: Instant::now(),
            next_expiry: Instant::now() + Duration::from_secs(86_400),
            control_fenced: false,
        };
        actor.schedule_renewal();
        actor.apply_current_snapshot().await?;
        actor.schedule_status_poll(false);
        let task = tokio::spawn(async move {
            let _health_guard = CoordinatorHealthGuard(health_tx);
            let result = actor.run().await;
            if result.is_err() {
                let _ = actor.fence_control_plane().await;
            }
            result
        });
        Ok(Self {
            commands,
            availability,
            health,
            gate_task_polling: config.gate_task_polling,
            task: Some(task),
        })
    }

    pub(crate) fn polling_gate(&self) -> RelayPollingGate {
        RelayPollingGate {
            availability: self.availability.clone(),
            required: self.gate_task_polling,
        }
    }

    pub(crate) fn health(&self) -> RelayCoordinatorHealth {
        RelayCoordinatorHealth {
            health: self.health.clone(),
        }
    }

    pub(crate) async fn shutdown(
        mut self,
        delete_booking: bool,
    ) -> Result<(), RelayCoordinatorError> {
        let (response, receiver) = oneshot::channel();
        let command_result = match self
            .commands
            .send(CoordinatorCommand::Stop {
                delete_booking,
                response,
            })
            .await
        {
            Ok(()) => receiver.await.map_err(|_| RelayCoordinatorError::Stopped)?,
            Err(_) => Err(RelayCoordinatorError::Stopped),
        };
        let task_result = match self.task.take() {
            Some(task) => task.await.map_err(|_| RelayCoordinatorError::Stopped)?,
            None => Ok(()),
        };
        task_result.and(command_result)
    }
}

struct CoordinatorHealthGuard(watch::Sender<bool>);

impl Drop for CoordinatorHealthGuard {
    fn drop(&mut self) {
        self.0.send_replace(false);
    }
}

#[derive(Clone)]
pub(crate) struct RelayCoordinatorHealth {
    health: watch::Receiver<bool>,
}

impl RelayCoordinatorHealth {
    pub(crate) fn is_failed(&self) -> bool {
        !*self.health.borrow()
    }

    pub(crate) async fn failed(&mut self) {
        if !*self.health.borrow() {
            return;
        }
        loop {
            if self.health.changed().await.is_err() || !*self.health.borrow() {
                return;
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct RelayPollingGate {
    availability: watch::Receiver<usize>,
    required: bool,
}

impl RelayPollingGate {
    pub(crate) async fn wait(
        &mut self,
        shutdown: &CancellationToken,
    ) -> Result<(), RelayCoordinatorError> {
        if !self.required || *self.availability.borrow() > 0 {
            return Ok(());
        }
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => return Err(RelayCoordinatorError::Stopped),
                changed = self.availability.changed() => {
                    changed.map_err(|_| RelayCoordinatorError::Stopped)?;
                    if *self.availability.borrow() > 0 {
                        return Ok(());
                    }
                }
            }
        }
    }
}

enum CoordinatorCommand {
    Stop {
        delete_booking: bool,
        response: oneshot::Sender<Result<(), RelayCoordinatorError>>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LocalSlotState {
    Reserving(Option<RelayReservationHandle>),
    Confirmed(RelayReservationHandle),
    ReportingFailure(ReservationFailureReason),
}

struct LocalSlot {
    fence: LocalRelayFence,
    relay_peer_id: String,
    provider_base_addresses: Vec<String>,
    limits: ExpectedRelayLimits,
    authorized_until: chrono::DateTime<chrono::Utc>,
    state: LocalSlotState,
    cancel_retry: CancellationToken,
    worker: Option<JoinHandle<Result<Option<RelayReservationHandle>, String>>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RetirementAction {
    Reconcile,
    ReportFailure(ReservationFailureReason),
    Remove,
}

struct RetiringSlot {
    local: LocalSlot,
    action: RetirementAction,
    cleanup: JoinHandle<Result<(), String>>,
}

enum ChildEvent {
    Started {
        fence: LocalRelayFence,
        handle: RelayReservationHandle,
    },
    Confirmed {
        fence: LocalRelayFence,
        snapshot: RelayReservationSnapshot,
    },
    Retrying {
        fence: LocalRelayFence,
        handle: RelayReservationHandle,
    },
    Failed {
        fence: LocalRelayFence,
        handle: Option<RelayReservationHandle>,
        reason: ReservationFailureReason,
    },
    RetryFailure {
        fence: LocalRelayFence,
        reason: ReservationFailureReason,
    },
    CleanupComplete {
        fence: LocalRelayFence,
    },
    DetachedCleanupFailed {
        error: String,
    },
}

struct CoordinatorActor {
    api: Arc<dyn RelayBookingApi>,
    backend: SharedReservationBackend,
    routes: SharedRouteRegistry,
    config: RelayCoordinatorConfig,
    snapshot: RelayBookingSnapshot,
    slots: HashMap<Uuid, LocalSlot>,
    retiring: HashMap<Uuid, RetiringSlot>,
    pending_relay_failures: HashMap<RelayReservationHandle, ReservationFailureReason>,
    next_generation: u64,
    command_rx: mpsc::Receiver<CoordinatorCommand>,
    events: mpsc::Sender<ChildEvent>,
    event_rx: mpsc::Receiver<ChildEvent>,
    relay_events: broadcast::Receiver<RelayTransportEvent>,
    availability: watch::Sender<usize>,
    next_poll: Instant,
    next_renew: Instant,
    next_expiry: Instant,
    control_fenced: bool,
}

impl CoordinatorActor {
    async fn run(&mut self) -> Result<(), RelayCoordinatorError> {
        loop {
            if self.control_fenced {
                return Err(RelayCoordinatorError::AuthorityEnded);
            }
            tokio::select! {
                biased;
                command = self.command_rx.recv() => {
                    let Some(command) = command else { break };
                    match command {
                        CoordinatorCommand::Stop { delete_booking, response } => {
                            let result = self.stop(delete_booking).await;
                            let _ = response.send(result);
                            break;
                        }
                    }
                }
                event = self.relay_events.recv() => {
                    self.handle_relay_receive(event).await?;
                }
                _ = tokio::time::sleep_until(self.next_renew), if !self.control_fenced => {
                    self.renew().await?;
                }
                _ = tokio::time::sleep_until(self.next_expiry), if !self.control_fenced => {
                    self.expire_local_authority().await?;
                }
                event = self.event_rx.recv() => {
                    if let Some(event) = event {
                        self.handle_child_event(event).await?;
                    }
                }
                _ = tokio::time::sleep_until(self.next_poll), if !self.control_fenced => {
                    let protected = self.next_renew.min(self.next_expiry);
                    if protected.saturating_duration_since(Instant::now())
                        <= self.config.http_timeout
                    {
                        self.next_poll = protected + Duration::from_millis(1);
                    } else {
                        self.poll().await?;
                    }
                }
            }
        }
        Ok(())
    }

    fn schedule_renewal(&mut self) {
        let now = chrono::Utc::now();
        let remaining = self
            .snapshot
            .authority_expires_at
            .signed_duration_since(now)
            .to_std()
            .unwrap_or_default();
        let safety = self.config.authority_safety_margin;
        let latest_safe_start = remaining
            .saturating_sub(safety)
            .saturating_sub(self.config.http_timeout);
        let renew_after = remaining
            .mul_f64(rand::random::<f64>() * 0.10 + 0.25)
            .min(latest_safe_start)
            .max(Duration::from_millis(1));
        self.next_renew = Instant::now() + renew_after;
    }

    async fn poll(&mut self) -> Result<(), RelayCoordinatorError> {
        let succeeded = match bounded_control_call(
            self.config.http_timeout,
            RelayOperation::Active,
            self.api.active(),
        )
        .await
        {
            Ok(Some(snapshot)) => {
                self.apply_snapshot(snapshot).await?;
                true
            }
            Ok(None) => {
                warn!("active relay booking disappeared");
                self.fence_control_plane().await?;
                return Err(RelayCoordinatorError::AuthorityEnded);
            }
            Err(error) => {
                warn!(error = %error, "relay booking status poll failed");
                if control_error_ends_authority(&error) {
                    self.fence_control_plane().await?;
                    return Err(RelayCoordinatorError::AuthorityEnded);
                }
                false
            }
        };
        let delay = if succeeded {
            self.status_poll_delay()
        } else {
            retry_jitter(self.config.retry_min, self.config.retry_max)
        };
        self.next_poll = Instant::now() + delay;
        Ok(())
    }

    async fn renew(&mut self) -> Result<(), RelayCoordinatorError> {
        match bounded_control_call(
            self.config.http_timeout,
            RelayOperation::Renew,
            self.api.renew(self.snapshot.booking_id),
        )
        .await
        {
            Ok(snapshot) => {
                self.apply_snapshot(snapshot).await?;
                self.schedule_renewal();
            }
            Err(error) => {
                warn!(error = %error, "relay booking authority renewal failed");
                if control_error_ends_authority(&error) {
                    self.fence_control_plane().await?;
                    return Err(RelayCoordinatorError::AuthorityEnded);
                }
                let now = chrono::Utc::now();
                let remaining = self
                    .snapshot
                    .authority_expires_at
                    .signed_duration_since(now)
                    .to_std()
                    .unwrap_or_default();
                let retry_window = remaining
                    .saturating_sub(self.config.authority_safety_margin)
                    .saturating_sub(self.config.http_timeout);
                if retry_window.is_zero() {
                    self.fence_control_plane().await?;
                    return Err(RelayCoordinatorError::AuthorityEnded);
                }
                let retry_max = self.config.retry_max.min(retry_window);
                let retry_min = self.config.retry_min.min(retry_max);
                self.next_renew = Instant::now() + retry_jitter(retry_min, retry_max);
            }
        }
        Ok(())
    }

    async fn apply_snapshot(
        &mut self,
        snapshot: RelayBookingSnapshot,
    ) -> Result<(), RelayCoordinatorError> {
        if snapshot.booking_id != self.snapshot.booking_id {
            return Err(RelayCoordinatorError::ActiveBookingMismatch);
        }
        if snapshot.state != RelayBookingState::Active {
            self.snapshot = snapshot;
            self.fence_control_plane().await?;
            return Err(RelayCoordinatorError::AuthorityEnded);
        }
        validate_booking_matches(&snapshot, &self.config)?;
        self.snapshot = snapshot;
        self.apply_current_snapshot().await?;
        self.next_poll = Instant::now();
        Ok(())
    }

    async fn apply_current_snapshot(&mut self) -> Result<(), RelayCoordinatorError> {
        if self.snapshot.state != RelayBookingState::Active {
            self.remove_all_slots().await?;
            return Err(RelayCoordinatorError::AuthorityEnded);
        }

        let desired: HashMap<Uuid, _> = self
            .snapshot
            .slots
            .iter()
            .filter(|slot| slot.state == RelaySlotState::Ready)
            .filter_map(|slot| {
                Some((
                    slot.slot_id,
                    (
                        slot.assignment_id?,
                        slot.reservation_epoch?,
                        slot.provider_peer_id.clone()?,
                        slot.provider_base_addresses.clone()?,
                        slot.limits?,
                        slot.provider_lease_expires_at?,
                    ),
                ))
            })
            .collect();

        let stale: Vec<_> = self
            .slots
            .values()
            .filter(|local| {
                desired.get(&local.fence.slot_id).is_none_or(
                    |(assignment, epoch, peer, bases, limits, _)| {
                        *assignment != local.fence.assignment_id
                            || *epoch != local.fence.reservation_epoch
                            || peer != &local.relay_peer_id
                            || bases != &local.provider_base_addresses
                            || local.limits.duration().as_secs()
                                != u64::from(limits.duration_seconds)
                            || local.limits.data_bytes_per_direction()
                                != limits.data_bytes_per_direction
                    },
                )
            })
            .map(|local| local.fence)
            .collect();
        for fence in stale {
            self.begin_retirement(fence, RetirementAction::Reconcile)
                .await?;
        }

        for (slot_id, (assignment_id, reservation_epoch, peer, bases, limits, lease_deadline)) in
            desired
        {
            let literal_deadline = self
                .snapshot
                .requested_until
                .min(self.snapshot.authority_expires_at)
                .min(lease_deadline);
            let authorized_until = literal_deadline
                - chrono::Duration::from_std(self.config.authority_safety_margin)
                    .map_err(|_| RelayCoordinatorError::ActiveBookingMismatch)?;
            if self.retiring.contains_key(&slot_id) {
                continue;
            }
            if authorized_until <= chrono::Utc::now() {
                if let Some(existing) = self.slots.get(&slot_id) {
                    self.begin_retirement(existing.fence, RetirementAction::Remove)
                        .await?;
                }
                continue;
            }
            if let Some(existing) = self.slots.get(&slot_id) {
                let fence = existing.fence;
                match existing.state {
                    LocalSlotState::Confirmed(_) => {
                        if self
                            .routes
                            .refresh_authority(fence, authorized_until)
                            .await
                            .map_err(RelayCoordinatorError::RouteRegistry)?
                        {
                            self.slots
                                .get_mut(&slot_id)
                                .expect("the coordinator serializes slot updates")
                                .authorized_until = authorized_until;
                            continue;
                        }
                        self.begin_retirement(fence, RetirementAction::Reconcile)
                            .await?;
                        continue;
                    }
                    LocalSlotState::Reserving(_) => {
                        self.slots
                            .get_mut(&slot_id)
                            .expect("the coordinator serializes slot updates")
                            .authorized_until = authorized_until;
                        continue;
                    }
                    LocalSlotState::ReportingFailure(_) => {
                        self.slots
                            .get_mut(&slot_id)
                            .expect("the coordinator serializes slot updates")
                            .authorized_until = authorized_until;
                        continue;
                    }
                }
            }

            let provider = match relay_provider(
                &peer,
                &bases,
                limits.duration_seconds,
                limits.data_bytes_per_direction,
            ) {
                Ok(provider) => provider,
                Err(error) => {
                    warn!(slot_id = %slot_id, error = %error, "DMS relay provider metadata is invalid");
                    let fence = self.next_fence(slot_id, assignment_id, reservation_epoch);
                    self.slots.insert(
                        slot_id,
                        LocalSlot {
                            fence,
                            relay_peer_id: peer,
                            provider_base_addresses: bases,
                            limits: ExpectedRelayLimits::new(
                                Duration::from_secs(u64::from(limits.duration_seconds)),
                                limits.data_bytes_per_direction,
                            )
                            .map_err(|_| RelayCoordinatorError::ActiveBookingMismatch)?,
                            authorized_until,
                            state: LocalSlotState::ReportingFailure(
                                ReservationFailureReason::AddressMismatch,
                            ),
                            cancel_retry: CancellationToken::new(),
                            worker: None,
                        },
                    );
                    let _ = self
                        .events
                        .send(ChildEvent::RetryFailure {
                            fence,
                            reason: ReservationFailureReason::AddressMismatch,
                        })
                        .await;
                    continue;
                }
            };
            let fence = self.next_fence(slot_id, assignment_id, reservation_epoch);
            let cancel_retry = CancellationToken::new();
            self.slots.insert(
                slot_id,
                LocalSlot {
                    fence,
                    relay_peer_id: provider.relay_peer_id().to_string(),
                    provider_base_addresses: bases,
                    limits: provider.expected_limits(),
                    authorized_until,
                    state: LocalSlotState::Reserving(None),
                    cancel_retry: cancel_retry.clone(),
                    worker: None,
                },
            );
            let worker = spawn_reservation_worker(
                Arc::clone(&self.backend),
                self.events.clone(),
                fence,
                provider,
                cancel_retry,
                self.config.clone(),
            );
            self.slots
                .get_mut(&slot_id)
                .expect("the coordinator serializes slot creation")
                .worker = Some(worker);
        }
        self.update_availability();
        self.reset_expiry_timer();
        Ok(())
    }

    fn next_fence(
        &mut self,
        slot_id: Uuid,
        assignment_id: Uuid,
        reservation_epoch: Uuid,
    ) -> LocalRelayFence {
        let local_generation = self.next_generation;
        self.next_generation = self.next_generation.saturating_add(1);
        LocalRelayFence {
            slot_id,
            assignment_id,
            reservation_epoch,
            local_generation,
        }
    }

    async fn handle_child_event(&mut self, event: ChildEvent) -> Result<(), RelayCoordinatorError> {
        match event {
            ChildEvent::Started { fence, handle } => {
                let accepted = self.slots.get_mut(&fence.slot_id).is_some_and(|local| {
                    if local.fence == fence
                        && matches!(local.state, LocalSlotState::Reserving(None))
                    {
                        local.state = LocalSlotState::Reserving(Some(handle));
                        true
                    } else {
                        false
                    }
                });
                if accepted {
                    self.pending_relay_failures.retain(|candidate, _| {
                        candidate.relay_peer_id() != handle.relay_peer_id()
                            || candidate.generation() >= handle.generation()
                    });
                } else {
                    self.pending_relay_failures.remove(&handle);
                    spawn_detached_cleanup(
                        Arc::clone(&self.backend),
                        self.events.clone(),
                        handle,
                        self.config.reservation_retry_budget,
                    );
                }
            }
            ChildEvent::Confirmed { fence, snapshot } => {
                let expected_handle = snapshot.handle();
                let current = self.slots.get(&fence.slot_id).is_some_and(|local| {
                    local.fence == fence
                        && matches!(local.state, LocalSlotState::Reserving(Some(handle)) if handle == expected_handle)
                        && self.snapshot_has_fence(fence)
                });
                if current {
                    if let Some(reason) = self.pending_relay_failures.remove(&expected_handle) {
                        let _ = self.join_slot_worker(fence).await?;
                        self.handle_local_failure(fence, reason).await?;
                        return Ok(());
                    }
                } else {
                    self.pending_relay_failures.remove(&snapshot.handle());
                    spawn_detached_cleanup(
                        Arc::clone(&self.backend),
                        self.events.clone(),
                        snapshot.handle(),
                        self.config.reservation_retry_budget,
                    );
                    return Ok(());
                }
                let _ = self.join_slot_worker(fence).await?;
                let Some(route) = confirmed_route(&snapshot) else {
                    self.handle_local_failure(fence, ReservationFailureReason::ReservationDenied)
                        .await?;
                    return Ok(());
                };
                let local = self.slots.get(&fence.slot_id).expect("current slot");
                let publish = self
                    .routes
                    .publish(PublishedRelayRoute {
                        fence,
                        route,
                        limits: local.limits,
                        authorized_until: local.authorized_until,
                        relay_peer_id: snapshot.handle().relay_peer_id(),
                    })
                    .await;
                if let Err(error) = publish {
                    warn!(slot_id = %fence.slot_id, %error, "confirmed relay route could not be published");
                    self.handle_local_failure(fence, ReservationFailureReason::ReservationLost)
                        .await?;
                    return Ok(());
                }
                self.slots
                    .get_mut(&fence.slot_id)
                    .expect("current slot")
                    .state = LocalSlotState::Confirmed(snapshot.handle());
                self.update_availability();
            }
            ChildEvent::Retrying { fence, handle } => {
                if let Some(local) = self
                    .slots
                    .get_mut(&fence.slot_id)
                    .filter(|local| local.fence == fence)
                {
                    if matches!(local.state, LocalSlotState::Reserving(Some(current)) if current == handle)
                    {
                        local.state = LocalSlotState::Reserving(None);
                    }
                }
                self.pending_relay_failures.remove(&handle);
            }
            ChildEvent::Failed {
                fence,
                handle,
                reason,
            } => {
                if self.is_reserving(fence) {
                    let worker_handle = self.join_slot_worker(fence).await?;
                    let owned_handle = handle.or(worker_handle);
                    let reason = owned_handle
                        .and_then(|handle| self.pending_relay_failures.remove(&handle))
                        .map_or(reason, |pending| preferred_failure_reason(reason, pending));
                    if let Some(handle) = owned_handle {
                        if let Some(local) = self
                            .slots
                            .get_mut(&fence.slot_id)
                            .filter(|local| local.fence == fence)
                        {
                            if matches!(local.state, LocalSlotState::Reserving(None)) {
                                local.state = LocalSlotState::Reserving(Some(handle));
                            }
                        }
                    }
                    self.handle_local_failure(fence, reason).await?;
                } else if let Some(handle) = handle {
                    spawn_detached_cleanup(
                        Arc::clone(&self.backend),
                        self.events.clone(),
                        handle,
                        self.config.reservation_retry_budget,
                    );
                }
            }
            ChildEvent::RetryFailure { fence, reason } => {
                if self.is_current(fence) {
                    self.report_failure(fence, reason).await?;
                }
            }
            ChildEvent::CleanupComplete { fence } => {
                self.finish_retirement(fence).await?;
            }
            ChildEvent::DetachedCleanupFailed { error } => {
                return Err(RelayCoordinatorError::ReservationCleanup(error));
            }
        }
        self.next_poll = Instant::now();
        Ok(())
    }

    async fn handle_relay_event(
        &mut self,
        event: RelayTransportEvent,
    ) -> Result<(), RelayCoordinatorError> {
        match event {
            RelayTransportEvent::Renewed(_) | RelayTransportEvent::Publishable(_) => {}
            RelayTransportEvent::Unpublished { handle } => {
                self.observe_relay_failure(handle, ReservationFailureReason::ReservationLost)
                    .await?;
            }
            RelayTransportEvent::ConfirmationRejected { handle, reason } => {
                let failure = match reason {
                    RelayConfirmationRejection::MissingLimits
                    | RelayConfirmationRejection::IncompleteLimits { .. }
                    | RelayConfirmationRejection::LimitMismatch { .. } => {
                        ReservationFailureReason::LimitMismatch
                    }
                    _ => ReservationFailureReason::ReservationLost,
                };
                self.observe_relay_failure(handle, failure).await?;
            }
            RelayTransportEvent::Canceled { handle } => {
                self.observe_relay_failure(handle, ReservationFailureReason::ReservationLost)
                    .await?;
            }
        }
        Ok(())
    }

    async fn handle_relay_receive(
        &mut self,
        event: Result<RelayTransportEvent, broadcast::error::RecvError>,
    ) -> Result<(), RelayCoordinatorError> {
        match event {
            Ok(event) => self.handle_relay_event(event).await,
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                Err(RelayCoordinatorError::RelayEventLagged(skipped))
            }
            Err(broadcast::error::RecvError::Closed) => {
                self.relay_events = self.backend.subscribe();
                Ok(())
            }
        }
    }

    async fn observe_relay_failure(
        &mut self,
        handle: RelayReservationHandle,
        reason: ReservationFailureReason,
    ) -> Result<(), RelayCoordinatorError> {
        if let Some(fence) = self.confirmed_fence_for_handle(handle) {
            self.pending_relay_failures.remove(&handle);
            self.handle_local_failure(fence, reason).await?;
        } else if self.slots.values().any(|slot| {
            matches!(slot.state, LocalSlotState::Reserving(_))
                && slot.relay_peer_id == handle.relay_peer_id().to_string()
        }) {
            // Relay transport broadcasts can overtake Started/Retrying child
            // events. Hold exact-handle evidence until that generation is
            // consumed; never map a late old handle to a newer attempt by Peer ID.
            self.pending_relay_failures
                .entry(handle)
                .and_modify(|current| {
                    if reason == ReservationFailureReason::LimitMismatch {
                        *current = reason;
                    }
                })
                .or_insert(reason);
        }
        Ok(())
    }

    async fn handle_local_failure(
        &mut self,
        fence: LocalRelayFence,
        reason: ReservationFailureReason,
    ) -> Result<(), RelayCoordinatorError> {
        self.begin_retirement(fence, RetirementAction::ReportFailure(reason))
            .await?;
        Ok(())
    }

    async fn report_failure(
        &mut self,
        fence: LocalRelayFence,
        reason: ReservationFailureReason,
    ) -> Result<(), RelayCoordinatorError> {
        if !self.snapshot_has_fence(fence) {
            return Ok(());
        }
        let protected = self.next_renew.min(self.next_expiry);
        let until_protected = protected.saturating_duration_since(Instant::now());
        if until_protected <= self.config.http_timeout {
            let sender = self.events.clone();
            tokio::spawn(async move {
                tokio::time::sleep(until_protected + Duration::from_millis(1)).await;
                let _ = sender
                    .send(ChildEvent::RetryFailure { fence, reason })
                    .await;
            });
            return Ok(());
        }
        let request = ReservationFailedRequest {
            slot_id: fence.slot_id,
            assignment_id: fence.assignment_id,
            reservation_epoch: fence.reservation_epoch,
            reason,
        };
        match bounded_control_call(
            self.config.http_timeout,
            RelayOperation::ReservationFailed,
            self.api
                .report_reservation_failed(self.snapshot.booking_id, &request),
        )
        .await
        {
            Ok(snapshot) => {
                if self.is_current(fence) {
                    self.begin_retirement(fence, RetirementAction::Reconcile)
                        .await?;
                }
                self.apply_snapshot(snapshot).await?;
                self.next_poll = Instant::now() + self.config.status_poll_interval;
            }
            Err(error) => {
                warn!(error = %error, slot_id = %fence.slot_id, "reservation-failure report failed");
                if control_error_ends_authority(&error) {
                    self.fence_control_plane().await?;
                    return Err(RelayCoordinatorError::AuthorityEnded);
                }
                self.next_poll = Instant::now();
                if error.is_retryable() {
                    let sender = self.events.clone();
                    let delay = retry_jitter(self.config.retry_min, self.config.retry_max);
                    tokio::spawn(async move {
                        tokio::time::sleep(delay).await;
                        let _ = sender
                            .send(ChildEvent::RetryFailure { fence, reason })
                            .await;
                    });
                }
            }
        }
        Ok(())
    }

    async fn begin_retirement(
        &mut self,
        fence: LocalRelayFence,
        action: RetirementAction,
    ) -> Result<(), RelayCoordinatorError> {
        if let Some(retiring) = self.retiring.get_mut(&fence.slot_id) {
            if retiring.local.fence == fence && action == RetirementAction::Remove {
                retiring.action = RetirementAction::Remove;
            }
            return Ok(());
        }
        let Some(current) = self.slots.get(&fence.slot_id) else {
            return Ok(());
        };
        if current.fence != fence {
            return Ok(());
        }
        self.routes
            .tombstone(fence)
            .await
            .map_err(RelayCoordinatorError::RouteRegistry)?;
        let mut local = self
            .slots
            .remove(&fence.slot_id)
            .expect("the coordinator serializes slot removal");
        local.cancel_retry.cancel();
        let handle = match local.state {
            LocalSlotState::Confirmed(handle) | LocalSlotState::Reserving(Some(handle)) => {
                Some(handle)
            }
            _ => None,
        };
        let worker = local.worker.take();
        let cleanup = spawn_retirement_cleanup(
            Arc::clone(&self.backend),
            self.events.clone(),
            fence,
            worker,
            handle,
            self.config.reservation_retry_budget,
        );
        self.retiring.insert(
            fence.slot_id,
            RetiringSlot {
                local,
                action,
                cleanup,
            },
        );
        self.update_availability();
        self.reset_expiry_timer();
        Ok(())
    }

    async fn finish_retirement(
        &mut self,
        fence: LocalRelayFence,
    ) -> Result<(), RelayCoordinatorError> {
        let Some(retiring) = self.retiring.remove(&fence.slot_id) else {
            return Ok(());
        };
        if retiring.local.fence != fence {
            self.retiring.insert(fence.slot_id, retiring);
            return Ok(());
        }
        retiring
            .cleanup
            .await
            .map_err(|error| RelayCoordinatorError::ReservationCleanup(error.to_string()))?
            .map_err(RelayCoordinatorError::ReservationCleanup)?;

        match retiring.action {
            RetirementAction::Reconcile => self.apply_current_snapshot().await?,
            RetirementAction::ReportFailure(reason) if self.snapshot_has_fence(fence) => {
                let mut local = retiring.local;
                local.state = LocalSlotState::ReportingFailure(reason);
                local.worker = None;
                self.slots.insert(fence.slot_id, local);
                self.reset_expiry_timer();
                self.report_failure(fence, reason).await?;
            }
            RetirementAction::ReportFailure(_) => self.apply_current_snapshot().await?,
            RetirementAction::Remove => {}
        }
        self.update_availability();
        self.reset_expiry_timer();
        Ok(())
    }

    async fn remove_all_slots(&mut self) -> Result<(), RelayCoordinatorError> {
        let fences: Vec<_> = self.slots.values().map(|slot| slot.fence).collect();
        for fence in fences {
            self.begin_retirement(fence, RetirementAction::Remove)
                .await?;
        }
        for retiring in self.retiring.values_mut() {
            retiring.action = RetirementAction::Remove;
        }
        let retirements = std::mem::take(&mut self.retiring);
        let mut first_error = None;
        for (_, retiring) in retirements {
            match retiring.cleanup.await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    first_error.get_or_insert(error);
                }
                Err(error) => {
                    first_error.get_or_insert(error.to_string());
                }
            }
        }
        if let Some(error) = first_error {
            return Err(RelayCoordinatorError::ReservationCleanup(error));
        }
        self.update_availability();
        self.reset_expiry_timer();
        Ok(())
    }

    async fn stop(&mut self, delete_booking: bool) -> Result<(), RelayCoordinatorError> {
        self.remove_all_slots().await?;
        if delete_booking && !self.control_fenced {
            let started = Instant::now();
            let mut retry_ceiling = self.config.retry_min;
            loop {
                match bounded_control_call(
                    self.config.http_timeout,
                    RelayOperation::Delete,
                    self.api.delete(self.snapshot.booking_id),
                )
                .await
                {
                    Ok(()) => break,
                    Err(error) if error.http_code() == Some(RelayErrorCode::AuthorityEnded) => {
                        break;
                    }
                    Err(error) if error.is_retryable() => {
                        let delay = retry_jitter(self.config.retry_min, retry_ceiling);
                        if started.elapsed().saturating_add(delay)
                            >= self.config.reservation_retry_budget
                        {
                            return Err(error.into());
                        }
                        tokio::time::sleep(delay).await;
                        retry_ceiling = (retry_ceiling * 2).min(self.config.retry_max);
                    }
                    Err(error) => return Err(error.into()),
                }
            }
        }
        Ok(())
    }

    async fn fence_control_plane(&mut self) -> Result<(), RelayCoordinatorError> {
        self.control_fenced = true;
        self.remove_all_slots().await
    }

    fn is_current(&self, fence: LocalRelayFence) -> bool {
        self.slots
            .get(&fence.slot_id)
            .is_some_and(|slot| slot.fence == fence)
            && self.snapshot_has_fence(fence)
    }

    fn is_reserving(&self, fence: LocalRelayFence) -> bool {
        self.is_current(fence)
            && self
                .slots
                .get(&fence.slot_id)
                .is_some_and(|slot| matches!(slot.state, LocalSlotState::Reserving(_)))
    }

    async fn join_slot_worker(
        &mut self,
        fence: LocalRelayFence,
    ) -> Result<Option<RelayReservationHandle>, RelayCoordinatorError> {
        let worker = self
            .slots
            .get_mut(&fence.slot_id)
            .filter(|slot| slot.fence == fence)
            .and_then(|slot| slot.worker.take());
        match worker {
            Some(worker) => worker
                .await
                .map_err(|error| RelayCoordinatorError::ReservationCleanup(error.to_string()))?
                .map_err(RelayCoordinatorError::ReservationCleanup),
            None => Ok(None),
        }
    }

    fn confirmed_fence_for_handle(
        &self,
        handle: RelayReservationHandle,
    ) -> Option<LocalRelayFence> {
        self.slots.values().find_map(|slot| match slot.state {
            LocalSlotState::Confirmed(current) if current == handle => Some(slot.fence),
            _ => None,
        })
    }

    fn snapshot_has_fence(&self, fence: LocalRelayFence) -> bool {
        self.snapshot.slots.iter().any(|slot| {
            slot.slot_id == fence.slot_id
                && slot.assignment_id == Some(fence.assignment_id)
                && slot.reservation_epoch == Some(fence.reservation_epoch)
        })
    }

    fn update_availability(&self) {
        let confirmed = self
            .slots
            .values()
            .filter(|slot| matches!(slot.state, LocalSlotState::Confirmed(_)))
            .count();
        self.availability.send_replace(confirmed);
    }

    fn schedule_status_poll(&mut self, immediate: bool) {
        self.next_poll = if immediate {
            Instant::now()
        } else {
            Instant::now() + self.status_poll_delay()
        };
    }

    fn status_poll_delay(&self) -> Duration {
        let configured = status_poll_jitter(self.config.status_poll_interval);
        let now = chrono::Utc::now();
        let deadline_delay = self
            .slots
            .values()
            .filter(|slot| !matches!(slot.state, LocalSlotState::ReportingFailure(_)))
            .map(|slot| slot.authorized_until)
            .min()
            .and_then(|deadline| deadline.signed_duration_since(now).to_std().ok())
            .map(|remaining| remaining.saturating_sub(self.config.http_timeout));
        deadline_delay
            .map(|delay| configured.min(delay).max(Duration::from_millis(1)))
            .unwrap_or(configured)
    }

    fn reset_expiry_timer(&mut self) {
        let now = chrono::Utc::now();
        let next = self
            .slots
            .values()
            .filter(|slot| !matches!(slot.state, LocalSlotState::ReportingFailure(_)))
            .map(|slot| slot.authorized_until)
            .min()
            .unwrap_or(now + chrono::Duration::hours(24));
        let delay = next.signed_duration_since(now).to_std().unwrap_or_default();
        self.next_expiry = Instant::now() + delay;
    }

    async fn expire_local_authority(&mut self) -> Result<(), RelayCoordinatorError> {
        let now = chrono::Utc::now();
        let expired: Vec<_> = self
            .slots
            .values()
            .filter(|slot| !matches!(slot.state, LocalSlotState::ReportingFailure(_)))
            .filter(|slot| slot.authorized_until <= now)
            .map(|slot| slot.fence)
            .collect();
        for fence in expired {
            self.begin_retirement(fence, RetirementAction::Remove)
                .await?;
        }
        self.reset_expiry_timer();
        Ok(())
    }
}

fn control_error_ends_authority(error: &RelayBookingClientError) -> bool {
    matches!(
        error.http_code(),
        Some(
            RelayErrorCode::StaleRobotPrincipal
                | RelayErrorCode::InvalidRobotPrincipal
                | RelayErrorCode::AuthorityEnded
                | RelayErrorCode::NotFound
        )
    )
}

async fn bounded_control_call<T, F>(
    timeout: Duration,
    operation: RelayOperation,
    future: F,
) -> Result<T, RelayBookingClientError>
where
    F: Future<Output = Result<T, RelayBookingClientError>>,
{
    tokio::time::timeout(timeout, future)
        .await
        .map_err(|_| RelayBookingClientError::Transport {
            operation,
            timeout: true,
        })?
}

fn spawn_retirement_cleanup(
    backend: SharedReservationBackend,
    sender: mpsc::Sender<ChildEvent>,
    fence: LocalRelayFence,
    worker: Option<JoinHandle<Result<Option<RelayReservationHandle>, String>>>,
    handle: Option<RelayReservationHandle>,
    timeout: Duration,
) -> JoinHandle<Result<(), String>> {
    tokio::spawn(async move {
        let result = async {
            let mut cleanup_error = None;
            let mut worker_handle = None;
            if let Some(mut worker) = worker {
                match tokio::time::timeout(timeout, &mut worker).await {
                    Ok(Ok(Ok(returned_handle))) => worker_handle = returned_handle,
                    Ok(Ok(Err(error))) => cleanup_error = Some(error),
                    Ok(Err(error)) => {
                        cleanup_error = Some(format!("reservation worker failed: {error}"));
                    }
                    Err(_) => {
                        worker.abort();
                        let _ = worker.await;
                        cleanup_error = Some("reservation worker cleanup timed out".to_string());
                    }
                }
            }
            let mut candidates = Vec::with_capacity(2);
            if let Some(handle) = handle {
                candidates.push(handle);
            }
            if let Some(worker_handle) = worker_handle {
                if !candidates.contains(&worker_handle) {
                    candidates.push(worker_handle);
                }
            }
            for candidate in candidates {
                if let Err(error) =
                    cancel_reservation_with_timeout(&backend, candidate, timeout).await
                {
                    cleanup_error.get_or_insert(error);
                }
            }
            cleanup_error.map_or(Ok(()), Err)
        }
        .await;
        let _ = sender.send(ChildEvent::CleanupComplete { fence }).await;
        result
    })
}

fn spawn_detached_cleanup(
    backend: SharedReservationBackend,
    sender: mpsc::Sender<ChildEvent>,
    handle: RelayReservationHandle,
    timeout: Duration,
) {
    tokio::spawn(async move {
        if let Err(error) = cancel_reservation_with_timeout(&backend, handle, timeout).await {
            let _ = sender
                .send(ChildEvent::DetachedCleanupFailed { error })
                .await;
        }
    });
}

async fn cancel_reservation_with_timeout(
    backend: &SharedReservationBackend,
    handle: RelayReservationHandle,
    timeout: Duration,
) -> Result<(), String> {
    match tokio::time::timeout(timeout, backend.cancel(handle)).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) if cancellation_already_complete(&error) => Ok(()),
        Ok(Err(error)) => Err(error.to_string()),
        Err(_) => Err("reservation cancellation timed out".to_string()),
    }
}

fn cancellation_already_complete(error: &auki_p2p::Error) -> bool {
    matches!(
        error,
        auki_p2p::Error::RelayReservation(
            auki_p2p::RelayReservationError::StaleHandle
                | auki_p2p::RelayReservationError::UnknownHandle
        )
    )
}

fn preferred_failure_reason(
    observed: ReservationFailureReason,
    pending: ReservationFailureReason,
) -> ReservationFailureReason {
    if observed == ReservationFailureReason::LimitMismatch
        || pending == ReservationFailureReason::LimitMismatch
    {
        ReservationFailureReason::LimitMismatch
    } else {
        pending
    }
}

fn spawn_reservation_worker(
    backend: SharedReservationBackend,
    sender: mpsc::Sender<ChildEvent>,
    fence: LocalRelayFence,
    provider: RelayProvider,
    cancel_retry: CancellationToken,
    config: RelayCoordinatorConfig,
) -> JoinHandle<Result<Option<RelayReservationHandle>, String>> {
    tokio::spawn(async move {
        let started = Instant::now();
        let mut retry_ceiling = config.retry_min;
        loop {
            if cancel_retry.is_cancelled() {
                return Ok(None);
            }
            let remaining_budget = config
                .reservation_retry_budget
                .saturating_sub(started.elapsed());
            if remaining_budget.is_zero() {
                return finish_reservation_worker(
                    &backend,
                    &sender,
                    fence,
                    None,
                    ReservationFailureReason::DialFailed,
                    config.reservation_retry_budget,
                )
                .await;
            }

            let attempt_backend = Arc::clone(&backend);
            let attempt_provider = provider.clone();
            let mut attempt =
                tokio::spawn(async move { attempt_backend.start(attempt_provider).await });
            let joined = tokio::select! {
                biased;
                _ = cancel_retry.cancelled() => {
                    return abort_reservation_start(attempt).await;
                }
                _ = tokio::time::sleep(remaining_budget) => {
                    let handle = abort_reservation_start(attempt).await?;
                    return finish_reservation_worker(
                        &backend,
                        &sender,
                        fence,
                        handle,
                        ReservationFailureReason::DialFailed,
                        config.reservation_retry_budget,
                    ).await;
                }
                joined = &mut attempt => joined,
            };
            let start_result = match joined {
                Ok(result) => result,
                Err(_) => Err(ReservationAttemptFailure {
                    handle: None,
                    reason: ReservationFailureReason::DialFailed,
                    retryable: false,
                }),
            };
            let failure = if let Ok(handle) = start_result {
                if sender
                    .send(ChildEvent::Started { fence, handle })
                    .await
                    .is_err()
                {
                    cancel_reservation_with_timeout(
                        &backend,
                        handle,
                        config.reservation_retry_budget,
                    )
                    .await?;
                    return Ok(None);
                }
                let remaining_budget = config
                    .reservation_retry_budget
                    .saturating_sub(started.elapsed());
                if remaining_budget.is_zero() {
                    return finish_reservation_worker(
                        &backend,
                        &sender,
                        fence,
                        Some(handle),
                        ReservationFailureReason::ReservationDenied,
                        config.reservation_retry_budget,
                    )
                    .await;
                }
                let wait_backend = Arc::clone(&backend);
                let wait = async move { wait_backend.wait(handle).await };
                tokio::pin!(wait);
                let result = tokio::select! {
                    biased;
                    _ = cancel_retry.cancelled() => {
                        return Ok(Some(handle));
                    }
                    _ = tokio::time::sleep(remaining_budget) => {
                        return finish_reservation_worker(
                            &backend,
                            &sender,
                            fence,
                            Some(handle),
                            ReservationFailureReason::ReservationDenied,
                            config.reservation_retry_budget,
                        ).await;
                    }
                    result = &mut wait => result,
                };
                match result {
                    Ok(snapshot) => {
                        if sender
                            .send(ChildEvent::Confirmed {
                                fence,
                                snapshot: snapshot.clone(),
                            })
                            .await
                            .is_err()
                        {
                            cancel_reservation_with_timeout(
                                &backend,
                                snapshot.handle(),
                                config.reservation_retry_budget,
                            )
                            .await?;
                            return Ok(None);
                        }
                        return Ok(Some(snapshot.handle()));
                    }
                    Err(failure) => failure,
                }
            } else {
                start_result.expect_err("the reservation start result was checked")
            };
            if !failure.retryable || started.elapsed() >= config.reservation_retry_budget {
                return finish_reservation_worker(
                    &backend,
                    &sender,
                    fence,
                    failure.handle,
                    failure.reason,
                    config.reservation_retry_budget,
                )
                .await;
            }
            if let Some(handle) = failure.handle {
                if cancel_reservation_with_timeout(
                    &backend,
                    handle,
                    config.reservation_retry_budget,
                )
                .await
                .is_err()
                {
                    return finish_reservation_worker(
                        &backend,
                        &sender,
                        fence,
                        Some(handle),
                        failure.reason,
                        config.reservation_retry_budget,
                    )
                    .await;
                }
                if sender
                    .send(ChildEvent::Retrying { fence, handle })
                    .await
                    .is_err()
                {
                    return Ok(None);
                }
            }
            let delay = retry_jitter(config.retry_min, retry_ceiling);
            if started.elapsed().saturating_add(delay) >= config.reservation_retry_budget {
                return finish_reservation_worker(
                    &backend,
                    &sender,
                    fence,
                    None,
                    failure.reason,
                    config.reservation_retry_budget,
                )
                .await;
            }
            tokio::select! {
                _ = cancel_retry.cancelled() => return Ok(None),
                _ = tokio::time::sleep(delay) => {}
            }
            retry_ceiling = (retry_ceiling * 2).min(config.retry_max);
        }
    })
}

async fn abort_reservation_start(
    attempt: JoinHandle<Result<RelayReservationHandle, ReservationAttemptFailure>>,
) -> Result<Option<RelayReservationHandle>, String> {
    attempt.abort();
    match attempt.await {
        Ok(Ok(handle)) => Ok(Some(handle)),
        Ok(Err(failure)) => Ok(failure.handle),
        Err(error) if error.is_cancelled() => Ok(None),
        Err(error) => Err(format!("reservation start task failed: {error}")),
    }
}

async fn finish_reservation_worker(
    backend: &SharedReservationBackend,
    sender: &mpsc::Sender<ChildEvent>,
    fence: LocalRelayFence,
    handle: Option<RelayReservationHandle>,
    reason: ReservationFailureReason,
    timeout: Duration,
) -> Result<Option<RelayReservationHandle>, String> {
    if sender
        .send(ChildEvent::Failed {
            fence,
            handle,
            reason,
        })
        .await
        .is_ok()
    {
        return Ok(handle);
    }
    if let Some(handle) = handle {
        cancel_reservation_with_timeout(backend, handle, timeout).await?;
    }
    Ok(None)
}

fn retry_jitter(minimum: Duration, maximum: Duration) -> Duration {
    let minimum_ms = u64::try_from(minimum.as_millis()).unwrap_or(u64::MAX);
    let maximum_ms = u64::try_from(maximum.max(minimum).as_millis()).unwrap_or(u64::MAX);
    Duration::from_millis(rand::thread_rng().gen_range(minimum_ms..=maximum_ms))
}

fn status_poll_jitter(interval: Duration) -> Duration {
    let interval_ms = u64::try_from(interval.as_millis()).unwrap_or(u64::MAX);
    let minimum_ms = interval_ms.saturating_mul(80) / 100;
    let maximum_ms = interval_ms.saturating_mul(120) / 100;
    Duration::from_millis(rand::thread_rng().gen_range(minimum_ms..=maximum_ms))
}

fn validate_booking_matches(
    snapshot: &RelayBookingSnapshot,
    config: &RelayCoordinatorConfig,
) -> Result<(), RelayCoordinatorError> {
    if snapshot.mode != config.mode
        || snapshot.requested_duration_seconds != config.requested_duration_seconds
        || snapshot.relay_count != config.relay_count
    {
        return Err(RelayCoordinatorError::ActiveBookingMismatch);
    }
    if snapshot.state != RelayBookingState::Active {
        return Err(RelayCoordinatorError::AuthorityEnded);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use parking_lot::Mutex;

    use crate::dms::relay::{
        CreateRelayBookingResponse, RelayBookingCreateDisposition, RelayLimits, RelaySlotSnapshot,
    };

    use super::*;

    type ApiResult<T> = Result<T, RelayBookingClientError>;

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum ApiCall {
        Active,
        Create {
            key: RelayIdempotencyKey,
            request: CreateRelayBookingRequest,
        },
        Renew(Uuid),
        ReservationFailed {
            booking_id: Uuid,
            request: ReservationFailedRequest,
        },
        Delete(Uuid),
    }

    #[derive(Default)]
    struct ScriptedApi {
        calls: Mutex<Vec<ApiCall>>,
        active: Mutex<VecDeque<ApiResult<Option<RelayBookingSnapshot>>>>,
        create: Mutex<VecDeque<ApiResult<CreateRelayBookingResponse>>>,
        renew: Mutex<VecDeque<ApiResult<RelayBookingSnapshot>>>,
        reservation_failed: Mutex<VecDeque<ApiResult<RelayBookingSnapshot>>>,
    }

    impl ScriptedApi {
        fn push_active(&self, response: ApiResult<Option<RelayBookingSnapshot>>) {
            self.active.lock().push_back(response);
        }

        fn push_create(&self, response: ApiResult<CreateRelayBookingResponse>) {
            self.create.lock().push_back(response);
        }

        fn push_renew(&self, response: ApiResult<RelayBookingSnapshot>) {
            self.renew.lock().push_back(response);
        }

        fn push_reservation_failed(&self, response: ApiResult<RelayBookingSnapshot>) {
            self.reservation_failed.lock().push_back(response);
        }

        fn calls(&self) -> Vec<ApiCall> {
            self.calls.lock().clone()
        }
    }

    fn take_scripted<T>(
        responses: &Mutex<VecDeque<ApiResult<T>>>,
        operation: &'static str,
    ) -> ApiResult<T> {
        responses
            .lock()
            .pop_front()
            .unwrap_or_else(|| panic!("missing scripted {operation} response"))
    }

    #[async_trait]
    impl RelayBookingApi for ScriptedApi {
        async fn active(&self) -> ApiResult<Option<RelayBookingSnapshot>> {
            self.calls.lock().push(ApiCall::Active);
            take_scripted(&self.active, "active")
        }

        async fn create(
            &self,
            idempotency_key: &RelayIdempotencyKey,
            request: &CreateRelayBookingRequest,
        ) -> ApiResult<CreateRelayBookingResponse> {
            self.calls.lock().push(ApiCall::Create {
                key: idempotency_key.clone(),
                request: request.clone(),
            });
            take_scripted(&self.create, "create")
        }

        async fn renew(&self, booking_id: Uuid) -> ApiResult<RelayBookingSnapshot> {
            self.calls.lock().push(ApiCall::Renew(booking_id));
            take_scripted(&self.renew, "renew")
        }

        async fn report_reservation_failed(
            &self,
            booking_id: Uuid,
            request: &ReservationFailedRequest,
        ) -> ApiResult<RelayBookingSnapshot> {
            self.calls.lock().push(ApiCall::ReservationFailed {
                booking_id,
                request: request.clone(),
            });
            take_scripted(&self.reservation_failed, "reservation-failed")
        }

        async fn delete(&self, booking_id: Uuid) -> ApiResult<()> {
            self.calls.lock().push(ApiCall::Delete(booking_id));
            Ok(())
        }
    }

    struct PendingStartDrop<'a>(&'a AtomicUsize);

    impl Drop for PendingStartDrop<'_> {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct PendingStartBackend {
        relay_events: broadcast::Sender<RelayTransportEvent>,
        starts: AtomicUsize,
        dropped_starts: AtomicUsize,
        cancellations: AtomicUsize,
    }

    impl PendingStartBackend {
        fn new() -> Self {
            let (relay_events, _) = broadcast::channel(8);
            Self {
                relay_events,
                starts: AtomicUsize::new(0),
                dropped_starts: AtomicUsize::new(0),
                cancellations: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl RelayReservationBackend for PendingStartBackend {
        async fn start(
            &self,
            _provider: RelayProvider,
        ) -> Result<RelayReservationHandle, ReservationAttemptFailure> {
            self.starts.fetch_add(1, Ordering::SeqCst);
            let _drop = PendingStartDrop(&self.dropped_starts);
            std::future::pending().await
        }

        async fn wait(
            &self,
            _handle: RelayReservationHandle,
        ) -> Result<RelayReservationSnapshot, ReservationAttemptFailure> {
            panic!("pending-start backend cannot reach wait")
        }

        async fn cancel(&self, _handle: RelayReservationHandle) -> Result<(), auki_p2p::Error> {
            self.cancellations.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn subscribe(&self) -> broadcast::Receiver<RelayTransportEvent> {
            self.relay_events.subscribe()
        }
    }

    #[derive(Default)]
    struct RecordingRoutes {
        publications: AtomicUsize,
        tombstones: Mutex<Vec<LocalRelayFence>>,
    }

    #[async_trait]
    impl RelayRouteRegistry for RecordingRoutes {
        async fn publish(&self, route: PublishedRelayRoute) -> Result<(), String> {
            self.publications.fetch_add(1, Ordering::SeqCst);
            let _ = route;
            Ok(())
        }

        async fn refresh_authority(
            &self,
            _fence: LocalRelayFence,
            _authorized_until: chrono::DateTime<chrono::Utc>,
        ) -> Result<bool, String> {
            Ok(true)
        }

        async fn tombstone(&self, fence: LocalRelayFence) -> Result<bool, String> {
            self.tombstones.lock().push(fence);
            Ok(true)
        }
    }

    fn coordinator_config(idempotency_key: &str) -> RelayCoordinatorConfig {
        RelayCoordinatorConfig {
            idempotency_key: RelayIdempotencyKey::new(idempotency_key).expect("valid test key"),
            mode: RelayBookingMode::Public,
            requested_duration_seconds: 900,
            relay_count: 1,
            status_poll_interval: Duration::from_secs(60),
            reservation_retry_budget: Duration::from_secs(30),
            retry_min: Duration::from_millis(5),
            retry_max: Duration::from_millis(5),
            http_timeout: Duration::from_millis(100),
            authority_safety_margin: Duration::from_secs(15),
            gate_task_polling: true,
        }
    }

    fn queued_snapshot(booking_id: Uuid) -> RelayBookingSnapshot {
        let now = chrono::Utc::now();
        RelayBookingSnapshot {
            booking_id,
            mode: RelayBookingMode::Public,
            state: RelayBookingState::Active,
            relay_count: 1,
            requested_duration_seconds: 900,
            requested_until: now + chrono::Duration::seconds(900),
            authority_expires_at: now + chrono::Duration::minutes(5),
            assigned_count: 0,
            provider_ready_count: 0,
            unfilled_count: 1,
            created_at: now - chrono::Duration::seconds(1),
            ended_at: None,
            slots: vec![RelaySlotSnapshot {
                slot_id: Uuid::new_v4(),
                slot_index: 0,
                state: RelaySlotState::Queued,
                assignment_id: None,
                reservation_epoch: None,
                provider_peer_id: None,
                provider_base_addresses: None,
                limits: None,
                provider_lease_expires_at: None,
                recovery_expires_at: None,
            }],
        }
    }

    fn ready_snapshot(
        booking_id: Uuid,
        slot_id: Uuid,
        assignment_id: Uuid,
        reservation_epoch: Uuid,
        relay_peer_id: auki_p2p::PeerId,
    ) -> RelayBookingSnapshot {
        let now = chrono::Utc::now();
        RelayBookingSnapshot {
            booking_id,
            mode: RelayBookingMode::Public,
            state: RelayBookingState::Active,
            relay_count: 1,
            requested_duration_seconds: 900,
            requested_until: now + chrono::Duration::seconds(900),
            authority_expires_at: now + chrono::Duration::minutes(5),
            assigned_count: 1,
            provider_ready_count: 1,
            unfilled_count: 0,
            created_at: now - chrono::Duration::seconds(1),
            ended_at: None,
            slots: vec![RelaySlotSnapshot {
                slot_id,
                slot_index: 0,
                state: RelaySlotState::Ready,
                assignment_id: Some(assignment_id),
                reservation_epoch: Some(reservation_epoch),
                provider_peer_id: Some(relay_peer_id.to_string()),
                provider_base_addresses: Some(vec![format!(
                    "/dns4/relay-a.dev.aukiverse.com/tcp/443/p2p/{relay_peer_id}"
                )]),
                limits: Some(RelayLimits {
                    duration_seconds: 900,
                    data_bytes_per_direction: 1_048_576,
                }),
                provider_lease_expires_at: Some(now + chrono::Duration::minutes(4)),
                recovery_expires_at: None,
            }],
        }
    }

    fn replayed_create(snapshot: RelayBookingSnapshot) -> CreateRelayBookingResponse {
        CreateRelayBookingResponse {
            disposition: RelayBookingCreateDisposition::Replayed,
            location: format!("/relay-bookings/{}", snapshot.booking_id),
            snapshot,
        }
    }

    fn control_transport_error(operation: RelayOperation) -> RelayBookingClientError {
        RelayBookingClientError::Transport {
            operation,
            timeout: true,
        }
    }

    fn actor_harness(
        api: Arc<ScriptedApi>,
        backend: Arc<PendingStartBackend>,
        routes: Arc<RecordingRoutes>,
        config: RelayCoordinatorConfig,
        snapshot: RelayBookingSnapshot,
    ) -> (CoordinatorActor, mpsc::Sender<CoordinatorCommand>) {
        let (commands, command_rx) = mpsc::channel(4);
        let (events, event_rx) = mpsc::channel(16);
        let (availability, _) = watch::channel(0);
        let relay_events = backend.subscribe();
        let now = Instant::now();
        (
            CoordinatorActor {
                api,
                backend,
                routes,
                config,
                snapshot,
                slots: HashMap::new(),
                retiring: HashMap::new(),
                pending_relay_failures: HashMap::new(),
                next_generation: 1,
                command_rx,
                events,
                event_rx,
                relay_events,
                availability,
                next_poll: now + Duration::from_secs(3_600),
                next_renew: now + Duration::from_secs(3_600),
                next_expiry: now + Duration::from_secs(3_600),
                control_fenced: false,
            },
            commands,
        )
    }

    async fn wait_for_count(counter: &AtomicUsize, expected: usize) {
        tokio::time::timeout(Duration::from_secs(1), async {
            while counter.load(Ordering::SeqCst) < expected {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("counter reached its expected value");
    }

    async fn process_next_child_event(actor: &mut CoordinatorActor) {
        let event = tokio::time::timeout(Duration::from_secs(1), actor.event_rx.recv())
            .await
            .expect("child event arrived")
            .expect("child event channel remained open");
        actor
            .handle_child_event(event)
            .await
            .expect("child event was accepted");
    }

    #[test]
    fn provider_construction_uses_the_exact_dms_limits_and_canonical_base() {
        let peer = auki_p2p::PeerId::random();
        let provider = relay_provider(
            &peer.to_string(),
            &[format!("/dns4/RELAY.Example.COM./tcp/0443/p2p/{peer}")],
            900,
            1_048_576,
        )
        .expect("provider");

        assert_eq!(provider.relay_peer_id(), peer);
        assert_eq!(
            provider.selected_base().to_string(),
            format!("/dns4/relay.example.com/tcp/443/p2p/{peer}")
        );
        assert_eq!(
            provider.expected_limits().duration(),
            Duration::from_secs(900)
        );
        assert_eq!(
            provider.expected_limits().data_bytes_per_direction(),
            1_048_576
        );
    }

    #[test]
    fn failure_mapping_does_not_confuse_provider_config_with_dial_loss() {
        let mismatch =
            auki_p2p::Error::RelayConfirmationRejected(RelayConfirmationRejection::MissingLimits);
        assert_eq!(
            reservation_failure_reason(&mismatch, false),
            ReservationFailureReason::LimitMismatch
        );
        assert!(!reservation_failure_is_retryable(&mismatch));

        let dns = auki_p2p::Error::Dns("NXDOMAIN".to_string());
        assert_eq!(
            reservation_failure_reason(&dns, false),
            ReservationFailureReason::DialFailed
        );
        assert!(reservation_failure_is_retryable(&dns));

        let closed = auki_p2p::Error::RelayReservationClosed("closed".to_string());
        assert_eq!(
            reservation_failure_reason(&closed, true),
            ReservationFailureReason::ReservationLost
        );
        assert_eq!(
            preferred_failure_reason(
                ReservationFailureReason::ReservationDenied,
                ReservationFailureReason::ReservationLost,
            ),
            ReservationFailureReason::ReservationLost
        );
        assert_eq!(
            preferred_failure_reason(
                ReservationFailureReason::ReservationLost,
                ReservationFailureReason::LimitMismatch,
            ),
            ReservationFailureReason::LimitMismatch
        );
    }

    #[tokio::test]
    async fn lost_create_response_is_recovered_by_active_lookup_without_duplicate_create() {
        let booking_id = Uuid::new_v4();
        let snapshot = queued_snapshot(booking_id);
        let api = Arc::new(ScriptedApi::default());
        api.push_active(Ok(None));
        api.push_create(Err(control_transport_error(RelayOperation::Create)));
        api.push_active(Ok(Some(snapshot)));
        let backend = Arc::new(PendingStartBackend::new());
        let routes = Arc::new(RecordingRoutes::default());
        let config = coordinator_config("lost-create-active-first");

        let first = RelayBookingCoordinator::start(
            api.clone(),
            backend.clone(),
            routes.clone(),
            config.clone(),
        )
        .await;
        assert!(matches!(
            first,
            Err(RelayCoordinatorError::Dms(
                RelayBookingClientError::Transport {
                    operation: RelayOperation::Create,
                    timeout: true
                }
            ))
        ));

        let recovered = RelayBookingCoordinator::start(api.clone(), backend, routes, config)
            .await
            .expect("active booking recovers the lost create response");
        recovered.shutdown(false).await.expect("clean shutdown");

        let calls = api.calls();
        assert_eq!(calls.len(), 3);
        assert!(matches!(calls[0], ApiCall::Active));
        assert!(matches!(calls[1], ApiCall::Create { .. }));
        assert!(matches!(calls[2], ApiCall::Active));
    }

    #[tokio::test]
    async fn lost_create_retry_reuses_one_stable_idempotency_key() {
        let snapshot = queued_snapshot(Uuid::new_v4());
        let api = Arc::new(ScriptedApi::default());
        api.push_active(Ok(None));
        api.push_create(Err(control_transport_error(RelayOperation::Create)));
        api.push_active(Ok(None));
        api.push_create(Ok(replayed_create(snapshot)));
        let backend = Arc::new(PendingStartBackend::new());
        let routes = Arc::new(RecordingRoutes::default());
        let config = coordinator_config("stable-create-key");
        let expected_key = config.idempotency_key.clone();

        let first = RelayBookingCoordinator::start(
            api.clone(),
            backend.clone(),
            routes.clone(),
            config.clone(),
        )
        .await;
        assert!(first.is_err());
        let recovered = RelayBookingCoordinator::start(api.clone(), backend, routes, config)
            .await
            .expect("idempotent create replay");
        recovered.shutdown(false).await.expect("clean shutdown");

        let calls = api.calls();
        assert_eq!(calls.len(), 4);
        assert!(matches!(calls[0], ApiCall::Active));
        assert!(matches!(calls[2], ApiCall::Active));
        let create_keys: Vec<_> = calls
            .iter()
            .filter_map(|call| match call {
                ApiCall::Create { key, .. } => Some(key),
                _ => None,
            })
            .collect();
        assert_eq!(create_keys, vec![&expected_key, &expected_key]);
    }

    #[tokio::test]
    async fn cancel_during_cooperative_blocked_start_is_bounded_and_emits_no_handle() {
        let backend = Arc::new(PendingStartBackend::new());
        let relay_peer_id = auki_p2p::PeerId::random();
        let provider = relay_provider(
            &relay_peer_id.to_string(),
            &[format!(
                "/dns4/relay-a.dev.aukiverse.com/tcp/443/p2p/{relay_peer_id}"
            )],
            900,
            1_048_576,
        )
        .expect("provider");
        let fence = LocalRelayFence {
            slot_id: Uuid::new_v4(),
            assignment_id: Uuid::new_v4(),
            reservation_epoch: Uuid::new_v4(),
            local_generation: 1,
        };
        let (events, mut event_rx) = mpsc::channel(4);
        let cancellation = CancellationToken::new();
        let worker = spawn_reservation_worker(
            backend.clone(),
            events,
            fence,
            provider,
            cancellation.clone(),
            coordinator_config("blocked-start"),
        );
        wait_for_count(&backend.starts, 1).await;

        cancellation.cancel();
        tokio::time::timeout(Duration::from_secs(1), worker)
            .await
            .expect("worker cancellation is bounded")
            .expect("worker did not panic")
            .expect("worker cleanup succeeded");

        assert_eq!(backend.dropped_starts.load(Ordering::SeqCst), 1);
        assert_eq!(backend.cancellations.load(Ordering::SeqCst), 0);
        assert!(matches!(
            event_rx.try_recv(),
            Err(mpsc::error::TryRecvError::Disconnected)
        ));
    }

    #[tokio::test]
    async fn preconfirmation_failure_tombstones_without_ever_publishing() {
        let booking_id = Uuid::new_v4();
        let slot_id = Uuid::new_v4();
        let assignment_id = Uuid::new_v4();
        let reservation_epoch = Uuid::new_v4();
        let ready = ready_snapshot(
            booking_id,
            slot_id,
            assignment_id,
            reservation_epoch,
            auki_p2p::PeerId::random(),
        );
        let api = Arc::new(ScriptedApi::default());
        api.push_reservation_failed(Ok(queued_snapshot(booking_id)));
        let backend = Arc::new(PendingStartBackend::new());
        let routes = Arc::new(RecordingRoutes::default());
        let (mut actor, _commands) = actor_harness(
            api.clone(),
            backend.clone(),
            routes.clone(),
            coordinator_config("preconfirmation-loss"),
            ready,
        );
        actor
            .apply_current_snapshot()
            .await
            .expect("ready slot starts reservation");
        wait_for_count(&backend.starts, 1).await;
        let fence = actor.slots[&slot_id].fence;

        actor
            .handle_local_failure(fence, ReservationFailureReason::ReservationLost)
            .await
            .expect("pre-confirmation loss is fenced");
        process_next_child_event(&mut actor).await;

        assert_eq!(routes.publications.load(Ordering::SeqCst), 0);
        assert!(routes.tombstones.lock().contains(&fence));
        assert_eq!(backend.cancellations.load(Ordering::SeqCst), 0);
        assert!(matches!(
            api.calls().as_slice(),
            [ApiCall::ReservationFailed {
                booking_id: observed_booking_id,
                request
            }] if *observed_booking_id == booking_id
                && request.slot_id == slot_id
                && request.assignment_id == assignment_id
                && request.reservation_epoch == reservation_epoch
                && request.reason == ReservationFailureReason::ReservationLost
        ));
    }

    #[tokio::test]
    async fn local_authority_cutoff_tombstones_without_reporting_provider_failure() {
        let booking_id = Uuid::new_v4();
        let slot_id = Uuid::new_v4();
        let assignment_id = Uuid::new_v4();
        let reservation_epoch = Uuid::new_v4();
        let relay_peer_id = auki_p2p::PeerId::random();
        let snapshot = ready_snapshot(
            booking_id,
            slot_id,
            assignment_id,
            reservation_epoch,
            relay_peer_id,
        );
        let api = Arc::new(ScriptedApi::default());
        let backend = Arc::new(PendingStartBackend::new());
        let routes = Arc::new(RecordingRoutes::default());
        let (mut actor, _commands) = actor_harness(
            api.clone(),
            backend,
            routes.clone(),
            coordinator_config("local-authority-cutoff"),
            snapshot.clone(),
        );
        let fence = LocalRelayFence {
            slot_id,
            assignment_id,
            reservation_epoch,
            local_generation: 1,
        };
        actor.slots.insert(
            slot_id,
            LocalSlot {
                fence,
                relay_peer_id: relay_peer_id.to_string(),
                provider_base_addresses: snapshot.slots[0].provider_base_addresses.clone().unwrap(),
                limits: ExpectedRelayLimits::new(Duration::from_secs(900), 1_048_576).unwrap(),
                authorized_until: chrono::Utc::now() - chrono::Duration::milliseconds(1),
                state: LocalSlotState::Reserving(None),
                cancel_retry: CancellationToken::new(),
                worker: None,
            },
        );

        actor
            .expire_local_authority()
            .await
            .expect("local cutoff starts fenced retirement");
        process_next_child_event(&mut actor).await;

        assert!(!actor.slots.contains_key(&slot_id));
        assert!(routes.tombstones.lock().contains(&fence));
        assert!(!api
            .calls()
            .iter()
            .any(|call| matches!(call, ApiCall::ReservationFailed { .. })));
    }

    #[tokio::test]
    async fn lagged_relay_events_fail_closed_instead_of_trusting_dms_liveness() {
        let api = Arc::new(ScriptedApi::default());
        let backend = Arc::new(PendingStartBackend::new());
        let routes = Arc::new(RecordingRoutes::default());
        let (mut actor, _commands) = actor_harness(
            api,
            backend,
            routes,
            coordinator_config("lagged-relay-events"),
            queued_snapshot(Uuid::new_v4()),
        );

        let result = actor
            .handle_relay_receive(Err(broadcast::error::RecvError::Lagged(129)))
            .await;

        assert!(matches!(
            result,
            Err(RelayCoordinatorError::RelayEventLagged(129))
        ));
    }

    #[tokio::test]
    async fn retiring_one_child_does_not_block_parent_renewal() {
        let booking_id = Uuid::new_v4();
        let slot_id = Uuid::new_v4();
        let assignment_id = Uuid::new_v4();
        let reservation_epoch = Uuid::new_v4();
        let relay_peer_id = auki_p2p::PeerId::random();
        let snapshot = ready_snapshot(
            booking_id,
            slot_id,
            assignment_id,
            reservation_epoch,
            relay_peer_id,
        );
        let api = Arc::new(ScriptedApi::default());
        api.push_renew(Ok(snapshot.clone()));
        let backend = Arc::new(PendingStartBackend::new());
        let routes = Arc::new(RecordingRoutes::default());
        let mut config = coordinator_config("retiring-sibling");
        config.reservation_retry_budget = Duration::from_millis(25);
        let (mut actor, _commands) =
            actor_harness(api.clone(), backend, routes, config, snapshot.clone());
        let fence = LocalRelayFence {
            slot_id,
            assignment_id,
            reservation_epoch,
            local_generation: 1,
        };
        actor.slots.insert(
            slot_id,
            LocalSlot {
                fence,
                relay_peer_id: relay_peer_id.to_string(),
                provider_base_addresses: snapshot.slots[0].provider_base_addresses.clone().unwrap(),
                limits: ExpectedRelayLimits::new(Duration::from_secs(900), 1_048_576).unwrap(),
                authorized_until: chrono::Utc::now() + chrono::Duration::minutes(2),
                state: LocalSlotState::Reserving(None),
                cancel_retry: CancellationToken::new(),
                worker: Some(tokio::spawn(std::future::pending())),
            },
        );

        actor
            .handle_local_failure(fence, ReservationFailureReason::ReservationLost)
            .await
            .expect("retirement begins without joining the blocked child");
        tokio::time::timeout(Duration::from_millis(10), actor.renew())
            .await
            .expect("parent renewal is not blocked by child cleanup")
            .expect("parent renewal succeeds");

        assert!(api
            .calls()
            .iter()
            .any(|call| matches!(call, ApiCall::Renew(id) if *id == booking_id)));
        assert!(!api
            .calls()
            .iter()
            .any(|call| matches!(call, ApiCall::ReservationFailed { .. })));
        assert!(matches!(
            actor.remove_all_slots().await,
            Err(RelayCoordinatorError::ReservationCleanup(_))
        ));
    }

    #[tokio::test]
    async fn stale_assignment_event_cannot_retire_replacement_generation() {
        let booking_id = Uuid::new_v4();
        let slot_id = Uuid::new_v4();
        let relay_peer_id = auki_p2p::PeerId::random();
        let snapshot_a = ready_snapshot(
            booking_id,
            slot_id,
            Uuid::new_v4(),
            Uuid::new_v4(),
            relay_peer_id,
        );
        let snapshot_b = ready_snapshot(
            booking_id,
            slot_id,
            Uuid::new_v4(),
            Uuid::new_v4(),
            relay_peer_id,
        );
        let api = Arc::new(ScriptedApi::default());
        let backend = Arc::new(PendingStartBackend::new());
        let routes = Arc::new(RecordingRoutes::default());
        let (mut actor, _commands) = actor_harness(
            api.clone(),
            backend.clone(),
            routes.clone(),
            coordinator_config("stale-a-b"),
            snapshot_a,
        );
        actor
            .apply_current_snapshot()
            .await
            .expect("generation A starts");
        wait_for_count(&backend.starts, 1).await;
        let fence_a = actor.slots[&slot_id].fence;

        actor
            .apply_snapshot(snapshot_b)
            .await
            .expect("generation B replaces A");
        process_next_child_event(&mut actor).await;
        wait_for_count(&backend.starts, 2).await;
        let fence_b = actor.slots[&slot_id].fence;
        assert_ne!(fence_a, fence_b);
        let tombstones_before_stale_event = routes.tombstones.lock().len();

        actor
            .handle_child_event(ChildEvent::Failed {
                fence: fence_a,
                handle: None,
                reason: ReservationFailureReason::ReservationLost,
            })
            .await
            .expect("stale failure is harmless");

        assert_eq!(actor.slots[&slot_id].fence, fence_b);
        assert!(matches!(
            actor.slots[&slot_id].state,
            LocalSlotState::Reserving(None)
        ));
        assert_eq!(
            routes.tombstones.lock().len(),
            tombstones_before_stale_event
        );
        assert!(api.calls().is_empty());

        actor
            .remove_all_slots()
            .await
            .expect("test cleanup is bounded");
    }

    #[tokio::test]
    async fn foreign_booking_snapshot_is_rejected_without_replacing_pinned_authority() {
        let booking_id = Uuid::new_v4();
        let original = queued_snapshot(booking_id);
        let foreign = queued_snapshot(Uuid::new_v4());
        let api = Arc::new(ScriptedApi::default());
        let backend = Arc::new(PendingStartBackend::new());
        let routes = Arc::new(RecordingRoutes::default());
        let (mut actor, _commands) = actor_harness(
            api.clone(),
            backend,
            routes.clone(),
            coordinator_config("booking-id-pin"),
            original,
        );

        let result = actor.apply_snapshot(foreign).await;
        assert!(matches!(
            result,
            Err(RelayCoordinatorError::ActiveBookingMismatch)
        ));
        assert_eq!(actor.snapshot.booking_id, booking_id);
        assert!(actor.slots.is_empty());
        assert!(api.calls().is_empty());
        assert_eq!(routes.publications.load(Ordering::SeqCst), 0);
        assert!(routes.tombstones.lock().is_empty());
    }

    #[tokio::test]
    async fn disappeared_booking_is_a_terminal_health_failure() {
        let snapshot = queued_snapshot(Uuid::new_v4());
        let api = Arc::new(ScriptedApi::default());
        api.push_active(Ok(Some(snapshot)));
        api.push_active(Ok(None));
        let backend = Arc::new(PendingStartBackend::new());
        let routes = Arc::new(RecordingRoutes::default());
        let mut config = coordinator_config("terminal-health");
        config.status_poll_interval = Duration::from_millis(10);

        let mut coordinator = RelayBookingCoordinator::start(api.clone(), backend, routes, config)
            .await
            .expect("coordinator starts");
        let mut health = coordinator.health();
        tokio::time::timeout(Duration::from_secs(1), health.failed())
            .await
            .expect("terminal health signal is bounded");
        let result = coordinator
            .task
            .take()
            .expect("coordinator task")
            .await
            .expect("coordinator task did not panic");
        assert!(matches!(result, Err(RelayCoordinatorError::AuthorityEnded)));
        assert!(matches!(
            api.calls().as_slice(),
            [ApiCall::Active, ApiCall::Active]
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn protected_renewal_deadline_runs_before_adjacent_status_poll() {
        let booking_id = Uuid::new_v4();
        let snapshot = queued_snapshot(booking_id);
        let api = Arc::new(ScriptedApi::default());
        api.push_renew(Ok(snapshot.clone()));
        api.push_active(Ok(Some(snapshot.clone())));
        let backend = Arc::new(PendingStartBackend::new());
        let routes = Arc::new(RecordingRoutes::default());
        let mut config = coordinator_config("renew-before-poll");
        config.http_timeout = Duration::from_secs(1);
        let (mut actor, commands) = actor_harness(api.clone(), backend, routes, config, snapshot);
        let now = Instant::now();
        actor.next_poll = now + Duration::from_millis(9_500);
        actor.next_renew = now + Duration::from_secs(10);
        actor.next_expiry = now + Duration::from_secs(3_600);
        let task = tokio::spawn(async move { actor.run().await });
        tokio::task::yield_now().await;

        tokio::time::advance(Duration::from_millis(9_500)).await;
        tokio::task::yield_now().await;
        assert!(api.calls().is_empty());

        tokio::time::advance(Duration::from_millis(500)).await;
        for _ in 0..16 {
            if api.calls().len() >= 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(matches!(
            api.calls().as_slice(),
            [ApiCall::Renew(observed_booking_id), ApiCall::Active]
                if *observed_booking_id == booking_id
        ));

        let (response, receiver) = oneshot::channel();
        commands
            .send(CoordinatorCommand::Stop {
                delete_booking: false,
                response,
            })
            .await
            .expect("actor receives stop");
        receiver
            .await
            .expect("actor responds to stop")
            .expect("actor stops cleanly");
        task.await
            .expect("actor did not panic")
            .expect("actor run completed");
    }
}
