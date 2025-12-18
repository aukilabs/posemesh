use compute_runner_api::LeaseEnvelope;
use posemesh_compute_node::dms::types;
use posemesh_compute_node::engine::{
    ControlState, HeartbeatDriver, HeartbeatDriverArgs, HeartbeatLoopResult, HeartbeatTransport,
};
use posemesh_compute_node::heartbeat::progress_channel;
use posemesh_compute_node::session::HeartbeatPolicy;
use posemesh_compute_node::session::{CapabilitySelector, SessionManager};
use posemesh_compute_node::storage::TokenRef;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde_json::json;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use url::Url;
use uuid::Uuid;

#[derive(Clone)]
struct StubTransport {
    responses: Arc<Mutex<VecDeque<anyhow::Result<types::HeartbeatResponse>>>>,
    requests: Arc<Mutex<Vec<types::HeartbeatRequest>>>,
}

impl StubTransport {
    fn new(responses: Vec<anyhow::Result<types::HeartbeatResponse>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn call_count(&self) -> usize {
        self.requests.lock().await.len()
    }
}

#[async_trait::async_trait]
impl HeartbeatTransport for StubTransport {
    async fn post_heartbeat(
        &self,
        task_id: Uuid,
        body: &types::HeartbeatRequest,
    ) -> anyhow::Result<types::HeartbeatResponse> {
        let mut reqs = self.requests.lock().await;
        reqs.push(body.clone());
        drop(reqs);

        let mut guard = self.responses.lock().await;
        guard
            .pop_front()
            .unwrap_or_else(|| Ok(heartbeat_from_lease(&test_lease(task_id, 5_000, false))))
    }
}

#[tokio::test]
async fn heartbeat_driver_triggers_on_ttl() {
    let capability = "/cap";
    let lease = test_lease_with_capability(capability, 200, false);
    let selector = CapabilitySelector::new(vec![capability.to_string()]);
    let session = SessionManager::new(selector);
    let policy = HeartbeatPolicy::default_policy();
    let mut rng = StdRng::seed_from_u64(7);
    session
        .start_session(&lease, std::time::Instant::now(), &policy, &mut rng)
        .await
        .unwrap();

    let transport = StubTransport::new(vec![Ok(heartbeat_from_lease(&lease))]);
    let token_ref = TokenRef::new(lease.access_token.clone().unwrap());
    let (progress_tx, progress_rx) = progress_channel();
    let control_state = Arc::new(Mutex::new(ControlState::default()));
    progress_tx.update(json!({}), json!({}));
    let runner_cancel = CancellationToken::new();
    let shutdown = CancellationToken::new();

    let driver = HeartbeatDriver::new(
        transport.clone(),
        HeartbeatDriverArgs {
            session,
            policy,
            rng,
            progress_rx,
            state: control_state,
            token_ref,
            runner_cancel: runner_cancel.clone(),
            shutdown: shutdown.clone(),
            task_id: lease.task.id,
        },
    );

    let handle = tokio::spawn(async move { driver.run().await });
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    shutdown.cancel();
    let result = handle.await.unwrap();
    assert!(matches!(result, HeartbeatLoopResult::Completed));
    assert!(transport.call_count().await >= 1);
}

#[tokio::test]
async fn heartbeat_driver_triggers_on_progress() {
    let capability = "/cap";
    let lease = test_lease_with_capability(capability, 5_000, false);
    let selector = CapabilitySelector::new(vec![capability.to_string()]);
    let session = SessionManager::new(selector);
    let policy = HeartbeatPolicy::default_policy();
    let mut rng = StdRng::seed_from_u64(11);
    session
        .start_session(&lease, std::time::Instant::now(), &policy, &mut rng)
        .await
        .unwrap();

    let transport = StubTransport::new(vec![Ok(heartbeat_from_lease(&lease))]);
    let token_ref = TokenRef::new(lease.access_token.clone().unwrap());
    let (progress_tx, progress_rx) = progress_channel();
    let control_state = Arc::new(Mutex::new(ControlState::default()));
    let runner_cancel = CancellationToken::new();
    let shutdown = CancellationToken::new();

    let driver = HeartbeatDriver::new(
        transport.clone(),
        HeartbeatDriverArgs {
            session,
            policy,
            rng,
            progress_rx,
            state: control_state.clone(),
            token_ref,
            runner_cancel: runner_cancel.clone(),
            shutdown: shutdown.clone(),
            task_id: lease.task.id,
        },
    );

    let handle = tokio::spawn(async move { driver.run().await });
    progress_tx.update(json!({"pct": 10}), json!({}));
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    shutdown.cancel();
    let result = handle.await.unwrap();
    assert!(matches!(result, HeartbeatLoopResult::Completed));
    assert!(transport.call_count().await >= 1);
}

#[tokio::test]
async fn heartbeat_driver_signals_cancellation() {
    let capability = "/cap";
    let lease = test_lease_with_capability(capability, 5_000, false);
    let selector = CapabilitySelector::new(vec![capability.to_string()]);
    let session = SessionManager::new(selector);
    let policy = HeartbeatPolicy::default_policy();
    let mut rng = StdRng::seed_from_u64(13);
    session
        .start_session(&lease, std::time::Instant::now(), &policy, &mut rng)
        .await
        .unwrap();

    let cancel_response = {
        let mut cancelled = lease.clone();
        cancelled.cancel = true;
        heartbeat_from_lease(&cancelled)
    };
    let transport = StubTransport::new(vec![Ok(cancel_response)]);
    let token_ref = TokenRef::new(lease.access_token.clone().unwrap());
    let (progress_tx, progress_rx) = progress_channel();
    let control_state = Arc::new(Mutex::new(ControlState::default()));
    let runner_cancel = CancellationToken::new();
    let shutdown = CancellationToken::new();

    let driver = HeartbeatDriver::new(
        transport.clone(),
        HeartbeatDriverArgs {
            session,
            policy,
            rng,
            progress_rx,
            state: control_state,
            token_ref,
            runner_cancel: runner_cancel.clone(),
            shutdown: shutdown.clone(),
            task_id: lease.task.id,
        },
    );

    let handle = tokio::spawn(async move { driver.run().await });
    progress_tx.update(json!({"pct": 10}), json!({}));
    let result = handle.await.unwrap();
    assert!(matches!(result, HeartbeatLoopResult::Cancelled));
    assert!(runner_cancel.is_cancelled());
}

#[tokio::test]
async fn heartbeat_driver_reports_lost_lease() {
    let capability = "/cap";
    let lease = test_lease_with_capability(capability, 5_000, false);
    let selector = CapabilitySelector::new(vec![capability.to_string()]);
    let session = SessionManager::new(selector);
    let policy = HeartbeatPolicy::default_policy();
    let mut rng = StdRng::seed_from_u64(17);
    session
        .start_session(&lease, std::time::Instant::now(), &policy, &mut rng)
        .await
        .unwrap();

    let transport = StubTransport::new(vec![Err(anyhow::anyhow!("network failure"))]);
    let token_ref = TokenRef::new(lease.access_token.clone().unwrap());
    let (progress_tx, progress_rx) = progress_channel();
    let control_state = Arc::new(Mutex::new(ControlState::default()));
    let runner_cancel = CancellationToken::new();
    let shutdown = CancellationToken::new();

    let driver = HeartbeatDriver::new(
        transport.clone(),
        HeartbeatDriverArgs {
            session,
            policy,
            rng,
            progress_rx,
            state: control_state,
            token_ref,
            runner_cancel: runner_cancel.clone(),
            shutdown: shutdown.clone(),
            task_id: lease.task.id,
        },
    );

    let handle = tokio::spawn(async move { driver.run().await });
    progress_tx.update(json!({"pct": 30}), json!({}));
    let result = handle.await.unwrap();
    assert!(matches!(result, HeartbeatLoopResult::LostLease(_)));
    assert!(runner_cancel.is_cancelled());
}

fn test_lease(task_id: Uuid, ttl_ms: i64, cancel: bool) -> LeaseEnvelope {
    let now = chrono::Utc::now();
    LeaseEnvelope {
        access_token: Some("token".into()),
        access_token_expires_at: Some(now + chrono::Duration::minutes(5)),
        lease_expires_at: Some(now + chrono::Duration::milliseconds(ttl_ms)),
        cancel,
        status: None,
        domain_id: None,
        domain_server_url: Some(Url::parse("https://example.com").unwrap()),
        task: compute_runner_api::TaskSpec {
            id: task_id,
            job_id: Some(Uuid::new_v4()),
            capability: "/cap".into(),
            capability_filters: json!({}),
            inputs_cids: vec![],
            outputs_prefix: None,
            label: None,
            stage: None,
            meta: json!({}),
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

fn test_lease_with_capability(capability: &str, ttl_ms: i64, cancel: bool) -> LeaseEnvelope {
    let mut lease = test_lease(Uuid::new_v4(), ttl_ms, cancel);
    lease.task.capability = capability.into();
    lease
}

fn heartbeat_from_lease(lease: &LeaseEnvelope) -> types::HeartbeatResponse {
    types::HeartbeatResponse {
        access_token: lease.access_token.clone(),
        access_token_expires_at: lease.access_token_expires_at,
        lease_expires_at: lease.lease_expires_at,
        cancel: Some(lease.cancel),
        status: lease.status.clone(),
        domain_id: lease.domain_id,
        domain_server_url: lease.domain_server_url.clone(),
        task: Some(lease.task.clone()),
        task_id: Some(lease.task.id),
        job_id: lease.task.job_id,
        attempts: lease.task.attempts,
        max_attempts: lease.task.max_attempts,
        deps_remaining: lease.task.deps_remaining,
    }
}
