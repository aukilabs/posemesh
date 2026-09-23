use axum::{http::StatusCode, routing::get, Router};

/// Build the host health router. SDK-managed workers register outbound with DDS;
/// the obsolete registration callback and global credential store are removed.
pub fn router() -> Router {
    Router::new().route("/health", get(|| async { StatusCode::OK }))
}
