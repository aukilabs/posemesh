//! Generation-fenced Circuit Relay v2 reservation state.
//!
//! This module deliberately contains no control-plane client, retry, or timer
//! policy. It validates an immutable provider snapshot and correlates the
//! public libp2p listener, reservation, and connection evidence needed by the
//! transport integration.

use libp2p::{
    core::transport::ListenerId, multiaddr::Protocol, swarm::ConnectionId, Multiaddr, PeerId,
};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    num::{NonZeroU32, NonZeroU64},
    str::FromStr,
    time::Duration,
};
use uuid::Uuid;

/// The positive finite limits expected in every reservation acceptance.
///
/// Circuit Relay v2 represents duration as whole seconds in a `uint32` and
/// data per direction in a `uint64`. Provider configuration is additionally
/// constrained to the positive signed 64-bit range before it reaches this
/// crate, so this type enforces that common, lossless intersection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpectedRelayLimits {
    duration_seconds: NonZeroU32,
    data_bytes_per_direction: NonZeroU64,
}

impl ExpectedRelayLimits {
    /// Validates positive, whole-second, losslessly representable limits.
    pub fn new(duration: Duration, data_bytes_per_direction: u64) -> RelayReservationResult<Self> {
        if duration.is_zero() {
            return Err(RelayReservationError::InvalidLimits(
                "duration must be positive".to_string(),
            ));
        }
        if duration.subsec_nanos() != 0 {
            return Err(RelayReservationError::InvalidLimits(
                "duration must be a whole number of seconds".to_string(),
            ));
        }
        let duration_seconds = u32::try_from(duration.as_secs()).map_err(|_| {
            RelayReservationError::InvalidLimits(
                "duration exceeds the Circuit Relay v2 uint32 wire limit".to_string(),
            )
        })?;
        let duration_seconds = NonZeroU32::new(duration_seconds).ok_or_else(|| {
            RelayReservationError::InvalidLimits("duration must be positive".to_string())
        })?;

        if data_bytes_per_direction > i64::MAX as u64 {
            return Err(RelayReservationError::InvalidLimits(
                "data limit exceeds the provider signed 64-bit range".to_string(),
            ));
        }
        let data_bytes_per_direction =
            NonZeroU64::new(data_bytes_per_direction).ok_or_else(|| {
                RelayReservationError::InvalidLimits("data limit must be positive".to_string())
            })?;

        Ok(Self {
            duration_seconds,
            data_bytes_per_direction,
        })
    }

    pub fn duration(self) -> Duration {
        Duration::from_secs(u64::from(self.duration_seconds.get()))
    }

    pub fn duration_seconds(self) -> u32 {
        self.duration_seconds.get()
    }

    pub fn data_bytes_per_direction(self) -> u64 {
        self.data_bytes_per_direction.get()
    }

    fn validate_observed(
        self,
        observed: Option<ObservedRelayLimits>,
    ) -> Result<(), RelayConfirmationRejection> {
        let Some(observed) = observed else {
            return Err(RelayConfirmationRejection::MissingLimits);
        };
        let (Some(duration), Some(data_bytes_per_direction)) =
            (observed.duration, observed.data_bytes_per_direction)
        else {
            return Err(RelayConfirmationRejection::IncompleteLimits { observed });
        };

        if duration != self.duration()
            || data_bytes_per_direction != self.data_bytes_per_direction()
        {
            return Err(RelayConfirmationRejection::LimitMismatch {
                expected: self,
                observed,
            });
        }
        Ok(())
    }
}

/// The optional fields exposed by a libp2p relay acceptance event.
///
/// Keeping the optionality here makes missing outer limits and partially
/// populated wire limits explicit testable failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservedRelayLimits {
    duration: Option<Duration>,
    data_bytes_per_direction: Option<u64>,
}

impl ObservedRelayLimits {
    pub const fn new(duration: Option<Duration>, data_bytes_per_direction: Option<u64>) -> Self {
        Self {
            duration,
            data_bytes_per_direction,
        }
    }

    pub fn duration(self) -> Option<Duration> {
        self.duration
    }

    pub fn data_bytes_per_direction(self) -> Option<u64> {
        self.data_bytes_per_direction
    }
}

/// A complete canonical provider snapshot for one relay Peer ID.
///
/// Bases are normalized, sorted, and kept private. Callers can inspect them as
/// an immutable slice but cannot mutate the set after validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayProvider {
    relay_peer_id: PeerId,
    bases: Box<[Multiaddr]>,
    expected_limits: ExpectedRelayLimits,
}

impl RelayProvider {
    /// Validates and canonicalizes the exact v1 base grammar:
    /// `/dns4/<lowercase-fqdn>/tcp/<1-65535>/p2p/<relay-peer-id>`.
    ///
    /// Host case, trailing root dots, and decimal port spelling are
    /// canonicalized. A duplicate created by that normalization is rejected.
    pub fn new<I, S>(
        relay_peer_id: PeerId,
        raw_bases: I,
        expected_limits: ExpectedRelayLimits,
    ) -> RelayReservationResult<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut canonical = Vec::new();
        let mut unique = HashSet::new();

        for raw in raw_bases {
            let address = canonicalize_provider_base(raw.as_ref(), relay_peer_id)?;
            if !unique.insert(address.to_string()) {
                return Err(RelayReservationError::DuplicateBase(address));
            }
            canonical.push(address);
        }
        if canonical.is_empty() {
            return Err(RelayReservationError::EmptyBases);
        }

