mod support;

use async_trait::async_trait;
use httpmock::prelude::*;
use posemesh_compute_node::auth::token_manager::{TokenProvider, TokenProviderResult};
use posemesh_compute_node::config::{LogFormat, NodeConfig};
use posemesh_compute_node::dms::client::DmsClient;
use posemesh_compute_node::engine::{run_cycle_with_dms, run_node_with_shutdown, RunnerRegistry};
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

fn base_cfg() -> NodeConfig {
    NodeConfig {
        dms_base_url: "https://dms.example".parse().unwrap(),
        node_version: "1.0.0".into(),
        request_timeout_secs: 10,
        dds_base_url: None,
        node_url: None,
        reg_secret: None,
        secp256k1_privhex: None,
        heartbeat_jitter_ms: 250,
        poll_backoff_ms_min: 1000,
        poll_backoff_ms_max: 30000,
        token_safety_ratio: 0.75,
        token_reauth_max_retries: 3,
        token_reauth_jitter_ms: 500,
        register_interval_secs: None,
        register_max_retry: None,
        max_concurrency: 1,
        log_format: LogFormat::Json,
        enable_noop: true,
        noop_sleep_secs: 1,
    }
}

#[derive(Clone)]
struct StaticProvider {
    token: String,
}

#[async_trait]
impl TokenProvider for StaticProvider {
    async fn bearer(&self) -> TokenProviderResult<String> {
        Ok(self.token.clone())
    }

    async fn on_unauthorized(&self) {}
}

#[tokio::test]
async fn happy_path_poll_run_complete_with_heartbeat_token_rotation() {
    let server = MockServer::start();
    let node_token = "node-abc";

    let reg = support::mock_runner::registry_with_mock();
    let capabilities = reg.capabilities();
    let cap = capabilities.first().cloned().expect("capability present");
    let base_url = server.base_url().to_string();

    let task_id = Uuid::new_v4();
    let job_id = Uuid::new_v4();
    let domain_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    // Lease: return token A and domain url pointing to same mock server
    let lease_body = json!({
        "access_token": "t-A",
        "access_token_expires_at": now,
        "lease_expires_at": now,
        "cancel": false,
        "status": "leased",
                "domain_id": domain_id,
                "domain_server_url": base_url.clone(),
                "task": {
                    "id": task_id,
                    "job_id": job_id,
                    "capability": cap.clone(),
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
    let lease_mock = server.mock(move |when, then| {
        when.method(GET)
            .path("/tasks")
            .header("authorization", format!("Bearer {}", node_token));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(lease_body.clone());
    });

    // Heartbeat rotates token to B
    let hb_base_url = base_url.clone();
    let hb_mock = server.mock(move |when, then| {
        when.method(POST)
            .path(format!("/tasks/{}/heartbeat", task_id))
            .header("authorization", format!("Bearer {}", node_token))
            .header("content-type", "application/json");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "access_token": "t-B",
                "access_token_expires_at": now,
                "lease_expires_at": now + chrono::Duration::seconds(30),
                "cancel": false,
                "status": "leased",
                "domain_id": domain_id,
                "domain_server_url": hb_base_url.clone(),
                "task_id": task_id,
                "job_id": job_id,
                "attempts": 1,
                "max_attempts": 5,
                "deps_remaining": 0
            }));
    });

    // Domain uploads should use new token B
    let upload_path = format!("/api/v1/domains/{}/data", domain_id);
    let upload_mock = server.mock({
        let upload_path = upload_path.clone();
        move |when, then| {
            when.method(POST)
                .path(upload_path.as_str())
                .header("authorization", "Bearer t-B");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"data":[{"id":"artifact-id","domain_id":"dom","name":"n","data_type":"d","size":1,"created_at":"2025-01-01T00:00:00Z","updated_at":"2025-01-01T00:00:00Z"}]}"#);
        }
    });

    // Complete
    let complete_cap = cap.clone();
    let complete_mock = server.mock(move |when, then| {
        when.method(POST)
            .path(format!("/tasks/{}/complete", task_id))
            .header("authorization", format!("Bearer {}", node_token))
            .header("content-type", "application/json")
            .body_contains("\"artifact-id\"")
            .body_contains(format!("\"job_id\":\"{}\"", job_id))
            .body_contains(format!("\"capability\":\"{}\"", complete_cap));
        then.status(200);
    });

    let cfg = base_cfg();
    let base: url::Url = server.base_url().parse().unwrap();
    let provider = Arc::new(StaticProvider {
        token: node_token.into(),
    });
    let dms = DmsClient::new(base, Duration::from_secs(5), provider).unwrap();
    let processed = run_cycle_with_dms(&cfg, &dms, &reg).await.unwrap();
    assert!(processed, "expected lease to be processed");

    lease_mock.assert();
    assert!(hb_mock.hits() >= 1, "expected at least one heartbeat");
    let start_upload = Instant::now();
    while upload_mock.hits() < 1 && start_upload.elapsed() < Duration::from_secs(5) {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let upload_hits = upload_mock.hits();
    if upload_hits < 1 {
        panic!(
            "expected at least one domain upload for runner artifacts, got {}",
            upload_hits
        );
    }
    complete_mock.assert();
}

