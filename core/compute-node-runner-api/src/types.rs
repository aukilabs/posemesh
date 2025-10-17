use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;
use uuid::Uuid;

/// Lease envelope received from DMS (see SPECS §3.1).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LeaseEnvelope {
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub access_token_expires_at: Option<DateTime<Utc>>,

    #[serde(default)]
    pub lease_expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub cancel: bool,
    #[serde(default)]
    pub status: Option<String>,

    #[serde(default)]
    pub domain_id: Option<Uuid>,
    #[serde(default)]
    pub domain_server_url: Option<Url>,

    pub task: TaskSpec,
}

/// Task specification (see SPECS §3.2).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TaskSpec {
    pub id: Uuid,
    #[serde(default)]
    pub job_id: Option<Uuid>,

    pub capability: String,
    #[serde(default)]
    pub capability_filters: Value,

    #[serde(default)]
    pub inputs_cids: Vec<String>,
    #[serde(default)]
    pub outputs_prefix: Option<String>,

    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub stage: Option<String>,
    #[serde(default)]
    pub meta: Value,

    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default)]
    pub attempts: Option<u64>,
    #[serde(default)]
    pub max_attempts: Option<u64>,
    #[serde(default)]
    pub deps_remaining: Option<u64>,

    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub organization_filter: Option<String>,

    #[serde(default)]
    pub billing_units: Option<String>,
    #[serde(default)]
    pub estimated_credit_cost: Option<String>,
    #[serde(default)]
    pub debited_amount: Option<String>,
    #[serde(default)]
    pub debited_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub lease_expires_at: Option<DateTime<Utc>>,
}
