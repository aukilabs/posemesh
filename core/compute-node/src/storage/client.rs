use crate::errors::StorageError;
use crate::storage::token::TokenRef;
use anyhow::{Context, Result};
use futures::StreamExt;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::{Client, Method, StatusCode};
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use url::Url;
use uuid::Uuid;
// zip extraction moved to reconstruction-specific runner

/// Representation of one multipart section downloaded from Domain.
#[derive(Debug, Clone)]
pub struct DownloadedPart {
    pub id: Option<String>,
    pub name: Option<String>,
    pub data_type: Option<String>,
    pub domain_id: Option<String>,
    pub path: PathBuf,
    pub root: PathBuf,
    pub relative_path: PathBuf,
    pub extracted_paths: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct UploadRequest<'a> {
    pub domain_id: &'a str,
    pub name: &'a str,
    pub data_type: &'a str,
    pub logical_path: &'a str,
    pub bytes: &'a [u8],
    pub existing_id: Option<&'a str>,
}

/// Domain server HTTP client (skeleton; HTTP added later).
#[derive(Clone)]
pub struct DomainClient {
    pub base: Url,
    pub token: TokenRef,
    http: Client,
}
impl DomainClient {
    pub fn new(base: Url, token: TokenRef) -> Result<Self> {
        let http = Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(30))
            .build()
            .context("build reqwest client")?;
        Ok(Self { base, token, http })
    }

    pub fn with_timeout(base: Url, token: TokenRef, timeout: Duration) -> Result<Self> {
        let http = Client::builder()
            .use_rustls_tls()
            .timeout(timeout)
            .build()
            .context("build reqwest client")?;
        Ok(Self { base, token, http })
    }

    fn auth_headers(&self) -> HeaderMap {
        let mut h = HeaderMap::new();
        let token = format!("Bearer {}", self.token.get());
        let mut v = HeaderValue::from_str(&token)
            .unwrap_or_else(|_| HeaderValue::from_static("Bearer INVALID"));
        v.set_sensitive(true);
        h.insert(AUTHORIZATION, v);
        h
    }

    /// Download a Domain data item referenced by an absolute URI, persisting each multipart
    /// part into a temporary file and returning its metadata.
    pub async fn download_uri(
        &self,
        uri: &str,
    ) -> std::result::Result<Vec<DownloadedPart>, StorageError> {
        let url =
            Url::parse(uri).map_err(|e| StorageError::Other(format!("parse domain uri: {}", e)))?;
        let url_for_log = url.clone();
        tracing::debug!(
            target: "posemesh_compute_node::storage::client",
            method = "GET",
            %url_for_log,
            "Sending domain request"
        );
        let client_id = std::env::var("CLIENT_ID")
            .unwrap_or_else(|_| format!("posemesh-compute-node/{}", uuid::Uuid::new_v4()));
        let res = posemesh_domain_http::domain_data::request_download_absolute(
            url.as_str(),
            &client_id,
            &self.token.get(),
        )
        .await
        .map_err(|e| StorageError::Network(e.to_string()))?;
        let status = res.status();
        tracing::debug!(
            target: "posemesh_compute_node::storage::client",
            method = "GET",
            %url_for_log,
            status = %status,
            "Domain response received"
        );
        if !status.is_success() {
            return Err(map_status(status));
        }

        let root = std::env::temp_dir().join(format!("domain-input-{}", Uuid::new_v4()));
        fs::create_dir_all(&root)
            .await
            .map_err(|e| StorageError::Other(format!("create download root: {}", e)))?;
        let datasets_root = root.join("datasets");
        fs::create_dir_all(&datasets_root)
            .await
            .map_err(|e| StorageError::Other(format!("create datasets root: {}", e)))?;

        let mut parts = Vec::new();
        let mut rx = posemesh_domain_http::domain_data::stream_from_response(res)
            .await
            .map_err(|e| StorageError::Other(e.to_string()))?;

        while let Some(item) = rx.next().await {
            let domain_item = item.map_err(|e| StorageError::Other(e.to_string()))?;
            let name = domain_item.metadata.name.clone();
            let data_type = domain_item.metadata.data_type.clone();

            let scan_folder = extract_timestamp(&name)
                .map(|ts| sanitize_component(&ts))
                .unwrap_or_else(|| sanitize_component(&name));
            let scan_dir = datasets_root.join(&scan_folder);
            fs::create_dir_all(&scan_dir)
                .await
                .map_err(|e| StorageError::Other(format!("create scan dir: {}", e)))?;

            let file_name = map_filename(&data_type, &name);
            let file_path = scan_dir.join(&file_name);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| StorageError::Other(format!("create parent dir: {}", e)))?;
            }
            fs::write(&file_path, &domain_item.data)
                .await
                .map_err(|e| StorageError::Other(format!("write temp file: {}", e)))?;

            let extracted_paths = Vec::new();

            let relative_path = file_path
                .strip_prefix(&root)
                .unwrap_or(&file_path)
                .to_path_buf();

            parts.push(DownloadedPart {
                id: Some(domain_item.metadata.id),
                name: Some(name),
                data_type: Some(data_type),
                domain_id: Some(domain_item.metadata.domain_id),
                path: file_path,
                root: root.clone(),
                relative_path,
                extracted_paths,
            });
        }

        if parts.is_empty() {
            return Err(StorageError::Other(
                "domain response did not contain any data parts".into(),
            ));
        }

        Ok(parts)
    }

    pub async fn upload_artifact(
        &self,
        request: UploadRequest<'_>,
    ) -> std::result::Result<Option<String>, StorageError> {
        let domain_id = request.domain_id.trim();
        if domain_id.is_empty() {
            return Err(StorageError::Other(
                "missing domain_id for artifact upload".into(),
            ));
        }
        let action = if let Some(id) = request.existing_id {
            posemesh_domain_http::domain_data::DomainAction::Update(
                posemesh_domain_http::domain_data::UpdateDomainData { id: id.to_string() },
            )
        } else {
            posemesh_domain_http::domain_data::DomainAction::Create(
                posemesh_domain_http::domain_data::CreateDomainData {
                    name: request.name.to_string(),
                    data_type: request.data_type.to_string(),
                },
            )
        };

        let upload = posemesh_domain_http::domain_data::UploadDomainData {
            action,
            data: request.bytes.to_vec(),
        };

        let base = self.base.as_str().trim_end_matches('/');
        let method = if matches!(
            upload.action,
            posemesh_domain_http::domain_data::DomainAction::Update(_)
        ) {
            Method::PUT
        } else {
            Method::POST
        };
        tracing::debug!(
            target: "posemesh_compute_node::storage::client",
            method = %method,
            url = %format!("{}/api/v1/domains/{}/data", base, domain_id),
            logical_path = request.logical_path,
            name = request.name,
            data_type = request.data_type,
            has_existing_id = request.existing_id.is_some(),
            "Sending domain upload request"
        );

        match posemesh_domain_http::domain_data::upload_one(
            base,
            &self.token.get(),
            domain_id,
            upload,
        )
        .await
        {
            Ok(mut items) => {
                let id = items.drain(..).next().map(|d| d.metadata.id);
                Ok(id)
            }
            Err((status, _body)) => Err(map_status(status)),
        }
    }

    pub async fn find_artifact_id(
        &self,
        domain_id: &str,
        name: &str,
        data_type: &str,
    ) -> std::result::Result<Option<String>, StorageError> {
        let domain_id = domain_id.trim();
        if domain_id.is_empty() {
            return Err(StorageError::Other(
                "missing domain_id for artifact lookup".into(),
            ));
        }
        let path = format!("api/v1/domains/{}/data", domain_id);
        let url = self
            .base
            .join(&path)
            .map_err(|e| StorageError::Other(format!("join lookup path: {}", e)))?;
        let mut headers = self.auth_headers();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        tracing::debug!(
            target: "posemesh_compute_node::storage::client",
            method = "GET",
            %url,
            artifact_name = name,
            artifact_type = data_type,
            "Looking up existing domain artifact"
        );
        let res = self
            .http
            .get(url.clone())
            .headers(headers)
            .query(&[("name", name), ("data_type", data_type)])
            .send()
            .await
            .map_err(|e| StorageError::Network(e.to_string()))?;
        let status = res.status();
        if status == StatusCode::NOT_FOUND {
            tracing::debug!(
                target: "posemesh_compute_node::storage::client",
                method = "GET",
                %url,
                artifact_name = name,
                artifact_type = data_type,
                "Artifact lookup returned 404"
            );
            return Ok(None);
        }
        if !status.is_success() {
            return Err(map_status(status));
        }
        let payload = res
            .json::<ListDomainDataResponse>()
            .await
            .map_err(|e| StorageError::Network(e.to_string()))?;
        let found = payload
            .data
            .into_iter()
            .find(|item| item.name == name && item.data_type == data_type);
        Ok(found.map(|item| item.id))
    }
}

#[derive(Debug, Deserialize)]
struct ListDomainDataResponse {
    #[serde(default)]
    data: Vec<DomainDataSummary>,
}

#[derive(Debug, Deserialize)]
struct DomainDataSummary {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    data_type: String,
}

fn map_status(status: reqwest::StatusCode) -> StorageError {
    match status.as_u16() {
        400 => StorageError::BadRequest,
        401 => StorageError::Unauthorized,
        404 => StorageError::NotFound,
        409 => StorageError::Conflict,
        n if (500..=599).contains(&n) => StorageError::Server(n),
        other => StorageError::Other(format!("unexpected status: {}", other)),
    }
}

// no parse_disposition_params; headers are parsed in posemesh-domain-http

fn sanitize_component(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "part".into()
    } else {
        sanitized
    }
}

fn extract_timestamp(name: &str) -> Option<String> {
    Regex::new(r"\d{4}-\d{2}-\d{2}[_-]\d{2}-\d{2}-\d{2}")
        .ok()
        .and_then(|re| re.find(name).map(|m| m.as_str().to_string()))
}

fn map_filename(data_type: &str, name: &str) -> String {
    format!(
        "{}.{}",
        sanitize_component(name),
        sanitize_component(data_type)
    )
}
