use jsonwebtoken::{encode, EncodingKey, Header};
use libp2p::Stream;
use networking::{client::Client, event, libp2p::{Networking, NetworkingConfig, Node}};
use nodes_management::NodesManagement;
use quick_protobuf::{deserialize_from_slice, serialize_into_vec};
use tasks_management::{task_id, TaskHandler, TaskPendingRequest, TasksManagement};
use tokio::{self, select, spawn, time::sleep};
use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
use std::{error::Error, time::{Duration, SystemTime, UNIX_EPOCH}};
use domain::{message::{prefix_size_message, read_prefix_size_message}, protobuf::task::{self, Code, GlobalRefinementInputV1, JobRequest, LocalRefinementOutputV1, Status}};
use sha2::{Digest, Sha256};
use hex;
use serde::{Serialize, Deserialize};
mod tasks_management;
mod nodes_management;

#[derive(Debug, Serialize, Deserialize)]
struct TaskTokenClaim {
    domain_id: String,
    task_name: String,
    job_id: String,
    sender: String,
    receiver: String,
    exp: usize,
    iat: usize,
    sub: String,
}

fn encode_jwt(domain_id: &str, job_id: &str, task_name: &str, sender: &str, receiver: &str, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
    let exp = now + Duration::from_secs(60*60);
    let claims = TaskTokenClaim {
        domain_id: domain_id.to_string(),
        task_name: task_name.to_string(),
        sender: sender.to_string(),
        receiver: receiver.to_string(),
        job_id: job_id.to_string(),
        // TODO: set exp, iat, sub and scope
        exp: exp.as_secs() as usize,
        iat: 0,
        sub: "".to_string(),
    };

    // TODO: use ed25519 key
    let token = encode(
        &Header::default(),
        &claims,
           &EncodingKey::from_secret(secret.as_ref()),
    )?;

    Ok(token)
}

async fn handshake_then_content(mut peer: Client, access_token: &str, receiver: &str, endpoint: &str, content: Vec<u8>, timeout: u32) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut upload_stream = peer.send(prefix_size_message(&task::DomainClusterHandshake{
        access_token: access_token.to_string(),
    }), receiver.to_string(), endpoint.to_string(), timeout).await?;

    // check if handshake succeed

    upload_stream.write_all(&content).await?;
    upload_stream.flush().await?;
    Ok(())
}

#[derive(Clone, Debug)]
struct DomainManager {
    peer: Networking,
    domain_id: String,
    task_mgmt: TasksManagement,
    node_mgmt: NodesManagement,
}

impl DomainManager {
    fn new(domain_id: String, peer: Networking) -> Self {
        let _ = tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
        DomainManager {
            peer,
            domain_id,
            task_mgmt: TasksManagement::new(),
            node_mgmt: NodesManagement::new(),
        }
    }

    #[tracing::instrument]
    async fn start(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let event_receiver = self.peer.event_receiver.clone();
        let mut job_handler = self.peer.client.set_stream_handler("/jobs/v1".to_string()).await.unwrap();
        // let mut monitor_jobs_handler = self.peer.client.set_stream_handler("/monitor/jobs/v1".to_string()).await.unwrap();

        loop {
            let mut rx_guard = event_receiver.lock().await;
            select! {
                Some((_, stream)) = job_handler.next() => {
                    let task_mgmt = self.task_mgmt.clone();
                    let node_mgmt = self.node_mgmt.clone();
                    let peer = self.peer.clone();
                    spawn(DomainManager::accept_job(node_mgmt, task_mgmt, peer, stream));
                }
                e = rx_guard.next() => {
                    match e {
                        Some(e) => {
                            match e {
                                event::Event::NewNodeRegistered { node } => {
                                    let mut node_mgmt = self.node_mgmt.clone();
                                    if node.id != self.peer.id {
                                        spawn(async move {
                                            node_mgmt.register_node(node).await;
                                        });
                                    }
                                }
                                event::Event::PubSubMessageReceivedEvent { from, message, .. } => {
                                    if from.is_none() || from.unwrap().to_string() != self.peer.id.clone() {
                                        let task_event = deserialize_from_slice::<task::Task>(&message)?;
                                        let task_mgmt = self.task_mgmt.clone();
                                        spawn(async move {
                                            task_mgmt.update_task(task_event).await
                                        });
                                    }
                                }
                            }
                        }
                        None => break
                    }
                }
                next_task = self.task_mgmt.get_next_task() => {
                    match next_task {
                        Some(task) => {
                            let task_mgmt = self.task_mgmt.clone();
                            let domain_id = self.domain_id.clone();
                            let peer = self.peer.clone();
                            spawn(async move {
                                DomainManager::run_task(&domain_id, peer, &task, task_mgmt).await;
                            });
                        }
                        None => sleep(Duration::from_secs(5)).await
                    }
                }
                else => break
            }
        }
        Ok(())
    }

