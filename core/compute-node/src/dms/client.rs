use crate::auth::token_manager::TokenProvider;
use crate::dms::types::{
    CompleteTaskRequest, FailTaskRequest, HeartbeatRequest, HeartbeatResponse, LeaseResponse,
};
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    StatusCode,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::Level;
use url::Url;
use uuid::Uuid;

/// Minimal DMS HTTP client using rustls with sensitive Authorization header.
#[derive(Clone)]
pub struct DmsClient {
    base: Url,
    http: Client,
    auth: Arc<dyn TokenProvider>,
}
impl DmsClient {
    /// Create client with base URL, timeout, and a token provider for Authorization.
    pub fn new(base: Url, timeout: Duration, auth: Arc<dyn TokenProvider>) -> Result<Self> {
        let http = Client::builder()
            .use_rustls_tls()
            .timeout(timeout)
            .build()
            .context("build dms reqwest client")?;
        Ok(Self { base, http, auth })
    }

    async fn auth_headers(&self) -> Result<HeaderMap> {
        let mut h = HeaderMap::new();
        let b = self
            .auth
            .bearer()
            .await
            .map_err(|e| anyhow!("token provider: {e}"))?;
        let token = format!("Bearer {}", b);
        let mut v = HeaderValue::from_str(&token)
            .unwrap_or_else(|_| HeaderValue::from_static("Bearer INVALID"));
        v.set_sensitive(true);
        h.insert(AUTHORIZATION, v);
        Ok(h)
    }

