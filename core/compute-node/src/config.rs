use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use url::Url;

const DEFAULT_DMS_BASE_URL: &str = "https://dms.auki.network/v1";
const DEFAULT_DDS_BASE_URL: &str = "https://dds.auki.network";
const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 60;
const DEFAULT_REGISTER_INTERVAL_SECS: u64 = 120;
const DEFAULT_REGISTER_MAX_RETRY: i32 = -1;

/// Log output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    #[default]
    Json,
    Text,
}

/// Node configuration loaded from environment (SPECS §8 Configuration).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeConfig {
    // Core settings (defaults available).
    pub dms_base_url: Url,
    pub node_version: String,
    pub request_timeout_secs: u64,

    // Auth: either static node identity or SIWE via DDS
    pub dds_base_url: Option<Url>,
    pub reg_secret: Option<String>,
    pub secp256k1_privhex: Option<String>,

    // Optional
    pub heartbeat_jitter_ms: u64,
    pub poll_backoff_ms_min: u64,
    pub poll_backoff_ms_max: u64,
    pub token_safety_ratio: f32,
    pub token_reauth_max_retries: u32,
    pub token_reauth_jitter_ms: u64,
    pub register_interval_secs: Option<u64>,
    pub register_max_retry: Option<i32>,
    pub max_concurrency: u32,
    pub log_format: LogFormat,
    pub enable_noop: bool,
    pub noop_sleep_secs: u64,
}

impl NodeConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        // Core settings (defaults when unset).
        let dms_base_url = parse_url_default("DMS_BASE_URL", DEFAULT_DMS_BASE_URL)?;
        let request_timeout_secs =
            parse_u64_default("REQUEST_TIMEOUT_SECS", DEFAULT_REQUEST_TIMEOUT_SECS)?;
        let node_version = env::var("NODE_VERSION")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

        // Auth options
        let dds_base_url = parse_url_default("DDS_BASE_URL", DEFAULT_DDS_BASE_URL)?;
        let reg_secret = env::var("REG_SECRET")
            .with_context(|| "REG_SECRET required for DDS SIWE authentication")?
            .trim()
            .to_string();
        if reg_secret.is_empty() {
            bail!("REG_SECRET required for DDS SIWE authentication");
        }
        let secp256k1_privhex = env::var("SECP256K1_PRIVHEX")
            .with_context(|| "SECP256K1_PRIVHEX required for DDS SIWE authentication")?
            .trim()
            .to_string();
        if secp256k1_privhex.is_empty() {
            bail!("SECP256K1_PRIVHEX required for DDS SIWE authentication");
        }

        // Optional
        let heartbeat_jitter_ms = parse_u64_opt("HEARTBEAT_JITTER_MS", 250)?;
        let poll_backoff_ms_min = parse_u64_opt("POLL_BACKOFF_MS_MIN", 1000)?;
        let poll_backoff_ms_max = parse_u64_opt("POLL_BACKOFF_MS_MAX", 30000)?;
        let token_safety_ratio = parse_f32_opt("TOKEN_SAFETY_RATIO", 0.75)?;
        let token_reauth_max_retries = parse_u32_opt("TOKEN_REAUTH_MAX_RETRIES", 3)?;
        let token_reauth_jitter_ms = parse_u64_opt("TOKEN_REAUTH_JITTER_MS", 500)?;
        let register_interval_secs = Some(parse_u64_default(
            "REGISTER_INTERVAL_SECS",
            DEFAULT_REGISTER_INTERVAL_SECS,
        )?);
        let register_max_retry = Some(parse_i32_default(
            "REGISTER_MAX_RETRY",
            DEFAULT_REGISTER_MAX_RETRY,
        )?);
        let max_concurrency = parse_u32_opt("MAX_CONCURRENCY", 1)?;
        let log_format = parse_log_format("LOG_FORMAT").unwrap_or_default();
        let enable_noop = parse_bool_opt("ENABLE_NOOP", false)?;
        let noop_sleep_secs = parse_u64_opt("NOOP_SLEEP_SECS", 5)?;

        Ok(Self {
            dms_base_url,
            node_version,
            request_timeout_secs,
            dds_base_url: Some(dds_base_url),
            reg_secret: Some(reg_secret),
            secp256k1_privhex: Some(secp256k1_privhex),
            heartbeat_jitter_ms,
            poll_backoff_ms_min,
            poll_backoff_ms_max,
            token_safety_ratio,
            token_reauth_max_retries,
            token_reauth_jitter_ms,
            register_interval_secs,
            register_max_retry,
            max_concurrency,
            log_format,
            enable_noop,
            noop_sleep_secs,
        })
    }
}

fn env_var_trimmed(key: &str) -> Option<String> {
    env::var(key).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn parse_url_default(key: &str, default: &str) -> Result<Url> {
    let raw = env_var_trimmed(key).unwrap_or_else(|| default.to_string());
    Url::parse(&raw).with_context(|| format!("invalid URL in {key}"))
}

fn parse_u64_default(key: &str, default: u64) -> Result<u64> {
    match env_var_trimmed(key) {
        Some(value) => value
            .parse()
            .with_context(|| format!("invalid integer in {key}")),
        None => Ok(default),
    }
}

fn parse_u64_opt(key: &str, default: u64) -> Result<u64> {
    match env::var(key) {
        Ok(v) => v
            .parse()
            .with_context(|| format!("invalid integer in {key}")),
        Err(_) => Ok(default),
    }
}

fn parse_u32_opt(key: &str, default: u32) -> Result<u32> {
    match env::var(key) {
        Ok(v) => v
            .parse()
            .with_context(|| format!("invalid integer in {key}")),
        Err(_) => Ok(default),
    }
}

fn parse_i32_default(key: &str, default: i32) -> Result<i32> {
    match env_var_trimmed(key) {
        Some(value) => {
            let parsed: i32 = value
                .parse()
                .with_context(|| format!("invalid integer in {key}"))?;
            if parsed < -1 {
                bail!("{key} must be -1 or a non-negative integer, got {parsed}");
            }
            Ok(parsed)
        }
        None => Ok(default),
    }
}

fn parse_f32_opt(key: &str, default: f32) -> Result<f32> {
    match env::var(key) {
        Ok(v) => v.parse().with_context(|| format!("invalid float in {key}")),
        Err(_) => Ok(default),
    }
}

fn parse_bool_opt(key: &str, default: bool) -> Result<bool> {
    match env::var(key) {
        Ok(v) => v
            .parse::<bool>()
            .with_context(|| format!("invalid bool in {key}; expected true/false")),
        Err(_) => Ok(default),
    }
}

fn parse_log_format(key: &str) -> Option<LogFormat> {
    match env::var(key).ok()?.to_lowercase().as_str() {
        "json" => Some(LogFormat::Json),
        "text" => Some(LogFormat::Text),
        _ => None,
    }
}
