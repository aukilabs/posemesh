use crate::crypto::{
    format_timestamp_nanos, load_secp256k1_privhex, secp256k1_pubkey_uncompressed_hex,
    sign_recoverable_keccak_hex,
};
use crate::state::{
    read_state, set_status, touch_healthcheck_now, LockGuard, RegistrationState,
    STATUS_DISCONNECTED, STATUS_REGISTERED, STATUS_REGISTERING,
};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use rand::Rng;
use reqwest::Client;
use secp256k1::SecretKey;
use serde::Serialize;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

#[derive(Debug, Serialize)]
pub struct NodeRegistrationRequest {
    pub url: String,
    pub version: String,
    pub registration_credentials: String,
    pub signature: String,
    pub timestamp: String,
    pub public_key: String,
    pub capabilities: Vec<String>,
}

fn registration_endpoint(dds_base_url: &str) -> String {
    let base = dds_base_url.trim_end_matches('/');
    format!("{}/internal/v1/nodes/register", base)
}

fn build_registration_request(
    node_url: &str,
    node_version: &str,
    reg_secret: &str,
    sk: &SecretKey,
    capabilities: &[String],
) -> NodeRegistrationRequest {
    let ts = format_timestamp_nanos(Utc::now());
    let msg = format!("{}{}", node_url, ts);
    let signature = sign_recoverable_keccak_hex(sk, msg.as_bytes());
    let public_key = secp256k1_pubkey_uncompressed_hex(sk);
    let registration_credentials = reg_secret.to_owned();
    NodeRegistrationRequest {
        url: node_url.to_owned(),
        version: node_version.to_owned(),
        registration_credentials,
        signature,
        timestamp: ts,
        public_key,
        capabilities: capabilities.to_vec(),
    }
}

pub async fn register_once(
    dds_base_url: &str,
    node_url: &str,
    node_version: &str,
    reg_secret: &str,
    sk: &SecretKey,
    client: &Client,
    capabilities: &[String],
) -> Result<()> {
    let req = build_registration_request(node_url, node_version, reg_secret, sk, capabilities);
    let endpoint = registration_endpoint(dds_base_url);

    let pk_short = req.public_key.get(0..16).unwrap_or(&req.public_key);
    info!(
        url = req.url,
        version = req.version,
        public_key_prefix = pk_short,
        capabilities = ?req.capabilities,
        "Registering node with DDS"
    );

    let res = client
        .post(&endpoint)
        .json(&req)
        .send()
        .await
        .with_context(|| format!("POST {} failed", endpoint))?;

    if res.status().is_success() {
        debug!(status = ?res.status(), "Registration ok");
        Ok(())
    } else {
        let status = res.status();
        let body_snippet = match res.text().await {
            Ok(mut text) => {
                if text.len() > 512 {
                    text.truncate(512);
                }
                text.replace('\n', " ")
            }
            Err(_) => "<unavailable>".to_string(),
        };
        Err(anyhow!(
            "registration failed: status {}, endpoint {}, body_snippet: {}",
            status,
            endpoint,
            body_snippet
        ))
    }
}

#[derive(Debug)]
pub struct RegistrationConfig {
    pub dds_base_url: String,
    pub node_url: String,
    pub node_version: String,
    pub reg_secret: String,
    pub secp256k1_privhex: String,
    pub client: Client,
    pub register_interval_secs: u64,
    pub max_retry: i32,
    pub capabilities: Vec<String>,
}

