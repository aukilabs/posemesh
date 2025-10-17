use compute_runner_api::LeaseEnvelope;
use rand::distributions::{Distribution, Uniform};
use rand::Rng;
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use url::Url;
use uuid::Uuid;

/// Session lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Pending,
    Running,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionState {
    pub task_id: Uuid,
    pub job_id: Option<Uuid>,
    pub capability: String,
    pub meta: Value,
    pub inputs_cids: Vec<String>,
    pub domain_id: Option<Uuid>,
    pub domain_server_url: Option<Url>,
    pub lease_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub access_token: Option<String>,
    pub access_token_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_progress: Option<Value>,
    pub next_heartbeat_due: Option<Instant>,
    pub status: SessionStatus,
    pub cancel: bool,
}

/// Immutable snapshot of session state.
#[derive(Debug, Clone)]
pub struct SessionSnapshot(SessionState);

impl SessionSnapshot {
    pub fn task_id(&self) -> Uuid {
        self.0.task_id
    }

    pub fn job_id(&self) -> Option<Uuid> {
        self.0.job_id
    }

    pub fn capability(&self) -> &str {
        &self.0.capability
    }

    pub fn meta(&self) -> &Value {
        &self.0.meta
    }

    pub fn inputs_cids(&self) -> &[String] {
        &self.0.inputs_cids
    }

    pub fn domain_id(&self) -> Option<Uuid> {
        self.0.domain_id
    }

    pub fn domain_server_url(&self) -> Option<&Url> {
        self.0.domain_server_url.as_ref()
    }

    pub fn access_token(&self) -> Option<&str> {
        self.0.access_token.as_deref()
    }

    pub fn access_token_expires_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.0.access_token_expires_at
    }

    pub fn lease_expires_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.0.lease_expires_at
    }

    pub fn next_heartbeat_due(&self) -> Option<Instant> {
        self.0.next_heartbeat_due
    }

    pub fn status(&self) -> SessionStatus {
        self.0.status
    }

    pub fn cancel(&self) -> bool {
        self.0.cancel
    }
}

/// Capabilities configured for the node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitySelector {
    capabilities: Vec<String>,
}

impl CapabilitySelector {
    pub fn new(capabilities: Vec<String>) -> Self {
        Self { capabilities }
    }

    pub fn choose(&self) -> Option<&str> {
        self.capabilities.first().map(|s| s.as_str())
    }

    pub fn accepts(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }

    pub fn all(&self) -> &[String] {
        &self.capabilities
    }
}

/// Distribution for randomized TTL heartbeats.
#[derive(Debug, Clone, Copy)]
pub struct HeartbeatPolicy {
    pub min_ratio: f64,
    pub max_ratio: f64,
}

impl HeartbeatPolicy {
    pub const fn new(min_ratio: f64, max_ratio: f64) -> Self {
        Self {
            min_ratio,
            max_ratio,
        }
    }

    pub const fn default_policy() -> Self {
        Self {
            min_ratio: 0.55,
            max_ratio: 0.65,
        }
    }

