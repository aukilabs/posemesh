use httpmock::prelude::*;
use posemesh_compute_node::dms::{
    client::DmsClient,
    types::{CompleteTaskRequest, FailTaskRequest},
};
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

const MOCK_CAPABILITY: &str = "/posemesh/mock/v1";

#[tokio::test]
async fn lease_by_capability_and_complete_fail() {
    let server = MockServer::start();
    let cap = MOCK_CAPABILITY;
    let node_token = "node-abc";

    // Build a minimal, but complete LeaseResponse body
    let task_id = Uuid::new_v4();
    let job_id = Uuid::new_v4();
    let domain_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let lease_body = json!({
        "access_token": "t-domain",
        "access_token_expires_at": now,
        "lease_expires_at": now,
        "cancel": false,
        "status": "leased",
        "domain_id": domain_id,
        "domain_server_url": server.base_url(),
        "task": {
            "id": task_id,
            "job_id": job_id,
            "capability": cap,
            "capability_filters": {},
            "inputs_cids": [],
            "outputs_prefix": "out",
            "label": null,
            "stage": null,
            "meta": {},
            "priority": null,
            "attempts": null,
            "max_attempts": null,
            "deps_remaining": null,
            "status": "leased",
            "mode": null,
            "organization_filter": null,
            "billing_units": null,
            "estimated_credit_cost": null,
            "debited_amount": null,
            "debited_at": null,
            "lease_expires_at": null
        }
    });

    let lease_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/tasks")
            .header("authorization", format!("Bearer {}", node_token));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(lease_body.clone());
    });

    let complete_mock = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/tasks/{}/complete", task_id))
            .header("authorization", format!("Bearer {}", node_token))
            .header("content-type", "application/json");
        then.status(200);
    });

    let fail_mock = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/tasks/{}/fail", task_id))
            .header("authorization", format!("Bearer {}", node_token))
            .header("content-type", "application/json")
            .json_body(json!({
                "reason": "e",
                "details": {}
            }));
        then.status(200);
    });

    let base: url::Url = server.base_url().parse().unwrap();
    let client = DmsClient::new(base, Duration::from_secs(10), Some(node_token.into())).unwrap();

    // Lease by capability
    let lease = client
        .lease_by_capability(cap)
        .await
        .unwrap()
        .expect("lease");
    lease_mock.assert();
    assert_eq!(lease.task.capability, cap);
    assert_eq!(lease.task.id, task_id);

    // Complete
    client
        .complete(
            task_id,
            &CompleteTaskRequest {
                outputs_index: json!({}),
                result: json!({}),
            },
        )
        .await
        .unwrap();
    complete_mock.assert();

    // Fail
    client
        .fail(
            task_id,
            &FailTaskRequest {
                reason: "e".into(),
                details: json!({}),
            },
        )
        .await
        .unwrap();
    fail_mock.assert();
}
