use chrono::Utc;
use compute_runner_api::{LeaseEnvelope, TaskSpec};
use posemesh_compute_node::storage::{build_ports, TokenRef};
use serde_json::json;
use uuid::Uuid;

const MOCK_CAPABILITY: &str = "/posemesh/mock/v1";

fn assert_send_sync<T: ?Sized + Send + Sync>(_t: &T) {}

#[test]
fn token_ref_swaps() {
    let t = TokenRef::new("a".into());
    assert_eq!(t.get(), "a");
    t.swap("b".into());
    assert_eq!(t.get(), "b");
}
#[test]
fn build_ports_returns_trait_objects() {
    let lease = LeaseEnvelope {
        access_token: Some("tkn".into()),
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
    let token = TokenRef::new(lease.access_token.clone().unwrap());
    let ports = build_ports(&lease, token).expect("ports");

    assert_send_sync(&*ports.input);
    assert_send_sync(&*ports.output);
}
