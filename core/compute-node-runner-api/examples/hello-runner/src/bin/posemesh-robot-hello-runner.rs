use anyhow::Result;
use posemesh_compute_node::config::RobotNodeConfig;
use posemesh_compute_node::engine::{run_robot_node, RunnerRegistry};
use posemesh_compute_node::telemetry;
use posemesh_hello_runner::HelloRunner;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env from CWD and crate dir for convenience.
    let _ = dotenvy::from_filename(".env");
    let _ = dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env"));

    telemetry::init_from_env()?;

    let cfg = RobotNodeConfig::from_env()?;

    let registry = RunnerRegistry::new().register(HelloRunner);
    let capabilities = registry.capabilities();
    info!(?capabilities, "robot hello runner registered capabilities");

    run_robot_node(cfg, registry).await?;

    Ok(())
}
