//! Immutable, mutually authenticated dataset publication and transfer over an
//! [`auki_sdk::AukiPeerProtocolContext`].
//!
//! This crate owns the dataset wire contract, integrity metadata, local file
//! registry, route eligibility, retry policy, and atomic destination writes.
//! It consumes the facade's identity, authentication, exact-route transport,
//! and read-only routes; it does not acquire DDS credentials, create DMS relay
//! bookings, mutate routes, or know about compute tasks.

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
    canonicalize_circuit_route, validate_direct_route, AuthenticatedRouteStream, Multiaddr, PeerId,
    PeerRole, Protocol, RouteSnapshot,
};
use auki_sdk::{
    AukiPeerAuthorizationError, AukiPeerProtocolContext, AukiPeerProtocolsError,
    AukiPeerRoutesError, DomainProtocolError, DomainProtocolRegistration, DomainProtocolSpec,
    DomainProtocolStream,
};
use chrono::{DateTime, Utc};
use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt as TokioAsyncReadExt, AsyncWriteExt as TokioAsyncWriteExt},
    sync::watch,
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use uuid::Uuid;

pub const DATASET_PROTOCOL: &str = "/auki-p2p/dataset/0";
pub const P2P_DATASET_SCHEMA: &str = "auki-p2p-dataset/v0";
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
/// Maximum routes carried by one immutable dataset reference.
pub const MAX_DATASET_ROUTES: usize = 16;
/// Maximum relay circuits carried by one immutable dataset reference.
pub const MAX_DATASET_RELAY_ROUTES: usize = 3;
const MAX_PUBLISHED_ROUTES: usize = MAX_DATASET_ROUTES;
const MAX_CIRCUIT_ROUTES: usize = MAX_DATASET_RELAY_ROUTES;
const MIN_CIRCUIT_DURATION: Duration = Duration::from_secs(15 * 60);
const RELAY_DATA_OVERHEAD_BYTES: u64 = 1_048_576;
const MAX_CONCURRENT_TRANSFERS: usize = 64;

/// Local file registration accepted by the authenticated dataset protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct P2pDatasetRegistration {
    pub dataset_id: String,
    pub name: String,
    pub path: PathBuf,
    pub available_until: DateTime<Utc>,
}

/// Non-secret routing and integrity metadata persisted by the control plane.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct P2pDatasetReference {
    pub schema: String,
    pub dataset_id: String,
    pub domain_id: Uuid,
    pub name: String,
    pub peer_id: String,
    pub multiaddrs: Vec<String>,
    pub size_bytes: u64,
    pub sha256: String,
    pub available_until: DateTime<Utc>,
}

/// Narrow protocol facade exposed to task runners.
///
/// It deliberately exposes neither the underlying peer runtime nor P2P tokens.
#[async_trait]
pub trait P2pDataset: Send + Sync {
    async fn register(
        &self,
        registration: P2pDatasetRegistration,
    ) -> anyhow::Result<P2pDatasetReference>;

    async fn fetch(
        &self,
        reference: &P2pDatasetReference,
        destination: &Path,
    ) -> anyhow::Result<()>;
}

/// Cloneable, protocol-specific capability installed into a runner's task
/// services. The wrapper keeps the implementation opaque and does not expose
/// the authenticated transport or its credentials.
#[derive(Clone)]
pub struct DatasetService {
    inner: Arc<dyn P2pDataset>,
}

impl DatasetService {
    pub fn new(inner: Arc<dyn P2pDataset>) -> Self {
        Self { inner }
    }

    pub async fn register(
        &self,
        registration: P2pDatasetRegistration,
    ) -> anyhow::Result<P2pDatasetReference> {
        self.inner.register(registration).await
    }

