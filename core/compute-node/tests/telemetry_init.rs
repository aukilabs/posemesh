use posemesh_compute_node::config::LogFormat;

#[test]
fn telemetry_initializes() {
    posemesh_compute_node::telemetry::init_with_format(LogFormat::Text).unwrap();
    tracing::info!(message = "telemetry initialized");
}

#[cfg(feature = "metrics")]
#[test]
fn metrics_feature_compiles() {
    // Ensure the metrics module exists and constants are accessible.
    let _ = posemesh_compute_node::telemetry::metrics::DMS_POLL_LATENCY_MS;
    posemesh_compute_node::telemetry::metrics::incr(
        posemesh_compute_node::telemetry::metrics::TOKEN_ROTATE_COUNT,
        1,
    );
}