        canonical.sort_unstable_by_key(ToString::to_string);
        Ok(Self {
            relay_peer_id,
            bases: canonical.into_boxed_slice(),
            expected_limits,
        })
    }

    pub fn relay_peer_id(&self) -> PeerId {
        self.relay_peer_id
    }

    pub fn bases(&self) -> &[Multiaddr] {
        &self.bases
    }

    pub fn expected_limits(&self) -> ExpectedRelayLimits {
        self.expected_limits
    }

    /// The lexicographically first canonical base is the deterministic v1
    /// selection. Several bases from this provider still represent one relay.
    pub fn selected_base(&self) -> &Multiaddr {
        &self.bases[0]
    }

    /// Address passed to `Swarm::listen_on` for a reservation.
    pub fn reservation_listen_address(&self) -> Multiaddr {
        self.selected_base().clone().with(Protocol::P2pCircuit)
    }

    fn publishable_route(&self, local_peer_id: PeerId) -> Multiaddr {
        self.reservation_listen_address()
            .with(Protocol::P2p(local_peer_id))
    }
}

/// A monotonically increasing local reservation generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ReservationGeneration(u64);

impl ReservationGeneration {
    pub fn get(self) -> u64 {
        self.0
    }
}

/// An immutable capability used to stamp evidence for one reservation.
///
/// The private node instance ID prevents a handle minted by a different
/// [`RelayReservationNode`] from mutating this node, even when both nodes use
/// the same local Peer ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RelayReservationHandle {
    node_instance_id: Uuid,
    relay_peer_id: PeerId,
    generation: ReservationGeneration,
    listener_id: ListenerId,
}

impl RelayReservationHandle {
    pub fn relay_peer_id(self) -> PeerId {
        self.relay_peer_id
    }

    pub fn generation(self) -> ReservationGeneration {
        self.generation
    }

    pub fn listener_id(self) -> ListenerId {
        self.listener_id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelayReservationState {
    /// Initial acceptance and listener evidence have not both arrived.
    AwaitingConfirmation,
    /// The selected trusted base may be published.
    Publishable,
    /// Publication is fenced while listener/connection teardown completes.
    Canceling,
}

/// An immutable point-in-time view of a reservation generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayReservationSnapshot {
    handle: RelayReservationHandle,
    state: RelayReservationState,
    selected_base: Multiaddr,
    expected_limits: ExpectedRelayLimits,
    direct_connection: Option<ConnectionId>,
    publishable_route: Option<Multiaddr>,
}

impl RelayReservationSnapshot {
    pub fn handle(&self) -> RelayReservationHandle {
        self.handle
    }

    pub fn state(&self) -> RelayReservationState {
        self.state
    }

    pub fn selected_base(&self) -> &Multiaddr {
        &self.selected_base
    }

    pub fn expected_limits(&self) -> ExpectedRelayLimits {
        self.expected_limits
    }

    pub fn direct_connection(&self) -> Option<ConnectionId> {
        self.direct_connection
    }

    /// Returns a route only while both confirmation signals remain live.
    pub fn publishable_route(&self) -> Option<&Multiaddr> {
        self.publishable_route.as_ref()
    }
}

/// The exact local resources that must be removed for a canceled generation.
///
/// Integrations remove the listener and close the selected direct connection.
/// The relay Peer ID is included so all other direct connections to that relay
/// can be enumerated and closed without affecting a different relay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelayCancellation {
    handle: RelayReservationHandle,
    relay_peer_id: PeerId,
    listener_id: ListenerId,
    direct_connection: Option<ConnectionId>,
}

impl RelayCancellation {
    pub fn handle(self) -> RelayReservationHandle {
        self.handle
    }

    pub fn relay_peer_id(self) -> PeerId {
        self.relay_peer_id
    }

    pub fn listener_id(self) -> ListenerId {
        self.listener_id
    }

    pub fn direct_connection(self) -> Option<ConnectionId> {
        self.direct_connection
    }
}

/// A typed reason why reservation acceptance was rejected and unpublished.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RelayConfirmationRejection {
    #[error("reservation acceptance omitted its finite limits")]
    MissingLimits,
    #[error("reservation acceptance returned incomplete finite limits: {observed:?}")]
    IncompleteLimits { observed: ObservedRelayLimits },
    #[error("reservation limits differ: expected {expected:?}, observed {observed:?}")]
    LimitMismatch {
        expected: ExpectedRelayLimits,
        observed: ObservedRelayLimits,
    },
    #[error("an initial acceptance was received after this generation was already accepted")]
    DuplicateInitialAcceptance,
    #[error("a renewal was received before this generation's initial acceptance")]
    RenewalBeforeInitialAcceptance,
}

/// State-machine output. No response-only address is ever carried in a
/// publishable event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelayReservationEvent {
    /// Valid evidence was recorded, but the route is not newly publishable.
    EvidenceRecorded { handle: RelayReservationHandle },
    /// Both initial acceptance and listener evidence exist with exact limits.
    Publishable {
        handle: RelayReservationHandle,
        route: Multiaddr,
        limits: ExpectedRelayLimits,
    },
    /// An accepted renewal matched the immutable expected limits exactly.
    Renewed {
        handle: RelayReservationHandle,
        limits: ExpectedRelayLimits,
    },
    /// Acceptance failed closed. The returned resources must be torn down.
    ConfirmationRejected {
        handle: RelayReservationHandle,
        reason: RelayConfirmationRejection,
        cancellation: RelayCancellation,
    },
    /// Publication was atomically fenced and teardown must begin.
    CancellationStarted { cancellation: RelayCancellation },
    /// A repeated cancellation returns the same bounded teardown target.
    CancellationPending { cancellation: RelayCancellation },
    /// A connection appeared after tombstoning and must be closed immediately.
    CloseLateConnection {
        handle: RelayReservationHandle,
        connection_id: ConnectionId,
    },
    /// A late event for a still-canceling generation was ignored.
    Fenced { handle: RelayReservationHandle },
    /// Listener closure and direct-connection terminal evidence have both
    /// arrived. A replacement generation may now start.
    Canceled { handle: RelayReservationHandle },
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RelayReservationError {
    #[error("at least one relay provider base is required")]
    EmptyBases,
    #[error("invalid relay provider base {address}: {reason}")]
    InvalidBase { address: String, reason: String },
    #[error("relay provider base has Peer ID {actual}, expected {expected}")]
    BasePeerMismatch { expected: PeerId, actual: String },
    #[error("duplicate relay provider base after normalization: {0}")]
    DuplicateBase(Multiaddr),
    #[error("invalid relay limits: {0}")]
    InvalidLimits(String),
    #[error("relay {0} already has an active or canceling generation")]
    ReservationAlreadyExists(PeerId),
    #[error("listener {0} is already assigned to another relay generation")]
    ListenerAlreadyAssigned(ListenerId),
    #[error("the reservation generation counter is exhausted")]
    GenerationExhausted,
    #[error("reservation handle belongs to a different relay node instance")]
    ForeignHandle,
    #[error("reservation handle is stale")]
    StaleHandle,
    #[error("reservation handle is not known by this relay node")]
    UnknownHandle,
    #[error("listener evidence used listener {actual}, expected {expected}")]
    ListenerMismatch {
        expected: ListenerId,
        actual: ListenerId,
    },
    #[error("connection evidence used connection {actual}, expected {expected}")]
    ConnectionMismatch {
        expected: ConnectionId,
        actual: ConnectionId,
    },
    #[error("this relay generation has no associated direct connection")]
    NoDirectConnection,
    #[error("no-connection evidence conflicts with open connection {0}")]
    ConnectionStillOpen(ConnectionId),
}

