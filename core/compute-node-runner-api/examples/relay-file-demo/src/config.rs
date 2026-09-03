use std::{env, fs, path::PathBuf, sync::Arc, time::Duration};

use anyhow::{bail, Context, Result};
use url::Url;
use uuid::Uuid;

const DEFAULT_REFERENCE_FILE: &str = "/tmp/posemesh-relay-file-demo.reference.json";
const DEFAULT_OUTPUT_FILE: &str = "/tmp/posemesh-relay-file-demo.download";
const DEFAULT_AVAILABILITY_SECONDS: u64 = 600;
const DEFAULT_NODE_WAIT_SECONDS: u64 = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskMode {
    Public,
    Dedicated,
}

impl TaskMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Dedicated => "dedicated",
        }
    }
}

#[derive(Clone)]
pub struct JobConfig {
    pub dms_base_url: Url,
    pub app_jwt: Arc<str>,
    pub domain_id: Uuid,
    pub reconstruction_mode: TaskMode,
    pub node_wait: Duration,
}

impl std::fmt::Debug for JobConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobConfig")
            .field("dms_base_url", &self.dms_base_url)
            .field("app_jwt", &"[REDACTED]")
            .field("domain_id", &self.domain_id)
            .field("reconstruction_mode", &self.reconstruction_mode)
            .field("node_wait", &self.node_wait)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct RobotDemoConfig {
    pub source_file: PathBuf,
    pub reference_file: PathBuf,
    pub availability: Duration,
    pub jobs: JobConfig,
}

impl RobotDemoConfig {
    pub fn from_env() -> Result<Self> {
        validate_dev_endpoints()?;
        let source_file = required_path("RELAY_DEMO_SOURCE_FILE")?;
        let reference_file = env_trimmed("RELAY_DEMO_REFERENCE_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_REFERENCE_FILE));
        let availability_seconds = parse_u64(
            "RELAY_DEMO_AVAILABILITY_SECONDS",
            DEFAULT_AVAILABILITY_SECONDS,
        )?;
        if !(60..=86_400).contains(&availability_seconds) {
            bail!("RELAY_DEMO_AVAILABILITY_SECONDS must be between 60 and 86400");
        }

        Ok(Self {
            source_file,
            reference_file,
            availability: Duration::from_secs(availability_seconds),
            jobs: job_config_from_env()?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ReconstructionDemoConfig {
    pub output_file: PathBuf,
}

impl ReconstructionDemoConfig {
    pub fn from_env() -> Result<Self> {
        validate_dev_endpoints()?;
        Ok(Self {
            output_file: env_trimmed("RELAY_DEMO_OUTPUT_FILE")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT_FILE)),
        })
    }
}

pub fn load_env_file(default_name: &str) -> Result<PathBuf> {
    let mut args = env::args_os().skip(1);
    let path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default_name));
    if args.next().is_some() {
        bail!("expected at most one argument: path to the .env file");
    }
    dotenvy::from_path(&path)
        .with_context(|| format!("load environment file {}", path.display()))?;
    Ok(path)
}

fn job_config_from_env() -> Result<JobConfig> {
    let dms_base_url = required_url("DMS_BASE_URL")?;
    let domain_id = env_trimmed("DOMAIN_ID")
        .context("DOMAIN_ID is required")?
        .parse::<Uuid>()
        .context("DOMAIN_ID must be a UUID")?;
    let app_jwt = secret_from_env_or_file("APP_JWT", "APP_JWT_FILE")?;
    let reconstruction_mode = match env_trimmed("RELAY_DEMO_RECONSTRUCTION_TASK_MODE").as_deref() {
        None | Some("dedicated") => TaskMode::Dedicated,
        Some("public") => TaskMode::Public,
        Some(_) => bail!("RELAY_DEMO_RECONSTRUCTION_TASK_MODE must be public or dedicated"),
    };
    let node_wait_seconds = parse_u64("RELAY_DEMO_NODE_WAIT_SECONDS", DEFAULT_NODE_WAIT_SECONDS)?;
    if !(5..=600).contains(&node_wait_seconds) {
        bail!("RELAY_DEMO_NODE_WAIT_SECONDS must be between 5 and 600");
    }

    Ok(JobConfig {
        dms_base_url,
        app_jwt: Arc::from(app_jwt),
        domain_id,
        reconstruction_mode,
        node_wait: Duration::from_secs(node_wait_seconds),
    })
}

fn validate_dev_endpoints() -> Result<()> {
    let dds = required_url("DDS_BASE_URL")?;
    let dms = required_url("DMS_BASE_URL")?;
    if parse_bool("RELAY_DEMO_ALLOW_NON_DEV", false)? {
        return Ok(());
    }
    if !is_exact_dev_endpoint(&dds, "dds.dev.aukiverse.com", "")
        || !is_exact_dev_endpoint(&dms, "dms.dev.aukiverse.com", "/v1")
    {
        bail!(
            "relay-file-demo is dev-only; DDS_BASE_URL and DMS_BASE_URL must be the canonical HTTPS dev endpoints (set RELAY_DEMO_ALLOW_NON_DEV=true only for an intentional non-dev test)"
        );
    }
    Ok(())
}

fn is_exact_dev_endpoint(url: &Url, host: &str, path: &str) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some(host)
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url.path().trim_end_matches('/') == path
}

