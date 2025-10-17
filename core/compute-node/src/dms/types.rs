use chrono::{DateTime, Utc};
use compute_runner_api::TaskSpec;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

/// Alias for the lease envelope as returned by DMS.
pub type LeaseResponse = compute_runner_api::LeaseEnvelope;
/// Heartbeat responses provide incremental lease updates.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HeartbeatResponse {
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub access_token_expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub lease_expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub cancel: Option<bool>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub domain_id: Option<Uuid>,
    #[serde(default)]
    pub domain_server_url: Option<Url>,
    #[serde(default)]
    pub task: Option<TaskSpec>,
    #[serde(default)]
    pub task_id: Option<Uuid>,
    #[serde(default)]
    pub job_id: Option<Uuid>,
    #[serde(default)]
    pub attempts: Option<u64>,
    #[serde(default)]
    pub max_attempts: Option<u64>,
    #[serde(default)]
    pub deps_remaining: Option<u64>,
}

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
