use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
#[cfg(not(target_family = "wasm"))]
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

use crate::errors::{AukiErrorResponse, DomainError};

#[derive(Debug, Serialize, Deserialize)]
pub struct JobRequest {
    pub data_ids: Vec<String>,
    #[serde(default = "default_processing_type")]
    pub processing_type: String,
    #[serde(default = "default_api_key")]
    pub server_api_key: String,
    pub server_url: String,
}

fn default_processing_type() -> String {
    "local_and_global_refinement".to_string()
}

fn default_api_key() -> String {
    "aukilabs123".to_string()
}

impl Default for JobRequest {
    fn default() -> Self {
        Self {
            data_ids: vec![],
            processing_type: default_processing_type(),
            server_api_key: default_api_key(),
            server_url: "".to_string(),
        }
    }
}

pub async fn forward_job_request_v1(
    domain_server_url: &str,
    client_id: &str,
    access_token: &str,
    domain_id: &str,
    request: &JobRequest,
) -> Result<Response, DomainError> {
    let response = Client::new()
        .post(&format!("{}/api/v1/domains/{}/process", domain_server_url, domain_id))
        .bearer_auth(access_token)
        .header("posemesh-client-id", client_id)
        .json(&request)
        .send()
        .await?;

    if response.status().is_success() {
        Ok(response)
    } else {
        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(AukiErrorResponse { status, error: format!("Failed to process domain. {}", text) }.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_request_structure() {
        let json = r#"{"data_ids":["test-id-1", "test-id-2"], "server_url": "https://example.com"}"#;
        assert!(json.contains("test-id-1"));
        assert!(json.contains("https://example.com"));

        let deserialized: JobRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.data_ids.len(), 2);
        assert_eq!(deserialized.processing_type, "local_and_global_refinement");
        assert_eq!(deserialized.server_api_key, "aukilabs123");
        assert_eq!(deserialized.server_url, "https://example.com")}
}
