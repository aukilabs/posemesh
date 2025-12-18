use posemesh_compute_node_runner_api::{runner::MultipartUpload, *};
use serde_json::json;

struct DummyInput;
#[async_trait::async_trait]
impl InputSource for DummyInput {
    async fn get_bytes_by_cid(&self, cid: &str) -> anyhow::Result<Vec<u8>> {
        Ok(cid.as_bytes().to_vec())
    }
    async fn materialize_cid_to_temp(&self, cid: &str) -> anyhow::Result<std::path::PathBuf> {
        let p = std::env::temp_dir().join(format!("cid-{cid}"));
        Ok(p)
    }
}

struct DummyUpload(Vec<u8>);
#[async_trait::async_trait]
impl MultipartUpload for DummyUpload {
    async fn write_chunk(&mut self, chunk: &[u8]) -> anyhow::Result<()> {
        self.0.extend_from_slice(chunk);
        Ok(())
    }
    async fn finish(self: Box<Self>) -> anyhow::Result<()> {
        assert!(!self.0.is_empty());
        Ok(())
    }
}

struct DummySink;
#[async_trait::async_trait]
impl ArtifactSink for DummySink {
    async fn put_bytes(&self, _rel_path: &str, bytes: &[u8]) -> anyhow::Result<()> {
        assert!(!bytes.is_empty());
        Ok(())
    }
    async fn put_file(&self, _rel_path: &str, _file_path: &std::path::Path) -> anyhow::Result<()> {
        Ok(())
    }
    async fn open_multipart(&self, _rel_path: &str) -> anyhow::Result<Box<dyn MultipartUpload>> {
        Ok(Box::new(DummyUpload(Vec::new())))
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

struct DummyRunner;
#[async_trait::async_trait]
impl Runner for DummyRunner {
    fn capability(&self) -> &'static str {
        "/dummy/v1"
    }
    async fn run(&self, ctx: TaskCtx<'_>) -> anyhow::Result<()> {
        // exercise ports
        let cid = ctx
            .lease
            .task
            .inputs_cids
            .first()
            .cloned()
            .unwrap_or_else(|| "x".into());
        let b = ctx.input.get_bytes_by_cid(&cid).await?;
        ctx.output.put_bytes("ack.txt", &b).await?;
        ctx.ctrl.progress(json!({"ok": true})).await?;
        Ok(())
    }
}

#[tokio::test]
async fn task_ctx_wiring_and_object_safety() {
    use chrono::Utc;
    use uuid::Uuid;
    let lease = LeaseEnvelope {
        access_token: Some("t".into()),
        access_token_expires_at: Some(Utc::now()),
        lease_expires_at: Some(Utc::now()),
        cancel: false,
        status: Some("leased".into()),
        domain_id: Some(Uuid::new_v4()),
        domain_server_url: Some("https://example.d/".parse().unwrap()),
        task: TaskSpec {
            id: Uuid::new_v4(),
            job_id: Some(Uuid::new_v4()),
            capability: "/dummy/v1".into(),
            capability_filters: json!({}),
            inputs_cids: vec!["cid123".into()],
            outputs_prefix: Some("outputs".into()),
            label: None,
            stage: None,
            meta: json!({}),
            priority: None,
            attempts: None,
            max_attempts: None,
            deps_remaining: None,
            status: Some("queued".into()),
            mode: None,
            organization_filter: None,
            billing_units: None,
            estimated_credit_cost: None,
            debited_amount: None,
            debited_at: None,
            lease_expires_at: None,
        },
    };

    let input: Box<dyn InputSource> = Box::new(DummyInput);
    let output: Box<dyn ArtifactSink> = Box::new(DummySink);
    let ctrl: Box<dyn ControlPlane> = Box::new(DummyCtrl);

    // Ensure Send + Sync object-safety by boxing behind Arc as well
    let input = &*input;
    let output = &*output;
    let ctrl = &*ctrl;
    struct DummyToken;
    impl posemesh_compute_node_runner_api::runner::AccessTokenProvider for DummyToken {
        fn get(&self) -> String {
            "t".into()
        }
    }
    let tok = DummyToken;
    let ctx = TaskCtx {
        lease: &lease,
        input,
        output,
        ctrl,
        access_token: &tok,
    };

    let r = DummyRunner;
    r.run(ctx).await.unwrap();
}

#[test]
fn task_spec_priority_allows_negative_values() {
    use uuid::Uuid;

    let spec: TaskSpec = serde_json::from_value(json!({
        "id": Uuid::nil(),
        "capability": "/dummy/v1",
        "capability_filters": {},
        "inputs_cids": [],
        "meta": {},
        "priority": -3
    }))
    .unwrap();

    assert_eq!(spec.priority, Some(-3));
}
