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
        auki_p2p_enabled: false,
        auki_p2p_listen_multiaddrs: Vec::new(),
        auki_p2p_advertised_multiaddrs: Vec::new(),
        auki_p2p_private_key: None,
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
    cfg.set_audience(format!("{}/robots", server.base_url()))
        .unwrap();
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
    let now = chrono::Utc::now() + chrono::Duration::minutes(1);
    // Lease: return token A and domain url pointing to same mock server
    let lease_body = json!({
        "access_token": support::sdk::data_token(&base_url, domain_id, now, "A"),
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
                "access_token": support::sdk::data_token(&hb_base_url, domain_id, now, "B"),
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
    support::sdk::info(&server);
    let artifact_id = Uuid::new_v4();
    let upload_path = format!("/api/v1/domains/{}/data", domain_id);
    let upload_mock = server.mock({
        let upload_path = upload_path.clone();
        let base_url = base_url.clone();
        move |when, then| {
            when.method(POST).path(upload_path.as_str()).header(
                "authorization",
                format!(
                    "Bearer {}",
                    support::sdk::data_token(&base_url, domain_id, now, "B")
                ),
            );
            then.status(200)
                .header("content-type", "application/json")
                .json_body(
                    json!({"data":[support::sdk::metadata(domain_id, artifact_id, "n", "d", 15)]}),
                );
        }
    });

    // Complete
    let complete_cap = cap.clone();
    let complete_mock = server.mock(move |when, then| {
        when.method(POST)
            .path(format!("/tasks/{}/complete", task_id))
            .header("authorization", format!("Bearer {}", node_token))
            .header("content-type", "application/json")
            .body_contains(artifact_id.to_string())
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
    let now = chrono::Utc::now() + chrono::Duration::minutes(1);

    let reg = RunnerRegistry::new().register(ErrRunner);
    let capabilities = reg.capabilities();
    let err_cap = capabilities.first().cloned().expect("capability present");
    let base_url = server.base_url().to_string();

    let lease_body = json!({
        "access_token": support::sdk::data_token(&base_url, domain_id, now, "A"),
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
                "access_token": support::sdk::data_token(&hb_base_url, domain_id, now, "A"),
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
            .body_contains("\"artifacts\"")
            .body_contains("boom");
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

    let task_id = Uuid::new_v4();
    let job_id = Uuid::new_v4();
    let domain_id = Uuid::new_v4();
    let issued_at = chrono::Utc::now();
    let lease_now = chrono::Utc::now() + chrono::Duration::minutes(1);
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

    let register = server.mock(|when, then| {
        when.method(POST)
            .path("/internal/v1/nodes/register-wallet")
            .body_contains(support::mock_runner::MOCK_CAPABILITY_LOCAL)
            .body_contains(support::mock_runner::MOCK_CAPABILITY_GLOBAL);
        then.status(200);
    });
    let mut runners = RunnerRegistry::new();
    for runner in support::mock_runner::runners_for_all_capabilities() {
        runners = runners.register(runner);
    }
    let capabilities = runners.capabilities();
    let cap = capabilities.get(1).cloned().expect("capability present");
    let base_url = server.base_url().to_string();

    let lease_mock = server.mock({
        let cap = cap.clone();
        let siwe_token = siwe_token.to_string();
        let base_url = base_url.clone();
        let lease_expiry = lease_now_iso.clone();
        move |when, then| {
            when.method(GET)
                .path("/tasks")
                .matches(|request| {
                    request
                        .query_params
                        .as_ref()
                        .is_none_or(|params| params.iter().all(|(name, _)| name != "capability"))
                })
                .header("authorization", format!("Bearer {}", siwe_token));
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "access_token": support::sdk::data_token(&base_url, domain_id, lease_now, "A"),
                    "access_token_expires_at": lease_expiry,
                    "lease_expires_at": lease_expiry,
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
        let lease_expiry = lease_now_iso.clone();
        let base_url = base_url.clone();
        move |when, then| {
            when.method(POST)
                .path(format!("/tasks/{}/heartbeat", task_id))
                .header("authorization", format!("Bearer {}", siwe_token))
                .header("content-type", "application/json");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "access_token": support::sdk::data_token(&base_url, domain_id, lease_now, "B"),
                    "access_token_expires_at": lease_expiry,
                    "lease_expires_at": lease_expiry,
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

    support::sdk::info(&server);
    let artifact_id = Uuid::new_v4();
    let upload_path = format!("/api/v1/domains/{}/data", domain_id);
    let upload_mock = server.mock({
        let upload_path = upload_path.clone();
        let base_url = base_url.clone();
        move |when, then| {
            when.method(POST).path(upload_path.as_str()).header(
                "authorization",
                format!(
                    "Bearer {}",
                    support::sdk::data_token(&base_url, domain_id, lease_now, "B")
                ),
            );
            then.status(200)
                .header("content-type", "application/json")
                .json_body(
                    json!({"data":[support::sdk::metadata(domain_id, artifact_id, "n", "d", 15)]}),
                );
        }
    });

    let complete_mock = server.mock({
        let siwe_token = siwe_token.to_string();
        let siwe_token = siwe_token.to_string();
        move |when, then| {
            when.method(POST)
                .path(format!("/tasks/{}/complete", task_id))
                .header("authorization", format!("Bearer {}", siwe_token))
                .header("content-type", "application/json");
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
        auki_p2p_enabled: false,
        auki_p2p_listen_multiaddrs: Vec::new(),
        auki_p2p_advertised_multiaddrs: Vec::new(),
        auki_p2p_private_key: None,
        register_interval_secs: None,
        register_max_retry: None,
        max_concurrency: 1,
        log_format: LogFormat::Json,
        enable_noop: true,
        noop_sleep_secs: 0,
    };

    // Existing hosts may still make this call. It must not start a registrar.
    posemesh_compute_node::dds::register::spawn_registration_if_configured(&cfg, &capabilities)
        .unwrap();
    register.assert_hits(0);
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
    while complete_mock.hits() < 1 && start_upload.elapsed() < Duration::from_secs(5) {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let upload_hits = upload_mock.hits();
    if upload_hits < 1 {
        panic!(
            "expected at least one domain upload for runner artifacts, got {}",
            upload_hits
        );
    }
    assert!(
        complete_mock.hits() >= 1,
        "Completion endpoint should be hit at least once"
    );

    register.assert_hits(1);
    shutdown.cancel();
    run_task
        .await
        .expect("task join")
        .expect("run_node_with_shutdown should exit cleanly after cancellation");
    register.assert_hits(1);
}

#[tokio::test]
async fn run_robot_node_refreshes_after_dms_401_and_retries_with_machine_token() {
    let server = MockServer::start();
    let robot_id = Uuid::new_v4();
    let domain_id = Uuid::new_v4();
    let expiry = chrono::Utc::now() + chrono::Duration::hours(1);
    let expires_at = expiry.to_rfc3339();
    let token_a =
        support::sdk::robot_token(&server.base_url(), robot_id, domain_id, expiry, "A", None);
    let token_b =
        support::sdk::robot_token(&server.base_url(), robot_id, domain_id, expiry, "B", None);

    let register_mock = server.mock({
        let expires_at = expires_at.clone();
        let token_a = token_a.clone();
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
                    "access_token": token_a,
                    "access_expires_at": expires_at,
                }));
        }
    });
    let verify_mock = server.mock({
        let expires_at = expires_at.clone();
        let token_b = token_b.clone();
        move |when, then| {
            when.method(POST)
                .path("/internal/v1/auth/robot/verify")
                .body_contains("\"registration_credentials\":\"robot-test-credentials\"");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "robot_id": robot_id,
                    "access_token": token_b,
                    "access_expires_at": expires_at,
                }));
        }
    });
    let stale_lease_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/tasks")
            .header("authorization", format!("Bearer {token_a}"));
        then.status(401);
    });
    let refreshed_lease_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/tasks")
            .header("authorization", format!("Bearer {token_b}"));
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
async fn run_robot_node_stops_without_siwe_fallback_when_verify_fails() {
    let server = MockServer::start();
    let robot_id = Uuid::new_v4();
    let domain_id = Uuid::new_v4();
    let expiry = chrono::Utc::now() + chrono::Duration::hours(1);
    let expires_at = expiry.to_rfc3339();
    let token_a =
        support::sdk::robot_token(&server.base_url(), robot_id, domain_id, expiry, "A", None);

    let register_mock = server.mock(|when, then| {
        when.method(POST).path("/internal/v1/robots/register");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "robot_id": robot_id,
                "access_token": token_a,
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
            .header("authorization", format!("Bearer {token_a}"));
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
    assert_eq!(
        verify_mock.hits(),
        1,
        "robot verify should be attempted once"
    );

    tokio::time::timeout(Duration::from_secs(2), run_task)
        .await
        .expect("failed authentication must stop the host")
        .expect("robot engine task join")
        .expect_err("SDK fails closed after denied refresh");

    register_mock.assert_hits(1);
    stale_lease_mock.assert_hits(1);
    verify_mock.assert_hits(1);
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
    let domain_id = Uuid::new_v4();
    let expiry = chrono::Utc::now() + chrono::Duration::hours(1);
    let expires_at = expiry.to_rfc3339();
    let token_a =
        support::sdk::robot_token(&server.base_url(), robot_id, domain_id, expiry, "A", None);

    let register_mock = server.mock(|when, then| {
        when.method(POST).path("/internal/v1/robots/register");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "robot_id": robot_id,
                "access_token": token_a,
                "access_expires_at": expires_at,
            }));
    });
    let lease_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/tasks")
            .header("authorization", format!("Bearer {token_a}"));
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
    let expiry = chrono::Utc::now() + chrono::Duration::hours(1);
    let expires_at = expiry.to_rfc3339();
    let token_a =
        support::sdk::robot_token(&server.base_url(), robot_id, domain_id, expiry, "A", None);
    let lease_expires_at = (chrono::Utc::now() + chrono::Duration::seconds(30)).to_rfc3339();

    let register_mock = server.mock(|when, then| {
        when.method(POST).path("/internal/v1/robots/register");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "robot_id": robot_id,
                "access_token": token_a,
                "access_expires_at": expires_at,
            }));
    });
    let lease_mock = server.mock({
        let base_url = server.base_url();
        let token_a = token_a.clone();
        let lease_expires_at = lease_expires_at.clone();
        move |when, then| {
            when.method(GET)
                .path("/tasks")
                .header("authorization", format!("Bearer {token_a}"));
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "access_token": support::sdk::data_token(&base_url, domain_id, expiry, "task"),
                    "access_token_expires_at": expiry,
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
        let token_a = token_a.clone();
        move |when, then| {
            when.method(POST)
                .path(format!("/tasks/{task_id}/heartbeat"))
                .header("authorization", format!("Bearer {token_a}"));
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "access_token": support::sdk::data_token(&base_url, domain_id, expiry, "task"),
                    "access_token_expires_at": expiry,
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
            .header("authorization", format!("Bearer {token_a}"));
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

#[tokio::test]
async fn managed_runner_preserves_inputs_tokens_events_replacements_and_failure_artifacts() {
    use compute_runner_api::runner::{DomainArtifactContent, DomainArtifactRequest};
    struct CompatibilityRunner {
        initial: String,
        renewed: String,
        job: Uuid,
        input: Uuid,
        artifact: Uuid,
        ready: Arc<Notify>,
        resume: Arc<Notify>,
    }
    #[async_trait]
    impl compute_runner_api::Runner for CompatibilityRunner {
        fn capability(&self) -> &'static str {
            "/test/compat/v1"
        }
        async fn run(&self, ctx: compute_runner_api::TaskCtx<'_>) -> anyhow::Result<()> {
            assert_eq!(ctx.lease.task.job_id, Some(self.job));
            assert_eq!(ctx.lease.task.attempts, Some(2));
            assert_eq!(ctx.access_token.get(), self.initial);
            assert!(ctx.lease.p2p_access_token.is_none());
            let input = ctx
                .input
                .materialize_cid_with_meta(&self.input.to_string())
                .await?;
            assert_eq!(
                input.data_id.as_deref(),
                Some(self.input.to_string().as_str())
            );
            assert_eq!(input.name.as_deref(), Some("scan_2026-09-01_12-30-00"));
            assert_eq!(
                input.path.strip_prefix(&input.root_dir).unwrap().to_str(),
                Some("datasets/2026-09-01_12-30-00/scan_2026-09-01_12-30-00.custom")
            );
            assert_eq!(tokio::fs::read(&input.path).await?, b"input");
            tokio::fs::remove_dir_all(input.root_dir).await?;
            let foreign = format!("/api/v1/domains/{}/data/{}", Uuid::new_v4(), self.input);
            assert!(ctx.input.materialize_cid_with_meta(&foreign).await.is_err());
            for bytes in [b"first".as_slice(), b"replacement".as_slice()] {
                let id = ctx
                    .output
                    .put_domain_artifact_with_metadata(
                        DomainArtifactRequest {
                            rel_path: "result.custom",
                            name: "stable-name",
                            data_type: "custom",
                            existing_id: None,
                            content: DomainArtifactContent::Bytes(bytes),
                        },
                        json!({"rows":7}),
                    )
                    .await?;
                assert_eq!(id, Some(self.artifact.to_string()));
            }
            self.ready.notify_one();
            self.resume.notified().await;
            ctx.ctrl.log_event(json!({"stage":"first"})).await?;
            ctx.ctrl.log_event(json!({"stage":"second"})).await?;
            tokio::time::timeout(Duration::from_secs(2), async {
                while ctx.access_token.get() != self.renewed {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await?;
            anyhow::bail!("compatibility failure")
        }
    }
    let server = MockServer::start();
    support::sdk::info(&server);
    let task = Uuid::new_v4();
    let job = Uuid::new_v4();
    let domain = Uuid::new_v4();
    let input = Uuid::new_v4();
    let artifact = Uuid::new_v4();
    let expiry = chrono::Utc::now() + chrono::Duration::minutes(1);
    let tokens: Vec<_> = ["A", "B", "C"]
        .iter()
        .map(|generation| support::sdk::data_token(&server.base_url(), domain, expiry, generation))
        .collect();
    let grant = json!({"task":{"id":task,"capability":"/test/compat/v1","outputs_prefix":"out"},
        "domain_id":domain,"domain_server_url":server.base_url(),"access_token":tokens[0],
        "access_token_expires_at":expiry,"lease_expires_at":expiry});
    server.mock(|when, then| {
        when.method(GET).path("/tasks");
        then.header("content-type", "application/json")
            .json_body(grant.clone());
    });
    let mut initial = grant.clone();
    initial["access_token"] = json!(tokens[1]);
    initial.as_object_mut().unwrap().remove("task");
    initial["task_id"] = json!(task);
    initial["job_id"] = json!(job);
    initial["attempts"] = json!(2);
    let mut initial_heartbeat = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/tasks/{task}/heartbeat"))
            .body_contains("\"events\":[]");
        then.header("content-type", "application/json")
            .json_body(initial.clone());
    });
    let mut renewed = initial.clone();
    renewed["access_token"] = json!(tokens[2]);
    let events = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/tasks/{task}/heartbeat"))
            .body_contains("\"events\":[{\"stage\":\"first\"},{\"stage\":\"second\"}]");
        then.header("content-type", "application/json")
            .json_body(renewed);
    });
    let path = format!("/api/v1/domains/{domain}/data");
    server.mock(|when, then| {
        when.method(GET).path(&path).query_param("ids",input.to_string());
        then.header("content-type","application/json").json_body(json!({"data":[support::sdk::metadata(domain,input,"scan_2026-09-01_12-30-00","custom",5)]}));
    });
    let download = server.mock(|when, then| {
        when.method(GET)
            .path(format!("{path}/{input}"))
            .query_param("raw", "true")
            .header("authorization", format!("Bearer {}", tokens[1]));
        then.body("input");
    });
    let find = server.mock(|when, then| {
        when.method(GET)
            .path(&path)
            .query_param("name", "stable-name")
            .query_param("data_type", "custom");
        then.header("content-type", "application/json")
            .json_body(json!({"data":[]}));
    });
    let create = server.mock(|when, then| {
        when.method(POST)
            .path(&path)
            .header("authorization", format!("Bearer {}", tokens[1]))
            .body_contains("first");
        then.header("content-type", "application/json").json_body(
            json!({"data":[support::sdk::metadata(domain,artifact,"stable-name","custom",5)]}),
        );
    });
    let replace = server.mock(|when, then| {
        when.method(PUT)
            .path(&path)
            .header("authorization", format!("Bearer {}", tokens[1]))
            .body_contains("replacement")
            .body_contains(format!("id=\"{artifact}\""));
        then.header("content-type", "application/json").json_body(
            json!({"data":[support::sdk::metadata(
                domain,
                artifact,
                "stable-name",
                "custom",
                11,
            )]}),
        );
    });
    let failure = server.mock(|when, then| {
        when.method(POST).path(format!("/tasks/{task}/fail")).json_body(json!({
            "reason":"runner failed: compatibility failure",
            "details":{"job":{"task_id":task,"job_id":job,"domain_id":domain,"capability":"/test/compat/v1"},
                "artifacts":[{"logical_path":"out/result.custom","name":"stable-name","data_type":"custom","id":artifact,"metadata":{"rows":7}}]}
        }));
        then.status(200);
    });
    let dms = DmsClient::new(
        server.base_url().parse().unwrap(),
        Duration::from_secs(2),
        Arc::new(StaticProvider {
            token: "machine".into(),
        }),
    )
    .unwrap();
    let ready = Arc::new(Notify::new());
    let resume = Arc::new(Notify::new());
    let runner = CompatibilityRunner {
        initial: tokens[1].clone(),
        renewed: tokens[2].clone(),
        job,
        input,
        artifact,
        ready: ready.clone(),
        resume: resume.clone(),
    };
    let run = tokio::spawn(async move {
        run_cycle_with_dms(&base_cfg(), &dms, &RunnerRegistry::new().register(runner)).await
    });
    tokio::time::timeout(Duration::from_secs(2), ready.notified())
        .await
        .unwrap();
    initial_heartbeat.delete();
    initial.as_object_mut().unwrap().remove("access_token");
    server.mock(|when, then| {
        when.method(POST)
            .path(format!("/tasks/{task}/heartbeat"))
            .body_contains("\"events\":[]");
        then.header("content-type", "application/json")
            .json_body(initial);
    });
    resume.notify_one();
    let result = tokio::time::timeout(Duration::from_secs(3), run)
        .await
        .unwrap()
        .unwrap();
    failure.assert_hits(1);
    assert!(result.unwrap());
    events.assert_hits(1);
    download.assert_hits(1);
    find.assert_hits(1);
    create.assert_hits(1);
    replace.assert_hits(1);
}

#[tokio::test]
async fn forced_robot_shutdown_revokes_token_and_awaits_runner_cleanup_without_a_receipt() {
    use posemesh_compute_node::engine::run_robot_node_with_shutdowns;
    struct CleanupRunner {
        entered: Arc<Notify>,
        stopping: Arc<Notify>,
        release: Arc<Notify>,
    }
    #[async_trait]
    impl compute_runner_api::Runner for CleanupRunner {
        fn capability(&self) -> &'static str {
            "/test/cleanup/v1"
        }
        async fn run(&self, ctx: compute_runner_api::TaskCtx<'_>) -> anyhow::Result<()> {
            self.entered.notify_one();
            while !ctx.ctrl.is_cancelled().await {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            assert!(
                ctx.access_token.get().is_empty(),
                "retained getter must stop exposing authority"
            );
            self.stopping.notify_one();
            self.release.notified().await;
            Ok(())
        }
    }
    let server = MockServer::start();
    let task = Uuid::new_v4();
    let robot = Uuid::new_v4();
    let domain = Uuid::new_v4();
    let expiry = chrono::Utc::now() + chrono::Duration::minutes(1);
    server.mock(|when, then| {
        when.method(POST).path("/internal/v1/robots/register");
        then.json_body(json!({"robot_id":robot,"access_token":support::sdk::robot_token(&server.base_url(),robot,domain,expiry,"A",None),"access_expires_at":expiry}));
    });
    let grant = json!({"task":{"id":task,"capability":"/test/cleanup/v1"},"domain_id":domain,"domain_server_url":server.base_url(),
        "access_token":support::sdk::data_token(&server.base_url(),domain,expiry,"task"),"access_token_expires_at":expiry,"lease_expires_at":expiry});
    let claim = server.mock(|when, then| {
        when.method(GET).path("/tasks");
        then.json_body(grant.clone());
    });
    let heartbeat = server.mock(|when, then| {
        when.method(POST).path(format!("/tasks/{task}/heartbeat"));
        then.json_body(grant.clone());
    });
    let complete = server.mock(|when, then| {
        when.method(POST).path(format!("/tasks/{task}/complete"));
        then.status(200);
    });
    let fail = server.mock(|when, then| {
        when.method(POST).path(format!("/tasks/{task}/fail"));
        then.status(200);
    });
    let entered = Arc::new(Notify::new());
    let stopping = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let forced = CancellationToken::new();
    let engine = tokio::spawn(run_robot_node_with_shutdowns(
        robot_cfg(&server),
        RunnerRegistry::new().register(CleanupRunner {
            entered: entered.clone(),
            stopping: stopping.clone(),
            release: release.clone(),
        }),
        CancellationToken::new(),
        forced.clone(),
    ));
    tokio::time::timeout(Duration::from_secs(2), entered.notified())
        .await
        .unwrap();
    forced.cancel();
    tokio::time::timeout(Duration::from_secs(2), stopping.notified())
        .await
        .unwrap();
    assert!(
        !engine.is_finished(),
        "host must await hardware/process cleanup"
    );
    release.notify_one();
    tokio::time::timeout(Duration::from_secs(2), engine)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    claim.assert_hits(1);
    heartbeat.assert_hits(1);
    complete.assert_hits(0);
    fail.assert_hits(0);
}

#[tokio::test]
async fn managed_large_file_upload_uses_bounded_multipart_and_preserves_receipt() {
    struct FileRunner(std::path::PathBuf);
    #[async_trait]
    impl compute_runner_api::Runner for FileRunner {
        fn capability(&self) -> &'static str {
            "/test/file/v1"
        }
        async fn run(&self, ctx: compute_runner_api::TaskCtx<'_>) -> anyhow::Result<()> {
            ctx.output.put_file("large.bin", &self.0).await
        }
    }
    const PART: usize = 16 * 1024 * 1024;
    const SIZE: usize = 64 * 1024 * 1024 + 1;
    let file = tempfile::NamedTempFile::new().unwrap();
    file.as_file().set_len(SIZE as u64).unwrap();
    let server = MockServer::start();
    let task = Uuid::new_v4();
    let domain = Uuid::new_v4();
    let artifact = Uuid::new_v4();
    let upload = Uuid::new_v4();
    let expiry = chrono::Utc::now() + chrono::Duration::minutes(1);
    let grant = json!({"task":{"id":task,"capability":"/test/file/v1","outputs_prefix":"out"},"domain_id":domain,"domain_server_url":server.base_url(),
        "access_token":support::sdk::data_token(&server.base_url(),domain,expiry,"task"),"access_token_expires_at":expiry,"lease_expires_at":expiry});
    server.mock(|when, then| {
        when.method(GET).path("/tasks");
        then.json_body(grant.clone());
    });
    server.mock(|when, then| {
        when.method(POST).path(format!("/tasks/{task}/heartbeat"));
        then.json_body(grant.clone());
    });
    server.mock(|when, then| {
        when.method(GET).path("/api/v1/info");
        then.header("content-type","application/json").json_body(json!({"upload":{"request_max_bytes":PART+4096,"domain_data_max_bytes":SIZE+1,"multipart":{"enabled":true,"part_size_bytes":PART}}}));
    });
    let path = format!("/api/v1/domains/{domain}/data");
    let name = format!("out_large_bin_{task}");
    server.mock(|when, then| {
        when.method(GET).path(&path).query_param("name", &name);
        then.header("content-type", "application/json")
            .json_body(json!({"data":[]}));
    });
    let initiate = server.mock(|when, then| {
        when.method(POST)
            .path(format!("{path}/multipart"))
            .query_param("uploads", "")
            .body_contains(format!("\"size\":{SIZE}"))
            .body_contains(format!("\"name\":\"{name}\""))
            .body_contains("\"data_type\":\"bin_data\"");
        then.header("content-type", "application/json").json_body(
            json!({"upload_id":upload,"data_id":artifact,"part_size":PART,"expires_at":expiry}),
        );
    });
    let parts: Vec<_> = (1..=5)
        .map(|part| {
            server.mock(|when, then| {
                when.method(PUT)
                    .path(format!("{path}/multipart"))
                    .query_param("uploadId", upload.to_string())
                    .query_param("partNumber", part.to_string())
                    .header(
                        "content-length",
                        if part == 5 { 1 } else { PART }.to_string(),
                    );
                then.header("content-type", "application/json")
                    .json_body(json!({"etag":format!("part-{part}")}));
            })
        })
        .collect();
    let commit = server.mock(|when, then| {
        when.method(POST).path(format!("{path}/multipart")).query_param("uploadId",upload.to_string())
            .json_body(json!({"parts":(1..=5).map(|part|json!({"part_number":part,"etag":format!("part-{part}")})).collect::<Vec<_>>()}));
        then.header("content-type","application/json").json_body(support::sdk::metadata(domain,artifact,&name,"bin_data",SIZE));
    });
    let complete = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/tasks/{task}/complete"))
            .body_contains(artifact.to_string())
            .body_contains("out/large.bin")
            .body_contains("bin_data");
        then.status(200);
    });
    let dms = DmsClient::new(
        server.base_url().parse().unwrap(),
        Duration::from_secs(2),
        Arc::new(StaticProvider {
            token: "machine".into(),
        }),
    )
    .unwrap();
    let result = run_cycle_with_dms(
        &base_cfg(),
        &dms,
        &RunnerRegistry::new().register(FileRunner(file.path().into())),
    )
    .await;
    initiate.assert_hits(1);
    for part in parts {
        part.assert_hits(1);
    }
    commit.assert_hits(1);
    complete.assert_hits(1);
    assert!(result.unwrap());
}
