use libp2p::{gossipsub::TopicHash, PeerId};
use networking::{context::{self, Context, Config}, event};
use futures::{channel::mpsc::{channel, Receiver, SendError, Sender}, AsyncReadExt, AsyncWriteExt, SinkExt, StreamExt};
use protobuf::task::{self,Job, Status};
use std::collections::HashMap;
use quick_protobuf::{deserialize_from_slice, serialize_into_vec, BytesReader};

pub enum TaskUpdateResult {
    Ok(task::Task),
    Err(Box<dyn std::error::Error + Send + Sync>),
}

pub struct TaskUpdateEvent {
    pub topic: TopicHash,
    pub from: Option<PeerId>,
    pub result: TaskUpdateResult,
}

const MAX_MESSAGE_SIZE_BYTES: usize = 1024 * 1024 * 10;

struct InnerDomainCluster {
    command_rx: Receiver<Command>,
    manager: String,
    peer: Box<context::Context>,
    jobs: HashMap<TopicHash, Sender<TaskUpdateEvent>>,
}

enum Command {
    SubmitJob {
        job: Job,
        response: Sender<TaskUpdateEvent>,
    },
}

impl InnerDomainCluster {
    fn init(mut self) {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(Command::SubmitJob { job, response }) = self.command_rx.next() => {
                        // TODO: error handling
                        let _ = self.submit_job(&job, response).await;
                    }
                    e = self.peer.poll() => {
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
                                            break;
                                        }
                                        task.status = Status::FAILED;
                                        let _ = tx.send(TaskUpdateEvent {
                                            topic: topic.clone(),
                                            from: from,
                                            result: TaskUpdateResult::Ok(task.clone()),
                                        }).await;
                                        // // TODO: send failed task update with error
                                        // self.peer.publish(topic.to_string().clone(), serialize_into_vec(&task).expect("can't serialize task update")).await.unwrap();
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                
            }
        });
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
        let job = {
            let mut reader = BytesReader::from_bytes(&out);
            reader.read_message::<task::SubmitJobResponse>(&out).expect("can't read job")
        };

        self.subscribe_to_job(job.job_id, tx).await
    }

    async fn subscribe_to_job(&mut self, job_id: String, tx: Sender<TaskUpdateEvent>) {
        self.peer.subscribe(job_id.clone()).await.unwrap();

        self.jobs.insert(TopicHash::from_raw(job_id.clone()), tx);
    }
}

pub struct DomainCluster {
    sender: Sender<Command>,
}

impl DomainCluster {
    pub fn new(manager: String, peer: Box<Context>) -> Self {
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
        let (tx, rx) = channel::<TaskUpdateEvent>(100);
        self.sender.send(Command::SubmitJob {
            job: job.clone(),
            response: tx,
        }).await.expect("can't send command");
        rx
    }
}
