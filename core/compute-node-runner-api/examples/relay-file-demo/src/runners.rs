use std::{path::Path, sync::Arc};

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use auki_p2p_dataset::{
    DatasetService, P2pDatasetReference, P2pDatasetRegistration, P2P_DATASET_SCHEMA,
};
use chrono::{DateTime, Utc};
use compute_runner_api::{
    runner::{DomainArtifactContent, DomainArtifactRequest},
    Runner, TaskCtx,
};
use posemesh_compute_node_runner_api as compute_runner_api;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::{fs, io::AsyncReadExt};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    config::{ReconstructionDemoConfig, RobotDemoConfig},
    jobs::{ReconstructionJobSubmitter, RECONSTRUCTION_CAPABILITY, ROBOT_CAPABILITY},
};

pub const REFERENCE_DATA_TYPE: &str = "scan_path_recording_reference_json";
const MAX_REFERENCE_BYTES: usize = 64 * 1024;
const COPY_BUFFER_BYTES: usize = 64 * 1024;

pub struct RobotFilePublisher {
    config: RobotDemoConfig,
    run_id: Uuid,
    dataset: DatasetService,
    submitter: Arc<dyn ReconstructionJobSubmitter>,
}

impl RobotFilePublisher {
    pub fn new(
        config: RobotDemoConfig,
        run_id: Uuid,
        dataset: DatasetService,
        submitter: Arc<dyn ReconstructionJobSubmitter>,
    ) -> Self {
        Self {
            config,
            run_id,
            dataset,
            submitter,
        }
    }
}

#[async_trait]
impl Runner for RobotFilePublisher {
    fn capability(&self) -> &'static str {
        ROBOT_CAPABILITY
    }

    async fn run(&self, ctx: TaskCtx<'_>) -> Result<()> {
        let task_run_id = validate_demo_task(&ctx.lease.task.meta)?;
        if task_run_id != self.run_id {
            bail!("relay demo task run_id does not match this Robot process");
        }
        let domain_id = ctx
            .lease
            .domain_id
            .context("relay demo Robot lease is missing its Domain")?;
        if domain_id != self.config.jobs.domain_id {
            bail!("relay demo Robot lease Domain does not match DOMAIN_ID");
        }
        let source = fs::canonicalize(&self.config.source_file)
            .await
            .with_context(|| {
                format!(
                    "open RELAY_DEMO_SOURCE_FILE {}",
                    self.config.source_file.display()
                )
            })?;
        let metadata = fs::metadata(&source)
            .await
            .with_context(|| format!("stat demo source {}", source.display()))?;
        if !metadata.is_file() || metadata.len() == 0 {
            bail!("RELAY_DEMO_SOURCE_FILE must be a non-empty regular file");
        }

        ctx.ctrl
            .progress(json!({"stage":"register","status":"starting"}))
            .await?;
        let dataset_id = ctx.lease.task.id.to_string();
        let timestamp = Utc::now().format("%Y-%m-%d_%H-%M-%S");
        let name = format!("scan_path_recording_{timestamp}_{}", self.run_id);
        let availability = chrono::Duration::from_std(self.config.availability)
            .context("RELAY_DEMO_AVAILABILITY_SECONDS is out of range")?;
        let available_until = Utc::now() + availability;
        let reference = self
            .dataset
            .register(P2pDatasetRegistration {
                dataset_id: dataset_id.clone(),
                name: name.clone(),
                path: source.clone(),
                available_until,
            })
            .await
            .context("register demo file with authenticated P2P service")?;
        validate_published_reference(&reference, &dataset_id, domain_id, &name, available_until)?;

        let bytes = serde_json::to_vec_pretty(&reference)
            .context("serialize relay demo dataset reference")?;
        write_new_file(&self.config.reference_file, &bytes).await?;

        let rel_path = format!("relay-file-demo/{}/reference.json", self.run_id);
        let artifact_id = ctx
            .output
            .put_domain_artifact(DomainArtifactRequest {
                rel_path: &rel_path,
                name: &name,
                data_type: REFERENCE_DATA_TYPE,
                existing_id: None,
                content: DomainArtifactContent::Bytes(&bytes),
            })
            .await
            .context("upload relay demo reference artifact")?
            .context("Domain Server did not return a reference artifact ID")?;

        ctx.ctrl
            .progress(json!({
                "stage":"publish",
                "status":"reference_uploaded",
                "dataset_id":dataset_id,
                "reference_artifact_id":artifact_id.as_str(),
            }))
            .await?;

        let reconstruction_job_id = self
            .submitter
            .submit_reconstruction(self.run_id, &artifact_id)
            .await
            .context("create relay demo reconstruction job")?;
        info!(
            run_id = %self.run_id,
            task_id = %ctx.lease.task.id,
            domain_id = %domain_id,
            source = %source.display(),
            reference_file = %self.config.reference_file.display(),
            reference_artifact_id = %artifact_id,
            reconstruction_job_id = %reconstruction_job_id,
            peer_id = %reference.peer_id,
            routes = ?reference.multiaddrs,
            bytes = reference.size_bytes,
            sha256 = %reference.sha256,
            available_until = %reference.available_until,
            "relay demo file published and reconstruction job queued"
        );
        ctx.ctrl
            .progress(json!({
                "stage":"publish",
                "status":"finished",
                "reconstruction_job_id":reconstruction_job_id,
            }))
            .await?;
        Ok(())
    }
}