fn required_path(key: &'static str) -> Result<PathBuf> {
    env_trimmed(key)
        .map(PathBuf::from)
        .with_context(|| format!("{key} is required"))
}

fn required_url(key: &'static str) -> Result<Url> {
    let value = env_trimmed(key).with_context(|| format!("{key} is required"))?;
    Url::parse(&value).with_context(|| format!("invalid URL in {key}"))
}

fn secret_from_env_or_file(inline_key: &'static str, file_key: &'static str) -> Result<String> {
    match (env_trimmed(inline_key), env_trimmed(file_key)) {
        (Some(_), Some(_)) => bail!("{inline_key} and {file_key} are mutually exclusive"),
        (Some(value), None) => Ok(value),
        (None, Some(path)) => {
            let value = fs::read_to_string(&path)
                .with_context(|| format!("read {file_key} file {path}"))?;
            let value = value.trim().to_string();
            if value.is_empty() {
                bail!("{file_key} file is empty");
            }
            Ok(value)
        }
        (None, None) => bail!("{inline_key} or {file_key} is required"),
    }
}

fn env_trimmed(key: &str) -> Option<String> {
    env::var(key).ok().and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

fn parse_u64(key: &'static str, default: u64) -> Result<u64> {
    match env_trimmed(key) {
        Some(value) => value
            .parse::<u64>()
            .with_context(|| format!("invalid integer in {key}")),
        None => Ok(default),
    }
}

fn parse_bool(key: &'static str, default: bool) -> Result<bool> {
    match env_trimmed(key).as_deref() {
        None => Ok(default),
        Some("true" | "1") => Ok(true),
        Some("false" | "0") => Ok(false),
        Some(_) => bail!("{key} must be true or false"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_modes_have_exact_wire_values() {
        assert_eq!(TaskMode::Public.as_str(), "public");
        assert_eq!(TaskMode::Dedicated.as_str(), "dedicated");
    }

    #[test]
    fn canonical_dev_endpoint_check_rejects_lookalikes_and_plaintext() {
        assert!(is_exact_dev_endpoint(
            &Url::parse("https://dds.dev.aukiverse.com/").unwrap(),
            "dds.dev.aukiverse.com",
            ""
        ));
        assert!(is_exact_dev_endpoint(
            &Url::parse("https://dms.dev.aukiverse.com/v1/").unwrap(),
            "dms.dev.aukiverse.com",
            "/v1"
        ));
        assert!(!is_exact_dev_endpoint(
            &Url::parse("http://dds.dev.aukiverse.com").unwrap(),
            "dds.dev.aukiverse.com",
            ""
        ));
        assert!(!is_exact_dev_endpoint(
            &Url::parse("https://dms.dev.aukiverse.com.evil.test/v1").unwrap(),
            "dms.dev.aukiverse.com",
            "/v1"
        ));
    }
}
