use compute_runner_api::{ArtifactSink, ControlPlane, InputSource, Runner, TaskCtx};
use posemesh_compute_node::engine::RunnerRegistry;
use posemesh_p2p_dataset::{
    DatasetService, P2pDataset, P2pDatasetReference, P2pDatasetRegistration,
};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

struct DummyInput;
#[async_trait::async_trait]
impl InputSource for DummyInput {
    async fn get_bytes_by_cid(&self, _cid: &str) -> anyhow::Result<Vec<u8>> {
        Ok(vec![])
    }
    async fn materialize_cid_to_temp(&self, _cid: &str) -> anyhow::Result<std::path::PathBuf> {
        Ok(std::env::temp_dir())
    }
}

struct DummySink;
#[async_trait::async_trait]
impl ArtifactSink for DummySink {
    async fn put_bytes(&self, _rel_path: &str, _bytes: &[u8]) -> anyhow::Result<()> {
        Ok(())
    }
    async fn put_file(&self, _rel_path: &str, _file_path: &std::path::Path) -> anyhow::Result<()> {
        Ok(())
    }
}

struct DummyCtrl;
#[async_trait::async_trait]
impl ControlPlane for DummyCtrl {
    async fn is_cancelled(&self) -> bool {
        false
    }
    async fn progress(&self, _value: serde_json::Value) -> anyhow::Result<()> {
        Ok(())
    }
    async fn log_event(&self, _fields: serde_json::Value) -> anyhow::Result<()> {
        Ok(())
    }
}

struct RCount {
    cap: &'static str,
    count: Arc<AtomicUsize>,
}
#[async_trait::async_trait]
impl Runner for RCount {
    fn capability(&self) -> &'static str {
        self.cap
    }
    async fn run(&self, ctx: TaskCtx<'_>) -> anyhow::Result<()> {
        let _ = ctx;
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

struct DummyP2pDataset;

#[async_trait::async_trait]
impl P2pDataset for DummyP2pDataset {
    async fn register(
        &self,
        _registration: P2pDatasetRegistration,
    ) -> anyhow::Result<P2pDatasetReference> {
        anyhow::bail!("not used")
    }

    async fn fetch(
        &self,
        _reference: &P2pDatasetReference,
        _destination: &Path,
    ) -> anyhow::Result<()> {
        anyhow::bail!("not used")
    }
}

struct BoundaryRunner {
    dataset: DatasetService,
    saw_narrow_handle: Arc<AtomicBool>,
}

#[async_trait::async_trait]
impl Runner for BoundaryRunner {
    fn capability(&self) -> &'static str {
        "/p2p-boundary"
    }

    async fn run(&self, ctx: TaskCtx<'_>) -> anyhow::Result<()> {
        assert_eq!(
            ctx.lease.access_token.as_deref(),
            Some("domain-http-secret")
        );
        assert_eq!(ctx.access_token.get(), "domain-http-secret");
        assert!(ctx.lease.p2p_access_token.is_none());
        assert!(ctx.lease.p2p_access_token_expires_at.is_none());
        let _dataset = &self.dataset;
        self.saw_narrow_handle.store(true, Ordering::SeqCst);
        Ok(())
    }
}

fn fake_lease(cap: &str) -> compute_runner_api::LeaseEnvelope {
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;
    compute_runner_api::LeaseEnvelope {
        access_token: Some("t".into()),
        access_token_expires_at: Some(Utc::now()),
        p2p_access_token: None,
        p2p_access_token_expires_at: None,
        lease_expires_at: Some(Utc::now()),
        cancel: false,
        status: Some("leased".into()),
        domain_id: Some(Uuid::new_v4()),
        domain_server_url: Some("https://domain.example".parse().unwrap()),
        task: compute_runner_api::TaskSpec {
            id: Uuid::new_v4(),
            job_id: Some(Uuid::new_v4()),
            capability: cap.into(),
            capability_filters: json!({}),
            inputs_cids: vec![],
            outputs_prefix: Some("out".into()),
            label: None,
            stage: None,
            meta: json!({}),
            priority: None,
            attempts: None,
            max_attempts: None,
            deps_remaining: None,
            status: Some("leased".into()),
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

#[tokio::test]
async fn dispatches_to_correct_runner_only() {
    let c_a = Arc::new(AtomicUsize::new(0));
    let c_b = Arc::new(AtomicUsize::new(0));
    let reg = RunnerRegistry::new()
        .register(RCount {
            cap: "/a",
            count: c_a.clone(),
        })
        .register(RCount {
            cap: "/b",
            count: c_b.clone(),
        });

    let lease = fake_lease("/b");
    let input = DummyInput;
    let output = DummySink;
    let ctrl = DummyCtrl;
    struct DummyToken;
    impl compute_runner_api::runner::AccessTokenProvider for DummyToken {
        fn get(&self) -> String {
            "t".into()
        }
    }
    let tok = DummyToken;
    reg.run_for_lease(&lease, &input, &output, &ctrl, &tok)
        .await
        .expect("run ok");

    assert_eq!(c_a.load(Ordering::SeqCst), 0);
    assert_eq!(c_b.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn runner_receives_constructor_injected_handle_but_not_p2p_credentials() {
    let saw_narrow_handle = Arc::new(AtomicBool::new(false));
    let p2p: Arc<dyn P2pDataset> = Arc::new(DummyP2pDataset);
    let registry = RunnerRegistry::new().register(BoundaryRunner {
        dataset: DatasetService::new(p2p),
        saw_narrow_handle: saw_narrow_handle.clone(),
    });
    let mut lease = fake_lease("/p2p-boundary");
    lease.access_token = Some("domain-http-secret".into());
    lease.p2p_access_token = Some("dds-p2p-secret".into());
    lease.p2p_access_token_expires_at = Some(chrono::Utc::now());
    let input = DummyInput;
    let output = DummySink;
    let ctrl = DummyCtrl;
    struct DomainToken;
    impl compute_runner_api::runner::AccessTokenProvider for DomainToken {
        fn get(&self) -> String {
            "domain-http-secret".into()
        }
    }

    registry
        .run_for_lease(&lease, &input, &output, &ctrl, &DomainToken)
        .await
        .expect("runner boundary");

    assert!(saw_narrow_handle.load(Ordering::SeqCst));
    assert_eq!(lease.p2p_access_token.as_deref(), Some("dds-p2p-secret"));
}