pub struct ReconstructionFileDownloader {
    config: ReconstructionDemoConfig,
    dataset: DatasetService,
}

impl ReconstructionFileDownloader {
    pub fn new(config: ReconstructionDemoConfig, dataset: DatasetService) -> Self {
        Self { config, dataset }
    }
}

#[async_trait]
impl Runner for ReconstructionFileDownloader {
    fn capability(&self) -> &'static str {
        RECONSTRUCTION_CAPABILITY
    }

    async fn run(&self, ctx: TaskCtx<'_>) -> Result<()> {
        let run_id = validate_demo_task(&ctx.lease.task.meta)?;
        let domain_id = ctx
            .lease
            .domain_id
            .context("relay demo reconstruction lease is missing its Domain")?;
        let [reference_cid] = ctx.lease.task.inputs_cids.as_slice() else {
            bail!("relay demo reconstruction task requires exactly one reference artifact");
        };
        if fs::try_exists(&self.config.output_file)
            .await
            .with_context(|| format!("inspect output {}", self.config.output_file.display()))?
        {
            bail!(
                "RELAY_DEMO_OUTPUT_FILE already exists: {}",
                self.config.output_file.display()
            );
        }

        ctx.ctrl
            .progress(json!({"stage":"reference","status":"downloading"}))
            .await?;
        let materialized = ctx
            .input
            .materialize_cid_with_meta(reference_cid)
            .await
            .with_context(|| format!("materialize reference artifact {reference_cid}"))?;
        validate_reference_artifact_metadata(&materialized, domain_id)?;
        let reference_bytes = read_bounded(&materialized.path, MAX_REFERENCE_BYTES)
            .await
            .context("read relay demo reference artifact")?;
        let document: StrictReference = serde_json::from_slice(&reference_bytes)
            .context("parse strict relay demo reference")?;
        let reference = validate_reference_document(document, domain_id, Utc::now())?;

        ctx.ctrl
            .progress(json!({
                "stage":"download",
                "status":"starting",
                "peer_id":reference.peer_id.as_str(),
                "bytes":reference.size_bytes,
            }))
            .await?;
        if let Err(error) = self
            .dataset
            .fetch(&reference, &self.config.output_file)
            .await
        {
            let error = error.context("download relay demo file over authenticated P2P");
            warn!(
                run_id = %run_id,
                task_id = %ctx.lease.task.id,
                peer_id = %reference.peer_id,
                routes = ?reference.multiaddrs,
                error = %format!("{error:#}"),
                "relay demo P2P download failed with full diagnostic chain"
            );
            return Err(error);
        }
        let (size_bytes, sha256) = hash_file(&self.config.output_file).await?;
        if size_bytes != reference.size_bytes || sha256 != reference.sha256 {
            bail!("downloaded relay demo file failed final size/SHA-256 verification");
        }

        info!(
            run_id = %run_id,
            task_id = %ctx.lease.task.id,
            domain_id = %domain_id,
            destination = %self.config.output_file.display(),
            peer_id = %reference.peer_id,
            routes = ?reference.multiaddrs,
            bytes = size_bytes,
            sha256 = %sha256,
            "relay demo download verified successfully"
        );
        ctx.ctrl
            .progress(json!({
                "stage":"download",
                "status":"finished",
                "bytes":size_bytes,
                "sha256":sha256.as_str(),
            }))
            .await?;
        Ok(())
    }
}