    pub async fn fetch(
        &self,
        reference: &P2pDatasetReference,
        destination: &Path,
    ) -> anyhow::Result<()> {
        self.inner.fetch(reference, destination).await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum P2pDatasetError {
    #[error("P2P dataset local authorization is unavailable")]
    Authorization(#[source] AukiPeerAuthorizationError),
    #[error("P2P dataset protocol operation failed")]
    Protocol(#[source] AukiPeerProtocolsError),
    #[error("P2P dataset protocol specification is invalid")]
    ProtocolSpec(#[source] DomainProtocolError),
    #[error("P2P dataset route view failed")]
    Routes(#[source] AukiPeerRoutesError),
    #[error("P2P dataset local peer type must be {expected}; got {actual}")]
    LocalPeerTypeMismatch {
        expected: &'static str,
        actual: String,
    },
    #[error("P2P dataset remote peer type must be {expected}; got {actual}")]
    RemotePeerTypeMismatch {
        expected: &'static str,
        actual: String,
    },
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
    #[error("P2P dataset reference belongs to another Domain")]
    DomainMismatch,
    #[error("P2P dataset service is already serving")]
    AlreadyServing,
    #[error("P2P dataset service is not serving")]
    NotServing,
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
}

pub type Result<T> = std::result::Result<T, P2pDatasetError>;

fn require_local_dataset_role(context: &AukiPeerProtocolContext, expected: PeerRole) -> Result<()> {
    let authorization = context
        .authorization()
        .current()
        .map_err(P2pDatasetError::Authorization)?;
    if authorization.peer_type() != Some(expected.as_str()) {
        return Err(P2pDatasetError::LocalPeerTypeMismatch {
            expected: expected.as_str(),
            actual: authorization.peer_type().unwrap_or("<missing>").to_owned(),
        });
    }
    Ok(())
}

fn require_remote_dataset_role(
    peer: &auki_p2p::AuthenticatedPeer,
    expected: PeerRole,
) -> Result<()> {
    if peer.peer_type.as_deref() != Some(expected.as_str()) {
        return Err(P2pDatasetError::RemotePeerTypeMismatch {
            expected: expected.as_str(),
            actual: peer.peer_type.clone().unwrap_or_else(|| "<missing>".into()),
        });
    }
    Ok(())
}

/// Publication rule selected by the Robot host.
///
/// `RelayRequired` covers both accepted relay-mode spellings. Immutable
/// dataset publication requires a locally confirmed relay route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatasetRoutePolicy {
    DirectOnly,
    RelayRequired,
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
    context: AukiPeerProtocolContext,
    route_policy: DatasetRoutePolicy,
    state: parking_lot::Mutex<ServingState>,
    status: watch::Sender<DatasetServingStatus>,
}

struct ServingState {
    registrations_open: bool,
    registry: HashMap<String, RegisteredDataset>,
    serving: bool,
    active_transfers: BTreeMap<u64, ActiveTransfer>,
    next_transfer_id: u64,
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
struct RegisteredDataset {
    dataset_id: String,
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

impl P2pDatasetAdapter {
    /// Attach the Posemesh dataset protocol to one fixed-Domain Auki peer.
    pub fn new(context: AukiPeerProtocolContext, route_policy: DatasetRoutePolicy) -> Result<Self> {
        let state = ServingState {
            registrations_open: true,
            registry: HashMap::new(),
            serving: false,
            active_transfers: BTreeMap::new(),
            next_transfer_id: 1,
        };
        let route_snapshot = context
            .routes()
            .snapshot()
            .map_err(P2pDatasetError::Routes)?;
        let (status, _) = watch::channel(serving_status(&state, &route_snapshot, Utc::now()));
        Ok(Self {
            inner: Arc::new(AdapterInner {
                context,
                route_policy,
                state: parking_lot::Mutex::new(state),
                status,
            }),
        })
    }

    /// Stop accepting immutable dataset registrations and return the current
    /// deadlines the host must drain.
    pub fn stop_registrations(&self) -> Result<DatasetServingStatus> {
        self.set_registrations_open(false)
    }

    /// Re-open registrations after startup or a completed configuration drain.
    pub fn start_registrations(&self) -> Result<DatasetServingStatus> {
        self.set_registrations_open(true)
    }

    fn set_registrations_open(&self, open: bool) -> Result<DatasetServingStatus> {
        let now = Utc::now();
        let routes = self
            .inner
            .context
            .routes()
            .snapshot()
            .map_err(P2pDatasetError::Routes)?;
        let status = {
            let mut state = self.inner.state.lock();
            prune_expired(&mut state.registry, now);
            state.registrations_open = open;
            serving_status(&state, &routes, now)
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
        let routes = self
            .inner
            .context
            .routes()
            .snapshot()
            .map_err(P2pDatasetError::Routes)?;
        let status = {
            let mut state = self.inner.state.lock();
            prune_expired(&mut state.registry, now);
            serving_status(&state, &routes, now)
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

    fn publish_status(&self) -> Result<DatasetServingStatus> {
        self.serving_status()
    }

    fn clear_serving(&self) {
        let routes = self.inner.context.routes().snapshot().ok();
        let status = {
            let mut state = self.inner.state.lock();
            state.serving = false;
            routes
                .as_ref()
                .map(|routes| serving_status(&state, routes, Utc::now()))
        };
        if let Some(status) = status {
            self.inner.status.send_replace(status);
        }
    }

    fn close_registrations(&self) {
        let now = Utc::now();
        let routes = self.inner.context.routes().snapshot().ok();
        let status = {
            let mut state = self.inner.state.lock();
            prune_expired(&mut state.registry, now);
            state.registrations_open = false;
            routes
                .as_ref()
                .map(|routes| serving_status(&state, routes, now))
        };
        if let Some(status) = status {
            self.inner.status.send_replace(status);
        }
    }

    pub async fn start_serving(&self) -> Result<P2pDatasetServer> {
        let routes = self
            .inner
            .context
            .routes()
            .snapshot()
            .map_err(P2pDatasetError::Routes)?;
        {
            let state = self.inner.state.lock();
            if self.inner.route_policy == DatasetRoutePolicy::DirectOnly
                && routes.direct_routes.is_empty()
            {
                return Err(P2pDatasetError::MissingAdvertisedAddress);
            }
            if state.serving {
                return Err(P2pDatasetError::AlreadyServing);
            }
        }
        require_local_dataset_role(&self.inner.context, PeerRole::Robot)?;
        let spec = DomainProtocolSpec::new(
            DATASET_PROTOCOL,
            MAX_CONCURRENT_TRANSFERS,
            MAX_REQUEST_BYTES as u32,
        )
        .map_err(P2pDatasetError::ProtocolSpec)?;

        {
            let mut state = self.inner.state.lock();
            if state.serving {
                return Err(P2pDatasetError::AlreadyServing);
            }
            state.serving = true;
        }
        let adapter = self.clone();
        let server = self
            .inner
            .context
            .protocols()
            .register(spec, move |stream| {
                let adapter = adapter.clone();
                async move {
                    if let Err(error) = adapter.serve_stream(stream).await {
                        warn!(error = %error, "P2P dataset stream failed");
                    }
                }
            });
        match server {
            Ok(server) => {
                let route_watch_stop = CancellationToken::new();
                let task_stop = route_watch_stop.clone();
                let mut route_updates = self.inner.context.routes().subscribe();
                let status_adapter = self.clone();
                let route_watch_task = tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            _ = task_stop.cancelled() => break,
                            changed = route_updates.changed() => {
                                if changed.is_err() {
                                    break;
                                }
                                let _ = status_adapter.publish_status();
                            }
                        }
                    }
                });
                Ok(P2pDatasetServer {
                    adapter: self.clone(),
                    server: Some(server),
                    route_watch_stop,
                    route_watch_task: Some(route_watch_task),
                })
            }
            Err(error) => {
                self.clear_serving();
                Err(P2pDatasetError::Protocol(error))
            }
        }
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
        require_local_dataset_role(&self.inner.context, PeerRole::Robot)?;

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
        let mut expected_routes = self.capture_registration_routes()?;
        let (size_bytes, sha256) = hasher(path.clone()).await?;
        if size_bytes == 0 {
            return Err(P2pDatasetError::EmptyDataset);
        }
        let registered = RegisteredDataset {
            dataset_id: registration.dataset_id.clone(),
            path,
            size_bytes,
            sha256: sha256.clone(),
            available_until: registration.available_until,
        };

        // Endpoint authority is separate from relay authority. Revalidate it
        // immediately before the immutable reference commit.
        require_local_dataset_role(&self.inner.context, PeerRole::Robot)?;

        let mut registered = Some(registered);
        let mut dataset_id = Some(registration.dataset_id);
        let mut name = Some(registration.name);
        let mut sha256 = Some(sha256);
        let (reference, status) = loop {
            let fences = expected_routes.fences().collect::<Vec<_>>();
            let committed = self
                .inner
                .context
                .routes()
                .commit_if_current(expected_routes.revision, &fences, |routes| {
                    let now = Utc::now();
                    if registration.available_until <= now {
                        return Err(P2pDatasetError::ExpiredDataset);
                    }
                    let mut state = self.inner.state.lock();
                    prune_expired(&mut state.registry, now);
                    validate_registration_state(&state, routes, self.inner.route_policy)?;
                    let multiaddrs = select_registration_routes(
                        routes,
                        self.inner.route_policy,
                        size_bytes,
                        now,
                    )?;
                    let dataset_id = dataset_id
                        .take()
                        .expect("route commit closure runs at most once");
                    state.registry.insert(
                        dataset_id.clone(),
                        registered
                            .take()
                            .expect("route commit closure runs at most once"),
                    );
                    let reference = P2pDatasetReference {
                        schema: P2P_DATASET_SCHEMA.to_string(),
                        dataset_id,
                        domain_id: self.inner.context.domain_id(),
                        name: name.take().expect("route commit closure runs at most once"),
                        peer_id: self.inner.context.peer_id().to_string(),
                        multiaddrs,
                        size_bytes,
                        sha256: sha256
                            .take()
                            .expect("route commit closure runs at most once"),
                        available_until: registration.available_until,
                    };
                    Ok((reference, serving_status(&state, routes, now)))
                })
                .map_err(P2pDatasetError::Routes)?;
            if let Some(result) = committed {
                break result?;
            }
            expected_routes = self
                .inner
                .context
                .routes()
                .snapshot()
                .map_err(P2pDatasetError::Routes)?;
        };
        self.inner.status.send_replace(status);
        Ok(reference)
    }

    fn capture_registration_routes(&self) -> Result<RouteSnapshot> {
        let now = Utc::now();
        let routes = self
            .inner
            .context
            .routes()
            .snapshot()
            .map_err(P2pDatasetError::Routes)?;
        let status = {
            let mut state = self.inner.state.lock();
            prune_expired(&mut state.registry, now);
            validate_registration_state(&state, &routes, self.inner.route_policy)?;
            serving_status(&state, &routes, now)
        };
        self.inner.status.send_replace(status);
        Ok(routes)
    }

    pub async fn fetch_dataset(
        &self,
        reference: &P2pDatasetReference,
        destination: &Path,
    ) -> Result<()> {
        if reference.domain_id != self.inner.context.domain_id() {
            return Err(P2pDatasetError::DomainMismatch);
        }
        let (peer_id, candidates) = validate_reference(reference)?;
        require_local_dataset_role(&self.inner.context, PeerRole::Compute)?;

        let parent = destination.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)
            .await
            .map_err(P2pDatasetError::Io)?;

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
                    Err(error @ P2pDatasetError::RemotePeerTypeMismatch { .. }) => {
                        return Err(error);
                    }
                    Err(error @ P2pDatasetError::LocalPeerTypeMismatch { .. })
                    | Err(error @ P2pDatasetError::Authorization(_)) => {
                        return Err(error);
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
            let transfer =
                receive_dataset_stream(&mut opened, &self.inner.context, reference, partial.path());
            let result = match tokio::time::timeout_at(round_deadline, transfer).await {
                Ok(Ok(())) if tokio::time::Instant::now() >= round_deadline => {
                    Err(P2pDatasetError::TransferTimeout)
                }
                Ok(result) => result,
                Err(_) => Err(P2pDatasetError::TransferTimeout),
            };
            let route_cleanup = opened
                .close()
                .await
                .map_err(P2pDatasetError::RelayRouteCleanup);

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
        round_deadline: tokio::time::Instant,
        available_until: DateTime<Utc>,
    ) -> Result<AuthenticatedRouteStream> {
        if available_until <= Utc::now() {
            return Err(P2pDatasetError::ExpiredDataset);
        }
        let availability_deadline = utc_deadline_as_instant(available_until);
        let bounded_candidate_deadline = std::cmp::min(
            round_deadline,
            tokio::time::Instant::now() + FETCH_CANDIDATE_TIMEOUT,
        );
        let candidate_deadline = std::cmp::min(bounded_candidate_deadline, availability_deadline);
        let protocols = self.inner.context.protocols();
        let open = protocols.open_exact(peer_id, candidate.route().clone(), DATASET_PROTOCOL);
        let stream = tokio::time::timeout_at(candidate_deadline, open)
            .await
            .map_err(|_| {
                candidate_timeout_error(availability_deadline, bounded_candidate_deadline)
            })?
            .map_err(P2pDatasetError::Protocol)?;
        if let Err(error) = require_remote_dataset_role(stream.remote_peer(), PeerRole::Robot) {
            stream
                .close()
                .await
                .map_err(P2pDatasetError::RelayRouteCleanup)?;
            return Err(error);
        }
        if let Err(error) = require_local_dataset_role(&self.inner.context, PeerRole::Compute) {
            stream
                .close()
                .await
                .map_err(P2pDatasetError::RelayRouteCleanup)?;
            return Err(error);
        }
        if tokio::time::Instant::now() >= candidate_deadline {
            let timeout_error =
                candidate_timeout_error(availability_deadline, bounded_candidate_deadline);
            stream
                .close()
                .await
                .map_err(P2pDatasetError::RelayRouteCleanup)?;
            return Err(timeout_error);
        }
        Ok(stream)
    }

    async fn serve_stream(&self, mut stream: DomainProtocolStream) -> Result<()> {
        require_remote_dataset_role(stream.remote_peer(), PeerRole::Compute)?;
        require_local_dataset_role(&self.inner.context, PeerRole::Robot)?;
        let request: DatasetRequest = read_json_frame(&mut stream, MAX_REQUEST_BYTES).await?;
        if request.version != DATASET_REQUEST_VERSION {
            return Err(P2pDatasetError::UnsupportedRequestVersion);
        }
        validate_dataset_id(&request.dataset_id)?;
        let (entry, _transfer_guard) = self.begin_transfer(&request.dataset_id)?;

        let transfer = async {
            require_local_dataset_role(&self.inner.context, PeerRole::Robot)?;
            self.serve_registered_stream(stream, entry).await
        };
        match tokio::time::timeout(FETCH_ATTEMPT_TIMEOUT, transfer).await {
            Ok(result) => result,
            Err(_) => Err(P2pDatasetError::TransferTimeout),
        }
    }

    fn begin_transfer(&self, dataset_id: &str) -> Result<(RegisteredDataset, ActiveTransferGuard)> {
        let now = Utc::now();
        let routes = self
            .inner
            .context
            .routes()
            .snapshot()
            .map_err(P2pDatasetError::Routes)?;
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
            (entry, transfer_id, serving_status(&state, &routes, now))
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

    async fn serve_registered_stream<S>(
        &self,
        mut stream: S,
        entry: RegisteredDataset,
    ) -> Result<()>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
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

async fn receive_dataset_stream<S>(
    stream: &mut S,
    context: &AukiPeerProtocolContext,
    reference: &P2pDatasetReference,
    temp_path: &Path,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    write_dataset_request(stream, context, &reference.dataset_id).await?;
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

async fn write_dataset_request<S>(
    stream: &mut S,
    context: &AukiPeerProtocolContext,
    dataset_id: &str,
) -> Result<()>
where
    S: AsyncWrite + Unpin,
{
    // Mutual authentication is intentionally role-neutral. Revalidate the
    // current local Compute claim at the application-byte boundary so a
    // credential rotation during dialing/authentication cannot authorize a
    // dataset request under stale diagnostic metadata.
    require_local_dataset_role(context, PeerRole::Compute)?;
    write_json_frame(
        stream,
        &DatasetRequest {
            version: DATASET_REQUEST_VERSION,
            dataset_id: dataset_id.to_owned(),
        },
        MAX_REQUEST_BYTES,
    )
    .await
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
        let routes = self.inner.context.routes().snapshot().ok();
        let status = {
            let mut state = self.inner.state.lock();
            state.active_transfers.remove(&self.transfer_id);
            routes
                .as_ref()
                .map(|routes| serving_status(&state, routes, Utc::now()))
        };
        if let Some(status) = status {
            self.inner.status.send_replace(status);
        }
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
    adapter: P2pDatasetAdapter,
    server: Option<DomainProtocolRegistration>,
    route_watch_stop: CancellationToken,
    route_watch_task: Option<JoinHandle<()>>,
}

impl P2pDatasetServer {
    pub async fn shutdown(mut self) -> Result<()> {
        self.adapter.close_registrations();
        if let Some(server) = self.server.take() {
            server.close().await;
        }
        self.route_watch_stop.cancel();
        if let Some(task) = self.route_watch_task.take() {
            let _ = task.await;
        }
        self.adapter.clear_serving();
        Ok(())
    }
}

impl Drop for P2pDatasetServer {
    fn drop(&mut self) {
        self.adapter.close_registrations();
        self.server.take();
        self.route_watch_stop.cancel();
        if let Some(task) = self.route_watch_task.take() {
            task.abort();
        }
        self.adapter.clear_serving();
    }
}

impl P2pDatasetError {
    fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Protocol(_) | Self::Io(_) | Self::InterruptedTransfer | Self::TransferTimeout
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

fn validate_registration_state(
    state: &ServingState,
    routes: &RouteSnapshot,
    policy: DatasetRoutePolicy,
) -> Result<()> {
    if !state.registrations_open {
        return Err(P2pDatasetError::RegistrationsStopped);
    }
    if !state.serving {
        return Err(P2pDatasetError::NotServing);
    }
    match policy {
        DatasetRoutePolicy::DirectOnly if routes.direct_routes.is_empty() => {
            Err(P2pDatasetError::MissingAdvertisedAddress)
        }
        DatasetRoutePolicy::RelayRequired if routes.relay_routes.is_empty() => {
            Err(P2pDatasetError::ConfirmedRelayRequired)
        }
        DatasetRoutePolicy::DirectOnly | DatasetRoutePolicy::RelayRequired => Ok(()),
    }
}

fn select_registration_routes(
    routes: &RouteSnapshot,
    policy: DatasetRoutePolicy,
    size_bytes: u64,
    now: DateTime<Utc>,
) -> Result<Vec<String>> {
    let mut selected = routes
        .direct_routes
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if policy == DatasetRoutePolicy::RelayRequired {
        let required_data_bytes = required_relay_data_bytes(size_bytes)?;
        let eligible = routes
            .relay_routes
            .iter()
            .filter(|entry| {
                entry.authorized_until > now
                    && entry.limits.duration() >= MIN_CIRCUIT_DURATION
                    && entry.limits.data_bytes_per_direction() >= required_data_bytes
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
        selected.extend(eligible);
    }
    if selected.len() > MAX_PUBLISHED_ROUTES {
        return Err(P2pDatasetError::RouteLimitExceeded {
            maximum: MAX_PUBLISHED_ROUTES,
        });
    }
    Ok(selected)
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

fn serving_status(
    state: &ServingState,
    routes: &RouteSnapshot,
    now: DateTime<Utc>,
) -> DatasetServingStatus {
    DatasetServingStatus {
        route_set_revision: routes.revision,
        registrations_open: state.registrations_open,
        direct_route_count: routes.direct_routes.len(),
        confirmed_relay_count: routes.relay_routes.len(),
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
    use auki_p2p::Identity;
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn dataset_identifiers_and_names_are_bounded() {
        assert!(validate_dataset_id("dataset-1").is_ok());
        assert!(validate_dataset_name("mapping capture").is_ok());
        assert!(matches!(
            validate_dataset_id("   "),
            Err(P2pDatasetError::InvalidDatasetId)
        ));
        assert!(matches!(
            validate_dataset_name(&"x".repeat(MAX_DATASET_NAME_BYTES + 1)),
            Err(P2pDatasetError::InvalidDatasetName)
        ));
    }

    #[test]
    fn relay_budget_rounds_up_and_checks_overflow() {
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
    fn registration_state_requires_serving_and_the_selected_route_kind() {
        let mut state = ServingState {
            registrations_open: true,
            registry: HashMap::new(),
            serving: false,
            active_transfers: BTreeMap::new(),
            next_transfer_id: 1,
        };
        let mut routes = RouteSnapshot {
            revision: 0,
            direct_routes: Vec::new(),
            relay_routes: Vec::new(),
        };

        assert!(matches!(
            validate_registration_state(&state, &routes, DatasetRoutePolicy::DirectOnly),
            Err(P2pDatasetError::NotServing)
        ));

        state.serving = true;
        assert!(matches!(
            validate_registration_state(&state, &routes, DatasetRoutePolicy::DirectOnly),
            Err(P2pDatasetError::MissingAdvertisedAddress)
        ));
        assert!(matches!(
            validate_registration_state(&state, &routes, DatasetRoutePolicy::RelayRequired),
            Err(P2pDatasetError::ConfirmedRelayRequired)
        ));

        let peer_id = Identity::generate().peer_id();
        routes.direct_routes = vec![format!("/ip4/192.0.2.1/tcp/4001/p2p/{peer_id}")
            .parse()
            .unwrap()];
        assert!(
            validate_registration_state(&state, &routes, DatasetRoutePolicy::DirectOnly).is_ok()
        );

        state.registrations_open = false;
        assert!(matches!(
            validate_registration_state(&state, &routes, DatasetRoutePolicy::DirectOnly),
            Err(P2pDatasetError::RegistrationsStopped)
        ));
    }

    #[test]
    fn reference_validation_skips_bad_routes_and_deduplicates_valid_routes() {
        let peer_id = Identity::generate().peer_id();
        let route = format!("/ip4/192.0.2.1/tcp/4001/p2p/{peer_id}");
        let reference = P2pDatasetReference {
            schema: P2P_DATASET_SCHEMA.into(),
            dataset_id: "dataset-1".into(),
            domain_id: Uuid::new_v4(),
            name: "capture".into(),
            peer_id: peer_id.to_string(),
            multiaddrs: vec!["not-a-multiaddr".into(), route.clone(), route.clone()],
            size_bytes: 4,
            sha256: "00".repeat(32),
            available_until: Utc::now() + chrono::Duration::minutes(5),
        };

        let (validated_peer, candidates) = validate_reference(&reference).unwrap();
        assert_eq!(validated_peer, peer_id);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].route().to_string(), "/ip4/192.0.2.1/tcp/4001");
        assert_eq!(candidates[0].kind(), "direct");
    }

    #[test]
    fn expired_reference_is_terminal() {
        let reference = P2pDatasetReference {
            schema: P2P_DATASET_SCHEMA.into(),
            dataset_id: "dataset-1".into(),
            domain_id: Uuid::new_v4(),
            name: "capture".into(),
            peer_id: Identity::generate().peer_id().to_string(),
            multiaddrs: vec!["/ip4/192.0.2.1/tcp/4001".into()],
            size_bytes: 4,
            sha256: "00".repeat(32),
            available_until: Utc::now() - chrono::Duration::seconds(1),
        };

        let error = validate_reference(&reference).unwrap_err();
        assert!(matches!(error, P2pDatasetError::ExpiredDataset));
        assert!(!error.is_retryable());
    }

    #[tokio::test]
    async fn file_hash_is_deterministic_and_partial_path_is_unique() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("capture.bin");
        fs::write(&path, b"posemesh").await.unwrap();

        let (size, hash) = hash_file(&path).await.unwrap();
        assert_eq!(size, 8);
        assert_eq!(
            hash,
            "92e36810da937802ea32f1f7f2b5706d1895f04d6c02708704d7a5eac541ff7b"
        );
        let partial = partial_path(&path);
        assert_eq!(partial.parent(), Some(temp.path()));
        let partial_name = partial.file_name().unwrap().to_str().unwrap();
        assert!(partial_name.starts_with(".capture.bin."));
        assert!(partial_name.ends_with(".part"));
        assert_ne!(partial, partial_path(&path));
    }

    #[tokio::test]
    async fn json_frames_round_trip_and_enforce_limits() {
        let mut frame = futures::io::Cursor::new(Vec::new());
        let value = DatasetRequest {
            version: DATASET_REQUEST_VERSION,
            dataset_id: "dataset-1".into(),
        };
        write_json_frame(&mut frame, &value, MAX_REQUEST_BYTES)
            .await
            .unwrap();
        frame.set_position(0);
        let decoded: DatasetRequest = read_json_frame(&mut frame, MAX_REQUEST_BYTES)
            .await
            .unwrap();
        assert_eq!(decoded.version, DATASET_REQUEST_VERSION);
        assert_eq!(decoded.dataset_id, "dataset-1");

        let mut writer = futures::io::Cursor::new(Vec::new());
        assert!(matches!(
            write_json_frame(&mut writer, &"x".repeat(32), 4).await,
            Err(P2pDatasetError::FrameTooLarge)
        ));
    }

    #[tokio::test]
    async fn partial_file_cleanup_failure_keeps_guard_armed() {
        let temp = TempDir::new().unwrap();
        let directory = temp.path().join("not-a-file.part");
        fs::create_dir(&directory).await.unwrap();
        let mut guard = PartialFileGuard::new(directory);

        assert!(matches!(
            guard.cleanup().await,
            Err(P2pDatasetError::PartialFileCleanup(_))
        ));
        assert!(guard.armed);
        guard.disarm();
    }

    #[test]
    fn timeout_classification_preserves_retry_contract() {
        let now = tokio::time::Instant::now();
        assert!(matches!(
            candidate_timeout_error(now, now + Duration::from_secs(1)),
            P2pDatasetError::ExpiredDataset
        ));
        assert!(matches!(
            candidate_timeout_error(now + Duration::from_secs(1), now),
            P2pDatasetError::TransferTimeout
        ));
        assert!(P2pDatasetError::InterruptedTransfer.is_retryable());
        assert!(!P2pDatasetError::HashMismatch.is_retryable());
    }
}
