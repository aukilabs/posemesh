use auki_p2p::Identity;
use auki_sdk::{AukiRelayConfig, AukiRelayMode};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use once_cell::sync::Lazy;
use posemesh_compute_node::config::{LogFormat, NodeConfig, P2pPrivateKey, RobotNodeConfig};
use std::{sync::Mutex, time::Duration};
use tempfile::NamedTempFile;

static ENV_GUARD: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

const RELAY_ENV_KEYS: &[&str] = &[
    "AUKI_P2P_RELAY_MODE",
    "AUKI_P2P_RELAY_BOOKING_MODE",
    "AUKI_P2P_RELAY_BOOKING_DURATION_SECONDS",
    "AUKI_P2P_RELAY_COUNT",
    "AUKI_P2P_RELAY_STATUS_POLL_INTERVAL_SECONDS",
];

const P2P_IDENTITY_ENV_KEYS: &[&str] = &["AUKI_P2P_PRIVATE_KEY", "AUKI_P2P_PRIVATE_KEY_FILE"];

fn clear(keys: &[&str]) {
    for k in keys
        .iter()
        .chain(RELAY_ENV_KEYS)
        .chain(P2P_IDENTITY_ENV_KEYS)
    {
        std::env::remove_var(k);
    }
}

fn install_test_p2p_identity() -> Identity {
    let identity = Identity::from_ed25519_seed(&[0x52; 32]);
    let protobuf = identity.to_protobuf_encoding().unwrap();
    std::env::set_var("AUKI_P2P_PRIVATE_KEY", STANDARD.encode(protobuf));
    identity
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
    assert!(cfg.p2p_peer_id().is_none());
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
fn enabled_p2p_requires_a_persisted_identity_for_each_production_loader() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "AUKI_P2P_ENABLED",
        "REG_SECRET",
        "SECP256K1_PRIVHEX",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);

    std::env::set_var("REG_SECRET", "secret");
    std::env::set_var("SECP256K1_PRIVHEX", "abcdef");
    std::env::set_var("AUKI_P2P_ENABLED", "true");
    let error = NodeConfig::from_env().expect_err("SIWE P2P without an identity must fail");
    assert!(error.to_string().contains("P2P_PRIVATE_KEY"));

    clear(&[
        "AUKI_P2P_ENABLED",
        "REG_SECRET",
        "SECP256K1_PRIVHEX",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
    std::env::set_var("ROBOT_REGISTRATION_CREDENTIALS", "robot-credentials");
    std::env::set_var("AUKI_P2P_ENABLED", "true");
    let error =
        RobotNodeConfig::from_env().expect_err("enabled Robot P2P without an identity must fail");
    assert!(error.to_string().contains("P2P_PRIVATE_KEY"));

    clear(&[
        "AUKI_P2P_ENABLED",
        "REG_SECRET",
        "SECP256K1_PRIVHEX",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
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
    assert!(cfg.p2p_peer_id().is_none());
    assert!(cfg.relay_config().is_none());
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
    let identity = install_test_p2p_identity();
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
    assert_eq!(cfg.p2p_peer_id(), Some(identity.peer_id()));
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
fn persisted_p2p_identity_loads_from_inline_or_file_and_is_redacted() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
    std::env::set_var("ROBOT_REGISTRATION_CREDENTIALS", "robot-credentials");

    let identity = Identity::generate();
    let protobuf = identity.to_protobuf_encoding().unwrap();
    let encoded = STANDARD.encode(&protobuf);
    std::env::set_var("AUKI_P2P_PRIVATE_KEY", &encoded);
    let inline = RobotNodeConfig::from_env().expect("inline P2P identity");
    assert_eq!(inline.p2p_peer_id(), Some(identity.peer_id()));
    let debug = format!("{inline:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains(&encoded));

    std::env::remove_var("AUKI_P2P_PRIVATE_KEY");
    let mut file = NamedTempFile::new().expect("P2P private-key file");
    std::io::Write::write_all(&mut file, &protobuf).expect("write P2P private key");
    std::env::set_var("AUKI_P2P_PRIVATE_KEY_FILE", file.path());
    let from_file = RobotNodeConfig::from_env().expect("file P2P identity");
    assert_eq!(from_file.p2p_peer_id(), Some(identity.peer_id()));

    clear(&[
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
}

#[test]
fn persisted_p2p_identity_sources_and_encoding_fail_closed() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
    std::env::set_var("ROBOT_REGISTRATION_CREDENTIALS", "robot-credentials");
    std::env::set_var("AUKI_P2P_PRIVATE_KEY", "not-base64");
    let error = RobotNodeConfig::from_env().expect_err("invalid Base64 must fail");
    assert!(error.to_string().contains("canonical Base64"));

    std::env::set_var("AUKI_P2P_PRIVATE_KEY", " Zm9v ");
    let error = RobotNodeConfig::from_env().expect_err("padded whitespace must fail");
    assert!(error.to_string().contains("canonical Base64"));

    let identity = Identity::generate();
    let protobuf = identity.to_protobuf_encoding().unwrap();
    let encoded = STANDARD.encode(&protobuf);
    std::env::set_var("AUKI_P2P_PRIVATE_KEY", &encoded);
    std::env::set_var("AUKI_P2P_PRIVATE_KEY_FILE", "/not/read");
    let error = RobotNodeConfig::from_env().expect_err("ambiguous sources must fail");
    assert!(error.to_string().contains("mutually exclusive"));

    std::env::remove_var("AUKI_P2P_PRIVATE_KEY");
    let file = NamedTempFile::new().expect("invalid P2P key file");
    std::fs::write(file.path(), b"not-a-libp2p-key").expect("write invalid key");
    std::env::set_var("AUKI_P2P_PRIVATE_KEY_FILE", file.path());
    let error = RobotNodeConfig::from_env().expect_err("invalid protobuf must fail");
    assert!(error.to_string().contains("canonical Ed25519"));

    clear(&[
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
}

#[cfg(unix)]
#[test]
fn p2p_private_key_file_rejects_group_or_other_access() {
    use std::os::unix::fs::PermissionsExt;

    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
    std::env::set_var("ROBOT_REGISTRATION_CREDENTIALS", "robot-credentials");
    let identity = Identity::generate();
    let file = NamedTempFile::new().expect("P2P key file");
    std::fs::write(file.path(), identity.to_protobuf_encoding().unwrap()).unwrap();
    std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o644)).unwrap();
    std::env::set_var("AUKI_P2P_PRIVATE_KEY_FILE", file.path());

    let error = RobotNodeConfig::from_env().expect_err("permissive key file must fail");
    assert!(error.to_string().contains("group or other"));

    clear(&[
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
}

#[test]
fn programmatic_private_key_preserves_peer_id_without_exposing_bytes() {
    let identity = Identity::generate();
    let protobuf = identity.to_protobuf_encoding().unwrap();
    let key = P2pPrivateKey::from_protobuf_encoding(protobuf.clone()).unwrap();
    let mut cfg = RobotNodeConfig::new(
        "https://dds.example".parse().unwrap(),
        "https://dms.example/v1".parse().unwrap(),
        "robot-credentials",
    )
    .unwrap();
    cfg.set_p2p_private_key(Some(key));

    assert_eq!(cfg.p2p_peer_id(), Some(identity.peer_id()));
    assert!(!format!("{cfg:?}").contains(&STANDARD.encode(protobuf)));
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

#[test]
fn loads_explicit_robot_relay_booking_policy() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "AUKI_P2P_ENABLED",
        "AUKI_P2P_LISTEN_MULTIADDRS",
        "AUKI_P2P_ADVERTISED_MULTIADDRS",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
    std::env::set_var("ROBOT_REGISTRATION_CREDENTIALS", "robot-credentials");
    std::env::set_var("AUKI_P2P_ENABLED", "true");
    std::env::set_var("AUKI_P2P_RELAY_MODE", "always");
    std::env::set_var("AUKI_P2P_RELAY_BOOKING_MODE", "dedicated");
    std::env::set_var("AUKI_P2P_RELAY_BOOKING_DURATION_SECONDS", "300");
    std::env::set_var("AUKI_P2P_RELAY_COUNT", "3");
    std::env::set_var("AUKI_P2P_RELAY_STATUS_POLL_INTERVAL_SECONDS", "60");
    install_test_p2p_identity();

    let cfg = RobotNodeConfig::from_env().expect("explicit relay config");
    let relay = cfg.relay_config().expect("relay enabled");
    assert_eq!(relay.mode, AukiRelayMode::Dedicated);
    assert_eq!(relay.requested_duration, Duration::from_secs(300));
    assert_eq!(relay.relay_count, 3);
    assert_eq!(relay.status_poll_interval, Duration::from_secs(60));

    clear(&[
        "AUKI_P2P_ENABLED",
        "AUKI_P2P_LISTEN_MULTIADDRS",
        "AUKI_P2P_ADVERTISED_MULTIADDRS",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
}

#[test]
fn enabled_robot_p2p_defaults_to_one_public_relay() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "AUKI_P2P_ENABLED",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
    std::env::set_var("ROBOT_REGISTRATION_CREDENTIALS", "robot-credentials");
    std::env::set_var("AUKI_P2P_ENABLED", "true");
    install_test_p2p_identity();

    let cfg = RobotNodeConfig::from_env().expect("default relay config");
    assert_eq!(cfg.relay_config(), Some(AukiRelayConfig::default()));
    assert!(cfg.auki_p2p_enabled);

    clear(&[
        "AUKI_P2P_ENABLED",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
}

#[test]
fn relay_mode_does_not_implicitly_enable_robot_p2p() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "AUKI_P2P_ENABLED",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
    std::env::set_var("ROBOT_REGISTRATION_CREDENTIALS", "robot-credentials");

    for mode in ["auto", "always"] {
        std::env::set_var("AUKI_P2P_RELAY_MODE", mode);
        let error = RobotNodeConfig::from_env()
            .expect_err("relay mode without the explicit P2P gate must fail");
        assert!(error.to_string().contains("AUKI_P2P_ENABLED=true"));
    }

    clear(&[
        "AUKI_P2P_ENABLED",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
}

#[test]
fn explicit_disabled_relay_mode_keeps_enabled_robot_p2p_direct_only() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "AUKI_P2P_ENABLED",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
    std::env::set_var("ROBOT_REGISTRATION_CREDENTIALS", "robot-credentials");
    std::env::set_var("AUKI_P2P_ENABLED", "true");
    std::env::set_var("AUKI_P2P_RELAY_MODE", "disabled");
    install_test_p2p_identity();

    let cfg = RobotNodeConfig::from_env().expect("explicit direct-only config");
    assert!(cfg.relay_config().is_none());
    assert!(cfg.auki_p2p_enabled);

    clear(&[
        "AUKI_P2P_ENABLED",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
}

#[test]
fn rejects_invalid_robot_relay_booking_environment_values() {
    let _g = ENV_GUARD.lock().unwrap();
    clear(&[
        "AUKI_P2P_ENABLED",
        "AUKI_P2P_ADVERTISED_MULTIADDRS",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
    std::env::set_var("ROBOT_REGISTRATION_CREDENTIALS", "robot-credentials");
    std::env::set_var("AUKI_P2P_ENABLED", "true");

    for (key, value, expected) in [
        ("AUKI_P2P_RELAY_MODE", "sometimes", "RELAY_MODE"),
        (
            "AUKI_P2P_RELAY_BOOKING_MODE",
            "private",
            "RELAY_BOOKING_MODE",
        ),
        (
            "AUKI_P2P_RELAY_BOOKING_DURATION_SECONDS",
            "299",
            "relay booking duration",
        ),
        (
            "AUKI_P2P_RELAY_BOOKING_DURATION_SECONDS",
            "86401",
            "relay booking duration",
        ),
        ("AUKI_P2P_RELAY_COUNT", "0", "relay count"),
        ("AUKI_P2P_RELAY_COUNT", "4", "relay count"),
        (
            "AUKI_P2P_RELAY_STATUS_POLL_INTERVAL_SECONDS",
            "0",
            "relay status poll interval",
        ),
        (
            "AUKI_P2P_RELAY_STATUS_POLL_INTERVAL_SECONDS",
            "61",
            "relay status poll interval",
        ),
    ] {
        clear(RELAY_ENV_KEYS);
        std::env::set_var(key, value);
        let error = RobotNodeConfig::from_env().expect_err("invalid relay value must fail");
        assert!(
            error.to_string().contains(expected),
            "unexpected error for {key}={value}: {error}"
        );
    }

    clear(&[
        "AUKI_P2P_ENABLED",
        "AUKI_P2P_ADVERTISED_MULTIADDRS",
        "ROBOT_REGISTRATION_CREDENTIALS",
        "ROBOT_REGISTRATION_CREDENTIALS_FILE",
    ]);
}

#[test]
fn programmatic_relay_config_enforces_p2p_gate() {
    let mut cfg = RobotNodeConfig::new(
        "https://dds.example".parse().unwrap(),
        "https://dms.example/v1".parse().unwrap(),
        "robot-credentials",
    )
    .unwrap();
    let relay = AukiRelayConfig::new(
        AukiRelayMode::Public,
        2,
        Duration::from_secs(86_400),
        Duration::from_secs(5),
    )
    .unwrap();
    let error = cfg
        .set_relay_config(Some(relay))
        .expect_err("programmatic relay mode must not implicitly enable P2P");
    assert!(error.to_string().contains("AUKI_P2P_ENABLED=true"));

    cfg.auki_p2p_enabled = true;
    cfg.set_relay_config(Some(relay)).unwrap();
    assert_eq!(cfg.relay_config(), Some(relay));
}
