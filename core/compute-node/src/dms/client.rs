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
use serde::Serialize;
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

    /// Lease a task: GET /tasks
    ///
    /// `capability` is accepted for optional filter but not implemented yet.
    pub async fn lease_by_capability(&self, _capability: &str) -> Result<Option<LeaseResponse>> {
        let url = self.base.join("tasks").context("join /tasks")?;
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
            let body_preview = String::from_utf8_lossy(&bytes);
            tracing::warn!(
                status = %status,
                body = %body_preview,
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
        let body_preview = String::from_utf8_lossy(&bytes);
        if !status.is_success() {
            tracing::warn!(
                status = %status,
                body = %body_preview,
                "DMS lease request returned non-success status"
            );
            return Err(anyhow!("/tasks status: {}", status));
        }
        let lease: LeaseResponse = serde_json::from_slice(&bytes)
            .map_err(|err| {
                tracing::error!(
                    error = %err,
                    body = %body_preview,
                    "Failed to decode DMS lease response"
                );
                err
            })
            .context("decode lease")?;

        if tracing::enabled!(Level::DEBUG) {
            tracing::debug!(
                status = %status,
                body = %body_preview,
                "Decoded DMS lease response"
            );
        }

        Ok(Some(lease))
    }

    /// Complete task: POST /tasks/{id}/complete
    pub async fn complete(&self, task_id: Uuid, body: &CompleteTaskRequest) -> Result<()> {
        let url = self
            .base
            .join(&format!("tasks/{}/complete", task_id))
            .context("join /complete")?;
        let mut headers = self.auth_headers().await?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(preview) = json_debug_preview(body) {
            tracing::debug!(
                endpoint = %url,
                task_id = %task_id,
                body = %preview,
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
        let mut body_text = res
            .text()
            .await
            .unwrap_or_else(|e| format!("<failed to read body: {e}>"));
        if status == StatusCode::UNAUTHORIZED {
            let preview = truncate_preview(&body_text);
            tracing::warn!(
                status = %status,
                body = %preview,
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
            body_text = res
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body (retry): {e}>"));
        }
        let preview = truncate_preview(&body_text);
        if tracing::enabled!(Level::DEBUG) {
            tracing::debug!(
                status = %status,
                body = %preview,
                task_id = %task_id,
                "DMS complete response"
            );
        }
        if !status.is_success() {
            tracing::error!(
                status = %status,
                body = %preview,
                task_id = %task_id,
                "DMS complete endpoint returned non-success status"
            );
            return Err(anyhow!(
                "POST /tasks/{task_id}/complete status {status}; body: {preview}"
            ));
        }
        Ok(())
    }

    /// Fail task: POST /tasks/{id}/fail
    pub async fn fail(&self, task_id: Uuid, body: &FailTaskRequest) -> Result<()> {
        let url = self
            .base
            .join(&format!("tasks/{}/fail", task_id))
            .context("join /fail")?;
        let mut headers = self.auth_headers().await?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(preview) = json_debug_preview(body) {
            tracing::debug!(
                endpoint = %url,
                task_id = %task_id,
                body = %preview,
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
        let mut body_text = res
            .text()
            .await
            .unwrap_or_else(|e| format!("<failed to read body: {e}>"));
        if status == StatusCode::UNAUTHORIZED {
            let preview = truncate_preview(&body_text);
            tracing::warn!(
                status = %status,
                body = %preview,
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
            body_text = res
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body (retry): {e}>"));
        }
        let preview = truncate_preview(&body_text);
        if tracing::enabled!(Level::DEBUG) {
            tracing::debug!(
                status = %status,
                body = %preview,
                task_id = %task_id,
                "DMS fail response"
            );
        }
        if !status.is_success() {
            tracing::error!(
                status = %status,
                body = %preview,
                task_id = %task_id,
                "DMS fail endpoint returned non-success status"
            );
            return Err(anyhow!(
                "POST /tasks/{task_id}/fail status {status}; body: {preview}"
            ));
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
            .base
            .join(&format!("tasks/{}/heartbeat", task_id))
            .context("join /heartbeat")?;
        let mut headers = self.auth_headers().await?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(preview) = json_debug_preview(body) {
            tracing::debug!(
                endpoint = %url,
                task_id = %task_id,
                body = %preview,
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
            let preview = truncate_preview(&String::from_utf8_lossy(&bytes));
            tracing::warn!(
                status = %status,
                body = %preview,
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
        let preview = truncate_preview(&String::from_utf8_lossy(&bytes));
        if tracing::enabled!(Level::DEBUG) {
            tracing::debug!(
                status = %status,
                body = %preview,
                task_id = %task_id,
                "DMS heartbeat response"
            );
        }
        if !status.is_success() {
            return Err(anyhow!(
                "POST /tasks/{task_id}/heartbeat status {status}; body: {preview}"
            ));
        }
        let hb = serde_json::from_slice::<HeartbeatResponse>(&bytes)
            .context("decode heartbeat response")?;
        Ok(hb)
    }
}

fn truncate_preview(body: &str) -> String {
    const MAX: usize = 512;
    if body.len() <= MAX {
        body.to_string()
    } else {
        let mut preview: String = body.chars().take(MAX).collect();
        preview.push_str("… (truncated)");
        preview
    }
}

fn json_debug_preview<T: Serialize>(value: &T) -> Option<String> {
    if !tracing::enabled!(Level::DEBUG) {
        return None;
    }
    serde_json::to_string(value)
        .map(|s| truncate_preview(&s))
        .ok()
}
