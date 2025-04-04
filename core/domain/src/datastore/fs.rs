use std::io::Write;

use crate::protobuf::domain_data::{Data, Metadata, Query};
use tokio::{fs::{self, OpenOptions}, io::AsyncWriteExt, sync::oneshot, spawn};
use super::{common::{DataReader, Datastore, DomainData, DomainError, ReliableDataProducer}, metadata::{InstantPush, MetadataProducer, MetadataStore}};
use async_trait::async_trait;
use futures::{channel::mpsc::{channel, Sender}, SinkExt, StreamExt};
use rs_merkle::{algorithms::Sha256, MerkleTree};
use sha2::{Digest, Sha256 as Sha256Hasher};
use uuid::Uuid;

pub struct FsDomainDataProducer {
    writer: Sender<InstantPush>,
    base_path: String,
}

#[async_trait]
impl ReliableDataProducer for FsDomainDataProducer {
    async fn push(&mut self, data: &Metadata) -> Result<Box<dyn DomainData>, DomainError> {
        let path = format!("{}/{}-{}.{}", self.base_path, data.name.clone(), Uuid::new_v4().to_string(), data.data_type.clone());
        let mut metadata = data.clone();
        metadata.link = Some(path.clone());
        Ok(Box::new(FsDomainData::new(metadata, self.writer.clone())))
    }

    async fn is_completed(&self) -> bool {
        true
    }

    async fn close(self) {
        drop(self.writer);
    }
}

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
    let res = merkle_tree.root();
    if res.is_none() {
        panic!("Error calculating merkle tree root");
    }
    hex::encode(res.unwrap())
}

pub struct FsDomainData {
    merkle_tree: MerkleTree<Sha256>,
    metadata: Metadata,
    writer: Sender<InstantPush>,
}
impl FsDomainData {
    pub fn new(metadata: Metadata, writer: Sender<InstantPush>) -> FsDomainData {
        FsDomainData {
            merkle_tree: MerkleTree::<Sha256>::new(),
            metadata,
            writer,
        }
    }
    pub fn root(&self) -> Result<String, DomainError> {
        let res = self.merkle_tree.root();
        if res.is_none() {
            return Err(DomainError::IoError(std::io::Error::new(std::io::ErrorKind::Other, "Failed to calculate merkle tree root")));
        }
        Ok(hex::encode(res.unwrap()))
    }
}

#[async_trait]
impl DomainData for FsDomainData {
    async fn push_chunk(&mut self, chunk: &[u8], more: bool) -> Result<String, DomainError> {
        let hash = hash_chunk(chunk);
        self.merkle_tree.insert(hash);
        let mut f = OpenOptions::new()
            .append(true)
            .create(true)
            .open(self.metadata.link.clone().unwrap()).await.map_err(|e| DomainError::IoError(e))?;
        f.write_all(chunk).await.map_err(|e| DomainError::IoError(e))?;
        if more {
            Ok(hex::encode(hash))
        } else {
            let size = f.metadata().await.map_err(|e| DomainError::IoError(e))?.len();
            if size != self.metadata.size as u64 {
                return Err(DomainError::Cancelled(format!("Unexpected {}.{} size, expected {}, got {}", self.metadata.name, self.metadata.data_type, self.metadata.size, size)));
            }

            let hash = self.root()?;
            let mut metadata = self.metadata.clone();
            metadata.hash = Some(hash);

            let (response, receiver) = oneshot::channel::<Result<Metadata, DomainError>>();
            let push = InstantPush {
                response,
                data: metadata,
            };
            if let Err(e) = self.writer.send(push).await {
                return Err(DomainError::InternalError(Box::new(e)));
            }
            let res = receiver.await.unwrap();
            if let Err(e) = res {
                return Err(e);
            }

            Ok(res.unwrap().link.unwrap())
        }
    }
}


#[derive(Clone)]
pub struct FsDatastore {
    metadata_store: MetadataStore,
    base_path: String,
}

impl FsDatastore {
    pub async fn new(metadata_store: MetadataStore, base_path: String) -> FsDatastore {
        FsDatastore { metadata_store, base_path }
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
                        match fs::read(path).await {
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
        let (writer, mut reader) = channel::<InstantPush>(240);

        spawn(async move {
            while let Some(data) = reader.next().await {
                let response = data.response;
                let mut data = data.data;
                // data.metadata.hash = Some(hash_content(&data.content));
                let res = meta_writer.push(&data).await;
                if let Err(e) = res {
                    if let Err(e) = fs::remove_file(data.link.clone().unwrap()).await {
                        let _ = response.send(Err(DomainError::IoError(e)));
                    } else {
                        let _ = response.send(Err(e));
                    }
                    continue;
                }
                let link = res.unwrap().push_chunk(&vec![], false).await.unwrap();
                data.link = Some(link);
                let _ = response.send(Ok(data));
            }
        });

        Box::new(MetadataProducer { writer })
    }
}
