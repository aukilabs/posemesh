use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use posemesh_compute_node::dds::persist;
use tower::util::ServiceExt;
use tracing::subscriber;
use tracing_subscriber::layer::SubscriberExt;

fn capture_logs() -> (
    tracing::subscriber::DefaultGuard,
    std::sync::Arc<parking_lot::Mutex<Vec<u8>>>,
) {
    use std::io;
    use std::sync::Arc;

    struct BufWriter(Arc<parking_lot::Mutex<Vec<u8>>>);
    impl io::Write for BufWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct MakeBufWriter(Arc<parking_lot::Mutex<Vec<u8>>>);
    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for MakeBufWriter {
        type Writer = BufWriter;
        fn make_writer(&'a self) -> Self::Writer {
            BufWriter(self.0.clone())
        }
    }

    let buffer = Arc::new(parking_lot::Mutex::new(Vec::<u8>::new()));
    let make_writer = MakeBufWriter(buffer.clone());
    let layer = tracing_subscriber::fmt::layer()
        .with_writer(make_writer)
        .with_ansi(false)
        .without_time();
    let subscriber = tracing_subscriber::registry().with(layer);
    let guard = subscriber::set_default(subscriber);
    (guard, buffer)
}

#[tokio::test(flavor = "current_thread")]
async fn health_ok() {
    persist::clear_node_secret().unwrap();
    let app = posemesh_compute_node::http::router();

    let res = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test(flavor = "current_thread")]
async fn registration_persists_and_redacts_secret() {
    persist::clear_node_secret().unwrap();
    let (guard, logs) = capture_logs();

    let app = posemesh_compute_node::http::router();
    let secret = "super-secret";
    let body = serde_json::json!({
        "id": "test-node",
        "secret": secret,
        "organization_id": "org-123",
        "lighthouses_in_domains": [],
        "domains": []
    })
    .to_string();

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/v1/registrations")
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let stored = persist::read_node_secret().unwrap();
    assert_eq!(stored.as_deref(), Some(secret));
    persist::clear_node_secret().unwrap();

    drop(guard);
    let captured = String::from_utf8(logs.lock().clone()).unwrap_or_default();
    assert!(captured.contains("Received registration callback"));
    assert!(
        !captured.contains(secret),
        "logs leaked secret: {}",
        captured
    );
}
