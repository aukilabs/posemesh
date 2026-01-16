use axum::Router;

/// Build the node HTTP router by delegating to the shared
/// `posemesh-node-registration` router. This is kept for legacy DDS callbacks
/// but is no longer required for URL-less compute node registration.
pub fn router() -> Router {
    posemesh_node_registration::http::router_dds(posemesh_node_registration::http::DdsState)
}
