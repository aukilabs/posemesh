use super::siwe;
use super::token_manager::{
    AccessAuthenticator, SystemClock, TokenManager, TokenManagerConfig, TokenProvider,
    TokenProviderError,
};
use crate::config::NodeConfig;
use crate::dds::persist as dds_state;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use posemesh_node_registration::state::{
    read_state, set_status, STATUS_DISCONNECTED, STATUS_REGISTERED,
};
use reqwest::StatusCode;
use sha3::{Digest, Keccak256};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{info, warn};

type ManagerCell = Arc<Mutex<Option<Arc<SiweTokenManager>>>>;
type SiweTokenManager = TokenManager<DdsAuthenticator, SystemClock>;

#[derive(Clone)]
struct DdsAuthenticator {
    base_url: Arc<String>,
    priv_hex: Arc<String>,
    address: Arc<String>,
}

impl DdsAuthenticator {
    fn new(base_url: String, priv_hex: String) -> Result<Self> {
        let address = derive_eth_address(&priv_hex)?;
        Ok(Self {
            base_url: Arc::new(base_url),
            priv_hex: Arc::new(priv_hex),
            address: Arc::new(address),
        })
    }
}

#[async_trait]
impl AccessAuthenticator for DdsAuthenticator {
    async fn login(&self) -> Result<super::siwe::AccessBundle, super::siwe::SiweError> {
        let meta = siwe::request_nonce(self.base_url.as_str(), self.address.as_str()).await?;
        let message = siwe::compose_message(&meta, self.address.as_str(), None)?;
        let signature = siwe::sign_message(self.priv_hex.as_str(), &message)?;
        match siwe::verify(
            self.base_url.as_str(),
            self.address.as_str(),
            &message,
            &signature,
        )
        .await
        {
            Ok(bundle) => Ok(bundle),
            Err(err) => {
                rearm_registration_if_invalid(&err);
                Err(err)
            }
        }
    }
}

fn rearm_registration_if_invalid(err: &siwe::SiweError) {
    let Some(status) = err.status_code() else {
        return;
    };
    if status != StatusCode::FORBIDDEN && status != StatusCode::NOT_FOUND {
        return;
    }

    if let Ok(state) = read_state() {
        if state.status == STATUS_DISCONNECTED {
            return;
        }
    }

    match set_status(STATUS_DISCONNECTED) {
        Ok(()) => warn!(
            status = %status,
            "DDS SIWE verification indicates registration is no longer valid; re-arming registration"
        ),
        Err(set_err) => warn!(
            status = %status,
            error = %set_err,
            "failed to re-arm registration after DDS SIWE verification invalidation"
        ),
    }
}

pub struct SiweAfterRegistration {
    authenticator: Arc<DdsAuthenticator>,
    config: TokenManagerConfig,
    manager: ManagerCell,
}

impl SiweAfterRegistration {
    pub fn from_config(cfg: &NodeConfig) -> Result<Self> {
        let dds_base_url = cfg
            .dds_base_url
            .as_ref()
            .ok_or_else(|| anyhow!("DDS_BASE_URL required for DDS SIWE authentication"))?
            .as_str()
            .to_string();

        let priv_hex = cfg
            .secp256k1_privhex
            .as_ref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow!("SECP256K1_PRIVHEX required for DDS SIWE authentication"))?
            .to_string();

        let config = TokenManagerConfig {
            safety_ratio: cfg.token_safety_ratio as f64,
            max_retries: cfg.token_reauth_max_retries,
            jitter: Duration::from_millis(cfg.token_reauth_jitter_ms),
        };

