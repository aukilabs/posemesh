use crate::errors::StorageError;
use crate::storage::token::TokenRef;
use anyhow::Result;
use futures::StreamExt;
use regex::Regex;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;
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

#[derive(Debug, Clone)]
struct UploadInfoV1 {
    request_max_bytes: i64,
    multipart_enabled: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct InfoResponseV1 {
    upload: InfoUploadV1,
}

#[derive(Debug, Deserialize, Clone)]
struct InfoUploadV1 {
    request_max_bytes: i64,
    multipart: InfoMultipartV1,
}

#[derive(Debug, Deserialize, Clone)]
struct InfoMultipartV1 {
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct InitiateMultipartRequestV1 {
    name: String,
    data_type: String,
    size: Option<i64>,
    content_type: Option<String>,
    existing_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InitiateMultipartResponseV1 {
    upload_id: String,
    part_size: i64,
}

#[derive(Debug, Deserialize)]
struct UploadPartResultV1 {
    etag: String,
}

#[derive(Debug, Serialize)]
struct CompletedPartV1 {
    part_number: i32,
    etag: String,
}

#[derive(Debug, Serialize)]
struct CompleteMultipartRequestV1 {
    parts: Vec<CompletedPartV1>,
}

#[derive(Debug, Deserialize)]
struct DomainDataMetadataV1 {
    id: String,
}

#[derive(Debug, Clone)]
struct InfoCacheEntryV1 {
    value: Option<UploadInfoV1>,
    expires_at: u64,
}

const INFO_CACHE_TTL_SECS: u64 = 60;
static INFO_CACHE_V1: OnceLock<parking_lot::Mutex<HashMap<String, InfoCacheEntryV1>>> =
    OnceLock::new();

fn info_cache_v1() -> &'static parking_lot::Mutex<HashMap<String, InfoCacheEntryV1>> {
    INFO_CACHE_V1.get_or_init(|| parking_lot::Mutex::new(HashMap::new()))
}

fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
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

