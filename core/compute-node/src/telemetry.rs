use crate::config::LogFormat;
use std::sync::Once;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

static INIT: Once = Once::new();

/// Initialize global tracing subscriber with the given log format.
/// Safe to call multiple times; only the first call installs a subscriber.
pub fn init_with_format(fmt_mode: LogFormat) -> anyhow::Result<()> {
    INIT.call_once(|| {
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        let fmt_layer = match fmt_mode {
            LogFormat::Json => fmt::layer().json().boxed(),
            LogFormat::Text => fmt::layer().boxed(),
        };

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    });
    Ok(())
}

/// Initialize tracing using `LOG_FORMAT` env var ("json" or "text", default json).
pub fn init_from_env() -> anyhow::Result<()> {
    let mode = match std::env::var("LOG_FORMAT").ok().as_deref() {
        Some("text") => LogFormat::Text,
        _ => LogFormat::Json,
    };
    init_with_format(mode)
}

/// Create a span for a task with common fields as per the spec.
pub fn task_span(
    task_id: uuid::Uuid,
    job_id: uuid::Uuid,
    capability: &str,
    domain_id: uuid::Uuid,
) -> tracing::Span {
    tracing::info_span!(
        "task",
        task_id = %task_id,
        job_id = %job_id,
        capability = %capability,
        domain_id = %domain_id
    )
}

#[cfg(feature = "metrics")]
pub mod metrics {
    /// Metric names as per §10 Telemetry.
    pub const DMS_POLL_LATENCY_MS: &str = "dms.poll.latency_ms";
    pub const DMS_ACTIVE_TASK: &str = "dms.active_task";
    pub const RUNNER_RUN_LATENCY_MS: &str = "runner.run.latency_ms";
    pub const TOKEN_ROTATE_COUNT: &str = "token.rotate.count";
    pub const STORAGE_BYTES_UPLOADED: &str = "storage.bytes.uploaded";
    pub const STORAGE_BYTES_DOWNLOADED: &str = "storage.bytes.downloaded";

    /// Increment a counter (no-op placeholder; exporter not wired in this refactor).
    pub fn incr(_name: &str, _by: u64) {}

    /// Record a gauge value (no-op placeholder).
    pub fn gauge(_name: &str, _value: u64) {}
}
