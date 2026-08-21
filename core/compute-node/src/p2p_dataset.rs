use std::{
    collections::{BTreeMap, HashMap, HashSet},
    future::Future,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use auki_p2p::{
    ApplicationProtocol, AuthenticatedStream, ExpectedRelayLimits, IncomingAuthenticatedStreams,
    Multiaddr, Node, PeerId, PeerRole, Protocol, RelayProvider, RelayReservationHandle,
    RelayRouteHandle, SessionRequirements,
};
use chrono::{DateTime, Utc};
use compute_runner_api::{
    P2pDataset, P2pDatasetReference, P2pDatasetRegistration, P2P_DATASET_SCHEMA,
};
use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt as TokioAsyncReadExt, AsyncWriteExt as TokioAsyncWriteExt},
    sync::watch,
    task::{JoinHandle, JoinSet},
};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use uuid::Uuid;

use crate::dds::p2p::{DdsP2pError, P2pCredentialStore};

pub const DATASET_PROTOCOL: &str = "/auki-p2p/dataset/0";
const DATASET_REQUEST_VERSION: u8 = 0;
const MAX_REQUEST_BYTES: usize = 4 * 1024;
const MAX_RESPONSE_HEADER_BYTES: usize = 16 * 1024;
const MAX_DATASET_ID_BYTES: usize = 512;
const MAX_DATASET_NAME_BYTES: usize = 1024;
const MAX_ROUTE_TEXT_BYTES: usize = 1024;
const TRANSFER_BUFFER_BYTES: usize = 64 * 1024;
const FETCH_ATTEMPTS: usize = 2;
const FETCH_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const FETCH_CANDIDATE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_PUBLISHED_ROUTES: usize = 16;
const MAX_CIRCUIT_ROUTES: usize = 3;
const MIN_CIRCUIT_DURATION: Duration = Duration::from_secs(15 * 60);
const RELAY_DATA_OVERHEAD_BYTES: u64 = 1_048_576;

