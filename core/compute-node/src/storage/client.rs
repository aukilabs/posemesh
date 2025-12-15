use crate::errors::StorageError;
use crate::storage::token::TokenRef;
use anyhow::Result;
use futures::StreamExt;
use regex::Regex;
use reqwest::Method;
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
    client_id: String,
}
impl DomainClient {
    pub fn new(base: Url, token: TokenRef) -> Result<Self> {
        let client_id = env_client_id();
        Ok(Self {
            base,
            token,
            client_id,
        })
    }

    pub fn with_timeout(base: Url, token: TokenRef, _timeout: Duration) -> Result<Self> {
        let client_id = env_client_id();
        Ok(Self {
            base,
            token,
            client_id,
        })
    }

    /// Download a Domain data item referenced by an absolute URI, persisting each multipart
    /// part into a temporary file and returning its metadata.
    pub async fn download_uri(
        &self,
        uri: &str,
    ) -> std::result::Result<Vec<DownloadedPart>, StorageError> {
        let resolved = resolve_domain_url(&self.base, uri)?;
        let (domain_id, query) = parse_download_target(&resolved, None)?;
        self.download_domain_data(&resolved, &domain_id, query)
            .await
    }

    /// Download domain data referenced by a CID, which can be either:
    /// - a bare domain-data ID (UUID), or
    /// - an absolute/relative URL under the domain server.
    pub async fn download_cid(
        &self,
        domain_id: &str,
        cid: &str,
    ) -> std::result::Result<Vec<DownloadedPart>, StorageError> {
        let cid = cid.trim();
        if cid.is_empty() {
            return Err(StorageError::Other("empty cid".into()));
        }

        if cid.contains("://") || cid.starts_with('/') {
            let resolved = resolve_domain_url(&self.base, cid)?;
            let (domain_id, query) = parse_download_target(&resolved, Some(domain_id))?;
            return self
                .download_domain_data(&resolved, &domain_id, query)
                .await;
        }

        let query = posemesh_domain_http::domain_data::DownloadQuery {
            ids: vec![cid.to_string()],
            name: None,
            data_type: None,
        };
        self.download_domain_data(&self.base, domain_id, query)
            .await
    }

