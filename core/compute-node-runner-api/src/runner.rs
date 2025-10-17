use crate::types::LeaseEnvelope;
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;

/// Result of materializing a CID, including discovered metadata from the domain server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializedInput {
    pub cid: String,
    pub path: PathBuf,
    pub data_id: Option<String>,
    pub name: Option<String>,
    pub data_type: Option<String>,
    pub domain_id: Option<String>,
    pub root_dir: PathBuf,
    pub related_files: Vec<PathBuf>,
    pub extracted_paths: Vec<PathBuf>,
}

impl MaterializedInput {
    pub fn new(cid: impl Into<String>, path: PathBuf) -> Self {
        let root_dir = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| path.clone());
        Self {
            cid: cid.into(),
            path,
            data_id: None,
            name: None,
            data_type: None,
            domain_id: None,
            root_dir,
            related_files: Vec::new(),
            extracted_paths: Vec::new(),
        }
    }
}

/// Source of input artifacts for a task.
#[async_trait]
pub trait InputSource: Send + Sync {
    /// Fetch object bytes by CID.
    async fn get_bytes_by_cid(&self, cid: &str) -> Result<Vec<u8>>;

    /// Materialize CID to a temporary file path and return its location.
    async fn materialize_cid_to_temp(&self, cid: &str) -> Result<std::path::PathBuf>;

    /// Materialize CID and include optional metadata describing the source artifact.
    async fn materialize_cid_with_meta(&self, cid: &str) -> Result<MaterializedInput> {
        let path = self.materialize_cid_to_temp(cid).await?;
        Ok(MaterializedInput::new(cid, path))
    }
}

/// Destination for task output artifacts.
#[async_trait]
pub trait ArtifactSink: Send + Sync {
    /// Upload raw bytes under a relative path within the outputs prefix.
    async fn put_bytes(&self, rel_path: &str, bytes: &[u8]) -> Result<()>;

    /// Upload a file from the local filesystem under a relative path.
    async fn put_file(&self, rel_path: &str, file_path: &std::path::Path) -> Result<()>;

    /// Optional: open a multipart upload writer for a large artifact.
    async fn open_multipart(&self, _rel_path: &str) -> Result<Box<dyn MultipartUpload>> {
        Err(anyhow::anyhow!("multipart not supported"))
    }
}

/// Handle returned by `ArtifactSink::open_multipart`.
#[async_trait]
pub trait MultipartUpload: Send + Sync {
    /// Write a chunk of the artifact.
    async fn write_chunk(&mut self, chunk: &[u8]) -> Result<()>;
    /// Finish and commit the artifact.
    async fn finish(self: Box<Self>) -> Result<()>;
}

/// Control-plane interface for a running task.
#[async_trait]
pub trait ControlPlane: Send + Sync {
    /// Returns true if the task has been cancelled.
    async fn is_cancelled(&self) -> bool;

    /// Report progress to the engine; opaque JSON accepted to avoid coupling.
    async fn progress(&self, value: serde_json::Value) -> Result<()>;

    /// Log an event with fields to be attached to heartbeats.
    async fn log_event(&self, fields: serde_json::Value) -> Result<()>;
}

/// Task context passed to runners.
pub struct TaskCtx<'a> {
    pub lease: &'a LeaseEnvelope,
    pub input: &'a dyn InputSource,
    pub output: &'a dyn ArtifactSink,
    pub ctrl: &'a dyn ControlPlane,
}

/// Runner entrypoint.
#[async_trait]
pub trait Runner: Send + Sync {
    /// Capability string this runner implements (e.g., "/reconstruction/legacy/v1").
    fn capability(&self) -> &'static str;

    /// Execute the task.
    async fn run(&self, ctx: TaskCtx<'_>) -> Result<()>;
}