#[derive(Debug, thiserror::Error)]
pub enum P2pDatasetError {
    #[error("P2P dataset credential is unavailable or unauthorized")]
    Credential(#[source] DdsP2pError),
    #[error("P2P dataset transport failed")]
    Transport(#[source] auki_p2p::Error),
    #[error("P2P dataset file operation failed")]
    Io(#[source] std::io::Error),
    #[error("P2P dataset protocol payload is malformed")]
    Json(#[source] serde_json::Error),
    #[error("P2P dataset request version is unsupported")]
    UnsupportedRequestVersion,
    #[error("P2P dataset protocol frame exceeds its limit")]
    FrameTooLarge,
    #[error("P2P dataset identifier is invalid")]
    InvalidDatasetId,
    #[error("P2P dataset name is invalid")]
    InvalidDatasetName,
    #[error("P2P dataset reference is invalid")]
    InvalidReference,
    #[error("P2P dataset file is empty")]
    EmptyDataset,
    #[error("P2P dataset is not registered")]
    UnknownDataset,
    #[error("P2P dataset is no longer available")]
    ExpiredDataset,
    #[error("P2P dataset service is not serving the requested Domain")]
    ServingDomainMismatch,
    #[error("P2P dataset service has no explicit advertised multiaddr")]
    MissingAdvertisedAddress,
    #[error("P2P dataset registrations are stopped")]
    RegistrationsStopped,
    #[error("P2P dataset registration requires a confirmed relay route")]
    ConfirmedRelayRequired,
    #[error("no confirmed relay route has enough capacity for this dataset")]
    NoEligibleRelayRoute,
    #[error("P2P dataset size exceeds the relay budget arithmetic range")]
    RelayBudgetOverflow,
    #[error("P2P dataset route set exceeds the {maximum}-route limit")]
    RouteLimitExceeded { maximum: usize },
    #[error("P2P dataset route set exceeds the {maximum}-circuit limit")]
    CircuitRouteLimitExceeded { maximum: usize },
    #[error("P2P dataset direct route is invalid: {0}")]
    InvalidDirectRoute(String),
    #[error("confirmed relay route is invalid: {0}")]
    InvalidRelayRoute(String),
    #[error("confirmed relay authorization is already expired")]
    RelayAuthorizationExpired,
    #[error("confirmed relay slot already contains a different fence")]
    RelaySlotOccupied,
    #[error("confirmed relay route fence is stale")]
    StaleRelayRouteFence,
    #[error("confirmed relay route was not found")]
    RelayRouteNotFound,
    #[error("confirmed relay Peer ID or endpoint duplicates another slot")]
    DuplicateRelayRoute,
    #[error("P2P dataset route revision is exhausted")]
    RouteRevisionExhausted,
    #[error("P2P dataset transfer generation is exhausted")]
    TransferGenerationExhausted,
    #[error("P2P dataset lifecycle deadline is out of range")]
    DeadlineOverflow,
    #[error("P2P dataset response does not match its source reference")]
    ReferenceMismatch,
    #[error("P2P dataset transfer ended before the declared size")]
    InterruptedTransfer,
    #[error("P2P dataset response size does not match its source reference")]
    SizeMismatch,
    #[error("P2P dataset response hash does not match its source reference")]
    HashMismatch,
    #[error("P2P dataset transfer timed out")]
    TransferTimeout,
    #[error("failed to close the exact P2P dataset relay route")]
    RelayRouteCleanup(#[source] auki_p2p::Error),
    #[error("failed to remove a partial P2P dataset file")]
    PartialFileCleanup(#[source] std::io::Error),
    #[error("P2P dataset server task failed")]
    ServerTask(#[source] tokio::task::JoinError),
}

pub type Result<T> = std::result::Result<T, P2pDatasetError>;

/// Publication rule selected by the Robot host.
///
/// `RelayRequired` deliberately covers both the `auto` and `always` booking
/// modes: their task-polling rules differ, but immutable dataset publication
/// requires a locally confirmed relay in both modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatasetRoutePolicy {
    DirectOnly,
    RelayRequired,
}

/// The control-plane and local-generation fence for one stable booking slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RelayRouteFence {
    pub slot_id: Uuid,
    pub assignment_id: Uuid,
    pub reservation_epoch: Uuid,
    pub local_generation: u64,
}

/// One locally confirmed circuit route supplied by the booking coordinator.
///
/// The coordinator computes `authorized_until` as the earliest applicable
/// booking horizon, Robot authority, and provider-lease deadline after its
/// safety margin. The adapter never extends that deadline itself.
#[derive(Clone, Debug)]
pub struct ConfirmedRelayRoute {
    pub fence: RelayRouteFence,
    pub reservation: RelayReservationHandle,
    pub route: Multiaddr,
    pub limits: ExpectedRelayLimits,
    pub authorized_until: DateTime<Utc>,
}

/// Observable publication and drain state. No credential or booking material
/// is exposed through this snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatasetServingStatus {
    pub route_set_revision: u64,
    pub registrations_open: bool,
    pub direct_route_count: usize,
    pub confirmed_relay_count: usize,
    pub max_available_until: Option<DateTime<Utc>>,
    pub active_transfer_count: usize,
    pub max_active_transfer_deadline: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct P2pDatasetAdapter {
    inner: Arc<AdapterInner>,
}

struct AdapterInner {
    node: Node,
    credentials: P2pCredentialStore,
    route_policy: DatasetRoutePolicy,
    state: parking_lot::Mutex<ServingState>,
    status: watch::Sender<DatasetServingStatus>,
}

struct ServingState {
    direct_routes: Vec<Multiaddr>,
    relay_routes: BTreeMap<Uuid, StoredRelayRoute>,
    route_set_revision: u64,
    registrations_open: bool,
    registry: HashMap<String, RegisteredDataset>,
    serving_domain: Option<Uuid>,
    active_transfers: BTreeMap<u64, ActiveTransfer>,
    next_transfer_id: u64,
}

#[derive(Clone)]
struct StoredRelayRoute {
    fence: RelayRouteFence,
    reservation: StoredReservationHandle,
    relay_peer_id: PeerId,
    endpoint: String,
    route: Multiaddr,
    limits: ExpectedRelayLimits,
    authorized_until: DateTime<Utc>,
    publishable: bool,
}

#[derive(Clone, Copy)]
enum StoredReservationHandle {
    Runtime(RelayReservationHandle),
    #[cfg(test)]
    Synthetic,
}

#[derive(Clone, Copy)]
struct ActiveTransfer {
    deadline: DateTime<Utc>,
}

struct ActiveTransferGuard {
    inner: Arc<AdapterInner>,
    transfer_id: u64,
}

#[derive(Clone)]
struct RegistrationRouteSnapshot {
    revision: u64,
    fences: Vec<RelayRouteFence>,
}

#[derive(Clone)]
struct RegisteredDataset {
    dataset_id: String,
    domain_id: Uuid,
    path: PathBuf,
    size_bytes: u64,
    sha256: String,
    available_until: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
struct DatasetRequest {
    version: u8,
    dataset_id: String,
}

#[derive(Serialize, Deserialize)]
struct DatasetResponseHeader {
    dataset_id: String,
    size_bytes: u64,
    sha256: String,
}

#[derive(Clone, Debug)]
enum DatasetRouteCandidate {
    Direct(Multiaddr),
    Circuit(Multiaddr),
}

impl DatasetRouteCandidate {
    fn route(&self) -> &Multiaddr {
        match self {
            Self::Direct(route) | Self::Circuit(route) => route,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::Direct(_) => "direct",
            Self::Circuit(_) => "circuit",
        }
    }
}

struct OpenedDatasetStream {
    stream: Option<AuthenticatedStream>,
    relay_route: Option<RelayRouteGuard>,
}

impl OpenedDatasetStream {
    fn stream_mut(&mut self) -> &mut AuthenticatedStream {
        self.stream
            .as_mut()
            .expect("an opened dataset stream remains present until cleanup")
    }

    async fn close(mut self) -> Result<()> {
        drop(self.stream.take());
        if let Some(mut relay_route) = self.relay_route.take() {
            relay_route.close().await?;
        }
        Ok(())
    }
}

struct RelayRouteGuard {
    node: Node,
    route: Option<RelayRouteHandle>,
}

impl RelayRouteGuard {
    fn new(node: Node, route: RelayRouteHandle) -> Self {
        Self {
            node,
            route: Some(route),
        }
    }

    async fn close(&mut self) -> Result<()> {
        let Some(route) = self.route.as_ref().cloned() else {
            return Ok(());
        };
        self.node
            .close_relay_route(&route)
            .await
            .map_err(P2pDatasetError::RelayRouteCleanup)?;
        self.route.take();
        Ok(())
    }
}

impl Drop for RelayRouteGuard {
    fn drop(&mut self) {
        let Some(route) = self.route.take() else {
            return;
        };
        let node = self.node.clone();
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        runtime.spawn(async move {
            if let Err(error) = node.close_relay_route(&route).await {
                warn!(error = %error, route = %route.route(), "failed to close dropped dataset relay route");
            }
        });
    }
}

struct PartialFileGuard {
    path: PathBuf,
    armed: bool,
}

impl PartialFileGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn disarm(&mut self) {
        self.armed = false;
    }

    async fn cleanup(&mut self) -> Result<()> {
        if !self.armed {
            return Ok(());
        }
        match fs::remove_file(&self.path).await {
            Ok(()) => {
                self.armed = false;
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.armed = false;
                Ok(())
            }
            // Retain ownership on failure so Drop can make one final
            // best-effort cleanup attempt while the typed error propagates.
            Err(error) => Err(P2pDatasetError::PartialFileCleanup(error)),
        }
    }
}

impl Drop for PartialFileGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

struct CanonicalCircuitRoute {
    route: Multiaddr,
    relay_peer_id: PeerId,
    endpoint_key: String,
}

impl P2pDatasetAdapter {
    /// Backwards-compatible direct-only constructor.
    pub fn new(
        node: Node,
        credentials: P2pCredentialStore,
        advertised_multiaddrs: Vec<Multiaddr>,
    ) -> Self {
        let direct_routes = sorted_unique_routes(advertised_multiaddrs);
        Self::build(
            node,
            credentials,
            direct_routes,
            DatasetRoutePolicy::DirectOnly,
        )
    }

    /// Construct an adapter with an explicit publication policy.
    ///
    /// Unlike [`Self::new`], this validates direct-route grammar and the shared
    /// v1 route bound. `RelayRequired` intentionally permits an empty direct
    /// vector; registration remains closed by readiness until a confirmed
    /// circuit is installed.
    pub fn new_with_route_policy(
        node: Node,
        credentials: P2pCredentialStore,
        direct_routes: Vec<Multiaddr>,
        route_policy: DatasetRoutePolicy,
    ) -> Result<Self> {
        let direct_routes = validate_and_sort_direct_routes(direct_routes, node.peer_id())?;
        Ok(Self::build(node, credentials, direct_routes, route_policy))
    }

    fn build(
        node: Node,
        credentials: P2pCredentialStore,
        direct_routes: Vec<Multiaddr>,
        route_policy: DatasetRoutePolicy,
    ) -> Self {
        let route_set_revision = u64::from(!direct_routes.is_empty());
        let state = ServingState {
            direct_routes,
            relay_routes: BTreeMap::new(),
            route_set_revision,
            registrations_open: true,
            registry: HashMap::new(),
            serving_domain: None,
            active_transfers: BTreeMap::new(),
            next_transfer_id: 1,
        };
        let (status, _) = watch::channel(serving_status(&state, Utc::now()));
        Self {
            inner: Arc::new(AdapterInner {
                node,
                credentials,
                route_policy,
                state: parking_lot::Mutex::new(state),
                status,
            }),
        }
    }

    pub fn credentials(&self) -> P2pCredentialStore {
        self.inner.credentials.clone()
    }

    /// Replace the direct publication set. An unchanged canonical set does not
    /// advance the route revision.
    pub fn replace_direct_routes(&self, direct_routes: Vec<Multiaddr>) -> Result<u64> {
        let direct_routes =
            validate_and_sort_direct_routes(direct_routes, self.inner.node.peer_id())?;
        let now = Utc::now();
        let (revision, status) = {
            let mut state = self.inner.state.lock();
            expire_relay_authorizations(&mut state, now)?;
            if state.direct_routes == direct_routes {
                return Ok(state.route_set_revision);
            }
            let confirmed = publishable_relay_count(&state, now);
            if direct_routes.len().saturating_add(confirmed) > MAX_PUBLISHED_ROUTES {
                return Err(P2pDatasetError::RouteLimitExceeded {
                    maximum: MAX_PUBLISHED_ROUTES,
                });
            }
            bump_route_revision(&mut state)?;
            state.direct_routes = direct_routes;
            (state.route_set_revision, serving_status(&state, now))
        };
        self.inner.status.send_replace(status);
        Ok(revision)
    }

    /// Publish one locally confirmed reservation generation.
    ///
    /// A slot holding a different fence must first be tombstoned with
    /// [`Self::tombstone_confirmed_relay_route`]. This makes cancellation order
    /// explicit and prevents a stale worker from replacing a recovered child.
    pub fn publish_confirmed_relay_route(&self, route: ConfirmedRelayRoute) -> Result<u64> {
        if route.fence.local_generation == 0 {
            return Err(P2pDatasetError::InvalidRelayRoute(
                "local generation must be positive".to_string(),
            ));
        }
        let (relay_peer_id, endpoint) = validate_circuit_route(
            &route.route,
            route.reservation.relay_peer_id(),
            self.inner.node.peer_id(),
        )?;
        let now = Utc::now();
        if route.authorized_until <= now {
            return Err(P2pDatasetError::RelayAuthorizationExpired);
        }

        let (revision, status) = {
            let mut state = self.inner.state.lock();
            expire_relay_authorizations(&mut state, now)?;
            if let Some(existing) = state.relay_routes.get(&route.fence.slot_id) {
                if existing.fence != route.fence {
                    return Err(P2pDatasetError::RelaySlotOccupied);
                }
                if !matches!(
                    existing.reservation,
                    StoredReservationHandle::Runtime(handle) if handle == route.reservation
                ) || existing.relay_peer_id != relay_peer_id
                    || existing.endpoint != endpoint
                    || existing.route != route.route
                    || existing.limits != route.limits
                {
                    return Err(P2pDatasetError::RelaySlotOccupied);
                }
            } else {
                if state.relay_routes.len() >= MAX_CIRCUIT_ROUTES {
                    return Err(P2pDatasetError::CircuitRouteLimitExceeded {
                        maximum: MAX_CIRCUIT_ROUTES,
                    });
                }
                if state.relay_routes.values().any(|existing| {
                    existing.relay_peer_id == relay_peer_id || existing.endpoint == endpoint
                }) {
                    return Err(P2pDatasetError::DuplicateRelayRoute);
                }
            }

            let already_publishable = state
                .relay_routes
                .get(&route.fence.slot_id)
                .is_some_and(|existing| existing.publishable);
            if !already_publishable
                && state
                    .direct_routes
                    .len()
                    .saturating_add(publishable_relay_count(&state, now))
                    .saturating_add(1)
                    > MAX_PUBLISHED_ROUTES
            {
                return Err(P2pDatasetError::RouteLimitExceeded {
                    maximum: MAX_PUBLISHED_ROUTES,
                });
            }
            if !already_publishable {
                bump_route_revision(&mut state)?;
            }

            state.relay_routes.insert(
                route.fence.slot_id,
                StoredRelayRoute {
                    fence: route.fence,
                    reservation: StoredReservationHandle::Runtime(route.reservation),
                    relay_peer_id,
                    endpoint,
                    route: route.route,
                    limits: route.limits,
                    authorized_until: route.authorized_until,
                    publishable: true,
                },
            );
            (state.route_set_revision, serving_status(&state, now))
        };
        self.inner.status.send_replace(status);
        Ok(revision)
    }

    /// Refresh only the absolute authorization deadline for the exact current
    /// fence. Extending a still-publishable route does not bump the revision;
    /// crossing the deadline in either direction does.
    pub fn refresh_confirmed_relay_authorization(
        &self,
        fence: RelayRouteFence,
        authorized_until: DateTime<Utc>,
    ) -> Result<u64> {
        let now = Utc::now();
        let (revision, status) = {
            let mut state = self.inner.state.lock();
            expire_relay_authorizations(&mut state, now)?;
            let entry = state
                .relay_routes
                .get(&fence.slot_id)
                .ok_or(P2pDatasetError::RelayRouteNotFound)?;
            if entry.fence != fence {
                return Err(P2pDatasetError::StaleRelayRouteFence);
            }
            let should_publish = authorized_until > now;
            let changed_publishability = entry.publishable != should_publish;
            if should_publish
                && changed_publishability
                && state
                    .direct_routes
                    .len()
                    .saturating_add(publishable_relay_count(&state, now))
                    .saturating_add(1)
                    > MAX_PUBLISHED_ROUTES
            {
                return Err(P2pDatasetError::RouteLimitExceeded {
                    maximum: MAX_PUBLISHED_ROUTES,
                });
            }
            if changed_publishability {
                bump_route_revision(&mut state)?;
            }
            let entry = state
                .relay_routes
                .get_mut(&fence.slot_id)
                .expect("relay entry remains locked");
            entry.authorized_until = authorized_until;
            entry.publishable = should_publish;
            (state.route_set_revision, serving_status(&state, now))
        };
        self.inner.status.send_replace(status);
        Ok(revision)
    }

    /// Atomically unpublish the exact current fence and return its reservation
    /// handle. The caller may begin `Node::cancel_relay_reservation` only after
    /// this method returns.
    pub fn tombstone_confirmed_relay_route(
        &self,
        fence: RelayRouteFence,
    ) -> Result<RelayReservationHandle> {
        let now = Utc::now();
        let (removed, status) = {
            let mut state = self.inner.state.lock();
            expire_relay_authorizations(&mut state, now)?;
            let existing = state
                .relay_routes
                .get(&fence.slot_id)
                .ok_or(P2pDatasetError::RelayRouteNotFound)?;
            if existing.fence != fence {
                return Err(P2pDatasetError::StaleRelayRouteFence);
            }
            let was_publishable = existing.publishable;
            if was_publishable {
                bump_route_revision(&mut state)?;
            }
            let removed = state
                .relay_routes
                .remove(&fence.slot_id)
                .expect("relay entry remains locked");
            (removed, serving_status(&state, now))
        };
        self.inner.status.send_replace(status);
        match removed.reservation {
            StoredReservationHandle::Runtime(handle) => Ok(handle),
            #[cfg(test)]
            StoredReservationHandle::Synthetic => {
                unreachable!("synthetic relay entries are private to unit tests")
            }
        }
    }

    /// Atomically unpublish every child before returning their cancellation
    /// handles in stable slot order.
    pub fn tombstone_all_confirmed_relay_routes(&self) -> Result<Vec<RelayReservationHandle>> {
        let now = Utc::now();
        let (routes, status) = {
            let mut state = self.inner.state.lock();
            expire_relay_authorizations(&mut state, now)?;
            let publishable = state
                .relay_routes
                .values()
                .filter(|entry| entry.publishable)
                .count();
            advance_route_revision(&mut state, publishable)?;
            let routes = std::mem::take(&mut state.relay_routes);
            (routes, serving_status(&state, now))
        };
        self.inner.status.send_replace(status);
        #[cfg(not(test))]
        let handles = routes
            .into_values()
            .map(|route| match route.reservation {
                StoredReservationHandle::Runtime(handle) => handle,
            })
            .collect();
        #[cfg(test)]
        let handles = routes
            .into_values()
            .filter_map(|route| match route.reservation {
                StoredReservationHandle::Runtime(handle) => Some(handle),
                StoredReservationHandle::Synthetic => None,
            })
            .collect();
        Ok(handles)
    }

    /// Stop accepting immutable dataset registrations and return the current
    /// deadlines the coordinator must drain.
    pub fn stop_registrations(&self) -> Result<DatasetServingStatus> {
        self.set_registrations_open(false)
    }

    /// Re-open registrations after startup or a completed configuration drain.
    pub fn start_registrations(&self) -> Result<DatasetServingStatus> {
        self.set_registrations_open(true)
    }

    fn set_registrations_open(&self, open: bool) -> Result<DatasetServingStatus> {
        let now = Utc::now();
        let status = {
            let mut state = self.inner.state.lock();
            expire_relay_authorizations(&mut state, now)?;
            prune_expired(&mut state.registry, now);
            state.registrations_open = open;
            serving_status(&state, now)
        };
        self.inner.status.send_if_modified(|current| {
            if *current == status {
                false
            } else {
                *current = status.clone();
                true
            }
        });
        Ok(status)
    }

    /// Return a current (time-pruned) route/readiness/drain snapshot.
    pub fn serving_status(&self) -> Result<DatasetServingStatus> {
        let now = Utc::now();
        let status = {
            let mut state = self.inner.state.lock();
            expire_relay_authorizations(&mut state, now)?;
            prune_expired(&mut state.registry, now);
            serving_status(&state, now)
        };
        self.inner.status.send_if_modified(|current| {
            if *current == status {
                false
            } else {
                *current = status.clone();
                true
            }
        });
        Ok(status)
    }

    /// Subscribe to route-readiness and drain-state changes.
    pub fn subscribe_serving_status(&self) -> watch::Receiver<DatasetServingStatus> {
        self.inner.status.subscribe()
    }

    pub async fn start_serving(
        &self,
        domain_id: Uuid,
        shutdown: &CancellationToken,
    ) -> Result<P2pDatasetServer> {
        {
            let state = self.inner.state.lock();
            if self.inner.route_policy == DatasetRoutePolicy::DirectOnly
                && state.direct_routes.is_empty()
            {
                return Err(P2pDatasetError::MissingAdvertisedAddress);
            }
            if state.serving_domain.is_some() {
                return Err(P2pDatasetError::ServingDomainMismatch);
            }
        }
        self.inner
            .credentials
            .require(PeerRole::Robot, domain_id)
            .await
            .map_err(P2pDatasetError::Credential)?;

        let protocol =
            ApplicationProtocol::new(DATASET_PROTOCOL).map_err(P2pDatasetError::Transport)?;
        let requirements = SessionRequirements::new(domain_id.to_string(), PeerRole::Compute)
            .map_err(P2pDatasetError::Transport)?;
        let incoming = self
            .inner
            .node
            .accept(protocol, requirements)
            .map_err(P2pDatasetError::Transport)?;
        {
            let mut state = self.inner.state.lock();
            if state.serving_domain.replace(domain_id).is_some() {
                return Err(P2pDatasetError::ServingDomainMismatch);
            }
        }

        let stop = shutdown.child_token();
        let task_stop = stop.clone();
        let adapter = self.clone();
        let task = tokio::spawn(async move {
            adapter.run_server(incoming, task_stop).await;
        });
        Ok(P2pDatasetServer {
            stop,
            task: Some(task),
        })
    }

    pub async fn register_dataset(
        &self,
        registration: P2pDatasetRegistration,
    ) -> Result<P2pDatasetReference> {
        self.register_dataset_with_hasher(
            registration,
            |path| async move { hash_file(&path).await },
        )
        .await
    }

    async fn register_dataset_with_hasher<H, F>(
        &self,
        registration: P2pDatasetRegistration,
        hasher: H,
    ) -> Result<P2pDatasetReference>
    where
        H: FnOnce(PathBuf) -> F,
        F: Future<Output = Result<(u64, String)>>,
    {
        validate_dataset_id(&registration.dataset_id)?;
        validate_dataset_name(&registration.name)?;
        if registration.available_until <= Utc::now() {
            return Err(P2pDatasetError::ExpiredDataset);
        }
        self.inner
            .credentials
            .require(PeerRole::Robot, registration.domain_id)
            .await
            .map_err(P2pDatasetError::Credential)?;

        let path = fs::canonicalize(&registration.path)
            .await
            .map_err(P2pDatasetError::Io)?;
        let metadata = fs::metadata(&path).await.map_err(P2pDatasetError::Io)?;
        if !metadata.is_file() {
            return Err(P2pDatasetError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "registered dataset is not a file",
            )));
        }

        // Capture readiness and all selected fences before the potentially
        // expensive hash. The final commit rechecks both under the same state
        // lock and recomputes from current routes on any revision change.
        let route_snapshot = self.capture_registration_routes(registration.domain_id)?;
        let (size_bytes, sha256) = hasher(path.clone()).await?;
        if size_bytes == 0 {
            return Err(P2pDatasetError::EmptyDataset);
        }
        let registered = RegisteredDataset {
            dataset_id: registration.dataset_id.clone(),
            domain_id: registration.domain_id,
            path,
            size_bytes,
            sha256: sha256.clone(),
            available_until: registration.available_until,
        };

        // Endpoint authority is separate from relay authority. Revalidate it
        // immediately before the immutable reference commit.
        self.inner
            .credentials
            .require(PeerRole::Robot, registration.domain_id)
            .await
            .map_err(P2pDatasetError::Credential)?;

        let now = Utc::now();
        let (reference, status) = {
            let mut state = self.inner.state.lock();
            expire_relay_authorizations(&mut state, now)?;
            prune_expired(&mut state.registry, now);
            validate_registration_state(
                &state,
                self.inner.route_policy,
                registration.domain_id,
                now,
            )?;

            let _snapshot_still_current = route_snapshot.revision == state.route_set_revision
                && route_snapshot.fences.iter().all(|fence| {
                    state
                        .relay_routes
                        .get(&fence.slot_id)
                        .is_some_and(|entry| entry.fence == *fence && entry.publishable)
                });
            // This is the retry/recompute point. Selection below always reads
            // the exclusively locked current state, so a newly confirmed
            // sibling is included and a lost/reassigned child is excluded.
            let multiaddrs =
                select_registration_routes(&state, self.inner.route_policy, size_bytes, now)?;
            state
                .registry
                .insert(registration.dataset_id.clone(), registered);
            let reference = P2pDatasetReference {
                schema: P2P_DATASET_SCHEMA.to_string(),
                dataset_id: registration.dataset_id,
                domain_id: registration.domain_id,
                name: registration.name,
                peer_id: self.inner.node.peer_id().to_string(),
                multiaddrs,
                size_bytes,
                sha256,
                available_until: registration.available_until,
            };
            (reference, serving_status(&state, now))
        };
        self.inner.status.send_replace(status);
        Ok(reference)
    }

    fn capture_registration_routes(&self, domain_id: Uuid) -> Result<RegistrationRouteSnapshot> {
        let now = Utc::now();
        let (snapshot, status) = {
            let mut state = self.inner.state.lock();
            expire_relay_authorizations(&mut state, now)?;
            prune_expired(&mut state.registry, now);
            validate_registration_state(&state, self.inner.route_policy, domain_id, now)?;
            let fences = state
                .relay_routes
                .values()
                .filter(|entry| entry.publishable && entry.authorized_until > now)
                .map(|entry| entry.fence)
                .collect();
            (
                RegistrationRouteSnapshot {
                    revision: state.route_set_revision,
                    fences,
                },
                serving_status(&state, now),
            )
        };
        self.inner.status.send_replace(status);
        Ok(snapshot)
    }

    pub async fn fetch_dataset(
        &self,
        reference: &P2pDatasetReference,
        destination: &Path,
    ) -> Result<()> {
        let (peer_id, candidates) = validate_reference(reference)?;
        self.inner
            .credentials
            .require(PeerRole::Compute, reference.domain_id)
            .await
            .map_err(P2pDatasetError::Credential)?;

        let parent = destination.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)
            .await
            .map_err(P2pDatasetError::Io)?;

        let protocol =
            ApplicationProtocol::new(DATASET_PROTOCOL).map_err(P2pDatasetError::Transport)?;
        let requirements =
            SessionRequirements::new(reference.domain_id.to_string(), PeerRole::Robot)
                .map_err(P2pDatasetError::Transport)?
                .with_expected_remote_peer_id(peer_id);

        let mut last_error = None;
        let mut round_start = 0_usize;
        for round in 0..FETCH_ATTEMPTS {
            // This is the one whole-file budget for the round. Candidate
            // connect, relay admission, endpoint authentication, and all bytes
            // share it; walking another candidate never starts a fresh budget.
            let round_deadline = tokio::time::Instant::now() + FETCH_ATTEMPT_TIMEOUT;
            let mut opened = None;
            for offset in 0..candidates.len() {
                if tokio::time::Instant::now() >= round_deadline {
                    last_error = Some(P2pDatasetError::TransferTimeout);
                    break;
                }
                let candidate_index = (round_start + offset) % candidates.len();
                let candidate = &candidates[candidate_index];
                match self
                    .open_dataset_candidate(
                        peer_id,
                        candidate,
                        &protocol,
                        &requirements,
                        round_deadline,
                        reference.available_until,
                    )
                    .await
                {
                    Ok(stream) => {
                        opened = Some((candidate_index, stream));
                        break;
                    }
                    Err(P2pDatasetError::ExpiredDataset) => {
                        return Err(P2pDatasetError::ExpiredDataset);
                    }
                    Err(error) => {
                        warn!(
                            error = %error,
                            route = %candidate.route(),
                            route_kind = candidate.kind(),
                            round = round + 1,
                            "P2P dataset candidate failed before streaming; trying the next safe route"
                        );
                        last_error = Some(error);
                    }
                }
            }

            let Some((candidate_index, mut opened)) = opened else {
                // If nothing opened, the only optional retry is a fresh final
                // round from the first deterministic candidate.
                round_start = 0;
                continue;
            };

            let mut partial = PartialFileGuard::new(partial_path(destination));
            let transfer = receive_dataset_stream(opened.stream_mut(), reference, partial.path());
            let result = match tokio::time::timeout_at(round_deadline, transfer).await {
                Ok(Ok(())) if tokio::time::Instant::now() >= round_deadline => {
                    Err(P2pDatasetError::TransferTimeout)
                }
                Ok(result) => result,
                Err(_) => Err(P2pDatasetError::TransferTimeout),
            };
            let route_cleanup = opened.close().await;

            match result {
                Ok(()) => {
                    if let Err(error) = route_cleanup {
                        partial.cleanup().await?;
                        return Err(error);
                    }
                    if let Err(error) = fs::rename(partial.path(), destination).await {
                        partial.cleanup().await?;
                        return Err(P2pDatasetError::Io(error));
                    }
                    partial.disarm();
                    return Ok(());
                }
                Err(error) => {
                    let partial_cleanup = partial.cleanup().await;
                    partial_cleanup?;
                    route_cleanup?;
                    let retry = error.is_retryable() && round + 1 < FETCH_ATTEMPTS;
                    last_error = Some(error);
                    if !retry {
                        break;
                    }
                    // Round two begins at the next route. With one candidate
                    // this wraps to the same route; with siblings, alternatives
                    // are visited before the failed route can be retried.
                    round_start = (candidate_index + 1) % candidates.len();
                }
            }
        }
        Err(last_error.unwrap_or(P2pDatasetError::InterruptedTransfer))
    }

    async fn open_dataset_candidate(
        &self,
        peer_id: PeerId,
        candidate: &DatasetRouteCandidate,
        protocol: &ApplicationProtocol,
        requirements: &SessionRequirements,
        round_deadline: tokio::time::Instant,
        available_until: DateTime<Utc>,
    ) -> Result<OpenedDatasetStream> {
        if available_until <= Utc::now() {
            return Err(P2pDatasetError::ExpiredDataset);
        }
        let availability_deadline = utc_deadline_as_instant(available_until);
        let bounded_candidate_deadline = std::cmp::min(
            round_deadline,
            tokio::time::Instant::now() + FETCH_CANDIDATE_TIMEOUT,
        );
        let candidate_deadline = std::cmp::min(bounded_candidate_deadline, availability_deadline);
        match candidate {
            DatasetRouteCandidate::Direct(route) => {
                let open = self.inner.node.open(
                    peer_id,
                    vec![route.clone()],
                    protocol.clone(),
                    requirements.clone(),
                );
                let stream = tokio::time::timeout_at(candidate_deadline, open)
                    .await
                    .map_err(|_| {
                        candidate_timeout_error(availability_deadline, bounded_candidate_deadline)
                    })?
                    .map_err(P2pDatasetError::Transport)?;
                if tokio::time::Instant::now() >= candidate_deadline {
                    drop(stream);
                    return Err(candidate_timeout_error(
                        availability_deadline,
                        bounded_candidate_deadline,
                    ));
                }
                Ok(OpenedDatasetStream {
                    stream: Some(stream),
                    relay_route: None,
                })
            }
            DatasetRouteCandidate::Circuit(route) => {
                let connect = self.inner.node.connect_relayed(route.clone(), requirements);
                let route_handle = tokio::time::timeout_at(candidate_deadline, connect)
                    .await
                    .map_err(|_| {
                        candidate_timeout_error(availability_deadline, bounded_candidate_deadline)
                    })?
                    .map_err(P2pDatasetError::Transport)?;
                let mut relay_route = RelayRouteGuard::new(self.inner.node.clone(), route_handle);
                if tokio::time::Instant::now() >= candidate_deadline {
                    let timeout_error =
                        candidate_timeout_error(availability_deadline, bounded_candidate_deadline);
                    relay_route.close().await?;
                    return Err(timeout_error);
                }
                // Source admission authorizes the CONNECT completed by
                // connect_relayed. Once that circuit exists, accepted_until is
                // not a deadline on its endpoint-authenticated application
                // substream; the candidate/round and dataset deadlines remain
                // authoritative here.
                let open_deadline = candidate_deadline;
                if tokio::time::Instant::now() >= open_deadline {
                    let timeout_error =
                        candidate_timeout_error(availability_deadline, bounded_candidate_deadline);
                    relay_route.close().await?;
                    return Err(timeout_error);
                }
                let open = self.inner.node.open_relayed(
                    relay_route
                        .route
                        .as_ref()
                        .expect("new relay route guard contains its handle"),
                    protocol.clone(),
                    requirements.clone(),
                );
                let stream = match tokio::time::timeout_at(open_deadline, open).await {
                    Ok(Ok(stream)) => {
                        if tokio::time::Instant::now() >= open_deadline {
                            drop(stream);
                            let timeout_error = candidate_timeout_error(
                                availability_deadline,
                                bounded_candidate_deadline,
                            );
                            relay_route.close().await?;
                            return Err(timeout_error);
                        }
                        stream
                    }
                    Ok(Err(error)) => {
                        relay_route.close().await?;
                        return Err(P2pDatasetError::Transport(error));
                    }
                    Err(_) => {
                        let timeout_error = candidate_timeout_error(
                            availability_deadline,
                            bounded_candidate_deadline,
                        );
                        relay_route.close().await?;
                        return Err(timeout_error);
                    }
                };
                Ok(OpenedDatasetStream {
                    stream: Some(stream),
                    relay_route: Some(relay_route),
                })
            }
        }
    }

    async fn run_server(
        &self,
        mut incoming: IncomingAuthenticatedStreams,
        stop: CancellationToken,
    ) {
        let mut handlers = JoinSet::new();
        loop {
            tokio::select! {
                _ = stop.cancelled() => break,
                incoming_stream = incoming.accept() => {
                    match incoming_stream {
                        Some(Ok(stream)) => {
                            let adapter = self.clone();
                            handlers.spawn(async move {
                                if let Err(error) = adapter.serve_stream(stream).await {
                                    warn!(error = %error, "P2P dataset stream failed");
                                }
                            });
                        }
                        Some(Err(error)) => {
                            warn!(error = %error, "P2P dataset session authentication failed");
                        }
                        None => break,
                    }
                }
                joined = handlers.join_next(), if !handlers.is_empty() => {
                    if let Some(Err(error)) = joined {
                        warn!(error = %error, "P2P dataset stream task failed");
                    }
                }
            }
        }
        handlers.abort_all();
        while handlers.join_next().await.is_some() {}
        let status = {
            let mut state = self.inner.state.lock();
            state.serving_domain = None;
            serving_status(&state, Utc::now())
        };
        self.inner.status.send_replace(status);
    }

    async fn serve_stream(&self, mut stream: AuthenticatedStream) -> Result<()> {
        let request: DatasetRequest = read_json_frame(&mut stream, MAX_REQUEST_BYTES).await?;
        if request.version != DATASET_REQUEST_VERSION {
            return Err(P2pDatasetError::UnsupportedRequestVersion);
        }
        validate_dataset_id(&request.dataset_id)?;
        let (entry, _transfer_guard) = self.begin_transfer(&request.dataset_id)?;

        let transfer = async {
            self.inner
                .credentials
                .require(PeerRole::Robot, entry.domain_id)
                .await
                .map_err(P2pDatasetError::Credential)?;
            self.serve_registered_stream(stream, entry).await
        };
        match tokio::time::timeout(FETCH_ATTEMPT_TIMEOUT, transfer).await {
            Ok(result) => result,
            Err(_) => Err(P2pDatasetError::TransferTimeout),
        }
    }

    fn begin_transfer(&self, dataset_id: &str) -> Result<(RegisteredDataset, ActiveTransferGuard)> {
        let now = Utc::now();
        let deadline = now
            .checked_add_signed(
                chrono::Duration::from_std(FETCH_ATTEMPT_TIMEOUT)
                    .map_err(|_| P2pDatasetError::DeadlineOverflow)?,
            )
            .ok_or(P2pDatasetError::DeadlineOverflow)?;
        let (entry, transfer_id, status) = {
            let mut state = self.inner.state.lock();
            prune_expired(&mut state.registry, now);
            let entry = state
                .registry
                .get(dataset_id)
                .cloned()
                .ok_or(P2pDatasetError::UnknownDataset)?;
            let transfer_id = state.next_transfer_id;
            state.next_transfer_id = state
                .next_transfer_id
                .checked_add(1)
                .ok_or(P2pDatasetError::TransferGenerationExhausted)?;
            state
                .active_transfers
                .insert(transfer_id, ActiveTransfer { deadline });
            (entry, transfer_id, serving_status(&state, now))
        };
        self.inner.status.send_replace(status);
        Ok((
            entry,
            ActiveTransferGuard {
                inner: Arc::clone(&self.inner),
                transfer_id,
            },
        ))
    }

    async fn serve_registered_stream(
        &self,
        mut stream: AuthenticatedStream,
        entry: RegisteredDataset,
    ) -> Result<()> {
        let header = DatasetResponseHeader {
            dataset_id: entry.dataset_id,
            size_bytes: entry.size_bytes,
            sha256: entry.sha256,
        };
        write_json_frame(&mut stream, &header, MAX_RESPONSE_HEADER_BYTES).await?;
        let mut file = File::open(&entry.path).await.map_err(P2pDatasetError::Io)?;
        let mut remaining = entry.size_bytes;
        let mut buffer = vec![0_u8; TRANSFER_BUFFER_BYTES];
        while remaining > 0 {
            let limit = usize::try_from(remaining)
                .unwrap_or(usize::MAX)
                .min(buffer.len());
            let count = TokioAsyncReadExt::read(&mut file, &mut buffer[..limit])
                .await
                .map_err(P2pDatasetError::Io)?;
            if count == 0 {
                return Err(P2pDatasetError::InterruptedTransfer);
            }
            stream
                .write_all(&buffer[..count])
                .await
                .map_err(P2pDatasetError::Io)?;
            remaining -= count as u64;
        }
        stream.flush().await.map_err(P2pDatasetError::Io)?;
        stream.close().await.map_err(P2pDatasetError::Io)?;
        Ok(())
    }
}

async fn receive_dataset_stream(
    stream: &mut AuthenticatedStream,
    reference: &P2pDatasetReference,
    temp_path: &Path,
) -> Result<()> {
    write_json_frame(
        stream,
        &DatasetRequest {
            version: DATASET_REQUEST_VERSION,
            dataset_id: reference.dataset_id.clone(),
        },
        MAX_REQUEST_BYTES,
    )
    .await?;
    stream.flush().await.map_err(P2pDatasetError::Io)?;
    let header: DatasetResponseHeader = read_json_frame(stream, MAX_RESPONSE_HEADER_BYTES).await?;
    if header.dataset_id != reference.dataset_id
        || header.size_bytes != reference.size_bytes
        || !header.sha256.eq_ignore_ascii_case(&reference.sha256)
    {
        return Err(P2pDatasetError::ReferenceMismatch);
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp_path)
        .await
        .map_err(P2pDatasetError::Io)?;
    let mut hasher = Sha256::new();
    let mut received = 0_u64;
    let mut buffer = vec![0_u8; TRANSFER_BUFFER_BYTES];
    while received < reference.size_bytes {
        let remaining = reference.size_bytes - received;
        let limit = usize::try_from(remaining)
            .unwrap_or(usize::MAX)
            .min(buffer.len());
        let count = stream
            .read(&mut buffer[..limit])
            .await
            .map_err(P2pDatasetError::Io)?;
        if count == 0 {
            return Err(P2pDatasetError::InterruptedTransfer);
        }
        TokioAsyncWriteExt::write_all(&mut file, &buffer[..count])
            .await
            .map_err(P2pDatasetError::Io)?;
        hasher.update(&buffer[..count]);
        received += count as u64;
    }
    let mut extra = [0_u8; 1];
    if stream.read(&mut extra).await.map_err(P2pDatasetError::Io)? != 0 {
        return Err(P2pDatasetError::SizeMismatch);
    }
    if !hex::encode(hasher.finalize()).eq_ignore_ascii_case(&reference.sha256) {
        return Err(P2pDatasetError::HashMismatch);
    }
    TokioAsyncWriteExt::flush(&mut file)
        .await
        .map_err(P2pDatasetError::Io)?;
    file.sync_all().await.map_err(P2pDatasetError::Io)?;
    drop(file);
    Ok(())
}

fn utc_deadline_as_instant(deadline: DateTime<Utc>) -> tokio::time::Instant {
    let now_instant = tokio::time::Instant::now();
    deadline
        .signed_duration_since(Utc::now())
        .to_std()
        .ok()
        .and_then(|remaining| now_instant.checked_add(remaining))
        .unwrap_or(now_instant)
}

fn candidate_timeout_error(
    availability_deadline: tokio::time::Instant,
    bounded_candidate_deadline: tokio::time::Instant,
) -> P2pDatasetError {
    if availability_deadline <= bounded_candidate_deadline {
        P2pDatasetError::ExpiredDataset
    } else {
        P2pDatasetError::TransferTimeout
    }
}

impl Drop for ActiveTransferGuard {
    fn drop(&mut self) {
        let status = {
            let mut state = self.inner.state.lock();
            state.active_transfers.remove(&self.transfer_id);
            serving_status(&state, Utc::now())
        };
        self.inner.status.send_replace(status);
    }
}

#[async_trait]
impl P2pDataset for P2pDatasetAdapter {
    async fn register(
        &self,
        registration: P2pDatasetRegistration,
    ) -> anyhow::Result<P2pDatasetReference> {
        Ok(self.register_dataset(registration).await?)
    }

    async fn fetch(
        &self,
        reference: &P2pDatasetReference,
        destination: &Path,
    ) -> anyhow::Result<()> {
        Ok(self.fetch_dataset(reference, destination).await?)
    }
}

pub struct P2pDatasetServer {
    stop: CancellationToken,
    task: Option<JoinHandle<()>>,
}

impl P2pDatasetServer {
    pub async fn shutdown(mut self) -> Result<()> {
        self.stop.cancel();
        if let Some(task) = self.task.take() {
            task.await.map_err(P2pDatasetError::ServerTask)?;
        }
        Ok(())
    }
}

impl Drop for P2pDatasetServer {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

impl P2pDatasetError {
    fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Transport(_) | Self::Io(_) | Self::InterruptedTransfer | Self::TransferTimeout
        )
    }
}

fn validate_dataset_id(dataset_id: &str) -> Result<()> {
    let dataset_id = dataset_id.trim();
    if dataset_id.is_empty() || dataset_id.len() > MAX_DATASET_ID_BYTES {
        return Err(P2pDatasetError::InvalidDatasetId);
    }
    Ok(())
}

fn validate_dataset_name(name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() || name.len() > MAX_DATASET_NAME_BYTES {
        return Err(P2pDatasetError::InvalidDatasetName);
    }
    Ok(())
}

fn sorted_unique_routes(mut routes: Vec<Multiaddr>) -> Vec<Multiaddr> {
    routes.sort_unstable_by_key(ToString::to_string);
    routes.dedup();
    routes
}

fn validate_and_sort_direct_routes(
    routes: Vec<Multiaddr>,
    local_peer_id: PeerId,
) -> Result<Vec<Multiaddr>> {
    let routes = routes
        .into_iter()
        .map(|route| validate_direct_route(&route, local_peer_id))
        .collect::<Result<Vec<_>>>()?;
    let routes = sorted_unique_routes(routes);
    if routes.len() > MAX_PUBLISHED_ROUTES {
        return Err(P2pDatasetError::RouteLimitExceeded {
            maximum: MAX_PUBLISHED_ROUTES,
        });
    }
    Ok(routes)
}

fn validate_direct_route(route: &Multiaddr, expected_peer_id: PeerId) -> Result<Multiaddr> {
    let protocols = route.iter().collect::<Vec<_>>();
    let (network, port, suffix) = match protocols.as_slice() {
        [network, Protocol::Tcp(port)] => (network, *port, None),
        [network, Protocol::Tcp(port), Protocol::P2p(peer_id)] => (network, *port, Some(*peer_id)),
        _ => {
            return Err(P2pDatasetError::InvalidDirectRoute(
                "expected exact address/tcp[/p2p] grammar".to_string(),
            ));
        }
    };
    if !matches!(
        network,
        Protocol::Ip4(_)
            | Protocol::Ip6(_)
            | Protocol::Dns(_)
            | Protocol::Dns4(_)
            | Protocol::Dns6(_)
    ) {
        return Err(P2pDatasetError::InvalidDirectRoute(
            "the address must be ip4, ip6, dns, dns4, or dns6".to_string(),
        ));
    }
    if port == 0 {
        return Err(P2pDatasetError::InvalidDirectRoute(
            "TCP port must be non-zero".to_string(),
        ));
    }
    if suffix.is_some_and(|peer_id| peer_id != expected_peer_id) {
        return Err(P2pDatasetError::InvalidDirectRoute(
            "the terminal p2p component must match the expected Peer ID".to_string(),
        ));
    }
    let mut canonical = route.clone();
    if suffix.is_some() {
        canonical.pop();
    }
    Ok(canonical)
}

fn validate_circuit_route(
    route: &Multiaddr,
    expected_relay_peer_id: PeerId,
    expected_target_peer_id: PeerId,
) -> Result<(PeerId, String)> {
    let canonical = canonicalize_circuit_route(route, expected_target_peer_id)?;
    if canonical.relay_peer_id != expected_relay_peer_id {
        return Err(P2pDatasetError::InvalidRelayRoute(
            "route relay Peer ID does not match the reservation".to_string(),
        ));
    }
    if canonical.route != *route {
        return Err(P2pDatasetError::InvalidRelayRoute(
            "relay route is not canonical".to_string(),
        ));
    }
    Ok((canonical.relay_peer_id, canonical.endpoint_key))
}

fn canonicalize_circuit_route(
    route: &Multiaddr,
    expected_target_peer_id: PeerId,
) -> Result<CanonicalCircuitRoute> {
    let mut protocols = route.iter();
    let (host, port, relay_peer_id, target_peer_id) = match (
        protocols.next(),
        protocols.next(),
        protocols.next(),
        protocols.next(),
        protocols.next(),
        protocols.next(),
    ) {
        (
            Some(Protocol::Dns4(host)),
            Some(Protocol::Tcp(port)),
            Some(Protocol::P2p(relay_peer_id)),
            Some(Protocol::P2pCircuit),
            Some(Protocol::P2p(target_peer_id)),
            None,
        ) => (host, port, relay_peer_id, target_peer_id),
        _ => {
            return Err(P2pDatasetError::InvalidRelayRoute(
                "expected exact dns4/tcp/p2p/p2p-circuit/p2p grammar".to_string(),
            ));
        }
    };
    if target_peer_id != expected_target_peer_id {
        return Err(P2pDatasetError::InvalidRelayRoute(
            "route target Peer ID does not match the expected node".to_string(),
        ));
    }
    let provider_base = format!("/dns4/{host}/tcp/{port}/p2p/{relay_peer_id}");
    let validation_limits = ExpectedRelayLimits::new(Duration::from_secs(1), 1)
        .map_err(|error| P2pDatasetError::InvalidRelayRoute(error.to_string()))?;
    let provider = RelayProvider::new(relay_peer_id, [&provider_base], validation_limits)
        .map_err(|error| P2pDatasetError::InvalidRelayRoute(error.to_string()))?;
    let canonical_base = provider.selected_base().clone();
    let mut base_protocols = canonical_base.iter();
    let (canonical_host, canonical_port) = match (
        base_protocols.next(),
        base_protocols.next(),
        base_protocols.next(),
        base_protocols.next(),
    ) {
        (Some(Protocol::Dns4(host)), Some(Protocol::Tcp(port)), Some(Protocol::P2p(_)), None) => {
            (host, port)
        }
        _ => {
            return Err(P2pDatasetError::InvalidRelayRoute(
                "canonical relay provider base has unexpected grammar".to_string(),
            ));
        }
    };
    let endpoint_key = format!("/dns4/{canonical_host}/tcp/{canonical_port}");
    let canonical_route = canonical_base
        .with(Protocol::P2pCircuit)
        .with(Protocol::P2p(target_peer_id));
    Ok(CanonicalCircuitRoute {
        route: canonical_route,
        relay_peer_id,
        endpoint_key,
    })
}

fn validate_registration_state(
    state: &ServingState,
    policy: DatasetRoutePolicy,
    domain_id: Uuid,
    now: DateTime<Utc>,
) -> Result<()> {
    if !state.registrations_open {
        return Err(P2pDatasetError::RegistrationsStopped);
    }
    if state.serving_domain != Some(domain_id) {
        return Err(P2pDatasetError::ServingDomainMismatch);
    }
    match policy {
        DatasetRoutePolicy::DirectOnly if state.direct_routes.is_empty() => {
            Err(P2pDatasetError::MissingAdvertisedAddress)
        }
        DatasetRoutePolicy::RelayRequired if publishable_relay_count(state, now) == 0 => {
            Err(P2pDatasetError::ConfirmedRelayRequired)
        }
        DatasetRoutePolicy::DirectOnly | DatasetRoutePolicy::RelayRequired => Ok(()),
    }
}

fn select_registration_routes(
    state: &ServingState,
    policy: DatasetRoutePolicy,
    size_bytes: u64,
    now: DateTime<Utc>,
) -> Result<Vec<String>> {
    let mut routes = state
        .direct_routes
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if policy == DatasetRoutePolicy::RelayRequired {
        let required_data_bytes = required_relay_data_bytes(size_bytes)?;
        let mut relay_peer_ids = HashSet::new();
        let mut relay_endpoints = HashSet::new();
        let eligible = state
            .relay_routes
            .values()
            .filter(|entry| {
                entry.publishable
                    && entry.authorized_until > now
                    && entry.limits.duration() >= MIN_CIRCUIT_DURATION
                    && entry.limits.data_bytes_per_direction() >= required_data_bytes
            })
            .filter(|entry| {
                relay_peer_ids.insert(entry.relay_peer_id)
                    && relay_endpoints.insert(entry.endpoint.as_str())
            })
            .map(|entry| entry.route.to_string())
            .collect::<Vec<_>>();
        if eligible.is_empty() {
            return Err(P2pDatasetError::NoEligibleRelayRoute);
        }
        if eligible.len() > MAX_CIRCUIT_ROUTES {
            return Err(P2pDatasetError::CircuitRouteLimitExceeded {
                maximum: MAX_CIRCUIT_ROUTES,
            });
        }
        routes.extend(eligible);
    }
    if routes.len() > MAX_PUBLISHED_ROUTES {
        return Err(P2pDatasetError::RouteLimitExceeded {
            maximum: MAX_PUBLISHED_ROUTES,
        });
    }
    Ok(routes)
}

fn required_relay_data_bytes(size_bytes: u64) -> Result<u64> {
    let quotient = size_bytes / 10;
    let remainder = u64::from(!size_bytes.is_multiple_of(10));
    let ten_percent = quotient
        .checked_add(remainder)
        .ok_or(P2pDatasetError::RelayBudgetOverflow)?;
    size_bytes
        .checked_add(ten_percent)
        .and_then(|value| value.checked_add(RELAY_DATA_OVERHEAD_BYTES))
        .ok_or(P2pDatasetError::RelayBudgetOverflow)
}

fn publishable_relay_count(state: &ServingState, now: DateTime<Utc>) -> usize {
    state
        .relay_routes
        .values()
        .filter(|entry| entry.publishable && entry.authorized_until > now)
        .count()
}

fn serving_status(state: &ServingState, now: DateTime<Utc>) -> DatasetServingStatus {
    DatasetServingStatus {
        route_set_revision: state.route_set_revision,
        registrations_open: state.registrations_open,
        direct_route_count: state.direct_routes.len(),
        confirmed_relay_count: publishable_relay_count(state, now),
        max_available_until: state
            .registry
            .values()
            .filter(|entry| entry.available_until > now)
            .map(|entry| entry.available_until)
            .max(),
        active_transfer_count: state.active_transfers.len(),
        max_active_transfer_deadline: state
            .active_transfers
            .values()
            .map(|transfer| transfer.deadline)
            .max(),
    }
}

fn bump_route_revision(state: &mut ServingState) -> Result<u64> {
    advance_route_revision(state, 1)?;
    Ok(state.route_set_revision)
}

fn advance_route_revision(state: &mut ServingState, count: usize) -> Result<()> {
    let count = u64::try_from(count).map_err(|_| P2pDatasetError::RouteRevisionExhausted)?;
    state.route_set_revision = state
        .route_set_revision
        .checked_add(count)
        .ok_or(P2pDatasetError::RouteRevisionExhausted)?;
    Ok(())
}

fn expire_relay_authorizations(state: &mut ServingState, now: DateTime<Utc>) -> Result<()> {
    let expired = state
        .relay_routes
        .values()
        .filter(|entry| entry.publishable && entry.authorized_until <= now)
        .map(|entry| entry.fence.slot_id)
        .collect::<Vec<_>>();
    advance_route_revision(state, expired.len())?;
    for slot_id in expired {
        if let Some(entry) = state.relay_routes.get_mut(&slot_id) {
            entry.publishable = false;
        }
    }
    Ok(())
}

fn validate_reference(
    reference: &P2pDatasetReference,
) -> Result<(PeerId, Vec<DatasetRouteCandidate>)> {
    validate_dataset_id(&reference.dataset_id)?;
    validate_dataset_name(&reference.name)?;
    if reference.schema != P2P_DATASET_SCHEMA
        || reference.available_until <= Utc::now()
        || reference.multiaddrs.is_empty()
        || reference.size_bytes == 0
    {
        return Err(if reference.available_until <= Utc::now() {
            P2pDatasetError::ExpiredDataset
        } else {
            P2pDatasetError::InvalidReference
        });
    }
    if reference.multiaddrs.len() > MAX_PUBLISHED_ROUTES {
        return Err(P2pDatasetError::RouteLimitExceeded {
            maximum: MAX_PUBLISHED_ROUTES,
        });
    }
    if reference.sha256.len() != 64 {
        return Err(P2pDatasetError::InvalidReference);
    }
    let hash = hex::decode(&reference.sha256).map_err(|_| P2pDatasetError::InvalidReference)?;
    if hash.len() != 32 {
        return Err(P2pDatasetError::InvalidReference);
    }
    let peer_id =
        PeerId::from_str(&reference.peer_id).map_err(|_| P2pDatasetError::InvalidReference)?;

    let mut parsed = Vec::with_capacity(reference.multiaddrs.len());
    for (route_index, raw) in reference.multiaddrs.iter().enumerate() {
        if raw.len() > MAX_ROUTE_TEXT_BYTES {
            warn!(
                route_index,
                route_bytes = raw.len(),
                maximum = MAX_ROUTE_TEXT_BYTES,
                "skipping oversized P2P dataset route candidate"
            );
            continue;
        }
        match Multiaddr::from_str(raw) {
            Ok(route) => parsed.push((route_index, raw.as_str(), route)),
            Err(error) => {
                warn!(
                    route_index,
                    route = %raw,
                    error = %error,
                    "skipping malformed P2P dataset route candidate"
                );
            }
        }
    }
    let circuit_count = parsed
        .iter()
        .filter(|(_, _, route)| {
            route
                .iter()
                .any(|protocol| matches!(protocol, Protocol::P2pCircuit))
        })
        .count();
    if circuit_count > MAX_CIRCUIT_ROUTES {
        return Err(P2pDatasetError::CircuitRouteLimitExceeded {
            maximum: MAX_CIRCUIT_ROUTES,
        });
    }

    let mut direct = Vec::new();
    let mut circuit = Vec::new();
    let mut direct_routes = HashSet::new();
    let mut relay_peer_ids = HashSet::new();
    let mut relay_endpoint_keys = HashSet::new();
    for (route_index, raw, route) in parsed {
        let is_circuit = route
            .iter()
            .any(|protocol| matches!(protocol, Protocol::P2pCircuit));
        if is_circuit {
            let canonical = match canonicalize_circuit_route(&route, peer_id) {
                Ok(canonical) => canonical,
                Err(error) => {
                    warn!(
                        route_index,
                        route = %raw,
                        error = %error,
                        "skipping unsafe P2P dataset circuit candidate"
                    );
                    continue;
                }
            };
            if relay_peer_ids.contains(&canonical.relay_peer_id)
                || relay_endpoint_keys.contains(&canonical.endpoint_key)
            {
                warn!(
                    route_index,
                    route = %raw,
                    relay_peer_id = %canonical.relay_peer_id,
                    endpoint_key = %canonical.endpoint_key,
                    "skipping duplicate P2P dataset circuit candidate"
                );
                continue;
            }
            relay_peer_ids.insert(canonical.relay_peer_id);
            relay_endpoint_keys.insert(canonical.endpoint_key);
            circuit.push(DatasetRouteCandidate::Circuit(canonical.route));
        } else {
            let canonical = match validate_direct_route(&route, peer_id) {
                Ok(canonical) => canonical,
                Err(error) => {
                    warn!(
                        route_index,
                        route = %raw,
                        error = %error,
                        "skipping unsafe P2P dataset direct candidate"
                    );
                    continue;
                }
            };
            if !direct_routes.insert(canonical.to_string()) {
                warn!(
                    route_index,
                    route = %raw,
                    "skipping duplicate P2P dataset direct candidate"
                );
                continue;
            }
            direct.push(DatasetRouteCandidate::Direct(canonical));
        }
    }
    direct.extend(circuit);
    if direct.is_empty() {
        return Err(P2pDatasetError::InvalidReference);
    }
    Ok((peer_id, direct))
}

async fn hash_file(path: &Path) -> Result<(u64, String)> {
    let mut file = File::open(path).await.map_err(P2pDatasetError::Io)?;
    let mut hasher = Sha256::new();
    let mut size_bytes = 0_u64;
    let mut buffer = vec![0_u8; TRANSFER_BUFFER_BYTES];
    loop {
        let count = TokioAsyncReadExt::read(&mut file, &mut buffer)
            .await
            .map_err(P2pDatasetError::Io)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        size_bytes += count as u64;
    }
    Ok((size_bytes, hex::encode(hasher.finalize())))
}

fn prune_expired(registry: &mut HashMap<String, RegisteredDataset>, now: DateTime<Utc>) {
    registry.retain(|_, dataset| dataset.available_until > now);
}

fn partial_path(destination: &Path) -> PathBuf {
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("dataset");
    destination.with_file_name(format!(".{file_name}.{}.part", Uuid::new_v4()))
}

async fn write_json_frame<W, T>(writer: &mut W, value: &T, limit: usize) -> Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let encoded = serde_json::to_vec(value).map_err(P2pDatasetError::Json)?;
    if encoded.len() > limit || encoded.len() > u32::MAX as usize {
        return Err(P2pDatasetError::FrameTooLarge);
    }
    writer
        .write_all(&(encoded.len() as u32).to_be_bytes())
        .await
        .map_err(P2pDatasetError::Io)?;
    writer
        .write_all(&encoded)
        .await
        .map_err(P2pDatasetError::Io)?;
    Ok(())
}

