use super::token_manager::{
    SystemClock, TokenManager, TokenManagerConfig, TokenProvider, TokenProviderError,
};
use super::PeerBoundAuthenticator;
use crate::config::RobotNodeConfig;
use crate::dds::p2p::PeerBindingClient;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use auki_auth::machine::robot::RobotAuthenticator;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use url::Url;

type ManagerCell = Arc<Mutex<Option<Arc<RobotTokenManager>>>>;
type RobotTokenManager = TokenManager<PeerBoundAuthenticator<RobotAuthenticator>, SystemClock>;

/// Robot machine-authentication lifecycle for the compute-node engine.
///
/// A new instance always registers first. Once registration succeeds, all
/// token refreshes use the verify endpoint and never fall back to SIWE.
pub struct RobotMachineAuth {
    authenticator: Arc<PeerBoundAuthenticator<RobotAuthenticator>>,
    config: TokenManagerConfig,
    manager: ManagerCell,
}

impl RobotMachineAuth {
    pub fn from_config(cfg: &RobotNodeConfig, capabilities: Vec<String>) -> Result<Self> {
        Self::from_config_with_peer_binding(cfg, capabilities, None)
    }

    pub(crate) fn from_config_with_peer_binding(
        cfg: &RobotNodeConfig,
        capabilities: Vec<String>,
        peer_binding: Option<PeerBindingClient>,
    ) -> Result<Self> {
        let token_config = TokenManagerConfig {
            safety_ratio: cfg.token_safety_ratio as f64,
            max_retries: cfg.token_reauth_max_retries,
            jitter: Duration::from_millis(cfg.token_reauth_jitter_ms),
        };

        Self::new_inner(
            cfg.dds_base_url.clone(),
            cfg.registration_credentials().to_string(),
            cfg.node_version.clone(),
            capabilities,
            Duration::from_secs(cfg.request_timeout_secs.max(1)),
            token_config,
            peer_binding,
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
        Self::new_inner(
            dds_base_url,
            registration_credentials,
            node_version,
            capabilities,
            request_timeout,
            token_config,
            None,
        )
    }

    pub fn new_peer_bound(
        dds_base_url: Url,
        registration_credentials: impl Into<String>,
        node_version: impl Into<String>,
        capabilities: Vec<String>,
        request_timeout: Duration,
        token_config: TokenManagerConfig,
        peer_binding: PeerBindingClient,
    ) -> Result<Self> {
        Self::new_inner(
            dds_base_url,
            registration_credentials,
            node_version,
            capabilities,
            request_timeout,
            token_config,
            Some(peer_binding),
        )
    }

    fn new_inner(
        dds_base_url: Url,
        registration_credentials: impl Into<String>,
        node_version: impl Into<String>,
        capabilities: Vec<String>,
        request_timeout: Duration,
        token_config: TokenManagerConfig,
        peer_binding: Option<PeerBindingClient>,
    ) -> Result<Self> {
        let base = RobotAuthenticator::new(
            dds_base_url,
            registration_credentials.into(),
            node_version.into(),
            capabilities,
            request_timeout,
        )?;
        let authenticator = Arc::new(PeerBoundAuthenticator::new(base, peer_binding));
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