pub type RelayReservationResult<T> = Result<T, RelayReservationError>;

/// Pure per-node reservation correlation and generation fencing.
///
/// Only one entry may exist for a relay Peer ID, including while it is
/// canceling. Different relay Peer IDs progress independently.
#[derive(Debug)]
pub struct RelayReservationNode {
    node_instance_id: Uuid,
    local_peer_id: PeerId,
    next_generation: u64,
    latest_generation: HashMap<PeerId, ReservationGeneration>,
    reservations: HashMap<PeerId, ReservationEntry>,
}

impl RelayReservationNode {
    pub fn new(local_peer_id: PeerId) -> Self {
        Self {
            node_instance_id: Uuid::new_v4(),
            local_peer_id,
            next_generation: 1,
            latest_generation: HashMap::new(),
            reservations: HashMap::new(),
        }
    }

    /// Returns the sole current handle for a relay, including while canceling.
    /// This is the mapping used to stamp relay events that expose only a Peer
    /// ID before they enter the generation-fenced methods below.
    pub fn handle_for_relay(&self, relay_peer_id: PeerId) -> Option<RelayReservationHandle> {
        self.reservations
            .get(&relay_peer_id)
            .map(|entry| entry.handle)
    }

    /// Returns the current handle associated with a public listener event.
    pub fn handle_for_listener(&self, listener_id: ListenerId) -> Option<RelayReservationHandle> {
        self.reservations
            .values()
            .find(|entry| entry.handle.listener_id == listener_id)
            .map(|entry| entry.handle)
    }

    /// Starts one generation after the caller has allocated the listener ID.
    /// The selected reservation address is available from [`RelayProvider`]
    /// before this call and remains pinned in the resulting snapshot.
    pub fn begin(
        &mut self,
        provider: RelayProvider,
        listener_id: ListenerId,
    ) -> RelayReservationResult<RelayReservationHandle> {
        let relay_peer_id = provider.relay_peer_id();
        if self.reservations.contains_key(&relay_peer_id) {
            return Err(RelayReservationError::ReservationAlreadyExists(
                relay_peer_id,
            ));
        }
        if self
            .reservations
            .values()
            .any(|entry| entry.handle.listener_id == listener_id)
        {
            return Err(RelayReservationError::ListenerAlreadyAssigned(listener_id));
        }

        let generation = ReservationGeneration(self.next_generation);
        self.next_generation = self
            .next_generation
            .checked_add(1)
            .ok_or(RelayReservationError::GenerationExhausted)?;
        let handle = RelayReservationHandle {
            node_instance_id: self.node_instance_id,
            relay_peer_id,
            generation,
            listener_id,
        };
        self.latest_generation.insert(relay_peer_id, generation);
        self.reservations
            .insert(relay_peer_id, ReservationEntry::new(handle, provider));
        Ok(handle)
    }

    pub fn snapshot(
        &self,
        handle: RelayReservationHandle,
    ) -> RelayReservationResult<RelayReservationSnapshot> {
        let entry = self.entry(handle)?;
        Ok(entry.snapshot(self.local_peer_id))
    }

    /// Associates the exact direct relay connection selected by the transport.
    pub fn observe_direct_connection(
        &mut self,
        handle: RelayReservationHandle,
        connection_id: ConnectionId,
    ) -> RelayReservationResult<RelayReservationEvent> {
        let entry = self.entry_mut(handle)?;
        if entry.state == RelayReservationState::Canceling {
            if entry.connection_terminal || entry.direct_connection.is_none() {
                entry.direct_connection = Some(connection_id);
                entry.connection_terminal = false;
            }
            return Ok(RelayReservationEvent::CloseLateConnection {
                handle,
                connection_id,
            });
        }

        match entry.direct_connection {
            None => entry.direct_connection = Some(connection_id),
            Some(expected) if expected == connection_id => {}
            Some(expected) => {
                return Err(RelayReservationError::ConnectionMismatch {
                    expected,
                    actual: connection_id,
                });
            }
        }
        Ok(RelayReservationEvent::EvidenceRecorded { handle })
    }

    /// Records one `NewListenAddr` for the reservation listener.
    ///
    /// `response_address` is deliberately consumed only as evidence. The
    /// publishable event is always constructed from the selected provider base.
    pub fn observe_listener_address(
        &mut self,
        handle: RelayReservationHandle,
        listener_id: ListenerId,
        _response_address: &Multiaddr,
    ) -> RelayReservationResult<RelayReservationEvent> {
        let local_peer_id = self.local_peer_id;
        let entry = self.entry_mut(handle)?;
        entry.ensure_listener(listener_id)?;
        if entry.state == RelayReservationState::Canceling {
            return Ok(RelayReservationEvent::Fenced { handle });
        }
        entry.listener_observed = true;
        Ok(entry.maybe_publish(local_peer_id))
    }

