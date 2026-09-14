use super::sdk::SdkDomainClient;
use crate::errors::StorageError;
use crate::storage::token::TokenRef;
use anyhow::Result;
use auki_auth::{DomainAccess, DomainAccessProvider, SecretString};
use auki_sdk::{AukiDomainData, DataLimits, DataListQuery};
use regex::Regex;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use url::Url;
use uuid::Uuid;

/// One materialized Domain data item, with its metadata and temporary paths.
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

#[derive(Debug)]
pub struct UploadFileRequest<'a> {
    pub domain_id: &'a str,
    pub name: &'a str,
    pub data_type: &'a str,
    pub logical_path: &'a str,
    pub path: &'a Path,
    pub existing_id: Option<&'a str>,
}

/// Domain storage adapter backed by the SDK data client.
///
/// Standalone callers supply an authenticated DDS/DMS data grant and continue
/// to own renewal through `TokenRef::swap`. Managed tasks use their SDK lease
/// credential directly. Neither path starts a second authentication owner.
#[derive(Clone)]
pub struct DomainClient {
    pub base: Url,
    pub token: TokenRef,
    backend: Backend,
}

#[derive(Clone)]
enum Backend {
    Managed(SdkDomainClient),
    External(AukiDomainData),
}

impl DomainClient {
    pub(crate) fn from_task(task: &auki_sdk::TaskContext) -> Result<Self> {
        let lease = task.credential.lease_snapshot()?;
        Ok(Self {
            base: lease
                .domain_server_url
                .ok_or_else(|| anyhow::anyhow!("task has no Domain Server"))?,
            token: TokenRef::from_task(task.access_token.clone()),
            backend: Backend::Managed(SdkDomainClient::new(task)?),
        })
    }

    pub(crate) async fn close(&self) {
        if let Backend::Managed(sdk) = &self.backend {
            sdk.close().await;
        }
    }

    pub fn new(base: Url, token: TokenRef) -> Result<Self> {
        Self::with_timeout(base, token, DataLimits::default().request_timeout)
    }

    pub fn with_timeout(base: Url, token: TokenRef, timeout: Duration) -> Result<Self> {
        let data = AukiDomainData::with_limits(
            ExternalGrant {
                base: base.clone(),
                token: token.clone(),
                client_id: env_client_id(),
            },
            DataLimits {
                request_timeout: timeout,
                max_data_bytes: super::sdk::BUFFER_LIMIT,
                ..DataLimits::default()
            },
        )?;
        Ok(Self {
            base,
            token,
            backend: Backend::External(data),
        })
    }

    fn sdk(&self, domain: &str) -> std::result::Result<SdkDomainClient, StorageError> {
        match &self.backend {
            Backend::Managed(sdk) => Ok(sdk.clone()),
            Backend::External(data) => Ok(SdkDomainClient::from_data(
                data.in_domain(Uuid::parse_str(domain).map_err(|_| StorageError::BadRequest)?),
            )),
        }
    }

    /// Materialize matching metadata and raw data into temporary files.
    pub async fn download_uri(
        &self,
        uri: &str,
    ) -> std::result::Result<Vec<DownloadedPart>, StorageError> {
        let resolved = resolve_domain_url(&self.base, uri)?;
        let (domain, query) = parse_download_target(&resolved, None)?;
        self.sdk(&domain)?.download(&domain, query).await
    }

    /// Accept a data UUID or an absolute/relative Domain data URL.
    pub async fn download_cid(
        &self,
        domain: &str,
        cid: &str,
    ) -> std::result::Result<Vec<DownloadedPart>, StorageError> {
        let cid = cid.trim();
        if cid.contains("://") || cid.starts_with('/') {
            let resolved = resolve_domain_url(&self.base, cid)?;
            let (domain, query) = parse_download_target(&resolved, Some(domain))?;
            return self.sdk(&domain)?.download(&domain, query).await;
        }
        let query = DataListQuery {
            ids: vec![Uuid::parse_str(cid).map_err(|_| StorageError::BadRequest)?],
            ..Default::default()
        };
        self.sdk(domain)?.download(domain, query).await
    }

    pub async fn upload_artifact(
        &self,
        request: UploadRequest<'_>,
    ) -> std::result::Result<Option<String>, StorageError> {
        self.sdk(request.domain_id)?.upload(request).await
    }

    pub async fn upload_artifact_file(
        &self,
        request: UploadFileRequest<'_>,
    ) -> std::result::Result<Option<String>, StorageError> {
        self.sdk(request.domain_id)?.upload_file(request).await
    }

    pub async fn find_artifact_id(
        &self,
        domain: &str,
        name: &str,
        data_type: &str,
    ) -> std::result::Result<Option<String>, StorageError> {
        self.sdk(domain)?.find(domain, name, data_type).await
    }
}

/// Read an already issued grant without logging in, polling or refreshing it.
struct ExternalGrant {
    base: Url,
    token: TokenRef,
    client_id: String,
}

#[async_trait::async_trait]
impl DomainAccessProvider for ExternalGrant {
    fn client_id(&self) -> &str {
        &self.client_id
    }

    async fn wait_closed(&self) {
        std::future::pending::<()>().await
    }

    async fn domain_access(
        &self,
        domain: Uuid,
        rejected: Option<&DomainAccess>,
        cancellation: &CancellationToken,
    ) -> auki_auth::Result<Arc<DomainAccess>> {
        if cancellation.is_cancelled() {
            return Err(auki_auth::Error::Cancelled {
                endpoint: "external task grant",
            });
        }
        let token = self.token.get();
        if token.is_empty() || rejected.is_some_and(|old| old.bearer().expose_secret() == token) {
            return Err(auki_auth::Error::AuthenticationRequired);
        }
        // The SDK validates issuer, Domain, server audience and JWT expiry.
        // There is no additional lease-expiry cap for an externally owned grant.
        DomainAccess::from_issued_grant(
            domain,
            self.base.as_str(),
            SecretString::new(token),
            chrono::DateTime::<chrono::Utc>::MAX_UTC,
        )
        .map(Arc::new)
    }
}

pub(crate) fn env_client_id() -> String {
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
) -> std::result::Result<(String, DataListQuery), StorageError> {
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
                name.get_or_insert_with(|| value.to_string());
            }
            "data_type" => {
                data_type.get_or_insert_with(|| value.to_string());
            }
            _ => {}
        }
    }

    if let Some(id) = data_id_from_path {
        ids = vec![id.to_string()];
    }

    Ok((
        domain_id,
        DataListQuery {
            ids: ids
                .iter()
                .map(|id| Uuid::parse_str(id).map_err(|_| StorageError::BadRequest))
                .collect::<std::result::Result<_, _>>()?,
            name,
            data_type,
        },
    ))
}

pub(super) fn sanitize_component(value: &str) -> String {
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

pub(super) fn extract_timestamp(name: &str) -> Option<String> {
    Regex::new(r"\d{4}-\d{2}-\d{2}[_-]\d{2}-\d{2}-\d{2}")
        .ok()
        .and_then(|re| re.find(name).map(|m| m.as_str().to_string()))
}

pub(super) fn map_filename(data_type: &str, name: &str) -> String {
    format!(
        "{}.{}",
        sanitize_component(name),
        sanitize_component(data_type)
    )
}
