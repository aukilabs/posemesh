use quick_protobuf::{deserialize_from_slice, serialize_into_vec};
#[cfg(not(target_family = "wasm"))]
use tokio::task::spawn as spawn;

#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

use std::{error::Error, future::Future};
use async_trait::async_trait;
use libp2p::{PeerId, Stream};
use networking::context::Context;
use protobuf::task::{self, mod_ResourceRecruitment as ResourceRecruitment, Status};
use crate::{cluster::{DomainCluster, TaskUpdateEvent, TaskUpdateResult}, datastore::Datastore, protobuf::domain_data::{self, Data}};
use futures::{channel::{mpsc::{channel, Receiver, Sender}, oneshot}, stream::{self, SplitStream}, AsyncReadExt, AsyncWriteExt, SinkExt, StreamExt};

pub type DataStream = Receiver<Result<Data, DomainError>>;

// Define a custom error type
#[derive(Debug)]
pub enum DomainError {
    NotFound,
    Interrupted,
    Cancelled,
}

impl Error for DomainError {}
impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DomainError::NotFound => write!(f, "Not found"),
            DomainError::Interrupted => write!(f, "Interrupted"),
            DomainError::Cancelled => write!(f, "Cancelled"),
        }
    }
}

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
    peer: Context,
}

impl RemoteDatastore {
    pub fn new(cluster: DomainCluster, peer: Context ) -> Self {
        Self { cluster, peer }
    }

    // TODO: error handling
    async fn read_from_stream(domain_id: String, mut src: Stream, mut dest: Sender<Result<Data, DomainError>>) {
        loop {
            let mut length_buf = [0u8; 4];
            let has = src.read(&mut length_buf).await.expect("Failed to read length");
            if has == 0 {
                break;
            }
    
            let length = u32::from_be_bytes(length_buf) as usize;
    
            // Read the data in chunks
            let mut buffer = vec![0u8; length];
            src.read_exact(&mut buffer).await.expect("Failed to read buffer");
    
            let metadata = deserialize_from_slice::<domain_data::Metadata>(&buffer).expect("Failed to deserialize metadata");
    
            let mut buffer = vec![0u8; metadata.size as usize];
            src.read_exact(&mut buffer).await.expect("Failed to read buffer");

            let data = Data {
                metadata,
                domain_id: domain_id.clone(),
                content: buffer,
            };
            let _ = dest.send(Ok(data));
        }
    }
}

#[async_trait]
impl Datastore for RemoteDatastore {
    async fn find(&mut self, domain_id: String, query: domain_data::Query) -> DataStream
    {
        let mut download_task = TaskHandler::new();
        let (data_sender, data_receiver) = channel::<Result<Data, DomainError>>(100);
        let peer_id = self.peer.id.clone();
        let mut peer = self.peer.clone();
        let domain_id = domain_id.clone();
        let query = query.clone();
        let job = &task::Job {
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
                        endpoint: "/download/v1".to_string(),
                        min_gpu: 0,
                        min_cpu: 0,
                    }),
                    // TODO: task::Any type_url
                    data: Some(task::Any {
                        type_url: "type.googleapis.com/posemesh.task.DownloadData".to_string(),
                        value: serialize_into_vec(&query).unwrap(),
                    }),
                    sender: peer_id.clone(),
                    receiver: "".to_string(),
                }
            ],
        };

        let mut download_task_recv = self.cluster.submit_job(job).await;

        spawn(async move {
            loop {
                let update = download_task_recv.try_next();
                match update {
                    Ok(Some(TaskUpdateEvent {
                        result: TaskUpdateResult::Ok(mut task),
                        ..
                    })) => match task.status {
                        Status::PENDING => {
                            task.status = Status::STARTED;
                            let domain_id_clone = domain_id.clone();
                            let data_sender_clone = data_sender.clone();
                            peer.publish(task.job_id.clone(), serialize_into_vec(&task).expect("Failed to serialize message")).await.expect("Failed to publish message");
                            let m_buf = serialize_into_vec(&task::DomainClusterHandshake{
                                access_token: task.access_token.clone(),
                            }).unwrap();
                            let mut length_buf = [0u8; 4];
                            let length = m_buf.len() as u32;
                            length_buf.copy_from_slice(&length.to_be_bytes());
                            let upload_stream = peer.send(length_buf.to_vec(), task.receiver.clone(), task.endpoint.clone(), 1000).await.expect("cant send handshake");
                            download_task.execute(async move {
                                RemoteDatastore::read_from_stream(domain_id_clone, upload_stream, data_sender_clone).await;
                            });
                        },
                        Status::FAILED => {
                            eprintln!("Failed to download data: {:?}", task);
                            download_task.cancel();
                        },
                        _ => {}
                    }
                    _ => {}
                }
            }
        });
        
        data_receiver
    }
}