        let base = self.base.as_str().trim_end_matches('/');
        if let Some(info) = get_upload_info_v1(base).await {
            if info.multipart_enabled && info.request_max_bytes > 0 {
                let fits_alone = fits_single_upload_request(
                    info.request_max_bytes,
                    request.name,
                    request.data_type,
                    request.existing_id,
                    request.bytes.len(),
                );
                if !fits_alone {
                    return self
                        .upload_artifact_v1_multipart(base, domain_id, request)
                        .await;
                }
            }
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

    async fn upload_artifact_v1_multipart(
        &self,
        base: &str,
        domain_id: &str,
        request: UploadRequest<'_>,
    ) -> std::result::Result<Option<String>, StorageError> {
        if request.bytes.is_empty() {
            return Err(StorageError::BadRequest);
        }

        let client = reqwest::Client::new();
        let initiate_endpoint = format!(
            "{}/api/v1/domains/{}/data/multipart?uploads",
            base, domain_id
        );

        let init_req = InitiateMultipartRequestV1 {
            name: request.name.to_string(),
            data_type: request.data_type.to_string(),
            size: Some(request.bytes.len() as i64),
            content_type: Some("application/octet-stream".to_string()),
            existing_id: request.existing_id.map(|id| id.to_string()),
        };

        tracing::debug!(
            target: "posemesh_compute_node::storage::client",
            method = "POST",
            url = %initiate_endpoint,
            logical_path = request.logical_path,
            name = request.name,
            data_type = request.data_type,
            has_existing_id = request.existing_id.is_some(),
            "Initiating multipart upload"
        );

        let init_resp = client
            .post(&initiate_endpoint)
            .bearer_auth(self.token.get())
            .header("posemesh-client-id", self.client_id.as_str())
            .header("Content-Type", "application/json")
            .json(&init_req)
            .send()
            .await
            .map_err(|e| StorageError::Network(e.to_string()))?;

        if !init_resp.status().is_success() {
            let status = init_resp.status();
            let err = init_resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            tracing::warn!(
                target: "posemesh_compute_node::storage::client",
                %status,
                error = %err,
                "Multipart initiation failed"
            );
            return Err(map_status(status));
        }

        let init: InitiateMultipartResponseV1 = init_resp
            .json()
            .await
            .map_err(|e| StorageError::Other(format!("invalid initiate response: {}", e)))?;

        let part_size = usize::try_from(init.part_size)
            .map_err(|_| StorageError::Other("invalid multipart part_size".into()))?;
        if part_size == 0 {
            return Err(StorageError::Other("invalid multipart part_size".into()));
        }

        let upload_id = init.upload_id;
        let mut completed_parts: Vec<CompletedPartV1> = Vec::new();

        let upload_res: std::result::Result<DomainDataMetadataV1, StorageError> = async {
            let mut offset: usize = 0;
            let mut part_number: i32 = 1;
            while offset < request.bytes.len() {
                let end = std::cmp::min(offset + part_size, request.bytes.len());
                let chunk = request.bytes[offset..end].to_vec();

                let part_endpoint = format!(
                    "{}/api/v1/domains/{}/data/multipart?uploadId={}&partNumber={}",
                    base, domain_id, upload_id, part_number
                );
                tracing::debug!(
                    target: "posemesh_compute_node::storage::client",
                    method = "PUT",
                    url = %part_endpoint,
                    part_number,
                    part_bytes = chunk.len(),
                    "Uploading multipart part"
                );

                let resp = client
                    .put(&part_endpoint)
                    .bearer_auth(self.token.get())
                    .header("posemesh-client-id", self.client_id.as_str())
                    .header("Content-Type", "application/octet-stream")
                    .body(chunk)
                    .send()
                    .await
                    .map_err(|e| StorageError::Network(e.to_string()))?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let err = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());
                    tracing::warn!(
                        target: "posemesh_compute_node::storage::client",
                        %status,
                        error = %err,
                        part_number,
                        "Multipart part upload failed"
                    );
                    return Err(map_status(status));
                }

                let res: UploadPartResultV1 = resp
                    .json()
                    .await
                    .map_err(|e| StorageError::Other(format!("invalid part response: {}", e)))?;

                completed_parts.push(CompletedPartV1 {
                    part_number,
                    etag: res.etag,
                });

                offset = end;
                part_number = part_number
                    .checked_add(1)
                    .ok_or_else(|| StorageError::Other("multipart upload too many parts".into()))?;
            }

            let complete_endpoint = format!(
                "{}/api/v1/domains/{}/data/multipart?uploadId={}",
                base, domain_id, upload_id
            );
            tracing::debug!(
                target: "posemesh_compute_node::storage::client",
                method = "POST",
                url = %complete_endpoint,
                parts = completed_parts.len(),
                "Completing multipart upload"
            );
            let resp = client
                .post(&complete_endpoint)
                .bearer_auth(self.token.get())
                .header("posemesh-client-id", self.client_id.as_str())
                .header("Content-Type", "application/json")
                .json(&CompleteMultipartRequestV1 {
                    parts: completed_parts,
                })
                .send()
                .await
                .map_err(|e| StorageError::Network(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let err = resp
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                tracing::warn!(
                    target: "posemesh_compute_node::storage::client",
                    %status,
                    error = %err,
                    "Multipart completion failed"
                );
                return Err(map_status(status));
            }

            resp.json::<DomainDataMetadataV1>()
                .await
                .map_err(|e| StorageError::Other(format!("invalid complete response: {}", e)))
        }
        .await;

        if upload_res.is_err() {
            let abort_endpoint = format!(
                "{}/api/v1/domains/{}/data/multipart?uploadId={}",
                base, domain_id, upload_id
            );
            let _ = client
                .delete(&abort_endpoint)
                .bearer_auth(self.token.get())
                .header("posemesh-client-id", self.client_id.as_str())
                .send()
                .await;
        }

        upload_res.map(|meta| Some(meta.id))
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

async fn fetch_info_v1(base: &str) -> Result<Option<UploadInfoV1>, ()> {
    let resp = reqwest::Client::new()
        .get(&format!("{}/api/v1/info", base))
        .send()
        .await
        .map_err(|_| ())?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !resp.status().is_success() {
        return Err(());
    }

    let info = resp.json::<InfoResponseV1>().await.map_err(|_| ())?;
    Ok(Some(UploadInfoV1 {
        request_max_bytes: info.upload.request_max_bytes,
        multipart_enabled: info.upload.multipart.enabled,
    }))
}

async fn get_upload_info_v1(base: &str) -> Option<UploadInfoV1> {
    let now = now_unix_secs();
    {
        let cache = info_cache_v1().lock();
        if let Some(entry) = cache.get(base) {
            if entry.expires_at > now {
                return entry.value.clone();
            }
        }
    }

    let fetched = match fetch_info_v1(base).await {
        Ok(v) => v,
        Err(_) => return None,
    };

    let mut cache = info_cache_v1().lock();
    cache.retain(|_, entry| entry.expires_at > now);
    cache.insert(
        base.to_string(),
        InfoCacheEntryV1 {
            value: fetched.clone(),
            expires_at: now.saturating_add(INFO_CACHE_TTL_SECS),
        },
    );
    fetched
}

fn fits_single_upload_request(
    request_max_bytes: i64,
    name: &str,
    data_type: &str,
    existing_id: Option<&str>,
    data_len: usize,
) -> bool {
    let boundary = "boundary";
    let header = if let Some(id) = existing_id {
        format!(
            "--{}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: form-data; id=\"{}\"\r\n\r\n",
            boundary, id
        )
    } else {
        format!(
            "--{}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: form-data; name=\"{}\"; data-type=\"{}\"\r\n\r\n",
            boundary, name, data_type
        )
    };
    let closing = format!("--{}--\r\n", boundary);
    let part_len = header.as_bytes().len() + data_len + 2;
    (part_len + closing.len()) as i64 <= request_max_bytes
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
