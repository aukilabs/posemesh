use axum::Router;

/// Build the node HTTP router by delegating to the shared
/// `posemesh-node-registration` router. This ensures that the registration
/// callback persists the node secret and that DDS health probes update the
/// registration health state in a single, canonical store used by the
/// outbound registration loop.
pub fn router() -> Router {
    posemesh_node_registration::http::router_dds(posemesh_node_registration::http::DdsState)
}
