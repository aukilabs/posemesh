//! Native, direct libp2p transport gated by mutual DDS authorization.
//!
//! libp2p owns Ed25519 identities, Peer IDs, TCP, Noise, Yamux, dialing, and
//! versioned streams. This crate adds only the DDS P2P JWT checks required
//! before one of those streams becomes an [`AuthenticatedStream`]. It does not
//! fetch credentials or understand tasks, datasets, or machine-auth flows.

mod error;
mod identity;
mod token;
mod transport;

pub use error::{Error, Result};
pub use identity::Identity;
pub use libp2p::{Multiaddr, PeerId};
pub use token::{
    DdsTokenVerifier, P2PAccessClaims, PeerRole, DOMAIN_SERVER_MAX_DOMAINS, P2P_TOKEN_AUDIENCE,
    P2P_TOKEN_ISSUER, P2P_TOKEN_SCOPE, P2P_TOKEN_TTL, P2P_TOKEN_TYPE,
};
pub use transport::{
    ApplicationProtocol, AuthenticatedPeer, AuthenticatedStream, IncomingAuthenticatedStreams,
    Node, SessionRequirements,
};
