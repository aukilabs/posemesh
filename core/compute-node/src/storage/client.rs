use crate::errors::StorageError;
use crate::storage::token::TokenRef;
use anyhow::{Context, Result};
use multer::Multipart;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, Method};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use tokio::task;
use url::Url;
use uuid::Uuid;
use zip::ZipArchive;

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
        let mut headers = self.auth_headers();
        headers.insert(ACCEPT, HeaderValue::from_static("multipart/form-data"));
        tracing::debug!(
            target: "posemesh_compute_node::storage::client",
            method = "GET",
            %url_for_log,
            "Sending domain request"
        );
        let res = self
            .http
            .get(url)
            .headers(headers)
            .send()
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

        let content_type = res
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                StorageError::Other("missing Content-Type header on domain response".into())
            })?;
        let boundary = multer::parse_boundary(content_type).map_err(|e| {
            StorageError::Other(format!("invalid multipart boundary from domain: {}", e))
        })?;
        let mut multipart = Multipart::new(res.bytes_stream(), boundary);
        let mut parts = Vec::new();

        while let Some(mut field) = multipart
            .next_field()
            .await
            .map_err(|e| StorageError::Other(format!("read multipart field: {}", e)))?
        {
            let disposition = field
                .headers()
                .get("content-disposition")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default();
            let params = parse_disposition_params(disposition);
            let name = params
                .get("name")
                .cloned()
                .unwrap_or_else(|| "domain-data".into());
            let data_type = params.get("data-type").cloned().unwrap_or_default();

            let mut buf = Vec::new();
            while let Some(chunk) = field
                .chunk()
                .await
                .map_err(|e| StorageError::Other(format!("stream multipart chunk: {}", e)))?
            {
                buf.extend_from_slice(&chunk);
            }

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
            fs::write(&file_path, &buf)
                .await
                .map_err(|e| StorageError::Other(format!("write temp file: {}", e)))?;

            let mut extracted_paths = Vec::new();
            if data_type == "refined_scan_zip" {
                let unzip_root = root
                    .join("refined")
                    .join("local")
                    .join(&scan_folder)
                    .join("sfm");
                extracted_paths = unzip_refined_scan(buf.clone(), unzip_root).await?;
            }

            let relative_path = file_path
                .strip_prefix(&root)
                .unwrap_or(&file_path)
                .to_path_buf();

            parts.push(DownloadedPart {
                id: params.get("id").cloned(),
                name: Some(name),
                data_type: Some(data_type),
                domain_id: params.get("domain-id").cloned(),
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
        let path = format!("api/v1/domains/{}/data", domain_id);
        let url = self
            .base
            .join(&path)
            .map_err(|e| StorageError::Other(format!("join upload path: {}", e)))?;
        let boundary = format!("------------------------{}", Uuid::new_v4().simple());
        let (body, content_type) = build_multipart_body(
            &boundary,
            request.name,
            request.data_type,
            domain_id,
            request.existing_id,
            request.bytes,
        );
        let mut headers = self.auth_headers();
        let ct_value = HeaderValue::from_str(&content_type)
            .unwrap_or_else(|_| HeaderValue::from_static("multipart/form-data"));
        headers.insert(CONTENT_TYPE, ct_value);
        let method = if request.existing_id.is_some() {
            Method::PUT
        } else {
            Method::POST
        };
        tracing::debug!(
            target: "posemesh_compute_node::storage::client",
            method = %method,
            %url,
            logical_path = request.logical_path,
            name = request.name,
            data_type = request.data_type,
            has_existing_id = request.existing_id.is_some(),
            "Sending domain upload request"
        );
        let res = self
            .http
            .request(method.clone(), url.clone())
            .headers(headers)
            .body(body)
            .send()
            .await
            .map_err(|e| StorageError::Network(e.to_string()))?;
        let status = res.status();
        tracing::debug!(
            target: "posemesh_compute_node::storage::client",
            method = %method,
            %url,
            status = %status,
            "Domain upload response received"
        );
        if !status.is_success() {
            return Err(map_status(status));
        }
        let text = res
            .text()
            .await
            .map_err(|e| StorageError::Network(e.to_string()))?;
        if text.trim().is_empty() {
            return Ok(None);
        }
        match serde_json::from_str::<PostDomainDataResponse>(&text) {
            Ok(parsed) => {
                let id = parsed.data.into_iter().next().map(|d| d.id);
                Ok(id)
            }
            Err(err) => {
                tracing::debug!(
                    target: "posemesh_compute_node::storage::client",
                    error = %err,
                    body = %text,
                    "Failed to parse domain upload response body as JSON"
                );
                Ok(None)
            }
        }
    }
}

fn build_multipart_body(
    boundary: &str,
    name: &str,
    data_type: &str,
    domain_id: &str,
    existing_id: Option<&str>,
    bytes: &[u8],
) -> (Vec<u8>, String) {
    let mut body = Vec::with_capacity(bytes.len().saturating_add(256));
    let disposition = if let Some(id) = existing_id {
        format!(
            "Content-Disposition: form-data; name=\"{}\"; data-type=\"{}\"; id=\"{}\"; domain-id=\"{}\"\r\n",
            name, data_type, id, domain_id
        )
    } else {
        format!(
            "Content-Disposition: form-data; name=\"{}\"; data-type=\"{}\"; domain-id=\"{}\"\r\n",
            name, data_type, domain_id
        )
    };
    let header = format!(
        "--{}\r\nContent-Type: application/octet-stream\r\n{}\r\n",
        boundary, disposition
    );
    body.extend_from_slice(header.as_bytes());
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());
    let content_type = format!("multipart/form-data; boundary={}", boundary);
    (body, content_type)
}

