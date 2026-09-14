//! Profile-correct credentials supplied only by the local mock DDS.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use httpmock::prelude::*;
use serde_json::{json, Value};
use uuid::Uuid;

fn token(claims: Value) -> String {
    format!(
        "e30.{}.local-fixture",
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap())
    )
}

pub fn data_token(base: &str, domain: Uuid, expires: DateTime<Utc>, generation: &str) -> String {
    token(json!({"iss":"dds", "aud":[base], "domain_id":domain,
        "exp":expires.timestamp(), "jti":generation}))
}

pub fn robot_token(
    base: &str,
    robot: Uuid,
    domain: Uuid,
    expires: DateTime<Utc>,
    generation: &str,
    peer: Option<String>,
) -> String {
    token(json!({"iss":"dds", "aud":[format!("{base}/robots")],
        "node_type":"robot", "node_mode":"dedicated", "sub":robot, "node_id":robot,
        "organization_id": Uuid::from_u128(1), "assigned_domain_id":domain,
        "iat": Utc::now().timestamp(), "exp":expires.timestamp(), "jti":generation, "peer_id":peer}))
}

pub fn metadata(domain: Uuid, id: Uuid, name: &str, data_type: &str, size: usize) -> Value {
    json!({"id":id,"domain_id":domain,"name":name,"data_type":data_type,"size":size,
        "created_at":"2026-09-01T00:00:00Z","updated_at":"2026-09-01T00:00:00Z"})
}

pub fn info(server: &MockServer) {
    server.mock(|when, then| {
        when.method(GET).path("/api/v1/info");
        then.header("content-type", "application/json").json_body(
            json!({"upload":{"domain_data_max_bytes":67108864,"request_max_bytes":134217728}}),
        );
    });
}
