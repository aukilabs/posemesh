use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rand::{rngs::StdRng, Rng, SeedableRng};
use tokio::{
    sync::{Mutex, Notify},
    task::JoinHandle,
    time::sleep,
};
use tracing::{info, warn};

use super::siwe::{AccessBundle, SiweError};

const DEFAULT_RATIO: f64 = 0.75;
const MIN_DURATION: Duration = Duration::from_millis(1);

#[async_trait]
pub trait AccessAuthenticator: Send + Sync {
    async fn login(&self) -> Result<AccessBundle, SiweError>;
}

pub trait Clock: Send + Sync {
    fn now_instant(&self) -> Instant;
    fn now_utc(&self) -> DateTime<Utc>;
}

#[derive(Clone)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_instant(&self) -> Instant {
        Instant::now()
    }

    fn now_utc(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Debug, Clone)]
pub struct TokenManagerConfig {
    pub safety_ratio: f64,
    pub max_retries: u32,
    pub jitter: Duration,
}

impl Default for TokenManagerConfig {
    fn default() -> Self {
        Self {
            safety_ratio: DEFAULT_RATIO,
            max_retries: 3,
            jitter: Duration::from_millis(500),
        }
    }
}

impl TokenManagerConfig {
    fn clamped_ratio(&self) -> f64 {
        self.safety_ratio.clamp(0.0, 1.0)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TokenManagerError {
    #[error("token manager stopped")]
    Stopped,
    #[error("token response already expired")]
    Expired,
    #[error("authentication failed after {attempts} attempts: {last_error}")]
    Authentication {
        attempts: u32,
        #[source]
        last_error: SiweError,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum TokenProviderError {
    #[error(transparent)]
    TokenManager(#[from] TokenManagerError),
    #[error("{0}")]
    Message(String),
}

pub type TokenProviderResult<T> = std::result::Result<T, TokenProviderError>;

#[async_trait]
pub trait TokenProvider: Send + Sync {
    async fn bearer(&self) -> TokenProviderResult<String>;
    async fn on_unauthorized(&self);
}

pub struct TokenManager<A: AccessAuthenticator, C: Clock> {
    auth: Arc<A>,
    clock: Arc<C>,
    config: TokenManagerConfig,
    rng: Arc<Mutex<StdRng>>,
    state: Arc<Mutex<State>>,
    stopped: Arc<AtomicBool>,
    stop_notify: Arc<Notify>,
    state_notify: Arc<Notify>,
    bg_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl<A: AccessAuthenticator, C: Clock> Clone for TokenManager<A, C> {
    fn clone(&self) -> Self {
        Self {
            auth: Arc::clone(&self.auth),
            clock: Arc::clone(&self.clock),
            config: self.config.clone(),
            rng: Arc::clone(&self.rng),
            state: Arc::clone(&self.state),
            stopped: Arc::clone(&self.stopped),
            stop_notify: Arc::clone(&self.stop_notify),
            state_notify: Arc::clone(&self.state_notify),
            bg_task: Arc::clone(&self.bg_task),
        }
    }
}

#[async_trait]
impl<A, C> TokenProvider for TokenManager<A, C>
where
    A: AccessAuthenticator + 'static,
    C: Clock + 'static,
{
    async fn bearer(&self) -> TokenProviderResult<String> {
        let now = self.clock.now_instant();
        self.get_access(now)
            .await
            .map_err(TokenProviderError::TokenManager)
    }

    async fn on_unauthorized(&self) {
        self.on_unauthorized_retry().await;
    }
}

struct State {
    token: Option<TokenEntry>,
    inflight: Option<Arc<Notify>>,
}

#[derive(Clone)]
struct TokenEntry {
    value: String,
    refresh_at: Instant,
}

impl TokenEntry {
    fn is_valid(&self, now: Instant) -> bool {
        now <= self.refresh_at
    }
}

impl<A, C> TokenManager<A, C>
where
    A: AccessAuthenticator + 'static,
    C: Clock + 'static,
{
    pub fn new(auth: Arc<A>, clock: Arc<C>, config: TokenManagerConfig) -> Self {
        let rng = StdRng::from_entropy();
        Self::with_rng(auth, clock, config, rng)
    }

    pub fn with_rng(auth: Arc<A>, clock: Arc<C>, config: TokenManagerConfig, rng: StdRng) -> Self {
        Self {
            auth,
            clock,
            config,
            rng: Arc::new(Mutex::new(rng)),
            state: Arc::new(Mutex::new(State {
                token: None,
                inflight: None,
            })),
            stopped: Arc::new(AtomicBool::new(false)),
            stop_notify: Arc::new(Notify::new()),
            state_notify: Arc::new(Notify::new()),
            bg_task: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn get_access(&self, now: Instant) -> Result<String, TokenManagerError> {
        loop {
            let (notify, is_owner) = {
                let mut state = self.state.lock().await;
                if self.stopped.load(Ordering::SeqCst) {
                    return Err(TokenManagerError::Stopped);
                }
                if let Some(entry) = &state.token {
                    if entry.is_valid(now) {
                        return Ok(entry.value.clone());
                    }
                }
                match state.inflight.clone() {
                    Some(existing) => (existing, false),
                    None => {
                        let notify = Arc::new(Notify::new());
                        state.inflight = Some(notify.clone());
                        (notify, true)
                    }
                }
            };

            if is_owner {
                let result = self.reauth(now).await;
                let mut refreshed = false;
                let (notify_to_signal, outcome) = {
                    let mut state = self.state.lock().await;
                    let notify_opt = state.inflight.take();
                    let outcome = match result {
                        Ok(entry) => {
                            let token = entry.value.clone();
                            state.token = Some(entry);
                            refreshed = true;
                            Ok(token)
                        }
                        Err(err) => Err(err),
                    };
                    (notify_opt, outcome)
                };
                if let Some(n) = notify_to_signal {
                    n.notify_waiters();
                }
                if refreshed {
                    self.state_notify.notify_waiters();
                }
                return outcome;
            } else {
                notify.notified().await;
            }
        }
    }

    pub async fn clear(&self) {
        let mut state = self.state.lock().await;
        state.token = None;
        drop(state);
        self.state_notify.notify_waiters();
    }

    pub async fn on_unauthorized_retry(&self) {
        let mut state = self.state.lock().await;
        if let Some(entry) = &mut state.token {
            let now = self.clock.now_instant();
            entry.refresh_at = now.checked_sub(MIN_DURATION).unwrap_or(now);
        }
        drop(state);
        self.state_notify.notify_waiters();
    }

    pub async fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.stop_notify.notify_waiters();
        let notify = {
            let mut state = self.state.lock().await;
            state.token = None;
            let inflight = state.inflight.take();
            drop(state);
            self.state_notify.notify_waiters();
            inflight
        };
        if let Some(waiters) = notify {
            waiters.notify_waiters();
        }
        if let Some(handle) = self.take_bg_handle().await {
            let _ = handle.await;
        }
    }

    pub async fn start_bg(&self) {
        let mut guard = self.bg_task.lock().await;
        if guard.is_some() {
            return;
        }
        if self.stopped.load(Ordering::SeqCst) {
            self.stopped.store(false, Ordering::SeqCst);
        }
        let manager = self.clone();
        let handle = tokio::spawn(async move {
            manager.background_loop().await;
        });
        *guard = Some(handle);
    }

    pub async fn stop_bg(&self) {
        self.stop().await;
    }

    async fn reauth(&self, now: Instant) -> Result<TokenEntry, TokenManagerError> {
        if self.stopped.load(Ordering::SeqCst) {
            return Err(TokenManagerError::Stopped);
        }

        let attempts = self.config.max_retries.saturating_add(1);
        let mut last_error: Option<SiweError> = None;

        for attempt in 1..=attempts {
            if self.stopped.load(Ordering::SeqCst) {
                return Err(TokenManagerError::Stopped);
            }

            let login_fut = self.auth.login();
            tokio::pin!(login_fut);
            let stop_fut = self.stop_notify.notified();
            tokio::pin!(stop_fut);

            let bundle = tokio::select! {
                res = &mut login_fut => res,
                _ = &mut stop_fut => return Err(TokenManagerError::Stopped),
            };

            match bundle {
                Ok(bundle) => match self.bundle_to_entry(&bundle, now).await {
                    Ok(entry) => {
                        let ttl = entry.refresh_at.saturating_duration_since(now);
                        info!(
                            expires_in_ms = ttl.as_millis(),
                            "Refreshed DDS access token"
                        );
                        return Ok(entry);
                    }
                    Err(err) => {
                        warn!(attempt, error = %err, "DDS SIWE response invalid");
                        last_error = Some(err);
                    }
                },
                Err(err) => {
                    warn!(attempt, error = %err, "DDS SIWE login failed");
                    last_error = Some(err);
                }
            }

            if attempt < attempts {
                let delay = self.next_delay().await;
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Err(TokenManagerError::Authentication {
            attempts,
            last_error: last_error.unwrap_or_else(|| SiweError::MissingField("access_token")),
        })
    }

    async fn bundle_to_entry(
        &self,
        bundle: &AccessBundle,
        now: Instant,
    ) -> Result<TokenEntry, SiweError> {
        let now_utc = self.clock.now_utc();
        let expires_at = bundle.expires_at();
        let ttl = expires_at
            .signed_duration_since(now_utc)
            .to_std()
            .map_err(|_| SiweError::MissingField("expires_at"))?;
        if ttl.is_zero() {
            return Err(SiweError::MissingField("expires_at"));
        }

        let ratio = self.config.clamped_ratio();
        let safe_secs = ttl.as_secs_f64() * ratio;
        let safe_duration = if safe_secs <= 0.0 {
            MIN_DURATION.min(ttl)
        } else {
            Duration::from_secs_f64(safe_secs).min(ttl)
        };

        let actual_expiry = now + ttl;
        let base_refresh = now + safe_duration;

        let jitter_ms = self.sample_jitter_ms().await;
        let refresh_at = if jitter_ms >= 0 {
            let add = Duration::from_millis(jitter_ms as u64);
            base_refresh + add
        } else {
            let sub = Duration::from_millis((-jitter_ms) as u64);
            base_refresh.checked_sub(sub).unwrap_or(now)
        };

        let min_refresh = now + MIN_DURATION.min(ttl);
        let mut refresh_at = refresh_at.max(min_refresh);
        refresh_at = refresh_at.min(actual_expiry);

        Ok(TokenEntry {
            value: bundle.token().to_string(),
            refresh_at,
        })
    }

    async fn next_delay(&self) -> Duration {
        if self.config.jitter.is_zero() {
            return Duration::ZERO;
        }
        let mut rng = self.rng.lock().await;
        let max_ms = self.config.jitter.as_millis() as u64;
        if max_ms == 0 {
            Duration::ZERO
        } else {
            let jitter_ms = rng.gen_range(0..=max_ms);
            Duration::from_millis(jitter_ms)
        }
    }

    async fn sample_jitter_ms(&self) -> i64 {
        if self.config.jitter.is_zero() {
            return 0;
        }
        let max_ms_u128 = self.config.jitter.as_millis();
        let max_ms = max_ms_u128.min(i64::MAX as u128) as i64;
        if max_ms == 0 {
            return 0;
        }
        let mut rng = self.rng.lock().await;
        rng.gen_range(-max_ms..=max_ms)
    }

    async fn take_bg_handle(&self) -> Option<JoinHandle<()>> {
        let mut guard = self.bg_task.lock().await;
        guard.take()
    }

    async fn background_loop(self) {
        loop {
            if self.stopped.load(Ordering::SeqCst) {
                break;
            }

            match self.next_refresh_target().await {
                Some(target) => {
                    let now = self.clock.now_instant();
                    if target <= now {
                        if let Err(err) = self.get_access(now).await {
                            warn!(error = %err, "Background reauth attempt failed");
                        }
                        continue;
                    }

                    let delay = target.saturating_duration_since(now);

                    tokio::select! {
                        _ = self.stop_notify.notified() => break,
                        _ = self.state_notify.notified() => continue,
                        _ = sleep(delay) => {}
                    }

                    if self.stopped.load(Ordering::SeqCst) {
                        break;
                    }

                    let now = self.clock.now_instant();
                    if let Err(err) = self.get_access(now).await {
                        warn!(error = %err, "Background reauth attempt failed");
                    }
                }
                None => {
                    tokio::select! {
                        _ = self.stop_notify.notified() => break,
                        _ = self.state_notify.notified() => {}
                    }
                }
            }
        }
    }

    async fn next_refresh_target(&self) -> Option<Instant> {
        let state = self.state.lock().await;
        state.token.as_ref().map(|entry| entry.refresh_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::VecDeque,
        sync::{
            atomic::{AtomicUsize, Ordering as AtomicOrdering},
            Arc,
        },
    };
    use tokio::task::yield_now;
    use tokio::time::advance;

    struct TestClock {
        start_instant: Instant,
        instant_offset: std::sync::Mutex<Duration>,
        start_utc: DateTime<Utc>,
        utc_offset: std::sync::Mutex<chrono::Duration>,
    }

    impl TestClock {
        fn new() -> Self {
            Self {
                start_instant: Instant::now(),
                instant_offset: std::sync::Mutex::new(Duration::ZERO),
                start_utc: Utc::now(),
                utc_offset: std::sync::Mutex::new(chrono::Duration::zero()),
            }
        }
    }

    impl Clock for TestClock {
        fn now_instant(&self) -> Instant {
            let offset = self.instant_offset.lock().unwrap();
            self.start_instant + *offset
        }

        fn now_utc(&self) -> DateTime<Utc> {
            let offset = self.utc_offset.lock().unwrap();
            self.start_utc + *offset
        }
    }

    struct QueueAuthenticator {
        calls: AtomicUsize,
        responses: Mutex<VecDeque<Result<AccessBundle, SiweError>>>,
    }

    impl QueueAuthenticator {
        fn new(responses: VecDeque<Result<AccessBundle, SiweError>>) -> Self {
            Self {
                calls: AtomicUsize::new(0),
                responses: Mutex::new(responses),
            }
        }

        fn calls(&self) -> usize {
            self.calls.load(AtomicOrdering::SeqCst)
        }
    }

    #[async_trait]
    impl AccessAuthenticator for QueueAuthenticator {
        async fn login(&self) -> Result<AccessBundle, SiweError> {
            self.calls.fetch_add(1, AtomicOrdering::SeqCst);
            let mut guard = self.responses.lock().await;
            guard
                .pop_front()
                .unwrap_or_else(|| Err(SiweError::MissingField("access_token")))
        }
    }

    fn access_bundle(token: &str, ttl_secs: i64) -> AccessBundle {
        let expires_at = Utc::now() + chrono::Duration::seconds(ttl_secs);
        AccessBundle::new(token.to_string(), expires_at)
    }

    struct TokioTestClock {
        start_tokio: tokio::time::Instant,
        start_std: Instant,
        start_utc: DateTime<Utc>,
    }

    impl TokioTestClock {
        fn new() -> Self {
            let start_tokio = tokio::time::Instant::now();
            Self {
                start_std: start_tokio.into_std(),
                start_utc: Utc::now(),
                start_tokio,
            }
        }
    }

    impl Clock for TokioTestClock {
        fn now_instant(&self) -> Instant {
            let now_tokio = tokio::time::Instant::now();
            let offset = now_tokio.duration_since(self.start_tokio);
            self.start_std + offset
        }

        fn now_utc(&self) -> DateTime<Utc> {
            let now_tokio = tokio::time::Instant::now();
            let offset = now_tokio.duration_since(self.start_tokio);
            self.start_utc + chrono::Duration::from_std(offset).unwrap()
        }
    }

    struct RecordingAuthenticator {
        clock: Arc<TokioTestClock>,
        ttl: chrono::Duration,
        calls: Mutex<Vec<tokio::time::Instant>>,
        counter: AtomicUsize,
    }

    impl RecordingAuthenticator {
        fn new(clock: Arc<TokioTestClock>, ttl: chrono::Duration) -> Self {
            Self {
                clock,
                ttl,
                calls: Mutex::new(Vec::new()),
                counter: AtomicUsize::new(0),
            }
        }

        fn call_count(&self) -> usize {
            self.counter.load(AtomicOrdering::SeqCst)
        }

        async fn call_history(&self) -> Vec<tokio::time::Instant> {
            self.calls.lock().await.clone()
        }
    }

    #[async_trait]
    impl AccessAuthenticator for RecordingAuthenticator {
        async fn login(&self) -> Result<AccessBundle, SiweError> {
            let order = self.counter.fetch_add(1, AtomicOrdering::SeqCst) + 1;
            self.calls.lock().await.push(tokio::time::Instant::now());
            let expires_at = self.clock.now_utc() + self.ttl;
            Ok(AccessBundle::new(format!("token-{order}"), expires_at))
        }
    }

    #[tokio::test]
    async fn concurrent_access_triggers_single_login() {
        let responses = VecDeque::from([Ok(access_bundle("token-1", 3600))]);
        let auth = Arc::new(QueueAuthenticator::new(responses));
        let clock = Arc::new(TestClock::new());
        let manager = TokenManager::with_rng(
            auth.clone(),
            clock.clone(),
            TokenManagerConfig {
                safety_ratio: 1.0,
                max_retries: 0,
                jitter: Duration::ZERO,
            },
            StdRng::seed_from_u64(42),
        );

        let now = clock.now_instant();
        let tasks = (0..5).map(|_| {
            let manager = manager.clone();
            async move { manager.get_access(now).await.unwrap() }
        });

        let tokens: Vec<_> = futures::future::join_all(tasks).await;
        assert!(tokens.iter().all(|t| t == "token-1"));
        assert_eq!(auth.calls(), 1);
    }

    #[tokio::test]
    async fn unauthorized_forces_refresh_on_next_access() {
        let responses = VecDeque::from([
            Ok(access_bundle("token-1", 3600)),
            Ok(access_bundle("token-2", 3600)),
        ]);
        let auth = Arc::new(QueueAuthenticator::new(responses));
        let clock = Arc::new(TestClock::new());
        let manager = TokenManager::with_rng(
            auth.clone(),
            clock.clone(),
            TokenManagerConfig {
                safety_ratio: 1.0,
                max_retries: 0,
                jitter: Duration::ZERO,
            },
            StdRng::seed_from_u64(1),
        );

        let now = clock.now_instant();
        assert_eq!(manager.get_access(now).await.unwrap(), "token-1");
        manager.on_unauthorized_retry().await;
        let later = clock.now_instant();
        assert_eq!(manager.get_access(later).await.unwrap(), "token-2");
        assert_eq!(auth.calls(), 2);
    }

    #[tokio::test]
    async fn retries_until_success() {
        let responses = VecDeque::from([
            Err(SiweError::MissingField("nonce")),
            Ok(access_bundle("token-3", 3600)),
        ]);
        let auth = Arc::new(QueueAuthenticator::new(responses));
        let clock = Arc::new(TestClock::new());
        let manager = TokenManager::with_rng(
            auth.clone(),
            clock.clone(),
            TokenManagerConfig {
                safety_ratio: 1.0,
                max_retries: 1,
                jitter: Duration::ZERO,
            },
            StdRng::seed_from_u64(7),
        );

        let now = clock.now_instant();
        assert_eq!(manager.get_access(now).await.unwrap(), "token-3");
        assert_eq!(auth.calls(), 2);
    }

    struct HangingAuthenticator {
        calls: AtomicUsize,
        notify: Notify,
    }

    impl HangingAuthenticator {
        fn new() -> Self {
            Self {
                calls: AtomicUsize::new(0),
                notify: Notify::new(),
            }
        }

        fn calls(&self) -> usize {
            self.calls.load(AtomicOrdering::SeqCst)
        }

        fn release(&self) {
            self.notify.notify_waiters();
        }
    }

    #[async_trait]
    impl AccessAuthenticator for HangingAuthenticator {
        async fn login(&self) -> Result<AccessBundle, SiweError> {
            self.calls.fetch_add(1, AtomicOrdering::SeqCst);
            self.notify.notified().await;
            Ok(access_bundle("never", 60))
        }
    }

    #[tokio::test]
    async fn stop_cancels_inflight_refresh() {
        let auth = Arc::new(HangingAuthenticator::new());
        let clock = Arc::new(TestClock::new());
        let manager = TokenManager::with_rng(
            auth.clone(),
            clock.clone(),
            TokenManagerConfig {
                safety_ratio: 1.0,
                max_retries: 0,
                jitter: Duration::ZERO,
            },
            StdRng::seed_from_u64(99),
        );

        let now = clock.now_instant();
        let mgr_clone = manager.clone();
        let handle = tokio::spawn(async move { mgr_clone.get_access(now).await });

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(auth.calls(), 1);

        manager.stop().await;
        let result = handle.await.unwrap();
        assert!(matches!(result, Err(TokenManagerError::Stopped)));

        auth.release();
    }

    #[tokio::test(start_paused = true)]
    async fn background_refresh_occurs_within_jitter() {
        let clock = Arc::new(TokioTestClock::new());
        let ttl = chrono::Duration::seconds(100);
        let auth = Arc::new(RecordingAuthenticator::new(clock.clone(), ttl));
        let config = TokenManagerConfig {
            safety_ratio: 0.5,
            max_retries: 0,
            jitter: Duration::from_secs(10),
        };

        let seed = 123_u64;
        let manager = TokenManager::with_rng(
            auth.clone(),
            clock.clone(),
            config.clone(),
            StdRng::seed_from_u64(seed),
        );

        manager.start_bg().await;
        yield_now().await;

        let now = clock.now_instant();
        assert_eq!(manager.get_access(now).await.unwrap(), "token-1");

        yield_now().await;

        // Compute expected refresh timing based on deterministic RNG
        let ttl_std = Duration::from_secs(100);
        let safe_duration = Duration::from_secs(50);
        let mut rng = StdRng::seed_from_u64(seed);
        let jitter_ms_range = config.jitter.as_millis() as i64;
        let jitter_ms = if jitter_ms_range == 0 {
            0
        } else {
            rng.gen_range(-jitter_ms_range..=jitter_ms_range)
        };

        let mut expected = if jitter_ms >= 0 {
            safe_duration + Duration::from_millis(jitter_ms as u64)
        } else {
            safe_duration
                .checked_sub(Duration::from_millis((-jitter_ms) as u64))
                .unwrap_or(Duration::ZERO)
        };
        let min_refresh = MIN_DURATION.min(ttl_std);
        if expected < min_refresh {
            expected = min_refresh;
        }
        if expected > ttl_std {
            expected = ttl_std;
        }

        let before = expected.saturating_sub(Duration::from_millis(1));
        advance(before).await;
        yield_now().await;
        assert_eq!(auth.call_count(), 1);

        advance(Duration::from_millis(1)).await;
        yield_now().await;

        for _ in 0..5 {
            if auth.call_count() >= 2 {
                break;
            }
            advance(Duration::from_millis(1)).await;
            yield_now().await;
        }

        assert_eq!(auth.call_count(), 2);
        let calls = auth.call_history().await;
        let elapsed = calls[1].duration_since(calls[0]);
        let elapsed_std = Duration::from_secs_f64(elapsed.as_secs_f64());
        assert!(elapsed_std >= expected);
        assert!(elapsed_std <= expected + Duration::from_millis(10));
    }

    #[tokio::test(start_paused = true)]
    async fn background_stop_prevents_refresh() {
        let clock = Arc::new(TokioTestClock::new());
        let ttl = chrono::Duration::seconds(100);
        let auth = Arc::new(RecordingAuthenticator::new(clock.clone(), ttl));
        let config = TokenManagerConfig {
            safety_ratio: 0.5,
            max_retries: 0,
            jitter: Duration::from_secs(5),
        };

        let manager = TokenManager::with_rng(
            auth.clone(),
            clock.clone(),
            config,
            StdRng::seed_from_u64(321),
        );

        manager.start_bg().await;
        yield_now().await;

        let now = clock.now_instant();
        assert_eq!(manager.get_access(now).await.unwrap(), "token-1");

        manager.stop_bg().await;

        advance(Duration::from_secs(200)).await;
        yield_now().await;

        assert_eq!(auth.call_count(), 1);
    }
}