struct ErrRunner;
#[async_trait::async_trait]
impl compute_runner_api::Runner for ErrRunner {
    fn capability(&self) -> &'static str {
        "/err"
    }
    async fn run(&self, _ctx: compute_runner_api::TaskCtx<'_>) -> anyhow::Result<()> {
        anyhow::bail!("boom")
    }
}

#[tokio::test]
async fn error_path_calls_fail() {
    let server = MockServer::start();
    let node_token = "node-abc";
    let task_id = Uuid::new_v4();
    let job_id = Uuid::new_v4();
    let domain_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let reg = RunnerRegistry::new().register(ErrRunner);
    let capabilities = reg.capabilities();
    let err_cap = capabilities.first().cloned().expect("capability present");
    let base_url = server.base_url().to_string();

    let lease_body = json!({
        "access_token": "t-A",
        "access_token_expires_at": now,
        "lease_expires_at": now,
        "cancel": false,
        "status": "leased",
        "domain_id": domain_id,
        "domain_server_url": base_url.clone(),
        "task": {
            "id": task_id,
            "job_id": job_id,
            "capability": err_cap.clone(),
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
    let lease_mock = server.mock(move |when, then| {
        when.method(GET)
            .path("/tasks")
            .header("authorization", format!("Bearer {}", node_token));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(lease_body.clone());
    });

    let hb_base_url = base_url.clone();
    let hb_mock = server.mock(move |when, then| {
        when.method(POST)
            .path(format!("/tasks/{}/heartbeat", task_id))
            .header("authorization", format!("Bearer {}", node_token))
            .header("content-type", "application/json");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "access_token": "t-A",
                "access_token_expires_at": now,
                "lease_expires_at": now + chrono::Duration::seconds(30),
                "cancel": false,
                "status": "leased",
                "domain_id": domain_id,
                "domain_server_url": hb_base_url.clone(),
                "task_id": task_id,
                "job_id": job_id,
                "attempts": 1,
                "max_attempts": 5,
                "deps_remaining": 0
            }));
    });

    let fail_mock = server.mock(move |when, then| {
        when.method(POST)
            .path(format!("/tasks/{}/fail", task_id))
            .header("authorization", format!("Bearer {}", node_token))
            .header("content-type", "application/json")
            .body_contains("\"job\"")
            .body_contains("\"artifacts\"");
        then.status(200);
    });

    let cfg = base_cfg();
    let base: url::Url = server.base_url().parse().unwrap();
    let provider = Arc::new(StaticProvider {
        token: node_token.into(),
    });
    let dms = DmsClient::new(base, Duration::from_secs(5), provider).unwrap();
    let processed = run_cycle_with_dms(&cfg, &dms, &reg).await.unwrap();
    assert!(
        processed,
        "expected lease to be processed even on failure path"
    );

    lease_mock.assert();
    assert!(hb_mock.hits() >= 1, "expected at least one heartbeat");
    fail_mock.assert();
}

