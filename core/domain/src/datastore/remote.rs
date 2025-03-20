use quick_protobuf::{deserialize_from_slice, serialize_into_vec};

#[cfg(not(target_family = "wasm"))]
use tokio::task::spawn as spawn;

use uuid::Uuid;
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

use std::{future::Future, sync::Arc};
use async_trait::async_trait;
use libp2p::Stream;
use crate::{cluster::{DomainCluster, TaskUpdateEvent, TaskUpdateResult}, datastore::common::{DataReader, DataWriter, Datastore, DomainError}, message::prefix_size_message, protobuf::{domain_data::{self, Data, Metadata},task::{self, mod_ResourceRecruitment as ResourceRecruitment, ConsumeDataInputV1, Status, Task}}};
use futures::{channel::{mpsc::channel, oneshot}, io::ReadHalf, lock::Mutex, AsyncReadExt, AsyncWriteExt, SinkExt, StreamExt};

use super::common::{ReliableDataProducer, Writer};

pub const CONSUME_DATA_PROTOCOL_V1: &str = "/consume/v1";
pub const PRODUCE_DATA_PROTOCOL_V1: &str = "/produce/v1";

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
    async fn read_from_stream(domain_id: String, mut src: ReadHalf<Stream>, mut dest: DataWriter) {
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
                    let mut offset = 0;
                    while offset < data.content.len() {
                        if offset + chunk_size > data.content.len() {
                            chunk_size = data.content.len() - offset;
                        }
                        println!("Uploading chunk: {}/{}", offset, data.content.len());
                        // TODO: Add timeout and retry
                        writer.write_all(&data.content[offset..offset + chunk_size]).await.expect("Failed to write content");
                        writer.flush().await.expect("Failed to flush");
                        offset += chunk_size;
                        println!("Uploaded");
                    }
                },
                Err(e) => {
                    eprintln!("Failed to read data: {:?}", e);
                    return;
                }
            }
        }
        writer.flush().await.expect("Failed to flush");
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
            query: Some(query),
            keep_alive,
        };
        let query_data = serialize_into_vec(&data).expect("cant serialize data length");
        let job = &task::JobRequest {
            name: "stream download domain data".to_string(),
            tasks: vec![
                task::TaskRequest {
                    needs: vec![],
                    resource_recruitment: Some(task::ResourceRecruitment {
                        recruitment_policy: ResourceRecruitment::RecruitmentPolicy::FAIL,
                        termination_policy: ResourceRecruitment::TerminationPolicy::KEEP,
                    }),
                    name: "download_data".to_string(),
                    timeout: "100m".to_string(),
                    max_budget: 1000,
                    capability_filters: Some(task::CapabilityFilters {
                        endpoint: "/consume/v1".to_string(),
                        min_gpu: 0,
                        min_cpu: 0,
                    }),
                    data: None,
                    sender: peer_id.clone(),
                    receiver: "".to_string(),
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
                            let m_buf = serialize_into_vec(&task::DomainClusterHandshake{
                                access_token: task.access_token.clone(),
                            }).unwrap();
                            let mut length_buf = [0u8; 4];
                            let length = m_buf.len() as u32;
                            length_buf.copy_from_slice(&length.to_be_bytes());
                            
                            let mut upload_stream = peer.send(length_buf.to_vec(), task.receiver.clone(), task.endpoint.clone(), 1000).await.expect("cant send handshake");
                            upload_stream.write_all(&m_buf).await.expect("cant write handshake");
                            upload_stream.write_all(&query_data).await.expect("cant write data length");
                            upload_stream.flush().await.expect("cant flush data");
                            upload_stream.close().await.expect("cant close data");
                            
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
        if let Err(e) = res {
            data_sender_clone.send(Err(DomainError::Cancelled)).await.expect("Failed to send error");
        } else if res == Ok(false) {
            data_sender_clone.send(Err(DomainError::Interrupted)).await.expect("Failed to send error");
        }
        data_receiver
    }

    async fn produce(&mut self, domain_id: String) -> ReliableDataProducer {
        let (data_sender, data_receiver) = channel::<Result<Data, DomainError>>(3072);
        let mut upload_task_handler = TaskHandler::new();
        let (uploaded_data_sender, uploaded_data_receiver) = channel::<Result<Metadata, DomainError>>(3072);
        let mut upload_job_recv = self.cluster.submit_job(&task::JobRequest {
            nonce: Uuid::new_v4().to_string(),
            name: "stream uploading recordings".to_string(),
            tasks: vec![
                task::TaskRequest {
                    needs: vec![],
                    resource_recruitment: Some(task::ResourceRecruitment {
                        recruitment_policy: ResourceRecruitment::RecruitmentPolicy::FAIL,
                        termination_policy: ResourceRecruitment::TerminationPolicy::KEEP,
                    }),
                    name: "store_recording".to_string(),
                    timeout: "100m".to_string(),
                    max_budget: 1000,
                    capability_filters: Some(task::CapabilityFilters {
                        endpoint: "/produce/v1".to_string(),
                        min_gpu: 0,
                        min_cpu: 0,
                    }),
                    data: None,
                    sender: self.cluster.peer.id.clone(),
                    receiver: "".to_string(),
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
                            let upload_stream = peer.send(prefix_size_message(&task::DomainClusterHandshake{
                                access_token: task.access_token.clone(),
                            }), task.receiver.clone(), task.endpoint.clone(), 1000).await.expect("cant send handshake");

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
        ReliableDataProducer::new(uploaded_data_receiver, data_sender)
    }
}
