use crate::crypto::load_secp256k1_privhex;
use crate::state::{
    read_state, set_status, LockGuard, RegistrationState, STATUS_DISCONNECTED, STATUS_REGISTERED,
    STATUS_REGISTERING,
};
use rand::Rng;
use reqwest::Client;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

use auki_auth::machine::registration::RegistrationAttemptKind;
pub use auki_auth::machine::registration::{
    register_once, NodeRegisterWalletRequest, RegistrationAttempt,
};

const PARKED_POLL_INTERVAL: Duration = Duration::from_secs(1);
const RATE_LIMITED_WARN_INTERVAL: Duration = Duration::from_secs(180);

#[derive(Debug)]
pub struct RegistrationConfig {
    pub dds_base_url: String,
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

    let register_interval = Duration::from_secs(register_interval_secs.max(1));
    let lock_stale_after = {
        let base = register_interval.saturating_mul(2);
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

    let mut transient_attempt: i32 = 0;
    let mut next_sleep = Duration::ZERO;
    let mut conflict_episode_started_at: Option<Instant> = None;
    let mut next_conflict_warn_at: Option<Instant> = None;
    let mut last_slow_warn_at: Option<Instant> = None;

    info!(
        event = "registration.loop.start",
        register_interval_sec = register_interval.as_secs() as i64,
        node_version = %node_version,
        "registration loop started"
    );

    loop {
        tokio::time::sleep(next_sleep).await;
        let RegistrationState { status, .. } = read_state().unwrap_or_default();

        match status.as_str() {
            STATUS_REGISTERED => {
                next_sleep = PARKED_POLL_INTERVAL;
                continue;
            }
            STATUS_DISCONNECTED | STATUS_REGISTERING => {
                let lock_guard = match LockGuard::try_acquire(lock_stale_after) {
                    Ok(Some(g)) => {
                        info!(event = "lock.acquired", "registration lock acquired");
                        Some(g)
                    }
                    Ok(None) => {
                        debug!(event = "lock.busy", "another registrar is active");
                        next_sleep = register_interval;
                        continue;
                    }
                    Err(e) => {
                        warn!(event = "lock.error", error = %e, "could not acquire lock");
                        next_sleep = register_interval;
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
                let start = Instant::now();
                let attempt = register_once(
                    &dds_base_url,
                    &node_version,
                    &reg_secret,
                    &sk,
                    &client,
                    &capabilities,
                )
                .await;
                let elapsed_ms = start.elapsed().as_millis();

                match attempt.kind() {
                    RegistrationAttemptKind::Registered => {
                        let _ = set_status(STATUS_REGISTERED);
                        info!(
                            event = "registration.success",
                            elapsed_ms = elapsed_ms as i64,
                            "successfully registered to DDS"
                        );
                        transient_attempt = 0;
                        conflict_episode_started_at = None;
                        next_conflict_warn_at = None;
                        last_slow_warn_at = None;
                        next_sleep = PARKED_POLL_INTERVAL;
                        drop(lock_guard);
                    }
                    RegistrationAttemptKind::Conflict => {
                        transient_attempt = 0;
                        last_slow_warn_at = None;
                        let now = Instant::now();
                        let should_warn = match next_conflict_warn_at {
                            Some(deadline) => now >= deadline,
                            None => true,
                        };
                        let blocked_ms = if let Some(started_at) = conflict_episode_started_at {
                            now.duration_since(started_at).as_millis() as i64
                        } else {
                            0
                        };
                        if conflict_episode_started_at.is_none() {
                            conflict_episode_started_at = Some(now);
                        }
                        if should_warn {
                            next_conflict_warn_at = Some(now + RATE_LIMITED_WARN_INTERVAL);
                            warn!(
                                event = "registration.conflict",
                                elapsed_ms = elapsed_ms as i64,
                                blocked_ms,
                                error = attempt.error_text(),
                                "registration blocked by an existing online node; will retry after cooldown"
                            );
                        } else {
                            debug!(
                                event = "registration.conflict",
                                elapsed_ms = elapsed_ms as i64,
                                blocked_ms,
                                error = attempt.error_text(),
                                "registration still blocked by an existing online node"
                            );
                        }
                        next_sleep = register_interval;
                        drop(lock_guard);
                    }
                    RegistrationAttemptKind::RetryableFailure => {
                        conflict_episode_started_at = None;
                        next_conflict_warn_at = None;
                        transient_attempt += 1;
                        warn!(
                            event = "registration.error",
                            elapsed_ms = elapsed_ms as i64,
                            error = attempt.error_text(),
                            attempt = transient_attempt,
                            "registration to DDS failed; will back off"
                        );
                        if max_retry >= 0 && transient_attempt >= max_retry {
                            warn!(
                                event = "registration.max_retry_reached",
                                max_retry = max_retry,
                                "max retry reached; pausing until next TTL window"
                            );
                            transient_attempt = 0;
                            next_sleep = register_interval;
                            drop(lock_guard);
                            continue;
                        }
                        let base = Duration::from_secs(timer_interval_secs(transient_attempt));
                        let jitter_factor: f64 = rand::thread_rng().gen_range(0.8..=1.2);
                        next_sleep =
                            Duration::from_secs_f64(base.as_secs_f64() * jitter_factor.max(0.1));
                        drop(lock_guard);
                    }
                    RegistrationAttemptKind::SlowRetryFailure => {
                        transient_attempt = 0;
                        conflict_episode_started_at = None;
                        next_conflict_warn_at = None;
                        let now = Instant::now();
                        let should_warn = match last_slow_warn_at {
                            Some(deadline) => {
                                now.duration_since(deadline) >= RATE_LIMITED_WARN_INTERVAL
                            }
                            None => true,
                        };
                        if should_warn {
                            last_slow_warn_at = Some(now);
                            warn!(
                                event = "registration.error",
                                elapsed_ms = elapsed_ms as i64,
                                error = attempt.error_text(),
                                "registration to DDS failed; will retry after cooldown"
                            );
                        } else {
                            debug!(
                                event = "registration.error",
                                elapsed_ms = elapsed_ms as i64,
                                error = attempt.error_text(),
                                "registration to DDS still blocked by a non-retryable error"
                            );
                        }
                        next_sleep = register_interval;
                        drop(lock_guard);
                    }
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
    use crate::state::{clear_node_secret, write_state, RegistrationState};
    use axum::{http::StatusCode, routing::post, Router};
    use parking_lot::Mutex as PLMutex;
    use std::io;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::sync::OnceLock;
    use tokio::net::TcpListener;
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

    fn test_lock() -> &'static PLMutex<()> {
        static TEST_LOCK: OnceLock<PLMutex<()>> = OnceLock::new();
        TEST_LOCK.get_or_init(|| PLMutex::new(()))
    }

    fn reset_registration_state() {
        clear_node_secret().unwrap();
        write_state(&RegistrationState::default()).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn logs_do_not_include_secret() {
        let _guard = test_lock().lock();
        reset_registration_state();

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

        let _ = register_once(dds, version, secret, &sk, &client, &capabilities).await;

        let captured = String::from_utf8(buf.lock().clone()).unwrap_or_default();
        assert!(captured.contains("Registering node with DDS"));
        assert!(
            !captured.contains(secret),
            "logs leaked secret: {}",
            captured
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn register_once_classifies_conflict() {
        let _guard = test_lock().lock();
        reset_registration_state();

        async fn conflict_handler() -> StatusCode {
            StatusCode::CONFLICT
        }

        let app = Router::new()
            .route("/internal/v1/auth/siwe/request", post(conflict_handler))
            .route("/internal/v1/nodes/register-wallet", post(conflict_handler));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let sk = load_secp256k1_privhex(
            "e331b6d69882b4ed5bb7f55b585d7d0f7dc3aeca4a3deee8d16bde3eca51aace",
        )
        .unwrap();
        let attempt = register_once(
            &format!("http://{}", addr),
            "1.2.3",
            "secret",
            &sk,
            &client,
            &["/cap/example/v1".to_string()],
        )
        .await;

        assert_eq!(attempt.kind(), RegistrationAttemptKind::Conflict);
        server.abort();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn registration_loop_parks_after_success() {
        let _guard = test_lock().lock();
        reset_registration_state();

        let request_hits = Arc::new(AtomicUsize::new(0));
        let register_hits = Arc::new(AtomicUsize::new(0));

        async fn nonce_handler() -> axum::Json<serde_json::Value> {
            axum::Json(serde_json::json!({
                "nonce": "abc12345",
                "domain": "dds.example.com",
                "uri": "https://dds.example.com",
                "version": "1",
                "chainId": 8453,
                "issuedAt": "2026-01-01T00:00:00Z"
            }))
        }

        let request_hits_clone = Arc::clone(&request_hits);
        let register_hits_clone = Arc::clone(&register_hits);
        let app = Router::new()
            .route(
                "/internal/v1/auth/siwe/request",
                post(move || {
                    let request_hits = Arc::clone(&request_hits_clone);
                    async move {
                        request_hits.fetch_add(1, Ordering::SeqCst);
                        nonce_handler().await
                    }
                }),
            )
            .route(
                "/internal/v1/nodes/register-wallet",
                post(move || {
                    let register_hits = Arc::clone(&register_hits_clone);
                    async move {
                        register_hits.fetch_add(1, Ordering::SeqCst);
                        StatusCode::OK
                    }
                }),
            );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let cfg = RegistrationConfig {
            dds_base_url: format!("http://{}", addr),
            node_version: "1.2.3".to_string(),
            reg_secret: "secret".to_string(),
            secp256k1_privhex: "e331b6d69882b4ed5bb7f55b585d7d0f7dc3aeca4a3deee8d16bde3eca51aace"
                .to_string(),
            client,
            register_interval_secs: 1,
            max_retry: -1,
            capabilities: vec!["/cap/example/v1".to_string()],
        };

        let handle = tokio::spawn(async move {
            run_registration_loop(cfg).await;
        });

        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(read_state().unwrap().status, STATUS_REGISTERED);

        tokio::time::sleep(Duration::from_millis(1300)).await;
        assert_eq!(request_hits.load(Ordering::SeqCst), 1);
        assert_eq!(register_hits.load(Ordering::SeqCst), 1);

        handle.abort();
        server.abort();
    }
}
