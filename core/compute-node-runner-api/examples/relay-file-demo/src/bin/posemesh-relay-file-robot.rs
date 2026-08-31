use std::sync::Arc;

use anyhow::{bail, Context, Result};
use posemesh_compute_node::{
    config::RobotNodeConfig,
    engine::{run_robot_node_with_shutdowns, RunnerComposition, RunnerRegistry},
    telemetry,
};
use posemesh_relay_file_demo::{
    config::{load_env_file, RobotDemoConfig},
    jobs::{JobClient, ReconstructionJobSubmitter},
    runners::RobotFilePublisher,
};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let env_file = load_env_file(".env.robot")?;
    telemetry::init_from_env()?;

    let cfg = RobotNodeConfig::from_env().context("load Robot node configuration")?;
    if cfg.relay_config().is_none() {
        bail!("the relay demo requires AUKI_P2P_ENABLED=true and relay mode auto or always");
    }
    if !cfg.auki_p2p_advertised_multiaddrs.is_empty() {
        bail!(
            "the relay demo requires an empty AUKI_P2P_ADVERTISED_MULTIADDRS so the published reference is circuit-only"
        );
    }
    let demo = RobotDemoConfig::from_env().context("load Robot demo configuration")?;
    let run_id = Uuid::new_v4();
    let jobs = Arc::new(JobClient::new(demo.jobs.clone())?);
    let submitter: Arc<dyn ReconstructionJobSubmitter> = jobs.clone();
    let runner_demo = demo.clone();
    let runners = RunnerComposition::with_dataset(move |dataset| {
        RunnerRegistry::new().register(RobotFilePublisher::new(
            runner_demo,
            run_id,
            dataset,
            submitter,
        ))
    });

    let shutdown = CancellationToken::new();
    let forced_shutdown = CancellationToken::new();
    let signal_task = spawn_signal_handler(shutdown.clone(), forced_shutdown.clone());
    info!(
        %run_id,
        env_file = %env_file.display(),
        source = %demo.source_file.display(),
        reference_file = %demo.reference_file.display(),
        "starting the dev relay-file Robot demo"
    );

    let mut engine = Box::pin(run_robot_node_with_shutdowns(
        cfg,
        runners,
        shutdown.clone(),
        forced_shutdown.clone(),
    ));
    let mut submit = Box::pin(jobs.submit_robot(run_id, &shutdown));

    let result = tokio::select! {
        engine_result = &mut engine => engine_result,
        submission_result = &mut submit => {
            match submission_result {
                Ok(job_id) => {
                    info!(%run_id, %job_id, "Robot demo job queued; waiting for transfer and shutdown");
                    engine.await
                }
                Err(error) if shutdown.is_cancelled() => {
                    warn!(%run_id, error = %error, "Robot demo job submission stopped during shutdown");
                    engine.await
                }
                Err(error) => {
                    shutdown.cancel();
                    forced_shutdown.cancel();
                    if let Err(cleanup_error) = engine.await {
                        warn!(error = %cleanup_error, "Robot engine cleanup also failed");
                    }
                    Err(error.context("queue initial Robot demo job"))
                }
            }
        }
    };

    shutdown.cancel();
    forced_shutdown.cancel();
    signal_task.abort();
    let _ = signal_task.await;
    result
}

fn spawn_signal_handler(
    shutdown: CancellationToken,
    forced_shutdown: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            warn!(
                "shutdown requested; draining published dataset references (Ctrl-C again to force)"
            );
            shutdown.cancel();
        }
        if tokio::signal::ctrl_c().await.is_ok() {
            warn!("forced shutdown requested; published references may be lost");
            forced_shutdown.cancel();
        }
    })
}
