use httpmock::prelude::*;
use posemesh_compute_node::auth::token_manager::{TokenManagerConfig, TokenProvider};
use posemesh_compute_node::auth::RobotMachineAuth;
use serde_json::json;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tracing::subscriber;
use tracing_subscriber::{layer::SubscriberExt, registry};
use uuid::Uuid;

const REGISTER_PATH: &str = "/internal/v1/robots/register";
const VERIFY_PATH: &str = "/internal/v1/auth/robot/verify";
const SIWE_REQUEST_PATH: &str = "/internal/v1/auth/siwe/request";
const SIWE_VERIFY_PATH: &str = "/internal/v1/auth/siwe/verify";

fn token_config() -> TokenManagerConfig {
    TokenManagerConfig {
        safety_ratio: 0.75,
        max_retries: 0,
        jitter: Duration::ZERO,
    }
}

fn access_response(robot_id: Uuid, token: &str) -> serde_json::Value {
    json!({
        "robot_id": robot_id,
        "access_token": token,
        "access_expires_at": (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339(),
    })
}

fn robot_auth(
    server: &MockServer,
    credentials: &str,
    capabilities: Vec<String>,
) -> RobotMachineAuth {
    RobotMachineAuth::new(
        server.base_url().parse().unwrap(),
        credentials,
        "1.2.3",
        capabilities,
        Duration::from_secs(5),
        token_config(),
    )
    .unwrap()
}

#[tokio::test]
async fn register_payload_is_exact_and_forced_refresh_is_single_flight_verify() {
    let server = MockServer::start();
    let robot_id = Uuid::new_v4();
    let credentials = "opaque-robot-credentials";
    let capabilities = vec![
        "/example/robot/local/v1".to_string(),
        "/example/robot/global/v1".to_string(),
    ];

    let register = server.mock(|when, then| {
        when.method(POST).path(REGISTER_PATH).json_body(json!({
            "registration_credentials": credentials,
            "version": "1.2.3",
            "capabilities": capabilities,
        }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(access_response(robot_id, "robot-token-a"));
    });
    let verify = server.mock(|when, then| {
        when.method(POST).path(VERIFY_PATH).json_body(json!({
            "registration_credentials": credentials,
        }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(access_response(robot_id, "robot-token-b"));
    });

    let auth = robot_auth(&server, credentials, capabilities);
    let handle = auth.start().await.expect("robot auth starts");
    assert_eq!(handle.bearer().await.unwrap(), "robot-token-a");
    register.assert_hits(1);
    verify.assert_hits(0);

    handle.on_unauthorized().await;
    let mut tasks = Vec::new();
    for _ in 0..16 {
        let cloned = handle.clone();
        tasks.push(tokio::spawn(async move { cloned.bearer().await.unwrap() }));
    }
    for task in tasks {
        assert_eq!(task.await.unwrap(), "robot-token-b");
    }

    register.assert_hits(1);
    verify.assert_hits(1);
    handle.shutdown().await;
}

#[tokio::test]
async fn a_fresh_authenticator_registers_again_after_restart() {
    let server = MockServer::start();
    let robot_id = Uuid::new_v4();
    let credentials = "restart-safe-credentials";
    let capabilities = vec!["/example/robot/v1".to_string()];

    let register = server.mock(|when, then| {
        when.method(POST).path(REGISTER_PATH).json_body(json!({
            "registration_credentials": credentials,
            "version": "1.2.3",
            "capabilities": capabilities,
        }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(access_response(robot_id, "restart-token"));
    });
    let verify = server.mock(|when, then| {
        when.method(POST).path(VERIFY_PATH);
        then.status(500);
    });

    let first = robot_auth(&server, credentials, capabilities.clone());
    let first_handle = first.start().await.expect("first process registers");
    first.shutdown().await;
    assert!(
        first_handle.bearer().await.is_err(),
        "the owning authenticator must stop handles when it shuts down"
    );

    let restarted = robot_auth(&server, credentials, capabilities);
    let restarted_handle = restarted
        .start()
        .await
        .expect("restart registers idempotently");
    restarted_handle.shutdown().await;

    register.assert_hits(2);
    verify.assert_hits(0);
}

#[tokio::test]
async fn rejected_credentials_fail_closed_without_siwe_or_sensitive_logs() {
    let server = MockServer::start();
    let robot_id = Uuid::new_v4();
    let credentials = "rotated-or-revoked-robot-secret";
    let issued_token = "sensitive-robot-access-token";
    let upstream_body = "sensitive-upstream-response-body";
    let capabilities = vec!["/example/robot/v1".to_string()];

    let register = server.mock(|when, then| {
        when.method(POST).path(REGISTER_PATH);
        then.status(200)
            .header("content-type", "application/json")
            .json_body(access_response(robot_id, issued_token));
    });
    let verify = server.mock(|when, then| {
        when.method(POST).path(VERIFY_PATH).json_body(json!({
            "registration_credentials": credentials,
        }));
        then.status(401).body(upstream_body);
    });
    let siwe_request = server.mock(|when, then| {
        when.method(POST).path(SIWE_REQUEST_PATH);
        then.status(200);
    });
    let siwe_verify = server.mock(|when, then| {
        when.method(POST).path(SIWE_VERIFY_PATH);
        then.status(200);
    });
    let (guard, logs) = capture_logs();

    let auth = robot_auth(&server, credentials, capabilities);
    let handle = auth.start().await.expect("initial registration succeeds");
    handle.on_unauthorized().await;
    let err = handle
        .bearer()
        .await
        .expect_err("rejected credentials must not retain or replace the token");
    handle.shutdown().await;
    drop(guard);

    register.assert_hits(1);
    assert!(verify.hits() >= 1);
    siwe_request.assert_hits(0);
    siwe_verify.assert_hits(0);

    let error_text = err.to_string();
    let captured = String::from_utf8(logs.lock().clone()).unwrap_or_default();
    for sensitive in [credentials, issued_token, upstream_body] {
        assert!(!error_text.contains(sensitive));
        assert!(
            !captured.contains(sensitive),
            "robot authentication logs exposed sensitive material"
        );
    }
}

fn capture_logs() -> (
    tracing::subscriber::DefaultGuard,
    Arc<parking_lot::Mutex<Vec<u8>>>,
) {
    struct BufferWriter(Arc<parking_lot::Mutex<Vec<u8>>>);

    impl io::Write for BufferWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.0.lock().extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct MakeBufferWriter(Arc<parking_lot::Mutex<Vec<u8>>>);

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for MakeBufferWriter {
        type Writer = BufferWriter;

        fn make_writer(&'a self) -> Self::Writer {
            BufferWriter(self.0.clone())
        }
    }

    let buffer = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let layer = tracing_subscriber::fmt::layer()
        .with_writer(MakeBufferWriter(buffer.clone()))
        .with_ansi(false)
        .without_time();
    let guard = subscriber::set_default(registry().with(layer));
    (guard, buffer)
}
