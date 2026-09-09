use anyhow::{ensure, Result};
use posemesh_compute_node::{
    config::RobotNodeConfig,
    engine::{run_robot_node, RunnerComposition, RunnerRegistry},
    telemetry,
};
use posemesh_p2p_echo::{DemoConfig, EchoRobotRunner};

#[tokio::main]
async fn main() -> Result<()> {
    telemetry::init_from_env()?;
    let demo = DemoConfig::from_env()?;
    let config = RobotNodeConfig::from_env()?;
    ensure!(config.auki_p2p_enabled, "set AUKI_P2P_ENABLED=true");
    let runners = RunnerComposition::with_protocols(move |protocols| {
        RunnerRegistry::new().register(EchoRobotRunner::new(protocols, demo))
    });
    run_robot_node(config, runners).await
}
