//! Storage module: TokenRef, client, and InputSource/ArtifactSink wrappers.

use anyhow::{anyhow, Result};
use compute_runner_api::LeaseEnvelope;
use parking_lot::Mutex;
use std::{collections::HashMap, sync::Arc};

pub mod client;
pub mod input;
pub mod output;
mod sdk;
pub mod token;

use output::{DomainOutput, UploadedArtifact};
pub use token::TokenRef;

/// Pair of input/output ports backed by the domain client.
pub struct Ports {
    pub input: Box<dyn compute_runner_api::InputSource>,
    pub output: Box<dyn compute_runner_api::ArtifactSink>,
    uploads: Arc<Mutex<HashMap<String, UploadedArtifact>>>,
    client: client::DomainClient,
}

impl Ports {
    pub fn uploaded_artifacts(&self) -> Vec<UploadedArtifact> {
        let guard = self.uploads.lock();
        guard.values().cloned().collect()
    }

    pub(crate) async fn close(&self) {
        self.client.close().await;
    }
}
/// Build storage ports from a lease and a TokenRef.
pub fn build_ports(lease: &LeaseEnvelope, token: TokenRef) -> Result<Ports> {
    let base = lease
        .domain_server_url
        .clone()
        .ok_or_else(|| anyhow!("lease missing domain_server_url"))?;
    let client = client::DomainClient::new(base, token)?;
    build_ports_with_client(lease, client)
}

pub(crate) fn build_task_ports(task: &auki_sdk::TaskContext) -> Result<Ports> {
    build_ports_with_client(
        &task.credential.lease_snapshot()?,
        client::DomainClient::from_task(task)?,
    )
}

fn build_ports_with_client(lease: &LeaseEnvelope, client: client::DomainClient) -> Result<Ports> {
    let outputs_prefix = lease.task.outputs_prefix.clone();
    if lease.task.outputs_prefix.is_none() {
        tracing::debug!(
            task_id = %lease.task.id,
            "Lease missing outputs_prefix; defaulting to empty prefix"
        );
    }
    let domain_id = lease
        .domain_id
        .map(|id| id.to_string())
        .ok_or_else(|| anyhow!("lease missing domain_id"))?;
    let task_id = lease.task.id.to_string();

    let uploads = Arc::new(Mutex::new(HashMap::new()));
    let output = DomainOutput::with_store(
        client.clone(),
        domain_id.clone(),
        outputs_prefix,
        task_id,
        Arc::clone(&uploads),
    );
    Ok(Ports {
        input: Box::new(input::DomainInput::new(client.clone(), domain_id)),
        output: Box::new(output),
        uploads,
        client,
    })
}
