use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to sign libp2p identity proof: {0}")]
    IdentitySigning(String),
    #[error("invalid DDS verification key: {0}")]
    InvalidVerificationKey(#[from] jsonwebtoken::errors::Error),
    #[error("invalid DDS P2P token: {0}")]
    InvalidToken(String),
    #[error("DDS P2P token signature or registered claims are invalid: {0}")]
    TokenVerification(jsonwebtoken::errors::Error),
    #[error("no current DDS P2P token is installed")]
    MissingToken,
    #[error("token Peer ID {token_peer_id} does not match Noise Peer ID {noise_peer_id}")]
    PeerIdMismatch {
        token_peer_id: String,
        noise_peer_id: String,
    },
    #[error("remote peer role {actual} is not allowed; expected {expected}")]
    RemoteRoleMismatch { expected: String, actual: String },
    #[error("remote token is not authorized for required Domain {0}")]
    RemoteDomainMismatch(String),
    #[error("expected remote Peer ID {expected}, connected to {actual}")]
    UnexpectedRemotePeer { expected: String, actual: String },
    #[error("remote peer rejected mutual authentication")]
    RemoteRejected,
    #[error("mutual authentication timed out")]
    AuthenticationTimeout,
    #[error("authentication token frame exceeds the {0}-byte limit")]
    TokenFrameTooLarge(usize),
    #[error("authentication token is not valid UTF-8")]
    InvalidTokenEncoding,
    #[error("invalid application protocol: {0}")]
    InvalidProtocol(String),
    #[error("application protocol is already registered")]
    ProtocolAlreadyRegistered,
    #[error("at least one explicit remote TCP multiaddr is required")]
    MissingRemoteAddress,
    #[error("invalid remote multiaddr {address}: {reason}")]
    InvalidRemoteAddress { address: String, reason: String },
    #[error("failed to build libp2p transport: {0}")]
    TransportBuild(String),
    #[error("auki-p2p requires an active Tokio runtime")]
    RuntimeUnavailable,
    #[error("failed to listen on {address}: {reason}")]
    Listen { address: String, reason: String },
    #[error("failed to open libp2p stream: {0}")]
    OpenStream(String),
    #[error("failed to dial libp2p peer: {0}")]
    Dial(String),
    #[error("libp2p swarm task has stopped")]
    SwarmStopped,
    #[error("failed to disconnect libp2p peer {0}")]
    Disconnect(String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
