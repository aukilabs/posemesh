use httpmock::prelude::*;
use posemesh_compute_node::dms::{client::DmsClient, types::HeartbeatRequest};
use posemesh_compute_node::engine::apply_heartbeat_token_update;
use posemesh_compute_node::storage::TokenRef;
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn heartbeat_returns_new_token_and_engine_applies_swap() {
    let server = MockServer::start();
    let node_token = "node-xyz";
    let task_id = Uuid::new_v4();
    let new_token = "t-new";

    let hb_mock = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/tasks/{}/heartbeat", task_id))
            .header("authorization", format!("Bearer {}", node_token))
            .header("content-type", "application/json");
        let now = chrono::Utc::now();
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "access_token": new_token,
                "access_token_expires_at": now,
                "lease_expires_at": now + chrono::Duration::seconds(30),
                "cancel": false,
                "status": "running",
                "domain_server_url": "https://example.com",
                "task_id": task_id,
                "job_id": null,
                "attempts": 1
            }));
    });

    let base: url::Url = server.base_url().parse().unwrap();
    let client = DmsClient::new(base, Duration::from_secs(5), Some(node_token.into())).unwrap();

    let token_ref = TokenRef::new("t-old".into());
    assert_eq!(token_ref.get(), "t-old");

    let hb = client
        .heartbeat(
            task_id,
            &HeartbeatRequest {
                progress: json!({"p":1}),
                events: json!({}),
            },
        )
        .await
        .unwrap();
    hb_mock.assert();

    // Engine applies token update
    apply_heartbeat_token_update(&token_ref, &hb);
    assert_eq!(token_ref.get(), new_token);
}
