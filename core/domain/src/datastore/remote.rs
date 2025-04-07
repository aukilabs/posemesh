use quick_protobuf::{deserialize_from_slice, serialize_into_vec};
use futures::{io::WriteHalf, select};

#[cfg(not(target_family = "wasm"))]
use tokio::task::spawn as spawn;
use uuid::Uuid;
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

use std::{collections::HashSet, future::Future, sync::Arc};
use async_trait::async_trait;
use libp2p::Stream;
use crate::{cluster::{DomainCluster, TaskUpdateEvent, TaskUpdateResult}, datastore::common::{DataReader, DataWriter, Datastore, DomainError}, message::{handshake, handshake_then_content, prefix_size_message}, protobuf::{domain_data::{self, Data, Metadata},task::{self, mod_ResourceRecruitment as ResourceRecruitment, ConsumeDataInputV1, Status, Task}}};
use super::common::{data_id_generator, DomainData, Reader, ReliableDataProducer, Writer};
use futures::{channel::{mpsc::{self, channel, Receiver}, oneshot}, lock::Mutex, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, SinkExt, StreamExt};

pub const CONSUME_DATA_PROTOCOL_V1: &str = "/consume/v1";
pub const PRODUCE_DATA_PROTOCOL_V1: &str = "/produce/v1";

// TODO: error handling
async fn read_from_stream(domain_id: String, mut src: impl AsyncRead + Unpin, mut dest: DataWriter) {
    loop {
        tracing::debug!("Reading data");
        let mut length_buf = [0u8; 4];
        let has = src.read(&mut length_buf).await.expect("Failed to read length");
        if has == 0 {
            break;
        }
        tracing::debug!("Reading data length");

        let length = u32::from_be_bytes(length_buf) as usize;

        // Read the data in chunks
        let mut buffer = vec![0u8; length];
        src.read_exact(&mut buffer).await.expect("Failed to read buffer");

        tracing::debug!("Reading data buffer");
        let metadata = deserialize_from_slice::<domain_data::Metadata>(&buffer).expect("Failed to deserialize metadata");

        let mut buffer = vec![0u8; metadata.size as usize];
        src.read_exact(&mut buffer).await.expect("Failed to read buffer");

        tracing::debug!("Read data: {}, {}/{}", metadata.name, metadata.size, buffer.len());
        let data = Data {
            metadata,
            domain_id: domain_id.clone(),
            content: buffer,
        };
        if let Err(e) = dest.send(Ok(data)).await {
            tracing::error!("{}", e);
            break;
        }
    }
}

async fn write_to_stream(src: Arc<Mutex<DataReader>>, stream: Stream, mut response_sender: Writer<Metadata>) {
    let mut src = src.lock().await;

    let (mut reader, mut writer) = stream.split();
    
    while let Some(data) = src.next().await {
        match data {
            Ok(data) => {
                let m_buf = prefix_size_message(&data.metadata);

                tracing::debug!("Uploading data {}, {}/{}", data.metadata.name, data.metadata.size, data.content.len());
                assert_eq!(data.metadata.size as usize, data.content.len(), "size should match");
            
                writer.write_all(&m_buf).await.expect("Failed to write metadata");
                writer.flush().await.expect("Failed to flush");

                let default_chunk_size = 5 * 1024; // wasm allows 8192 = 8KB the most
                let mut chunk_size = default_chunk_size;
                // let mut offset = 0;
                let mut written = 0;
                while written < data.content.len() {
                    if written + chunk_size > data.content.len() {
                        chunk_size = data.content.len() - written;
                    }
                    tracing::debug!("Uploading chunk: {}/{}", written, data.content.len());
                    match writer.write(&data.content[written..written + chunk_size]).await {
                        Ok(0) => {
                            tracing::error!("Failed to write content, is it backpressure?");
                            continue;
                        }
                        Ok(n) => {
                            written += n;
                            tracing::debug!("Uploaded chunk: {}/{}", written, data.content.len());
                        },
                        Err(e) => {
                            tracing::error!("Failed to write content: {:?}", e);
                            break;
                        }
                    }
                    writer.flush().await.expect("Failed to flush after chunk");
                }
            },
            Err(e) => {
                tracing::error!("Failed to read data: {:?}", e);
                return;
            }
        }
    }
    tracing::info!("Flushed all data");
}
pub(crate) struct RemoteDomainData {
    writer: Arc<Mutex<WriteHalf<Stream>>>,
    expected_size: usize,
    sent_size: Arc<Mutex<usize>>,
    id: String,
}

impl RemoteDomainData {
    pub fn new(id: String, expected_size: usize, writer: Arc<Mutex<WriteHalf<Stream>>>) -> Self {
        Self { writer, expected_size, sent_size: Arc::new(Mutex::new(0)), id }
    }
}

