use libp2p::{gossipsub::TopicHash, PeerId};
use networking::{context::{self, Context}, event};
use futures::{channel::{mpsc::{channel, Receiver, SendError, Sender}, oneshot}, AsyncReadExt, AsyncWriteExt, FutureExt, SinkExt, StreamExt};
use crate::protobuf::task::{self, Job, Status, SubmitJobResponse, Task};
use std::collections::HashMap;
use quick_protobuf::{deserialize_from_slice, serialize_into_vec};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Debug)]
pub enum TaskUpdateResult {
    Ok(task::Task),
    Err(Box<dyn std::error::Error + Send + Sync>),
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
    peer: Box<context::Context>,
    jobs: HashMap<TopicHash, Sender<TaskUpdateEvent>>,
}

enum Command {
    SubmitJob {
        job: Job,
        task_updates_channel: Sender<TaskUpdateEvent>,
        response: oneshot::Sender<bool>,
    },
    UpdateTask {
        task: task::Task,
    }
}

impl InnerDomainCluster {
    fn init(mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    command = self.command_rx.select_next_some() => self.handle_command(command).await,
                    event = self.peer.poll() => self.handle_event(event).await,
                    else => break,
                }
            }
        });

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                futures::select! {
                    command = self.command_rx.select_next_some() => self.handle_command(command).await,
                    event = self.peer.poll().fuse() => self.handle_event(event).await,
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
                let _ = self.peer.publish(task.job_id.clone(), serialize_into_vec(&task).expect("can't serialize task update")).await;
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
                            eprintln!("Error sending failed task update: {:?}", e);
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
                tracing::debug!("New node registered: {:?}", node.name);
            }
            _ => {}
        }
    }

    async fn submit_job(&mut self, job: &Job, tx: Sender<TaskUpdateEvent>) {
        let res = self.peer.send(serialize_into_vec(job).expect("can't serialize job"), self.manager.clone(), "/jobs/v1".to_string(), 0).await;
        if let Err(e) = res {
            // TODO: handle error
            panic!("Error sending task request: {:?}", e); 
        }
        let mut s = res.unwrap();
        s.close().await.expect("can't close stream");

        let mut out = Vec::new();
        let _ = s.read_to_end(&mut out).await.expect("can't read from stream");
        let job = deserialize_from_slice::<SubmitJobResponse>(&out).expect("can't deserialize job"); 

        self.subscribe_to_job(job.job_id, tx).await
    }

    async fn subscribe_to_job(&mut self, job_id: String, tx: Sender<TaskUpdateEvent>) {
        self.peer.subscribe(job_id.clone()).await.unwrap();

        self.jobs.insert(TopicHash::from_raw(job_id.clone()), tx);
    }
}

#[derive(Clone)]
pub struct DomainCluster {
    sender: Sender<Command>,
}

impl DomainCluster {
    pub fn new(manager: String, peer: Box<Context>) -> Self {
        #[cfg(not(target_family="wasm"))]
        let _ = tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();

        let (tx, rx) = channel::<Command>(100);
        let dc = InnerDomainCluster {
            manager: manager,
            peer: peer,
            jobs: HashMap::new(),
            command_rx: rx,
        };
        dc.init();

        DomainCluster {
            sender: tx,
        }
    }

    pub async fn submit_job(&mut self, job: &Job) -> Receiver<TaskUpdateEvent> {
        let (tx, rx) = oneshot::channel::<bool>();
        let (updates_tx, updates_rx) = channel::<TaskUpdateEvent>(100);
        self.sender.send(Command::SubmitJob {
            job: job.clone(),
            response: tx,
            task_updates_channel: updates_tx,
        }).await.expect("can't send command");
        let _ = rx.await.expect("can't wait for response");
        updates_rx
    }

    // pub async fn update_task(&mut self, task: &task::Task) {
    //     self.sender.send(Command::UpdateTask  {
    //         task: task.clone(),
    //     }).await.expect("can't send command"); 
    // }

    // pub async fn request_response(&mut self, message: Vec<u8>, peer_id: String, protocol: String, timeout: u32) -> Result<Stream, Box<dyn std::error::Error + Send + Sync>>
}