    #[tracing::instrument]
    async fn accept_job(node_mgmt: NodesManagement, task_mgmt: TasksManagement, mut peer: Client, stream: Stream) {
        let (reader, mut writer) = stream.split();
        let job = read_prefix_size_message::<JobRequest>(reader).await.expect("failed to load job request");

        let mut hasher = Sha256::new();
        hasher.update(&serialize_into_vec(&job).unwrap());
        let result = hasher.finalize();
        let job_id = hex::encode(result);
        println!("Job received: {:?}-{}", job.name, job_id);

        let mut resp = task::SubmitJobResponse {
            job_id: job_id.clone(),
            code: task::Code::Accepted,
            err_msg: "".to_string(),
        };
        writer.write_all(&prefix_size_message(&resp)).await?;
        writer.flush().await?;
        self.peer.subscribe(job_id.clone()).await?;

        let mut tasks: Vec<TaskPendingRequest> = Vec::new();
        for task_req in job.tasks {
            let task_mgmt = task_mgmt.clone();
            let mut node_mgmt = node_mgmt.clone();
            let job_id = job_id.clone();
            let res = task_mgmt.validate_task(&mut node_mgmt, &job_id, &task_req).await;
            if let Err(err) = res {
                tracing::error!("Error adding task: {:?}", err);
                resp.code = Code::BadRequest;
                resp.err_msg = err.to_string();
                writer.write_all(&prefix_size_message(&resp)).await.expect("failed to write job submittion response");
                writer.flush().await.expect("failed to flush result");
                for handler in tasks {
                    if let Some(node_request) = handler.node_request {
                        node_request.abort();
                    }
                }
                task_mgmt.remove_job(&job_id).await;
                return;
            } else {
                let handler = res.unwrap();
                if handler.dependencies_ready && handler.node_request.is_none() {
                    tasks.push(handler);
                }
            }
        }
        writer.write_all(&prefix_size_message(&resp)).await.expect("failed to write job submittion response");
        writer.flush().await.expect("failed to flush result");
        for handler in tasks {
            task_mgmt.push_tasks(vec![handler.task_id]).await;
        }
    }