    fn join_segments(&self, segments: &[&str]) -> Result<Url> {
        let mut url = self.base.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("invalid DMS base URL; cannot be a base"))?
            .extend(segments.iter().copied());
        Ok(url)
    }

    /// Lease a task: GET /tasks
    ///
    /// `capability` is accepted for optional filter but not implemented yet.
    pub async fn lease_by_capability(&self, _capability: &str) -> Result<Option<LeaseResponse>> {
        let url = self.join_segments(&["tasks"]).context("join /tasks")?;
        if tracing::enabled!(Level::DEBUG) {
            tracing::debug!(
                endpoint = %url,
                "Sending DMS lease request"
            );
        }
        // First attempt
        let mut headers = self.auth_headers().await?;
        let mut res = self
            .http
            .get(url.clone())
            .headers(headers.clone())
            .send()
            .await
            .context("send GET /tasks")?;
        let mut status = res.status();
        let mut bytes = res.bytes().await.context("read lease body")?;
        // Retry once on 401
        if status == StatusCode::UNAUTHORIZED {
            tracing::warn!(
                status = %status,
                "DMS lease unauthorized; refreshing token and retrying"
            );
            self.auth.on_unauthorized().await;
            headers = self.auth_headers().await?;
            res = self
                .http
                .get(url)
                .headers(headers)
                .send()
                .await
                .context("retry GET /tasks")?;
            status = res.status();
            bytes = res.bytes().await.context("read lease body (retry)")?;
        }
        if status == StatusCode::NO_CONTENT {
            tracing::debug!("DMS lease returned 204 (no work available)");
            return Ok(None);
        }
        if status == StatusCode::CONFLICT {
            tracing::debug!(
                status = %status,
                "DMS lease returned conflict (busy); treating as no work"
            );
            return Ok(None);
        }
        if !status.is_success() {
            tracing::warn!(
                status = %status,
                "DMS lease request returned non-success status"
            );
            return Err(anyhow!("/tasks status: {}", status));
        }
        let lease: LeaseResponse = serde_json::from_slice(&bytes)
            .map_err(|err| {
                tracing::error!(
                    status = %status,
                    error = %err,
                    "Failed to decode DMS lease response"
                );
                err
            })
            .context("decode lease")?;

        if tracing::enabled!(Level::DEBUG) {
            tracing::debug!(
                status = %status,
                task_id = %lease.task.id,
                capability = %lease.task.capability,
                access_token_updated = lease.access_token.is_some(),
                "Decoded DMS lease response"
            );
        }

        Ok(Some(lease))
    }

    /// Complete task: POST /tasks/{id}/complete
    pub async fn complete(&self, task_id: Uuid, body: &CompleteTaskRequest) -> Result<()> {
        let url = self
            .join_segments(&["tasks", &task_id.to_string(), "complete"])
            .context("join /complete")?;
        let mut headers = self.auth_headers().await?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if tracing::enabled!(Level::DEBUG) {
            tracing::debug!(
                endpoint = %url,
                task_id = %task_id,
                "Sending DMS complete request"
            );
        }
        // First attempt
        let mut res = self
            .http
            .post(url.clone())
            .headers(headers.clone())
            .json(body)
            .send()
            .await
            .context("send POST /complete")?;
        let mut status = res.status();
        let _ = res.bytes().await;
        if status == StatusCode::UNAUTHORIZED {
            tracing::warn!(
                status = %status,
                task_id = %task_id,
                "DMS complete unauthorized; refreshing token and retrying"
            );
            self.auth.on_unauthorized().await;
            headers = self.auth_headers().await?;
            res = self
                .http
                .post(url)
                .headers(headers)
                .json(body)
                .send()
                .await
                .context("retry POST /complete")?;
            status = res.status();
            let _ = res.bytes().await;
        }
        if tracing::enabled!(Level::DEBUG) {
            tracing::debug!(
                status = %status,
                task_id = %task_id,
                "DMS complete response"
            );
        }
        if !status.is_success() {
            tracing::error!(
                status = %status,
                task_id = %task_id,
                "DMS complete endpoint returned non-success status"
            );
            return Err(anyhow!("POST /tasks/{task_id}/complete status {status}"));
        }
        Ok(())
    }

    /// Fail task: POST /tasks/{id}/fail
    pub async fn fail(&self, task_id: Uuid, body: &FailTaskRequest) -> Result<()> {
        let url = self
            .join_segments(&["tasks", &task_id.to_string(), "fail"])
            .context("join /fail")?;
        let mut headers = self.auth_headers().await?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if tracing::enabled!(Level::DEBUG) {
            tracing::debug!(
                endpoint = %url,
                task_id = %task_id,
                "Sending DMS fail request"
            );
        }
        // First attempt
        let mut res = self
            .http
            .post(url.clone())
            .headers(headers.clone())
            .json(body)
            .send()
            .await
            .context("send POST /fail")?;
        let mut status = res.status();
        let _ = res.bytes().await;
        if status == StatusCode::UNAUTHORIZED {
            tracing::warn!(
                status = %status,
                task_id = %task_id,
                "DMS fail unauthorized; refreshing token and retrying"
            );
            self.auth.on_unauthorized().await;
            headers = self.auth_headers().await?;
            res = self
                .http
                .post(url)
                .headers(headers)
                .json(body)
                .send()
                .await
                .context("retry POST /fail")?;
            status = res.status();
            let _ = res.bytes().await;
        }
        if tracing::enabled!(Level::DEBUG) {
            tracing::debug!(
                status = %status,
                task_id = %task_id,
                "DMS fail response"
            );
        }
        if !status.is_success() {
            tracing::error!(
                status = %status,
                task_id = %task_id,
                "DMS fail endpoint returned non-success status"
            );
            return Err(anyhow!("POST /tasks/{task_id}/fail status {status}"));
        }
        Ok(())
    }

    /// Heartbeat: POST /tasks/{id}/heartbeat with progress payload.
    /// Returns potential new access token for storage.
    pub async fn heartbeat(
        &self,
        task_id: Uuid,
        body: &HeartbeatRequest,
    ) -> Result<HeartbeatResponse> {
        let url = self
            .join_segments(&["tasks", &task_id.to_string(), "heartbeat"])
            .context("join /heartbeat")?;
        let mut headers = self.auth_headers().await?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if tracing::enabled!(Level::DEBUG) {
            tracing::debug!(
                endpoint = %url,
                task_id = %task_id,
                "Sending DMS heartbeat request"
            );
        }
        // First attempt
        let mut res = self
            .http
            .post(url.clone())
            .headers(headers.clone())
            .json(body)
            .send()
            .await
            .context("send POST /heartbeat")?;
        let mut status = res.status();
        let mut bytes = res.bytes().await.context("read heartbeat response body")?;
        if status == StatusCode::UNAUTHORIZED {
            tracing::warn!(
                status = %status,
                task_id = %task_id,
                "DMS heartbeat unauthorized; refreshing token and retrying"
            );
            self.auth.on_unauthorized().await;
            headers = self.auth_headers().await?;
            res = self
                .http
                .post(url)
                .headers(headers)
                .json(body)
                .send()
                .await
                .context("retry POST /heartbeat")?;
            status = res.status();
            bytes = res
                .bytes()
                .await
                .context("read heartbeat response body (retry)")?;
        }
        if !status.is_success() {
            tracing::warn!(
                status = %status,
                task_id = %task_id,
                "DMS heartbeat endpoint returned non-success status"
            );
            return Err(anyhow!("POST /tasks/{task_id}/heartbeat status {status}"));
        }
        let hb = serde_json::from_slice::<HeartbeatResponse>(&bytes)
            .map_err(|err| {
                tracing::error!(
                    status = %status,
                    task_id = %task_id,
                    error = %err,
                    "Failed to decode DMS heartbeat response"
                );
                err
            })
            .context("decode heartbeat response")?;
        if tracing::enabled!(Level::DEBUG) {
            tracing::debug!(
                status = %status,
                task_id = %task_id,
                access_token_updated = hb.access_token.is_some(),
                cancel = ?hb.cancel,
                "Decoded DMS heartbeat response"
            );
        }
        Ok(hb)
    }
}
