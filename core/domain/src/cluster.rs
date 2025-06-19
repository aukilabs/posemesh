use async_trait::async_trait;
use libp2p::PeerId;
use futures::{channel::{mpsc::{unbounded, UnboundedReceiver, UnboundedSender}, oneshot}, lock::Mutex, Sink, SinkExt, Stream, StreamExt};
#[cfg(not(target_family="wasm"))]
use posemesh_networking::AsyncStream;
use posemesh_networking::{client::Client, event, libp2p::{Networking, NetworkingConfig}};
use posemesh_utils::retry_with_delay;

#[cfg(not(target_family="wasm"))]
use posemesh_disco::client::DiscoClient;

#[cfg(not(target_family = "wasm"))]
use tokio::time::sleep;
#[cfg(target_family = "wasm")]
use posemesh_utils::sleep;

use crate::{datastore::common::DomainError, message::{prefix_size_message, read_prefix_size_message, request_response}, protobuf::{common::Capability, discovery::{JoinClusterRequest, JoinClusterResponse, Node}, task::{self, JobRequest, SubmitJobResponse, Task}}};
use std::{collections::HashMap, pin::Pin, sync::Arc, task::{Context, Poll}, time::Duration};
use quick_protobuf::deserialize_from_slice;
use posemesh_networking::client::TClient;

#[cfg(not(target_family = "wasm"))]
use tokio::spawn;
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

pub const UNLIMITED_CAPACITY: i32 = -1;
pub const JOIN_CLUSTER_PROTOCOL_V1: &str = "/join/v1";
pub const SUBMIT_JOB_PROTOCOL_V1: &str = "/jobs/v1";

// pub trait TaskUpdateHandler: Send {
//     fn on_task_update(&self, t: &Task);
// }

pub struct TaskUpdatesSink {
    sender: UnboundedSender<Task>,
}

pub struct TaskUpdatesStream {
    receiver: UnboundedReceiver<Task>,
}

impl TaskUpdatesStream {
    pub fn new(receiver: UnboundedReceiver<Task>) -> Self {
        TaskUpdatesStream { receiver }
    }
}

pub fn new_task_update_duplex() -> (TaskUpdatesSink, TaskUpdatesStream) {
    let (sender, receiver) = unbounded();
    (TaskUpdatesSink { sender }, TaskUpdatesStream { receiver })
}

impl Stream for TaskUpdatesStream {
    type Item = Task;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_next_unpin(cx)
    }
}

impl Sink<Task> for TaskUpdatesSink {
    type Error = DomainError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.sender.poll_ready(cx).map_err(|e| DomainError::SendCommandError(e))
    }

    fn start_send(mut self: Pin<&mut Self>, item: Task) -> Result<(), Self::Error> {
        self.sender.start_send(item).map_err(|e| DomainError::SendCommandError(e))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.sender.close_channel();
        Poll::Ready(Ok(()))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl TaskUpdatesSink {
    fn on_task_update(&self, t: &Task) {
        let mut sender = self.sender.clone();
        let task = t.clone();
        spawn(async move {
            if let Err(e) = sender.send(task).await {
                tracing::error!("Failed to send task update: {:?}", e);
            }
        });
    }
}

async fn join(manager_id: &str, client: Client, id: &str, name: &str, capabilities: &[Capability]) -> Result<JoinClusterResponse, DomainError> {
    request_response::<JoinClusterRequest, JoinClusterResponse>(
        client,
        manager_id,
        JOIN_CLUSTER_PROTOCOL_V1,
        &JoinClusterRequest {
            node: Node {
                id: id.to_string(),
                name: name.to_string(),
                capabilities: capabilities.to_vec(),
            }
        },
        15000
    ).await
}
use futures::channel::mpsc::{self, Sender, Receiver};

#[derive(Clone)]
pub struct DomainCluster {
    command_tx: Sender<DomainCommand>,
    pub manager_id: String,
    pub peer: Networking,
}

enum DomainCommand {
    SubmitJob(JobRequest, TaskUpdatesSink, oneshot::Sender<Result<(), DomainError>>),
    MonitorJobs(TaskUpdatesSink, oneshot::Sender<Result<(), DomainError>>),
    OnEvent(event::Event),
    OnTaskUpdate(Task),
}

struct InnerDomainCluster {
    manager_id: String,
    manager: String,
    peer: Networking,
    name: String,
    command_rx: Receiver<DomainCommand>,
    jobs: Arc<Mutex<HashMap<String, TaskUpdatesSink>>>,
    capabilities: Vec<Capability>,
}

impl DomainCluster {
    pub fn init(manager_id: String, manager: String, peer: Networking, name: String, capabilities: Vec<Capability>) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let mut inner = InnerDomainCluster {
            manager_id: manager_id.clone(),
            manager,
            peer: peer.clone(),
            name,
            command_rx: rx,
            jobs: Arc::new(Mutex::new(HashMap::new())),
            capabilities,
        };

        spawn(async move {
            inner.run().await;
        });

        Self {
            command_tx: tx,
            manager_id,
            peer
        }
    }

    pub async fn submit_job(&mut self, job: &JobRequest, handler: TaskUpdatesSink) -> Result<(), DomainError> {
        let (response_tx, response_rx) = oneshot::channel();
        self.command_tx.send(DomainCommand::SubmitJob(job.clone(), handler, response_tx)).await?;
        response_rx.await.map_err(|e| DomainError::Cancelled("submit job", e))?
    }

    pub async fn monitor_jobs(&mut self, handler: TaskUpdatesSink) -> Result<(), DomainError> {
        let (response_tx, response_rx) = oneshot::channel();
        self.command_tx.send(DomainCommand::MonitorJobs(handler, response_tx)).await?;
        response_rx.await.map_err(|e| DomainError::Cancelled("monitor jobs", e))?
    }
}

