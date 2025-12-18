//! DDS registration loop wiring (outbound `/internal/v1/nodes/register`).

use std::time::Duration;

use anyhow::{Context, Result};
use posemesh_node_registration::{
    crypto,
    register::{self, RegistrationConfig},
};
use semver::Version;
use tracing::{info, warn};

use crate::config::NodeConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistrationSettings {
    dds_base_url: String,
    node_url: String,
    node_version: String,
    reg_secret: String,
    secp256k1_privhex: String,
    register_interval_secs: u64,
    max_retry: i32,
    request_timeout_secs: u64,
    capabilities: Vec<String>,
}

fn registration_settings(
    cfg: &NodeConfig,
    capabilities: &[String],
) -> Option<RegistrationSettings> {
    let dds_base = cfg.dds_base_url.as_ref()?;
    let node_url = cfg.node_url.as_ref()?;
    let reg_secret = cfg.reg_secret.as_ref()?;
    let privhex = cfg.secp256k1_privhex.as_ref()?;

    let register_interval_secs = cfg.register_interval_secs.unwrap_or(120).max(1);
    let max_retry = cfg.register_max_retry.unwrap_or(-1).max(-1);
    let request_timeout_secs = cfg.request_timeout_secs.max(1);
    let node_version = normalize_node_version(&cfg.node_version);

    Some(RegistrationSettings {
        dds_base_url: dds_base.to_string(),
        node_url: node_url.to_string(),
        node_version,
        reg_secret: reg_secret.clone(),
        secp256k1_privhex: privhex.clone(),
        register_interval_secs,
        max_retry,
        request_timeout_secs,
        capabilities: capabilities.to_vec(),
    })
}

fn normalize_node_version(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        let fallback = env!("CARGO_PKG_VERSION").to_string();
        warn!(
            raw_version = raw,
            fallback, "NODE_VERSION missing; falling back to package version"
        );
        return fallback;
    }

    if Version::parse(trimmed).is_ok() {
        return trimmed.to_string();
    }

    let without_v = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    if Version::parse(without_v).is_ok() {
        warn!(
            raw_version = trimmed,
            normalized = without_v,
            "NODE_VERSION had leading 'v'; normalizing for DDS registration"
        );
        return without_v.to_string();
    }

    warn!(
        raw_version = trimmed,
        fallback = env!("CARGO_PKG_VERSION"),
        "NODE_VERSION invalid semver; falling back to package version"
    );
    env!("CARGO_PKG_VERSION").to_string()
}

