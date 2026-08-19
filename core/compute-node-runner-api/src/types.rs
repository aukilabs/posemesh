use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use url::Url;
use uuid::Uuid;

pub const P2P_DATASET_SCHEMA: &str = "auki-p2p-dataset/v0";

/// Lease envelope received from DMS (see SPECS §3.1).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LeaseEnvelope {
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub access_token_expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    #[doc(hidden)]
    pub p2p_access_token: Option<String>,
    #[serde(default)]
    #[doc(hidden)]
    pub p2p_access_token_expires_at: Option<DateTime<Utc>>,

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

impl LeaseEnvelope {
    /// Clone the runner-visible lease without internal peer credentials.
    pub fn without_p2p_credentials(&self) -> Self {
        let mut lease = self.clone();
        lease.p2p_access_token = None;
        lease.p2p_access_token_expires_at = None;
        lease
    }
}

/// Local ZIP registration accepted by the process-level P2P dataset service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct P2pDatasetRegistration {
    pub dataset_id: String,
    pub domain_id: Uuid,
    pub name: String,
    pub path: PathBuf,
    pub available_until: DateTime<Utc>,
}

/// Routing and integrity metadata persisted as the small Domain Data reference.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct P2pDatasetReference {
    pub schema: String,
    pub dataset_id: String,
    pub domain_id: Uuid,
    pub name: String,
    pub peer_id: String,
    pub multiaddrs: Vec<String>,
    pub size_bytes: u64,
    pub sha256: String,
    pub available_until: DateTime<Utc>,
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