fn validate_demo_task(meta: &serde_json::Value) -> Result<Uuid> {
    if meta
        .get("relay_file_demo")
        .and_then(|value| value.as_bool())
        != Some(true)
    {
        bail!("refusing a non-demo task on the relay-file-demo runner");
    }
    let run_id = meta
        .get("run_id")
        .and_then(|value| value.as_str())
        .context("relay demo task is missing run_id")?;
    Uuid::parse_str(run_id).context("relay demo task run_id is invalid")
}

fn validate_published_reference(
    reference: &P2pDatasetReference,
    dataset_id: &str,
    domain_id: Uuid,
    name: &str,
    available_until: DateTime<Utc>,
) -> Result<()> {
    if reference.schema != P2P_DATASET_SCHEMA
        || reference.dataset_id != dataset_id
        || reference.domain_id != domain_id
        || reference.name != name
        || reference.available_until != available_until
    {
        bail!("P2P dataset service returned mismatched relay demo metadata");
    }
    if reference.size_bytes == 0 || !valid_sha256(&reference.sha256) {
        bail!("P2P dataset service returned invalid relay demo integrity metadata");
    }
    if reference.multiaddrs.is_empty()
        || reference
            .multiaddrs
            .iter()
            .any(|route| !route.contains("/p2p-circuit/"))
    {
        bail!("relay demo requires one or more confirmed circuit-only routes");
    }
    Ok(())
}

