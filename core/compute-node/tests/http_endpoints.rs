use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::util::ServiceExt;

#[tokio::test]
async fn health_ok() {
    let response = posemesh_compute_node::http::router()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn obsolete_registration_callback_is_not_exposed() {
    let response = posemesh_compute_node::http::router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/v1/registrations")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"secret":"local-fixture"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
