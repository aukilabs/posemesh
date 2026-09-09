use anyhow::{bail, ensure, Context, Result};
use async_trait::async_trait;
use auki_p2p::PeerId;
use auki_portable_echo::{EchoClient, EchoEndpoint, EchoRequest, EchoServeEvent, PROTOCOL_ID};
use auki_sdk::AukiPeerProtocolContext;
use compute_runner_api::{ControlPlane, Runner, TaskCtx};
use posemesh_compute_node::engine::AukiProtocolsHandle;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

pub const SERVE_CAPABILITY: &str = "/examples/p2p-echo/serve/v1";
pub const SEND_CAPABILITY: &str = "/examples/p2p-echo/send/v1";

/// Explicit placement expected by both example processes. DDS/DMS remain the
/// authority for organization membership, Robot assignment and task placement.
#[derive(Clone, Copy)]
pub struct DemoConfig {
    pub organization_id: Uuid,
    pub domain_id: Uuid,
}

impl DemoConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            organization_id: std::env::var("ECHO_ORGANIZATION_ID")
                .context("set ECHO_ORGANIZATION_ID for both workers")?
                .parse()
                .context("ECHO_ORGANIZATION_ID must be a UUID")?,
            domain_id: std::env::var("ECHO_DOMAIN_ID")
                .context("set ECHO_DOMAIN_ID to the Robot's assigned Domain")?
                .parse()
                .context("ECHO_DOMAIN_ID must be a UUID")?,
        })
    }

    fn validate(
        &self,
        ctx: &TaskCtx<'_>,
        peer: &AukiPeerProtocolContext,
        role: &str,
    ) -> Result<()> {
        ensure!(
            ctx.lease.task.mode.as_deref() == Some("dedicated"),
            "Echo requires a dedicated task"
        );
        ensure!(
            ctx.lease.domain_id == Some(self.domain_id),
            "task Domain differs from ECHO_DOMAIN_ID"
        );
        ensure!(
            peer.domain_id() == self.domain_id,
            "peer Domain differs from ECHO_DOMAIN_ID"
        );
        let authorization = peer.authorization().current()?;
        ensure!(
            authorization.peer_type() == Some(role),
            "unexpected peer role for this runner"
        );
        let organization = authorization
            .claims()
            .organization_id
            .as_deref()
            .context("DDS P2P credential has no organization_id")?
            .parse::<Uuid>()
            .context("invalid P2P organization_id")?;
        ensure!(
            organization == self.organization_id,
            "peer organization differs from ECHO_ORGANIZATION_ID"
        );
        Ok(())
    }
}

#[derive(Deserialize)]
struct ServeTask {
    run_id: Uuid,
    message: String,
    expected_compute_peer_id: String,
    #[serde(default = "serve_timeout")]
    timeout_seconds: u64,
}

fn serve_timeout() -> u64 {
    120
}

#[derive(Deserialize)]
struct SendTask {
    run_id: Uuid,
    message: String,
    robot_peer_id: String,
    route: String,
}

fn echo_payload(run_id: Uuid, message: &str) -> Result<Vec<u8>> {
    ensure!(!message.is_empty(), "message must not be empty");
    Ok(EchoRequest::new(serde_json::to_vec(&(run_id, message))?)?.into_bytes())
}

async fn cancelled(ctrl: &dyn ControlPlane) {
    while !ctrl.is_cancelled().await {
        sleep(Duration::from_millis(100)).await;
    }
}

pub struct EchoRobotRunner {
    protocols: AukiProtocolsHandle,
    config: DemoConfig,
}

impl EchoRobotRunner {
    pub fn new(protocols: AukiProtocolsHandle, config: DemoConfig) -> Self {
        Self { protocols, config }
    }
}

