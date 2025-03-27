use std::{fs, io::Write};

use crate::protobuf::domain_data::{Data, Metadata, Query};

use super::{common::{DataReader, Datastore, DomainError, ReliableDataProducer}, metadata::{InstantPush, LocalProducer, MetadataStore}};
use async_trait::async_trait;
use futures::{channel::mpsc::channel, StreamExt, SinkExt};
use rs_merkle::{algorithms::Sha256, MerkleTree};
use sha2::{Digest, Sha256 as Sha256Hasher};
use tokio::spawn;
use uuid::Uuid;

fn hash_chunk(chunk: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256Hasher::new();
    hasher.update(chunk);
    hasher.finalize().into()
}

fn hash_content(content: &[u8]) -> String {
    let chunk_size = 1024 * 1024;
    let mut chunks = content.chunks(chunk_size);
    let mut merkle_tree = MerkleTree::<Sha256>::new();
    while let Some(chunk) = chunks.next() {
         merkle_tree.insert(hash_chunk(chunk));
    }
    hex::encode(merkle_tree.root().ok_or("Failed to calculate merkle tree root").unwrap())
}

#[derive(Clone)]
pub struct FsDatastore {
    metadata_store: MetadataStore,
}

impl FsDatastore {
    pub async fn new(metadata_store: MetadataStore) -> FsDatastore {
        FsDatastore { metadata_store }
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

    async fn produce(&mut self, domain_id: String) -> Box<dyn ReliableDataProducer> {
        let mut meta_writer = self.metadata_store.produce(domain_id).await;
        let (writer, mut reader) = channel::<Result<InstantPush, DomainError>>(240);

        let mut writer_clone = writer.clone();
        spawn(async move {
            while let Some(data) = reader.next().await {
                match data {
                    Ok(data) => {
                        let response = data.response;
                        let mut data = data.data;
                        if data.metadata.id.is_none() {
                            data.metadata.id = Some(Uuid::new_v4().to_string());
                        }
                        data.metadata.hash = Some(hash_content(&data.content));
                        if let Err(e) = meta_writer.push(&Data {
                            domain_id: data.domain_id.clone(),
                            metadata: data.metadata.clone(),
                            content: vec![],
                        }).await {
                            let _ = response.send(Err(e));
                            continue;
                        }
                        let path = data.metadata.link.clone().unwrap();
                        match fs::write(path, data.content) {
                            Ok(_) => {
                                let _ = response.send(Ok(data.metadata));
                            }
                            Err(e) => {
                                let _ = response.send(Err(DomainError::IoError(e)));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = writer_clone.send(Err(e)).await;
                        break;
                    }
                }
            }
        });

        Box::new(LocalProducer { writer })
    }
}
