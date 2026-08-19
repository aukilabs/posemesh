use once_cell::sync::Lazy;
use posemesh_compute_node::config::{LogFormat, NodeConfig, RobotNodeConfig};
use std::sync::Mutex;
use tempfile::NamedTempFile;

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
        "HEARTBEAT_MIN_RATIO",
        "HEARTBEAT_MAX_RATIO",
        "POLL_BACKOFF_MS_MIN",
        "POLL_BACKOFF_MS_MAX",
        "TOKEN_SAFETY_RATIO",
        "TOKEN_REAUTH_MAX_RETRIES",
        "TOKEN_REAUTH_JITTER_MS",
        "AUKI_P2P_ENABLED",
        "AUKI_P2P_LISTEN_MULTIADDRS",
        "AUKI_P2P_ADVERTISED_MULTIADDRS",
        "REGISTER_INTERVAL_SECS",
        "REGISTER_MAX_RETRY",
        "MAX_CONCURRENCY",
        "LOG_FORMAT",
        "ENABLE_NOOP",
        "NOOP_SLEEP_SECS",
        "DDS_BASE_URL",
        "SECP256K1_PRIVHEX",
        "REG_SECRET",
    ]);

    std::env::set_var("REG_SECRET", "super-secret");
    std::env::set_var("SECP256K1_PRIVHEX", "abcdef");

    let cfg = NodeConfig::from_env().expect("config");
    assert_eq!(cfg.dms_base_url.as_str(), "https://dms.auki.network/v1");
    assert_eq!(cfg.node_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(cfg.request_timeout_secs, 60);
    assert_eq!(
        cfg.dds_base_url.as_ref().unwrap().as_str(),
        "https://dds.auki.network/"
    );
    assert_eq!(cfg.reg_secret.as_deref(), Some("super-secret"));
    assert_eq!(cfg.secp256k1_privhex.as_deref(), Some("abcdef"));
    assert_eq!(cfg.heartbeat_jitter_ms, 250);
    assert!((cfg.heartbeat_min_ratio - 0.25).abs() < f64::EPSILON);
    assert!((cfg.heartbeat_max_ratio - 0.35).abs() < f64::EPSILON);
    assert_eq!(cfg.poll_backoff_ms_min, 1000);
    assert_eq!(cfg.poll_backoff_ms_max, 30000);
    assert!((cfg.token_safety_ratio - 0.75).abs() < f32::EPSILON);
    assert_eq!(cfg.token_reauth_max_retries, 3);
    assert_eq!(cfg.token_reauth_jitter_ms, 500);
    assert!(!cfg.auki_p2p_enabled);
    assert!(cfg.auki_p2p_listen_multiaddrs.is_empty());
    assert!(cfg.auki_p2p_advertised_multiaddrs.is_empty());
    assert_eq!(cfg.register_interval_secs, Some(120));
    assert_eq!(cfg.register_max_retry, Some(-1));
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
        "SECP256K1_PRIVHEX",
        "REG_SECRET",
        "HEARTBEAT_MIN_RATIO",
        "HEARTBEAT_MAX_RATIO",
    ]);

    std::env::set_var("SECP256K1_PRIVHEX", "abcdef");

    let err = NodeConfig::from_env().expect_err("should error");
    let msg = format!("{}", err);
    assert!(msg.contains("REG_SECRET required"));
}

#[test]
fn log_format_text_is_parsed() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "DMS_BASE_URL",
        "REQUEST_TIMEOUT_SECS",
        "LOG_FORMAT",
        "DDS_BASE_URL",
        "SECP256K1_PRIVHEX",
        "REG_SECRET",
        "HEARTBEAT_MIN_RATIO",
        "HEARTBEAT_MAX_RATIO",
    ]);

    std::env::set_var("DMS_BASE_URL", "https://dms.example");
    std::env::set_var("REQUEST_TIMEOUT_SECS", "10");
    std::env::set_var("LOG_FORMAT", "text");
    std::env::set_var("DDS_BASE_URL", "https://dds.example");
    std::env::set_var("REG_SECRET", "secret");
    std::env::set_var("SECP256K1_PRIVHEX", "abcdef");

    let cfg = NodeConfig::from_env().expect("config");
    assert_eq!(cfg.log_format, LogFormat::Text);
}

