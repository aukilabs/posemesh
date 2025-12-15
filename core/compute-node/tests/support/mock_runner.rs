use anyhow::Result;
use async_trait::async_trait;
use compute_runner_api::{Runner, TaskCtx};
use posemesh_compute_node::engine::RunnerRegistry;
use serde_json::json;

pub const MOCK_CAPABILITY: &str = "/posemesh/mock/v1";
pub const MOCK_CAPABILITY_LOCAL: &str = "/posemesh/mock/local/v1";
pub const MOCK_CAPABILITY_GLOBAL: &str = "/posemesh/mock/global/v1";
pub const MOCK_CAPABILITIES: [&str; 3] = [
    MOCK_CAPABILITY_LOCAL,
    MOCK_CAPABILITY_GLOBAL,
    MOCK_CAPABILITY,
];

/// Construct a registry with a single mock runner for convenience.
pub fn registry_with_mock() -> RunnerRegistry {
    RunnerRegistry::new().register(MockRunner::new())
}

/// Build mock runners for every advertised capability.
pub fn runners_for_all_capabilities() -> Vec<MockRunner> {
    MOCK_CAPABILITIES
        .iter()
        .copied()
        .map(MockRunner::with_capability)
        .collect()
}

#[derive(Clone, Copy, Default)]
pub struct MockRunner {
    capability: &'static str,
}

impl MockRunner {
    pub fn new() -> Self {
        Self::with_capability(MOCK_CAPABILITY)
    }

    pub fn with_capability(capability: &'static str) -> Self {
        Self { capability }
    }
}

#[async_trait]
impl Runner for MockRunner {
    fn capability(&self) -> &'static str {
        self.capability
    }

    async fn run(&self, ctx: TaskCtx<'_>) -> Result<()> {
        // Touch inputs so tests verify download paths.
        for cid in &ctx.lease.task.inputs_cids {
            let _ = ctx.input.get_bytes_by_cid(cid).await?;
        }

        // Produce a minimal artifact so storage uploads fire.
        let prefix = ctx
            .lease
            .task
            .outputs_prefix
            .clone()
            .unwrap_or_else(|| "outputs".into());
        let path = format!("{}/mock-output.json", prefix);
        ctx.output.put_bytes(&path, br#"{"status":"ok"}"#).await?;

        // Emit a progress entry to exercise heartbeat code paths.
        ctx.ctrl
            .progress(json!({ "capability": self.capability }))
            .await?;

        Ok(())
    }
}
