use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{env, fmt, fs};
use url::Url;

const DEFAULT_DMS_BASE_URL: &str = "https://dms.auki.network/v1";
const DEFAULT_DDS_BASE_URL: &str = "https://dds.auki.network";
const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 60;
const DEFAULT_REGISTER_INTERVAL_SECS: u64 = 120;
const DEFAULT_REGISTER_MAX_RETRY: i32 = -1;
const DEFAULT_HEARTBEAT_MIN_RATIO: f64 = 0.25;
const DEFAULT_HEARTBEAT_MAX_RATIO: f64 = 0.35;

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
    pub heartbeat_min_ratio: f64,
    pub heartbeat_max_ratio: f64,
    pub poll_backoff_ms_min: u64,
    pub poll_backoff_ms_max: u64,
    pub token_safety_ratio: f32,
    pub token_reauth_max_retries: u32,
    pub token_reauth_jitter_ms: u64,
    #[serde(default)]
    pub auki_p2p_enabled: bool,
    #[serde(default)]
    pub auki_p2p_listen_multiaddrs: Vec<String>,
    #[serde(default)]
    pub auki_p2p_advertised_multiaddrs: Vec<String>,
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
        let heartbeat_min_ratio =
            parse_f64_opt("HEARTBEAT_MIN_RATIO", DEFAULT_HEARTBEAT_MIN_RATIO)?;
        let heartbeat_max_ratio =
            parse_f64_opt("HEARTBEAT_MAX_RATIO", DEFAULT_HEARTBEAT_MAX_RATIO)?;
        let poll_backoff_ms_min = parse_u64_opt("POLL_BACKOFF_MS_MIN", 1000)?;
        let poll_backoff_ms_max = parse_u64_opt("POLL_BACKOFF_MS_MAX", 30000)?;
        let token_safety_ratio = parse_f32_opt("TOKEN_SAFETY_RATIO", 0.75)?;
        let token_reauth_max_retries = parse_u32_opt("TOKEN_REAUTH_MAX_RETRIES", 3)?;
        let token_reauth_jitter_ms = parse_u64_opt("TOKEN_REAUTH_JITTER_MS", 500)?;
        let auki_p2p_enabled = parse_bool_opt("AUKI_P2P_ENABLED", false)?;
        let auki_p2p_listen_multiaddrs = parse_csv_opt("AUKI_P2P_LISTEN_MULTIADDRS");
        let auki_p2p_advertised_multiaddrs = parse_csv_opt("AUKI_P2P_ADVERTISED_MULTIADDRS");
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
            heartbeat_min_ratio,
            heartbeat_max_ratio,
            poll_backoff_ms_min,
            poll_backoff_ms_max,
            token_safety_ratio,
            token_reauth_max_retries,
            token_reauth_jitter_ms,
            auki_p2p_enabled,
            auki_p2p_listen_multiaddrs,
            auki_p2p_advertised_multiaddrs,
            register_interval_secs,
            register_max_retry,
            max_concurrency,
            log_format,
            enable_noop,
            noop_sleep_secs,
        })
    }
}

/// Configuration for the opt-in robot machine-authentication entrypoint.
///
/// This is deliberately separate from [`NodeConfig`] so the existing SIWE
/// configuration surface and public struct shape remain unchanged. Complete
/// registration credentials are kept private and are always redacted from
/// debug output.
#[derive(Clone, PartialEq)]
pub struct RobotNodeConfig {
    // Core settings (defaults available).
    pub dms_base_url: Url,
    pub node_version: String,
    pub request_timeout_secs: u64,

    // Robot machine authentication.
    pub dds_base_url: Url,
    registration_credentials: String,

    // Optional runtime tuning shared with the SIWE entrypoint.
    pub heartbeat_jitter_ms: u64,
    pub heartbeat_min_ratio: f64,
    pub heartbeat_max_ratio: f64,
    pub poll_backoff_ms_min: u64,
    pub poll_backoff_ms_max: u64,
    pub token_safety_ratio: f32,
    pub token_reauth_max_retries: u32,
    pub token_reauth_jitter_ms: u64,
    pub auki_p2p_enabled: bool,
    pub auki_p2p_listen_multiaddrs: Vec<String>,
    pub auki_p2p_advertised_multiaddrs: Vec<String>,
    pub max_concurrency: u32,
    pub log_format: LogFormat,
    pub enable_noop: bool,
    pub noop_sleep_secs: u64,
}

