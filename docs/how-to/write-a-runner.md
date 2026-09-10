# Write a runner

A runner implements one DMS capability. The same implementation can run in a
compute host or a robot host. This example reads a message from task metadata
and uploads an uppercase text artifact to the task's Domain.

## Create a project

Use Rust 1.89+. Create `my-runner` beside your Posemesh checkout:

~~~text
work/
  posemesh/
  my-runner/
    Cargo.toml
    src/lib.rs
    src/bin/compute.rs
    src/bin/robot.rs
~~~

Put this in `my-runner/Cargo.toml`:

~~~toml
[package]
name = "my-runner"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
async-trait = "0.1"
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
posemesh-compute-node = { path = "../posemesh/core/compute-node" }
compute-runner-api = { package = "posemesh-compute-node-runner-api", path = "../posemesh/core/compute-node-runner-api" }
~~~

These paths use the checked-out implementation. When using Git dependencies,
pin both Posemesh crates to the same reviewed revision. Commit your application's
`Cargo.lock` after resolving dependencies.

## Implement the capability

Put this in `src/lib.rs`:

~~~rust
use anyhow::{ensure, Context, Result};
use async_trait::async_trait;
use compute_runner_api::{Runner, TaskCtx};
use serde_json::json;

pub struct UppercaseRunner;

#[async_trait]
impl Runner for UppercaseRunner {
    fn capability(&self) -> &'static str {
        "/my-team/uppercase/v1"
    }

    async fn run(&self, ctx: TaskCtx<'_>) -> Result<()> {
        let message = ctx.lease.task.meta["message"]
            .as_str()
            .context("task.meta.message must be a string")?;
        ensure!(!message.is_empty() && message.len() <= 1024, "message must be 1..=1024 bytes");
        ensure!(!ctx.ctrl.is_cancelled().await, "task cancelled");
        ctx.ctrl.progress(json!({"phase": "working"})).await?;

        let result = message.to_uppercase();
        ensure!(!ctx.ctrl.is_cancelled().await, "task cancelled");
        ctx.output.put_bytes("uppercase.txt", result.as_bytes()).await?;
        ctx.ctrl.progress(json!({"phase": "done"})).await?;
        Ok(())
    }
}
~~~

Choose your own versioned capability namespace. Use dedicated tasks for custom
capabilities; public scheduling has additional capability and pricing policy.
The capability string must match in the runner, worker registration, and job.
`/auki/` and `auki/` namespaces are reserved.

## Add the hosts

Put this in `src/bin/compute.rs`:

~~~rust
use anyhow::Result;
use my_runner::UppercaseRunner;
use posemesh_compute_node::{
    config::NodeConfig,
    dds::register::spawn_registration_if_configured,
    engine::{run_node, RunnerRegistry},
    telemetry,
};

#[tokio::main]
async fn main() -> Result<()> {
    telemetry::init_from_env()?;
    let config = NodeConfig::from_env()?;
    let runners = RunnerRegistry::new().register(UppercaseRunner);
    spawn_registration_if_configured(&config, &runners.capabilities())?;
    run_node(config, runners).await
}
~~~

The compute host needs the explicit registration call to advertise its
capabilities. `run_node` handles task execution after authentication.

Put this in `src/bin/robot.rs`:

~~~rust
use anyhow::Result;
use my_runner::UppercaseRunner;
use posemesh_compute_node::{
    config::RobotNodeConfig,
    engine::{run_robot_node, RunnerRegistry},
    telemetry,
};

#[tokio::main]
async fn main() -> Result<()> {
    telemetry::init_from_env()?;
    let config = RobotNodeConfig::from_env()?;
    let runners = RunnerRegistry::new().register(UppercaseRunner);
    run_robot_node(config, runners).await
}
~~~

The robot entrypoint registers the registry's capabilities itself. Provision
and assign the robot as described in [provisioning](provision-workers.md#create-and-assign-a-robot),
using `/my-team/uppercase/v1` as its capability.

From `my-runner`, compile both without contacting DDS or DMS:

~~~sh
cargo check --bins
~~~

Then [configure a worker](configure-workers.md), leave `AUKI_P2P_ENABLED=false`,
and run **one** host with `cargo run --locked --bin compute` or
`cargo run --locked --bin robot`. This runner needs no P2P key.

## Submit a task and read its artifact

With `DMS_BASE_URL`, `DOMAIN_ID`, `APP_JWT_FILE`, and `WORKER_STATE_DIR` from
[job authorization](provision-workers.md#authorize-job-submission), use `curl` and `jq`:

~~~sh
umask 077
jq -Rrn '"Authorization: Bearer \(input)"' < "$APP_JWT_FILE" > "$WORKER_STATE_DIR/job.headers"
jq -n --arg domain "$DOMAIN_ID" '{
  label: "uppercase-example", domain_id: $domain, priority: 0,
  tasks: [{
    label: "uppercase", stage: "transform", capability: "/my-team/uppercase/v1",
    capability_filters: {}, mode: "dedicated", inputs_cids: [],
    outputs_prefix: "uppercase-example/", max_attempts: 1,
    meta: {message: "hello from my runner"}
  }],
  edges: []
}' > job.json
curl --fail-with-body --silent --show-error \
  --header @"$WORKER_STATE_DIR/job.headers" --header 'Content-Type: application/json' \
  --data-binary @job.json "$DMS_BASE_URL/jobs" > job-created.json
export JOB_ID="$(jq -er '.job_id' job-created.json)"
curl --fail-with-body --silent --show-error \
  --header @"$WORKER_STATE_DIR/job.headers" "$DMS_BASE_URL/jobs/$JOB_ID" > job-status.json
jq '.tasks[] | {id, status, meta}' job-status.json
jq '.receipts[] | {task_id, outputs, artifacts: .meta.artifacts}' job-status.json
~~~

Repeat the last request until the task is `completed` or `failed`.
The job response separates `tasks` from `receipts`. A completion receipt contains
the uploaded artifact's Domain data ID in `meta.artifacts[].id` (the full response
path is `receipts[].meta.artifacts[].id`); use your Domain client to download
that ID. Progress reports execution state, while the artifact contains the
result. Cancel an unfinished job with `POST $DMS_BASE_URL/jobs/$JOB_ID/cancel`
using the same header.

For input artifacts, read `ctx.lease.task.inputs_cids` through `ctx.input`.
For explicit Domain metadata or updates to an existing data ID, use
`put_domain_artifact`; see the [runner reference](../reference/runner.md).
Before adding long-running work, implement [cancellation and retry handling](task-lifecycle.md).
Add peer communication through [the host's P2P handle](use-p2p.md).
