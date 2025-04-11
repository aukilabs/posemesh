use std::sync::Arc;

use crate::protobuf::domain_data::{Data, Metadata, Query};
use tokio::{fs::{self, OpenOptions}, io::AsyncWriteExt, sync::oneshot, spawn};
use super::{common::{data_id_generator, hash_chunk, DataReader, Datastore, DomainData, DomainError, ReliableDataProducer}, metadata::{InstantPush, MetadataProducer, MetadataStore}};
use async_trait::async_trait;
use futures::{channel::mpsc::{channel, Sender}, lock::Mutex, SinkExt, StreamExt};
use rs_merkle::{algorithms::Sha256, MerkleTree};
use sha2::{Digest, Sha256 as Sha256Hasher};
use uuid::Uuid;

pub struct FsDomainDataProducer {
    metadata_writer: Arc<Mutex<Box<dyn ReliableDataProducer>>>,
    base_path: String,
}

#[async_trait]
impl ReliableDataProducer for FsDomainDataProducer {
    async fn push(&mut self, data: &Metadata) -> Result<Box<dyn DomainData>, DomainError> {
        Ok(Box::new(FsDomainData::new(data, self.metadata_writer.clone(), self.base_path.clone())))
    }

    async fn is_completed(&self) -> bool {
        true
    }

    async fn close(&mut self) {
        let mut metadata_writer = self.metadata_writer.lock().await;
        metadata_writer.close().await;
    }
}

pub(crate) struct FsDomainData {
    merkle_tree: MerkleTree<Sha256>,
    metadata: Metadata,
    metadata_writer: Arc<Mutex<Box<dyn ReliableDataProducer>>>,
    base_path: String,
    temp_path: String,
}

fn data_path_v2(name: &String, data_type: &String, hash: &String, base_path: &String) -> String {
    format!("{}/{}-{}.{}", base_path, name, data_type, hash)
}

// v1 used in old version of domain server
fn data_path_v1(id: &String, base_path: &String) -> String {
    format!("{}/{}.{}", base_path, id, data_id_generator())
}

pub fn from_path_to_hash(path: &String) -> Result<&str, DomainError> {
    Ok(path.split('.').last().unwrap())
}

impl FsDomainData {
    pub fn new(metadata: &Metadata, metadata_writer: Arc<Mutex<Box<dyn ReliableDataProducer>>>, base_path: String) -> FsDomainData {
        let temp_hash = data_id_generator();
        let temp_path = {
            if metadata.link.is_some() {
                metadata.link.clone().unwrap()
            } else {
                data_path_v2(&metadata.name, &metadata.data_type, &temp_hash, &base_path)
            }
        };
        FsDomainData {
            merkle_tree: MerkleTree::<Sha256>::new(),
            metadata: metadata.clone(),
            metadata_writer: metadata_writer.clone(),
            base_path,
            temp_path,
        }
    }
    pub fn root(&mut self) -> Result<String, DomainError> {
        self.merkle_tree.commit();
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
            .open(self.temp_path.clone()).await.map_err(|e| DomainError::IoError(e))?;
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
            metadata.hash = Some(hash.clone());
            let path = data_path_v2(&metadata.name, &metadata.data_type, &hash, &self.base_path);
            fs::rename(self.temp_path.clone(), path.clone()).await.map_err(|e| DomainError::IoError(e))?;
            metadata.link = Some(path.clone());

            let mut metadata_writer = self.metadata_writer.lock().await;
            let mut chunk = metadata_writer.push(&metadata).await?;
            drop(metadata_writer);
            chunk.push_chunk(&[], false).await?;
            Ok(hash)
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
        std::fs::create_dir_all(base_path.clone()).expect(&format!("Failed to create base path: {}", base_path));
        FsDatastore { metadata_store, base_path }
    }
}

#[async_trait]
impl Datastore for FsDatastore {
    async fn load(&mut self, domain_id: String, query: Query, keep_alive: bool) -> DataReader {
        let meta_reader = self.metadata_store.load(domain_id, query, keep_alive).await;
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

    async fn upsert(&mut self, domain_id: String) -> Box<dyn ReliableDataProducer> {
        let _ = Uuid::parse_str(&domain_id).expect("Failed to parse domain id"); // TODO: handle error
        let path = format!("{}/{}", self.base_path, domain_id);
        fs::create_dir_all(path.clone()).await.expect(&format!("Failed to create domain data path: {}", path));
        let meta_writer = self.metadata_store.upsert(domain_id).await;

        Box::new(FsDomainDataProducer { metadata_writer: Arc::new(Mutex::new(meta_writer)), base_path: path.clone() })
    }
}