    /// Records initial or renewal acceptance. Missing or unequal limits
    /// atomically fence publication and return the teardown target.
    pub fn observe_acceptance(
        &mut self,
        handle: RelayReservationHandle,
        renewal: bool,
        observed_limits: Option<ObservedRelayLimits>,
    ) -> RelayReservationResult<RelayReservationEvent> {
        let local_peer_id = self.local_peer_id;
        let entry = self.entry_mut(handle)?;
        if entry.state == RelayReservationState::Canceling {
            return Ok(RelayReservationEvent::Fenced { handle });
        }

        let validation = entry
            .provider
            .expected_limits()
            .validate_observed(observed_limits);
        let validation = validation.and({
            if renewal && !entry.initial_acceptance_observed {
                Err(RelayConfirmationRejection::RenewalBeforeInitialAcceptance)
            } else if !renewal && entry.initial_acceptance_observed {
                Err(RelayConfirmationRejection::DuplicateInitialAcceptance)
            } else {
                Ok(())
            }
        });
        if let Err(reason) = validation {
            entry.start_canceling();
            return Ok(RelayReservationEvent::ConfirmationRejected {
                handle,
                reason,
                cancellation: entry.cancellation(),
            });
        }

        if renewal {
            return Ok(RelayReservationEvent::Renewed {
                handle,
                limits: entry.provider.expected_limits(),
            });
        }

        entry.initial_acceptance_observed = true;
        Ok(entry.maybe_publish(local_peer_id))
    }

    /// Atomically tombstones publication and returns exact teardown resources.
    /// A repeat is idempotent and does not create another generation.
    pub fn cancel(
        &mut self,
        handle: RelayReservationHandle,
    ) -> RelayReservationResult<RelayReservationEvent> {
        let entry = self.entry_mut(handle)?;
        if entry.state == RelayReservationState::Canceling {
            return Ok(RelayReservationEvent::CancellationPending {
                cancellation: entry.cancellation(),
            });
        }
        entry.start_canceling();
        Ok(RelayReservationEvent::CancellationStarted {
            cancellation: entry.cancellation(),
        })
    }

    /// Records terminal listener closure. Address-expiry events are not
    /// equivalent and must not call this method.
    pub fn observe_listener_closed(
        &mut self,
        handle: RelayReservationHandle,
        listener_id: ListenerId,
    ) -> RelayReservationResult<RelayReservationEvent> {
        let (was_canceling, cancellation, complete) = {
            let entry = self.entry_mut(handle)?;
            entry.ensure_listener(listener_id)?;
            let was_canceling = entry.state == RelayReservationState::Canceling;
            entry.start_canceling();
            entry.listener_closed = true;
            (
                was_canceling,
                entry.cancellation(),
                entry.cancellation_complete(),
            )
        };
        if complete {
            self.finish_cancellation(handle);
            return Ok(RelayReservationEvent::Canceled { handle });
        }
        if was_canceling {
            Ok(RelayReservationEvent::Fenced { handle })
        } else {
            Ok(RelayReservationEvent::CancellationStarted { cancellation })
        }
    }

    /// Records closure of the selected direct relay connection.
    pub fn observe_direct_connection_closed(
        &mut self,
        handle: RelayReservationHandle,
        connection_id: ConnectionId,
    ) -> RelayReservationResult<RelayReservationEvent> {
        let (was_canceling, cancellation, complete) = {
            let entry = self.entry_mut(handle)?;
            let expected = entry
                .direct_connection
                .ok_or(RelayReservationError::NoDirectConnection)?;
            if expected != connection_id {
                return Err(RelayReservationError::ConnectionMismatch {
                    expected,
                    actual: connection_id,
                });
            }
            let was_canceling = entry.state == RelayReservationState::Canceling;
            entry.start_canceling();
            entry.connection_terminal = true;
            (
                was_canceling,
                entry.cancellation(),
                entry.cancellation_complete(),
            )
        };
        if complete {
            self.finish_cancellation(handle);
            return Ok(RelayReservationEvent::Canceled { handle });
        }
        if was_canceling {
            Ok(RelayReservationEvent::Fenced { handle })
        } else {
            Ok(RelayReservationEvent::CancellationStarted { cancellation })
        }
    }

    /// Records that the reservation dial has terminated without ever yielding
    /// a direct connection. This is the only safe no-connection completion
    /// evidence for a generation canceled while its dial was pending.
    pub fn observe_no_direct_connection(
        &mut self,
        handle: RelayReservationHandle,
    ) -> RelayReservationResult<RelayReservationEvent> {
        let (was_canceling, cancellation, complete) = {
            let entry = self.entry_mut(handle)?;
            if let Some(connection_id) = entry.direct_connection {
                if !entry.connection_terminal {
                    return Err(RelayReservationError::ConnectionStillOpen(connection_id));
                }
            }
            let was_canceling = entry.state == RelayReservationState::Canceling;
            entry.start_canceling();
            entry.connection_terminal = true;
            (
                was_canceling,
                entry.cancellation(),
                entry.cancellation_complete(),
            )
        };
        if complete {
            self.finish_cancellation(handle);
            return Ok(RelayReservationEvent::Canceled { handle });
        }
        if was_canceling {
            Ok(RelayReservationEvent::Fenced { handle })
        } else {
            Ok(RelayReservationEvent::CancellationStarted { cancellation })
        }
    }

    fn entry(&self, handle: RelayReservationHandle) -> RelayReservationResult<&ReservationEntry> {
        self.validate_handle(handle)?;
        self.reservations
            .get(&handle.relay_peer_id)
            .ok_or(RelayReservationError::StaleHandle)
    }