/// Spawn the DDS registration loop when all required configuration is available.
pub fn spawn_registration_if_configured(cfg: &NodeConfig, capabilities: &[String]) -> Result<()> {
    let Some(settings) = registration_settings(cfg, capabilities) else {
        warn!("DDS registration disabled: missing DDS_BASE_URL, NODE_URL, REG_SECRET, or SECP256K1_PRIVHEX");
        return Ok(());
    };

    let request_timeout = Duration::from_secs(settings.request_timeout_secs);
    let client = reqwest::Client::builder()
        .timeout(request_timeout)
        .build()
        .context("build DDS registration client")?;

    match crypto::load_secp256k1_privhex(&settings.secp256k1_privhex) {
        Ok(sk) => {
            let pk_hex = crypto::secp256k1_pubkey_uncompressed_hex(&sk);
            let prefix = pk_hex.get(0..16).unwrap_or(&pk_hex);
            info!(public_key_prefix = prefix, "Derived secp256k1 public key");
        }
        Err(err) => {
            warn!(error = %err, "Invalid SECP256K1_PRIVHEX; DDS registration disabled");
            return Ok(());
        }
    }

    let RegistrationSettings {
        dds_base_url,
        node_url,
        node_version,
        reg_secret,
        secp256k1_privhex,
        register_interval_secs,
        max_retry,
        capabilities,
        ..
    } = settings;

    info!(
        dds_base = %dds_base_url,
        node_url = %node_url,
        register_interval_secs,
        max_retry,
        "Starting DDS registration loop"
    );

    let cfg = RegistrationConfig {
        dds_base_url,
        node_url,
        node_version,
        reg_secret,
        secp256k1_privhex,
        client,
        register_interval_secs,
        max_retry,
        capabilities,
    };

    tokio::spawn(async move {
        register::run_registration_loop(cfg).await;
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LogFormat, NodeConfig};
    use url::Url;

    const MOCK_CAPABILITY: &str = "/posemesh/mock/v1";

    fn base_cfg() -> NodeConfig {
        NodeConfig {
            dms_base_url: Url::parse("https://dms.example").unwrap(),
            node_version: "1.0.0".into(),
            request_timeout_secs: 10,
            dds_base_url: None,
            node_url: None,
            reg_secret: None,
            secp256k1_privhex: None,
            heartbeat_jitter_ms: 250,
            poll_backoff_ms_min: 1000,
            poll_backoff_ms_max: 30000,
            token_safety_ratio: 0.75,
            token_reauth_max_retries: 3,
            token_reauth_jitter_ms: 500,
            register_interval_secs: None,
            register_max_retry: None,
            max_concurrency: 1,
            log_format: LogFormat::Json,
            enable_noop: true,
            noop_sleep_secs: 1,
        }
    }

    #[test]
    fn settings_none_without_required_fields() {
        let cfg = base_cfg();
        assert!(registration_settings(&cfg, &[MOCK_CAPABILITY.into()]).is_none());
    }

    #[test]
    fn settings_present_when_all_fields_available() {
        let mut cfg = base_cfg();
        cfg.dds_base_url = Some(Url::parse("https://dds.example").unwrap());
        cfg.node_url = Some(Url::parse("https://node.example").unwrap());
        cfg.reg_secret = Some("super-secret".into());
        cfg.secp256k1_privhex =
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());
        cfg.register_interval_secs = Some(60);
        cfg.register_max_retry = Some(3);
        cfg.request_timeout_secs = 0;

        let settings = registration_settings(&cfg, &[MOCK_CAPABILITY.into()]).expect("settings");
        assert_eq!(settings.dds_base_url, "https://dds.example/");
        assert_eq!(settings.node_url, "https://node.example/");
        assert_eq!(settings.node_version, "1.0.0");
        assert_eq!(settings.reg_secret, "super-secret");
        assert_eq!(
            settings.secp256k1_privhex,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(settings.register_interval_secs, 60);
        assert_eq!(settings.max_retry, 3);
        assert_eq!(settings.request_timeout_secs, 1);
        assert_eq!(settings.capabilities, vec![MOCK_CAPABILITY.to_string()]);
    }

    #[test]
    fn node_version_normalized_without_leading_v() {
        let mut cfg = base_cfg();
        cfg.dds_base_url = Some(Url::parse("https://dds.example").unwrap());
        cfg.node_url = Some(Url::parse("https://node.example").unwrap());
        cfg.reg_secret = Some("secret".into());
        cfg.secp256k1_privhex =
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());
        cfg.node_version = "v2.3.4".into();

        let settings = registration_settings(&cfg, &[MOCK_CAPABILITY.into()]).expect("settings");
        assert_eq!(settings.node_version, "2.3.4");
    }

    #[test]
    fn node_version_falls_back_when_invalid() {
        let mut cfg = base_cfg();
        cfg.dds_base_url = Some(Url::parse("https://dds.example").unwrap());
        cfg.node_url = Some(Url::parse("https://node.example").unwrap());
        cfg.reg_secret = Some("secret".into());
        cfg.secp256k1_privhex =
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());
        cfg.node_version = "not-a-semver".into();

        let caps = vec![MOCK_CAPABILITY.to_string()];
        let settings = registration_settings(&cfg, &caps).expect("settings");
        assert_eq!(settings.node_version, env!("CARGO_PKG_VERSION").to_string());
    }

    #[test]
    fn negative_one_max_retry_interpreted_as_infinite() {
        let mut cfg = base_cfg();
        cfg.dds_base_url = Some(Url::parse("https://dds.example").unwrap());
        cfg.node_url = Some(Url::parse("https://node.example").unwrap());
        cfg.reg_secret = Some("secret".into());
        cfg.secp256k1_privhex =
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());
        cfg.register_max_retry = Some(-1);

        let caps = vec![MOCK_CAPABILITY.to_string()];
        let settings = registration_settings(&cfg, &caps).expect("settings");
        assert_eq!(settings.max_retry, -1);
    }

    #[tokio::test]
    async fn spawn_skips_when_missing_config() {
        let cfg = base_cfg();
        spawn_registration_if_configured(&cfg, &[]).unwrap();
    }

    #[tokio::test]
    async fn spawn_returns_ok_when_key_invalid() {
        let mut cfg = base_cfg();
        cfg.dds_base_url = Some(Url::parse("https://dds.example").unwrap());
        cfg.node_url = Some(Url::parse("https://node.example").unwrap());
        cfg.reg_secret = Some("super-secret".into());
        cfg.secp256k1_privhex = Some("not-a-hex-key".into());

        let caps = vec![MOCK_CAPABILITY.to_string()];
        spawn_registration_if_configured(&cfg, &caps).unwrap();
    }
}