    #[tracing::instrument]
    async fn run_task(domain_id:&str, mut peer: Networking, th: &TaskHandler, task_mgmt: TasksManagement) {
        let mut serialized_input: Vec<u8> = vec![];
        let mut t = th.task.clone();
        let input = th.input.clone();
        let dependency_list = th.dependencies.clone();

        match input {
            Some(input) => {
                let type_name = input.type_url.clone(); // TODO: should have from_any to convert Any to specific type
                match type_name.as_str() {
                    "GlobalRefinementInputV1" => {
                        if input.value.is_empty() {
                            let global_refinement_input = GlobalRefinementInputV1 {
                                local_refinement_results: {
                                    let mut dependencies: Vec<LocalRefinementOutputV1> = Vec::new();
                                    for (k, v) in dependency_list {
                                        if v == false {
                                            tracing::error!("Task {} failed to run due to dependency {}", t.name, k);
                                            return;
                                        }
                                        match task_mgmt.get_task(&k).await {
                                            Some(task) => {
                                                match task.task.output {
                                                    Some(output) => {
                                                        let output = deserialize_from_slice(&output.value).expect("failed to deserialize output");
                                                        dependencies.push(output);
                                                    }
                                                    None => {
                                                        tracing::error!("Task {} failed to run due to missing dependency {}", t.name, k);
                                                        return;
                                                    }
                                                }
                                            }
                                            None => {
                                                tracing::error!("Task {} failed to run due to missing dependency {}", t.name, k);
                                                return;
                                            }
                                        }
                                    }
                                    dependencies
                                }
                            };
                            serialized_input = prefix_size_message(&global_refinement_input);
                        } else {
                            serialized_input = Vec::with_capacity(4 + input.value.len());
                            let size = input.value.len() as u32;
                            let size_buffer = size.to_be_bytes();
                            serialized_input.extend_from_slice(&size_buffer);
                            serialized_input.append(&mut input.value.clone());
                        }
                    },
                    "LocalRefinementInputV1" => {
                        serialized_input = Vec::with_capacity(4 + input.value.len());
                        let size = input.value.len() as u32;
                        let size_buffer = size.to_be_bytes();
                        serialized_input.extend_from_slice(&size_buffer);
                        serialized_input.append(&mut input.value.clone());
                    },
                    _ => {}
                }
            }
            None => {}
        }
        
        let access_token = encode_jwt(domain_id, &t.job_id, &t.name, &t.sender, &t.receiver, "secret").expect("failed to encode jwt");
        t.status = Status::PENDING;
        if t.sender == peer.id {
            if let Err(e) = handshake_then_content(peer, &access_token, &t.receiver, &t.endpoint, serialized_input, th.timeout).await {
                tracing::error!("Error triggering task: {:?}", e);
                task_mgmt.push_tasks(vec![task_id(&t.job_id, &t.name)]).await;
            } else {
                task_mgmt.update_task(t).await;
            }
        } else {
            t.access_token = access_token;
            if let Err(e) = peer.publish(t.job_id.clone(), serialize_into_vec(&t).unwrap()).await {
                tracing::error!("Error publishing message for task job {} {}: {:?}", t.job_id, t.name, e);
                task_mgmt.push_tasks(vec![task_id(&t.job_id, &t.name)]).await;
                return;
            }
            task_mgmt.update_task(t).await; 
        }
    }
}

/*
    * This is a simple example of a domain_manager node. It will connect to a set of bootstraps and accept tasks.
    * Usage: cargo run --package domain-manager <port> <name> [private_key_path]
    * Example: cargo run --package domain-manager 18804 domain_manager 
 */
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        println!("Arguments received: {:?}", args);
        args.push("18800".to_string());
        args.push("domain_manager".to_string());
        args.push("xxx".to_string());
        // println!("Missing arguments, Usage: {} <port> <name> <domain_id> [private_key_path]", args[0]);
        // return Ok(());
    }
    let port = args[1].parse::<u16>().unwrap();
    let name = args[2].clone();
    let domain_id = args[3].clone();
    let base_path = format!("./volume/{}", name);
    let mut private_key_path = format!("{}/pkey", base_path);
    if args.len() == 5 {
        private_key_path = args[4].clone();
    }

    let cfg = &NetworkingConfig{
        port,
        bootstrap_nodes: vec![],
        enable_relay_server: false,
        enable_kdht: true,
        enable_mdns: false,
        relay_nodes: vec![],
        private_key: None,
        private_key_path: Some(private_key_path),
        name,
    };
    let c = Networking::new(cfg)?;
    let mut domain_manager = DomainManager::new(&domain_id, c);
    
    domain_manager.start().await
}
