use thiserror::Error;

/// Errors during task execution orchestration.
#[derive(Debug, Error)]
pub enum ExecutorError {
    #[error("no runner registered for capability: {0}")]
    NoRunner(String),
    #[error("runner failed: {0}")]
    Runner(String),
}

/// Errors from Domain storage requests.
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
