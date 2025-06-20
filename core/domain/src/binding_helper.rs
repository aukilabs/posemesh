use std::{io::{Error, ErrorKind}, pin::Pin, task::{Context, Poll}};

use async_trait::async_trait;
use futures::{channel::mpsc, executor::block_on, AsyncWrite, SinkExt};
use quick_protobuf::deserialize_from_slice;

#[cfg(not(target_family = "wasm"))]
use tokio::task::spawn;
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;


use crate::{cluster::{join_domain, DomainCluster, PosemeshSwarm}, datastore::{common::{self, DomainError}, remote::RemoteDatastore}, protobuf::domain_data};

pub(crate) fn init_r_remote_storage(cluster: *mut DomainCluster) -> RemoteDatastore {
    unsafe {
        // Ensure the pointers are not null
        assert!(!cluster.is_null(), "init_r_remote_storage(): cluster is null");

        // Copy the values without consuming the pointers
        let cluster_copy = (*cluster).clone();

        RemoteDatastore::new(cluster_copy)
    }
}

pub(crate) async fn init_r_domain_cluster(domain_manager_addr: String, name: String, private_key: Option<String>, private_key_path: Option<String>, relay_nodes: Vec<String>) -> Result<DomainCluster, DomainError> {
    let mut swarm = PosemeshSwarm::init(false, 0, false, false, private_key, private_key_path, relay_nodes).await?;
    join_domain(&mut swarm, &domain_manager_addr, &name).await
}
pub struct DomainDataWriter {
    metadata: Option<domain_data::Metadata>,
    content: Option<Vec<u8>>,
    domain_id: String,
    metadata_only: bool,
    sender: mpsc::Sender<domain_data::Data>,
}

impl DomainDataWriter {
    pub fn new(domain_id: String, metadata_only: bool, sender: mpsc::Sender<domain_data::Data>) -> Self {
        Self { metadata: None, content: None, domain_id, metadata_only, sender }
    }
}

#[async_trait]
impl AsyncWrite for DomainDataWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        content: &[u8],
    ) -> Poll<Result<usize, Error>> {
        if self.metadata.is_none() {
            // Read only the first 4 bytes (size prefix)
            if content.len() < 4 {
                return Poll::Ready(Err(Error::new(ErrorKind::UnexpectedEof, "Incomplete metadata")));
            }

            match deserialize_from_slice::<domain_data::Metadata>(&content[4..]) {
                Ok(metadata) => {
                    self.metadata = Some(metadata);
                }
                Err(e) => {
                    tracing::error!("Failed to deserialize metadata: {}", e);
                    return Poll::Ready(Err(Error::new(ErrorKind::Other, "Failed to deserialize metadata")));
                }
            }
        } else if !self.metadata_only {
            self.content
                .get_or_insert_with(Vec::new)
                .extend_from_slice(content);
        }

        let mut done = false;
        if let Some(ref metadata) = self.metadata {
            let current_len = self.content.as_ref().map(|c| c.len()).unwrap_or(0);
            if self.metadata_only || current_len == metadata.size as usize {
                done = true;
            }
            if current_len > metadata.size as usize {
                tracing::error!("content length {} is greater than metadata size {}", current_len, metadata.size);
                return Poll::Ready(Err(Error::new(ErrorKind::Other, "Content length is greater than metadata size")));
            }
        }

        if done {
            let metadata = self.metadata.take().unwrap();
            let data = domain_data::Data {
                domain_id: self.domain_id.clone(),
                metadata: metadata.clone(),
                content: self.content.take().unwrap_or_default(),
            };
            let mut sender = self.sender.clone();

            // spawn async send
            spawn(async move {
                if let Err(e) = sender.send(data).await {
                    tracing::error!("Failed to send data: {}", e);
                }
            });
        }

        Poll::Ready(Ok(content.len()))
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Result<(), Error>> {
        tracing::info!("closing DomainDataWriter");
        self.metadata = None;
        self.content = None;
        let mut sender = self.sender.clone();
        block_on(async move {
            if let Err(e) = sender.close().await {
                tracing::error!("Failed to send data: {}", e);
            }
        });
        Poll::Ready(Ok(()))
    }
}

#[cfg_attr(target_family = "wasm", wasm_bindgen)]
pub struct DataConsumer {
    inner: Box<dyn common::DataConsumer>,
}

#[cfg_attr(target_family = "wasm", wasm_bindgen)]
impl DataConsumer {
    #[cfg_attr(target_family = "wasm", wasm_bindgen)]
    pub fn close(&mut self) {
        block_on(async move {
            self.inner.close().await;
        });
    }
}

impl DataConsumer {
    pub fn new(inner: Box<dyn common::DataConsumer>) -> Self {
        Self { inner }
    }
}

pub async fn initialize_consumer(mut store: impl common::Datastore, domain_id: String, query: domain_data::Query, keep_alive: bool) -> Result<(Box<dyn common::DataConsumer>, mpsc::Receiver<domain_data::Data>), DomainError> {
    let (tx, rx) = mpsc::channel::<domain_data::Data>(100);
    let writer = DomainDataWriter::new(domain_id.clone(), query.metadata_only, tx);
    let consumer = store.load(domain_id.clone(), query, keep_alive, writer).await?;
    
    Ok((consumer, rx))
}
