# Use P2P in a runner

Inject the host's `AukiProtocolsHandle` into your runner, then call `get()`
inside each task. The host owns the authenticated peer, authority renewal,
and shutdown. Your runner owns its protocol operations and registrations.
See the [paired Echo tutorial](../tutorials/robot-and-compute.md) for a complete
working sender and server.

## Use the host's peer

Starting from [Write a runner](write-a-runner.md), add `time` to Tokio's features
and this dependency to your application's `[dependencies]`:

~~~toml
auki-portable-echo = { git = "https://github.com/aukilabs/auki-sdk", rev = "3ee1142529d5f64af87f6fc2276d3a6a78e728cc" }
~~~

That revision matches this checkout's [SDK dependencies](../../core/Cargo.toml).
Keep any direct SDK dependencies on the same Git source and revision as the
host. Mixing a local SDK path or another revision creates incompatible Rust
types at the protocol boundary.

For example, replace `src/lib.rs` with this task-driven Echo sender:

~~~rust
use anyhow::{bail, ensure, Context, Result};
use async_trait::async_trait;
use auki_portable_echo::EchoClient;
use compute_runner_api::{Runner, TaskCtx};
use posemesh_compute_node::engine::AukiProtocolsHandle;
use serde_json::json;
use std::time::Duration;

pub const CAPABILITY: &str = "/my-team/echo/v1";

pub struct PeerRunner {
    protocols: AukiProtocolsHandle,
}

impl PeerRunner {
    pub fn new(protocols: AukiProtocolsHandle) -> Self {
        Self { protocols }
    }
}

#[async_trait]
impl Runner for PeerRunner {
    fn capability(&self) -> &'static str {
        CAPABILITY
    }

    async fn run(&self, ctx: TaskCtx<'_>) -> Result<()> {
        let peer = self.protocols.get()?;
        ensure!(ctx.lease.domain_id == Some(peer.domain_id()), "peer Domain differs from task");
        let meta = &ctx.lease.task.meta;
        let remote = meta["peer_id"].as_str().context("missing peer_id")?.parse()?;
        let route = meta["route"].as_str().context("missing route")?.parse()?;
        let message = meta["message"].as_str().context("missing message")?;
        ensure!(!ctx.ctrl.is_cancelled().await, "task cancelled");
        let client = EchoClient::new(peer.protocols());
        let cancelled = async {
            while !ctx.ctrl.is_cancelled().await {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        };
        let receipt = tokio::select! {
            _ = cancelled => bail!("task cancelled"),
            result = client.send_exact(remote, route, message.as_bytes().to_vec()) => result?,
        };
        ctx.ctrl.progress(json!({
            "phase": "verified", "remote_peer_id": receipt.remote_peer_id.to_string()
        })).await?;
        Ok(())
    }
}
~~~

In both host binaries, replace the `UppercaseRunner` import and registry
construction with these imports and construction:

~~~rust
use my_runner::PeerRunner;
use posemesh_compute_node::engine::{RunnerComposition, RunnerRegistry};

// Inside main, after loading configuration:
let runners = RunnerComposition::with_protocols(|protocols| {
    RunnerRegistry::new().register(PeerRunner::new(protocols))
});
~~~

Remove `RunnerRegistry` from the earlier grouped engine import to avoid a
duplicate import. In the compute binary, register the known capability with
`spawn_registration_if_configured(&config, &[my_runner::CAPABILITY.into()])?`
instead of calling `capabilities()` on the composition. Keep each host's
existing `run_node` or `run_robot_node` call.

Enable P2P and provide a persistent [identity and routes](../reference/configuration.md#p2p).
Run `cargo check --bins` to check your changes. The job's capability is now
`/my-team/echo/v1`; metadata needs `peer_id`, `route`, and a 1–1024-byte `message`.
The remote peer must already serve `/example/echo/1.0.0` in the same Domain.
This minimal sender is for a generic Echo endpoint. The paired tutorial has
its own payload format that also includes a run ID; use its shipped sender
when talking to its robot task.

## Serve requests and choose routes

Mount your protocol on `peer.protocols()`. Hold its registration while the
task serves requests, then await closure on every outcome. The
[Echo robot runner](../../core/compute-node-runner-api/examples/p2p-echo/src/lib.rs)
shows mounting `EchoEndpoint`, publishing readiness, waiting with cancellation
and a timeout, and closing the endpoint before returning.

`peer.routes().snapshot()` supplies current advertised direct and confirmed
relay routes. Exchange a reachable route and Peer ID through your application.
The paired tutorial carries them in DMS progress and subsequent task metadata;
it does not enable DDS discovery. A route is a connection hint, not authority.

Do not call `get()` in a runner constructor: composition happens before the
peer is ready. Compute fills the handle for the current task and clears it
when that task ends; robot fills it for the process's assigned Domain.
Do not start another SDK peer with the same private key inside a runner.

## Define your own protocol

Follow the SDK's [custom protocol guide](https://github.com/aukilabs/auki-sdk/blob/main/docs/how-to/protocols.md)
for protocol IDs, framing, bounded reads, handlers, and exact-route streams.
Use a new protocol ID for incompatible wire changes. Keep network operations
bounded and respond to [task cancellation](task-lifecycle.md).

Authorize application operations in the handler before performing them.
Authenticated P2P admission and a matching Peer ID do not grant arbitrary
Domain writes or robot commands. Echo's expected-peer check observes a served
response; it is demonstration correlation, not an admission rule for privileged
operations. Keep DMS task authorization, transport identity, and application
permissions explicit in your own protocol.