        Self::new(dds_base_url, priv_hex, config)
    }

    pub fn new(dds_base_url: String, priv_hex: String, config: TokenManagerConfig) -> Result<Self> {
        let authenticator = Arc::new(DdsAuthenticator::new(dds_base_url, priv_hex)?);
        Ok(Self {
            authenticator,
            config,
            manager: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn start(&self) -> Result<SiweHandle> {
        let manager = self.ensure_started().await?;
        Ok(SiweHandle { manager })
    }

    async fn ensure_started(&self) -> Result<Arc<SiweTokenManager>> {
        {
            let guard = self.manager.lock().await;
            if let Some(existing) = guard.as_ref() {
                return Ok(existing.clone());
            }
        }

        self.wait_for_registration().await?;

        let manager: Arc<SiweTokenManager> = Arc::new(TokenManager::new(
            Arc::clone(&self.authenticator),
            Arc::new(SystemClock),
            self.config.clone(),
        ));
        manager.start_bg().await;

        manager
            .bearer()
            .await
            .map_err(|err| anyhow!("initial DDS SIWE login failed: {err}"))?;

        let mut guard = self.manager.lock().await;
        if let Some(existing) = guard.as_ref() {
            manager.stop_bg().await;
            return Ok(existing.clone());
        }
        *guard = Some(manager.clone());
        Ok(manager)
    }

    async fn wait_for_registration(&self) -> Result<()> {
        loop {
            // Prefer the explicit registration state first.
            match read_state() {
                Ok(state) if state.status == STATUS_REGISTERED => {
                    info!(
                        "DDS registration confirmed (status=registered); starting SIWE token manager"
                    );
                    return Ok(());
                }
                Ok(_) => {
                    // Fall through to secret check as a secondary signal (legacy flows).
                }
                Err(err) => {
                    warn!(error = %err, "Failed to read DDS registration state; retrying");
                }
            }

            match dds_state::read_node_secret() {
                Ok(Some(_)) => {
                    info!(
                        "DDS registration confirmed (secret present); starting SIWE token manager"
                    );
                    return Ok(());
                }
                Ok(None) => {}
                Err(err) => {
                    warn!(error = %err, "Failed to read DDS registration secret; retrying");
                }
            }

            sleep(Duration::from_secs(1)).await;
        }
    }
}

#[derive(Clone)]
pub struct SiweHandle {
    manager: Arc<SiweTokenManager>,
}

impl SiweHandle {
    pub async fn bearer(&self) -> Result<String, TokenProviderError> {
        self.manager.bearer().await
    }

    pub async fn shutdown(&self) {
        self.manager.stop_bg().await;
    }
}

#[async_trait]
impl TokenProvider for SiweHandle {
    async fn bearer(&self) -> crate::auth::token_manager::TokenProviderResult<String> {
        // Delegate to internal manager
        self.manager.bearer().await
    }

    async fn on_unauthorized(&self) {
        // Force early refresh on next bearer() call
        self.manager.on_unauthorized_retry().await;
    }
}

fn derive_eth_address(priv_hex: &str) -> Result<String> {
    use k256::{ecdsa::SigningKey, FieldBytes};

    let trimmed = priv_hex.trim_start_matches("0x");
    let key_bytes =
        hex::decode(trimmed).map_err(|_| anyhow!("invalid secp256k1 private key hex"))?;
    if key_bytes.len() != 32 {
        return Err(anyhow!("secp256k1 private key must be 32 bytes"));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&key_bytes);
    let field_bytes: FieldBytes = key.into();
    let signing_key = SigningKey::from_bytes(&field_bytes)
        .map_err(|e| anyhow!("failed to construct signing key: {e}"))?;
    let verifying_key = signing_key.verifying_key();
    let encoded = verifying_key.to_encoded_point(false);
    let pubkey = encoded.as_bytes();

    let mut hasher = Keccak256::new();
    hasher.update(&pubkey[1..]);
    let hashed = hasher.finalize();
    let address_bytes = &hashed[12..];
    Ok(format!("0x{}", hex::encode(address_bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use posemesh_node_registration::state::{write_state, RegistrationState};
    use reqwest::StatusCode;
    use std::sync::{Mutex, OnceLock};
    use url::Url;

    fn test_lock() -> &'static Mutex<()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_LOCK.get_or_init(|| Mutex::new(()))
    }

    fn base_cfg() -> NodeConfig {
        NodeConfig {
            dms_base_url: Url::parse("https://dms.example").unwrap(),
            node_version: "1.0.0".into(),
            request_timeout_secs: 10,
            dds_base_url: None,
            reg_secret: None,
            secp256k1_privhex: None,
            heartbeat_jitter_ms: 250,
            heartbeat_min_ratio: 0.25,
            heartbeat_max_ratio: 0.35,
            poll_backoff_ms_min: 1000,
            poll_backoff_ms_max: 30000,
            token_safety_ratio: 0.75,
            token_reauth_max_retries: 3,
            token_reauth_jitter_ms: 500,
            register_interval_secs: None,
            register_max_retry: None,
            max_concurrency: 1,
            log_format: crate::config::LogFormat::Json,
            enable_noop: true,
            noop_sleep_secs: 1,
        }
    }

    #[test]
    fn from_config_errors_when_missing_siwe_fields() {
        let cfg = base_cfg();
        assert!(SiweAfterRegistration::from_config(&cfg).is_err());
    }

    #[test]
    fn derive_eth_address_matches_expected_value() {
        let priv_hex = "4c0883a69102937d6231471b5dbb6204fe5129617082798ce3f4fdf2548b6f90";
        let addr = derive_eth_address(priv_hex).expect("address");
        assert_eq!(addr, "0xfdbb6caf01414300c16ea14859fec7736d95355f");
    }

    #[test]
    fn from_config_produces_instance_when_siwe_configured() {
        let mut cfg = base_cfg();
        cfg.dds_base_url = Some(Url::parse("https://dds.example").unwrap());
        cfg.secp256k1_privhex =
            Some("4c0883a69102937d6231471b5dbb6204fe5129617082798ce3f4fdf2548b6f90".into());

        assert!(SiweAfterRegistration::from_config(&cfg).is_ok());
    }

    #[test]
    fn forbidden_siwe_verify_rearms_registration() {
        let _guard = test_lock().lock().unwrap();
        write_state(&RegistrationState {
            status: STATUS_REGISTERED.to_string(),
            last_healthcheck: None,
        })
        .unwrap();

        rearm_registration_if_invalid(&siwe::SiweError::UpstreamStatus(StatusCode::FORBIDDEN));

        assert_eq!(read_state().unwrap().status, STATUS_DISCONNECTED);
    }

    #[test]
    fn unrelated_siwe_error_keeps_registration_state() {
        let _guard = test_lock().lock().unwrap();
        write_state(&RegistrationState {
            status: STATUS_REGISTERED.to_string(),
            last_healthcheck: None,
        })
        .unwrap();

        rearm_registration_if_invalid(&siwe::SiweError::UpstreamStatus(StatusCode::BAD_GATEWAY));

        assert_eq!(read_state().unwrap().status, STATUS_REGISTERED);
    }
}
