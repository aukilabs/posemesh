use libp2p::{gossipsub::TopicHash, PeerId};
use futures::{channel::{mpsc::{channel, Receiver, SendError, Sender}, oneshot}, AsyncReadExt, SinkExt, StreamExt};
use posemesh_networking::{event, libp2p::{Networking, NetworkingConfig}};
use crate::{datastore::common::DomainError, message::{prefix_size_message, read_prefix_size_message, request_response}, protobuf::task::{self, Job, JobRequest, Status, SubmitJobResponse}};
use std::{collections::HashMap, fmt::Error};
use quick_protobuf::{deserialize_from_slice, serialize_into_vec};
use posemesh_networking::client::TClient;

#[cfg(not(target_arch = "wasm32"))]
use tokio::spawn;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local as spawn;

#[derive(Debug)]
pub enum TaskUpdateResult {
    Ok(task::Task),
    Err(DomainError),
}

#[derive(Debug)]
pub struct TaskUpdateEvent {
    pub topic: TopicHash,
    pub from: Option<PeerId>,
    pub result: TaskUpdateResult,
}

struct InnerDomainCluster {
    command_rx: Receiver<Command>,
    manager: String,
    peer: Networking,
    jobs: HashMap<TopicHash, Sender<TaskUpdateEvent>>,
}

enum Command {
    SubmitJob {
        job: JobRequest,
        task_updates_channel: Sender<TaskUpdateEvent>,
        response: oneshot::Sender<bool>,
    },
    UpdateTask {
        task: task::Task,
    },
    MonitorJobs {
        response: oneshot::Sender<Receiver<Job>>,
    }
}

impl InnerDomainCluster {
    fn init(mut self) {
        let event_receiver = self.peer.event_receiver.clone();
        #[cfg(not(target_arch = "wasm32"))]
        spawn(async move {
            loop {
                let mut event_receiver = event_receiver.lock().await;
                tokio::select! {
                    Some(command) = self.command_rx.next() => self.handle_command(command).await,
                    event = event_receiver.next() => self.handle_event(event).await,
                    else => break,
                }
            }
        });

        #[cfg(target_arch = "wasm32")]
        spawn(async move {
            loop {
                let mut event_receiver = event_receiver.lock().await;
                futures::select! {
                    command = self.command_rx.select_next_some() => self.handle_command(command).await,
                    event = event_receiver.next() => self.handle_event(event).await,
                    complete => break,
                }
            }
        })
    }

    async fn handle_command(&mut self, command: Command) {
        match command {
            Command::SubmitJob { job, task_updates_channel, response } => {
                let _ = self.submit_job(&job, task_updates_channel).await;
                let _ = response.send(true);
            },
            Command::UpdateTask { task } => {
                let _ = self.peer.client.publish(task.job_id.clone(), serialize_into_vec(&task).expect("can't serialize task update")).await;
            }
            Command::MonitorJobs { response } => {
                let _ = response.send(self.monitor_jobs().await);
            }
        }
    }

    async fn handle_event(&mut self, e: Option<event::Event>) {
        match e {
            Some(event::Event::PubSubMessageReceivedEvent { topic, message, from }) => {
                let mut task = deserialize_from_slice::<task::Task>(&message).expect("can't deserialize task");
                if let Some(tx) = self.jobs.get_mut(&topic) {
                    if let Err(e) = tx.send(TaskUpdateEvent {
                        topic: topic.clone(),
                        from: from,
                        result: TaskUpdateResult::Ok(task.clone()),
                    }).await {
                        if SendError::is_disconnected(&e) {
                            self.jobs.remove(&topic);
                            return;
                        }
                        task.status = Status::FAILED;
                        if let Err(e) = tx.send(TaskUpdateEvent {
                            topic: topic.clone(),
                            from: from,
                            result: TaskUpdateResult::Ok(task.clone()),
                        }).await {
                            tracing::error!("Error sending failed task update: {:?}", e);
                            if SendError::is_disconnected(&e) {
                                self.jobs.remove(&topic);
                                return;
                            }
                        }
                        // // TODO: send failed task update with error
                        // self.peer.publish(topic.to_string().clone(), serialize_into_vec(&task).expect("can't serialize task update")).await.unwrap();
                    }
                }
            }
            Some(event::Event::NewNodeRegistered { node }) => {
                tracing::info!("New node registered: {:?}", node.name);
            }
            _ => {}
        }
    }

