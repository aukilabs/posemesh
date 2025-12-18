# posemesh-compute-node-runner-api

`posemesh-compute-node-runner-api` defines the narrow contract between the compute node
engine and capability-specific runners. The crate is intentionally tiny: it
contains no HTTP clients or storage logic, just the traits and data models a
runner needs in order to receive work, download inputs, and upload artifacts.

## What lives here
- `types.rs` — serde-friendly structs mirroring the DMS lease envelope and task
  specification (`LeaseEnvelope`, `TaskSpec`).
- `runner.rs` — async traits that make up the runner interface:
  `Runner`, `TaskCtx`, `InputSource`, `ArtifactSink`, `ControlPlane`, and
  helpers like `MaterializedInput`.
- `CRATE_NAME` — a stable identifier used by workspace smoke tests to assert
  that every crate was compiled and linked.

## Runner interface at a glance
- `Runner` — implement `capability()` and `run()` to register your capability.
- `TaskCtx` — passed to `run()`, bundles the current lease, an input source,
  an artifact sink, and a control-plane for cancellation/progress.
- `InputSource` — abstraction over fetching CIDs from domain storage; comes with
  helpers to materialize CIDs to temp files.
- `ArtifactSink` — abstraction over uploading result artifacts; supports bytes,
  files, and optional multipart streaming.
- `ControlPlane` — lets runners observe cancellation and push progress / log
  events that will get relayed via heartbeats.

```rust
use anyhow::Result;
use async_trait::async_trait;
use posemesh_compute_node_runner_api::{Runner, TaskCtx};
use serde_json::json;

struct HelloRunner;

#[async_trait]
impl Runner for HelloRunner {
    fn capability(&self) -> &'static str {
        "/examples/hello/v1"
    }

    async fn run(&self, ctx: TaskCtx<'_>) -> Result<()> {
        let lease = ctx.lease;
        ctx.ctrl.progress(json!({ "status": "started" })).await?;
        let bytes = ctx.input.get_bytes_by_cid(&lease.task.inputs_cids[0]).await?;
        ctx.output.put_bytes("outputs/hello.bin", &bytes).await?;
        ctx.ctrl.progress(json!({ "status": "finished" })).await?;
        Ok(())
    }
}
```

## Development notes
- `cargo test -p posemesh-compute-node-runner-api` exercises trait object safety and serde
  round-trips of the contract types.
- The crate is `no_std`-out-of-scope by design; runner implementers are expected
  to depend on Tokio and friends via their own crates.
- Changes here should be treated as breaking API changes for every runner,
  so prefer additive evolution and keep the interface documentable in a README.
