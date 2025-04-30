use crate::protobuf::domain_data::{Data, Metadata, Query, UpsertMetadata as ProtoUpsertMetadata};
use tokio::{fs::{self, OpenOptions}, io::{AsyncReadExt, AsyncWriteExt}, spawn};
use super::{common::{data_id_generator, hash_chunk, DataReader, Datastore, DomainData, DomainError, ReliableDataProducer, CHUNK_SIZE}, metadata::{MetadataStore, UpsertMetadata}};
use async_trait::async_trait;
use futures::{channel::mpsc::channel, SinkExt, StreamExt};
use rs_merkle::{algorithms::Sha256, MerkleTree};
use uuid::Uuid;

pub struct FsDomainDataProducer {
    metadata_store: MetadataStore,
    base_path: String,
    domain_id: String,
}

#[async_trait]
impl ReliableDataProducer for FsDomainDataProducer {
    async fn push(&mut self, data: &ProtoUpsertMetadata) -> Result<Box<dyn DomainData>, DomainError> {
        Ok(Box::new(FsDomainData::new(data, self.metadata_store.clone(), self.base_path.clone(), self.domain_id.clone())))
    }

    async fn is_completed(&self) -> bool {
        true
    }

    async fn close(&mut self) {
        // do nothing
    }
}

pub(crate) struct FsDomainData {
    merkle_tree: MerkleTree<Sha256>,
    metadata: ProtoUpsertMetadata,
    metadata_store: MetadataStore,
    base_path: String,
    temp_path: String,
    domain_id: String,
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
    pub fn new(metadata: &ProtoUpsertMetadata, metadata_store: MetadataStore, base_path: String, domain_id: String) -> FsDomainData {
        let temp_hash = data_id_generator();
        let temp_path = data_path_v2(&metadata.name, &metadata.data_type, &temp_hash, &base_path);
        FsDomainData {
            merkle_tree: MerkleTree::<Sha256>::new(),
            metadata: metadata.clone(),
            metadata_store,
            base_path,
            temp_path,
            domain_id,
        }
    }
    pub fn root(&mut self) -> Result<String, DomainError> {
        self.merkle_tree.commit();
        let res = self.merkle_tree.root();
        if res.is_none() {
            return Err(DomainError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Failed to calculate merkle tree root")));
        }
        Ok(hex::encode(res.unwrap()))
    }
}

#[async_trait]
impl DomainData for FsDomainData {
    async fn next_chunk(&mut self, chunk: &[u8], more: bool) -> Result<String, DomainError> {
        let mut f = OpenOptions::new()
            .append(true)
            .create(true)
            .open(self.temp_path.clone()).await?;
        f.write_all(chunk).await?;
        if more {
            Ok("".to_string())
        } else {
            f.flush().await?;
            let size = f.metadata().await?.len();
            if size != self.metadata.size as u64 {
                return Err(DomainError::SizeMismatch(self.metadata.size as usize, size as usize));
            }

            let mut buffer = vec![0; CHUNK_SIZE];
            while let Ok(n) = f.read(&mut buffer).await {
                if n == 0 {
                    break;
                }
                let hash = hash_chunk(&buffer[..n]);
                self.merkle_tree.insert(hash);
            }

            let hash = self.root()?;
            let path = data_path_v2(&self.metadata.name, &self.metadata.data_type, &hash, &self.base_path);
            fs::rename(self.temp_path.clone(), path.clone()).await?;

            let _ = self.metadata_store.upsert(self.domain_id.clone(), UpsertMetadata {
                name: self.metadata.name.clone(),
                data_type: self.metadata.data_type.clone(),
                size: self.metadata.size,
                id: self.metadata.id.clone().unwrap_or_else(|| data_id_generator()),
                properties: self.metadata.properties.clone(),
                is_new: self.metadata.is_new,
                link: path.clone(),
                hash: hash.clone(),
            }).await?;
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
    async fn load(&mut self, domain_id: String, query: Query, keep_alive: bool) -> Result<DataReader, DomainError> {
        let meta_reader = self.metadata_store.load(domain_id.clone(), query, keep_alive).await?;
        let (mut writer, reader) = channel::<Result<Data, DomainError>>(240);
        let domain_id = domain_id.clone();
        spawn(async move {
            let mut meta_reader = meta_reader;
            while let Some(meta) = meta_reader.next().await {
                match meta {
                    Ok(data) => {
                        let path = data.link.clone();
                        match fs::read(path).await {
                            Ok(content) => {
                                let data = Data {
                                    domain_id: domain_id.clone(),
                                    metadata: Metadata {
                                        id: data.id.clone(),
                                        name: data.name.clone(),
                                        data_type: data.data_type.clone(),
                                        size: data.size,
                                        properties: data.properties.clone(),
                                        hash: Some(data.hash.clone()),
                                    },
                                    content,
                                };
                                let _ = writer.send(Ok(data)).await;
                            }
                            Err(e) => {
                                let _ = writer.send(Err(DomainError::Io(e))).await;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = writer.send(Err(e)).await;
                    }
                }
            }
        });

        Ok(reader)
    }

    async fn upsert(&mut self, domain_id: String) -> Result<Box<dyn ReliableDataProducer>, DomainError> {
        let _ = Uuid::parse_str(&domain_id.clone()).map_err(|e| DomainError::Invalid("domain_id".to_string(), domain_id.clone(), e.to_string()))?;
        let path = format!("{}/{}", self.base_path, domain_id);
        fs::create_dir_all(path.clone()).await?;

        Ok(Box::new(FsDomainDataProducer { metadata_store: self.metadata_store.clone(), base_path: path.clone(), domain_id }))
    }
}