async fn read_json_frame<R, T>(reader: &mut R, limit: usize) -> Result<T>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let mut length = [0_u8; 4];
    reader
        .read_exact(&mut length)
        .await
        .map_err(P2pDatasetError::Io)?;
    let length = u32::from_be_bytes(length) as usize;
    if length > limit {
        return Err(P2pDatasetError::FrameTooLarge);
    }
    let mut encoded = vec![0_u8; length];
    reader
        .read_exact(&mut encoded)
        .await
        .map_err(P2pDatasetError::Io)?;
    serde_json::from_slice(&encoded).map_err(P2pDatasetError::Json)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use auki_p2p::{
        DdsTokenVerifier, Identity, P2PAccessClaims, P2P_TOKEN_AUDIENCE, P2P_TOKEN_ISSUER,
        P2P_TOKEN_SCOPE, P2P_TOKEN_TTL, P2P_TOKEN_TYPE,
    };
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use tempfile::TempDir;
    use tokio::sync::oneshot;

    use super::*;

    const TEST_DDS_PRIVATE_KEY: &[u8] = br#"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQggm4twpf4y/yNNw/k
fqecEEl4zBTwZdRDFUFp/fSxV8qhRANCAARUxrDWJ0AtEGTAYZ4412VPHqMCKoPw
UphDkcOIk7SODsKwUvTIiUr11NbXBJmbBRfhERczsuK4PVha5eg0fVqo
-----END PRIVATE KEY-----"#;

    const TEST_DDS_PUBLIC_KEY: &[u8] = br#"-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEVMaw1idALRBkwGGeONdlTx6jAiqD
