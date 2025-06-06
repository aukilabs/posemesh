
use quick_protobuf::{deserialize_from_slice, serialize_into_vec};
use futures::{channel::oneshot::Canceled, io::WriteHalf, select};
use uuid::Uuid;

#[cfg(not(target_family = "wasm"))]
use tokio::task::spawn as spawn;
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

use std::{collections::HashSet, future::Future, sync::Arc};
use async_trait::async_trait;
use posemesh_networking::client::{Client, TClient};
use libp2p::Stream;
use crate::{capabilities::domain_data::{CONSUME_DATA_PROTOCOL_V1, PRODUCE_DATA_PROTOCOL_V1}, cluster::{DomainCluster, TaskUpdateEvent, TaskUpdateResult}, datastore::common::{Datastore, DomainError}, message::{handshake, handshake_then_prefixed_content, prefix_size_message, read_prefix_size_message}, protobuf::{domain_data, task::{self, mod_ResourceRecruitment as ResourceRecruitment, Any, ConsumeDataInputV1, Status, Task}}};
use super::common::{hash_chunk, DataConsumer, DomainData, ReliableDataProducer, CHUNK_SIZE};
use futures::{channel::oneshot, lock::Mutex, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, StreamExt};
use rs_merkle::{algorithms::Sha256, MerkleTree};
use futures::future::FutureExt;

// TODO: error handling
async fn read_from_stream<W: AsyncWrite + Unpin + Send + 'static>(metadata_only: bool, mut src: impl AsyncRead + Unpin, mut dest: W) -> Result<(), DomainError> {
    loop {
        let mut length_buf = [0u8; 4];
        let has = src.read(&mut length_buf).await?;
        if has == 0 {
            return Ok(());
        }

        let length = u32::from_be_bytes(length_buf) as usize;

        let mut buffer = vec![0u8; length];
        src.read_exact(&mut buffer).await?;

        let metadata = deserialize_from_slice::<domain_data::Metadata>(&buffer)?;
        let written = dest.write(&prefix_size_message(&metadata)).await?;
        let skip = written == 0; // dest skips this data
        dest.flush().await?;
        if !metadata_only {
            let mut content = vec![0u8; metadata.size as usize];
            src.read_exact(&mut content).await?;
            if !skip {
                dest.write_all(&content).await?;
                dest.flush().await?;
    
                tracing::debug!("Read data: {}, {}/{}", metadata.name, metadata.size, content.len());
            }
        }
    }
}

pub(crate) struct RemoteDomainData {
    writer: Arc<Mutex<WriteHalf<Stream>>>,
    expected_size: usize,
    sent_size: usize,
    merkle_tree: MerkleTree<Sha256>,
    left_buffer: Vec<u8>
}

impl RemoteDomainData {
    pub fn new(expected_size: usize, writer: Arc<Mutex<WriteHalf<Stream>>>) -> Self {
        Self { writer, expected_size, sent_size: 0, merkle_tree: MerkleTree::<Sha256>::new(), left_buffer: vec![] }
    }
}

