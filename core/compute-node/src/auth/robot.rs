use super::siwe::{AccessBundle, SiweError};
use super::token_manager::{
    AccessAuthenticator, SystemClock, TokenManager, TokenManagerConfig, TokenProvider,
    TokenProviderError,
};
use crate::config::RobotNodeConfig;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use url::Url;
use uuid::Uuid;

const REGISTER_PATH: &str = "/internal/v1/robots/register";
const VERIFY_PATH: &str = "/internal/v1/auth/robot/verify";

type ManagerCell = Arc<Mutex<Option<Arc<RobotTokenManager>>>>;
type RobotTokenManager = TokenManager<RobotAuthenticator, SystemClock>;

#[derive(Serialize)]
struct RegisterRequest<'a> {
    registration_credentials: &'a str,
    version: &'a str,
    capabilities: &'a [String],
}

#[derive(Serialize)]
struct VerifyRequest<'a> {
    registration_credentials: &'a str,
}

#[derive(Deserialize)]
struct AccessResponse {
    robot_id: Option<Uuid>,
    access_token: Option<String>,
    access_expires_at: Option<String>,
}

struct RobotAuthenticator {
    base_url: Arc<String>,
    registration_credentials: Arc<String>,
    node_version: Arc<String>,
    capabilities: Arc<Vec<String>>,
    client: Client,
    registered: AtomicBool,
}

impl RobotAuthenticator {
    fn new(
        base_url: Url,
        registration_credentials: String,
        node_version: String,
        capabilities: Vec<String>,
        request_timeout: Duration,
    ) -> Result<Self> {
        let registration_credentials = registration_credentials.trim().to_string();
        if registration_credentials.is_empty() {
            return Err(anyhow!("robot registration credentials must not be empty"));
        }
        let node_version = node_version.trim().to_string();
        if node_version.is_empty() {
            return Err(anyhow!("robot node version must not be empty"));
        }

        let client = Client::builder()
            .use_rustls_tls()
            .timeout(request_timeout)
            .build()
            .context("build DDS robot authentication client")?;

        Ok(Self {
            base_url: Arc::new(base_url.to_string()),
            registration_credentials: Arc::new(registration_credentials),
            node_version: Arc::new(node_version),
            capabilities: Arc::new(capabilities),
            client,
            registered: AtomicBool::new(false),
        })
    }

    async fn register(&self) -> std::result::Result<AccessBundle, SiweError> {
        let endpoint = self.endpoint(REGISTER_PATH);
        let response = self
            .client
            .post(endpoint)
            .json(&RegisterRequest {
                registration_credentials: self.registration_credentials.as_str(),
                version: self.node_version.as_str(),
                capabilities: self.capabilities.as_slice(),
            })
            .send()
            .await?;
        decode_access_response(response).await
    }

    async fn verify(&self) -> std::result::Result<AccessBundle, SiweError> {
        let endpoint = self.endpoint(VERIFY_PATH);
        let response = self
            .client
            .post(endpoint)
            .json(&VerifyRequest {
                registration_credentials: self.registration_credentials.as_str(),
            })
            .send()
            .await?;
        decode_access_response(response).await
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }
}

#[async_trait]
impl AccessAuthenticator for RobotAuthenticator {
    async fn login(&self) -> std::result::Result<AccessBundle, SiweError> {
        if self.registered.load(Ordering::Acquire) {
            return self.verify().await;
        }

        let bundle = self.register().await?;
        self.registered.store(true, Ordering::Release);
        Ok(bundle)
    }
}

async fn decode_access_response(
    response: Response,
) -> std::result::Result<AccessBundle, SiweError> {
    if !response.status().is_success() {
        return Err(SiweError::UpstreamStatus(response.status()));
    }

    let body: AccessResponse = response.json().await?;
    if body.robot_id.filter(|id| !id.is_nil()).is_none() {
        return Err(SiweError::MissingField("robot_id"));
    }
    let token = body
        .access_token
        .filter(|value| !value.is_empty())
        .ok_or(SiweError::MissingField("access_token"))?;
    let expires_at_raw = body
        .access_expires_at
        .ok_or(SiweError::MissingField("access_expires_at"))?;
    let expires_at = DateTime::parse_from_rfc3339(&expires_at_raw)?.with_timezone(&Utc);

    Ok(AccessBundle::new(token, expires_at))
}

/// Robot machine-authentication lifecycle for the compute-node engine.
///
/// A new instance always registers first. Once registration succeeds, all
/// token refreshes use the verify endpoint and never fall back to SIWE.
pub struct RobotMachineAuth {
    authenticator: Arc<RobotAuthenticator>,
    config: TokenManagerConfig,
    manager: ManagerCell,
}

impl RobotMachineAuth {
    pub fn from_config(cfg: &RobotNodeConfig, capabilities: Vec<String>) -> Result<Self> {
        let token_config = TokenManagerConfig {
            safety_ratio: cfg.token_safety_ratio as f64,
            max_retries: cfg.token_reauth_max_retries,
            jitter: Duration::from_millis(cfg.token_reauth_jitter_ms),
        };

        Self::new(
            cfg.dds_base_url.clone(),
            cfg.registration_credentials().to_string(),
            cfg.node_version.clone(),
            capabilities,
            Duration::from_secs(cfg.request_timeout_secs.max(1)),
            token_config,
        )
    }

    pub fn new(
        dds_base_url: Url,
        registration_credentials: impl Into<String>,
        node_version: impl Into<String>,
        capabilities: Vec<String>,
        request_timeout: Duration,
        token_config: TokenManagerConfig,
    ) -> Result<Self> {
        let authenticator = Arc::new(RobotAuthenticator::new(
            dds_base_url,
            registration_credentials.into(),
            node_version.into(),
            capabilities,
            request_timeout,
        )?);
        Ok(Self {
            authenticator,
            config: token_config,
            manager: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn start(&self) -> Result<RobotHandle> {
        let mut guard = self.manager.lock().await;
        if let Some(existing) = guard.as_ref() {
            return Ok(RobotHandle {
                manager: existing.clone(),
            });
        }

        let manager = Arc::new(TokenManager::new(
            Arc::clone(&self.authenticator),
            Arc::new(SystemClock),
            self.config.clone(),
        ));

        manager
            .bearer()
            .await
            .map_err(|err| anyhow!("initial DDS robot authentication failed: {err}"))?;

        // Track the authenticated manager before starting its owned task. If
        // startup is cancelled at this boundary, `shutdown` can still stop it.
        *guard = Some(manager.clone());
        manager.start_bg().await;

        Ok(RobotHandle { manager })
    }

    /// Stop and discard any token manager installed by [`Self::start`].
    ///
    /// Calling this after a cancelled startup is safe even when initial DDS
    /// authentication never completed.
    pub async fn shutdown(&self) {
        let manager = self.manager.lock().await.take();
        if let Some(manager) = manager {
            manager.stop_bg().await;
        }
    }
}

#[derive(Clone)]
pub struct RobotHandle {
    manager: Arc<RobotTokenManager>,
}

impl RobotHandle {
    pub async fn bearer(&self) -> Result<String, TokenProviderError> {
        self.manager.bearer().await
    }

    pub async fn shutdown(&self) {
        self.manager.stop_bg().await;
    }
}

#[async_trait]
impl TokenProvider for RobotHandle {
    async fn bearer(&self) -> super::token_manager::TokenProviderResult<String> {
        self.manager.bearer().await
    }

    async fn on_unauthorized(&self) {
        self.manager.on_unauthorized_retry().await;
    }
}
