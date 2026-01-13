use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use posemesh_compute_node::storage::client::{DomainClient, UploadRequest};
use posemesh_compute_node::storage::TokenRef;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex};

#[derive(Debug, Deserialize)]
struct MultipartQuery {
    #[serde(default)]
    uploads: Option<String>,
    #[serde(default, rename = "uploadId")]
    upload_id: Option<String>,
    #[serde(default, rename = "partNumber")]
    part_number: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct InitiateMultipartRequest {
    name: String,
    data_type: String,
    size: Option<i64>,
    content_type: Option<String>,
    existing_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct InitiateMultipartResponse {
    upload_id: String,
    part_size: i64,
}

#[derive(Debug, Serialize)]
struct UploadPartResult {
    etag: String,
}

#[derive(Debug, Deserialize)]
struct CompleteMultipartRequest {
    parts: Vec<CompletedPart>,
}

#[derive(Debug, Deserialize)]
struct CompletedPart {
    part_number: i32,
    etag: String,
}

#[derive(Debug, Serialize)]
struct DomainDataMetadata {
    id: String,
    domain_id: String,
    name: String,
    data_type: String,
    size: i64,
    created_at: String,
    updated_at: String,
}

#[derive(Clone)]
struct ServerState {
    init_seen_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    allow_init_rx: Arc<Mutex<Option<oneshot::Receiver<()>>>>,
}

fn bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_string())
}

#[tokio::test]
async fn multipart_upload_uses_latest_token_after_rotation() {
    let (init_seen_tx, init_seen_rx) = oneshot::channel::<()>();
    let (allow_init_tx, allow_init_rx) = oneshot::channel::<()>();
    let state = ServerState {
        init_seen_tx: Arc::new(Mutex::new(Some(init_seen_tx))),
        allow_init_rx: Arc::new(Mutex::new(Some(allow_init_rx))),
    };

    async fn info() -> impl IntoResponse {
        // Force multipart by advertising a tiny request limit.
        let resp = serde_json::json!({
            "upload": {
                "request_max_bytes": 1,
                "multipart": { "enabled": true }
            }
        });
        (StatusCode::OK, Json(resp))
    }

    async fn multipart_post(
        State(state): State<ServerState>,
        Path(domain_id): Path<String>,
        Query(q): Query<MultipartQuery>,
        headers: HeaderMap,
        body: Bytes,
    ) -> impl IntoResponse {
        if domain_id != "dom1" {
            return (StatusCode::NOT_FOUND, "unknown domain").into_response();
        }

        if q.uploads.is_some() {
            if bearer(&headers).as_deref() != Some("Bearer tA") {
                return (StatusCode::UNAUTHORIZED, "bad token for initiate").into_response();
            }
            let req: InitiateMultipartRequest =
                serde_json::from_slice(&body).expect("initiate request json");
            assert_eq!(req.name, "big.bin");
            assert_eq!(req.data_type, "binary");
            assert_eq!(req.size, Some(12));
            assert_eq!(
                req.content_type.as_deref(),
                Some("application/octet-stream")
            );
            assert!(req.existing_id.is_none());

            if let Some(tx) = state.init_seen_tx.lock().await.take() {
                let _ = tx.send(());
            }
            if let Some(rx) = state.allow_init_rx.lock().await.take() {
                let _ = rx.await;
            }

            return (
                StatusCode::OK,
                Json(InitiateMultipartResponse {
                    upload_id: "up1".into(),
                    part_size: 5,
                }),
            )
                .into_response();
        }

        if q.upload_id.as_deref() == Some("up1") {
            if bearer(&headers).as_deref() != Some("Bearer tB") {
                return (StatusCode::UNAUTHORIZED, "bad token for complete").into_response();
            }
            let req: CompleteMultipartRequest =
                serde_json::from_slice(&body).expect("complete request json");
            assert_eq!(req.parts.len(), 3);
            for (idx, part) in req.parts.iter().enumerate() {
                assert_eq!(part.part_number, (idx + 1) as i32);
                assert_eq!(part.etag, format!("etag-{}", idx + 1));
            }
            return (
                StatusCode::OK,
                Json(DomainDataMetadata {
                    id: "data-123".into(),
                    domain_id: "dom1".into(),
                    name: "big.bin".into(),
                    data_type: "binary".into(),
                    size: 12,
                    created_at: "2025-01-01T00:00:00Z".into(),
                    updated_at: "2025-01-01T00:00:00Z".into(),
                }),
            )
                .into_response();
        }

        (StatusCode::BAD_REQUEST, "missing uploads or uploadId").into_response()
    }

    async fn multipart_put(
        Path(domain_id): Path<String>,
        Query(q): Query<MultipartQuery>,
        headers: HeaderMap,
        body: Bytes,
    ) -> impl IntoResponse {
        if domain_id != "dom1" || q.upload_id.as_deref() != Some("up1") {
            return (StatusCode::NOT_FOUND, "unknown upload").into_response();
        }
        if bearer(&headers).as_deref() != Some("Bearer tB") {
            return (StatusCode::UNAUTHORIZED, "bad token for part").into_response();
        }
        let part_no = q.part_number.unwrap_or_default();
        let expected_len = match part_no {
            1 | 2 => 5,
            3 => 2,
            _ => 0,
        };
        assert_eq!(body.len(), expected_len);
        (
            StatusCode::OK,
            Json(UploadPartResult {
                etag: format!("etag-{}", part_no),
            }),
        )
            .into_response()
    }

    async fn multipart_delete() -> impl IntoResponse {
        StatusCode::OK
    }

    let app = Router::new()
        .route("/api/v1/info", get(info))
        .route(
            "/api/v1/domains/:domain_id/data/multipart",
            post(multipart_post)
                .put(multipart_put)
                .delete(delete(multipart_delete)),
        )
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let token = TokenRef::new("tA".into());
    let base: url::Url = format!("http://{}", addr).parse().unwrap();
    let client = DomainClient::new(base, token.clone()).unwrap();

    let upload_future = tokio::spawn(async move {
        client
            .upload_artifact(UploadRequest {
                domain_id: "dom1",
                name: "big.bin",
                data_type: "binary",
                logical_path: "out/big.bin",
                bytes: &[42u8; 12],
                existing_id: None,
            })
            .await
    });

    // Wait for initiation request to be received, then rotate the token before
    // any part uploads begin.
    init_seen_rx.await.unwrap();
    token.swap("tB".into());
    allow_init_tx.send(()).unwrap();

    let result = upload_future.await.unwrap();
    assert!(result.is_ok(), "expected multipart upload to succeed");
    assert_eq!(result.unwrap().as_deref(), Some("data-123"));
}