    async fn download_domain_data(
        &self,
        url_for_log: &Url,
        domain_id: &str,
        query: posemesh_domain_http::domain_data::DownloadQuery,
    ) -> std::result::Result<Vec<DownloadedPart>, StorageError> {
        let domain_id = domain_id.trim();
        if domain_id.is_empty() {
            return Err(StorageError::Other("missing domain_id for download".into()));
        }

        tracing::debug!(
            target: "posemesh_compute_node::storage::client",
            method = "GET",
            %url_for_log,
            domain_id = domain_id,
            ids = ?query.ids,
            name = ?query.name,
            data_type = ?query.data_type,
            "Downloading domain data"
        );

        let base = self.base.as_str().trim_end_matches('/');
        let mut rx = posemesh_domain_http::domain_data::download_v1_stream(
            base,
            self.client_id.as_str(),
            self.token.get().as_str(),
            domain_id,
            &query,
        )
        .await
        .map_err(map_domain_error)?;

        let root = std::env::temp_dir().join(format!("domain-input-{}", Uuid::new_v4()));
        fs::create_dir_all(&root)
            .await
            .map_err(|e| StorageError::Other(format!("create download root: {}", e)))?;
        let datasets_root = root.join("datasets");
        fs::create_dir_all(&datasets_root)
            .await
            .map_err(|e| StorageError::Other(format!("create datasets root: {}", e)))?;

        let mut parts = Vec::new();

        while let Some(item) = rx.next().await {
            let domain_item = item.map_err(map_domain_error)?;
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
            return Err(StorageError::NotFound);
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
            posemesh_domain_http::domain_data::DomainAction::Update { id: id.to_string() }
        } else {
            posemesh_domain_http::domain_data::DomainAction::Create {
                name: request.name.to_string(),
                data_type: request.data_type.to_string(),
            }
        };

        let upload = posemesh_domain_http::domain_data::UploadDomainData {
            action,
            data: request.bytes.to_vec(),
        };

        let base = self.base.as_str().trim_end_matches('/');
        let method = if request.existing_id.is_some() {
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

        let items = posemesh_domain_http::domain_data::upload_v1(
            base,
            self.token.get().as_str(),
            domain_id,
            vec![upload],
        )
        .await
        .map_err(map_domain_error)?;

        Ok(items.into_iter().next().map(|d| d.id))
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

        let query = posemesh_domain_http::domain_data::DownloadQuery {
            ids: Vec::new(),
            name: Some(name.to_string()),
            data_type: Some(data_type.to_string()),
        };

        let base = self.base.as_str().trim_end_matches('/');
        let url = format!("{}/api/v1/domains/{}/data", base, domain_id);
        tracing::debug!(
            target: "posemesh_compute_node::storage::client",
            method = "GET",
            %url,
            artifact_name = name,
            artifact_type = data_type,
            "Looking up existing domain artifact"
        );

        let results = posemesh_domain_http::domain_data::download_metadata_v1(
            base,
            self.client_id.as_str(),
            self.token.get().as_str(),
            domain_id,
            &query,
        )
        .await;

        let results = match results {
            Ok(items) => items,
            Err(err) => {
                if let posemesh_domain_http::errors::DomainError::AukiErrorResponse(resp) = &err {
                    if resp.status == reqwest::StatusCode::NOT_FOUND {
                        return Ok(None);
                    }
                }
                return Err(map_domain_error(err));
            }
        };

        Ok(results
            .into_iter()
            .find(|item| item.name == name && item.data_type == data_type)
            .map(|item| item.id))
    }
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

fn map_domain_error(err: posemesh_domain_http::errors::DomainError) -> StorageError {
    use posemesh_domain_http::errors::{AuthError, DomainError};

    match err {
        DomainError::AukiErrorResponse(resp) => map_status(resp.status),
        DomainError::ReqwestError(e) => StorageError::Network(e.to_string()),
        DomainError::AuthError(AuthError::Unauthorized(_)) => StorageError::Unauthorized,
        other => StorageError::Other(other.to_string()),
    }
}

fn env_client_id() -> String {
    std::env::var("CLIENT_ID")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| format!("posemesh-compute-node/{}", Uuid::new_v4()))
}

fn resolve_domain_url(base: &Url, value: &str) -> std::result::Result<Url, StorageError> {
    if value.contains("://") {
        Url::parse(value).map_err(|e| StorageError::Other(format!("parse domain url: {e}")))
    } else {
        base.join(value)
            .map_err(|e| StorageError::Other(format!("join domain url: {e}")))
    }
}

fn parse_download_target(
    url: &Url,
    fallback_domain_id: Option<&str>,
) -> std::result::Result<(String, posemesh_domain_http::domain_data::DownloadQuery), StorageError> {
    let segments: Vec<&str> = url
        .path_segments()
        .map(|segments| segments.filter(|seg| !seg.is_empty()).collect())
        .unwrap_or_default();

    let mut domain_id_from_path: Option<&str> = None;
    let mut data_id_from_path: Option<&str> = None;

    for idx in 0..segments.len() {
        if segments[idx] == "domains" && idx + 2 < segments.len() && segments[idx + 2] == "data" {
            domain_id_from_path = Some(segments[idx + 1]);
            data_id_from_path = segments.get(idx + 3).copied();
            break;
        }
    }

    let domain_id = domain_id_from_path
        .or(fallback_domain_id)
        .ok_or_else(|| StorageError::Other(format!("cid url missing domain_id: {}", url)))?
        .to_string();

    let mut ids: Vec<String> = Vec::new();
    let mut name: Option<String> = None;
    let mut data_type: Option<String> = None;

    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "ids" => ids.extend(
                value
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string()),
            ),
            "name" => {
                if name.is_none() {
                    name = Some(value.to_string());
                }
            }
            "data_type" => {
                if data_type.is_none() {
                    data_type = Some(value.to_string());
                }
            }
            _ => {}
        }
    }

    if let Some(id) = data_id_from_path {
        ids = vec![id.to_string()];
    }

    Ok((
        domain_id,
        posemesh_domain_http::domain_data::DownloadQuery {
            ids,
            name,
            data_type,
        },
    ))
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