    fn sample_ratio<R: Rng>(&self, rng: &mut R) -> f64 {
        let min = self.min_ratio.min(self.max_ratio).max(0.0);
        let max = self.max_ratio.max(self.min_ratio).max(min);
        let dist = Uniform::new_inclusive(min, max);
        dist.sample(rng)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SessionError {
    #[error("lease did not include a task")]
    MissingTask,
    #[error("task capability `{got}` not in configured set {expected:?}")]
    CapabilityMismatch { expected: Vec<String>, got: String },
    #[error("no active session")]
    NoActiveSession,
}

#[derive(Clone)]
pub struct SessionManager {
    selector: CapabilitySelector,
    state: Arc<Mutex<Option<SessionState>>>,
}

impl SessionManager {
    pub fn new(selector: CapabilitySelector) -> Self {
        Self {
            selector,
            state: Arc::new(Mutex::new(None)),
        }
    }

    pub fn selector(&self) -> &CapabilitySelector {
        &self.selector
    }

    pub async fn snapshot(&self) -> Option<SessionSnapshot> {
        let guard = self.state.lock().await;
        guard.as_ref().cloned().map(SessionSnapshot)
    }

    pub async fn clear(&self) {
        *self.state.lock().await = None;
    }

    pub async fn start_session<R: Rng>(
        &self,
        lease: &LeaseEnvelope,
        now: Instant,
        policy: &HeartbeatPolicy,
        rng: &mut R,
    ) -> Result<SessionSnapshot, SessionError> {
        let task = lease.task.clone();
        if !self.selector.accepts(&task.capability) {
            return Err(SessionError::CapabilityMismatch {
                expected: self.selector.all().to_vec(),
                got: task.capability,
            });
        }

        let mut state = SessionState {
            task_id: task.id,
            job_id: task.job_id,
            capability: task.capability,
            meta: task.meta,
            inputs_cids: task.inputs_cids,
            domain_id: lease.domain_id,
            domain_server_url: extract_domain_server_url(lease),
            lease_expires_at: lease.lease_expires_at,
            access_token: lease.access_token.clone(),
            access_token_expires_at: lease.access_token_expires_at,
            last_progress: None,
            next_heartbeat_due: None,
            status: SessionStatus::Pending,
            cancel: lease.cancel,
        };
        state.next_heartbeat_due = compute_next_heartbeat(now, state.lease_expires_at, policy, rng);

        *self.state.lock().await = Some(state.clone());
        Ok(SessionSnapshot(state))
    }

    pub async fn apply_heartbeat<R: Rng>(
        &self,
        lease: &LeaseEnvelope,
        progress: Option<Value>,
        now: Instant,
        policy: &HeartbeatPolicy,
        rng: &mut R,
    ) -> Result<SessionSnapshot, SessionError> {
        let mut guard = self.state.lock().await;
        let state = guard.as_mut().ok_or(SessionError::NoActiveSession)?;

        let task = lease.task.clone();
        state.task_id = task.id;
        state.job_id = task.job_id;
        state.capability = task.capability;
        state.meta = task.meta;
        state.inputs_cids = task.inputs_cids;

        if let Some(domain_id) = lease.domain_id {
            state.domain_id = Some(domain_id);
        }
        if let Some(url) = extract_domain_server_url(lease) {
            state.domain_server_url = Some(url);
        }

        if let Some(token) = &lease.access_token {
            state.access_token = Some(token.clone());
        }
        if let Some(expiry) = lease.access_token_expires_at {
            state.access_token_expires_at = Some(expiry);
        }
        if let Some(lease_expiry) = lease.lease_expires_at {
            state.lease_expires_at = Some(lease_expiry);
        }

        state.last_progress = progress;
        state.status = SessionStatus::Running;
        state.cancel = lease.cancel;
        state.next_heartbeat_due = compute_next_heartbeat(now, state.lease_expires_at, policy, rng);

        Ok(SessionSnapshot(state.clone()))
    }
}

fn compute_next_heartbeat<R: Rng>(
    now: Instant,
    lease_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    policy: &HeartbeatPolicy,
    rng: &mut R,
) -> Option<Instant> {
    let expires = lease_expires_at?;
    let ttl = expires.signed_duration_since(chrono::Utc::now());
    if ttl.num_milliseconds() <= 0 {
        return Some(now);
    }
    let ttl = ttl.to_std().ok()?;
    let ratio = policy.sample_ratio(rng).clamp(0.0, 1.0);
    let mut delay = ttl.mul_f64(ratio);
    if delay > ttl {
        delay = ttl;
    }
    if delay.is_zero() {
        delay = Duration::from_millis(100);
    }
    Some(now + delay.min(ttl))
}

fn extract_domain_server_url(lease: &LeaseEnvelope) -> Option<Url> {
    if let Some(url) = &lease.domain_server_url {
        return Some(url.clone());
    }
    lookup_domain_url_from_meta(&lease.task.meta)
}

fn lookup_domain_url_from_meta(meta: &Value) -> Option<Url> {
    meta.get("domain_server_url")
        .and_then(|value| value.as_str())
        .and_then(|raw| Url::parse(raw).ok())
        .or_else(|| {
            meta.get("legacy")
                .and_then(|legacy| legacy.get("domain_server_url"))
                .and_then(|value| value.as_str())
                .and_then(|raw| Url::parse(raw).ok())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration as ChronoDuration, Utc};
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use serde_json::json;
    use uuid::Uuid;

    fn selector() -> CapabilitySelector {
        CapabilitySelector::new(vec!["cap-a".to_string(), "cap-b".to_string()])
    }

    fn policy() -> HeartbeatPolicy {
        HeartbeatPolicy::default_policy()
    }

    fn lease_base() -> LeaseEnvelope {
        let now = Utc::now();
        LeaseEnvelope {
            access_token: Some("token".into()),
            access_token_expires_at: Some(now + ChronoDuration::minutes(5)),
            lease_expires_at: Some(now + ChronoDuration::minutes(10)),
            cancel: false,
            status: None,
            domain_id: None,
            domain_server_url: None,
            task: compute_runner_api::TaskSpec {
                id: Uuid::new_v4(),
                job_id: Some(Uuid::new_v4()),
                capability: "cap-a".into(),
                capability_filters: json!({}),
                inputs_cids: vec!["cid-1".into()],
                outputs_prefix: None,
                label: None,
                stage: None,
                meta: json!({ "hello": "world" }),
                priority: None,
                attempts: None,
                max_attempts: None,
                deps_remaining: None,
                status: None,
                mode: None,
                organization_filter: None,
                billing_units: None,
                estimated_credit_cost: None,
                debited_amount: None,
                debited_at: None,
                lease_expires_at: None,
            },
        }
    }

    #[test]
    fn capability_selector_choose() {
        let selector = selector();
        assert_eq!(selector.choose(), Some("cap-a"));
        assert!(selector.accepts("cap-b"));
        assert!(!selector.accepts("other"));
    }

    #[tokio::test]
    async fn start_session_rejects_unknown_capability() {
        let manager = SessionManager::new(selector());
        let mut lease = lease_base();
        lease.task.capability = "other".into();
        let mut rng = StdRng::seed_from_u64(123);
        let res = manager
            .start_session(&lease, Instant::now(), &policy(), &mut rng)
            .await;
        assert_eq!(
            res.unwrap_err(),
            SessionError::CapabilityMismatch {
                expected: vec!["cap-a".into(), "cap-b".into()],
                got: "other".into()
            }
        );
    }

    #[tokio::test]
    async fn start_session_sets_next_heartbeat() {
        let manager = SessionManager::new(selector());
        let lease = lease_base();
        let mut rng = StdRng::seed_from_u64(7);
        let snapshot = manager
            .start_session(&lease, Instant::now(), &policy(), &mut rng)
            .await
            .unwrap();
        assert!(snapshot.next_heartbeat_due().is_some());
        assert_eq!(snapshot.status(), SessionStatus::Pending);
    }

    #[tokio::test]
    async fn apply_heartbeat_updates_state_and_cancel_flag() {
        let manager = SessionManager::new(selector());
        let mut lease = lease_base();
        let mut rng = StdRng::seed_from_u64(9);
        manager
            .start_session(&lease, Instant::now(), &policy(), &mut rng)
            .await
            .unwrap();

        lease.cancel = true;
        lease.access_token = Some("new-token".into());
        let snapshot = manager
            .apply_heartbeat(
                &lease,
                Some(json!({"pct": 42})),
                Instant::now(),
                &policy(),
                &mut rng,
            )
            .await
            .unwrap();

        assert_eq!(snapshot.access_token(), Some("new-token"));
        assert!(snapshot.cancel());
        assert_eq!(snapshot.status(), SessionStatus::Running);
        assert!(snapshot.next_heartbeat_due().is_some());
    }
}