impl InnerDomainCluster {
    async fn handle_command(&mut self, cmd: DomainCommand) -> Result<(), DomainError> {
        match cmd {
            DomainCommand::SubmitJob(job, handler, response) => {
                let result = async {
                    let response = request_response::<JobRequest, SubmitJobResponse>(
                        self.peer.client.clone(), 
                        &self.manager_id,
                        SUBMIT_JOB_PROTOCOL_V1,
                        &job,
                        0
                    ).await?;
                    
                    let job_id = response.job_id.clone();
                    self.peer.client.subscribe(job_id.clone()).await?;
                    tracing::debug!("Subscribed to job: {:?}", job_id);
                    let mut jobs = self.jobs.lock().await;
                    jobs.insert(job_id, handler);
                    Ok(())
                }.await;
                let _ = response.send(result);
                Ok(())
            }

            DomainCommand::MonitorJobs(handler, response) => {
                let result = async {
                    let mut stream = self.peer.client.send(
                        "ack".as_bytes().to_vec(),
                        &self.manager_id,
                        "/monitor/v1",
                        0
                    ).await?;

                    spawn(async move {
                        loop {
                            match read_prefix_size_message::<task::Task, _>(&mut stream).await {
                                Ok(task) => {
                                    handler.on_task_update(&task);
                                }
                                Err(e) => {
                                    if let quick_protobuf::Error::Io(e) = e {
                                        if e.kind() == std::io::ErrorKind::UnexpectedEof {
                                            break;
                                        }
                                        tracing::error!("Error loading tasks update from domain manager: {:?}", e);
                                        break;
                                    }
                                    tracing::error!("Error loading tasks update from domain manager: {:?}", e);
                                    break;
                                }
                            }
                        }
                    });

                    Ok(())
                }.await;
                let _ = response.send(result);
                Ok(())
            }

            DomainCommand::OnEvent(e) => {
                match e {
                    event::Event::NodeUnregistered { node_id } => {
                        if node_id == self.manager_id {
                            let name = self.name.clone();
                            let jobs = self.jobs.clone();
                            let manager_id = self.manager_id.clone();
                            let client = self.peer.client.clone();
                            let capabilities = self.capabilities.clone();
                            let peer_id = self.peer.id.clone();

                            spawn(async move {
                                use posemesh_utils::INFINITE_RETRIES;

                                let mut jobs = jobs.lock().await;
                                jobs.clear();

                                let _ = posemesh_utils::retry_with_delay(move || {
                                    let manager_id = manager_id.clone();
                                    let client = client.clone();
                                    let name = name.clone();
                                    let capabilities = capabilities.clone();
                                    let peer_id = peer_id.clone();
                                    Box::pin(async move {
                                        join(&manager_id, client, &peer_id, &name, &capabilities).await
                                    })
                                },
                                    INFINITE_RETRIES,
                                    Duration::from_secs(60)
                                ).await;
                            });
                        }
                    }
                    _ => {}
                }
                Ok(())
            }

            DomainCommand::OnTaskUpdate(t) => {
                let mut jobs = self.jobs.lock().await;
                if let Some(handler) = jobs.get_mut(&t.job_id) {
                    handler.on_task_update(&t);
                }
                Ok(())
            }
        }
    }

