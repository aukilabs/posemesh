use quick_protobuf::{deserialize_from_slice, serialize_into_vec};
use futures::select;

#[cfg(not(target_family = "wasm"))]
use tokio::task::spawn as spawn;
use uuid::Uuid;
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

use std::{collections::HashSet, future::Future, sync::Arc};
use async_trait::async_trait;
use libp2p::Stream;
use crate::{cluster::{DomainCluster, TaskUpdateEvent, TaskUpdateResult}, datastore::common::{DataReader, DataWriter, Datastore, DomainError}, message::{handshake, handshake_then_content, prefix_size_message}, protobuf::{domain_data::{self, Data, Metadata},task::{self, mod_ResourceRecruitment as ResourceRecruitment, ConsumeDataInputV1, Status}}};
use futures::{channel::{mpsc::channel, oneshot}, io::ReadHalf, lock::Mutex, AsyncReadExt, AsyncWriteExt, SinkExt, StreamExt};
use super::common::{data_id_generator, Reader, ReliableDataProducer, Writer};

pub const CONSUME_DATA_PROTOCOL_V1: &str = "/consume/v1";
pub const PRODUCE_DATA_PROTOCOL_V1: &str = "/produce/v1";

#[derive(Clone)]
pub struct RemoteReliableDataProducer {
    writer: DataWriter,
    pendings: Arc<Mutex<HashSet<String>>>,
    pub progress: Arc<Mutex<Receiver<i32>>>,
    total: Arc<Mutex<i32>>,
    completed: Arc<Mutex<oneshot::Receiver<()>>>,
}

impl RemoteReliableDataProducer {
    pub fn new(mut response: Reader<domain_data::Metadata>, writer: DataWriter) -> Self {
        let pendings = Arc::new(Mutex::new(HashSet::new()));
        let pending_clone = pendings.clone();
        let total: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
        let total_clone = total.clone();
        let (mut progress_sender, progress_receiver) = mpsc::channel(100);
        let (completed_sender, completed_receiver) = oneshot::channel::<()>();
        spawn(async move {
            while let Some(m) = response.next().await {
                match m {
                    Ok(metadata) => {
                        let id = metadata.id.unwrap_or("why no id".to_string());
                        let mut pendings = pending_clone.lock().await;
                        let total = total_clone.lock().await;
                        let completed = *total as usize - pendings.len() + 1;
                        pendings.remove(&id);
                        if pendings.is_empty() {
                            let _ = completed_sender.send(());
                        }
                        let progress = completed * 100 / *total as usize;
                        progress_sender.send(progress as i32).await.expect("can't send progress");
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                    }
                }
            }
        });

        Self {
            writer, progress: Arc::new(Mutex::new(progress_receiver)), pendings, total, completed: Arc::new(Mutex::new(completed_receiver))
        }
    }

    pub async fn wait_for_completion(&self) -> Result<(), DomainError> {
        let mut completed = self.completed.lock().await;
        completed.await.map_err(|_| DomainError::Interrupted)
    }
}

#[async_trait]
impl ReliableDataProducer for RemoteReliableDataProducer {
    async fn push(&mut self, data: &domain_data::Data) -> Result<String, DomainError> {
        let mut data = data.clone();
        if data.metadata.id.is_none() {
            data.metadata.id = Some(data_id_generator());
        }
        let id = data.metadata.id.clone().unwrap();
        let res = self.writer.send(Ok(data)).await;
        match res {
            Ok(_) => {
                let mut pendings = self.pendings.lock().await;
                pendings.insert(id.clone());
                let mut total = self.total.lock().await;
                *total += 1;
                Ok(id)
            },
            Err(e) => {
                eprintln!("{}", e);
                Err(DomainError::Interrupted)
            },
        }
    }

    async fn is_completed(&self) -> bool {
        let pendings = self.pendings.lock();
        pendings.await.is_empty()
    }

    async fn close(mut self) {
        #[cfg(target_family = "wasm")]
        self.writer.close().await.expect("can't close writer");

        self.progress.lock().await.close();
        self.pendings.lock().await.clear();
    }

    async fn await_completion(&self) {
        let completed = self.completed.lock().await;
        completed.await.unwrap();
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
                match response_sender.send(Ok(metadata)).await {
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("{}", e);
                        break;
                    }
                }
            }
        });
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
}

#[async_trait]
impl Datastore for RemoteDatastore {
    async fn consume(&mut self, domain_id: String, query: domain_data::Query, keep_alive: bool) -> DataReader
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
                                RemoteDatastore::read_from_stream(domain_id_clone, reader, data_sender).await;
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
        if let Err(_) = res {
            data_sender_clone.send(Err(DomainError::Cancelled)).await.expect("Failed to send error");
        } else if res == Ok(false) {
            data_sender_clone.send(Err(DomainError::Interrupted)).await.expect("Failed to send error");
        }
        data_receiver
    }

    async fn produce(&mut self, domain_id: String) -> Box<dyn ReliableDataProducer>{
        let (data_sender, data_receiver) = channel::<Result<Data, DomainError>>(3720);
        let mut upload_task_handler = TaskHandler::new();
        let (uploaded_data_sender, uploaded_data_receiver) = channel::<Result<Metadata, DomainError>>(3072);
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
        let data_receiver = Arc::new(Mutex::new(data_receiver));
        let (tx, rx) = oneshot::channel::<bool>();
        spawn(async move{
            let data_receiver = data_receiver.clone();
            let uploaded_data_sender = uploaded_data_sender.clone();
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
                                tx.send(false).expect("Failed to send completion signal");
                                upload_task_handler.cancel();
                                return;
                            }
                            let upload_stream = upload_stream.unwrap();
                            let data_receiver = data_receiver.clone();
                            let uploaded_data_sender = uploaded_data_sender.clone();
                            let mut peer = peer.clone();
                            let handler = async move {
                                RemoteDatastore::write_to_stream(data_receiver, upload_stream, uploaded_data_sender).await;
                                let mut task = task.clone();
                                task.status = Status::DONE;
                                peer.publish(task.job_id.clone(), serialize_into_vec(&task).expect("Failed to serialize message")).await.expect("Failed to publish message");
                            };
                            upload_task_handler.execute(handler);
                            tx.send(true).expect("Failed to send completion signal");
                            break;
                        }
                        Status::FAILED => {
                            // TODO: handle error
                            tracing::error!("Failed to upload data: {:?}", task);
                            upload_task_handler.cancel();
                            break;
                        },
                        _ => {
                            println!("Task status: {:?}", task.status);
                        }
                    }
                    None => {
                        tracing::debug!("task update channel is closed");
                        upload_task_handler.cancel();
                        break;
                    }
                    Some(TaskUpdateEvent {
                        result: TaskUpdateResult::Err(e),
                        ..
                    }) => {
                        eprintln!("Task update failure: {:?}", e);
                        upload_task_handler.cancel();
                        break;
                    }
                }
            }
        });

        let _ = rx.await;
        Box::new(RemoteReliableDataProducer::new(uploaded_data_receiver, data_sender))
    }
}
