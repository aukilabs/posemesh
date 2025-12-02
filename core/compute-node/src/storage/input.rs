use super::client::DomainClient;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use tokio::fs;

/// Domain InputSource implementation (skeleton).
#[derive(Clone)]
pub struct DomainInput {
    client: DomainClient,
}
impl DomainInput {
    pub fn new(client: DomainClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl compute_runner_api::InputSource for DomainInput {
    async fn get_bytes_by_cid(&self, cid: &str) -> Result<Vec<u8>> {
        let materialized = self.materialize_cid_with_meta(cid).await?;
        let source_path = materialized
            .extracted_paths
            .first()
            .cloned()
            .unwrap_or_else(|| materialized.path.clone());
        let bytes = fs::read(&source_path)
            .await
            .with_context(|| format!("read domain download {}", source_path.display()))?;
        Ok(bytes)
    }

    async fn materialize_cid_to_temp(&self, cid: &str) -> Result<std::path::PathBuf> {
        let materialized = self.materialize_cid_with_meta(cid).await?;
        Ok(materialized.path)
    }

    async fn materialize_cid_with_meta(
        &self,
        cid: &str,
    ) -> Result<compute_runner_api::MaterializedInput> {
        let mut parts = self
            .client
            .download_uri(cid)
            .await
            .map_err(|e| anyhow!(e))?;
        if parts.is_empty() {
            return Err(anyhow!("domain response missing data for {}", cid));
        }
        // Choose the first part as primary. Runners can interpret
        // additional parts or data_type as needed.
        let primary = parts.remove(0);
        let related_files = parts.into_iter().map(|p| p.path).collect();

        Ok(compute_runner_api::MaterializedInput {
            cid: cid.to_string(),
            path: primary.path,
            data_id: primary.id,
            name: primary.name,
            data_type: primary.data_type,
            domain_id: primary.domain_id,
            root_dir: primary.root,
            related_files,
            extracted_paths: primary.extracted_paths,
        })
    }
}