#[derive(Debug, Deserialize)]
struct PostDomainDataResponse {
    #[serde(default)]
    data: Vec<PostDomainDataItem>,
}

#[derive(Debug, Deserialize)]
struct PostDomainDataItem {
    #[serde(default)]
    id: String,
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

fn parse_disposition_params(value: &str) -> HashMap<String, String> {
    value
        .split(';')
        .filter_map(|segment| {
            let trimmed = segment.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("form-data") {
                return None;
            }
            let (key, val) = trimmed.split_once('=')?;
            let cleaned = val.trim().trim_matches('"').to_string();
            Some((key.trim().to_ascii_lowercase(), cleaned))
        })
        .collect()
}

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
    match data_type {
        "dmt_manifest_json" => "Manifest.json".into(),
        "dmt_featurepoints_ply" | "dmt_pointcloud_ply" => "FeaturePoints.ply".into(),
        "dmt_arposes_csv" => "ARposes.csv".into(),
        "dmt_portal_detections_csv" | "dmt_observations_csv" => "PortalDetections.csv".into(),
        "dmt_intrinsics_csv" | "dmt_cameraintrinsics_csv" => "CameraIntrinsics.csv".into(),
        "dmt_frames_csv" => "Frames.csv".into(),
        "dmt_gyro_csv" => "Gyro.csv".into(),
        "dmt_accel_csv" => "Accel.csv".into(),
        "dmt_gyroaccel_csv" => "gyro_accel.csv".into(),
        "dmt_recording_mp4" => "Frames.mp4".into(),
        "refined_scan_zip" => "RefinedScan.zip".into(),
        _ => format!(
            "{}.{}",
            sanitize_component(name),
            sanitize_component(data_type)
        ),
    }
}

async fn unzip_refined_scan(
    zip_bytes: Vec<u8>,
    unzip_root: PathBuf,
) -> Result<Vec<PathBuf>, StorageError> {
    task::spawn_blocking(move || {
        std::fs::create_dir_all(&unzip_root)
            .map_err(|e| StorageError::Other(format!("create unzip dir: {}", e)))?;
        let mut archive = ZipArchive::new(Cursor::new(zip_bytes))
            .map_err(|e| StorageError::Other(format!("open zip: {}", e)))?;
        let mut extracted = Vec::new();
        for idx in 0..archive.len() {
            let mut file = archive
                .by_index(idx)
                .map_err(|e| StorageError::Other(format!("read zip entry: {}", e)))?;
            if file.is_dir() {
                continue;
            }
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .map_err(|e| StorageError::Other(format!("read zip data: {}", e)))?;
            let out_path = unzip_root.join(file.name());
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| StorageError::Other(format!("create unzip parent: {}", e)))?;
            }
            std::fs::write(&out_path, &buf)
                .map_err(|e| StorageError::Other(format!("write unzip file: {}", e)))?;
            extracted.push(out_path);
        }
        Ok(extracted)
    })
    .await
    .map_err(|e| StorageError::Other(format!("zip task join: {}", e)))?
}