    fn entry_mut(
        &mut self,
        handle: RelayReservationHandle,
    ) -> RelayReservationResult<&mut ReservationEntry> {
        self.validate_handle(handle)?;
        self.reservations
            .get_mut(&handle.relay_peer_id)
            .ok_or(RelayReservationError::StaleHandle)
    }

    fn validate_handle(&self, handle: RelayReservationHandle) -> RelayReservationResult<()> {
        if handle.node_instance_id != self.node_instance_id {
            return Err(RelayReservationError::ForeignHandle);
        }
        match self.reservations.get(&handle.relay_peer_id) {
            Some(entry) if entry.handle.generation == handle.generation => Ok(()),
            Some(_) => Err(RelayReservationError::StaleHandle),
            None if self
                .latest_generation
                .get(&handle.relay_peer_id)
                .is_some_and(|latest| *latest >= handle.generation) =>
            {
                Err(RelayReservationError::StaleHandle)
            }
            None => Err(RelayReservationError::UnknownHandle),
        }
    }

    fn finish_cancellation(&mut self, handle: RelayReservationHandle) {
        let removed = self.reservations.remove(&handle.relay_peer_id);
        debug_assert!(removed.is_some(), "validated reservation must exist");
    }
}

#[derive(Debug)]
struct ReservationEntry {
    handle: RelayReservationHandle,
    provider: RelayProvider,
    state: RelayReservationState,
    direct_connection: Option<ConnectionId>,
    connection_terminal: bool,
    listener_observed: bool,
    listener_closed: bool,
    initial_acceptance_observed: bool,
}

impl ReservationEntry {
    fn new(handle: RelayReservationHandle, provider: RelayProvider) -> Self {
        Self {
            handle,
            provider,
            state: RelayReservationState::AwaitingConfirmation,
            direct_connection: None,
            connection_terminal: false,
            listener_observed: false,
            listener_closed: false,
            initial_acceptance_observed: false,
        }
    }

    fn snapshot(&self, local_peer_id: PeerId) -> RelayReservationSnapshot {
        let publishable_route = (self.state == RelayReservationState::Publishable)
            .then(|| self.provider.publishable_route(local_peer_id));
        RelayReservationSnapshot {
            handle: self.handle,
            state: self.state,
            selected_base: self.provider.selected_base().clone(),
            expected_limits: self.provider.expected_limits(),
            direct_connection: self.direct_connection,
            publishable_route,
        }
    }

    fn ensure_listener(&self, actual: ListenerId) -> RelayReservationResult<()> {
        let expected = self.handle.listener_id;
        if expected != actual {
            return Err(RelayReservationError::ListenerMismatch { expected, actual });
        }
        Ok(())
    }

    fn maybe_publish(&mut self, local_peer_id: PeerId) -> RelayReservationEvent {
        if self.initial_acceptance_observed
            && self.listener_observed
            && self.state == RelayReservationState::AwaitingConfirmation
        {
            self.state = RelayReservationState::Publishable;
            return RelayReservationEvent::Publishable {
                handle: self.handle,
                route: self.provider.publishable_route(local_peer_id),
                limits: self.provider.expected_limits(),
            };
        }
        RelayReservationEvent::EvidenceRecorded {
            handle: self.handle,
        }
    }

    fn start_canceling(&mut self) {
        self.state = RelayReservationState::Canceling;
    }

    fn cancellation(&self) -> RelayCancellation {
        RelayCancellation {
            handle: self.handle,
            relay_peer_id: self.provider.relay_peer_id(),
            listener_id: self.handle.listener_id,
            direct_connection: self.direct_connection,
        }
    }

    fn cancellation_complete(&self) -> bool {
        self.state == RelayReservationState::Canceling
            && self.listener_closed
            && self.connection_terminal
    }
}

pub(crate) fn canonicalize_provider_base(
    raw: &str,
    expected_peer_id: PeerId,
) -> RelayReservationResult<Multiaddr> {
    let parts = raw.split('/').collect::<Vec<_>>();
    if parts.len() != 7
        || !parts[0].is_empty()
        || parts[1] != "dns4"
        || parts[3] != "tcp"
        || parts[5] != "p2p"
    {
        return Err(invalid_base(raw, "expected exact dns4/tcp/p2p grammar"));
    }

    let host = canonicalize_fqdn(parts[2])
        .ok_or_else(|| invalid_base(raw, "host is not an allowed public FQDN"))?;
    let port = parts[4]
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(|| invalid_base(raw, "TCP port must be in 1..=65535"))?;

    let actual_peer_id = PeerId::from_str(parts[6])
        .map_err(|_| invalid_base(raw, "terminal p2p component is not a Peer ID"))?;
    if actual_peer_id != expected_peer_id || parts[6] != expected_peer_id.to_string() {
        return Err(RelayReservationError::BasePeerMismatch {
            expected: expected_peer_id,
            actual: parts[6].to_string(),
        });
    }

    let canonical = format!("/dns4/{host}/tcp/{port}/p2p/{expected_peer_id}");
    canonical
        .parse()
        .map_err(|error| invalid_base(raw, format!("invalid canonical multiaddr: {error}")))
}

fn canonicalize_fqdn(raw: &str) -> Option<String> {
    let host = raw.trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() || host.len() > 253 || !host.contains('.') {
        return None;
    }
    if host.parse::<std::net::IpAddr>().is_ok() {
        return None;
    }

    const FORBIDDEN_SUFFIXES: &[&str] = &[
        ".localhost",
        ".local",
        ".internal",
        ".invalid",
        ".test",
        ".example",
        ".home.arpa",
    ];
    if host == "localhost"
        || FORBIDDEN_SUFFIXES
            .iter()
            .any(|suffix| host.ends_with(suffix))
    {
        return None;
    }

    if host.split('.').any(|label| {
        label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    }) {
        return None;
    }
    Some(host)
}