    pub async fn run(&mut self) {
        while let Some(cmd) = self.command_rx.next().await {
            if let Err(e) = self.handle_command(cmd).await {
                tracing::error!("Error handling command: {:?}", e);
            }
        }
    }
}

#[async_trait]
pub trait EventHandler: Send {
    fn handle_event(&mut self, e: event::Event);
    fn handle_task_update(&mut self, t: &Task); 
}

impl EventHandler for DomainCluster {
    fn handle_event(&mut self, e: event::Event) {
        let _ = self.command_tx.try_send(DomainCommand::OnEvent(e));
    }

    fn handle_task_update(&mut self, t: &Task) {
        let _ = self.command_tx.try_send(DomainCommand::OnTaskUpdate(t.clone()));
    }
}

pub fn validate_pubsub_message(topic: &str, message: &[u8], from: Option<PeerId>) -> Result<Task, DomainError> {
    let mut task = deserialize_from_slice::<task::Task>(&message)?;
    if from.is_none() {
        tracing::error!("Received task update from unknown peer: {:?}", task);
        return Err(DomainError::InvalidPubsubMessage("task update from unknown peer"));
    }
    if task.receiver.get_or_insert("".to_string()).to_string() != from.unwrap().to_string() && task.sender != from.unwrap().to_string() {
        tracing::error!("Received task update for wrong receiver: {:?}", task);
        return Err(DomainError::InvalidPubsubMessage("task update for wrong receiver")); 
    }
    if task.job_id != topic.to_string() {
        tracing::error!("Received task update for wrong job: {:?}", task);
        return Err(DomainError::InvalidPubsubMessage("task update for wrong job"));
    }
    Ok(task)
}

#[derive(Clone)]
pub struct PosemeshSwarm {
    pub peer: Networking,

    addresses: Arc<Mutex<Vec<String>>>,
    expected_addresses_length: usize,
    addresses_ready: Arc<Mutex<Option<oneshot::Sender<Vec<String>>>>>,
    #[cfg(not(target_family="wasm"))]
    disco_client: Arc<Mutex<Option<DiscoClient>>>,

    domains: Arc<Mutex<HashMap<String, Box<dyn EventHandler>>>>,
    capabilities: Arc<Mutex<Vec<Capability>>>,
}

impl PosemeshSwarm {
    #[cfg(not(target_family="wasm"))]
    pub async fn as_node(&mut self, disco_url: &str, wallet_private_key: Option<&str>, wallet_private_key_path: Option<&str>, registration_secret: &str, capabilities: &[Capability]) -> Result<Vec<impl futures::Stream<Item = (PeerId, impl AsyncStream)>>, DomainError> {
        let disco_client = DiscoClient::new(wallet_private_key, wallet_private_key_path, disco_url, registration_secret).await?;
        self.disco_client = Arc::new(Mutex::new(Some(disco_client)));

        self.with_capabilities(capabilities).await
    }
    pub async fn init(
        join_as_relay: bool,
        port: u16,
        enable_websocket: bool,
        enable_webrtc: bool,
        private_key: Option<String>,
        private_key_path: Option<String>,
        relays: Vec<String>,
    ) -> Result<PosemeshSwarm, DomainError> {
        let networking = Networking::new(&NetworkingConfig {
            bootstrap_nodes: vec![],
            relay_nodes: relays,
            private_key,
            private_key_path,
            enable_mdns: false,
            enable_kdht: true,
            enable_relay_server: join_as_relay,
            port,
            enable_websocket,
            enable_webrtc,
            namespace: None,
        })?;
        #[cfg(not(target_family = "wasm"))]
        let mut expected_addresses_len: usize = 2;
        #[cfg(target_family = "wasm")]
        let mut expected_addresses_len: usize = 0;
        if enable_webrtc {
            expected_addresses_len+=1;
        }
        if enable_websocket {
            expected_addresses_len+=1;
        }

        let dc = PosemeshSwarm {
            peer: networking,
            #[cfg(not(target_family="wasm"))]
            disco_client: Arc::new(Mutex::new(None)),
            addresses: Arc::new(Mutex::new(Vec::new())),
            expected_addresses_length: expected_addresses_len,
            addresses_ready: Arc::new(Mutex::new(None)),
            domains: Arc::new(Mutex::new(HashMap::new())),
            capabilities: Arc::new(Mutex::new(Vec::new()))
        };

        dc.clone().listen().await;
        Ok(dc)
    }

