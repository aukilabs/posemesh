use serde::{Deserialize, Serialize};

/// Alias for the lease envelope as returned by DMS.
pub type LeaseResponse = compute_runner_api::LeaseEnvelope;
/// Heartbeat responses reuse the same payload as lease envelopes.
pub type HeartbeatResponse = compute_runner_api::LeaseEnvelope;

/// Heartbeat request payload (progress and optional events).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HeartbeatRequest {
    pub progress: serde_json::Value,
    #[serde(default)]
    pub events: serde_json::Value,
}

/// Complete task request payload (shape is intentionally minimal here).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompleteTaskRequest {
    pub outputs_index: serde_json::Value,
    pub result: serde_json::Value,
}

/// Fail task request payload.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FailTaskRequest {
    pub reason: String,
    #[serde(default)]
    pub details: serde_json::Value,
}