impl fmt::Debug for RobotNodeConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RobotNodeConfig")
            .field("dms_base_url", &self.dms_base_url)
            .field("node_version", &self.node_version)
            .field("request_timeout_secs", &self.request_timeout_secs)
            .field("dds_base_url", &self.dds_base_url)
            .field("registration_credentials", &"[REDACTED]")
            .field("heartbeat_jitter_ms", &self.heartbeat_jitter_ms)
            .field("heartbeat_min_ratio", &self.heartbeat_min_ratio)
            .field("heartbeat_max_ratio", &self.heartbeat_max_ratio)
            .field("poll_backoff_ms_min", &self.poll_backoff_ms_min)
            .field("poll_backoff_ms_max", &self.poll_backoff_ms_max)
            .field("token_safety_ratio", &self.token_safety_ratio)
            .field("token_reauth_max_retries", &self.token_reauth_max_retries)
            .field("token_reauth_jitter_ms", &self.token_reauth_jitter_ms)
            .field("auki_p2p_enabled", &self.auki_p2p_enabled)
            .field(
                "auki_p2p_listen_multiaddrs",
                &self.auki_p2p_listen_multiaddrs,
            )
            .field(
                "auki_p2p_advertised_multiaddrs",
                &self.auki_p2p_advertised_multiaddrs,
            )
            .field("max_concurrency", &self.max_concurrency)
            .field("log_format", &self.log_format)
            .field("enable_noop", &self.enable_noop)
            .field("noop_sleep_secs", &self.noop_sleep_secs)
            .finish()
    }
}

impl RobotNodeConfig {
    /// Construct robot configuration without reading process-global state.
    ///
    /// Operational fields receive the same defaults as [`Self::from_env`] and
    /// remain publicly overridable. The opaque credential stays private and is
    /// redacted by this type's [`fmt::Debug`] implementation.
    pub fn new(
        dds_base_url: Url,
        dms_base_url: Url,
        registration_credentials: impl Into<String>,
    ) -> Result<Self> {
        let registration_credentials = registration_credentials.into().trim().to_string();
        if registration_credentials.is_empty() {
            bail!("robot registration credentials must not be empty");
        }

        Ok(Self {
            dms_base_url,
            node_version: env!("CARGO_PKG_VERSION").to_string(),
            request_timeout_secs: DEFAULT_REQUEST_TIMEOUT_SECS,
            dds_base_url,
            registration_credentials,
            heartbeat_jitter_ms: 250,
            heartbeat_min_ratio: DEFAULT_HEARTBEAT_MIN_RATIO,
            heartbeat_max_ratio: DEFAULT_HEARTBEAT_MAX_RATIO,
            poll_backoff_ms_min: 1000,
            poll_backoff_ms_max: 30000,
            token_safety_ratio: 0.75,
            token_reauth_max_retries: 3,
            token_reauth_jitter_ms: 500,
            auki_p2p_enabled: false,
            auki_p2p_listen_multiaddrs: Vec::new(),
            auki_p2p_advertised_multiaddrs: Vec::new(),
            max_concurrency: 1,
            log_format: LogFormat::default(),
            enable_noop: false,
            noop_sleep_secs: 5,
        })
    }

    /// Load the robot entrypoint configuration from environment variables.
    ///
    /// Unlike [`NodeConfig::from_env`], this requires only opaque robot
    /// credentials, supplied either inline or through a file. It never reads
    /// or falls back to the legacy SIWE secret or secp256k1 private key.
    pub fn from_env() -> Result<Self> {
        let dms_base_url = parse_url_default("DMS_BASE_URL", DEFAULT_DMS_BASE_URL)?;
        let dds_base_url = parse_url_default("DDS_BASE_URL", DEFAULT_DDS_BASE_URL)?;
        let registration_credentials = robot_registration_credentials_from_env()?;

        let mut cfg = Self::new(dds_base_url, dms_base_url, registration_credentials)?;
        cfg.request_timeout_secs =
            parse_u64_default("REQUEST_TIMEOUT_SECS", DEFAULT_REQUEST_TIMEOUT_SECS)?;
        cfg.node_version = env_var_trimmed("NODE_VERSION")
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
        cfg.heartbeat_jitter_ms = parse_u64_opt("HEARTBEAT_JITTER_MS", 250)?;
        cfg.heartbeat_min_ratio =
            parse_f64_opt("HEARTBEAT_MIN_RATIO", DEFAULT_HEARTBEAT_MIN_RATIO)?;
        cfg.heartbeat_max_ratio =
            parse_f64_opt("HEARTBEAT_MAX_RATIO", DEFAULT_HEARTBEAT_MAX_RATIO)?;
        cfg.poll_backoff_ms_min = parse_u64_opt("POLL_BACKOFF_MS_MIN", 1000)?;
        cfg.poll_backoff_ms_max = parse_u64_opt("POLL_BACKOFF_MS_MAX", 30000)?;
        cfg.token_safety_ratio = parse_f32_opt("TOKEN_SAFETY_RATIO", 0.75)?;
        cfg.token_reauth_max_retries = parse_u32_opt("TOKEN_REAUTH_MAX_RETRIES", 3)?;
        cfg.token_reauth_jitter_ms = parse_u64_opt("TOKEN_REAUTH_JITTER_MS", 500)?;
        cfg.auki_p2p_enabled = parse_bool_opt("AUKI_P2P_ENABLED", false)?;
        cfg.auki_p2p_listen_multiaddrs = parse_csv_opt("AUKI_P2P_LISTEN_MULTIADDRS");
        cfg.auki_p2p_advertised_multiaddrs = parse_csv_opt("AUKI_P2P_ADVERTISED_MULTIADDRS");
        cfg.max_concurrency = parse_u32_opt("MAX_CONCURRENCY", 1)?;
        cfg.log_format = parse_log_format("LOG_FORMAT").unwrap_or_default();
        cfg.enable_noop = parse_bool_opt("ENABLE_NOOP", false)?;
        cfg.noop_sleep_secs = parse_u64_opt("NOOP_SLEEP_SECS", 5)?;

        Ok(cfg)
    }