#[async_trait]
impl Runner for EchoRobotRunner {
    fn capability(&self) -> &'static str {
        SERVE_CAPABILITY
    }

    async fn run(&self, ctx: TaskCtx<'_>) -> Result<()> {
        let task: ServeTask = serde_json::from_value(ctx.lease.task.meta.clone())
            .context("decode Echo serve task")?;
        ensure!(
            (1..=600).contains(&task.timeout_seconds),
            "timeout_seconds must be 1..=600"
        );
        let payload = echo_payload(task.run_id, &task.message)?;
        let expected_peer: PeerId = task
            .expected_compute_peer_id
            .parse()
            .context("invalid Compute Peer ID")?;
        let peer = self.protocols.get()?;
        self.config.validate(&ctx, &peer, "robot")?;
        ensure!(
            expected_peer != peer.peer_id(),
            "Compute and Robot need separate peer identities"
        );

        let endpoint = EchoEndpoint::mount(peer.protocols())?;
        let operation = async {
            let routes = peer.routes().snapshot()?;
            let route = routes
                .relay_routes
                .first()
                .map(|relay| relay.routes.tcp().clone())
                .or_else(|| routes.direct_routes.first().cloned())
                .context("Robot has no confirmed relay or advertised direct route")?;
            ctx.ctrl.progress(json!({
                "phase": "ready", "run_id": task.run_id, "peer_id": peer.peer_id().to_string(),
                "domain_id": peer.domain_id(), "organization_id": self.config.organization_id,
                "route": route.to_string(), "protocol": PROTOCOL_ID,
                "expected_compute_peer_id": expected_peer.to_string(),
            })).await?;
            tracing::info!(run_id = %task.run_id, peer_id = %peer.peer_id(), %route, "Robot Echo ready");
            let events = endpoint.events();
            timeout(Duration::from_secs(task.timeout_seconds), async {
                loop {
                    let event = tokio::select! {
                        _ = cancelled(ctx.ctrl) => bail!("Echo serve task cancelled"),
                        event = events.recv() => event.context("Echo endpoint stopped")?,
                    };
                    match event {
                        EchoServeEvent::Served(receipt)
                            if receipt.remote_peer_id == expected_peer && receipt.payload == payload => {
                                return Ok(());
                            }
                        EchoServeEvent::Failed { remote_peer_id, error }
                            if remote_peer_id == expected_peer => {
                                bail!("expected Compute Echo failed: {error}");
                            }
                        EchoServeEvent::Lagged { .. } => {
                            bail!("Echo observations overflowed");
                        }
                        _ => {},
                    }
                }
            }).await.context("timed out waiting for the matching Compute Echo")??;
            ctx.ctrl.progress(json!({
                "phase": "echoed", "run_id": task.run_id, "remote_peer_id": expected_peer.to_string(),
                "message": task.message,
            })).await?;
            Ok::<(), anyhow::Error>(())
        }.await;
        let cleanup = endpoint.close().await;
        operation?;
        cleanup?;
        Ok(())
    }
}

pub struct EchoComputeRunner {
    protocols: AukiProtocolsHandle,
    config: DemoConfig,
}

impl EchoComputeRunner {
    pub fn new(protocols: AukiProtocolsHandle, config: DemoConfig) -> Self {
        Self { protocols, config }
    }
}

#[async_trait]
impl Runner for EchoComputeRunner {
    fn capability(&self) -> &'static str {
        SEND_CAPABILITY
    }

    async fn run(&self, ctx: TaskCtx<'_>) -> Result<()> {
        let task: SendTask =
            serde_json::from_value(ctx.lease.task.meta.clone()).context("decode Echo send task")?;
        let peer = self
            .protocols
            .get()
            .context("no task P2P peer; check the Compute key and DMS peer-bound lease support")?;
        self.config.validate(&ctx, &peer, "compute")?;
        let remote_peer: PeerId = task
            .robot_peer_id
            .parse()
            .context("invalid Robot Peer ID")?;
        ensure!(
            remote_peer != peer.peer_id(),
            "Compute and Robot need separate peer identities"
        );
        let payload = echo_payload(task.run_id, &task.message)?;
        let client = EchoClient::new(peer.protocols());
        let receipt = tokio::select! {
            _ = cancelled(ctx.ctrl) => bail!("Echo send task cancelled"),
            result = client.send_exact(remote_peer, task.route.parse().context("invalid Robot route")?, payload) => result?,
        };
        ctx.ctrl.progress(json!({
            "phase": "verified", "run_id": task.run_id, "remote_peer_id": receipt.remote_peer_id.to_string(),
            "message": task.message, "relayed": receipt.relayed,
        })).await?;
        tracing::info!(run_id = %task.run_id, remote_peer_id = %receipt.remote_peer_id, relayed = receipt.relayed, "Compute Echo verified");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_messages_are_correlated_and_bounded() {
        let run_id = Uuid::new_v4();
        assert_eq!(
            echo_payload(run_id, "hello").unwrap(),
            echo_payload(run_id, "hello").unwrap()
        );
        assert_ne!(
            echo_payload(run_id, "hello").unwrap(),
            echo_payload(Uuid::new_v4(), "hello").unwrap()
        );
        assert!(echo_payload(run_id, "").is_err());
        assert!(echo_payload(run_id, &"x".repeat(1024)).is_err());
    }
}