fn invalid_base(raw: &str, reason: impl fmt::Display) -> RelayReservationError {
    RelayReservationError::InvalidBase {
        address: raw.to_string(),
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::Keypair;

    const DATA_LIMIT: u64 = 10_737_418_240;

    fn peer_id() -> PeerId {
        Keypair::generate_ed25519().public().to_peer_id()
    }

    fn limits() -> ExpectedRelayLimits {
        ExpectedRelayLimits::new(Duration::from_secs(900), DATA_LIMIT).unwrap()
    }

    fn observed_limits() -> Option<ObservedRelayLimits> {
        Some(ObservedRelayLimits::new(
            Some(Duration::from_secs(900)),
            Some(DATA_LIMIT),
        ))
    }

    fn provider(relay_peer_id: PeerId, host: &str) -> RelayProvider {
        RelayProvider::new(
            relay_peer_id,
            [format!("/dns4/{host}/tcp/443/p2p/{relay_peer_id}")],
            limits(),
        )
        .unwrap()
    }

    fn response_address(relay_peer_id: PeerId, local_peer_id: PeerId) -> Multiaddr {
        format!("/ip4/203.0.113.9/tcp/4001/p2p/{relay_peer_id}/p2p-circuit/p2p/{local_peer_id}")
            .parse()
            .unwrap()
    }

    #[test]
    fn canonical_provider_is_sorted_and_selects_first_base() {
        let relay = peer_id();
        let provider = RelayProvider::new(
            relay,
            [
                format!("/dns4/Relay-B.Dev.Aukiverse.com./tcp/0443/p2p/{relay}"),
                format!("/dns4/relay-a.dev.aukiverse.com/tcp/443/p2p/{relay}"),
            ],
            limits(),
        )
        .unwrap();

        assert_eq!(
            provider
                .bases()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec![
                format!("/dns4/relay-a.dev.aukiverse.com/tcp/443/p2p/{relay}"),
                format!("/dns4/relay-b.dev.aukiverse.com/tcp/443/p2p/{relay}"),
            ]
        );
        assert_eq!(
            provider.selected_base().to_string(),
            format!("/dns4/relay-a.dev.aukiverse.com/tcp/443/p2p/{relay}")
        );
        assert_eq!(
            provider.reservation_listen_address().to_string(),
            format!("/dns4/relay-a.dev.aukiverse.com/tcp/443/p2p/{relay}/p2p-circuit")
        );
    }

    #[test]
    fn duplicate_after_normalization_is_rejected() {
        let relay = peer_id();
        let error = RelayProvider::new(
            relay,
            [
                format!("/dns4/RELAY.dev.aukiverse.com/tcp/0443/p2p/{relay}"),
                format!("/dns4/relay.dev.aukiverse.com./tcp/443/p2p/{relay}"),
            ],
            limits(),
        )
        .unwrap_err();
        assert!(matches!(error, RelayReservationError::DuplicateBase(_)));
    }

    #[test]
    fn exact_base_grammar_and_public_fqdn_are_required() {
        let relay = peer_id();
        let other = peer_id();
        let invalid = [
            format!("/ip4/203.0.113.1/tcp/443/p2p/{relay}"),
            format!("/dns/relay.dev.aukiverse.com/tcp/443/p2p/{relay}"),
            format!("/dns6/relay.dev.aukiverse.com/tcp/443/p2p/{relay}"),
            format!("/dnsaddr/relay.dev.aukiverse.com/tcp/443/p2p/{relay}"),
            format!("/dns4/relay.dev.aukiverse.com/udp/443/p2p/{relay}"),
            format!("/dns4/relay.dev.aukiverse.com/tcp/0/p2p/{relay}"),
            format!("/dns4/relay.dev.aukiverse.com/tcp/65536/p2p/{relay}"),
            "/dns4/relay.dev.aukiverse.com/tcp/443".to_string(),
            format!("/dns4/relay.dev.aukiverse.com/tcp/443/p2p/{relay}/p2p-circuit"),
            format!("/dns4/relay.dev.aukiverse.com/tcp/443/p2p/{other}"),
            format!("/dns4/localhost/tcp/443/p2p/{relay}"),
            format!("/dns4/relay.localhost/tcp/443/p2p/{relay}"),
            format!("/dns4/relay.local/tcp/443/p2p/{relay}"),
            format!("/dns4/relay.internal/tcp/443/p2p/{relay}"),
            format!("/dns4/relay.invalid/tcp/443/p2p/{relay}"),
            format!("/dns4/relay.test/tcp/443/p2p/{relay}"),
            format!("/dns4/relay.example/tcp/443/p2p/{relay}"),
            format!("/dns4/relay.home.arpa/tcp/443/p2p/{relay}"),
            format!("/dns4/singlelabel/tcp/443/p2p/{relay}"),
            format!("/dns4/192.0.2.1/tcp/443/p2p/{relay}"),
            format!("/dns4/-relay.dev.aukiverse.com/tcp/443/p2p/{relay}"),
            format!("/dns4/relay-.dev.aukiverse.com/tcp/443/p2p/{relay}"),
            format!("/dns4/relay_.dev.aukiverse.com/tcp/443/p2p/{relay}"),
        ];

        for raw in invalid {
            assert!(
                RelayProvider::new(relay, [raw.clone()], limits()).is_err(),
                "accepted invalid base {raw}"
            );
        }
        assert!(matches!(
            RelayProvider::new(relay, std::iter::empty::<String>(), limits()),
            Err(RelayReservationError::EmptyBases)
        ));
    }

    #[test]
    fn expected_limits_are_positive_finite_whole_seconds() {
        assert!(ExpectedRelayLimits::new(Duration::ZERO, 1).is_err());
        assert!(ExpectedRelayLimits::new(Duration::from_secs(1), 0).is_err());
        assert!(ExpectedRelayLimits::new(Duration::from_nanos(1_000_000_001), 1).is_err());
        assert!(ExpectedRelayLimits::new(Duration::from_secs(u64::from(u32::MAX) + 1), 1).is_err());
        assert!(ExpectedRelayLimits::new(Duration::from_secs(1), i64::MAX as u64 + 1).is_err());
        assert!(ExpectedRelayLimits::new(
            Duration::from_secs(u64::from(u32::MAX)),
            i64::MAX as u64
        )
        .is_ok());
    }

    #[test]
    fn acceptance_then_listener_makes_only_selected_base_publishable() {
        let local = peer_id();
        let relay = peer_id();
        let provider = provider(relay, "relay-a.dev.aukiverse.com");
        let listener = ListenerId::next();
        let mut node = RelayReservationNode::new(local);
        let handle = node.begin(provider, listener).unwrap();

        assert!(matches!(
            node.observe_acceptance(handle, false, observed_limits())
                .unwrap(),
            RelayReservationEvent::EvidenceRecorded { .. }
        ));
        let response_only = response_address(relay, local);
        let event = node
            .observe_listener_address(handle, listener, &response_only)
            .unwrap();
        let RelayReservationEvent::Publishable { route, .. } = event else {
            panic!("expected publishable event");
        };
        assert_eq!(
            route.to_string(),
            format!("/dns4/relay-a.dev.aukiverse.com/tcp/443/p2p/{relay}/p2p-circuit/p2p/{local}")
        );
        assert_ne!(route, response_only);
        assert_eq!(
            node.snapshot(handle).unwrap().publishable_route(),
            Some(&route)
        );
    }

    #[test]
    fn listener_then_acceptance_is_equally_confirmed() {
        let local = peer_id();
        let relay = peer_id();
        let listener = ListenerId::next();
        let mut node = RelayReservationNode::new(local);
        let handle = node
            .begin(provider(relay, "relay-b.dev.aukiverse.com"), listener)
            .unwrap();

        let response = response_address(relay, local);
        assert!(matches!(
            node.observe_listener_address(handle, listener, &response)
                .unwrap(),
            RelayReservationEvent::EvidenceRecorded { .. }
        ));
        assert!(matches!(
            node.observe_acceptance(handle, false, observed_limits())
                .unwrap(),
            RelayReservationEvent::Publishable { .. }
        ));
        assert_eq!(
            node.snapshot(handle).unwrap().state(),
            RelayReservationState::Publishable
        );
    }

    #[test]
    fn different_relays_progress_independently() {
        let local = peer_id();
        let relay_a = peer_id();
        let relay_b = peer_id();
        let listener_a = ListenerId::next();
        let listener_b = ListenerId::next();
        let mut node = RelayReservationNode::new(local);
        let handle_a = node
            .begin(provider(relay_a, "relay-a.dev.aukiverse.com"), listener_a)
            .unwrap();
        let handle_b = node
            .begin(provider(relay_b, "relay-b.dev.aukiverse.com"), listener_b)
            .unwrap();

        assert!(matches!(
            node.begin(
                provider(relay_a, "relay-a2.dev.aukiverse.com"),
                ListenerId::next()
            ),
            Err(RelayReservationError::ReservationAlreadyExists(peer)) if peer == relay_a
        ));
        let response_a = response_address(relay_a, local);
        node.observe_listener_address(handle_a, listener_a, &response_a)
            .unwrap();
        node.observe_acceptance(handle_a, false, observed_limits())
            .unwrap();
        assert_eq!(
            node.snapshot(handle_a).unwrap().state(),
            RelayReservationState::Publishable
        );
        assert_eq!(
            node.snapshot(handle_b).unwrap().state(),
            RelayReservationState::AwaitingConfirmation
        );
    }

    #[test]
    fn missing_and_mismatched_limits_fail_closed_without_touching_sibling() {
        let local = peer_id();
        let relay_a = peer_id();
        let relay_b = peer_id();
        let listener_a = ListenerId::next();
        let listener_b = ListenerId::next();
        let mut node = RelayReservationNode::new(local);
        let handle_a = node
            .begin(provider(relay_a, "relay-a.dev.aukiverse.com"), listener_a)
            .unwrap();
        let handle_b = node
            .begin(provider(relay_b, "relay-b.dev.aukiverse.com"), listener_b)
            .unwrap();

        assert!(matches!(
            node.observe_acceptance(handle_a, false, None).unwrap(),
            RelayReservationEvent::ConfirmationRejected {
                reason: RelayConfirmationRejection::MissingLimits,
                ..
            }
        ));
        assert_eq!(
            node.snapshot(handle_a).unwrap().state(),
            RelayReservationState::Canceling
        );
        assert!(node
            .snapshot(handle_a)
            .unwrap()
            .publishable_route()
            .is_none());

        let response_b = response_address(relay_b, local);
        node.observe_listener_address(handle_b, listener_b, &response_b)
            .unwrap();
        let mismatch = Some(ObservedRelayLimits::new(
            Some(Duration::from_secs(901)),
            Some(DATA_LIMIT),
        ));
        assert!(matches!(
            node.observe_acceptance(handle_b, false, mismatch).unwrap(),
            RelayReservationEvent::ConfirmationRejected {
                reason: RelayConfirmationRejection::LimitMismatch { .. },
                ..
            }
        ));
        assert_eq!(
            node.snapshot(handle_b).unwrap().state(),
            RelayReservationState::Canceling
        );
    }

    #[test]
    fn renewal_requires_initial_acceptance_and_exact_limits() {
        let local = peer_id();
        let relay = peer_id();
        let listener = ListenerId::next();
        let mut node = RelayReservationNode::new(local);
        let handle = node
            .begin(provider(relay, "relay.dev.aukiverse.com"), listener)
            .unwrap();

        assert!(matches!(
            node.observe_acceptance(handle, true, observed_limits())
                .unwrap(),
            RelayReservationEvent::ConfirmationRejected {
                reason: RelayConfirmationRejection::RenewalBeforeInitialAcceptance,
                ..
            }
        ));

        node.observe_listener_closed(handle, listener).unwrap();
        node.observe_no_direct_connection(handle).unwrap();
        let next_listener = ListenerId::next();
        let next = node
            .begin(provider(relay, "relay.dev.aukiverse.com"), next_listener)
            .unwrap();
        node.observe_acceptance(next, false, observed_limits())
            .unwrap();
        assert!(matches!(
            node.observe_acceptance(next, true, observed_limits())
                .unwrap(),
            RelayReservationEvent::Renewed { .. }
        ));
        assert!(matches!(
            node.observe_acceptance(
                next,
                true,
                Some(ObservedRelayLimits::new(
                    Some(Duration::from_secs(900)),
                    Some(DATA_LIMIT - 1)
                ))
            )
            .unwrap(),
            RelayReservationEvent::ConfirmationRejected {
                reason: RelayConfirmationRejection::LimitMismatch { .. },
                ..
            }
        ));
        assert_eq!(
            node.snapshot(next).unwrap().state(),
            RelayReservationState::Canceling
        );
    }

    #[test]
    fn cancellation_tombstones_until_listener_and_connection_close() {
        let local = peer_id();
        let relay = peer_id();
        let listener = ListenerId::next();
        let connection = ConnectionId::new_unchecked(41);
        let mut node = RelayReservationNode::new(local);
        let handle = node
            .begin(provider(relay, "relay.dev.aukiverse.com"), listener)
            .unwrap();
        node.observe_direct_connection(handle, connection).unwrap();
        let response = response_address(relay, local);
        node.observe_listener_address(handle, listener, &response)
            .unwrap();
        node.observe_acceptance(handle, false, observed_limits())
            .unwrap();

        let RelayReservationEvent::CancellationStarted { cancellation } =
            node.cancel(handle).unwrap()
        else {
            panic!("expected cancellation start");
        };
        assert_eq!(cancellation.listener_id(), listener);
        assert_eq!(cancellation.direct_connection(), Some(connection));
        assert!(node.snapshot(handle).unwrap().publishable_route().is_none());
        assert!(matches!(
            node.cancel(handle).unwrap(),
            RelayReservationEvent::CancellationPending { .. }
        ));
        assert!(matches!(
            node.observe_acceptance(handle, true, observed_limits())
                .unwrap(),
            RelayReservationEvent::Fenced { .. }
        ));
        assert!(matches!(
            node.begin(
                provider(relay, "relay-new.dev.aukiverse.com"),
                ListenerId::next()
            ),
            Err(RelayReservationError::ReservationAlreadyExists(_))
        ));

        assert!(matches!(
            node.observe_listener_closed(handle, listener).unwrap(),
            RelayReservationEvent::Fenced { .. }
        ));
        assert!(matches!(
            node.observe_direct_connection_closed(handle, connection)
                .unwrap(),
            RelayReservationEvent::Canceled { .. }
        ));
        assert!(matches!(
            node.snapshot(handle),
            Err(RelayReservationError::StaleHandle)
        ));

        let replacement = node
            .begin(
                provider(relay, "relay-new.dev.aukiverse.com"),
                ListenerId::next(),
            )
            .unwrap();
        assert!(replacement.generation() > handle.generation());
        assert!(matches!(
            node.observe_acceptance(handle, false, observed_limits()),
            Err(RelayReservationError::StaleHandle)
        ));
    }

    #[test]
    fn cancel_before_connection_waits_for_terminal_dial_evidence() {
        let local = peer_id();
        let relay = peer_id();
        let listener = ListenerId::next();
        let mut node = RelayReservationNode::new(local);
        let handle = node
            .begin(provider(relay, "relay.dev.aukiverse.com"), listener)
            .unwrap();
        node.cancel(handle).unwrap();
        node.observe_listener_closed(handle, listener).unwrap();
        assert!(node.snapshot(handle).is_ok());
        assert!(matches!(
            node.observe_no_direct_connection(handle).unwrap(),
            RelayReservationEvent::Canceled { .. }
        ));
    }

    #[test]
    fn connection_appearing_after_cancel_is_closed_and_fenced() {
        let local = peer_id();
        let relay = peer_id();
        let listener = ListenerId::next();
        let late_connection = ConnectionId::new_unchecked(51);
        let mut node = RelayReservationNode::new(local);
        let handle = node
            .begin(provider(relay, "relay.dev.aukiverse.com"), listener)
            .unwrap();
        node.cancel(handle).unwrap();

        assert_eq!(
            node.observe_direct_connection(handle, late_connection)
                .unwrap(),
            RelayReservationEvent::CloseLateConnection {
                handle,
                connection_id: late_connection,
            }
        );
        node.observe_listener_closed(handle, listener).unwrap();
        assert!(matches!(
            node.observe_direct_connection_closed(handle, late_connection)
                .unwrap(),
            RelayReservationEvent::Canceled { .. }
        ));
    }

    #[test]
    fn foreign_handles_and_wrong_evidence_ids_are_rejected() {
        let local = peer_id();
        let relay = peer_id();
        let listener = ListenerId::next();
        let mut first = RelayReservationNode::new(local);
        let second = RelayReservationNode::new(local);
        let handle = first
            .begin(provider(relay, "relay.dev.aukiverse.com"), listener)
            .unwrap();

        assert!(matches!(
            second.snapshot(handle),
            Err(RelayReservationError::ForeignHandle)
        ));
        let response = response_address(relay, local);
        let wrong_listener = ListenerId::next();
        assert!(matches!(
            first.observe_listener_address(handle, wrong_listener, &response),
            Err(RelayReservationError::ListenerMismatch { .. })
        ));

        let selected = ConnectionId::new_unchecked(61);
        first.observe_direct_connection(handle, selected).unwrap();
        assert!(matches!(
            first.observe_direct_connection(handle, ConnectionId::new_unchecked(62)),
            Err(RelayReservationError::ConnectionMismatch { .. })
        ));
        first.cancel(handle).unwrap();
        assert!(matches!(
            first.observe_no_direct_connection(handle),
            Err(RelayReservationError::ConnectionStillOpen(id)) if id == selected
        ));
    }
}