#[async_trait]
impl DomainData for RemoteDomainData {
    async fn push_chunk(&mut self, datum: &[u8], more: bool) -> Result<String, DomainError> {
        if datum.len() != 0 {
            let chunk_size = 5 * 1024; // wasm allows 8192 = 8KB the most

            let mut writer = self.writer.lock().await;
            let chunks = datum.chunks(chunk_size);
            for chunk in chunks {
                writer.write_all(chunk).await.map_err(|e| DomainError::InternalError(Box::new(e)))?;
                writer.flush().await.map_err(|e| DomainError::InternalError(Box::new(e)))?;
                let mut sent_size = self.sent_size.lock().await;
                *sent_size += chunk.len();
                drop(sent_size);
            }
        }

        if !more {
            let sent_size = self.sent_size.lock().await;
            if *sent_size != self.expected_size {
                return Err(DomainError::Cancelled(format!("size mismatch: expected {}, got {}", self.expected_size, *sent_size)));
            }
        }

        Ok(self.id.clone())
    }
}

#[derive(Clone, Debug)]
pub struct RemoteReliableDataProducer {
    // writer: DataWriter,
    pendings: Arc<Mutex<HashSet<String>>>,
    // pub progress: Arc<Mutex<Receiver<i32>>>,
    total: Arc<Mutex<i32>>,
    stream: Option<Arc<Mutex<WriteHalf<Stream>>>>,
}

impl RemoteReliableDataProducer {
    pub fn new() -> Self {
        let pendings = Arc::new(Mutex::new(HashSet::new()));
        // let pending_clone = pendings.clone();
        let total: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
        // let total_clone = total.clone();
        // spawn(async move {
        //     while let Some(m) = response.next().await {
        //         match m {
        //             Ok(metadata) => {
        //                 let id = metadata.id.unwrap_or("why no id".to_string());
        //                 let mut pendings = pending_clone.lock().await;
        //                 let total = total_clone.lock().await;
        //                 let completed = *total as usize - pendings.len() + 1;
        //                 pendings.remove(&id);
        //                 let progress = completed * 100 / *total as usize;
        //                 progress_sender.send(progress as i32).await.expect("can't send progress");
        //             }
        //             Err(e) => {
        //                 eprintln!("{}", e);
        //             }
        //         }
        //     }
        // });

        Self {
            stream: None,
            pendings,
            // progress: Arc::new(Mutex::new(progress_receiver)),
            total,
        }
    }

    pub fn init(&mut self, stream: Stream) {
        let (mut reader, writehalf) = stream.split();
        self.stream = Some(Arc::new(Mutex::new(writehalf)));
        let pending_clone = self.pendings.clone();
        // let total_clone = self.total.clone();
        // let (mut progress_sender, progress_receiver) = mpsc::channel(100);
        // let progress_sender = self.progress_sender.clone();

        spawn(async move {
            loop {
                let mut length_buf = [0u8; 4];
                let has = reader.read(&mut length_buf).await.expect("Failed to read length");
                if has == 0 {
                    break;
                }
        
                let length = u32::from_be_bytes(length_buf) as usize;
        
                // Read the data in chunks
                let mut buffer = vec![0u8; length];
                reader.read_exact(&mut buffer).await.expect("Failed to read buffer");
        
                let metadata = deserialize_from_slice::<domain_data::Metadata>(&buffer).expect("Failed to deserialize metadata");
                let id = metadata.id.unwrap_or("why no id".to_string());
                let mut pendings = pending_clone.lock().await;
                pendings.remove(&id);
                // let total = total_clone.lock().await;
                // let completed = *total as usize - pendings.len() + 1;
                // let progress = completed * 100 / *total as usize;
                // progress_sender.send(progress as i32).await.expect("can't send progress");
            }
        });
    }
}