    async fn submit_job(&mut self, job: &JobRequest, mut tx: Sender<TaskUpdateEvent>) {
        let response = request_response::<JobRequest, SubmitJobResponse>(self.peer.client.clone(), &self.manager, "/jobs/v1", job, 0).await;
        match response {
            Ok(response) => {
                self.peer.client.subscribe(response.job_id.clone()).await.unwrap();
                self.jobs.insert(TopicHash::from_raw(response.job_id.clone()), tx);
            }
            Err(e) => {
                tracing::error!("Error submitting job: {:?}", e);
                tx.close_channel();
            }
        }
    }

    async fn monitor_jobs(&mut self) -> Receiver<Job> {
        let (mut tx, rx) = channel::<Job>(3072);
        let mut stream = self.peer.client.send("ack".as_bytes().to_vec(), self.manager.clone(), "/monitor/v1".to_string(), 0).await.expect("monitor jobs");

        spawn(async move {
            loop {
                let mut size_buffer = [0u8; 4];
                if let Err(e) = stream.read_exact(&mut size_buffer).await {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        tx.close_channel();
                        break;
                    }
                    tracing::error!("Error reading size: {:?}", e);
                    continue;
                }
                let size = u32::from_be_bytes(size_buffer);
                let mut message_buffer = vec![0u8; size as usize];
                stream.read_exact(&mut message_buffer).await.expect("can't read message");
                let job = deserialize_from_slice::<Job>(&message_buffer).expect("can't deserialize job");
                tx.send(job).await.expect("can't send job to monitor");
            }
        });

        rx
    }
}

#[derive(Clone)]
pub struct DomainCluster {
    sender: Sender<Command>,
    pub peer: Networking,
    pub manager_id: String,
}

impl DomainCluster {
    pub fn new(
        manager_addr: String,
        node_name: String,
        join_as_relay: bool,
        port: u16,
        enable_websocket: bool,
        enable_webrtc: bool,
        private_key: Option<Vec<u8>>,
        private_key_path: Option<String>,
        relays: Vec<String>,
    ) -> Self {
        let networking = Networking::new(&NetworkingConfig {
            bootstrap_nodes: vec![manager_addr.clone()],
            relay_nodes: relays,
            private_key,
            private_key_path,
            enable_mdns: false,
            enable_kdht: true,
            enable_relay_server: join_as_relay,
            name: node_name,
            port,
            enable_websocket,
            enable_webrtc,
            namespace: None,
        }).unwrap();
        let domain_manager_id = manager_addr.split("/").last().unwrap().to_string();

        let (tx, rx) = channel::<Command>(3072);
        let dc = InnerDomainCluster {
            manager: domain_manager_id.clone(),
            peer: networking.clone(),
            jobs: HashMap::new(),
            command_rx: rx,
        };
        dc.init();

        DomainCluster {
            sender: tx,
            peer: networking.clone(),
            manager_id: domain_manager_id.clone(),
        }
    }

    pub async fn submit_job(&mut self, job: &JobRequest) -> Receiver<TaskUpdateEvent> {
        let (tx, rx) = oneshot::channel::<bool>();
        let (updates_tx, updates_rx) = channel::<TaskUpdateEvent>(3072);
        let cmd = Command::SubmitJob {
            job: job.clone(),
            response: tx,
            task_updates_channel: updates_tx,
        };
        self.sender.send(cmd).await.unwrap_or_else(|_| panic!("can't send command {}", job.name));
        let _ = rx.await.unwrap_or_else(|_| panic!("can't wait for response {}", job.name));
        updates_rx
    }

    pub async fn monitor_jobs(&mut self) -> Receiver<Job> {
        let (tx, rx) = oneshot::channel::<Receiver<Job>>();
        let cmd = Command::MonitorJobs {
            response: tx,
        };
        self.sender.send(cmd).await.expect("can't send command");
        rx.await.expect("can't wait for response")
    }

    pub async fn fail_task(&mut self, task: &task::Task, err: Error) {
        let mut t = task.clone();
        t.status = Status::FAILED;
        t.output = Some(task::Any {
            type_url: "Error".to_string(),
            value: serialize_into_vec(&task::Error {
                message: format!("{:?}", err),
            }).unwrap(),
        });
        self.sender.send(Command::UpdateTask  {
            task: t,
        }).await.expect("can't send command"); 
    }

    // pub async fn request_response(&mut self, message: Vec<u8>, peer_id: String, protocol: String, timeout: u32) -> Result<Stream, Box<dyn std::error::Error + Send + Sync>>
}
