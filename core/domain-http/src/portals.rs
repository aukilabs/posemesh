use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::errors::{AukiErrorResponse, DomainError};

/// Matches Domain Server `LighthouseShortIDLength` (nanoid, 11 chars).
const LIGHTHOUSE_SHORT_ID_LENGTH: usize = 11;

/// Pose from Domain Server `GET /api/v1/domains/{domainID}/lighthouses`
/// (`view.PoseView` / `view.PoseListView`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pose {
    pub id: String,
    pub short_id: String,
    pub domain_id: String,
    pub reported_size: f64,
    pub px: f64,
    pub py: f64,
    pub pz: f64,
    pub rx: f64,
    pub ry: f64,
    pub rz: f64,
    pub rw: f64,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    pub vertical_accuracy: Option<f64>,
    pub horizontal_accuracy: Option<f64>,
    pub gps_timestamp: Option<f64>,
    pub scanner_device_id: String,
    pub scanner_device_name: String,
    pub scanner_device_model: String,
    pub placed_at: String,
}

#[derive(Debug, Deserialize)]
struct PoseListView {
    poses: Vec<Pose>,
}

/// DS looks up short ids case-insensitively after uppercasing.
pub(crate) fn lighthouse_path_id(lighthouse_id: &str) -> String {
    let id = lighthouse_id.trim();
    if id.len() == LIGHTHOUSE_SHORT_ID_LENGTH {
        id.to_uppercase()
    } else {
        id.to_string()
    }
}

pub async fn list_poses(
    domain_server_url: &str,
    access_token: &str,
    client_id: &str,
    domain_id: &str,
) -> Result<Vec<Pose>, DomainError> {
    let base = domain_server_url.trim_end_matches('/');
    let response = Client::new()
        .get(format!("{}/api/v1/domains/{}/lighthouses", base, domain_id))
        .bearer_auth(access_token)
        .header("Content-Type", "application/json")
        .header("posemesh-client-id", client_id)
        .header("posemesh-sdk-version", crate::VERSION)
        .send()
        .await?;

    if response.status().is_success() {
        let body: PoseListView = response.json().await?;
        Ok(body.poses)
    } else {
        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(AukiErrorResponse {
            status,
            error: format!("Failed to list poses. {}", text),
        }
        .into())
    }
}

pub async fn get_pose(
    domain_server_url: &str,
    access_token: &str,
    client_id: &str,
    domain_id: &str,
    lighthouse_id: &str,
) -> Result<Pose, DomainError> {
    let base = domain_server_url.trim_end_matches('/');
    let lighthouse_id = lighthouse_path_id(lighthouse_id);
    let response = Client::new()
        .get(format!(
            "{}/api/v1/domains/{}/lighthouses/{}",
            base, domain_id, lighthouse_id
        ))
        .bearer_auth(access_token)
        .header("Content-Type", "application/json")
        .header("posemesh-client-id", client_id)
        .header("posemesh-sdk-version", crate::VERSION)
        .send()
        .await?;

    if response.status().is_success() {
        Ok(response.json().await?)
    } else {
        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(AukiErrorResponse {
            status,
            error: format!("Failed to get pose. {}", text),
        }
        .into())
    }
}