#[async_trait]
impl ReliableDataProducer for RemoteReliableDataProducer {
    async fn push(&mut self, data: &domain_data::Metadata) -> Result<Box<dyn DomainData>, DomainError> {
        let mut data = data.clone();
        if data.id.is_none() {
            data.id = Some(data_id_generator());
        }
        let id = data.id.clone().unwrap();
        if self.stream.is_none() {
            return Err(DomainError::InternalError(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Stream not initialized"))));
        }
        let stream = self.stream.clone().unwrap();
        let mut writer = stream.lock().await;
        writer.write_all(&prefix_size_message(&data)).await.expect("Failed to write metadata");
        writer.flush().await.expect("Failed to flush");
        drop(writer);
        Ok(Box::new(RemoteDomainData::new(id, data.size as usize, stream)))
    }

    async fn is_completed(&self) -> bool {
        let pendings = self.pendings.lock();
        pendings.await.is_empty()
    }

    async fn close(mut self) {
        // self.writer.close().await.expect("can't close writer");
        // self.progress.lock().await.close();
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
    #[cfg(not(target_family = "wasm"))]
    handler: Option<tokio::task::JoinHandle<()>>,
    #[cfg(target_family = "wasm")]
    handler: Option<()>,
}

impl TaskHandler {
    fn new() -> Self {
        Self { handler: None }
    }
    // Define the function to execute the handler
    fn execute<F>(&mut self, handler: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        #[cfg(not(target_family = "wasm"))]
        {
            // Non-WASM environment: spawn using tokio
            self.handler = Some(spawn(handler));
        }

        #[cfg(target_family = "wasm")]
        {
            // WASM environment: spawn using spawn_local
            spawn(handler);
        }
    }

    fn cancel(&mut self) {
        #[cfg(not(target_family = "wasm"))]
        {
            if let Some(handler) = self.handler.take() {
                handler.abort();
            }
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

#[async_trait]
impl Datastore for RemoteDatastore {
    async fn load(&mut self, domain_id: String, query: domain_data::Query, keep_alive: bool) -> DataReader
    {
        let mut download_task = TaskHandler::new();
        let (data_sender, data_receiver) = channel::<Result<Data, DomainError>>(3072);
        let peer_id = self.cluster.peer.id.clone();
        let mut peer = self.cluster.peer.client.clone();
        let domain_id = domain_id.clone();
        let query = query.clone();
        let data = ConsumeDataInputV1 {
            query,
            keep_alive,
        };
        let job = &task::JobRequest {
            name: "stream download domain data".to_string(),
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
                        endpoint: "/consume/v1".to_string(),
                        min_gpu: None,
                        min_cpu: None,
                    },
                    data: None,
                    sender: peer_id.clone(),
                    receiver: None,
                }
            ],
            nonce: Uuid::new_v4().to_string(),
        };

        let mut download_task_recv = self.cluster.submit_job(job).await;

        let (tx, rx) = oneshot::channel::<bool>();
        let mut data_sender_clone = data_sender.clone();

        spawn(async move {
            let data_sender = data_sender.clone();
            loop {
                let update = download_task_recv.next().await;
                match update {
                    Some(TaskUpdateEvent {
                        result: TaskUpdateResult::Ok(mut task),
                        ..
                    }) => match task.status {
                        Status::PENDING => {
                            task.status = Status::STARTED;
                            let domain_id_clone = domain_id.clone();
                            peer.publish(task.job_id.clone(), serialize_into_vec(&task).expect("Failed to serialize message")).await.expect("Failed to publish message");
                            
                            let res = handshake_then_content(peer.clone(), &task.access_token.clone().unwrap(), &task.receiver.clone().unwrap(), &task.endpoint.clone(), &data, 5000).await;
                            if let Err(e) = res {
                                tracing::error!("Failed to send handshake: {:?}", e);
                                tx.send(false).expect("Failed to send completion signal");
                                download_task.cancel();
                                return;
                            }
                            let mut upload_stream = res.unwrap();
                            upload_stream.close().await.expect("Failed to close stream");

                            let (reader, _) = upload_stream.split();
                            download_task.execute(async move {
                                read_from_stream(domain_id_clone, reader, data_sender).await;
                            });
                            tx.send(true).expect("Failed to send completion signal");
                            return;
                        },
                        Status::FAILED => {
                            tracing::error!("Failed to download data: {:?}", task);
                            tx.send(false).expect("Failed to send completion signal");
                            download_task.cancel();
                            return;
                        },
                        _ => ()
                    }
                    Some(TaskUpdateEvent {
                        result: TaskUpdateResult::Err(e),
                        ..
                    }) => {
                        eprintln!("Task update failure: {:?}", e);
                        tx.send(false).expect("Failed to send completion signal");
                        download_task.cancel();
                        return;
                    }
                    None => {
                        println!("task update channel is closed");
                        tx.send(false).expect("Failed to send completion signal");
                        download_task.cancel();
                        return;
                    }
                }
            }
        });

        let res = rx.await;
        if let Err(e) = res {
            data_sender_clone.send(Err(DomainError::Cancelled(e.to_string()))).await.expect("Failed to send error");
        } else if res == Ok(false) {
            data_sender_clone.send(Err(DomainError::Interrupted)).await.expect("Failed to send error");
        }
        data_receiver
    }

    async fn upsert(&mut self, _: String) -> Box<dyn ReliableDataProducer>{
        let mut upload_job_recv = self.cluster.submit_job(&task::JobRequest {
            nonce: Uuid::new_v4().to_string(),
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
                        endpoint: "/produce/v1".to_string(),
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
                            let upload_stream = handshake(peer.clone(), &task.access_token.clone().unwrap(), &task.receiver.clone().unwrap(), &task.endpoint.clone(), 5000).await;
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
                            println!("Task status: {:?}", task.status);
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
                        eprintln!("Task update failure: {:?}", e);
                        break;
                    }
                }
            }
        });

        let producer = rx.await;
        if let Ok(Some(producer)) = producer {
            Box::new(producer)
        } else {
            // TODO: handle error instead of creating a new producer
            Box::new(RemoteReliableDataProducer::new())
        }
    }
}