8FKYQ5HDiJO0jg7CsFL0yIlK9dTW1wSZmwUX4REXM7LiuD1YWuXoNH1aqA==
-----END PUBLIC KEY-----"#;

    #[test]
    fn fetch_budgets_timeout_causes_and_retry_classes_are_frozen() {
        assert_eq!(FETCH_ATTEMPTS, 2);
        assert_eq!(FETCH_ATTEMPT_TIMEOUT, Duration::from_secs(900));
        assert_eq!(FETCH_CANDIDATE_TIMEOUT, Duration::from_secs(10));

        let now = tokio::time::Instant::now();
        assert!(matches!(
            candidate_timeout_error(now, now + Duration::from_secs(1)),
            P2pDatasetError::ExpiredDataset
        ));
        assert!(matches!(
            candidate_timeout_error(now + Duration::from_secs(1), now),
            P2pDatasetError::TransferTimeout
        ));
        assert!(matches!(
            candidate_timeout_error(now, now),
            P2pDatasetError::ExpiredDataset
        ));

        for retryable in [
            P2pDatasetError::Transport(auki_p2p::Error::SwarmStopped),
            P2pDatasetError::Io(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "stream interrupted",
            )),
            P2pDatasetError::InterruptedTransfer,
            P2pDatasetError::TransferTimeout,
        ] {
            assert!(
                retryable.is_retryable(),
                "{retryable} must consume one round"
            );
        }
        for terminal in [
            P2pDatasetError::ReferenceMismatch,
            P2pDatasetError::SizeMismatch,
            P2pDatasetError::HashMismatch,
            P2pDatasetError::ExpiredDataset,
        ] {
            assert!(!terminal.is_retryable(), "{terminal} must fail immediately");
        }
    }

    #[tokio::test]
    async fn partial_file_cleanup_failure_retains_guard_ownership() {
        let temp = TempDir::new().unwrap();
        let directory = temp.path().join("not-a-removable-file.part");
        fs::create_dir(&directory).await.unwrap();
        let mut guard = PartialFileGuard::new(directory);

        assert!(matches!(
            guard.cleanup().await,
            Err(P2pDatasetError::PartialFileCleanup(_))
        ));
        assert!(guard.armed, "failed cleanup silently disarmed the guard");
        guard.disarm();
    }

    #[test]
    fn relay_data_budget_is_exact_and_overflow_checked() {
        assert_eq!(
            required_relay_data_bytes(1).unwrap(),
            1 + 1 + RELAY_DATA_OVERHEAD_BYTES
        );
        assert_eq!(
            required_relay_data_bytes(10).unwrap(),
            10 + 1 + RELAY_DATA_OVERHEAD_BYTES
        );
        assert_eq!(
            required_relay_data_bytes(11).unwrap(),
            11 + 2 + RELAY_DATA_OVERHEAD_BYTES
        );
        assert!(matches!(
            required_relay_data_bytes(u64::MAX),
            Err(P2pDatasetError::RelayBudgetOverflow)
        ));
    }

    #[test]
    fn direct_route_validation_enforces_bounds_and_local_peer_suffix() {
        let local = Identity::generate().peer_id();
        let other = Identity::generate().peer_id();
        let matching: Multiaddr = format!("/ip4/192.0.2.1/tcp/4001/p2p/{local}")
            .parse()
            .unwrap();
        assert_eq!(
            validate_and_sort_direct_routes(vec![matching.clone(), matching], local)
                .unwrap()
                .len(),
            1
        );
        let wrong: Multiaddr = format!("/ip4/192.0.2.1/tcp/4001/p2p/{other}")
            .parse()
            .unwrap();
        assert!(matches!(
            validate_and_sort_direct_routes(vec![wrong], local),
            Err(P2pDatasetError::InvalidDirectRoute(_))
        ));
        let too_many = (1..=MAX_PUBLISHED_ROUTES + 1)
            .map(|port| format!("/ip4/192.0.2.1/tcp/{port}").parse().unwrap())
            .collect();
        assert!(matches!(
            validate_and_sort_direct_routes(too_many, local),
            Err(P2pDatasetError::RouteLimitExceeded {
                maximum: MAX_PUBLISHED_ROUTES
            })
        ));
    }

    #[test]
    fn direct_route_validation_freezes_grammar_and_normalizes_peer_suffix() {
        let target = Identity::generate().peer_id();
        let other = Identity::generate().peer_id();
        let bare: Multiaddr = "/ip4/192.0.2.10/tcp/4001".parse().unwrap();
        let suffixed: Multiaddr = format!("{bare}/p2p/{target}").parse().unwrap();
        assert_eq!(
            validate_direct_route(&bare, target).unwrap(),
            validate_direct_route(&suffixed, target).unwrap()
        );

        for route in [
            "/ip6/2001:db8::1/tcp/4001".to_string(),
            "/dns/relay.dev.aukiverse.com/tcp/4001".to_string(),
            "/dns4/relay.dev.aukiverse.com/tcp/4001".to_string(),
            "/dns6/relay.dev.aukiverse.com/tcp/4001".to_string(),
        ] {
            assert!(validate_direct_route(&route.parse().unwrap(), target).is_ok());
        }

        for route in [
            "/ip4/192.0.2.10/tcp/0".to_string(),
            "/ip4/192.0.2.10/udp/4001".to_string(),
            "/ip4/192.0.2.10/tcp/4001/ws".to_string(),
            format!("/ip4/192.0.2.10/tcp/4001/p2p/{other}"),
        ] {
            assert!(matches!(
                validate_direct_route(&route.parse().unwrap(), target),
                Err(P2pDatasetError::InvalidDirectRoute(_))
            ));
        }
    }

    #[test]
    fn reference_routes_are_canonical_deduplicated_and_stably_direct_first() {
        let target = Identity::generate().peer_id();
        let relay_a = Identity::generate().peer_id();
        let relay_b = Identity::generate().peer_id();
        let direct_a = "/ip4/192.0.2.20/tcp/4020";
        let direct_b = "/dns4/robot.dev.aukiverse.com/tcp/4021";
        let circuit_a = format!(
            "/dns4/relay-a.dev.aukiverse.com/tcp/4101/p2p/{relay_a}/p2p-circuit/p2p/{target}"
        );
        let circuit_b = format!(
            "/dns4/relay-b.dev.aukiverse.com/tcp/4102/p2p/{relay_b}/p2p-circuit/p2p/{target}"
        );
        let reference = reference_with_routes(
            target,
            vec![
                circuit_a.clone(),
                format!("{direct_a}/p2p/{target}"),
                direct_a.to_string(),
                "not-a-multiaddr".to_string(),
                direct_b.to_string(),
                circuit_b.clone(),
            ],
        );

        let (_, candidates) = validate_reference(&reference).unwrap();
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| (candidate.kind(), candidate.route().to_string()))
                .collect::<Vec<_>>(),
            vec![
                ("direct", direct_a.to_string()),
                ("direct", direct_b.to_string()),
                ("circuit", circuit_a),
                ("circuit", circuit_b),
            ]
        );
    }

    #[test]
    fn reference_circuit_routes_deduplicate_relay_and_endpoint_independently() {
        let target = Identity::generate().peer_id();
        let relay_a = Identity::generate().peer_id();
        let relay_b = Identity::generate().peer_id();
        let relay_c = Identity::generate().peer_id();
        let route = |host: &str, port: u16, relay: PeerId| {
            format!("/dns4/{host}/tcp/{port}/p2p/{relay}/p2p-circuit/p2p/{target}")
        };

        let first_a = route("relay-a.dev.aukiverse.com", 4101, relay_a);
        let distinct_b = route("relay-b.dev.aukiverse.com", 4102, relay_b);
        let (_, by_peer) = validate_reference(&reference_with_routes(
            target,
            vec![
                first_a.clone(),
                route("relay-a-alt.dev.aukiverse.com", 4199, relay_a),
                distinct_b.clone(),
            ],
        ))
        .unwrap();
        assert_eq!(
            by_peer
                .iter()
                .map(|candidate| candidate.route().to_string())
                .collect::<Vec<_>>(),
            vec![first_a.clone(), distinct_b]
        );

        let distinct_c = route("relay-c.dev.aukiverse.com", 4103, relay_c);
        let (_, by_endpoint) = validate_reference(&reference_with_routes(
            target,
            vec![
                first_a.clone(),
                route("relay-a.dev.aukiverse.com", 4101, relay_b),
                distinct_c.clone(),
            ],
        ))
        .unwrap();
        assert_eq!(
            by_endpoint
                .iter()
                .map(|candidate| candidate.route().to_string())
                .collect::<Vec<_>>(),
            vec![first_a, distinct_c]
        );
    }

    #[test]
    fn reference_route_bounds_count_parsed_circuits_before_dedup() {
        let target = Identity::generate().peer_id();
        let seventeen = (1..=MAX_PUBLISHED_ROUTES + 1)
            .map(|port| format!("/ip4/192.0.2.30/tcp/{port}"))
            .collect();
        assert!(matches!(
            validate_reference(&reference_with_routes(target, seventeen)),
            Err(P2pDatasetError::RouteLimitExceeded {
                maximum: MAX_PUBLISHED_ROUTES
            })
        ));

        let four_circuits = (0..=MAX_CIRCUIT_ROUTES)
            .map(|index| {
                let relay = Identity::generate().peer_id();
                format!(
                    "/dns4/relay-{index}.dev.aukiverse.com/tcp/{}/p2p/{relay}/p2p-circuit/p2p/{target}",
                    4200 + index
                )
            })
            .collect();
        assert!(matches!(
            validate_reference(&reference_with_routes(target, four_circuits)),
            Err(P2pDatasetError::CircuitRouteLimitExceeded {
                maximum: MAX_CIRCUIT_ROUTES
            })
        ));

        let dns_value_named_like_protocol = (4300..4304)
            .map(|port| format!("/dns4/p2p-circuit/tcp/{port}"))
            .collect();
        let (_, direct) = validate_reference(&reference_with_routes(
            target,
            dns_value_named_like_protocol,
        ))
        .unwrap();
        assert_eq!(direct.len(), 4);
        assert!(direct
            .iter()
            .all(|candidate| matches!(candidate, DatasetRouteCandidate::Direct(_))));

        let oversized = format!("/dns4/{}/tcp/443", "a".repeat(MAX_ROUTE_TEXT_BYTES));
        let (_, bounded) = validate_reference(&reference_with_routes(
            target,
            vec![oversized, "/ip4/192.0.2.31/tcp/443".to_string()],
        ))
        .unwrap();
        assert_eq!(bounded.len(), 1);
    }

    #[tokio::test]
    async fn direct_and_circuit_routes_share_the_sixteen_route_bound() {
        let node = unbound_node();
        let credentials = P2pCredentialStore::new(node.clone());
        let adapter = P2pDatasetAdapter::new_with_route_policy(
            node.clone(),
            credentials,
            Vec::new(),
            DatasetRoutePolicy::RelayRequired,
        )
        .unwrap();
        install_synthetic_relay(
            &adapter,
            0,
            Duration::from_secs(900),
            RELAY_DATA_OVERHEAD_BYTES,
        );
        let routes = |count: usize| {
            (1..=count)
                .map(|port| format!("/ip4/192.0.2.1/tcp/{port}").parse().unwrap())
                .collect::<Vec<Multiaddr>>()
        };
        adapter.replace_direct_routes(routes(15)).unwrap();
        assert!(matches!(
            adapter.replace_direct_routes(routes(16)),
            Err(P2pDatasetError::RouteLimitExceeded {
                maximum: MAX_PUBLISHED_ROUTES
            })
        ));
        assert_eq!(adapter.serving_status().unwrap().direct_route_count, 15);
        node.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn reading_serving_status_does_not_wake_its_own_drain_watcher() {
        let node = unbound_node();
        let credentials = P2pCredentialStore::new(node.clone());
        let adapter = P2pDatasetAdapter::new_with_route_policy(
            node.clone(),
            credentials,
            Vec::new(),
            DatasetRoutePolicy::RelayRequired,
        )
        .unwrap();
        let mut status = adapter.subscribe_serving_status();

        adapter.serving_status().unwrap();

        assert!(
            tokio::time::timeout(Duration::from_millis(20), status.changed())
                .await
                .is_err()
        );
        node.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn relay_required_allows_empty_direct_serving_and_filters_limits_exactly() {
        let domain_id = Uuid::new_v4();
        let (node, adapter, server) = relay_serving_adapter(domain_id).await;
        let temp = TempDir::new().unwrap();
        let bytes = zip_like_bytes(2048);
        let source = temp.path().join("limits.zip");
        fs::write(&source, &bytes).await.unwrap();
        let registration = registration("limits", domain_id, source);

        assert!(matches!(
            adapter.register_dataset(registration.clone()).await,
            Err(P2pDatasetError::ConfirmedRelayRequired)
        ));

        let required = required_relay_data_bytes(bytes.len() as u64).unwrap();
        let under = install_synthetic_relay(&adapter, 0, Duration::from_secs(900), required - 1);
        assert!(matches!(
            adapter.register_dataset(registration.clone()).await,
            Err(P2pDatasetError::NoEligibleRelayRoute)
        ));

        tombstone_synthetic_relay(&adapter, under);
        let route = install_synthetic_relay(&adapter, 1, Duration::from_secs(900), required);
        let reference = adapter.register_dataset(registration).await.unwrap();
        assert_eq!(
            reference.multiaddrs,
            vec![synthetic_route(&adapter, 1).to_string()]
        );
        assert_eq!(adapter.serving_status().unwrap().confirmed_relay_count, 1);

        adapter
            .refresh_confirmed_relay_authorization(
                route,
                Utc::now() + chrono::Duration::minutes(10),
            )
            .unwrap();
        server.shutdown().await.unwrap();
        node.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn duration_899_is_omitted_while_900_second_sibling_is_published() {
        let domain_id = Uuid::new_v4();
        let (node, adapter, server) = relay_serving_adapter(domain_id).await;
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("duration.zip");
        let bytes = zip_like_bytes(512);
        fs::write(&source, &bytes).await.unwrap();
        let required = required_relay_data_bytes(bytes.len() as u64).unwrap();

        install_synthetic_relay(&adapter, 0, Duration::from_secs(899), required);
        install_synthetic_relay(&adapter, 1, Duration::from_secs(900), required);
        let reference = adapter
            .register_dataset(registration("duration", domain_id, source))
            .await
            .unwrap();
        assert_eq!(
            reference.multiaddrs,
            vec![synthetic_route(&adapter, 1).to_string()]
        );

        server.shutdown().await.unwrap();
        node.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn new_sibling_during_hash_forces_current_a_plus_b_snapshot() {
        let domain_id = Uuid::new_v4();
        let (node, adapter, server) = relay_serving_adapter(domain_id).await;
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("race.zip");
        let bytes = zip_like_bytes(1024);
        fs::write(&source, &bytes).await.unwrap();
        let required = required_relay_data_bytes(bytes.len() as u64).unwrap();
        install_synthetic_relay(&adapter, 0, Duration::from_secs(900), required);

        let (captured_tx, captured_rx) = oneshot::channel();
        let (resume_tx, resume_rx) = oneshot::channel();
        let registering = adapter.clone();
        let registration = registration("race", domain_id, source);
        let task = tokio::spawn(async move {
            registering
                .register_dataset_with_hasher(registration, move |path| async move {
                    let _ = captured_tx.send(());
                    let _ = resume_rx.await;
                    hash_file(&path).await
                })
                .await
        });
        captured_rx.await.unwrap();
        install_synthetic_relay(&adapter, 1, Duration::from_secs(900), required);
        resume_tx.send(()).unwrap();
        let reference = task.await.unwrap().unwrap();
        assert_eq!(
            reference.multiaddrs,
            vec![
                synthetic_route(&adapter, 0).to_string(),
                synthetic_route(&adapter, 1).to_string(),
            ]
        );

        server.shutdown().await.unwrap();
        node.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn count_one_loss_during_hash_cannot_commit_a_stale_route() {
        let domain_id = Uuid::new_v4();
        let (node, adapter, server) = relay_serving_adapter(domain_id).await;
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("lost.zip");
        let bytes = zip_like_bytes(1024);
        fs::write(&source, &bytes).await.unwrap();
        let required = required_relay_data_bytes(bytes.len() as u64).unwrap();
        let fence = install_synthetic_relay(&adapter, 0, Duration::from_secs(900), required);

        let (captured_tx, captured_rx) = oneshot::channel();
        let (resume_tx, resume_rx) = oneshot::channel();
        let registering = adapter.clone();
        let registration = registration("lost", domain_id, source);
        let task = tokio::spawn(async move {
            registering
                .register_dataset_with_hasher(registration, move |path| async move {
                    let _ = captured_tx.send(());
                    let _ = resume_rx.await;
                    hash_file(&path).await
                })
                .await
        });
        captured_rx.await.unwrap();
        tombstone_synthetic_relay(&adapter, fence);
        resume_tx.send(()).unwrap();
        assert!(matches!(
            task.await.unwrap(),
            Err(P2pDatasetError::ConfirmedRelayRequired)
        ));
        assert!(!adapter.inner.state.lock().registry.contains_key("lost"));

        server.shutdown().await.unwrap();
        node.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn authorization_refresh_and_drain_state_are_fenced_and_observable() {
        let domain_id = Uuid::new_v4();
        let (node, adapter, server) = relay_serving_adapter(domain_id).await;
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("drain.zip");
        let bytes = zip_like_bytes(256);
        fs::write(&source, &bytes).await.unwrap();
        let required = required_relay_data_bytes(bytes.len() as u64).unwrap();
        let mut status_updates = adapter.subscribe_serving_status();
        let fence = install_synthetic_relay(&adapter, 0, Duration::from_secs(900), required);
        status_updates.changed().await.unwrap();
        assert_eq!(status_updates.borrow().confirmed_relay_count, 1);
        let initial_revision = adapter.serving_status().unwrap().route_set_revision;

        let refreshed = adapter
            .refresh_confirmed_relay_authorization(fence, Utc::now() + chrono::Duration::hours(1))
            .unwrap();
        assert_eq!(refreshed, initial_revision);
        let expired = adapter
            .refresh_confirmed_relay_authorization(fence, Utc::now() - chrono::Duration::seconds(1))
            .unwrap();
        assert_eq!(expired, initial_revision + 1);
        assert_eq!(adapter.serving_status().unwrap().confirmed_relay_count, 0);
        let republished = adapter
            .refresh_confirmed_relay_authorization(fence, Utc::now() + chrono::Duration::hours(1))
            .unwrap();
        assert_eq!(republished, initial_revision + 2);

        let available_until = Utc::now() + chrono::Duration::minutes(5);
        let mut drain_registration = registration("drain", domain_id, source);
        drain_registration.available_until = available_until;
        adapter.register_dataset(drain_registration).await.unwrap();
        let stopped = adapter.stop_registrations().unwrap();
        assert!(!stopped.registrations_open);
        assert_eq!(stopped.max_available_until, Some(available_until));
        let (_entry, guard) = adapter.begin_transfer("drain").unwrap();
        let active = adapter.serving_status().unwrap();
        assert_eq!(active.active_transfer_count, 1);
        assert!(active.max_active_transfer_deadline > Some(Utc::now()));
        drop(guard);
        assert_eq!(adapter.serving_status().unwrap().active_transfer_count, 0);

        let next = temp.path().join("blocked.zip");
        fs::write(&next, zip_like_bytes(64)).await.unwrap();
        assert!(matches!(
            adapter
                .register_dataset(registration("blocked", domain_id, next))
                .await,
            Err(P2pDatasetError::RegistrationsStopped)
        ));
        adapter.start_registrations().unwrap();

        server.shutdown().await.unwrap();
        node.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn registered_dataset_survives_registration_scope_and_streams_without_buffering() {
        let domain_id = Uuid::new_v4();
        let robot = listening_node();
        let robot_address = listen_address(&robot).await;
        let robot_credentials = P2pCredentialStore::new(robot.clone());
        install_current_token(
            &robot_credentials,
            &robot,
            PeerRole::Robot,
            domain_id,
            unix_time(),
        )
        .await;
        let robot_adapter =
            P2pDatasetAdapter::new(robot.clone(), robot_credentials, vec![robot_address]);
        let shutdown = CancellationToken::new();
        let server = robot_adapter
            .start_serving(domain_id, &shutdown)
            .await
            .unwrap();

        let compute = unbound_node();
        let compute_credentials = P2pCredentialStore::new(compute.clone());
        install_current_token(
            &compute_credentials,
            &compute,
            PeerRole::Compute,
            domain_id,
            unix_time(),
        )
        .await;
        let compute_adapter = P2pDatasetAdapter::new(compute.clone(), compute_credentials, vec![]);

        let temp = TempDir::new().unwrap();
        let first_bytes = zip_like_bytes(3 * TRANSFER_BUFFER_BYTES + 17);
        let first_path = temp.path().join("first.zip");
        fs::write(&first_path, &first_bytes).await.unwrap();
        let first_reference = robot_adapter
            .register_dataset(registration("first", domain_id, first_path))
            .await
            .unwrap();

        let second_path = temp.path().join("second.zip");
        fs::write(&second_path, zip_like_bytes(128)).await.unwrap();
        robot_adapter
            .register_dataset(registration("second", domain_id, second_path))
            .await
            .unwrap();

        let destination = temp.path().join("downloaded.zip");
        compute_adapter
            .fetch_dataset(&first_reference, &destination)
            .await
            .unwrap();
        assert_eq!(fs::read(&destination).await.unwrap(), first_bytes);
        assert_no_partial_files(&temp).await;

        tokio::time::timeout(Duration::from_secs(2), server.shutdown())
            .await
            .expect("dataset server shutdown timed out")
            .unwrap();
        robot.shutdown().await.unwrap();
        compute.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unknown_expired_and_mismatched_references_fail_without_partial_files() {
        let domain_id = Uuid::new_v4();
        let robot = listening_node();
        let robot_address = listen_address(&robot).await;
        let robot_credentials = P2pCredentialStore::new(robot.clone());
        install_current_token(
            &robot_credentials,
            &robot,
            PeerRole::Robot,
            domain_id,
            unix_time(),
        )
        .await;
        let robot_adapter =
            P2pDatasetAdapter::new(robot.clone(), robot_credentials, vec![robot_address]);
        let shutdown = CancellationToken::new();
        let server = robot_adapter
            .start_serving(domain_id, &shutdown)
            .await
            .unwrap();

        let compute = unbound_node();
        let compute_credentials = P2pCredentialStore::new(compute.clone());
        install_current_token(
            &compute_credentials,
            &compute,
            PeerRole::Compute,
            domain_id,
            unix_time(),
        )
        .await;
        let compute_adapter = P2pDatasetAdapter::new(compute.clone(), compute_credentials, vec![]);
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.zip");
        fs::write(&source, zip_like_bytes(4096)).await.unwrap();
        let reference = robot_adapter
            .register_dataset(registration("known", domain_id, source))
            .await
            .unwrap();

        let mut unknown = reference.clone();
        unknown.dataset_id = "unknown".into();
        assert!(compute_adapter
            .fetch_dataset(&unknown, &temp.path().join("unknown.zip"))
            .await
            .is_err());

        let mut expired = reference.clone();
        expired.available_until = Utc::now() - chrono::Duration::seconds(1);
        assert!(matches!(
            compute_adapter
                .fetch_dataset(&expired, &temp.path().join("expired.zip"))
                .await,
            Err(P2pDatasetError::ExpiredDataset)
        ));

        let mut wrong_size = reference.clone();
        wrong_size.size_bytes += 1;
        assert!(matches!(
            compute_adapter
                .fetch_dataset(&wrong_size, &temp.path().join("wrong-size.zip"))
                .await,
            Err(P2pDatasetError::ReferenceMismatch)
        ));

        let mut wrong_hash = reference;
        wrong_hash.sha256 = "00".repeat(32);
        assert!(matches!(
            compute_adapter
                .fetch_dataset(&wrong_hash, &temp.path().join("wrong-hash.zip"))
                .await,
            Err(P2pDatasetError::ReferenceMismatch)
        ));
        assert_no_partial_files(&temp).await;

        server.shutdown().await.unwrap();
        robot.shutdown().await.unwrap();
        compute.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn receiver_rejects_body_size_and_hash_mismatches_without_completion() {
        let domain_id = Uuid::new_v4();
        let expected = zip_like_bytes(TRANSFER_BUFFER_BYTES + 29);
        let expected_sha256 = hex::encode(Sha256::digest(&expected));

        let robot = listening_node();
        let robot_address = listen_address(&robot).await;
        let robot_credentials = P2pCredentialStore::new(robot.clone());
        install_current_token(
            &robot_credentials,
            &robot,
            PeerRole::Robot,
            domain_id,
            unix_time(),
        )
        .await;
        let mut incoming = robot
            .accept(
                ApplicationProtocol::new(DATASET_PROTOCOL).unwrap(),
                SessionRequirements::new(domain_id.to_string(), PeerRole::Compute).unwrap(),
            )
            .unwrap();

        let compute = unbound_node();
        let compute_credentials = P2pCredentialStore::new(compute.clone());
        install_current_token(
            &compute_credentials,
            &compute,
            PeerRole::Compute,
            domain_id,
            unix_time(),
        )
        .await;
        let compute_adapter = P2pDatasetAdapter::new(compute.clone(), compute_credentials, vec![]);
        let reference = |dataset_id: &str| P2pDatasetReference {
            schema: P2P_DATASET_SCHEMA.into(),
            dataset_id: dataset_id.into(),
            domain_id,
            name: format!("{dataset_id}.zip"),
            peer_id: robot.peer_id().to_string(),
            multiaddrs: vec![robot_address.to_string()],
            size_bytes: expected.len() as u64,
            sha256: expected_sha256.clone(),
            available_until: Utc::now() + chrono::Duration::minutes(10),
        };
        let hash_reference = reference("wrong-body-hash");
        let size_reference = reference("wrong-body-size");

        let server_bytes = expected.clone();
        let server_sha256 = expected_sha256.clone();
        let server = tokio::spawn(async move {
            let mut hash_stream = incoming.accept().await.unwrap().unwrap();
            let hash_request: DatasetRequest = read_json_frame(&mut hash_stream, MAX_REQUEST_BYTES)
                .await
                .unwrap();
            write_json_frame(
                &mut hash_stream,
                &DatasetResponseHeader {
                    dataset_id: hash_request.dataset_id,
                    size_bytes: server_bytes.len() as u64,
                    sha256: server_sha256.clone(),
                },
                MAX_RESPONSE_HEADER_BYTES,
            )
            .await
            .unwrap();
            let mut corrupted = server_bytes.clone();
            *corrupted.last_mut().unwrap() ^= 0xff;
            hash_stream.write_all(&corrupted).await.unwrap();
            hash_stream.close().await.unwrap();

            let mut size_stream = incoming.accept().await.unwrap().unwrap();
            let size_request: DatasetRequest = read_json_frame(&mut size_stream, MAX_REQUEST_BYTES)
                .await
                .unwrap();
            write_json_frame(
                &mut size_stream,
                &DatasetResponseHeader {
                    dataset_id: size_request.dataset_id,
                    size_bytes: server_bytes.len() as u64,
                    sha256: server_sha256,
                },
                MAX_RESPONSE_HEADER_BYTES,
            )
            .await
            .unwrap();
            size_stream.write_all(&server_bytes).await.unwrap();
            size_stream.write_all(&[0xff]).await.unwrap();
            size_stream.close().await.unwrap();
        });

        let temp = TempDir::new().unwrap();
        let hash_destination = temp.path().join("wrong-body-hash.zip");
        assert!(matches!(
            compute_adapter
                .fetch_dataset(&hash_reference, &hash_destination)
                .await,
            Err(P2pDatasetError::HashMismatch)
        ));
        assert!(fs::metadata(&hash_destination).await.is_err());

        let size_destination = temp.path().join("wrong-body-size.zip");
        assert!(matches!(
            compute_adapter
                .fetch_dataset(&size_reference, &size_destination)
                .await,
            Err(P2pDatasetError::SizeMismatch)
        ));
        assert!(fs::metadata(&size_destination).await.is_err());
        assert_no_partial_files(&temp).await;

        server.await.unwrap();
        robot.shutdown().await.unwrap();
        compute.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn interrupted_transfer_reconnects_from_zero_and_completes_atomically() {
        let domain_id = Uuid::new_v4();
        let bytes = zip_like_bytes(2 * TRANSFER_BUFFER_BYTES + 33);
        let sha256 = hex::encode(Sha256::digest(&bytes));

        let robot = listening_node();
        let robot_address = listen_address(&robot).await;
        let robot_credentials = P2pCredentialStore::new(robot.clone());
        install_current_token(
            &robot_credentials,
            &robot,
            PeerRole::Robot,
            domain_id,
            unix_time(),
        )
        .await;
        let protocol = ApplicationProtocol::new(DATASET_PROTOCOL).unwrap();
        let mut incoming = robot
            .accept(
                protocol,
                SessionRequirements::new(domain_id.to_string(), PeerRole::Compute).unwrap(),
            )
            .unwrap();

        let compute = unbound_node();
        let compute_credentials = P2pCredentialStore::new(compute.clone());
        install_current_token(
            &compute_credentials,
            &compute,
            PeerRole::Compute,
            domain_id,
            unix_time(),
        )
        .await;
        let compute_adapter = P2pDatasetAdapter::new(compute.clone(), compute_credentials, vec![]);
        let reference = P2pDatasetReference {
            schema: P2P_DATASET_SCHEMA.into(),
            dataset_id: "retry-dataset".into(),
            domain_id,
            name: "retry.zip".into(),
            peer_id: robot.peer_id().to_string(),
            multiaddrs: vec![robot_address.to_string()],
            size_bytes: bytes.len() as u64,
            sha256: sha256.clone(),
            available_until: Utc::now() + chrono::Duration::minutes(10),
        };
        let server_bytes = bytes.clone();
        let server = tokio::spawn(async move {
            for attempt in 0..FETCH_ATTEMPTS {
                let mut stream = incoming.accept().await.unwrap().unwrap();
                let request: DatasetRequest = read_json_frame(&mut stream, MAX_REQUEST_BYTES)
                    .await
                    .unwrap();
                assert_eq!(request.version, DATASET_REQUEST_VERSION);
                assert_eq!(request.dataset_id, "retry-dataset");
                write_json_frame(
                    &mut stream,
                    &DatasetResponseHeader {
                        dataset_id: request.dataset_id,
                        size_bytes: server_bytes.len() as u64,
                        sha256: sha256.clone(),
                    },
                    MAX_RESPONSE_HEADER_BYTES,
                )
                .await
                .unwrap();
                let body = if attempt == 0 {
                    &server_bytes[..server_bytes.len() / 2]
                } else {
                    server_bytes.as_slice()
                };
                stream.write_all(body).await.unwrap();
                stream.close().await.unwrap();
            }
        });

        let temp = TempDir::new().unwrap();
        let destination = temp.path().join("retry.zip");
        tokio::time::timeout(
            Duration::from_secs(10),
            compute_adapter.fetch_dataset(&reference, &destination),
        )
        .await
        .expect("retry transfer timed out")
        .unwrap();
        server.await.unwrap();
        assert_eq!(fs::read(&destination).await.unwrap(), bytes);
        assert_no_partial_files(&temp).await;

        robot.shutdown().await.unwrap();
        compute.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn missing_and_expired_compute_credentials_fail_before_dial() {
        let domain_id = Uuid::new_v4();
        let compute = unbound_node();
        let credentials = P2pCredentialStore::new(compute.clone());
        let adapter = P2pDatasetAdapter::new(compute.clone(), credentials.clone(), vec![]);
        let reference = unreachable_reference(domain_id);
        let temp = TempDir::new().unwrap();

        assert!(matches!(
            adapter
                .fetch_dataset(&reference, &temp.path().join("missing.zip"))
                .await,
            Err(P2pDatasetError::Credential(DdsP2pError::MissingCredential))
        ));

        let issued_at = unix_time() - P2P_TOKEN_TTL.as_secs() + 2;
        install_current_token(
            &credentials,
            &compute,
            PeerRole::Compute,
            domain_id,
            issued_at,
        )
        .await;
        tokio::time::sleep(Duration::from_secs(3)).await;
        assert!(matches!(
            adapter
                .fetch_dataset(&reference, &temp.path().join("expired.zip"))
                .await,
            Err(P2pDatasetError::Credential(DdsP2pError::ExpiredCredential))
        ));

        compute.shutdown().await.unwrap();
    }

    async fn relay_serving_adapter(domain_id: Uuid) -> (Node, P2pDatasetAdapter, P2pDatasetServer) {
        let node = unbound_node();
        let credentials = P2pCredentialStore::new(node.clone());
        install_current_token(&credentials, &node, PeerRole::Robot, domain_id, unix_time()).await;
        let adapter = P2pDatasetAdapter::new_with_route_policy(
            node.clone(),
            credentials,
            Vec::new(),
            DatasetRoutePolicy::RelayRequired,
        )
        .unwrap();
        let shutdown = CancellationToken::new();
        let server = adapter.start_serving(domain_id, &shutdown).await.unwrap();
        (node, adapter, server)
    }

    fn synthetic_fence(index: u8) -> RelayRouteFence {
        RelayRouteFence {
            slot_id: Uuid::from_u128(u128::from(index) + 1),
            assignment_id: Uuid::from_u128(u128::from(index) + 101),
            reservation_epoch: Uuid::from_u128(u128::from(index) + 201),
            local_generation: u64::from(index) + 1,
        }
    }

    fn synthetic_route(adapter: &P2pDatasetAdapter, index: u8) -> Multiaddr {
        adapter
            .inner
            .state
            .lock()
            .relay_routes
            .get(&synthetic_fence(index).slot_id)
            .unwrap()
            .route
            .clone()
    }

    fn install_synthetic_relay(
        adapter: &P2pDatasetAdapter,
        index: u8,
        duration: Duration,
        data_bytes_per_direction: u64,
    ) -> RelayRouteFence {
        let fence = synthetic_fence(index);
        let relay_peer_id = Identity::generate().peer_id();
        let route = format!(
            "/dns4/relay-{index}.testnet.aukiverse.com/tcp/{}/p2p/{relay_peer_id}/p2p-circuit/p2p/{}",
            41000_u16 + u16::from(index),
            adapter.inner.node.peer_id()
        )
        .parse::<Multiaddr>()
        .unwrap();
        let (_, endpoint) =
            validate_circuit_route(&route, relay_peer_id, adapter.inner.node.peer_id()).unwrap();
        let now = Utc::now();
        let status = {
            let mut state = adapter.inner.state.lock();
            expire_relay_authorizations(&mut state, now).unwrap();
            assert!(state.relay_routes.len() < MAX_CIRCUIT_ROUTES);
            assert!(!state.relay_routes.contains_key(&fence.slot_id));
            assert!(!state.relay_routes.values().any(|entry| {
                entry.relay_peer_id == relay_peer_id || entry.endpoint == endpoint
            }));
            assert!(
                state.direct_routes.len() + publishable_relay_count(&state, now)
                    < MAX_PUBLISHED_ROUTES
            );
            bump_route_revision(&mut state).unwrap();
            state.relay_routes.insert(
                fence.slot_id,
                StoredRelayRoute {
                    fence,
                    reservation: StoredReservationHandle::Synthetic,
                    relay_peer_id,
                    endpoint,
                    route,
                    limits: ExpectedRelayLimits::new(duration, data_bytes_per_direction).unwrap(),
                    authorized_until: now + chrono::Duration::hours(1),
                    publishable: true,
                },
            );
            serving_status(&state, now)
        };
        adapter.inner.status.send_replace(status);
        fence
    }

    fn tombstone_synthetic_relay(adapter: &P2pDatasetAdapter, fence: RelayRouteFence) {
        let now = Utc::now();
        let status = {
            let mut state = adapter.inner.state.lock();
            let existing = state.relay_routes.get(&fence.slot_id).unwrap();
            assert_eq!(existing.fence, fence);
            if existing.publishable {
                bump_route_revision(&mut state).unwrap();
            }
            state.relay_routes.remove(&fence.slot_id).unwrap();
            serving_status(&state, now)
        };
        adapter.inner.status.send_replace(status);
    }

    fn listening_node() -> Node {
        Node::start(
            Identity::generate(),
            verifier(),
            ["/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap()],
        )
        .unwrap()
    }

    fn unbound_node() -> Node {
        Node::start(
            Identity::generate(),
            verifier(),
            std::iter::empty::<Multiaddr>(),
        )
        .unwrap()
    }

    async fn listen_address(node: &Node) -> Multiaddr {
        tokio::time::timeout(Duration::from_secs(5), node.first_listen_address())
            .await
            .expect("listener did not start")
            .unwrap()
    }

    async fn install_current_token(
        credentials: &P2pCredentialStore,
        node: &Node,
        role: PeerRole,
        domain_id: Uuid,
        issued_at: u64,
    ) {
        let expires_at_unix = issued_at + P2P_TOKEN_TTL.as_secs();
        let claims = P2PAccessClaims {
            token_type: P2P_TOKEN_TYPE.into(),
            iss: P2P_TOKEN_ISSUER.into(),
            aud: vec![P2P_TOKEN_AUDIENCE.into()],
            sub: Uuid::new_v4().to_string(),
            peer_type: role,
            peer_id: node.peer_id().to_string(),
            domain_ids: vec![domain_id.to_string()],
            scopes: vec![P2P_TOKEN_SCOPE.into()],
            iat: issued_at,
            exp: expires_at_unix,
        };
        let token = encode(
            &Header::new(Algorithm::ES256),
            &claims,
            &EncodingKey::from_ec_pem(TEST_DDS_PRIVATE_KEY).unwrap(),
        )
        .unwrap();
        credentials
            .install(
                token,
                DateTime::from_timestamp(expires_at_unix as i64, 0).unwrap(),
            )
            .await
            .unwrap();
    }

    fn verifier() -> DdsTokenVerifier {
        DdsTokenVerifier::from_es256_pem(TEST_DDS_PUBLIC_KEY).unwrap()
    }

    fn registration(dataset_id: &str, domain_id: Uuid, path: PathBuf) -> P2pDatasetRegistration {
        P2pDatasetRegistration {
            dataset_id: dataset_id.into(),
            domain_id,
            name: format!("{dataset_id}.zip"),
            path,
            available_until: Utc::now() + chrono::Duration::minutes(10),
        }
    }

    fn unreachable_reference(domain_id: Uuid) -> P2pDatasetReference {
        P2pDatasetReference {
            schema: P2P_DATASET_SCHEMA.into(),
            dataset_id: "unreachable".into(),
            domain_id,
            name: "unreachable.zip".into(),
            peer_id: Identity::generate().peer_id().to_string(),
            multiaddrs: vec!["/ip4/127.0.0.1/tcp/9".into()],
            size_bytes: 1,
            sha256: "00".repeat(32),
            available_until: Utc::now() + chrono::Duration::minutes(10),
        }
    }

    fn reference_with_routes(peer_id: PeerId, multiaddrs: Vec<String>) -> P2pDatasetReference {
        P2pDatasetReference {
            schema: P2P_DATASET_SCHEMA.into(),
            dataset_id: "route-validation".into(),
            domain_id: Uuid::new_v4(),
            name: "route-validation.zip".into(),
            peer_id: peer_id.to_string(),
            multiaddrs,
            size_bytes: 1,
            sha256: "00".repeat(32),
            available_until: Utc::now() + chrono::Duration::minutes(10),
        }
    }

    fn zip_like_bytes(length: usize) -> Vec<u8> {
        let mut bytes = vec![0_u8; length.max(4)];
        bytes[..4].copy_from_slice(b"PK\x03\x04");
        for (index, byte) in bytes[4..].iter_mut().enumerate() {
            *byte = (index % 251) as u8;
        }
        bytes
    }

    async fn assert_no_partial_files(temp: &TempDir) {
        let mut entries = fs::read_dir(temp.path()).await.unwrap();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            assert!(
                !entry.file_name().to_string_lossy().ends_with(".part"),
                "partial transfer file was not cleaned up"
            );
        }
    }

    fn unix_time() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}