#[test]
fn loads_robot_defaults_without_siwe_fields_and_redacts_credentials() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "DMS_BASE_URL",
        "REQUEST_TIMEOUT_SECS",
        "NODE_VERSION",
        "HEARTBEAT_JITTER_MS",
        "HEARTBEAT_MIN_RATIO",
        "HEARTBEAT_MAX_RATIO",
        "POLL_BACKOFF_MS_MIN",
        "POLL_BACKOFF_MS_MAX",
        "TOKEN_SAFETY_RATIO",
        "TOKEN_REAUTH_MAX_RETRIES",
        "TOKEN_REAUTH_JITTER_MS",
        "AUKI_P2P_ENABLED",
        "AUKI_P2P_LISTEN_MULTIADDRS",
        "AUKI_P2P_ADVERTISED_MULTIADDRS",
        "MAX_CONCURRENCY",
        "LOG_FORMAT",
        "ENABLE_NOOP",
        "NOOP_SLEEP_SECS",
        "DDS_BASE_URL",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
        "REG_SECRET",
        "SECP256K1_PRIVHEX",
    ]);

    let credentials = "opaque-robot-id-and-secret";
    std::env::set_var("ROBOT_REGISTRATION_CREDENTIALS", credentials);

    let cfg = RobotNodeConfig::from_env().expect("robot config");
    assert_eq!(cfg.dms_base_url.as_str(), "https://dms.auki.network/v1");
    assert_eq!(cfg.dds_base_url.as_str(), "https://dds.auki.network/");
    assert_eq!(cfg.node_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(cfg.request_timeout_secs, 60);
    assert_eq!(cfg.heartbeat_jitter_ms, 250);
    assert!((cfg.heartbeat_min_ratio - 0.25).abs() < f64::EPSILON);
    assert!((cfg.heartbeat_max_ratio - 0.35).abs() < f64::EPSILON);
    assert_eq!(cfg.poll_backoff_ms_min, 1000);
    assert_eq!(cfg.poll_backoff_ms_max, 30000);
    assert!((cfg.token_safety_ratio - 0.75).abs() < f32::EPSILON);
    assert_eq!(cfg.token_reauth_max_retries, 3);
    assert_eq!(cfg.token_reauth_jitter_ms, 500);
    assert!(!cfg.auki_p2p_enabled);
    assert!(cfg.auki_p2p_listen_multiaddrs.is_empty());
    assert!(cfg.auki_p2p_advertised_multiaddrs.is_empty());
    assert_eq!(cfg.max_concurrency, 1);
    assert_eq!(cfg.log_format, LogFormat::Json);
    assert!(!cfg.enable_noop);
    assert_eq!(cfg.noop_sleep_secs, 5);

    let debug = format!("{cfg:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains(credentials));
}

#[test]
fn loads_explicit_p2p_multiaddrs() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "AUKI_P2P_ENABLED",
        "AUKI_P2P_LISTEN_MULTIADDRS",
        "AUKI_P2P_ADVERTISED_MULTIADDRS",
        "REG_SECRET",
        "SECP256K1_PRIVHEX",
    ]);
    std::env::set_var("REG_SECRET", "secret");
    std::env::set_var("SECP256K1_PRIVHEX", "abcdef");
    std::env::set_var("AUKI_P2P_ENABLED", "true");
    std::env::set_var(
        "AUKI_P2P_LISTEN_MULTIADDRS",
        " /ip4/127.0.0.1/tcp/0 , /ip4/0.0.0.0/tcp/41001 ",
    );
    std::env::set_var(
        "AUKI_P2P_ADVERTISED_MULTIADDRS",
        "/ip4/192.0.2.10/tcp/41001",
    );

    let cfg = NodeConfig::from_env().expect("P2P config");
    assert!(cfg.auki_p2p_enabled);
    assert_eq!(
        cfg.auki_p2p_listen_multiaddrs,
        ["/ip4/127.0.0.1/tcp/0", "/ip4/0.0.0.0/tcp/41001"]
    );
    assert_eq!(
        cfg.auki_p2p_advertised_multiaddrs,
        ["/ip4/192.0.2.10/tcp/41001"]
    );

    clear(&[
        "AUKI_P2P_ENABLED",
        "AUKI_P2P_LISTEN_MULTIADDRS",
        "AUKI_P2P_ADVERTISED_MULTIADDRS",
        "REG_SECRET",
        "SECP256K1_PRIVHEX",
    ]);
}

