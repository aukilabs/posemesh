#[cfg(test)]
use crate::state::clear_node_secret;
use crate::state::{read_node_secret, touch_healthcheck_now, write_node_secret};
use axum::extract::State;
use axum::http::{header::USER_AGENT, HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

#[derive(Clone)]
pub struct DdsState;

#[derive(Debug, Deserialize)]
pub struct RegistrationCallbackRequest {
    pub id: String,
    pub secret: String,
    pub organization_id: Option<String>,
    pub lighthouses_in_domains: Option<serde_json::Value>,
    pub domains: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct OkResponse {
    ok: bool,
}

#[derive(Debug)]
enum CallbackError {
    Unprocessable(&'static str), // 422
    Forbidden(&'static str),     // 403
    Conflict(&'static str),      // 409
}

impl IntoResponse for CallbackError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match self {
            CallbackError::Unprocessable(m) => (StatusCode::UNPROCESSABLE_ENTITY, m),
            CallbackError::Forbidden(m) => (StatusCode::FORBIDDEN, m),
            CallbackError::Conflict(m) => (StatusCode::CONFLICT, m),
        };
        (status, msg).into_response()
    }
}

pub fn router_dds(state: DdsState) -> axum::Router {
    axum::Router::new()
        .route("/internal/v1/registrations", post(callback_registration))
        .route("/health", get(health))
        .with_state(state)
}

async fn health(State(_state): State<DdsState>, headers: HeaderMap) -> impl IntoResponse {
    let ua = headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if ua.starts_with("DDS v") {
        match touch_healthcheck_now() {
            Ok(()) => debug!(
                event = "healthcheck.touch",
                user_agent = ua,
                "last_healthcheck updated via /health"
            ),
            Err(e) => {
                warn!(event = "healthcheck.touch.error", user_agent = ua, error = %e, "failed to update last_healthcheck")
            }
        }
    } else {
        debug!(
            event = "healthcheck.skip",
            user_agent = ua,
            "health check not from DDS; not updating last_healthcheck"
        );
    }
    StatusCode::OK
}

async fn callback_registration(
    State(_state): State<DdsState>,
    Json(payload): Json<RegistrationCallbackRequest>,
) -> Result<Json<OkResponse>, CallbackError> {
    // Basic shape validation
    if payload.id.trim().is_empty() {
        return Err(CallbackError::Unprocessable("missing id"));
    }
    if payload.secret.trim().is_empty() {
        return Err(CallbackError::Unprocessable("missing secret"));
    }

    // Optional: enforce some maximum size to avoid abuse
    if payload.secret.len() > 4096 {
        return Err(CallbackError::Forbidden("secret too large"));
    }

    // Log without exposing sensitive secret
    let secret_len = payload.secret.len();
    let org = payload.organization_id.as_deref().unwrap_or("");
    info!(id = %payload.id, org = %org, secret_len = secret_len, "Received registration callback");

    // Persist atomically
    write_node_secret(&payload.secret).map_err(|_| CallbackError::Conflict("persist failed"))?;

    // Sanity read-back (optional; not exposing value)
    match read_node_secret() {
        Ok(Some(_)) => {}
        Ok(None) => {
            warn!("persisted secret missing after write");
            return Err(CallbackError::Conflict("persist verify failed"));
        }
        Err(_) => {
            return Err(CallbackError::Conflict("persist verify failed"));
        }
    }

    Ok(Json(OkResponse { ok: true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use parking_lot::Mutex as PLMutex;
    use std::io;
    use std::sync::Arc;
    use tower::ServiceExt;
    use tracing::subscriber;
    use tracing_subscriber::layer::SubscriberExt;

    #[tokio::test]
    async fn callback_persists_and_redacts_secret() {
        struct BufWriter(Arc<PLMutex<Vec<u8>>>);
        impl io::Write for BufWriter {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.0.lock().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        struct MakeBufWriter(Arc<PLMutex<Vec<u8>>>);
        impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for MakeBufWriter {
            type Writer = BufWriter;
            fn make_writer(&'a self) -> Self::Writer {
                BufWriter(self.0.clone())
            }
        }

        let buf = Arc::new(PLMutex::new(Vec::<u8>::new()));
        let make = MakeBufWriter(buf.clone());
        let layer = tracing_subscriber::fmt::layer()
            .with_writer(make)
            .with_ansi(false)
            .without_time();
        let subscriber = tracing_subscriber::registry().with(layer);
        let _guard = subscriber::set_default(subscriber);

        clear_node_secret().unwrap();
        let app = router_dds(DdsState);

        let secret = "my-very-secret";
        let body = serde_json::json!({
            "id": "abc123",
            "secret": secret,
            "organization_id": "org1",
            "lighthouses_in_domains": [],
            "domains": []
        })
        .to_string();

        let req = Request::builder()
            .method("POST")
            .uri("/internal/v1/registrations")
            .header(axum::http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let got = read_node_secret().unwrap();
        assert_eq!(got.as_deref(), Some(secret));

        let captured = String::from_utf8(buf.lock().clone()).unwrap_or_default();
        assert!(captured.contains("Received registration callback"));
        assert!(
            !captured.contains(secret),
            "logs leaked secret: {}",
            captured
        );
    }

    #[tokio::test]
    async fn health_ok() {
        clear_node_secret().unwrap();
        let app = router_dds(DdsState);

        let req = Request::builder()
            .method("GET")
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
