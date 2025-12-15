use thiserror::Error;

/// Errors originating from DMS client operations.
#[derive(Debug, Error)]
pub enum DmsClientError {
    #[error("unauthorized (401)")]
    Unauthorized,
    #[error("request timed out")]
    Timeout,
    #[error("http error: {0}")]
    Http(String),
    #[error("transport error: {0}")]
    Transport(String),
}

/// Errors during task execution orchestration.
#[derive(Debug, Error)]
pub enum ExecutorError {
    #[error("no runner registered for capability: {0}")]
    NoRunner(String),
    #[error("runner failed: {0}")]
    Runner(String),
}

/// Errors in token management / rotation.
#[derive(Debug, Error)]
pub enum TokenManagerError {
    #[error("token rotation failed: {0}")]
    Rotation(String),
}

/// Storage client error mapping (see SPECS §9 Errors).
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("bad request (400)")]
    BadRequest,
    #[error("unauthorized (401)")]
    Unauthorized,
    #[error("not found (404)")]
    NotFound,
    #[error("conflict (409)")]
    Conflict,
    #[error("server error ({0})")]
    Server(u16),
    #[error("network error: {0}")]
    Network(String),
    #[error("other storage error: {0}")]
    Other(String),
}
