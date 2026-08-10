use async_trait::async_trait;
use httpmock::prelude::*;
use posemesh_compute_node::auth::token_manager::{TokenProvider, TokenProviderResult};
use posemesh_compute_node::dms::{client::DmsClient, types::HeartbeatRequest};
use serde_json::json;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::subscriber;
use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt, registry};
use uuid::Uuid;

#[derive(Clone)]
struct RotatingProvider {
    tokens: Arc<Vec<String>>,
    current: Arc<AtomicUsize>,
}

impl RotatingProvider {
    fn new(tokens: &[&str]) -> Self {
        Self {
            tokens: Arc::new(tokens.iter().map(|token| (*token).to_string()).collect()),
            current: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl TokenProvider for RotatingProvider {
    async fn bearer(&self) -> TokenProviderResult<String> {
        let index = self
            .current
            .load(Ordering::SeqCst)
            .min(self.tokens.len() - 1);
        Ok(self.tokens[index].clone())
    }

    async fn on_unauthorized(&self) {
        self.current.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn dms_response_tokens_and_bodies_are_absent_from_logs_and_errors() {
    const LEASE_TOKEN: &str = "secret-lease-task-access-token";
    const HEARTBEAT_TOKEN: &str = "secret-heartbeat-task-access-token";
    const LEASE_UNAUTHORIZED_BODY: &str = "secret-lease-unauthorized-body";
    const HEARTBEAT_UNAUTHORIZED_BODY: &str = "secret-heartbeat-unauthorized-body";
    const LEASE_ERROR_BODY: &str = "secret-lease-final-error-body";
    const HEARTBEAT_ERROR_BODY: &str = "secret-heartbeat-final-error-body";

    let server = MockServer::start();
    let task_id = Uuid::new_v4();
    let error_task_id = Uuid::new_v4();

    let lease_unauthorized = server.mock(|when, then| {
        when.method(GET)
            .path("/tasks")
            .header("authorization", "Bearer node-token-a");
        then.status(401).body(LEASE_UNAUTHORIZED_BODY);
    });
    let lease_success = server.mock(|when, then| {
        when.method(GET)
            .path("/tasks")
            .header("authorization", "Bearer node-token-b");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "access_token": LEASE_TOKEN,
                "task": {
                    "id": task_id,
                    "capability": "/posemesh/redaction-test/v1"
                }
            }));
    });
    let heartbeat_unauthorized = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/tasks/{task_id}/heartbeat"))
            .header("authorization", "Bearer node-token-b");
        then.status(401).body(HEARTBEAT_UNAUTHORIZED_BODY);
    });
    let heartbeat_success = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/tasks/{task_id}/heartbeat"))
            .header("authorization", "Bearer node-token-c");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "access_token": HEARTBEAT_TOKEN,
                "task_id": task_id,
                "cancel": false
            }));
    });
    let lease_error = server.mock(|when, then| {
        when.method(GET)
            .path("/tasks")
            .header("authorization", "Bearer node-token-c");
        then.status(502).body(LEASE_ERROR_BODY);
    });
    let heartbeat_error = server.mock(|when, then| {
        when.method(POST)
            .path(format!("/tasks/{error_task_id}/heartbeat"))
            .header("authorization", "Bearer node-token-c");
        then.status(503).body(HEARTBEAT_ERROR_BODY);
    });

    let (guard, logs) = capture_logs();
    let provider = Arc::new(RotatingProvider::new(&[
        "node-token-a",
        "node-token-b",
        "node-token-c",
    ]));
    let client = DmsClient::new(
        server.base_url().parse().unwrap(),
        Duration::from_secs(2),
        provider,
    )
    .unwrap();

    let lease = client
        .lease_by_capability("/posemesh/redaction-test/v1")
        .await
        .unwrap()
        .expect("lease after one 401 retry");
    assert_eq!(lease.access_token.as_deref(), Some(LEASE_TOKEN));

    let heartbeat = client
        .heartbeat(task_id, &HeartbeatRequest::default())
        .await
        .expect("heartbeat after one 401 retry");
    assert_eq!(heartbeat.access_token.as_deref(), Some(HEARTBEAT_TOKEN));

    let lease_err = client
        .lease_by_capability("/posemesh/redaction-test/v1")
        .await
        .expect_err("final lease error should be returned");
    let heartbeat_err = client
        .heartbeat(error_task_id, &HeartbeatRequest::default())
        .await
        .expect_err("final heartbeat error should be returned");
    drop(guard);

    lease_unauthorized.assert_hits(1);
    lease_success.assert_hits(1);
    heartbeat_unauthorized.assert_hits(1);
    heartbeat_success.assert_hits(1);
    lease_error.assert_hits(1);
    heartbeat_error.assert_hits(1);

    let lease_error_text = lease_err.to_string();
    let heartbeat_error_text = heartbeat_err.to_string();
    let captured = String::from_utf8(logs.lock().clone()).unwrap_or_default();
    assert!(captured.contains("DMS lease unauthorized"));
    assert!(captured.contains("Decoded DMS lease response"));
    assert!(captured.contains("Decoded DMS heartbeat response"));
    assert!(captured.contains(&task_id.to_string()));

    for sensitive in [
        "node-token-a",
        "node-token-b",
        "node-token-c",
        LEASE_TOKEN,
        HEARTBEAT_TOKEN,
        LEASE_UNAUTHORIZED_BODY,
        HEARTBEAT_UNAUTHORIZED_BODY,
        LEASE_ERROR_BODY,
        HEARTBEAT_ERROR_BODY,
    ] {
        assert!(
            !captured.contains(sensitive),
            "DMS logs exposed sensitive response data: {captured}"
        );
        assert!(!lease_error_text.contains(sensitive));
        assert!(!heartbeat_error_text.contains(sensitive));
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
    let guard = subscriber::set_default(registry().with(LevelFilter::DEBUG).with(layer));
    (guard, buffer)
}
