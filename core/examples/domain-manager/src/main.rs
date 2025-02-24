use jsonwebtoken::{encode, EncodingKey, Header};
use libp2p::{gossipsub::{PublishError, TopicHash}, Stream};
use networking::{context, event, network::{self, Node}};
use quick_protobuf::{deserialize_from_slice, serialize_into_slice, serialize_into_vec};
use serde::de;
use tokio::{self, select, signal, time::sleep};
use futures::{channel::mpsc::{channel, Receiver, Sender}, AsyncReadExt, AsyncWriteExt, StreamExt, SinkExt};
use std::{any::Any, borrow::BorrowMut, collections::{HashMap, VecDeque}, error::Error, hash::Hash, time::{Duration, SystemTime, UNIX_EPOCH}};
use protobuf::task::{self, GlobalRefinementInputV1, LocalRefinementOutputV1, mod_ResourceRecruitment as ResourceRecruitment};
use sha2::{Digest, Sha256};
use hex;
use std::fs;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct TaskTokenClaim {
    task_name: String,
    job_id: String,
    sender: String,
    receiver: String,
    exp: usize,
    iat: usize,
    sub: String,
}

fn encode_jwt(job_id: &str, task_name: &str, sender: &str, receiver: &str, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
    let exp = now + Duration::from_secs(60*60);
    let claims = TaskTokenClaim {
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

pub async fn start_task(task: &mut task::Task,  c: &mut context::Context) -> Result<Stream, Box<dyn Error + Send + Sync>> {
    let m_buf = serialize_into_vec(&task::DomainClusterHandshake{
        access_token: task.access_token.clone(),
    })?;
    let mut length_buf = [0u8; 4];
    let length = m_buf.len() as u32;
    length_buf.copy_from_slice(&length.to_be_bytes());
    let mut upload_stream = c.send(length_buf.to_vec(), task.receiver.clone(), task.endpoint.clone(), 0).await.expect(&format!("cant send handshake with {}", task.receiver.clone()));
    upload_stream.write_all(&m_buf).await.expect("cant write handshake");
    upload_stream.flush().await.expect("cant flush handshake");
    task.status = task::Status::STARTED;

    let task_update = serialize_into_vec(task).expect("cant serialize task update");
    c.publish(task.job_id.clone(), task_update).await.expect("cant publish task update");

    Ok(upload_stream)
}

fn parse_duration_to_millis(input: &str) -> Result<u32, Box<dyn Error>> {
    let (value, unit) = input
        .trim()
        .split_at(input.find(|c: char| c.is_alphabetic()).unwrap_or(input.len()));

    let value: u32 = value.parse()?;
    let millis = match unit {
        "ms" => value,
        "s" => value * 1000,
        "m" => value * 60 * 1000,
        "h" => value * 60 * 60 * 1000,
        "" => value,  // Default to milliseconds if no unit is provided
        _ => return Err(format!("Unsupported unit: {}", unit).into()),
    };

    Ok(millis)
}

#[derive(Debug, Clone)]
struct TaskHandler {
    name: String,
    capability_filters: task::CapabilityFilters,
    resource_recruitment: task::ResourceRecruitment,
    needs: Vec<String>,
    job_id: String,
    timeout: u32,
    input: Option<task::Any>,
}
struct Queue<T> {
    sender: Sender<T>,
    pub(crate) receiver: Receiver<T>,
}

impl<T> Queue<T> {
    // Initialize the queue and start the background worker
    pub fn new() -> Self {
        let (sender, receiver) = channel::<T>(100);

        Self { sender, receiver }
    }

    // Add a task to the queue
    pub async fn add(&mut self, datum: T) {
        self.sender.send(datum).await.unwrap();
    }

    // Get the next task from the queue
    pub async fn next(&mut self) -> Option<T> {
        self.receiver.next().await
    }
}

#[derive(Clone, Debug)]
struct JobHandler {
    id: String,
    tasks: HashMap<String, task::Task>,
}

struct DomainManager {
    peer: context::Context,
    task_req_queue: Queue<TaskHandler>,
    jobs: HashMap<String, JobHandler>,
    nodes: HashMap<String, Node>,
}

impl DomainManager {
    fn new(peer: context::Context) -> Self {
        DomainManager {
            peer: peer,
            task_req_queue: Queue::new(),
            jobs: HashMap::new(),
            nodes: HashMap::new(),
        }
    }

    async fn start(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut job_handler = self.peer.set_stream_handler("/jobs/v1".to_string()).await.unwrap();
        // periodically print latest status of tasks
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

        loop {
            select! {
                Some((_, stream)) = job_handler.next() => {
                    self.accept_job(stream).await?;
                }
                e = self.peer.poll() => {
                    match e {
                        Some(e) => {
                            match e {
                                event::Event::NewNodeRegistered { node } => {
                                    self.nodes.insert(node.id.clone(), node);
                                }
                                event::Event::PubSubMessageReceivedEvent { from, message, .. } => {
                                    if from.is_none() || from.unwrap().to_string() != self.peer.id {
                                        let task_event = deserialize_from_slice::<task::Task>(&message)?;
                                        // TODO: verify access token, ignore if not valid. Ignore expiration time
                                        if let Some(j) = self.jobs.get_mut(&task_event.job_id) {
                                            if let Some(t) = j.tasks.get_mut(&task_event.name) {
                                                t.status = task_event.status;
                                                t.output = task_event.output;
                                                println!("Task {} status updated: {:?}", t.name, t.status);
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        None => break
                    }
                }
                Some(task_handler) = self.task_req_queue.next() => {
                    self.poll_task_req(&task_handler).await?;
                }
                _ = interval.tick() => {
                    println!("##########################");
                    // print task status as a table | Job | Task Name | Status | Receiver | Output |
                    println!("| {:<64} | {:<20} | {:<30} | {:<60} |", "Job", "Task Name", "Status", "Done By");
                    for (job_id, job) in self.jobs.iter() {
                        for (task_name, task) in job.tasks.iter() {
                            println!("| {:<64} | {:<20} | {:<30} | {:<60} |", job_id, task_name, format!("{:?}", task.status), task.receiver);
                        }
                    }
                    println!("##########################");
                }
                else => break
            }
        }
        Ok(())
    }

    fn find_nodes(&mut self, capability_filter: task::CapabilityFilters) -> Vec<Node> {
        self.nodes.values().filter(|n| n.capabilities.contains(&capability_filter.endpoint)).cloned().collect()
    }

    async fn accept_job(&mut self, mut stream: Stream) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut buf = Vec::new();
        let _ = stream.read_to_end(&mut buf).await?;
        let job = deserialize_from_slice::<task::Job>(&buf)?;

        let mut hasher = Sha256::new();
        hasher.update(&buf);
        let result = hasher.finalize();
        let job_id = hex::encode(result);

        let mut job_handler = JobHandler {
            id: job_id.clone(),
            tasks: HashMap::new(),
        };

        let resp = task::SubmitJobResponse {
            job_id: job_id.clone(),
            code: task::Code::Accepted,
            err_msg: "".to_string(),
        };
        stream.write_all(&serialize_into_vec(&resp).unwrap()).await?;
        stream.flush().await?;
        self.peer.subscribe(job_id.clone()).await?;

        for task_req in job.tasks.iter() {
            let mut task = task::Task {
                name: task_req.name.clone(),
                receiver: "".to_string(),
                endpoint: task_req.capability_filters.clone().unwrap().endpoint.clone(),
                access_token: job_id.clone(), // TODO: generate access token
                job_id: job_id.clone(),
                sender: task_req.sender.clone(),
                status: task::Status::WAITING_FOR_RESOURCE,
                output: None,
            };
            let task_handler = TaskHandler {
                name: task.name.clone(),
                capability_filters: task_req.capability_filters.clone().get_or_insert(task::CapabilityFilters {
                    endpoint: "/".to_string(),
                    min_cpu: 0,
                    min_gpu: 0,
                }).clone(),
                resource_recruitment: task_req.resource_recruitment.clone().get_or_insert(task::ResourceRecruitment {
                    recruitment_policy: ResourceRecruitment::RecruitmentPolicy::FAIL,
                    termination_policy: ResourceRecruitment::TerminationPolicy::KEEP,
                }).clone(),
                needs: task_req.needs.clone(),
                job_id: job_id.clone(),
                timeout: parse_duration_to_millis(&task_req.timeout).map_err(|e| format!("Error parsing timeout: {}", e))?,
                input: task_req.data.clone(),
            };
            
            if !task_req.needs.is_empty() {
                for need in task_req.needs.iter() {
                    if !job_handler.tasks.contains_key(need) {
                        panic!("Task not found: {:?}", need); // TODO: handle error
                    }
                }
            }
            if !task_req.receiver.is_empty() {
                // TODO: validate receiver
                task.receiver = task_req.receiver.clone();
                task.status = task::Status::PENDING;
            } else {
                let nodes = self.find_nodes(task_req.capability_filters.clone().unwrap());
                if nodes.is_empty() {
                    if task_handler.resource_recruitment.recruitment_policy == ResourceRecruitment::RecruitmentPolicy::FAIL {
                        panic!("No nodes found for task: {:?}", task_req); // TODO: handle error
                    }
                } else {
                    task.receiver = nodes[0].id.clone();
                    task.status = task::Status::PENDING;

                    // if task_req.needs.is_empty() {
                    //     self.run_task(task.borrow_mut(), task_req.data.clone(), HashMap::new(), task_handler.timeout).await?;
                    // }
                }
            }
            self.task_req_queue.add(task_handler).await;
            
            if let Some(t) = job_handler.tasks.insert(task.name.clone(), task.clone()) {
                panic!("Task already exists: {:?}", t); // TODO: handle error
            }
        }

        self.jobs.insert(job_id.clone(), job_handler);
        Ok(())
    }

    async fn run_task(&mut self, t: &mut task::Task, input: Option<task::Any>, dependencies: HashMap<String, task::Task>, timeout: u32) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut serialized_input: Vec<u8> = vec![];

        match input {
            Some(input) => {
                let type_name = input.type_url.clone(); // TODO: should have from_any to convert Any to specific type
                match type_name.as_str() {
                    "GlobalRefinementInputV1" => {
                        if input.value.is_empty() {
                            let global_refinement_input = GlobalRefinementInputV1 {
                                local_refinement_results: dependencies.iter().map(|(_, v)| deserialize_from_slice::<LocalRefinementOutputV1>(&v.output.as_ref().unwrap().value).expect("failed to deserialize local refinement output")).collect(),
                            };
                            serialized_input = serialize_into_vec(&global_refinement_input).expect("failed to serialize");
                        } else {
                            serialized_input = input.value.clone();
                        }
                    },
                    "LocalRefinementInputV1" => {
                        serialized_input = input.value.clone();
                    },
                    _ => {}
                }
            }
            None => {}
        }
        
        t.access_token = encode_jwt(&t.job_id, &t.name, &t.sender, &t.receiver, "secret")?;
        if t.sender == self.peer.id {
            let mut s = start_task(t, &mut self.peer).await?;
            let mut length_buf = [0u8; 4];
            let length = serialized_input.len() as u32;
            length_buf.copy_from_slice(&length.to_be_bytes());
            s.write_all(length_buf.to_vec().as_slice()).await?;
            s.write_all(&serialized_input).await?;
            s.flush().await?;
        } else {
            if let Err(e) = self.peer.publish(t.job_id.clone(), serialize_into_vec(&t.clone())?).await {
                println!("Error publishing message: {:?}", e);
                return Err(e);
            }
        }
        Ok(())
    }

    // TODO: handle task failure
    async fn poll_task_req(&mut self, task_handler: &TaskHandler) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut jobs = self.jobs.clone();
        if let Some(j) = jobs.get_mut(&task_handler.job_id) {
            if let Some(t) = j.clone().tasks.get_mut(&task_handler.name) {
                // TODO: adding sleep so we wont check the same task again and again too often
                if t.status == task::Status::WAITING_FOR_RESOURCE {
                    let nodes = self.find_nodes(task_handler.capability_filters.clone());
                    if nodes.is_empty() {
                        self.task_req_queue.add(task_handler.clone()).await;
                        return Ok(());
                    } else {
                        t.receiver = nodes[0].id.clone();
                        t.status = task::Status::PENDING;
                    }
                }
                if t.status == task::Status::PENDING {
                    let mut dependencies = HashMap::<String, task::Task>::new();
                    for need in task_handler.needs.iter() {
                        if let Some(j) = self.jobs.get(&task_handler.job_id) {
                            if let Some(t) = j.tasks.get(need) {
                                if t.status != task::Status::DONE {
                                    sleep(std::time::Duration::from_secs(30)).await;
                                    self.task_req_queue.add(task_handler.clone()).await;
                                    return Ok(());
                                }
                                dependencies.insert(need.clone(), t.clone());
                            }
                        }
                    }
                    if let Err(e) = self.run_task(t, task_handler.input.clone(), dependencies, task_handler.timeout).await {
                        println!("Error running task: {:?}", e);
                        // TODO: not every failure worths retry
                        t.status = task::Status::PENDING;
                        sleep(std::time::Duration::from_secs(5)).await;
                        self.task_req_queue.add(task_handler.clone()).await;
                    }
                    j.tasks.insert(t.name.clone(), t.clone());
                    self.jobs.insert(task_handler.job_id.clone(), j.clone());
                    return Ok(());
                } else {
                    self.task_req_queue.add(task_handler.clone()).await;
                }
            }
        }
        self.task_req_queue.add(task_handler.clone()).await;
        Ok(())
    }
}

/*
    * This is a simple example of a domain_manager node. It will connect to a set of bootstraps and accept tasks.
    * Usage: cargo run --example domain_manager --features rust <port> <name> [private_key_path]
    * Example: cargo run --example domain_manager --features rust 18804 domain_manager 
 */
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} <port> <name> [private_key_path]", args[0]);
        return Ok(());
    }
    let port = args[1].parse::<u16>().unwrap();
    let name = args[2].clone();
    let base_path = format!("./volume/{}", name);
    let mut private_key_path = format!("{}/pkey", base_path);
    if args.len() == 4 {
        private_key_path = args[3].clone();
    }

    let cfg = &network::NetworkingConfig{
        port: port,
        bootstrap_nodes: vec![],
        enable_relay_server: false,
        enable_kdht: true,
        enable_mdns: false,
        relay_nodes: vec![],
        private_key: vec![],
        private_key_path: private_key_path,
        name: name,
        // node_capabilities: vec![],
        // node_types: vec!["relay".to_string()],
    };
    let c = context::context_create(cfg)?;
    let mut domain_manager = DomainManager::new(c);
    
    domain_manager.start().await
}