pub async fn run_registration_loop(cfg: RegistrationConfig) {
    let RegistrationConfig {
        dds_base_url,
        node_url,
        node_version,
        reg_secret,
        secp256k1_privhex,
        client,
        register_interval_secs,
        max_retry,
        capabilities,
    } = cfg;
    let sk = match load_secp256k1_privhex(&secp256k1_privhex) {
        Ok(k) => k,
        Err(e) => {
            warn!("Invalid secp256k1 private key (redacted): {}", e);
            return;
        }
    };

    let healthcheck_ttl = Duration::from_secs(register_interval_secs.max(1));
    let lock_stale_after = {
        let base = healthcheck_ttl.saturating_mul(2);
        let min = Duration::from_secs(30);
        let max = Duration::from_secs(600);
        if base < min {
            min
        } else if base > max {
            max
        } else {
            base
        }
    };

    fn timer_interval_secs(attempt: i32) -> u64 {
        if attempt <= 0 {
            return 0;
        }
        let p = 2_i64.saturating_pow(attempt as u32);
        p.clamp(0, 60) as u64
    }

    let _ = set_status(read_state().map(|s| s.status).unwrap_or_default().as_str());

    let mut attempt: i32 = 0;
    let mut next_sleep = Duration::from_secs(1);

    info!(
        event = "registration.loop.start",
        healthcheck_ttl_sec = healthcheck_ttl.as_secs() as i64,
        node_url = %node_url,
        node_version = %node_version,
        "registration loop started"
    );

    loop {
        tokio::time::sleep(next_sleep).await;
        let RegistrationState {
            status,
            last_healthcheck,
        } = read_state().unwrap_or_default();

        match status.as_str() {
            STATUS_DISCONNECTED | STATUS_REGISTERING => {
                let lock_guard = match LockGuard::try_acquire(lock_stale_after) {
                    Ok(Some(g)) => {
                        info!(event = "lock.acquired", "registration lock acquired");
                        Some(g)
                    }
                    Ok(None) => {
                        debug!(event = "lock.busy", "another registrar is active");
                        next_sleep = healthcheck_ttl;
                        continue;
                    }
                    Err(e) => {
                        warn!(event = "lock.error", error = %e, "could not acquire lock");
                        next_sleep = healthcheck_ttl;
                        continue;
                    }
                };

                if status.as_str() == STATUS_DISCONNECTED {
                    if let Err(e) = set_status(STATUS_REGISTERING) {
                        warn!(event = "status.transition.error", error = %e);
                    } else {
                        info!(
                            event = "status.transition",
                            from = STATUS_DISCONNECTED,
                            to = STATUS_REGISTERING,
                            "moved to registering"
                        );
                    }
                }

                attempt += 1;

                let start = Instant::now();
                let res = register_once(
                    &dds_base_url,
                    &node_url,
                    &node_version,
                    &reg_secret,
                    &sk,
                    &client,
                    &capabilities,
                )
                .await;
                let elapsed_ms = start.elapsed().as_millis();

                match res {
                    Ok(()) => {
                        let _ = set_status(STATUS_REGISTERED);
                        let _ = touch_healthcheck_now();
                        info!(
                            event = "registration.success",
                            elapsed_ms = elapsed_ms as i64,
                            "successfully registered to DDS"
                        );
                        attempt = 0;
                        next_sleep = healthcheck_ttl;
                        drop(lock_guard);
                    }
                    Err(e) => {
                        warn!(
                            event = "registration.error",
                            elapsed_ms = elapsed_ms as i64,
                            error = %e,
                            error_debug = ?e,
                            attempt = attempt,
                            "registration to DDS failed; will back off"
                        );
                        if max_retry >= 0 && attempt >= max_retry {
                            warn!(
                                event = "registration.max_retry_reached",
                                max_retry = max_retry,
                                "max retry reached; pausing until next TTL window"
                            );
                            attempt = 0;
                            next_sleep = healthcheck_ttl;
                            drop(lock_guard);
                            continue;
                        }
                        let base = Duration::from_secs(timer_interval_secs(attempt));
                        let jitter_factor: f64 = rand::thread_rng().gen_range(0.8..=1.2);
                        next_sleep =
                            Duration::from_secs_f64(base.as_secs_f64() * jitter_factor.max(0.1));
                        drop(lock_guard);
                    }
                }
            }
            STATUS_REGISTERED => {
                let elapsed = last_healthcheck
                    .map(|t| Utc::now() - t)
                    .map(|d| d.to_std().unwrap_or_default())
                    .unwrap_or_else(|| Duration::from_secs(u64::MAX / 2));

                if elapsed > healthcheck_ttl {
                    info!(
                        event = "healthcheck.expired",
                        elapsed_since_healthcheck_sec = elapsed.as_secs() as i64,
                        "healthcheck TTL exceeded; re-entering registering"
                    );
                    let _ = set_status(STATUS_REGISTERING);
                    next_sleep = Duration::from_secs(1);
                } else {
                    next_sleep = healthcheck_ttl;
                }
            }
            other => {
                warn!(
                    event = "status.unknown",
                    status = other,
                    "unknown status; resetting to disconnected"
                );
                let _ = set_status(STATUS_DISCONNECTED);
                next_sleep = Duration::from_secs(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::load_secp256k1_privhex;
    use parking_lot::Mutex as PLMutex;
    use std::io;
    use std::sync::Arc;
    use tracing::subscriber;
    use tracing_subscriber::layer::SubscriberExt;

    struct BufWriter(Arc<PLMutex<Vec<u8>>>);
    impl io::Write for BufWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    struct MakeBufWriter(Arc<PLMutex<Vec<u8>>>);
    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for MakeBufWriter {
        type Writer = BufWriter;
        fn make_writer(&'a self) -> Self::Writer {
            BufWriter(self.0.clone())
        }
    }

    #[tokio::test]
    async fn logs_do_not_include_secret() {
        let buf = Arc::new(PLMutex::new(Vec::<u8>::new()));
        let make = MakeBufWriter(buf.clone());
        let layer = tracing_subscriber::fmt::layer()
            .with_writer(make)
            .with_ansi(false)
            .without_time();
        let subscriber = tracing_subscriber::registry().with(layer);
        let _guard = subscriber::set_default(subscriber);

        let secret = "my-super-secret";
        let dds = "http://127.0.0.1:9";
        let url = "https://node.example.com";
        let version = "1.2.3";
        let sk = load_secp256k1_privhex(
            "e331b6d69882b4ed5bb7f55b585d7d0f7dc3aeca4a3deee8d16bde3eca51aace",
        )
        .unwrap();
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_millis(200))
            .build()
            .unwrap();
        let capabilities = vec![
            "/reconstruction/global-refinement/v1".to_string(),
            "/reconstruction/local-refinement/v1".to_string(),
        ];

        let _ = register_once(dds, url, version, secret, &sk, &client, &capabilities).await;

        let captured = String::from_utf8(buf.lock().clone()).unwrap_or_default();
        assert!(captured.contains("Registering node with DDS"));
        assert!(
            !captured.contains(secret),
            "logs leaked secret: {}",
            captured
        );
    }
}
