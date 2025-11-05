use futures::channel::{mpsc::SendError, oneshot::Canceled};
use reqwest::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials: {0}")]
    Unauthorized(&'static str),
    #[error("Parse JWT token: base64 decode error: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),
    #[error("JSON parse error: {0}")]
    JsonParseError(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct AukiErrorResponse {
    pub status: StatusCode,
    pub error: String,
}

impl std::fmt::Display for AukiErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Auki response - status: {}, error: {}", self.status, self.error)
    }
}

impl std::error::Error for AukiErrorResponse {}

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("{0}")]
    AukiErrorResponse(#[from] AukiErrorResponse),
    #[error("Invalid content-type header")]
    InvalidContentTypeHeader,
    #[error("Stream error: {0}")]
    StreamError(#[from] SendError),
    #[error("Stream cancelled: {0}")]
    StreamCancelled(#[from] Canceled),
    #[error("Auth error: {0}")]
    AuthError(#[from] AuthError),
}
