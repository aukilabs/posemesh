//! Native, direct libp2p transport gated by mutual DDS authorization.
//!
//! libp2p owns Ed25519 identities, Peer IDs, TCP, Noise, Yamux, dialing, and
//! versioned streams. This crate adds only the DDS P2P JWT checks required
//! before one of those streams becomes an [`AuthenticatedStream`]. It does not
//! fetch credentials or understand tasks, datasets, or machine-auth flows.

mod authority;
mod error;
mod identity;
mod relay;
mod relay_client;
mod routing;
mod runtime;
mod source_admission;
mod targeted_stream;
mod token;
mod transport;

pub use authority::{P2pCredentialError, P2pCredentialResult, P2pCredentialStore};
pub use error::{Error, Result};
pub use identity::Identity;
pub use libp2p::{multiaddr::Protocol, Multiaddr, PeerId};
pub use relay::{
    ExpectedRelayLimits, RelayConfirmationRejection, RelayProvider, RelayReservationError,
    RelayReservationHandle, RelayReservationSnapshot, RelayReservationState, ReservationGeneration,
};
pub use routing::{
    canonicalize_circuit_route, validate_direct_route, CanonicalCircuitRoute, ConfirmedRoute,
    PublishedRoute, RouteCatalog, RouteCatalogError, RouteCatalogLimits, RouteCatalogResult,
    RouteCatalogStatus, RouteFence, RouteSnapshot,
};
pub use runtime::{AuthenticatedRouteStream, ExactRoute, ProtocolServer, ProtocolSpec};
pub use targeted_stream::TargetedStreamError;
pub use token::{
    DdsTokenVerifier, P2PAccessClaims, PeerRole, DOMAIN_SERVER_MAX_DOMAINS, P2P_TOKEN_AUDIENCE,
    P2P_TOKEN_ISSUER, P2P_TOKEN_SCOPE, P2P_TOKEN_TTL, P2P_TOKEN_TYPE,
};
pub use transport::{
    ApplicationProtocol, AuthenticatedPeer, AuthenticatedStream, IncomingAuthenticatedStreams,
    Node, RelayRouteHandle, RelayTransportEvent, SessionRequirements,
};
