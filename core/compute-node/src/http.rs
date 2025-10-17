use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::dds::persist;

async fn health() -> impl IntoResponse {
    StatusCode::OK
}

#[derive(Debug, Deserialize)]
struct RegistrationRequest {
    id: String,
    secret: String,
    organization_id: Option<String>,
    #[serde(rename = "lighthouses_in_domains")]
    _lighthouses_in_domains: Option<serde_json::Value>,
    #[serde(rename = "domains")]
    _domains: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct RegistrationResponse {
    ok: bool,
}

#[derive(Debug)]
enum RegistrationError {
    Unprocessable(&'static str),
    Forbidden(&'static str),
    Conflict(&'static str),
}

impl IntoResponse for RegistrationError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match self {
            RegistrationError::Unprocessable(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            RegistrationError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            RegistrationError::Conflict(msg) => (StatusCode::CONFLICT, msg),
        };
        (status, msg).into_response()
    }
}

async fn register(
    Json(payload): Json<RegistrationRequest>,
) -> Result<Json<RegistrationResponse>, RegistrationError> {
    if payload.id.trim().is_empty() {
        return Err(RegistrationError::Unprocessable("missing id"));
    }
    if payload.secret.trim().is_empty() {
        return Err(RegistrationError::Unprocessable("missing secret"));
    }
    if payload.secret.len() > 4096 {
        return Err(RegistrationError::Forbidden("secret too large"));
    }

    let secret_len = payload.secret.len();
    let org = payload.organization_id.as_deref().unwrap_or("");
    info!(
        id = %payload.id,
        org = %org,
        secret_len,
        "Received registration callback"
    );

    persist::write_node_secret(&payload.secret)
        .map_err(|_| RegistrationError::Conflict("persist failed"))?;

    match persist::read_node_secret() {
        Ok(Some(_)) => {}
        Ok(None) => {
            warn!(
                id = %payload.id,
                "persisted secret missing after write"
            );
            return Err(RegistrationError::Conflict("persist verify failed"));
        }
        Err(_) => {
            return Err(RegistrationError::Conflict("persist verify failed"));
        }
    }

    Ok(Json(RegistrationResponse { ok: true }))
}

/// Build the node HTTP router exposing /health and internal registration endpoint.
pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/internal/v1/registrations", post(register))
}
