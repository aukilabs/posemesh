use anyhow::{ensure, Result};
use posemesh_compute_node::{
    config::NodeConfig,
    dds::register::spawn_registration_if_configured,
    engine::{run_node, RunnerComposition, RunnerRegistry},
    telemetry,
};
use posemesh_p2p_echo::{DemoConfig, EchoComputeRunner, SEND_CAPABILITY};

#[tokio::main]
async fn main() -> Result<()> {
    telemetry::init_from_env()?;
    let demo = DemoConfig::from_env()?;
    let config = NodeConfig::from_env()?;
    ensure!(config.auki_p2p_enabled, "set AUKI_P2P_ENABLED=true");
    spawn_registration_if_configured(&config, &[SEND_CAPABILITY.into()])?;
    let runners = RunnerComposition::with_protocols(move |protocols| {
        RunnerRegistry::new().register(EchoComputeRunner::new(protocols, demo))
    });
    run_node(config, runners).await
}