#[tokio::test]
async fn run_node_uses_siwe_token_and_completes_task() {
    let server = MockServer::start();
    posemesh_compute_node::dds::persist::clear_node_secret().unwrap();
    posemesh_compute_node::dds::persist::write_node_secret("node-secret").unwrap();

    let task_id = Uuid::new_v4();
    let job_id = Uuid::new_v4();
    let domain_id = Uuid::new_v4();
    let issued_at = chrono::Utc::now();
    let lease_now = chrono::Utc::now();
    let lease_now_iso = lease_now.to_rfc3339();
    let siwe_expiry = issued_at + chrono::Duration::hours(1);
    let siwe_token = "siwe-access-token";

    let request_mock = server.mock({
        let issued_at = issued_at.to_rfc3339();
        move |when, then| {
            when.method(POST).path("/internal/v1/auth/siwe/request");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "nonce": "nonce-123",
                    "domain": "d.example",
                    "uri": "https://d.example/login",
                    "version": "1",
                    "chainId": 1,
                    "issuedAt": issued_at,
                }));
        }
    });

    let verify_mock = server.mock({
        let token = siwe_token.to_string();
        let expiry = siwe_expiry.to_rfc3339();
        move |when, then| {
            when.method(POST).path("/internal/v1/auth/siwe/verify");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "access_token": token,
                    "access_expires_at": expiry,
                }));
        }
    });

    let mut runners = RunnerRegistry::new();
    for runner in support::mock_runner::runners_for_all_capabilities() {
        runners = runners.register(runner);
    }
    let capabilities = runners.capabilities();
    let cap = capabilities.first().cloned().expect("capability present");
    let base_url = server.base_url().to_string();

    let lease_mock = server.mock({
        let cap = cap.clone();
        let siwe_token = siwe_token.to_string();
        let base_url = base_url.clone();
        let lease_now = lease_now_iso.clone();
        move |when, then| {
            when.method(GET)
                .path("/tasks")
                .header("authorization", format!("Bearer {}", siwe_token));
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "access_token": "session-A",
                    "access_token_expires_at": lease_now,
                    "lease_expires_at": lease_now,
                    "cancel": false,
                    "status": "leased",
                    "domain_id": domain_id,
                    "domain_server_url": base_url.clone(),
                    "task": {
                        "id": task_id,
                        "job_id": job_id,
                    "capability": cap.clone(),
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
                }));
        }
    });

    let heartbeat_mock = server.mock({
        let siwe_token = siwe_token.to_string();
        let lease_now = lease_now_iso.clone();
        let base_url = base_url.clone();
        move |when, then| {
            when.method(POST)
                .path(format!("/tasks/{}/heartbeat", task_id))
                .header("authorization", format!("Bearer {}", siwe_token))
                .header("content-type", "application/json");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "access_token": "session-B",
                    "access_token_expires_at": lease_now,
                    "lease_expires_at": lease_now,
                    "cancel": false,
                    "status": "leased",
                    "domain_id": domain_id,
                    "domain_server_url": base_url.clone(),
                    "task_id": task_id,
                    "job_id": job_id,
                    "attempts": 2,
                    "max_attempts": 5,
                    "deps_remaining": 0
                }));
        }
    });

    let completion_counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let upload_path = format!("/api/v1/domains/{}/data", domain_id);
    let upload_mock = server.mock({
        let upload_path = upload_path.clone();
        move |when, then| {
            when.method(POST)
                .path(upload_path.as_str())
                .header("authorization", "Bearer session-B");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"data":[{"id":"artifact-id","domain_id":"dom","name":"n","data_type":"d","size":1,"created_at":"2025-01-01T00:00:00Z","updated_at":"2025-01-01T00:00:00Z"}]}"#);
        }
    });

    let _complete_mock = server.mock({
        let siwe_token = siwe_token.to_string();
        let counter = completion_counter.clone();
        let siwe_token = siwe_token.to_string();
        move |when, then| {
            when.method(POST)
                .path(format!("/tasks/{}/complete", task_id))
                .header("authorization", format!("Bearer {}", siwe_token))
                .header("content-type", "application/json");
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            then.status(200);
        }
    });

    let cfg = NodeConfig {
        dms_base_url: server.base_url().parse().unwrap(),
        node_version: "1.0.0".into(),
        request_timeout_secs: 5,
        dds_base_url: Some(server.base_url().parse().unwrap()),
        node_url: Some(server.base_url().parse().unwrap()),
        reg_secret: Some("reg-secret".into()),
        secp256k1_privhex: Some(
            "4c0883a69102937d6231471b5dbb6204fe5129617082798ce3f4fdf2548b6f90".into(),
        ),
        heartbeat_jitter_ms: 250,
        poll_backoff_ms_min: 1000,
        poll_backoff_ms_max: 30000,
        token_safety_ratio: 0.75,
        token_reauth_max_retries: 3,
        token_reauth_jitter_ms: 500,
        register_interval_secs: None,
        register_max_retry: None,
        max_concurrency: 1,
        log_format: LogFormat::Json,
        enable_noop: true,
        noop_sleep_secs: 0,
    };

    let shutdown = CancellationToken::new();
    let run_task = tokio::spawn(run_node_with_shutdown(
        cfg.clone(),
        runners,
        shutdown.clone(),
    ));

    // Allow the node to process at least one lease.
    let start = Instant::now();
    while lease_mock.hits() == 0 && start.elapsed() < Duration::from_millis(500) {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert!(
        request_mock.hits() >= 1,
        "SIWE request should be invoked at least once"
    );
    assert!(
        verify_mock.hits() >= 1,
        "SIWE verify should be invoked at least once"
    );
    assert!(
        lease_mock.hits() >= 1,
        "Lease endpoint should be hit at least once"
    );
    assert!(
        heartbeat_mock.hits() >= 1,
        "Heartbeat endpoint should be hit at least once"
    );
    let start_upload = Instant::now();
    while upload_mock.hits() < 5 && start_upload.elapsed() < Duration::from_secs(5) {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let upload_hits = upload_mock.hits();
    if upload_hits < 5 {
        panic!(
            "expected at least five domain uploads for runner artifacts, got {}",
            upload_hits
        );
    }
    assert!(
        completion_counter.load(std::sync::atomic::Ordering::SeqCst) >= 1,
        "Completion endpoint should be hit at least once"
    );

    shutdown.cancel();
    run_task
        .await
        .expect("task join")
        .expect("run_node_with_shutdown should exit cleanly after cancellation");

    posemesh_compute_node::dds::persist::clear_node_secret().unwrap();
}
