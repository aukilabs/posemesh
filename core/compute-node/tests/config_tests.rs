use once_cell::sync::Lazy;
use posemesh_compute_node::config::{LogFormat, NodeConfig};
use std::sync::Mutex;

static ENV_GUARD: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn clear(keys: &[&str]) {
    for k in keys {
        std::env::remove_var(k);
    }
}

#[test]
fn loads_required_siwe_defaults() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "DMS_BASE_URL",
        "REQUEST_TIMEOUT_SECS",
        "NODE_VERSION",
        "HEARTBEAT_JITTER_MS",
        "POLL_BACKOFF_MS_MIN",
        "POLL_BACKOFF_MS_MAX",
        "TOKEN_SAFETY_RATIO",
        "TOKEN_REAUTH_MAX_RETRIES",
        "TOKEN_REAUTH_JITTER_MS",
        "REGISTER_INTERVAL_SECS",
        "REGISTER_MAX_RETRY",
        "MAX_CONCURRENCY",
        "LOG_FORMAT",
        "ENABLE_NOOP",
        "NOOP_SLEEP_SECS",
        "DDS_BASE_URL",
        "NODE_URL",
        "SECP256K1_PRIVHEX",
        "REG_SECRET",
    ]);

    std::env::set_var("DMS_BASE_URL", "https://dms.example");
    std::env::set_var("REQUEST_TIMEOUT_SECS", "15");
    std::env::set_var("DDS_BASE_URL", "https://dds.example");
    std::env::set_var("NODE_URL", "https://node.example");
    std::env::set_var("REG_SECRET", "super-secret");
    std::env::set_var("SECP256K1_PRIVHEX", "abcdef");

    let cfg = NodeConfig::from_env().expect("config");
    assert_eq!(cfg.dms_base_url.as_str(), "https://dms.example/");
    assert_eq!(cfg.node_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(cfg.request_timeout_secs, 15);
    assert_eq!(
        cfg.dds_base_url.as_ref().unwrap().as_str(),
        "https://dds.example/"
    );
    assert_eq!(
        cfg.node_url.as_ref().unwrap().as_str(),
        "https://node.example/"
    );
    assert_eq!(cfg.reg_secret.as_deref(), Some("super-secret"));
    assert_eq!(cfg.secp256k1_privhex.as_deref(), Some("abcdef"));
    assert_eq!(cfg.heartbeat_jitter_ms, 250);
    assert_eq!(cfg.poll_backoff_ms_min, 1000);
    assert_eq!(cfg.poll_backoff_ms_max, 30000);
    assert!((cfg.token_safety_ratio - 0.75).abs() < f32::EPSILON);
    assert_eq!(cfg.token_reauth_max_retries, 3);
    assert_eq!(cfg.token_reauth_jitter_ms, 500);
    assert_eq!(cfg.register_interval_secs, None);
    assert_eq!(cfg.register_max_retry, None);
    assert_eq!(cfg.max_concurrency, 1);
    assert_eq!(cfg.log_format, LogFormat::Json);
    assert!(!cfg.enable_noop);
    assert_eq!(cfg.noop_sleep_secs, 5);
}

#[test]
fn missing_siwe_fields_fails() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "DMS_BASE_URL",
        "REQUEST_TIMEOUT_SECS",
        "DDS_BASE_URL",
        "NODE_URL",
        "SECP256K1_PRIVHEX",
        "REG_SECRET",
    ]);

    std::env::set_var("DMS_BASE_URL", "https://dms.example");
    std::env::set_var("REQUEST_TIMEOUT_SECS", "10");

    let err = NodeConfig::from_env().expect_err("should error");
    let msg = format!("{}", err);
    assert!(msg.contains("DDS_BASE_URL required"));
}

#[test]
fn log_format_text_is_parsed() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "DMS_BASE_URL",
        "REQUEST_TIMEOUT_SECS",
        "LOG_FORMAT",
        "DDS_BASE_URL",
        "NODE_URL",
        "SECP256K1_PRIVHEX",
        "REG_SECRET",
    ]);

    std::env::set_var("DMS_BASE_URL", "https://dms.example");
    std::env::set_var("REQUEST_TIMEOUT_SECS", "10");
    std::env::set_var("LOG_FORMAT", "text");
    std::env::set_var("DDS_BASE_URL", "https://dds.example");
    std::env::set_var("NODE_URL", "https://node.example");
    std::env::set_var("REG_SECRET", "secret");
    std::env::set_var("SECP256K1_PRIVHEX", "abcdef");

    let cfg = NodeConfig::from_env().expect("config");
    assert_eq!(cfg.log_format, LogFormat::Text);
}
