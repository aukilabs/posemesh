use super::client::{DomainClient, UploadRequest};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;

use compute_runner_api::runner::{DomainArtifactContent, DomainArtifactRequest};
#[derive(Clone, Debug)]
struct UploadDescriptor {
    logical_path: String,
    name: String,
    data_type: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct UploadedArtifact {
    pub logical_path: String,
    pub name: String,
    pub data_type: String,
    pub id: Option<String>,
}

/// Domain ArtifactSink implementation (skeleton).
#[derive(Clone)]
pub struct DomainOutput {
    client: DomainClient,
    domain_id: String,
    outputs_prefix: Option<String>,
    task_id: String,
    uploads: Arc<Mutex<HashMap<String, UploadedArtifact>>>,
}

impl DomainOutput {
    pub fn new(
        client: DomainClient,
        domain_id: String,
        outputs_prefix: Option<String>,
        task_id: String,
    ) -> Self {
        Self::with_store(
            client,
            domain_id,
            outputs_prefix,
            task_id,
            Arc::new(Mutex::new(HashMap::new())),
        )
    }

    pub fn with_store(
        client: DomainClient,
        domain_id: String,
        outputs_prefix: Option<String>,
        task_id: String,
        uploads: Arc<Mutex<HashMap<String, UploadedArtifact>>>,
    ) -> Self {
        Self {
            client,
            domain_id,
            outputs_prefix,
            task_id,
            uploads,
        }
    }

    fn name_suffix(&self) -> String {
        self.task_id.clone()
    }

    fn apply_outputs_prefix(&self, rel_path: &str) -> String {
        let trimmed_rel = rel_path.trim_start_matches('/');
        match self
            .outputs_prefix
            .as_ref()
            .map(|p| p.trim_matches('/'))
            .filter(|p| !p.is_empty())
        {
            Some(prefix) if trimmed_rel.is_empty() => prefix.to_string(),
            Some(prefix) => format!("{}/{}", prefix, trimmed_rel),
            None => trimmed_rel.to_string(),
        }
    }

    fn descriptor_for(&self, rel_path: &str) -> UploadDescriptor {
        let logical_path = self.apply_outputs_prefix(rel_path);
        let sanitized = sanitize_component(&logical_path.replace('/', "_"));
        let data_type = infer_data_type(rel_path);
        UploadDescriptor {
            logical_path,
            name: format!("{}_{}", sanitized, self.name_suffix()),
            data_type,
        }
    }
}

#[async_trait]
impl compute_runner_api::ArtifactSink for DomainOutput {
    async fn put_bytes(&self, rel_path: &str, bytes: &[u8]) -> Result<()> {
        let descriptor = self.descriptor_for(rel_path);
        self.put_domain_artifact(DomainArtifactRequest {
            rel_path,
            name: &descriptor.name,
            data_type: &descriptor.data_type,
            existing_id: None,
            content: DomainArtifactContent::Bytes(bytes),
        })
        .await
        .map(|_| ())
    }

    async fn put_file(&self, rel_path: &str, file_path: &std::path::Path) -> Result<()> {
        let descriptor = self.descriptor_for(rel_path);
        self.put_domain_artifact(DomainArtifactRequest {
            rel_path,
            name: &descriptor.name,
            data_type: &descriptor.data_type,
            existing_id: None,
            content: DomainArtifactContent::File(file_path),
        })
        .await
        .map(|_| ())
    }

    async fn open_multipart(
        &self,
        _rel_path: &str,
    ) -> Result<Box<dyn compute_runner_api::runner::MultipartUpload>> {
        // Implemented in later prompt.
        unimplemented!("multipart not implemented yet")
    }

    async fn put_domain_artifact(
        &self,
        request: DomainArtifactRequest<'_>,
    ) -> Result<Option<String>> {
        let logical_path = self.apply_outputs_prefix(request.rel_path);
        let key = logical_path.clone();
        let mut existing_id = request.existing_id.map(|s| s.to_string());
        if existing_id.is_none() {
            let uploads = self.uploads.lock();
            existing_id = uploads.get(&key).and_then(|record| record.id.clone());
        }
        if existing_id.is_none() {
            existing_id = self
                .client
                .find_artifact_id(&self.domain_id, request.name, request.data_type)
                .await
                .map_err(|e| anyhow!(e))?;
        }

        let bytes_owned;
        let bytes = match request.content {
            DomainArtifactContent::Bytes(b) => b,
            DomainArtifactContent::File(path) => {
                bytes_owned = fs::read(path).await?;
                bytes_owned.as_slice()
            }
        };

        let upload_req = UploadRequest {
            domain_id: &self.domain_id,
            name: request.name,
            data_type: request.data_type,
            logical_path: &logical_path,
            bytes,
            existing_id: existing_id.as_deref(),
        };

        let maybe_id = self
            .client
            .upload_artifact(upload_req)
            .await
            .map_err(|e| anyhow!(e))?;
        let final_id = maybe_id.or(existing_id);

        let mut uploads = self.uploads.lock();
        uploads.insert(
            key,
            UploadedArtifact {
                logical_path,
                name: request.name.to_string(),
                data_type: request.data_type.to_string(),
                id: final_id.clone(),
            },
        );

        Ok(final_id)
    }
}

impl DomainOutput {
    pub fn uploaded_artifacts(&self) -> Vec<UploadedArtifact> {
        let guard = self.uploads.lock();
        guard.values().cloned().collect()
    }

    pub fn seed_uploaded_artifact(&self, rel_path: &str, id: impl Into<String>) {
        let descriptor = self.descriptor_for(rel_path);
        let mut uploads = self.uploads.lock();
        uploads.insert(
            descriptor.logical_path.clone(),
            UploadedArtifact {
                logical_path: descriptor.logical_path,
                name: descriptor.name,
                data_type: descriptor.data_type,
                id: Some(id.into()),
            },
        );
    }
}

fn infer_data_type(rel_path: &str) -> String {
    let ext = Path::new(rel_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.trim().to_ascii_lowercase());
    match ext.as_deref() {
        Some("json") => "json".into(),
        Some("ply") => "ply".into(),
        Some("drc") => "ply_draco".into(),
        Some("glb") => "glb".into(),
        Some("obj") => "obj".into(),
        Some("csv") => "csv".into(),
        Some("mp4") => "mp4".into(),
        Some(other) => format!("{}_data", sanitize_component(other)),
        None => "binary".into(),
    }
}

fn sanitize_component(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "artifact".into()
    } else {
        sanitized
    }
}