#[async_trait]
impl DomainData for RemoteDomainData {
    async fn next_chunk(&mut self, datum: &[u8], more: bool) -> Result<String, DomainError> {
        let mut writer = self.writer.lock().await;
        self.left_buffer.extend_from_slice(datum);
        while self.left_buffer.len() > 0 && (self.left_buffer.len() >= CHUNK_SIZE || !more) {
            let mut length = CHUNK_SIZE;
            if self.left_buffer.len() < CHUNK_SIZE {
                length = self.left_buffer.len();
            }
            let chunk = self.left_buffer.drain(..length).collect::<Vec<u8>>();
            let hash = hash_chunk(&chunk);
            self.merkle_tree.insert(hash);
            writer.write_all(&chunk).await?;
            writer.flush().await?;
            self.sent_size += chunk.len();
            tracing::debug!("uploaded {}/{} bytes", self.sent_size, self.expected_size);
        }
        drop(writer);

        if !more {
            if self.sent_size != self.expected_size {
                return Err(DomainError::SizeMismatch(self.expected_size, self.sent_size));
            }
            self.merkle_tree.commit();
            let hash = self.merkle_tree.root();
            if hash.is_none() {
                return Err(DomainError::InternalError(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Failed to calculate merkle tree root"))));
            }
            let hash = hex::encode(hash.unwrap());
            Ok(hash)
        } else {
            Ok("".to_string())
        }
    }
}

#[derive(Debug)]
pub struct RemoteReliableDataProducer {
    pendings: Arc<Mutex<HashSet<String>>>,
    stream: Option<Arc<Mutex<WriteHalf<Stream>>>>,
}

impl RemoteReliableDataProducer {
    pub fn new() -> Self {
        let pendings = Arc::new(Mutex::new(HashSet::new()));

        Self {
            stream: None,
            pendings,
        }
    }

    pub fn init(&mut self, stream: Stream) {
        let (mut reader, writehalf) = stream.split();
        self.stream = Some(Arc::new(Mutex::new(writehalf)));
        let pending_clone = self.pendings.clone();

        spawn(async move {
            loop {
                let res = read_prefix_size_message::<domain_data::Metadata, _>(&mut reader).await;
                match res {
                    Ok(metadata) => {
                        let id = metadata.id;
                        let mut pendings = pending_clone.lock().await;
                        pendings.remove(&id);
                    }
                    Err(e) => {
                        tracing::error!("Failed to read metadata: {:?}", e);
                        break;
                    }
                }
            }
        });
    }
}

#[async_trait]
impl ReliableDataProducer for RemoteReliableDataProducer {
    async fn push(&mut self, data: &domain_data::UpsertMetadata) -> Result<Box<dyn DomainData>, DomainError> {
        let data = data.clone();
        if self.stream.is_none() {
            return Err(DomainError::InternalError(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Stream not initialized"))));
        }
        let stream = self.stream.clone().unwrap();
        let mut writer = stream.lock().await;
        writer.write_all(&prefix_size_message(&data)).await.expect("Failed to write metadata");
        writer.flush().await.expect("Failed to flush");
        drop(writer);
        let id = data.id.clone();
        let mut pendings = self.pendings.lock().await;
        pendings.insert(id.clone());
        drop(pendings);
        Ok(Box::new(RemoteDomainData::new(data.size as usize, stream)))
    }

    async fn is_completed(&self) -> bool {
        let pendings = self.pendings.lock().await;
        pendings.is_empty()
    }

    async fn close(&mut self) {
        if !self.is_completed().await {
            tracing::warn!("You are closing the producer before it's completed");
        }
        self.pendings.lock().await.clear();
        let stream = self.stream.take();
        if let Some(stream) = stream {
            if let Err(e) = stream.lock().await.close().await {
                tracing::error!("Failed to close stream: {:?}", e);
            }
        }
    }
}

#[derive(Debug)]
struct TaskHandler {
    cancel_tx: Option<oneshot::Sender<()>>,
    done_rx: Option<oneshot::Receiver<Result<(), DomainError>>>,
}

impl TaskHandler {
    fn new() -> Self {
        Self { cancel_tx: None, done_rx: None }
    }
    // Define the function to execute the handler
    fn execute<F>(&mut self, cancel_tx: oneshot::Sender<()>, handler: F)
    where
        F: Future<Output = Result<(), DomainError>> + Send + 'static,
    {
        self.cancel_tx = Some(cancel_tx);
        let (done_tx, done_rx) = oneshot::channel::<Result<(), DomainError>>();
        self.done_rx = Some(done_rx);
        spawn(async move {
            let res = handler.await;
            let _ = done_tx.send(res);
        });
    }
}

#[async_trait]
impl DataConsumer for TaskHandler {
    async fn close(&mut self) {
        if let Some(cancel_tx) = self.cancel_tx.take() {
            cancel_tx.send(()).expect("Failed to send cancel signal");
        }
    }

    async fn wait_for_done(&mut self) -> Result<(), DomainError> {
        if let Some(done_rx) = self.done_rx.take() {
            match done_rx.await {
                Ok(res) => res,
                Err(e) => Err(DomainError::Cancelled("TaskHandler has been cancelled".to_string(), e)),
            }
        } else {
            Err(DomainError::Cancelled("TaskHandler is not initialized".to_string(), Canceled))
        }
    }
}

#[derive(Clone)]
pub struct RemoteDatastore {
    cluster: DomainCluster,
}

impl RemoteDatastore {
    pub fn new(cluster: DomainCluster) -> Self {
        Self { cluster }
    }
}

async fn download_data<W: AsyncWrite + Unpin + Send + 'static>(peer: Client, domain_id: String, metadata_only: bool, writer: W, mut task: Task, data: task::ConsumeDataInputV1) -> Result<TaskHandler, DomainError> {
    let upload_stream = handshake_then_prefixed_content::<ConsumeDataInputV1>(peer.clone(), &domain_id, &task.access_token.clone().unwrap(), &task.receiver.clone().unwrap(), &task.endpoint.clone(), &data, 5000).await?;
    let mut download_task = TaskHandler::new();
    let mut peer_clone = peer.clone();
    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
    download_task.execute(cancel_tx, async move {
        let (reader, mut stream_writer) = upload_stream.split();
        let read_result: Result<(), DomainError> = select! {
            result = read_from_stream(metadata_only, reader, writer).fuse() => result,
            _ = cancel_rx.fuse() => {
                tracing::debug!("Download cancelled");
                let res = async {
                    stream_writer.write_all(&prefix_size_message(&task::UnsubscribeDataQueryV1{})).await
                        .map_err(DomainError::Io)?;
        
                    stream_writer.flush().await
                        .map_err(DomainError::Io)?;
        
                    stream_writer.close().await
                        .map_err(DomainError::Io)?;
        
                    Ok(())
                }.await;
        
                res
            }
        };

        if let Err(e) = read_result {
            tracing::error!("Failed to read data: {:?}", e);
            task.status = Status::FAILED;
            task.output = Some(Any { type_url: "Error".to_string(), value: serialize_into_vec(&task::Error {
                message: e.to_string(),
            }).expect("Failed to serialize error") });
            peer_clone.publish(task.job_id.clone(), serialize_into_vec(&task).expect("Failed to serialize message")).await.expect("Failed to publish message");
            return Err(e);
        }
        task.status = Status::DONE;
        peer_clone.publish(task.job_id.clone(), serialize_into_vec(&task).expect("Failed to publish message")).await.expect("Failed to publish message");
        Ok(())
    });
    Ok(download_task)
}

#[async_trait]
impl Datastore for RemoteDatastore {
    async fn load<W: AsyncWrite + Unpin + Send + 'static>(&mut self, domain_id: String, query: domain_data::Query, keep_alive: bool, writer: W) -> Result<Box<dyn DataConsumer>, DomainError>
    {
        let peer_id = self.cluster.peer.id.clone();
        let mut peer = self.cluster.peer.client.clone();
        let domain_id = domain_id.clone();
        let metadata_only = query.metadata_only;
        let data = ConsumeDataInputV1 {
            query: query.clone(),
            keep_alive,
        };
        let job = &task::JobRequest {
            name: "stream download domain data".to_string(),
            domain_id: domain_id.clone(),
            tasks: vec![
                task::TaskRequest {
                    needs: vec![],
                    resource_recruitment: task::ResourceRecruitment {
                        recruitment_policy: ResourceRecruitment::RecruitmentPolicy::FAIL,
                        termination_policy: ResourceRecruitment::TerminationPolicy::KEEP,
                    },
                    name: "download_data".to_string(),
                    timeout: "100m".to_string(),
                    max_budget: Some(1000),
                    capability_filters: task::CapabilityFilters {
                        endpoint: CONSUME_DATA_PROTOCOL_V1.to_string(),
                        min_gpu: None,
                        min_cpu: None,
                    },
                    data: Some(Any {
                        type_url: "ConsumeDataInputV1".to_string(),
                        value: serialize_into_vec(&data).expect("Failed to serialize message"),
                    }),
                    sender: peer_id.clone(),
                    receiver: None,
                }
            ],
            nonce: Uuid::new_v4().to_string(),
        };

        let mut download_task_recv = self.cluster.submit_job(job).await;
        let (tx, rx) = oneshot::channel::<Result<TaskHandler, DomainError>>();

        spawn(async move {
            if let Some(update) = download_task_recv.next().await {
                match update {
                    TaskUpdateEvent {
                        result: TaskUpdateResult::Ok(mut task),
                        ..
                    } => match task.status {
                        Status::PENDING => {
                            task.status = Status::STARTED;
                            if let Err(e) = peer.publish(task.job_id.clone(), serialize_into_vec(&task).expect("Failed to serialize message")).await {
                                tracing::error!("Failed to publish message: {:?}", e);
                            }
                            let download_task = download_data(peer.clone(), domain_id.clone(), metadata_only, writer, task, data).await;
                            tx.send(download_task).expect("Failed to send completion signal");
                            return;
                        },
                        Status::FAILED => {
                            tracing::error!("Failed to download data: {:?}", task);
                            tx.send(Err(DomainError::Cancelled("Failed to download data".to_string(), Canceled))).expect("Failed to send completion signal");
                            return;
                        },
                        _ => {
                            tracing::debug!("Task status: {:?}", task.status);
                            tx.send(Err(DomainError::Cancelled("We are not supposed to handle this status".to_string(), Canceled))).expect("Failed to send completion signal");
                            return;
                        }
                    }
                    TaskUpdateEvent {
                        result: TaskUpdateResult::Err(e),
                        ..
                    } => {
                        tracing::error!("Task update failure: {:?}", e);
                        tx.send(Err(e)).expect("Failed to send completion signal");
                        return;
                    }
                }
            }

            tracing::debug!("task update channel is closed");
            tx.send(Err(DomainError::Cancelled("Task update channel is closed".to_string(), Canceled))).expect("Failed to send completion signal");
        });

        // TODO: handle more statuses for example domain manager cancel this task by sending a Failed status when it runs out of credits
        // spawn(async move {
            
        // });
        let res = rx.await;
        match res {
            Ok(Ok(download_task)) => Ok(Box::new(download_task)),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(DomainError::Cancelled("Failed to download data".to_string(), e)),
        }
    }

    async fn upsert(&mut self, domain_id: String) -> Result<Box<dyn ReliableDataProducer>, DomainError>{
        let mut upload_job_recv = self.cluster.submit_job(&task::JobRequest {
            nonce: Uuid::new_v4().to_string(),
            domain_id: domain_id.clone(),
            name: "stream uploading recordings".to_string(),
            tasks: vec![
                task::TaskRequest {
                    needs: vec![],
                    resource_recruitment: task::ResourceRecruitment {
                        recruitment_policy: ResourceRecruitment::RecruitmentPolicy::FAIL,
                        termination_policy: ResourceRecruitment::TerminationPolicy::KEEP,
                    },
                    name: "store_recording".to_string(),
                    timeout: "100m".to_string(),
                    max_budget: Some(1000),
                    capability_filters: task::CapabilityFilters {
                        endpoint: PRODUCE_DATA_PROTOCOL_V1.to_string(),
                        min_gpu: Some(0),
                        min_cpu: Some(0),
                    },
                    data: None,
                    sender: self.cluster.peer.id.clone(),
                    receiver: None,
                }
            ],
        }).await;

        let mut peer = self.cluster.peer.client.clone();
        let (tx, rx) = oneshot::channel::<Option<RemoteReliableDataProducer>>();
        let domain_id = domain_id.clone();
        spawn(async move{
            loop {
                let update = upload_job_recv.next().await;
                match update {
                    Some(TaskUpdateEvent {
                        result: TaskUpdateResult::Ok(mut task),
                        ..
                    }) => match task.status {
                        Status::PENDING => {
                            task.status = Status::STARTED;

                            if let Err(e) = peer.publish(task.job_id.clone(), serialize_into_vec(&task).expect("Failed to serialize message")).await {
                                tracing::error!("Failed to publish message: {:?}", e);
                            }

                            let task = task.clone();
                            let upload_stream = handshake(peer.clone(), &domain_id, &task.access_token.clone().unwrap(), &task.receiver.clone().unwrap(), &task.endpoint.clone(), 5000).await;
                            if let Err(e) = upload_stream {
                                tracing::error!("Failed to send handshake: {:?}", e);
                                tx.send(None).expect("Failed to send completion signal");
                                return;
                            }
                            let upload_stream = upload_stream.unwrap();
                            let mut producer = RemoteReliableDataProducer::new();
                            producer.init(upload_stream);
                            tx.send(Some(producer)).expect("Failed to send completion signal");
                            break;
                        }
                        Status::FAILED => {
                            // TODO: handle error
                            tracing::error!("Failed to upload data: {:?}", task);
                            break;
                        },
                        _ => {
                            tracing::debug!("Task status: {:?}", task.status);
                        }
                    }
                    None => {
                        tracing::debug!("task update channel is closed");
                        break;
                    }
                    Some(TaskUpdateEvent {
                        result: TaskUpdateResult::Err(e),
                        ..
                    }) => {
                        tracing::error!("Task update failure: {:?}", e);
                        break;
                    }
                }
            }
        });

        let producer = rx.await;
        if let Ok(Some(producer)) = producer {
            Ok(Box::new(producer))
        } else {
            Err(DomainError::Cancelled("Failed to upload data".to_string(), Canceled))
        }
    }
}
