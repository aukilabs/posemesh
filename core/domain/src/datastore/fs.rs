use std::io::SeekFrom;

use crate::{message::prefix_size_message, protobuf::domain_data::{Metadata, Query, UpsertMetadata as ProtoUpsertMetadata}};
use tokio::{fs::{self, File, OpenOptions}, io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt as TokioWriteExt}, select, spawn, sync::{oneshot, watch}};
use super::{common::{data_id_generator, hash_chunk, DataConsumer, Datastore, DomainData, DomainError, ReliableDataProducer, CHUNK_SIZE}, metadata::{MetadataReader, UpsertMetadata, MetadataStore }};
use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt, StreamExt};
use rs_merkle::{algorithms::Sha256, MerkleTree};
use uuid::Uuid;

pub struct FsDomainDataProducer<M: MetadataStore + Clone + 'static> {
    metadata_store: M,
    base_path: String,
    domain_id: String,
}

#[async_trait]
impl<M: MetadataStore + Clone + 'static> ReliableDataProducer for FsDomainDataProducer<M> {
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

pub(crate) struct FsDomainData<M: MetadataStore> {
    merkle_tree: MerkleTree<Sha256>,
    metadata: ProtoUpsertMetadata,
    metadata_store: M,
    base_path: String,
    temp_path: String,
    domain_id: String
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

impl<M: MetadataStore> FsDomainData<M> {
    pub fn new(metadata: &ProtoUpsertMetadata, metadata_store: M, base_path: String, domain_id: String) -> FsDomainData<M> {
        let temp_hash = data_id_generator();
        let temp_path = data_path_v2(&metadata.name, &metadata.data_type, &temp_hash, &base_path);
        FsDomainData {
            merkle_tree: MerkleTree::<Sha256>::new(),
            metadata: metadata.clone(),
            metadata_store,
            base_path,
            temp_path,
            domain_id
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
    async fn create_file(&mut self) -> Result<File, DomainError> {
        let f = OpenOptions::new()
            .append(true)
            .create(true)
            .read(true)
            .open(self.temp_path.clone()).await?;
        Ok(f)
    }
}

#[async_trait]
impl<M: MetadataStore> DomainData for FsDomainData<M> {
    async fn next_chunk(&mut self, chunk: &[u8], more: bool) -> Result<String, DomainError> {
        let mut f = self.create_file().await?;
        f.write_all(chunk).await?;
        if more {
            Ok("".to_string())
        } else {
            f.flush().await?;
            let size = f.metadata().await?.len();
            if size != self.metadata.size as u64 {
                return Err(DomainError::SizeMismatch(self.metadata.size as usize, size as usize));
            }

            f.seek(SeekFrom::Start(0)).await?;
            let mut buffer = vec![0; CHUNK_SIZE];
            let mut read_size = f.read(&mut buffer).await?;
            while read_size > 0 {
                let hash = hash_chunk(&buffer[..read_size]);
                self.merkle_tree.insert(hash);
                if read_size < CHUNK_SIZE {
                    f.shutdown().await?;
                    break;
                }
                read_size = f.read(&mut buffer).await?;
            }

            let hash = self.root()?;
            let path = data_path_v2(&self.metadata.name, &self.metadata.data_type, &hash, &self.base_path);
            fs::rename(self.temp_path.clone(), path.clone()).await?;

            let _ = self.metadata_store.upsert(self.domain_id.clone(), UpsertMetadata {
                name: self.metadata.name.clone(),
                data_type: self.metadata.data_type.clone(),
                size: self.metadata.size,
                id: self.metadata.id.clone(),
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
pub struct FsDatastore<M: MetadataStore + Clone + 'static> {
    metadata_store: M,
    base_path: String,
}

impl<M: MetadataStore + Clone + 'static> FsDatastore<M> {
    pub async fn new(metadata_store: M, base_path: String) -> FsDatastore<M> {
        std::fs::create_dir_all(base_path.clone()).expect(&format!("Failed to create base path: {}", base_path));
        FsDatastore { metadata_store, base_path }
    }
}

pub struct FsDataConsumer {
    close_signal_tx: Option<watch::Sender<()>>,
    executor_rx: Option<oneshot::Receiver<Result<(), DomainError>>>
}


#[async_trait]
impl DataConsumer for FsDataConsumer {
    async fn close(&mut self) {
        if let Some(tx) = self.close_signal_tx.take() {
            let _ = tx.send(());
        }
        if let Some(mut receiver) = self.executor_rx.take() {
            let _ = receiver.close();
        }
    }
    async fn wait_for_done(&mut self) -> Result<(), DomainError> {
        if let Some(receiver) = self.executor_rx.take() {
            return receiver.await.map_err(|e| DomainError::InternalError(Box::new(e)))?;
        } else {
            Err(DomainError::Io(std::io::Error::new(std::io::ErrorKind::Other, "channel closed")))
        }
    }
}

async fn write_to_downstream<M: MetadataStore + Clone + 'static>(
    mut meta_reader: MetadataReader,
    mut writer: impl AsyncWrite + Unpin,
    metadata_only: bool,
    mut rx: watch::Receiver<()>,
    mut metadata_store: M,
    domain_id: String,
) -> Result<(), DomainError> {
    loop {
        select! {
            result = meta_reader.reader.next() => {
                match result {
                    Some(Ok(metadata)) => {
                        writer.write_all(&prefix_size_message(&Metadata {
                            id: metadata.id.clone(),
                            name: metadata.name.clone(),
                            data_type: metadata.data_type.clone(),
                            size: metadata.size,
                            properties: metadata.properties.clone(),
                            hash: Some(metadata.hash.clone()),
                        })).await?;
                        writer.flush().await?;
                
                        if !metadata_only {
                            let path = metadata.link.clone();
                            let mut file = File::open(path).await?;
                            let mut written_size = 0;
                            let mut buffer = vec![0; 8*1024*1024];
                            let mut read_size = file.read(&mut buffer).await?;
                            while read_size > 0 {
                                writer.write_all(&buffer[..read_size]).await?;
                                writer.flush().await?;
                                written_size += read_size;
                                read_size = file.read(&mut buffer).await?;
                            }
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!("Failed to send error to reader: {}", e);
                        metadata_store.close_reader(domain_id.clone(), meta_reader).await;
                        writer.close().await?;
                        return Err(e);
                    }
                    None => {
                        tracing::info!("no more metadata");
                        break;
                    }
                }
            }
            _ = rx.changed() => {
                tracing::debug!("close signal received");
                break;
            }
        }
    }


    metadata_store.close_reader(domain_id.clone(), meta_reader).await;
    writer.close().await?;
    Ok(())
}

#[async_trait]
impl<M: MetadataStore + Clone + 'static> Datastore for FsDatastore<M> {
    async fn load<W: AsyncWrite + Unpin + Send + 'static>(&mut self, domain_id: String, query: Query, keep_alive: bool, writer: W) -> Result<Box<dyn DataConsumer>, DomainError> {
        let metadata_only = query.metadata_only;
        let meta_reader = self.metadata_store.load(domain_id.clone(), query, keep_alive).await?;
        let domain_id = domain_id.clone();
        let (close_signal_tx, close_signal_rx) = watch::channel(());
        let (done_signal_tx, done_signal_rx) = oneshot::channel::<Result<(), DomainError>>();
        let metadata_store = self.metadata_store.clone();
        spawn(async move {
            let res = write_to_downstream(meta_reader, writer, metadata_only, close_signal_rx, metadata_store, domain_id).await;
            if let Err(_) = done_signal_tx.send(res) {
                tracing::error!("Failed to send response to executor");
            }
        });
        Ok(Box::new(FsDataConsumer { close_signal_tx: Some(close_signal_tx), executor_rx: Some(done_signal_rx) }))
    }

    async fn upsert(&mut self, domain_id: String) -> Result<Box<dyn ReliableDataProducer>, DomainError> {
        let _ = Uuid::parse_str(&domain_id.clone()).map_err(|e| DomainError::Invalid("domain_id".to_string(), domain_id.clone(), e.to_string()))?;
        let path = format!("{}/{}", self.base_path, domain_id);
        fs::create_dir_all(path.clone()).await?;

        Ok(Box::new(FsDomainDataProducer { metadata_store: self.metadata_store.clone(), base_path: path.clone(), domain_id }))
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, pin::Pin, sync::{Arc, Mutex}, task::{Context, Poll}, time::Duration};
    use futures::channel::mpsc::channel;
    use mockall::predicate::eq;
    use tokio::time::sleep;
    use crate::{datastore::metadata::{MockMetadataStore, Metadata as r_Metadata}, protobuf::domain_data::Query};
    use super::*;

    impl Clone for MockMetadataStore {
        fn clone(&self) -> Self {
            let mut mock = MockMetadataStore::new();

            mock.expect_close_reader().withf(|_: &String, reader: &MetadataReader| {
                reader.id.is_none()
            }).return_once(|_, _| {
                // do nothing
            });

            mock
        }
    }

    #[tokio::test]
    async fn test_fs_datastore_success_load() {
        let mut mock_metadata_store = MockMetadataStore::new();

        let domain_id = "test-domain".to_string();
        let mut query = Query::default();
        query.metadata_only = true;
        let keep_alive = false;

        mock_metadata_store.expect_load().with(eq(domain_id.clone()), eq(query.clone()), eq(keep_alive)).returning(|_domain_id, _query, _keep_alive| {
            let (mut tx, rx) = channel::<Result<r_Metadata, DomainError>>(100);
            let metadata = vec![
                r_Metadata {
                    id: "test-id-1".to_string(),
                    name: "test1.txt".to_string(),
                    data_type: "text/plain".to_string(),
                    size: 10,
                    properties: HashMap::new(),
                    hash: "test-hash-1".to_string(),
                    link: "test-link-1".to_string(),
                },
            ];
            for metadata in metadata.iter() {
                tx.try_send(Ok(metadata.clone())).unwrap();
            }
            let reader = MetadataReader {
                reader: rx,
                id: None
            };
            Ok(reader)
        });

        let mut datastore = FsDatastore {
            metadata_store: mock_metadata_store,
            base_path: "test-base-path".to_string(),
        };

        let mut consumer = datastore.load(domain_id.clone(), query, false, Vec::new()).await.unwrap();

        consumer.wait_for_done().await.expect("Failed to wait for done");
    }

    #[tokio::test]
    async fn test_fs_datastore_failed_load() {
        let mut mock_metadata_store = MockMetadataStore::new();

        let domain_id = "test-domain".to_string();
        let mut query = Query::default();
        query.metadata_only = true;
        let keep_alive = false;
        mock_metadata_store.expect_load().with(eq(domain_id.clone()), eq(query.clone()), eq(keep_alive)).returning(|_domain_id, _query, _keep_alive| {
            let (mut tx, rx) = channel::<Result<r_Metadata, DomainError>>(100);
            
            tx.try_send(Err(DomainError::Io(std::io::Error::new(std::io::ErrorKind::Other, "test-error")))).unwrap();
            
            let reader = MetadataReader {
                reader: rx,
                id: None
            };
            Ok(reader)
        });

        let mut datastore = FsDatastore {
            metadata_store: mock_metadata_store,
            base_path: "test-base-path".to_string(),
        };
        
        let mut consumer = datastore.load(domain_id.clone(), query, false, Vec::new()).await.unwrap();
        consumer.wait_for_done().await.expect_err("should failed");
    }

    #[tokio::test]
    async fn test_fs_database_cancel_load() {
        let mut mock_metadata_store = MockMetadataStore::new();

        let domain_id = "test-domain".to_string();
        let mut query = Query::default();
        query.metadata_only = true;
        let keep_alive = false;

        mock_metadata_store.expect_load().with(eq(domain_id.clone()), eq(query.clone()), eq(keep_alive)).returning(|_domain_id, _query, _keep_alive| {
            let (mut tx, rx) = channel::<Result<r_Metadata, DomainError>>(100);
            let metadata = vec![
                r_Metadata {
                    id: "test-id-1".to_string(),
                    name: "test1.txt".to_string(),
                    data_type: "text/plain".to_string(),
                    size: 10,
                    properties: HashMap::new(),
                    hash: "test-hash-1".to_string(),
                    link: "test-link-1".to_string(),
                },
                r_Metadata {
                    id: "test-id-2".to_string(),
                    name: "test2.txt".to_string(),
                    data_type: "text/plain".to_string(),
                    size: 20,
                    properties: HashMap::new(),
                    hash: "test-hash-2".to_string(),
                    link: "test-link-2".to_string(),
                },
                r_Metadata {
                    id: "test-id-3".to_string(),
                    name: "test3.txt".to_string(),
                    data_type: "text/plain".to_string(),
                    size: 30,
                    properties: HashMap::new(),
                    hash: "test-hash-3".to_string(),
                    link: "test-link-3".to_string(),
                },
                r_Metadata {
                    id: "test-id-4".to_string(),
                    name: "test4.txt".to_string(),
                    data_type: "text/plain".to_string(),
                    size: 40,
                    properties: HashMap::new(),
                    hash: "test-hash-4".to_string(),
                    link: "test-link-4".to_string(),
                },
                r_Metadata {
                    id: "test-id-5".to_string(),
                    name: "test5.txt".to_string(),
                    data_type: "text/plain".to_string(),
                    size: 50,
                    properties: HashMap::new(),
                    hash: "test-hash-5".to_string(),
                    link: "test-link-5".to_string(),
                },
                r_Metadata {
                    id: "test-id-6".to_string(),
                    name: "test6.txt".to_string(),
                    data_type: "text/plain".to_string(),
                    size: 60,
                    properties: HashMap::new(),
                    hash: "test-hash-6".to_string(),
                    link: "test-link-6".to_string(),
                },
                r_Metadata {
                    id: "test-id-7".to_string(),
                    name: "test7.txt".to_string(),
                    data_type: "text/plain".to_string(),
                    size: 70,
                    properties: HashMap::new(),
                    hash: "test-hash-7".to_string(),
                    link: "test-link-7".to_string(),
                },
                r_Metadata {
                    id: "test-id-8".to_string(),
                    name: "test8.txt".to_string(),
                    data_type: "text/plain".to_string(),
                    size: 80,
                    properties: HashMap::new(),
                    hash: "test-hash-8".to_string(),
                    link: "test-link-8".to_string(),
                },
                r_Metadata {
                    id: "test-id-9".to_string(),
                    name: "test9.txt".to_string(),
                    data_type: "text/plain".to_string(),
                    size: 90,
                    properties: HashMap::new(),
                    hash: "test-hash-9".to_string(),
                    link: "test-link-9".to_string(),
                },
                r_Metadata {
                    id: "test-id-10".to_string(),
                    name: "test10.txt".to_string(),
                    data_type: "text/plain".to_string(),
                    size: 100,
                    properties: HashMap::new(),
                    hash: "test-hash-10".to_string(),
                    link: "test-link-10".to_string(),
                },
            ];
            for metadata in metadata.iter() {
                tx.try_send(Ok(metadata.clone())).unwrap();
            }
            let reader = MetadataReader {
                reader: rx,
                id: None
            };
            Ok(reader)
        });

        let mut datastore = FsDatastore {
            metadata_store: mock_metadata_store,
            base_path: "test-base-path".to_string(),
        };

        #[derive(Clone)]
        struct Writer {
            i: Arc<Mutex<i32>>,
            closed: Arc<Mutex<bool>>,
        }

        impl AsyncWrite for Writer {
            fn poll_write(self: Pin<&mut Self>, _: &mut Context<'_>, content: &[u8]) -> Poll<Result<usize, std::io::Error>> {
                let mut i = self.i.lock().unwrap();
                *i += 1;
                Poll::Ready(Ok(content.len()))
            }
            fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
                Poll::Ready(Ok(()))
            }
            fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
                let mut closed = self.closed.lock().unwrap();
                *closed = true;
                Poll::Ready(Ok(()))
            }
        }

        let writer = Writer { i: Arc::new(Mutex::new(0)), closed: Arc::new(Mutex::new(false)) };
        let mut consumer = datastore.load(domain_id.clone(), query, false, writer.clone()).await.unwrap();

        sleep(Duration::from_secs(5)).await;
        consumer.close().await;
        let written = *writer.i.lock().unwrap();
        assert!(written > 2, "writer.i should be greater than 2, got {}", written);
        let closed = *writer.closed.lock().unwrap();
        assert!(closed, "writer should be closed");
        consumer.wait_for_done().await.expect_err("should failed because of closed");
    }

    #[tokio::test]
    async fn test_fs_datastore_load_files_greater_than_10mb() {
        let mut mock_metadata_store = MockMetadataStore::new();
        let domain_id = "test-domain".to_string();
        let mut query = Query::default();
        query.metadata_only = false;
        let keep_alive = false;

        // Create a 15MB test file
        let test_file_path = "test-base-path/test-domain/test-file.bin";
        std::fs::create_dir_all("test-base-path/test-domain").unwrap();
        let mut file = std::fs::File::create(test_file_path).unwrap();
        let data = vec![1u8; 15 * 1024 * 1024]; // 15MB of data
        std::io::Write::write_all(&mut file, &data).unwrap();

        mock_metadata_store.expect_load().with(eq(domain_id.clone()), eq(query.clone()), eq(keep_alive)).returning(move |_, _, _| {
            let (mut tx, rx) = channel(100);
            let metadata = r_Metadata {
                id: "test-id".to_string(),
                name: "test-file.bin".to_string(),
                data_type: "binary".to_string(),
                size: (15 * 1024 * 1024) as u32,
                properties: HashMap::new(),
                hash: "test-hash".to_string(),
                link: test_file_path.to_string(),
            };
            tx.try_send(Ok(metadata.clone())).unwrap();
            Ok(MetadataReader {
                reader: rx,
                id: None
            })
        });

        let mut datastore = FsDatastore {
            metadata_store: mock_metadata_store,
            base_path: "test-base-path".to_string(),
        };

        // Track total bytes written
        let total_bytes = Arc::new(Mutex::new(0usize));
        
        struct TestWriter {
            total_bytes: Arc<Mutex<usize>>,
        }

        impl AsyncWrite for TestWriter {
            fn poll_write(self: Pin<&mut Self>, _: &mut Context<'_>, content: &[u8]) -> Poll<Result<usize, std::io::Error>> {
                let mut total = self.total_bytes.lock().unwrap();
                *total += content.len();
                Poll::Ready(Ok(content.len()))
            }
            fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
                Poll::Ready(Ok(()))
            }
            fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
                Poll::Ready(Ok(()))
            }
        }

