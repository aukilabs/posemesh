mod support;

use async_trait::async_trait;
use httpmock::prelude::*;
use posemesh_compute_node::auth::token_manager::{TokenProvider, TokenProviderResult};
use posemesh_compute_node::config::{LogFormat, NodeConfig, RobotNodeConfig};
use posemesh_compute_node::dms::client::DmsClient;
use posemesh_compute_node::engine::{
    run_cycle_with_dms, run_node_with_shutdown, run_robot_node_with_shutdown, RunnerRegistry,
};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

fn base_cfg() -> NodeConfig {
    NodeConfig {
        dms_base_url: "https://dms.example".parse().unwrap(),
        node_version: "1.0.0".into(),
        request_timeout_secs: 10,
        dds_base_url: None,
        reg_secret: None,
        secp256k1_privhex: None,
        heartbeat_jitter_ms: 250,
        heartbeat_min_ratio: 0.25,
        heartbeat_max_ratio: 0.35,
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

fn robot_cfg(server: &MockServer) -> RobotNodeConfig {
    let base_url: url::Url = server.base_url().parse().unwrap();
    let mut cfg = RobotNodeConfig::new(base_url.clone(), base_url, "robot-test-credentials")
        .expect("robot test configuration");
    cfg.node_version = "robot-test-version".to_string();
    cfg.request_timeout_secs = 2;
    cfg.heartbeat_jitter_ms = 0;
    cfg.poll_backoff_ms_min = 1000;
    cfg.poll_backoff_ms_max = 1000;
    cfg.token_reauth_max_retries = 0;
    cfg.token_reauth_jitter_ms = 0;
    cfg.noop_sleep_secs = 0;
    cfg
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
        reg_secret: Some("reg-secret".into()),
        secp256k1_privhex: Some(
            "4c0883a69102937d6231471b5dbb6204fe5129617082798ce3f4fdf2548b6f90".into(),
        ),
        heartbeat_jitter_ms: 250,
        heartbeat_min_ratio: 0.25,
        heartbeat_max_ratio: 0.35,
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

    // Allow the node to acquire the lease and enter the heartbeat-backed
    // execution path. A lease hit alone does not mean the async heartbeat
    // request has reached the mock server yet.
    let start = Instant::now();
    while (lease_mock.hits() == 0 || heartbeat_mock.hits() == 0)
        && start.elapsed() < Duration::from_secs(2)
    {
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

#[tokio::test]
async fn run_robot_node_refreshes_after_dms_401_and_retries_with_machine_token() {
    let server = MockServer::start();
    let robot_id = Uuid::new_v4();
    let expires_at = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();

    let register_mock = server.mock({
        let expires_at = expires_at.clone();
        move |when, then| {
            when.method(POST)
                .path("/internal/v1/robots/register")
                .body_contains("\"registration_credentials\":\"robot-test-credentials\"")
                .body_contains("\"version\":\"robot-test-version\"")
                .body_contains(support::mock_runner::MOCK_CAPABILITY);
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "robot_id": robot_id,
                    "access_token": "robot-token-a",
                    "access_expires_at": expires_at,
                }));
        }
    });
    let verify_mock = server.mock({
        let expires_at = expires_at.clone();
        move |when, then| {
            when.method(POST)
                .path("/internal/v1/auth/robot/verify")
                .body_contains("\"registration_credentials\":\"robot-test-credentials\"");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "robot_id": robot_id,
                    "access_token": "robot-token-b",
                    "access_expires_at": expires_at,
                }));
        }
    });
    let stale_lease_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/tasks")
            .header("authorization", "Bearer robot-token-a");
        then.status(401);
    });
    let refreshed_lease_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/tasks")
            .header("authorization", "Bearer robot-token-b");
        then.status(204);
    });
    let siwe_request_mock = server.mock(|when, then| {
        when.method(POST).path("/internal/v1/auth/siwe/request");
        then.status(500);
    });
    let siwe_verify_mock = server.mock(|when, then| {
        when.method(POST).path("/internal/v1/auth/siwe/verify");
        then.status(500);
    });

    let cfg = robot_cfg(&server);
    let shutdown = CancellationToken::new();
    let run_task = tokio::spawn(run_robot_node_with_shutdown(
        cfg,
        support::mock_runner::registry_with_mock(),
        shutdown.clone(),
    ));

    let started = Instant::now();
    while refreshed_lease_mock.hits() == 0 && started.elapsed() < Duration::from_secs(3) {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    shutdown.cancel();
    tokio::time::timeout(Duration::from_secs(2), run_task)
        .await
        .expect("robot engine should stop promptly")
        .expect("robot engine task join")
        .expect("robot engine should shut down cleanly");

    register_mock.assert_hits(1);
    stale_lease_mock.assert_hits(1);
    verify_mock.assert_hits(1);
    refreshed_lease_mock.assert_hits(1);
    siwe_request_mock.assert_hits(0);
    siwe_verify_mock.assert_hits(0);
}

