use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
#[cfg(not(target_family = "wasm"))]
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessDomainRequest {
    pub data_ids: Vec<String>,
    #[serde(default = "default_processing_type")]
    pub processing_type: Option<String>,
    #[serde(default = "default_api_key")]
    pub server_api_key: Option<String>,
    pub server_url: String,
}

fn default_processing_type() -> Option<String> {
    Some("local_and_global_refinement".to_string())
}

fn default_api_key() -> Option<String> {
    Some("aukilabs123".to_string())
}

pub async fn process_domain_v1(
    url: &str,
    client_id: &str,
    access_token: &str,
    domain_id: &str,
    request: &ProcessDomainRequest,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let request_with_defaults = ProcessDomainRequest {
        data_ids: request.data_ids.clone(),
        processing_type: Some(
            request
                .processing_type
                .clone()
                .unwrap_or_else(|| "local_and_global_refinement".to_string()),
        ),
        server_api_key: Some(
            request
                .server_api_key
                .clone()
                .unwrap_or_else(|| "aukilabs123".to_string()),
        ),
        server_url: request.server_url.clone(),
    };

    let response = Client::new()
        .post(&format!("{}/api/v1/domains/{}/process", url, domain_id))
        .bearer_auth(access_token)
        .header("posemesh-client-id", client_id)
        .json(&request_with_defaults)
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
        Err(format!("Failed to process domain. Status: {} - {}", status, text).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_domain_request_structure() {
        let request = ProcessDomainRequest {
            data_ids: vec!["test-id-1".to_string(), "test-id-2".to_string()],
            processing_type: None,
            server_api_key: None,
            server_url: "https://example.com".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test-id-1"));
        assert!(json.contains("https://example.com"));

        let deserialized: ProcessDomainRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.processing_type, Some("local_and_global_refinement".to_string()));
        assert_eq!(deserialized.data_ids.len(), 2);
        assert_eq!(deserialized.server_api_key, Some("aukilabs123".to_string()));
    }
}