        let writer = TestWriter { total_bytes: total_bytes.clone() };
        let mut consumer = datastore.load(domain_id.clone(), query, false, writer).await.unwrap();

        // Wait for completion
        consumer.wait_for_done().await.unwrap();

        let final_bytes = *total_bytes.lock().unwrap();
        assert!(final_bytes > 15 * 1024 * 1024, "Should have written more than 15MB, got {} bytes", final_bytes);

        // Cleanup
        std::fs::remove_dir_all("test-base-path").unwrap();
    }

    #[tokio::test]
    async fn test_fs_datastore_close_writer() {
        let mut mock_metadata_store = MockMetadataStore::new();
        let domain_id = "test-domain".to_string();
        let mut query = Query::default();
        query.metadata_only = false;
        let keep_alive = true;

        mock_metadata_store.expect_load().with(eq(domain_id.clone()), eq(query.clone()), eq(keep_alive)).returning(|_domain_id, _query, _keep_alive| {
            let (mut tx, rx) = channel(100);
            let metadata = r_Metadata {
                id: "test-id".to_string(),
                name: "test-file.bin".to_string(),
                data_type: "binary".to_string(),
                size: 100,
                properties: HashMap::new(),
                hash: "test-hash".to_string(),
                link: "test-link".to_string(),
            };
            tx.try_send(Ok(metadata.clone())).unwrap();
            Ok(MetadataReader {
                reader: rx,
                id: None
            })
        });

        let mut datastore = FsDatastore {
            metadata_store: mock_metadata_store,
            base_path: "test-base-path".to_string(),
        };

        #[derive(Clone)]
        struct TestWriter {
            closed: Arc<Mutex<bool>>,
        }

        impl AsyncWrite for TestWriter {
            fn poll_write(self: Pin<&mut Self>, _: &mut Context<'_>, content: &[u8]) -> Poll<Result<usize, std::io::Error>> {
                if *self.closed.lock().unwrap() {
                    Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, "writer closed")))
                } else {
                    Poll::Ready(Ok(content.len()))
                }
            }
            fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
                if *self.closed.lock().unwrap() {
                    Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, "writer closed")))
                } else {
                    Poll::Ready(Ok(()))
                }
            }
            fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
                let mut closed = self.closed.lock().unwrap();
                *closed = true;
                Poll::Ready(Ok(()))
            }
        }

        let mut writer = TestWriter { closed: Arc::new(Mutex::new(false)) };
        let mut consumer = datastore.load(domain_id.clone(), query, keep_alive, writer.clone()).await.unwrap();
        
        spawn(async move {
            sleep(Duration::from_secs(5)).await;
            writer.close().await.expect("should close writer");
        });
        consumer.wait_for_done().await.expect_err("should failed because of closed");
    }
}
