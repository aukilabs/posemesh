use anyhow::{Context, Result};
use async_trait::async_trait;
use posemesh_compute_node::engine::RunnerRegistry;
use posemesh_compute_node::telemetry;
use posemesh_compute_node_runner_api as compute_runner_api;
use serde_json::json;
use std::path::Path;
use tracing::info;
use uuid::Uuid;

struct HelloRunner;

#[async_trait]
impl compute_runner_api::Runner for HelloRunner {
    fn capability(&self) -> &'static str {
        "/examples/hello/v1"
    }

    async fn run(&self, ctx: compute_runner_api::TaskCtx<'_>) -> Result<()> {
        let lease = ctx.lease;

        // Attach common task identifiers to every tracing event emitted in this task.
        let task_span = telemetry::task_span(
            lease.task.id,
            lease.task.job_id.unwrap_or_else(Uuid::nil),
            &lease.task.capability,
            lease.domain_id.unwrap_or_else(Uuid::nil),
        );
        let _span_guard = task_span.enter();

        const FILE_NAME: &str = "posemesh-multipart-test.bin";
        let file_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FILE_NAME);

        ctx.ctrl
            .progress(json!({ "status": "started", "file": FILE_NAME }))
            .await?;

        let file_size = tokio::fs::metadata(&file_path)
            .await
            .with_context(|| format!("stat {}", file_path.display()))?
            .len();

        ctx.ctrl
            .log_event(json!({
                "level": "info",
                "message": "uploading file",
                "task_id": lease.task.id,
                "job_id": lease.task.job_id,
                "path": file_path.to_string_lossy(),
                "rel_path": FILE_NAME,
                "bytes": file_size,
            }))
            .await?;

        info!(
            task_id = %lease.task.id,
            job_id = ?lease.task.job_id,
            path = %file_path.display(),
            rel_path = FILE_NAME,
            bytes = file_size,
            "uploading file"
        );

        ctx.output
            .put_file(FILE_NAME, &file_path)
            .await
            .with_context(|| format!("upload {}", file_path.display()))?;

        ctx.ctrl
            .progress(json!({
                "status": "finished",
                "rel_path": FILE_NAME,
                "bytes_written": file_size
            }))
            .await?;

        info!(
            bytes_written = file_size,
            rel_path = FILE_NAME,
            "hello-runner finished"
        );

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env from CWD and crate dir for convenience.
    let _ = dotenvy::from_filename(".env");
    let _ = dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env"));

    telemetry::init_from_env()?;

    let app = posemesh_compute_node::http::router();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    let addr = listener.local_addr()?;
    println!("http listening on {}", addr);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let cfg = posemesh_compute_node::config::NodeConfig::from_env()?;

    let registry = RunnerRegistry::new().register(HelloRunner);
    let capabilities = registry.capabilities();

    posemesh_compute_node::dds::register::spawn_registration_if_configured(&cfg, &capabilities)?;
    info!(?capabilities, "hello runner registered capabilities");

    posemesh_compute_node::engine::run_node(cfg, registry).await?;

    Ok(())
}
