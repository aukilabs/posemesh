use std::{collections::BTreeMap, time::Duration};

use chrono::{DateTime, Utc};
use tokio::sync::watch;
use uuid::Uuid;

use crate::{ExpectedRelayLimits, Multiaddr, PeerId, Protocol, RelayProvider};

/// Opaque authority fence for one stable advertised route slot.
///
/// The P2P layer deliberately does not interpret the external authority IDs.
/// A DMS-backed host can map its assignment and epoch into these fields while
/// another host can use any equivalent stable UUID generations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RouteFence {
    pub route_id: Uuid,
    pub authority_id: Uuid,
    pub authority_epoch: Uuid,
    pub local_generation: u64,
}

#[derive(Clone, Debug)]
pub struct ConfirmedRoute {
    pub fence: RouteFence,
    pub relay_peer_id: PeerId,
    pub route: Multiaddr,
    pub limits: ExpectedRelayLimits,
    pub authorized_until: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublishedRoute {
    pub fence: RouteFence,
    pub relay_peer_id: PeerId,
    pub route: Multiaddr,
    pub limits: ExpectedRelayLimits,
    pub authorized_until: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteSnapshot {
    pub revision: u64,
    pub direct_routes: Vec<Multiaddr>,
    pub relay_routes: Vec<PublishedRoute>,
}

impl RouteSnapshot {
    pub fn fences(&self) -> impl Iterator<Item = RouteFence> + '_ {
        self.relay_routes.iter().map(|route| route.fence)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteCatalogStatus {
    pub revision: u64,
    pub direct_route_count: usize,
    pub confirmed_relay_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteCatalogLimits {
    pub maximum_routes: usize,
    pub maximum_relay_routes: usize,
}

impl RouteCatalogLimits {
    pub fn new(maximum_routes: usize, maximum_relay_routes: usize) -> Self {
        Self {
            maximum_routes,
            maximum_relay_routes,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RouteCatalogError {
    #[error("P2P route set exceeds the {maximum}-route limit")]
    RouteLimitExceeded { maximum: usize },
    #[error("P2P route set exceeds the {maximum}-circuit limit")]
    CircuitRouteLimitExceeded { maximum: usize },
    #[error("P2P direct route is invalid: {0}")]
    InvalidDirectRoute(String),
    #[error("confirmed relay route is invalid: {0}")]
    InvalidRelayRoute(String),
    #[error("confirmed relay authorization is already expired")]
    RelayAuthorizationExpired,
    #[error("confirmed relay slot already contains a different fence")]
    RelaySlotOccupied,
    #[error("confirmed relay route fence is stale")]
    StaleRouteFence,
    #[error("confirmed relay route was not found")]
    RouteNotFound,
    #[error("confirmed relay Peer ID or endpoint duplicates another slot")]
    DuplicateRelayRoute,
    #[error("P2P route revision is exhausted")]
    RevisionExhausted,
}

pub type RouteCatalogResult<T> = std::result::Result<T, RouteCatalogError>;

#[derive(Clone)]
pub struct RouteCatalog {
    inner: std::sync::Arc<RouteCatalogInner>,
}

struct RouteCatalogInner {
    local_peer_id: PeerId,
    limits: RouteCatalogLimits,
    state: parking_lot::Mutex<RouteState>,
    status: watch::Sender<RouteCatalogStatus>,
}

struct RouteState {
    direct_routes: Vec<Multiaddr>,
    relay_routes: BTreeMap<Uuid, StoredRelayRoute>,
    revision: u64,
}

#[derive(Clone)]
struct StoredRelayRoute {
    fence: RouteFence,
    relay_peer_id: PeerId,
    endpoint: String,
    route: Multiaddr,
    limits: ExpectedRelayLimits,
    authorized_until: DateTime<Utc>,
    publishable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalCircuitRoute {
    pub route: Multiaddr,
    pub relay_peer_id: PeerId,
    pub endpoint_key: String,
}

impl RouteCatalog {
    pub fn new(
        local_peer_id: PeerId,
        direct_routes: Vec<Multiaddr>,
        limits: RouteCatalogLimits,
    ) -> RouteCatalogResult<Self> {
        let direct_routes =
            validate_and_sort_direct_routes(direct_routes, local_peer_id, limits.maximum_routes)?;
        Ok(Self::from_routes(local_peer_id, direct_routes, limits))
    }

    /// Compatibility constructor for callers that historically accepted any
    /// sorted direct multiaddr. New protocol hosts should use [`Self::new`].
    #[doc(hidden)]
    pub fn from_unvalidated_routes(
        local_peer_id: PeerId,
        direct_routes: Vec<Multiaddr>,
        limits: RouteCatalogLimits,
    ) -> Self {
        Self::from_routes(local_peer_id, sorted_unique_routes(direct_routes), limits)
    }

    fn from_routes(
        local_peer_id: PeerId,
        direct_routes: Vec<Multiaddr>,
        limits: RouteCatalogLimits,
    ) -> Self {
        let revision = u64::from(!direct_routes.is_empty());
        let state = RouteState {
            direct_routes,
            relay_routes: BTreeMap::new(),
            revision,
        };
        let (status, _) = watch::channel(route_status(&state, Utc::now()));
        Self {
            inner: std::sync::Arc::new(RouteCatalogInner {
                local_peer_id,
                limits,
                state: parking_lot::Mutex::new(state),
                status,
            }),
        }
    }

    pub fn local_peer_id(&self) -> PeerId {
        self.inner.local_peer_id
    }

    pub fn replace_direct_routes(&self, direct_routes: Vec<Multiaddr>) -> RouteCatalogResult<u64> {
        let direct_routes = validate_and_sort_direct_routes(
            direct_routes,
            self.inner.local_peer_id,
            self.inner.limits.maximum_routes,
        )?;
        let now = Utc::now();
        let (revision, status) = {
            let mut state = self.inner.state.lock();
            expire_authorizations(&mut state, now)?;
            if state.direct_routes == direct_routes {
                return Ok(state.revision);
            }
            let confirmed = publishable_relay_count(&state, now);
            if direct_routes.len().saturating_add(confirmed) > self.inner.limits.maximum_routes {
                return Err(RouteCatalogError::RouteLimitExceeded {
                    maximum: self.inner.limits.maximum_routes,
                });
            }
            bump_revision(&mut state)?;
            state.direct_routes = direct_routes;
            (state.revision, route_status(&state, now))
        };
        self.inner.status.send_replace(status);
        Ok(revision)
    }

    pub fn publish_confirmed(&self, route: ConfirmedRoute) -> RouteCatalogResult<u64> {
        if route.fence.local_generation == 0 {
            return Err(RouteCatalogError::InvalidRelayRoute(
                "local generation must be positive".to_string(),
            ));
        }
        let (relay_peer_id, endpoint) =
            validate_circuit_route(&route.route, route.relay_peer_id, self.inner.local_peer_id)?;
        let now = Utc::now();
        if route.authorized_until <= now {
            return Err(RouteCatalogError::RelayAuthorizationExpired);
        }

        let (revision, status) = {
            let mut state = self.inner.state.lock();
            expire_authorizations(&mut state, now)?;
            if let Some(existing) = state.relay_routes.get(&route.fence.route_id) {
                if existing.fence != route.fence
                    || existing.relay_peer_id != relay_peer_id
                    || existing.endpoint != endpoint
                    || existing.route != route.route
                    || existing.limits != route.limits
                {
                    return Err(RouteCatalogError::RelaySlotOccupied);
                }
            } else {
                if state.relay_routes.len() >= self.inner.limits.maximum_relay_routes {
                    return Err(RouteCatalogError::CircuitRouteLimitExceeded {
                        maximum: self.inner.limits.maximum_relay_routes,
                    });
                }
                if state.relay_routes.values().any(|existing| {
                    existing.relay_peer_id == relay_peer_id || existing.endpoint == endpoint
                }) {
                    return Err(RouteCatalogError::DuplicateRelayRoute);
                }
            }

            let already_publishable = state
                .relay_routes
                .get(&route.fence.route_id)
                .is_some_and(|existing| existing.publishable);
            if !already_publishable
                && state
                    .direct_routes
                    .len()
                    .saturating_add(publishable_relay_count(&state, now))
                    .saturating_add(1)
                    > self.inner.limits.maximum_routes
            {
                return Err(RouteCatalogError::RouteLimitExceeded {
                    maximum: self.inner.limits.maximum_routes,
                });
            }
            if !already_publishable {
                bump_revision(&mut state)?;
            }
            state.relay_routes.insert(
                route.fence.route_id,
                StoredRelayRoute {
                    fence: route.fence,
                    relay_peer_id,
                    endpoint,
                    route: route.route,
                    limits: route.limits,
                    authorized_until: route.authorized_until,
                    publishable: true,
                },
            );
            (state.revision, route_status(&state, now))
        };
        self.inner.status.send_replace(status);
        Ok(revision)
    }

    pub fn refresh_authorization(
        &self,
        fence: RouteFence,
        authorized_until: DateTime<Utc>,
    ) -> RouteCatalogResult<u64> {
        let now = Utc::now();
        let (revision, status) = {
            let mut state = self.inner.state.lock();
            expire_authorizations(&mut state, now)?;
            let entry = state
                .relay_routes
                .get(&fence.route_id)
                .ok_or(RouteCatalogError::RouteNotFound)?;
            if entry.fence != fence {
                return Err(RouteCatalogError::StaleRouteFence);
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
                    > self.inner.limits.maximum_routes
            {
                return Err(RouteCatalogError::RouteLimitExceeded {
                    maximum: self.inner.limits.maximum_routes,
                });
            }
            if changed_publishability {
                bump_revision(&mut state)?;
            }
            let entry = state
                .relay_routes
                .get_mut(&fence.route_id)
                .expect("route entry remains locked");
            entry.authorized_until = authorized_until;
            entry.publishable = should_publish;
            (state.revision, route_status(&state, now))
        };
        self.inner.status.send_replace(status);
        Ok(revision)
    }

    pub fn tombstone(&self, fence: RouteFence) -> RouteCatalogResult<()> {
        let now = Utc::now();
        let status = {
            let mut state = self.inner.state.lock();
            expire_authorizations(&mut state, now)?;
            let existing = state
                .relay_routes
                .get(&fence.route_id)
                .ok_or(RouteCatalogError::RouteNotFound)?;
            if existing.fence != fence {
                return Err(RouteCatalogError::StaleRouteFence);
            }
            if existing.publishable {
                bump_revision(&mut state)?;
            }
            state.relay_routes.remove(&fence.route_id);
            route_status(&state, now)
        };
        self.inner.status.send_replace(status);
        Ok(())
    }

    pub fn tombstone_all(&self) -> RouteCatalogResult<Vec<RouteFence>> {
        let now = Utc::now();
        let (fences, status) = {
            let mut state = self.inner.state.lock();
            expire_authorizations(&mut state, now)?;
            let publishable = state
                .relay_routes
                .values()
                .filter(|entry| entry.publishable)
                .count();
            advance_revision(&mut state, publishable)?;
            let routes = std::mem::take(&mut state.relay_routes);
            (
                routes.into_values().map(|route| route.fence).collect(),
                route_status(&state, now),
            )
        };
        self.inner.status.send_replace(status);
        Ok(fences)
    }

    pub fn snapshot(&self) -> RouteCatalogResult<RouteSnapshot> {
        let now = Utc::now();
        let (snapshot, status) = {
            let mut state = self.inner.state.lock();
            expire_authorizations(&mut state, now)?;
            (route_snapshot(&state, now), route_status(&state, now))
        };
        self.inner.status.send_if_modified(|current| {
            if *current == status {
                false
            } else {
                *current = status.clone();
                true
            }
        });
        Ok(snapshot)
    }

    /// Execute a synchronous publication commit while the exact route revision
    /// and selected authority fences remain current.
    pub fn commit_if_current<T>(
        &self,
        expected_revision: u64,
        expected_fences: &[RouteFence],
        commit: impl FnOnce(&RouteSnapshot) -> T,
    ) -> RouteCatalogResult<Option<T>> {
        let now = Utc::now();
        let mut state = self.inner.state.lock();
        expire_authorizations(&mut state, now)?;
        let current = state.revision == expected_revision
            && expected_fences.iter().all(|fence| {
                state
                    .relay_routes
                    .get(&fence.route_id)
                    .is_some_and(|route| {
                        route.fence == *fence && route.publishable && route.authorized_until > now
                    })
            });
        if !current {
            return Ok(None);
        }
        let snapshot = route_snapshot(&state, now);
        Ok(Some(commit(&snapshot)))
    }

    pub fn status(&self) -> RouteCatalogResult<RouteCatalogStatus> {
        self.snapshot()?;
        Ok(self.inner.status.borrow().clone())
    }

    pub fn subscribe(&self) -> watch::Receiver<RouteCatalogStatus> {
        self.inner.status.subscribe()
    }
}

fn route_snapshot(state: &RouteState, now: DateTime<Utc>) -> RouteSnapshot {
    RouteSnapshot {
        revision: state.revision,
        direct_routes: state.direct_routes.clone(),
        relay_routes: state
            .relay_routes
            .values()
            .filter(|route| route.publishable && route.authorized_until > now)
            .map(|route| PublishedRoute {
                fence: route.fence,
                relay_peer_id: route.relay_peer_id,
                route: route.route.clone(),
                limits: route.limits,
                authorized_until: route.authorized_until,
            })
            .collect(),
    }
}

fn route_status(state: &RouteState, now: DateTime<Utc>) -> RouteCatalogStatus {
    RouteCatalogStatus {
        revision: state.revision,
        direct_route_count: state.direct_routes.len(),
        confirmed_relay_count: publishable_relay_count(state, now),
    }
}

fn sorted_unique_routes(mut routes: Vec<Multiaddr>) -> Vec<Multiaddr> {
    routes.sort_unstable_by_key(ToString::to_string);
    routes.dedup();
    routes
}

fn validate_and_sort_direct_routes(
    routes: Vec<Multiaddr>,
    local_peer_id: PeerId,
    maximum: usize,
) -> RouteCatalogResult<Vec<Multiaddr>> {
    let routes = routes
        .into_iter()
        .map(|route| validate_direct_route(&route, local_peer_id))
        .collect::<RouteCatalogResult<Vec<_>>>()?;
    let routes = sorted_unique_routes(routes);
    if routes.len() > maximum {
        return Err(RouteCatalogError::RouteLimitExceeded { maximum });
    }
    Ok(routes)
}

pub fn validate_direct_route(
    route: &Multiaddr,
    expected_peer_id: PeerId,
) -> RouteCatalogResult<Multiaddr> {
    let protocols = route.iter().collect::<Vec<_>>();
    let (network, port, suffix) = match protocols.as_slice() {
        [network, Protocol::Tcp(port)] => (network, *port, None),
        [network, Protocol::Tcp(port), Protocol::P2p(peer_id)] => (network, *port, Some(*peer_id)),
        _ => {
            return Err(RouteCatalogError::InvalidDirectRoute(
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
        return Err(RouteCatalogError::InvalidDirectRoute(
            "the address must be ip4, ip6, dns, dns4, or dns6".to_string(),
        ));
    }
    if port == 0 {
        return Err(RouteCatalogError::InvalidDirectRoute(
            "TCP port must be non-zero".to_string(),
        ));
    }
    if suffix.is_some_and(|peer_id| peer_id != expected_peer_id) {
        return Err(RouteCatalogError::InvalidDirectRoute(
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
) -> RouteCatalogResult<(PeerId, String)> {
    let canonical = canonicalize_circuit_route(route, expected_target_peer_id)?;
    if canonical.relay_peer_id != expected_relay_peer_id {
        return Err(RouteCatalogError::InvalidRelayRoute(
            "route relay Peer ID does not match the reservation".to_string(),
        ));
    }
    if canonical.route != *route {
        return Err(RouteCatalogError::InvalidRelayRoute(
            "relay route is not canonical".to_string(),
        ));
    }
    Ok((canonical.relay_peer_id, canonical.endpoint_key))
}

pub fn canonicalize_circuit_route(
    route: &Multiaddr,
    expected_target_peer_id: PeerId,
) -> RouteCatalogResult<CanonicalCircuitRoute> {
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
            return Err(RouteCatalogError::InvalidRelayRoute(
                "expected exact dns4/tcp/p2p/p2p-circuit/p2p grammar".to_string(),
            ));
        }
    };
    if target_peer_id != expected_target_peer_id {
        return Err(RouteCatalogError::InvalidRelayRoute(
            "route target Peer ID does not match the expected node".to_string(),
        ));
    }
    let provider_base = format!("/dns4/{host}/tcp/{port}/p2p/{relay_peer_id}");
    let validation_limits = ExpectedRelayLimits::new(Duration::from_secs(1), 1)
        .map_err(|error| RouteCatalogError::InvalidRelayRoute(error.to_string()))?;
    let provider = RelayProvider::new(relay_peer_id, [&provider_base], validation_limits)
        .map_err(|error| RouteCatalogError::InvalidRelayRoute(error.to_string()))?;
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
            return Err(RouteCatalogError::InvalidRelayRoute(
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

fn publishable_relay_count(state: &RouteState, now: DateTime<Utc>) -> usize {
    state
        .relay_routes
        .values()
        .filter(|route| route.publishable && route.authorized_until > now)
        .count()
}

fn bump_revision(state: &mut RouteState) -> RouteCatalogResult<u64> {
    advance_revision(state, 1)?;
    Ok(state.revision)
}

fn advance_revision(state: &mut RouteState, count: usize) -> RouteCatalogResult<()> {
    let count = u64::try_from(count).map_err(|_| RouteCatalogError::RevisionExhausted)?;
    state.revision = state
        .revision
        .checked_add(count)
        .ok_or(RouteCatalogError::RevisionExhausted)?;
    Ok(())
}

fn expire_authorizations(state: &mut RouteState, now: DateTime<Utc>) -> RouteCatalogResult<()> {
    let expired = state
        .relay_routes
        .values()
        .filter(|route| route.publishable && route.authorized_until <= now)
        .map(|route| route.fence.route_id)
        .collect::<Vec<_>>();
    advance_revision(state, expired.len())?;
    for route_id in expired {
        if let Some(route) = state.relay_routes.get_mut(&route_id) {
            route.publishable = false;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_routes_are_canonical_and_revisioned() {
        let peer = PeerId::random();
        let route: Multiaddr = format!("/ip4/127.0.0.1/tcp/4001/p2p/{peer}")
            .parse()
            .unwrap();
        let catalog = RouteCatalog::new(peer, vec![route], RouteCatalogLimits::new(16, 3)).unwrap();
        let snapshot = catalog.snapshot().unwrap();
        assert_eq!(snapshot.revision, 1);
        assert_eq!(
            snapshot.direct_routes[0].to_string(),
            "/ip4/127.0.0.1/tcp/4001"
        );
    }
}
