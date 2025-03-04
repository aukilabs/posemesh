use std::{fs, io::Write};

use crate::protobuf::domain_data::{Data, Metadata, Query};

use super::{common::{DataReader, Datastore, DomainError, ReliableDataProducer}, metadata::{InstantPush, LocalProducer, MetadataStore}};
use async_trait::async_trait;
use futures::{channel::mpsc::channel, StreamExt, SinkExt};
use rs_merkle::{algorithms::Sha256, MerkleTree};
use sha2::{Digest, Sha256 as Sha256Hasher};
use tokio::{spawn, sync::mpsc};
use uuid::Uuid;

struct FsDatastore {
    pub metadata_store: MetadataStore,
}

impl FsDatastore {
    pub async fn new(metadata_store: MetadataStore) -> FsDatastore {
        FsDatastore { metadata_store }
    }

    fn hash_chunk(&self, chunk: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256Hasher::new();
        hasher.update(chunk);
        hasher.finalize().into()
    }

    fn hash_content(&self, content: Vec<u8>) -> String {
        let chunk_size = 1024 * 1024;
        let mut chunks = content.chunks(chunk_size);
        let mut merkle_tree = MerkleTree::<Sha256>::new();
        while let Some(chunk) = chunks.next() {
             merkle_tree.insert(self.hash_chunk(chunk));
        }
        hex::encode(merkle_tree.root().ok_or("Failed to calculate merkle tree root").unwrap())
    }
}

#[async_trait]
impl Datastore for FsDatastore {
    async fn consume(&mut self, domain_id: String, query: Query, keep_alive: bool) -> DataReader {
        let meta_reader = self.metadata_store.consume(domain_id, query, keep_alive).await;
        let (mut writer, reader) = channel::<Result<Data, DomainError>>(240);

        spawn(async move {
            let mut meta_reader = meta_reader;
            while let Some(meta) = meta_reader.next().await {
                match meta {
                    Ok(data) => {
                        let path = data.metadata.link.clone().unwrap();
                        match fs::read(path) {
                            Ok(content) => {
                                let data = Data {
                                    domain_id: data.domain_id,
                                    metadata: data.metadata,
                                    content,
                                };
                                let _ = writer.send(Ok(data)).await;
                            }
                            Err(e) => {
                                let _ = writer.send(Err(DomainError::IoError(e))).await;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = writer.send(Err(e)).await;
                    }
                }
            }
        });

        reader
    }

    async fn produce(&mut self, domain_id: String) -> LocalProducer {
        let meta_writer = self.metadata_store.produce(domain_id).await;
        let (mut writer, reader) = mpsc::channel::<Result<InstantPush, DomainError>>(240);
        
        spawn(async move {
            while let Some(data) = reader.recv().await {
                match data {
                    Ok(mut data) => {
                        let response = data.response;
                        let data = data.data;
                        if data.metadata.id.is_none() {
                            data.metadata.id = Some(Uuid::new_v4().to_string());
                        }
                        data.metadata.hash = Some(self.hash_content(data.content));
                        meta_writer.push(&Data {
                            domain_id: data.domain_id.clone(),
                            metadata: data.metadata.clone(),
                            content: vec![],
                        }).await.unwrap();
                        let path = data.metadata.link.clone().unwrap();
                        match fs::write(path, data.content) {
                            Ok(_) => {
                                let _ = response_writer.send(Ok(data.metadata)).await;
                            }
                            Err(e) => {
                                let _ = response_writer.send(Err(DomainError::IoError(e))).await;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = response_writer.send(Err(e)).await;
                    }
                }
            }
        });

        LocalProducer { writer }
    }
}