#[test]
fn constructs_robot_config_without_process_environment() {
    let credentials = "programmatic-opaque-credentials";
    let cfg = RobotNodeConfig::new(
        "https://dds.example/".parse().unwrap(),
        "https://dms.example/v1".parse().unwrap(),
        credentials,
    )
    .expect("programmatic robot config");

    assert_eq!(cfg.dds_base_url.as_str(), "https://dds.example/");
    assert_eq!(cfg.dms_base_url.as_str(), "https://dms.example/v1");
    assert_eq!(cfg.node_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(cfg.request_timeout_secs, 60);
    assert!(!format!("{cfg:?}").contains(credentials));
}

#[test]
fn robot_credentials_are_required_and_must_not_be_blank() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "DMS_BASE_URL",
        "REQUEST_TIMEOUT_SECS",
        "DDS_BASE_URL",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
        "HEARTBEAT_MIN_RATIO",
        "HEARTBEAT_MAX_RATIO",
        "LOG_FORMAT",
    ]);

    let err = RobotNodeConfig::from_env().expect_err("missing credentials must fail");
    assert!(err.to_string().contains("ROBOT_REGISTRATION_CREDENTIALS"));

    std::env::set_var("ROBOT_REGISTRATION_CREDENTIALS", "   ");
    let err = RobotNodeConfig::from_env().expect_err("blank credentials must fail");
    assert!(err.to_string().contains("ROBOT_REGISTRATION_CREDENTIALS"));
}

#[test]
fn loads_robot_credentials_from_file_and_trims_newline() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "DMS_BASE_URL",
        "REQUEST_TIMEOUT_SECS",
        "DDS_BASE_URL",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
        "HEARTBEAT_MIN_RATIO",
        "HEARTBEAT_MAX_RATIO",
        "LOG_FORMAT",
    ]);

    let credentials = "opaque-file-robot-credentials";
    let file = NamedTempFile::new().expect("credential file");
    std::fs::write(file.path(), format!("{credentials}\n")).expect("write credential file");
    std::env::set_var("ROBOT_REGISTRATION_CREDENTIALS_FILE", file.path());

    let cfg = RobotNodeConfig::from_env().expect("robot file config");
    let expected = RobotNodeConfig::new(
        "https://dds.auki.network".parse().unwrap(),
        "https://dms.auki.network/v1".parse().unwrap(),
        credentials,
    )
    .expect("expected robot config");
    assert_eq!(cfg, expected);
    let debug = format!("{cfg:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains(credentials));
}

#[test]
fn robot_credential_sources_are_mutually_exclusive() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "DMS_BASE_URL",
        "REQUEST_TIMEOUT_SECS",
        "DDS_BASE_URL",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
        "HEARTBEAT_MIN_RATIO",
        "HEARTBEAT_MAX_RATIO",
        "LOG_FORMAT",
    ]);

    std::env::set_var("ROBOT_REGISTRATION_CREDENTIALS", "inline-credentials");
    std::env::set_var("ROBOT_REGISTRATION_CREDENTIALS_FILE", "/not/read");

    let err = RobotNodeConfig::from_env().expect_err("ambiguous credentials must fail");
    assert!(err.to_string().contains("mutually exclusive"));
}

#[test]
fn robot_credential_file_must_be_readable_and_nonempty() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "DMS_BASE_URL",
        "REQUEST_TIMEOUT_SECS",
        "DDS_BASE_URL",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
        "HEARTBEAT_MIN_RATIO",
        "HEARTBEAT_MAX_RATIO",
        "LOG_FORMAT",
    ]);

    std::env::set_var(
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
        "/definitely/missing/robot-credentials",
    );
    let err = RobotNodeConfig::from_env().expect_err("missing file must fail");
    assert!(err
        .to_string()
        .contains("read robot registration credentials file"));

    let file = NamedTempFile::new().expect("empty credential file");
    std::env::set_var("ROBOT_REGISTRATION_CREDENTIALS_FILE", file.path());
    let err = RobotNodeConfig::from_env().expect_err("empty file must fail");
    assert!(err.to_string().contains("file must not be empty"));
}

#[test]
fn robot_credentials_do_not_switch_the_existing_siwe_loader() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "DMS_BASE_URL",
        "REQUEST_TIMEOUT_SECS",
        "DDS_BASE_URL",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
        "REG_SECRET",
        "SECP256K1_PRIVHEX",
        "HEARTBEAT_MIN_RATIO",
        "HEARTBEAT_MAX_RATIO",
        "LOG_FORMAT",
    ]);

    std::env::set_var("ROBOT_REGISTRATION_CREDENTIALS", "opaque-robot-credentials");
    std::env::set_var("SECP256K1_PRIVHEX", "abcdef");

    let err = NodeConfig::from_env().expect_err("SIWE must remain the default loader");
    assert!(err.to_string().contains("REG_SECRET required"));
}
