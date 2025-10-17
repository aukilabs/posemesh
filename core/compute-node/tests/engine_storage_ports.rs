use chrono::Utc;
use compute_runner_api::{LeaseEnvelope, TaskSpec};
use posemesh_compute_node::engine::build_storage_for_lease;
use serde_json::json;
use uuid::Uuid;

const MOCK_CAPABILITY: &str = "/posemesh/mock/v1";

fn assert_send_sync<T: ?Sized + Send + Sync>(_t: &T) {}

#[test]
fn engine_builds_storage_ports_from_lease() {
    let lease = LeaseEnvelope {
        access_token: Some("tok123".into()),
        access_token_expires_at: Some(Utc::now()),
        lease_expires_at: Some(Utc::now()),
        cancel: false,
        status: Some("leased".into()),
        domain_id: Some(Uuid::new_v4()),
        domain_server_url: Some("https://domain.example".parse().unwrap()),
        task: TaskSpec {
            id: Uuid::new_v4(),
            job_id: Some(Uuid::new_v4()),
            capability: MOCK_CAPABILITY.into(),
            capability_filters: json!({}),
            inputs_cids: vec![],
            outputs_prefix: Some("out".into()),
            label: None,
            stage: None,
            meta: json!({}),
            priority: None,
            attempts: None,
            max_attempts: None,
            deps_remaining: None,
            status: Some("leased".into()),
            mode: None,
            organization_filter: None,
            billing_units: None,
            estimated_credit_cost: None,
            debited_amount: None,
            debited_at: None,
            lease_expires_at: None,
        },
    };

    let ports = build_storage_for_lease(&lease).expect("ports");
    assert_send_sync(&*ports.input);
    assert_send_sync(&*ports.output);
}
