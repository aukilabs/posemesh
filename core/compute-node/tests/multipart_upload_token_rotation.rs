#[path = "support/sdk.rs"]
#[allow(dead_code)]
mod sdk;
use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use posemesh_compute_node::storage::{
    client::{DomainClient, UploadRequest},
    TokenRef,
};
use serde_json::{json, Value};
use std::{collections::HashMap, future::IntoFuture, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

struct UploadState {
    domain: Uuid,
    id: Uuid,
    upload: Uuid,
    token: TokenRef,
    first: String,
    second: String,
    parts: std::sync::Mutex<Vec<Vec<u8>>>,
}

fn authorized(headers: &HeaderMap, token: &str) -> bool {
    headers.get("authorization").and_then(|v| v.to_str().ok())
        == Some(format!("Bearer {token}").as_str())
}

async fn multipart_post(
    State(state): State<Arc<UploadState>>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if query.contains_key("uploads") {
        if !authorized(&headers, &state.first) {
            return StatusCode::UNAUTHORIZED.into_response();
        }
        assert_eq!(body["name"], "big.bin");
        assert_eq!(body["data_type"], "binary");
        assert_eq!(body["size"], 12);
        assert_eq!(body["content_type"], "application/octet-stream");
        assert!(body["existing_id"].is_null());
        // The external owner rotates after initiation, before the SDK reads parts.
        state.token.swap(state.second.clone());
        return Json(
            json!({"upload_id":state.upload,"data_id":state.id,"part_size":5,
            "expires_at":chrono::Utc::now()+chrono::Duration::hours(1)}),
        )
        .into_response();
    }
    assert_eq!(query.get("uploadId"), Some(&state.upload.to_string()));
    if !authorized(&headers, &state.second) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    assert_eq!(
        body,
        json!({"parts":[{"part_number":1,"etag":"etag-1"},{"part_number":2,"etag":"etag-2"},{"part_number":3,"etag":"etag-3"}]})
    );
    Json(sdk::metadata(
        state.domain,
        state.id,
        "big.bin",
        "binary",
        12,
    ))
    .into_response()
}

async fn multipart_put(
    State(state): State<Arc<UploadState>>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    assert_eq!(query.get("uploadId"), Some(&state.upload.to_string()));
    if !authorized(&headers, &state.second) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let mut parts = state.parts.lock().unwrap();
    let part = parts.len() + 1;
    assert_eq!(query.get("partNumber"), Some(&part.to_string()));
    assert_eq!(body.len(), if part == 3 { 2 } else { 5 });
    parts.push(body.to_vec());
    Json(json!({"etag":format!("etag-{part}")})).into_response()
}

#[tokio::test]
async fn multipart_upload_uses_latest_token_after_rotation() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let domain = Uuid::new_v4();
    let expires = chrono::Utc::now() + chrono::Duration::hours(1);
    let first = sdk::data_token(&base, domain, expires, "A");
    let second = sdk::data_token(&base, domain, expires, "B");
    let token = TokenRef::new(first.clone());
    let state = Arc::new(UploadState {
        domain,
        id: Uuid::new_v4(),
        upload: Uuid::new_v4(),
        token: token.clone(),
        first,
        second,
        parts: Default::default(),
    });
    let app = Router::new()
        .route(
            "/api/v1/info",
            get(|| async {
                Json(json!({"upload":{
                    "request_max_bytes":128,"domain_data_max_bytes":1000,
                    "multipart":{"enabled":true,"part_size_bytes":5}
                }}))
            }),
        )
        .route(
            &format!("/api/v1/domains/{domain}/data/multipart"),
            post(multipart_post).put(multipart_put),
        )
        .with_state(state.clone());
    let stop = CancellationToken::new();
    let server = tokio::spawn(
        axum::serve(listener, app)
            .with_graceful_shutdown(stop.clone().cancelled_owned())
            .into_future(),
    );
    let client = DomainClient::new(base.parse().unwrap(), token).unwrap();
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        client.upload_artifact(UploadRequest {
            domain_id: &domain.to_string(),
            name: "big.bin",
            data_type: "binary",
            logical_path: "out/big.bin",
            bytes: &[42; 12],
            existing_id: None,
        }),
    )
    .await;
    stop.cancel();
    server.await.unwrap().unwrap();
    assert_eq!(result.unwrap().unwrap(), Some(state.id.to_string()));
    assert_eq!(state.parts.lock().unwrap().concat(), [42; 12]);
}
