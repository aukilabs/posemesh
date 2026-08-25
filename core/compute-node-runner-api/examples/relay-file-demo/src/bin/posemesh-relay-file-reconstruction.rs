use anyhow::{bail, Context, Result};
use posemesh_compute_node::{
    config::NodeConfig,
    dds::register::spawn_registration_if_configured,
    engine::{run_node, RunnerComposition, RunnerRegistry},
    telemetry,
};
use posemesh_relay_file_demo::{
    config::{load_env_file, ReconstructionDemoConfig},
    jobs::RECONSTRUCTION_CAPABILITY,
    runners::ReconstructionFileDownloader,
};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let env_file = load_env_file(".env.reconstruction")?;
    telemetry::init_from_env()?;

    let cfg = NodeConfig::from_env().context("load reconstruction node configuration")?;
    if !cfg.auki_p2p_enabled {
        bail!("the relay demo reconstruction node requires AUKI_P2P_ENABLED=true");
    }
    let demo =
        ReconstructionDemoConfig::from_env().context("load reconstruction demo configuration")?;
    let capabilities = vec![RECONSTRUCTION_CAPABILITY.to_string()];
    spawn_registration_if_configured(&cfg, &capabilities)?;
    let runner_demo = demo.clone();
    let runners = RunnerComposition::with_dataset(move |dataset| {
        RunnerRegistry::new().register(ReconstructionFileDownloader::new(runner_demo, dataset))
    });

    info!(
        env_file = %env_file.display(),
        destination = %demo.output_file.display(),
        ?capabilities,
        "starting the dev relay-file reconstruction demo"
    );
    run_node(cfg, runners).await
}