#[tokio::test]
async fn run_robot_node_does_not_fall_back_to_siwe_when_verify_fails() {
    let server = MockServer::start();
    let robot_id = Uuid::new_v4();
    let expires_at = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();

    let register_mock = server.mock(move |when, then| {
        when.method(POST).path("/internal/v1/robots/register");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "robot_id": robot_id,
                "access_token": "robot-token-a",
                "access_expires_at": expires_at,
            }));
    });
    let verify_mock = server.mock(|when, then| {
        when.method(POST).path("/internal/v1/auth/robot/verify");
        then.status(403);
    });
    let stale_lease_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/tasks")
            .header("authorization", "Bearer robot-token-a");
        then.status(401);
    });
    let siwe_request_mock = server.mock(|when, then| {
        when.method(POST).path("/internal/v1/auth/siwe/request");
        then.status(200);
    });
    let siwe_verify_mock = server.mock(|when, then| {
        when.method(POST).path("/internal/v1/auth/siwe/verify");
        then.status(200);
    });

    let cfg = robot_cfg(&server);
    let shutdown = CancellationToken::new();
    let run_task = tokio::spawn(run_robot_node_with_shutdown(
        cfg,
        support::mock_runner::registry_with_mock(),
        shutdown.clone(),
    ));

    let started = Instant::now();
    while verify_mock.hits() == 0 && started.elapsed() < Duration::from_secs(3) {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    shutdown.cancel();
    tokio::time::timeout(Duration::from_secs(2), run_task)
        .await
        .expect("robot engine should stop promptly after failed verification")
        .expect("robot engine task join")
        .expect("robot engine should shut down cleanly");

    register_mock.assert_hits(1);
    stale_lease_mock.assert_hits(1);
    assert!(verify_mock.hits() >= 1, "robot verify should be attempted");
    siwe_request_mock.assert_hits(0);
    siwe_verify_mock.assert_hits(0);
}

#[tokio::test]
async fn run_robot_node_cancels_while_initial_dds_authentication_hangs() {
    let server = MockServer::start();
    let register_mock = server.mock(|when, then| {
        when.method(POST).path("/internal/v1/robots/register");
        then.delay(Duration::from_secs(5)).status(503);
    });

    let cfg = robot_cfg(&server);
    let shutdown = CancellationToken::new();
    let run_task = tokio::spawn(run_robot_node_with_shutdown(
        cfg,
        support::mock_runner::registry_with_mock(),
        shutdown.clone(),
    ));

    let started = Instant::now();
    while register_mock.hits() == 0 && started.elapsed() < Duration::from_secs(2) {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(register_mock.hits(), 1, "robot registration should start");

    shutdown.cancel();
    tokio::time::timeout(Duration::from_millis(500), run_task)
        .await
        .expect("hanging DDS authentication must not delay shutdown")
        .expect("robot engine task join")
        .expect("robot engine should shut down cleanly");
}

#[tokio::test]
async fn run_robot_node_cancels_while_forced_dds_refresh_hangs() {
    let server = MockServer::start();
    let robot_id = Uuid::new_v4();
    let expires_at = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();

    let register_mock = server.mock(move |when, then| {
        when.method(POST).path("/internal/v1/robots/register");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "robot_id": robot_id,
                "access_token": "robot-token-before-refresh",
                "access_expires_at": expires_at,
            }));
    });
    let lease_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/tasks")
            .header("authorization", "Bearer robot-token-before-refresh");
        then.status(401);
    });
    let verify_mock = server.mock(|when, then| {
        when.method(POST).path("/internal/v1/auth/robot/verify");
        then.delay(Duration::from_secs(5)).status(503);
    });

    let cfg = robot_cfg(&server);
    let shutdown = CancellationToken::new();
    let run_task = tokio::spawn(run_robot_node_with_shutdown(
        cfg,
        support::mock_runner::registry_with_mock(),
        shutdown.clone(),
    ));

    let started = Instant::now();
    while verify_mock.hits() == 0 && started.elapsed() < Duration::from_secs(2) {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(register_mock.hits(), 1);
    assert_eq!(lease_mock.hits(), 1);
    assert_eq!(verify_mock.hits(), 1, "forced robot refresh should start");

    shutdown.cancel();
    tokio::time::timeout(Duration::from_millis(500), run_task)
        .await
        .expect("hanging forced refresh must not delay robot shutdown")
        .expect("robot engine task join")
        .expect("robot engine should shut down cleanly");
}