    pub(crate) fn registration_credentials(&self) -> &str {
        &self.registration_credentials
    }

    /// Produce the existing engine configuration used by the shared runner,
    /// heartbeat, storage, and DMS loop. SIWE-only fields remain empty and are
    /// never consulted by the robot entrypoint.
    pub(crate) fn runtime_config(&self) -> NodeConfig {
        NodeConfig {
            dms_base_url: self.dms_base_url.clone(),
            node_version: self.node_version.clone(),
            request_timeout_secs: self.request_timeout_secs,
            dds_base_url: Some(self.dds_base_url.clone()),
            reg_secret: None,
            secp256k1_privhex: None,
            heartbeat_jitter_ms: self.heartbeat_jitter_ms,
            heartbeat_min_ratio: self.heartbeat_min_ratio,
            heartbeat_max_ratio: self.heartbeat_max_ratio,
            poll_backoff_ms_min: self.poll_backoff_ms_min,
            poll_backoff_ms_max: self.poll_backoff_ms_max,
            token_safety_ratio: self.token_safety_ratio,
            token_reauth_max_retries: self.token_reauth_max_retries,
            token_reauth_jitter_ms: self.token_reauth_jitter_ms,
            auki_p2p_enabled: self.auki_p2p_enabled,
            auki_p2p_listen_multiaddrs: self.auki_p2p_listen_multiaddrs.clone(),
            auki_p2p_advertised_multiaddrs: self.auki_p2p_advertised_multiaddrs.clone(),
            register_interval_secs: None,
            register_max_retry: None,
            max_concurrency: self.max_concurrency,
            log_format: self.log_format,
            enable_noop: self.enable_noop,
            noop_sleep_secs: self.noop_sleep_secs,
        }
    }
}

fn robot_registration_credentials_from_env() -> Result<String> {
    let inline = env::var("ROBOT_REGISTRATION_CREDENTIALS").ok();
    let file = env::var("ROBOT_REGISTRATION_CREDENTIALS_FILE").ok();

    match (inline, file) {
        (Some(_), Some(_)) => bail!(
            "ROBOT_REGISTRATION_CREDENTIALS and ROBOT_REGISTRATION_CREDENTIALS_FILE are mutually exclusive"
        ),
        (Some(credentials), None) => {
            let credentials = credentials.trim().to_string();
            if credentials.is_empty() {
                bail!("ROBOT_REGISTRATION_CREDENTIALS must not be empty");
            }
            Ok(credentials)
        }
        (None, Some(path)) => {
            let path = path.trim();
            if path.is_empty() {
                bail!("ROBOT_REGISTRATION_CREDENTIALS_FILE must not be empty");
            }
            let credentials = fs::read_to_string(path).with_context(|| {
                format!("read robot registration credentials file {path}")
            })?;
            let credentials = credentials.trim().to_string();
            if credentials.is_empty() {
                bail!("robot registration credentials file must not be empty");
            }
            Ok(credentials)
        }
        (None, None) => bail!(
            "ROBOT_REGISTRATION_CREDENTIALS or ROBOT_REGISTRATION_CREDENTIALS_FILE required for robot machine authentication"
        ),
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

fn parse_f64_opt(key: &str, default: f64) -> Result<f64> {
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

fn parse_csv_opt(key: &str) -> Vec<String> {
    env_var_trimmed(key)
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_log_format(key: &str) -> Option<LogFormat> {
    match env::var(key).ok()?.to_lowercase().as_str() {
        "json" => Some(LogFormat::Json),
        "text" => Some(LogFormat::Text),
        _ => None,
    }
}