fn validate_reference_artifact_metadata(
    materialized: &compute_runner_api::MaterializedInput,
    domain_id: Uuid,
) -> Result<()> {
    if materialized.data_type.as_deref() != Some(REFERENCE_DATA_TYPE) {
        bail!("relay demo input has the wrong Domain artifact data type");
    }
    let artifact_domain = materialized
        .domain_id
        .as_deref()
        .context("relay demo reference artifact is missing Domain metadata")?;
    if Uuid::parse_str(artifact_domain).context("reference artifact Domain is invalid")?
        != domain_id
    {
        bail!("relay demo reference artifact belongs to a different Domain");
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictReference {
    schema: String,
    dataset_id: String,
    domain_id: Uuid,
    name: String,
    peer_id: String,
    multiaddrs: Vec<String>,
    size_bytes: u64,
    sha256: String,
    available_until: DateTime<Utc>,
}

fn validate_reference_document(
    document: StrictReference,
    domain_id: Uuid,
    now: DateTime<Utc>,
) -> Result<P2pDatasetReference> {
    if document.schema != P2P_DATASET_SCHEMA {
        bail!("relay demo reference has an unsupported schema");
    }
    let dataset_id =
        Uuid::parse_str(&document.dataset_id).context("relay demo dataset_id must be a UUID")?;
    if dataset_id.to_string() != document.dataset_id {
        bail!("relay demo dataset_id must use canonical UUID form");
    }
    if document.domain_id != domain_id {
        bail!("relay demo reference belongs to a different Domain");
    }
    if !document.name.starts_with("scan_path_recording_") {
        bail!("relay demo reference name is invalid");
    }
    if document.peer_id.trim().is_empty() || document.multiaddrs.is_empty() {
        bail!("relay demo reference has no source Peer ID or route");
    }
    if document
        .multiaddrs
        .iter()
        .any(|route| !route.contains("/p2p-circuit/"))
    {
        bail!("relay demo reference contains a non-circuit route");
    }
    if document.size_bytes == 0 || !valid_sha256(&document.sha256) {
        bail!("relay demo reference integrity metadata is invalid");
    }
    if document.available_until <= now {
        bail!("relay demo reference has expired");
    }

    Ok(P2pDatasetReference {
        schema: document.schema,
        dataset_id: document.dataset_id,
        domain_id: document.domain_id,
        name: document.name,
        peer_id: document.peer_id,
        multiaddrs: document.multiaddrs,
        size_bytes: document.size_bytes,
        sha256: document.sha256,
        available_until: document.available_until,
    })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

async fn read_bounded(path: &Path, maximum: usize) -> Result<Vec<u8>> {
    let metadata = fs::metadata(path)
        .await
        .with_context(|| format!("stat {}", path.display()))?;
    if !metadata.is_file() || metadata.len() == 0 {
        bail!("reference artifact must be a non-empty regular file");
    }
    let maximum_u64 = u64::try_from(maximum).context("reference size limit overflow")?;
    if metadata.len() > maximum_u64 {
        bail!("reference artifact exceeds {maximum} bytes");
    }
    fs::read(path)
        .await
        .with_context(|| format!("read {}", path.display()))
}

async fn write_new_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if fs::try_exists(path)
        .await
        .with_context(|| format!("inspect {}", path.display()))?
    {
        bail!(
            "RELAY_DEMO_REFERENCE_FILE already exists: {}",
            path.display()
        );
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .await
        .with_context(|| format!("create {}", parent.display()))?;
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("RELAY_DEMO_REFERENCE_FILE has no valid filename")?;
    let temporary = parent.join(format!(".{filename}.{}.part", Uuid::new_v4()));
    fs::write(&temporary, bytes)
        .await
        .with_context(|| format!("write {}", temporary.display()))?;
    if let Err(error) = fs::hard_link(&temporary, path).await {
        let _ = fs::remove_file(&temporary).await;
        return Err(error).with_context(|| format!("publish {}", path.display()));
    }
    let _ = fs::remove_file(&temporary).await;
    Ok(())
}

async fn hash_file(path: &Path) -> Result<(u64, String)> {
    let mut file = fs::File::open(path)
        .await
        .with_context(|| format!("open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = vec![0_u8; COPY_BUFFER_BYTES];
    loop {
        let read = file
            .read(&mut buffer)
            .await
            .with_context(|| format!("read {}", path.display()))?;
        if read == 0 {
            break;
        }
        size = size
            .checked_add(u64::try_from(read).context("download size overflow")?)
            .context("download size overflow")?;
        hasher.update(&buffer[..read]);
    }
    Ok((size, hex::encode(hasher.finalize())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_reference_rejects_non_circuit_routes() {
        let domain_id = Uuid::new_v4();
        let document = StrictReference {
            schema: P2P_DATASET_SCHEMA.into(),
            dataset_id: Uuid::new_v4().to_string(),
            domain_id,
            name: "scan_path_recording_test".into(),
            peer_id: "peer".into(),
            multiaddrs: vec!["/ip4/127.0.0.1/tcp/4001".into()],
            size_bytes: 1,
            sha256: "ab".repeat(32),
            available_until: Utc::now() + chrono::Duration::minutes(5),
        };
        assert!(validate_reference_document(document, domain_id, Utc::now()).is_err());
    }

    #[tokio::test]
    async fn bounded_reader_accepts_small_reference_and_rejects_oversize() {
        let temp = tempfile::tempdir().unwrap();
        let small = temp.path().join("small.json");
        fs::write(&small, b"{}").await.unwrap();
        assert_eq!(read_bounded(&small, 2).await.unwrap(), b"{}");
        assert!(read_bounded(&small, 1).await.is_err());
    }
}
