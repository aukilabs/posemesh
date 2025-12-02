use anyhow::{Context, Result};
use async_trait::async_trait;
use compute_runner_api::runner::{DomainArtifactContent, DomainArtifactRequest};
use posemesh_compute_node::engine::RunnerRegistry;
use posemesh_compute_node::telemetry;
use posemesh_compute_node_runner_api as compute_runner_api;
use serde_json::json;
use std::process::Stdio;
use tokio::process::Command;
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

        let message = lease
            .task
            .meta
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Hello from posemesh-hello-runner!");

        ctx.ctrl
            .progress(json!({ "status": "started", "message": message }))
            .await?;

        ctx.ctrl
            .log_event(json!({
                "level": "info",
                "message": "hello-runner task started",
                "task_id": lease.task.id,
                "job_id": lease.task.job_id,
                "stage": lease.task.stage,
                "inputs_cids": lease.task.inputs_cids,
            }))
            .await?;
        info!(task_id=%lease.task.id, job_id=?lease.task.job_id, inputs=?lease.task.inputs_cids, "hello-runner started");

        info!(inputs = ?lease.task.inputs_cids, "leasing inputs");

        // If an input CID is provided, materialize it and pass its contents to Python.
        let maybe_cid = lease.task.inputs_cids.first().cloned();
        let mut input_preview = "<none>".to_string();
        let processed_text: String;

        if let Some(cid) = maybe_cid.as_deref() {
            ctx.ctrl
                .log_event(json!({
                    "level": "info",
                    "message": "materializing input cid",
                    "cid": cid,
                }))
                .await?;

            info!(%cid, "materializing input cid");

            let materialized = ctx
                .input
                .materialize_cid_with_meta(cid)
                .await
                .with_context(|| format!("materialize cid {}", cid))?;

            // Pick the largest part among primary + related files. Some domain
            // objects include a tiny manifest alongside the real payload; this
            // avoids previewing an empty JSON file.
            let mut candidates = vec![materialized.path.clone()];
            candidates.extend(materialized.related_files.clone());

            let mut chosen_path = materialized.path.clone();
            let mut chosen_bytes: Vec<u8> = Vec::new();
            let mut max_len: u64 = 0;
            let mut sizes = Vec::new();

            for p in candidates {
                let len = tokio::fs::metadata(&p).await.map(|m| m.len()).unwrap_or(0);
                sizes.push((p.to_string_lossy().to_string(), len));
                if len >= max_len {
                    if let Ok(data) = tokio::fs::read(&p).await {
                        max_len = len;
                        chosen_path = p.clone();
                        chosen_bytes = data;
                    }
                }
            }

            let bytes = chosen_bytes;
            input_preview = String::from_utf8_lossy(&bytes).chars().take(128).collect();

            let related_files: Vec<String> = materialized
                .related_files
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();

            ctx.ctrl
                .log_event(json!({
                    "level": "info",
                    "message": "fetched input cid",
                    "cid": cid,
                    "path": chosen_path.to_string_lossy(),
                    "bytes": bytes.len(),
                    "data_id": materialized.data_id,
                    "related_files": related_files,
                    "candidate_sizes": sizes,
                }))
                .await?;

            info!(
                %cid,
                path = %chosen_path.display(),
                bytes = bytes.len(),
                data_id = ?materialized.data_id,
                related_files = ?related_files,
                "fetched input cid"
            );

            // Call a tiny Python snippet that uppercases the payload.
            let output = Command::new("python3")
                .arg("-c")
                .arg("import sys; data=sys.stdin.read(); print(data.upper())")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?;

            let mut child = output;
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                stdin.write_all(&bytes).await?;
            }
            let out = child.wait_with_output().await?;

            if !out.status.success() {
                anyhow::bail!(
                    "python transform failed: status={}, stderr={}",
                    out.status,
                    String::from_utf8_lossy(&out.stderr)
                );
            }

            processed_text = String::from_utf8_lossy(&out.stdout).to_string();

            ctx.ctrl
                .log_event(json!({
                    "level": "info",
                    "message": "transformed input to uppercase",
                    "cid": cid,
                    "output_bytes": processed_text.len(),
                }))
                .await?;

            info!(%cid, output_bytes = processed_text.len(), "transformed input to uppercase");
        } else {
            processed_text = format!("{message}\n(no input cid provided)\n");
            ctx.ctrl
                .log_event(json!({
                    "level": "warn",
                    "message": "no input cid provided; returning default message",
                }))
                .await?;

            info!("no input cid provided; returning default message");
        }

        // Upload the result text and capture the Domain data id (if returned).
        let data_id = ctx
            .output
            .put_domain_artifact(DomainArtifactRequest {
                rel_path: "hello.txt",
                name: "hello_txt",
                data_type: "txt",
                existing_id: None,
                content: DomainArtifactContent::Bytes(processed_text.as_bytes()),
            })
            .await?;

        ctx.ctrl
            .log_event(json!({
                "level": "info",
                "message": "uploaded output artifact",
                "rel_path": "hello.txt",
                "bytes": processed_text.len(),
                "domain_data_id": data_id,
            }))
            .await?;

        info!(bytes = processed_text.len(), domain_data_id = ?data_id, "uploaded output artifact");

        ctx.ctrl
            .progress(json!({
                "status": "finished",
                "bytes_written": processed_text.len(),
                "input_preview": input_preview,
                "new_domain_data_id": data_id
            }))
            .await?;

        info!(bytes_written = processed_text.len(), input_preview = %input_preview, domain_data_id = ?data_id, "hello-runner finished");

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
