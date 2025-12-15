use compute_runner_api::{Runner, TaskCtx};
use posemesh_compute_node::engine::RunnerRegistry;

struct R1;
struct R2;

#[async_trait::async_trait]
impl Runner for R1 {
    fn capability(&self) -> &'static str {
        "/a"
    }
    async fn run(&self, _ctx: TaskCtx<'_>) -> anyhow::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl Runner for R2 {
    fn capability(&self) -> &'static str {
        "/b"
    }
    async fn run(&self, _ctx: TaskCtx<'_>) -> anyhow::Result<()> {
        Ok(())
    }
}

#[test]
fn register_and_lookup_by_capability() {
    let reg = RunnerRegistry::new().register(R1).register(R2);
    let a = reg.get("/a").expect("runner /a");
    let b = reg.get("/b").expect("runner /b");
    assert_eq!(a.capability(), "/a");
    assert_eq!(b.capability(), "/b");
    assert!(reg.get("/missing").is_none());
}