    async fn listen(mut self) -> Vec<String> {
        let (tx, rx) = oneshot::channel::<Vec<String>>();
        let event_receiver = self.peer.event_receiver.clone();
        if self.expected_addresses_length > 0 {
            self.addresses_ready = Arc::new(Mutex::new(Some(tx)));
        } else {
            let _ = tx.send(vec![]);
        }
        #[cfg(not(target_family = "wasm"))]
        spawn(async move {
            loop {
                let mut event_receiver = event_receiver.lock().await;
                tokio::select! {
                    event = event_receiver.next() => self.handle_event(event).await,
                    else => break,
                }
            }
        });

        #[cfg(target_family = "wasm")]
        spawn(async move {
            loop {
                let event = {
                    let mut event_receiver = event_receiver.lock().await;
                    event_receiver.next().await
                };
                self.handle_event(event).await;
            }
        });

        rx.await.unwrap()
    }

    async fn handle_event(&mut self, e: Option<event::Event>) {
        match e {
            Some(event::Event::PubSubMessageReceivedEvent { topic, message, from }) => {
                let task = validate_pubsub_message(&topic.to_string(), &message, from);
                if let Ok(task) = task {
                    let domains = self.domains.clone();
                    spawn(async move {
                        let mut domains = domains.lock().await;
                        domains.iter_mut().for_each(|(_, d)| d.handle_task_update(&task));
                    });
                }
            }
            Some(event::Event::NewAddress { address }) => {
                let mut addresses = self.addresses.lock().await;
                addresses.push(address.to_string());
                if addresses.len() == self.expected_addresses_length {
                    if let Some(tx) = self.addresses_ready.lock().await.take() {
                        let _ = tx.send(addresses.clone());
                    }
                }
            }
            Some(e) => {
                let domains = self.domains.clone();
                spawn(async move {
                    let mut domains = domains.lock().await;
                    domains.iter_mut().for_each(|(_, d)| d.handle_event(e.clone()));
                });
            }
            _ => {}
        }
    }

    #[cfg(not(target_family="wasm"))]
    async fn with_capabilities(&mut self, capabilities: &[Capability]) -> Result<Vec<impl futures::Stream<Item = (PeerId, impl AsyncStream)>>, DomainError> {
        let mut disco_client_lock = self.disco_client.lock().await;
        if let Some(disco_client) = disco_client_lock.as_mut() {
            // TODO: can't add capabilities here, because they are using different protobuf builders
            disco_client.register_compatible(self.addresses.lock().await.clone(), &vec![]).await?;
            drop(disco_client_lock);
            let mut streams = Vec::new();
            for capability in capabilities {
                let stream = self.peer.client.set_stream_handler(&capability.endpoint).await?;
                streams.push(stream);
            }
            Ok(streams)
        } else {
            Err(DomainError::RegisterCapabilityError("Disco client not found"))
        }
    }
    // TODO: should take domain_id, and disco will find the manager by id
    pub async fn join_domain(&mut self, domain_manager_id: &str, dc: impl EventHandler + 'static) -> Result<(), DomainError> {  
        let mut domains = self.domains.lock().await;
        domains.insert(domain_manager_id.to_string(), Box::new(dc));

        Ok(())
    }
}

pub async fn join_domain(swarm: &mut PosemeshSwarm, domain_manager: &str, name: &str) -> Result<DomainCluster, DomainError> {
    let domain_manager_id = if domain_manager.split('/').last().is_none() {
        return Err(DomainError::InvalidManagerAddress(domain_manager.to_string()));
    } else {
        domain_manager.split('/').last().unwrap()
    };

    let peer = swarm.peer.clone();

    let capabilities = swarm.capabilities.clone();
    let capabilities = capabilities.lock().await;
    let dc = DomainCluster::init(domain_manager_id.to_string(), domain_manager.to_string(), peer.clone(), name.to_string(), capabilities.clone());
    let mut parsed_addresses = HashMap::new();
    parsed_addresses.insert(domain_manager_id.to_string(), vec![domain_manager.to_string()]);
    swarm.peer.client.bootstrap(parsed_addresses).await?;
    sleep(Duration::from_secs(10)).await;
    let _ = join(domain_manager_id, peer.client.clone(), &peer.id, name, &capabilities).await?;
    drop(capabilities);
    swarm.join_domain(domain_manager_id, dc.clone()).await?;
    Ok(dc)
}
