use posemesh_utils::crypto::CryptoError;
use prost::{DecodeError, EncodeError};

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[cfg(target_family = "wasm")]
    #[error("Failed to open socket: {0}")]
    OpenSocketError(String),
    #[cfg(not(target_family = "wasm"))]
    #[error("Failed to open socket: {0}")]
    OpenSocketError(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("Failed to decode ws meesage: {0}")]
    DecodeError(#[from] DecodeError),
    #[error("Failed to encode ws message: {0}")]
    EncodeError(#[from] EncodeError),
    #[error("Invalid ws url: {0}")]
    InvalidUrl(String),
    #[error("Invalid registration credential")]
    InvalidCredentials,
    #[error("Crypto error: {0}")]
    CryptoError(#[from] CryptoError),
}