struct BlockingRunner {
    started: Arc<AtomicBool>,
    release: Arc<Notify>,
}

#[async_trait]
impl compute_runner_api::Runner for BlockingRunner {
    fn capability(&self) -> &'static str {
        "/posemesh/blocking/v1"
    }

    async fn run(&self, _ctx: compute_runner_api::TaskCtx<'_>) -> anyhow::Result<()> {
        let release = self.release.notified();
        self.started.store(true, Ordering::Release);
        release.await;
        Ok(())
    }
}

#[tokio::test]
async fn run_robot_node_finishes_an_active_lease_before_shutdown() {
    let server = MockServer::start();
    let robot_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();
    let domain_id = Uuid::new_v4();
    let expires_at = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
    let lease_expires_at = (chrono::Utc::now() + chrono::Duration::seconds(30)).to_rfc3339();

    let register_mock = server.mock(move |when, then| {
        when.method(POST).path("/internal/v1/robots/register");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "robot_id": robot_id,
                "access_token": "robot-active-token",
                "access_expires_at": expires_at,
            }));
    });
    let lease_mock = server.mock({
        let base_url = server.base_url();
        let lease_expires_at = lease_expires_at.clone();
        move |when, then| {
            when.method(GET)
                .path("/tasks")
                .header("authorization", "Bearer robot-active-token");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "access_token": "active-session-token",
                    "lease_expires_at": lease_expires_at,
                    "domain_id": domain_id,
                    "domain_server_url": base_url,
                    "task": {
                        "id": task_id,
                        "capability": "/posemesh/blocking/v1",
                        "outputs_prefix": "out"
                    }
                }));
        }
    });
    let heartbeat_mock = server.mock({
        let base_url = server.base_url();
        move |when, then| {
            when.method(POST)
                .path(format!("/tasks/{task_id}/heartbeat"))
                .header("authorization", "Bearer robot-active-token");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "access_token": "active-session-token",
                    "lease_expires_at": (chrono::Utc::now()
                        + chrono::Duration::seconds(30))
                        .to_rfc3339(),
                    "cancel": false,
                    "status": "running",
                    "domain_id": domain_id,
                    "domain_server_url": base_url,
                    "task_id": task_id
                }));
        }
    });
    let complete_mock = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/tasks/{task_id}/complete"))
            .header("authorization", "Bearer robot-active-token");
        then.status(200);
    });

    let started = Arc::new(AtomicBool::new(false));
    let release = Arc::new(Notify::new());
    let runners = RunnerRegistry::new().register(BlockingRunner {
        started: started.clone(),
        release: release.clone(),
    });
    let cfg = robot_cfg(&server);
    let shutdown = CancellationToken::new();
    let mut run_task = tokio::spawn(run_robot_node_with_shutdown(cfg, runners, shutdown.clone()));

    let wait_started = Instant::now();
    while !started.load(Ordering::Acquire) && wait_started.elapsed() < Duration::from_secs(2) {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        started.load(Ordering::Acquire),
        "runner should start after a lease is acquired"
    );

    shutdown.cancel();
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut run_task)
            .await
            .is_err(),
        "shutdown must not drop an active leased cycle"
    );
    complete_mock.assert_hits(0);

    release.notify_one();
    tokio::time::timeout(Duration::from_secs(2), run_task)
        .await
        .expect("active cycle should finish after the runner is released")
        .expect("robot engine task join")
        .expect("robot engine should shut down after completing the lease");

    register_mock.assert_hits(1);
    lease_mock.assert_hits(1);
    assert!(heartbeat_mock.hits() >= 1, "heartbeat should remain active");
    complete_mock.assert_hits(1);
}
