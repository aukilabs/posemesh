use jsonwebtoken::{encode, EncodingKey, Header};
use libp2p::{gossipsub::{PublishError, TopicHash}, Stream};
use networking::{client::Client, event, libp2p::{Networking, NetworkingConfig, Node}};
use quick_protobuf::{deserialize_from_slice, serialize_into_slice, serialize_into_vec};
use serde::de;
use tokio::{self, select, signal, spawn, time::sleep};
use futures::{channel::mpsc::{channel, Receiver, Sender}, lock::Mutex, AsyncReadExt, AsyncWriteExt, SinkExt, StreamExt};
use std::{any::Any, borrow::BorrowMut, collections::{HashMap, VecDeque}, error::Error, hash::Hash, sync::{atomic::AtomicUsize, Arc}, time::{Duration, SystemTime, UNIX_EPOCH}};
use domain::{message::{prefix_size_message, read_prefix_size_message}, protobuf::task::{self, mod_ResourceRecruitment as ResourceRecruitment, GlobalRefinementInputV1, Job, JobRequest, LocalRefinementOutputV1, Status, Task}};
use sha2::{Digest, Sha256};
use hex;
use std::fs;
use serde::{Serialize, Deserialize};

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

pub async fn start_task(task: &task::Task,  c: &mut Client) -> Result<Stream, Box<dyn Error + Send + Sync>> {
    let m_buf = serialize_into_vec(&task::DomainClusterHandshake{
        access_token: task.access_token.clone(),
    })?;
    let mut length_buf = [0u8; 4];
    let length = m_buf.len() as u32;
    length_buf.copy_from_slice(&length.to_be_bytes());
    let mut upload_stream = c.send(length_buf.to_vec(), task.receiver.clone(), task.endpoint.clone(), 0).await.expect(&format!("cant send handshake with {}", task.receiver.clone()));
    upload_stream.write_all(&m_buf).await.expect("cant write handshake");
    upload_stream.flush().await.expect("cant flush handshake");

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

#[derive(Debug, Clone)]
struct Queue<T> {
    sender: Sender<T>,
    pub(crate) receiver: Arc<Mutex<Receiver<T>>>,
}

impl<T> Queue<T> {
    // Initialize the queue and start the background worker
    pub fn new() -> Self {
        let (sender, receiver) = channel::<T>(3072);

        Self { sender, receiver: Arc::new(Mutex::new(receiver)) }
    }

    // Add a task to the queue
    pub async fn add(&mut self, datum: T) {
        self.sender.send(datum).await.unwrap();
    }

    // Get the next task from the queue
    pub async fn next(&mut self) -> Option<T> {
        let mut receiver = self.receiver.lock().await;
        receiver.next().await
    }
}

#[derive(Clone, Debug)]
struct JobHandler {
    id: String,
    name: String,
    tasks: Arc<Mutex<HashMap<String, task::Task>>>,
}

impl JobHandler {
    // Add or update a task to the job, return true if the task is added successfully, false if the task already exists
    async fn add_task(&mut self, task: task::Task) -> bool {
        match self.tasks.lock().await.insert(task.name.clone(), task) {
            Some(_) => false,
            None => true,
        }
    }

    async fn get_task(&self, name: &str) -> Option<task::Task> {
        let tasks = self.tasks.lock().await;
        tasks.get(name).cloned()
    }

    async fn contains_task(&self, name: &str) -> bool {
        let tasks = self.tasks.lock().await;
        tasks.contains_key(name)
    }
}

#[derive(Clone)]
struct DomainManager {
    peer: Networking,
    task_req_queue: Queue<TaskHandler>,
    jobs: Arc<Mutex<HashMap<String, JobHandler>>>,
    nodes: Arc<Mutex<HashMap<String, Node>>>,
    task_updates_notifiers: Arc<Mutex<Vec<Sender<Job>>>>,
    capabilities: Arc<Mutex<HashMap<String, Vec<String>>>>,
    node_indices: Arc<Mutex<HashMap<String, AtomicUsize>>>,
    domain_id: String,
}

impl DomainManager {
    fn new(domain_id: &str, peer: Networking) -> Self {
        let _ = tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
        DomainManager {
            peer,
            task_req_queue: Queue::new(),
            jobs: Arc::new(Mutex::new(HashMap::new())),
            nodes: Arc::new(Mutex::new(HashMap::new())),
            task_updates_notifiers: Arc::new(Mutex::new(vec![])),
            capabilities: Arc::new(Mutex::new(HashMap::new())),
            node_indices: Arc::new(Mutex::new(HashMap::new())),
            domain_id: domain_id.to_string(),
        }
    }

    async fn start(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let event_receiver = self.peer.event_receiver.clone();
        let mut job_handler = self.peer.client.set_stream_handler("/jobs/v1".to_string()).await.unwrap();
        let mut monitor_jobs_handler = self.peer.client.set_stream_handler("/monitor/jobs/v1".to_string()).await.unwrap();

        loop {
            let mut rx_guard = event_receiver.lock().await;
            select! {
                Some((_, stream)) = job_handler.next() => {
                    let mut this = self.clone();
                    spawn(async move {
                        let _ = this.accept_job(stream).await;
                    });
                }
                Some((_, mut stream)) = monitor_jobs_handler.next() => {
                    let (mut job_updates_tx, mut job_updates_rx) = channel::<Job>(3072);
                    // write existing jobs to job_updates_tx
                    let jobs = self.jobs.lock().await;
                    for (_, j) in jobs.iter() {
                        job_updates_tx.send(Job {
                            id: j.id.clone(),
                            name: j.name.clone(),
                            tasks: j.tasks.lock().await.iter().map(|th| Task {
                                name: th.0.clone(),
                                status: th.1.status.clone(),
                                output: th.1.output.clone(),
                                receiver: th.1.receiver.clone(),
                                endpoint: th.1.endpoint.clone(),
                                access_token: "".to_string(),
                                job_id: th.1.job_id.clone(),
                                sender: th.1.sender.clone(),
                            }).collect(),
                        }).await.expect("Error sending job update");
                    }
                    self.task_updates_notifiers.lock().await.push(job_updates_tx);
                    spawn(async move {
                        loop {
                            let job = job_updates_rx.next().await;
                            if let Some(job) = job {
                                stream.write_all(&prefix_size_message(&job)).await.expect("Error writing job update");
                                stream.flush().await.expect("Error flushing job update");
                            }
                        }
                    });
                }
                e = rx_guard.next() => {
                    match e {
                        Some(e) => {
                            match e {
                                event::Event::NewNodeRegistered { node } => {
                                    self.nodes.lock().await.insert(node.id.clone(), node.clone());
                                    for capability in node.capabilities.iter() {
                                        self.capabilities.lock().await.entry(capability.clone()).or_insert(vec![]).push(node.id.clone());
                                        self.node_indices.lock().await.entry(capability.clone()).or_insert(AtomicUsize::new(0));
                                    }
                                }
                                event::Event::PubSubMessageReceivedEvent { from, message, .. } => {
                                    if from.is_none() || from.unwrap().to_string() != self.peer.id.clone() {
                                        let task_event = deserialize_from_slice::<task::Task>(&message)?;
                                        let jobs = self.jobs.clone();
                                        let task_updates_notifiers = self.task_updates_notifiers.clone();
                                        // TODO: verify access token, ignore if not valid. Ignore expiration time
                                        spawn(async move {
                                            let mut jobs = jobs.lock().await;
                                            if let Some(mut j) = jobs.get(&task_event.job_id).cloned() {
                                                if let Some(mut t) = j.get_task(&task_event.name).await {
                                                    t.status = task_event.status;
                                                    t.output = task_event.output;
                                                    println!("Task {} status updated: {:?}, sender: {:?}, receiver: {:?}", t.name, t.status, t.sender, t.receiver);
                                                    j.add_task(t.clone()).await;
                                                }
                                                jobs.insert(task_event.job_id.clone(), j.clone());
                                                spawn(async move {
                                                    let mut notifiers = task_updates_notifiers.lock().await;
                                                    for notifier in notifiers.iter_mut() {
                                                        let job = Job {
                                                            id: j.id.clone(),
                                                            name: j.name.clone(),
                                                            tasks: j.tasks.lock().await.iter().map(|th| Task {
                                                                name: th.0.clone(),
                                                                status: th.1.status.clone(),
                                                                output: th.1.output.clone(),
                                                                receiver: th.1.receiver.clone(),
                                                                endpoint: th.1.endpoint.clone(),
                                                                access_token: "".to_string(),
                                                                job_id: th.1.job_id.clone(),
                                                                sender: th.1.sender.clone(),
                                                            }).collect(),
                                                        };
                                                        notifier.send(job).await.expect("Error sending task update");
                                                    }
                                                });
                                            }
                                        });
                                    }
                                }
                            }
                        }
                        None => break
                    }
                }
                Some(task_handler) = self.task_req_queue.next() => {
                    let mut this_clone = self.clone();
                    spawn(async move {
                        let _ = this_clone.poll_task_req(&task_handler).await;
                    });
                }
                else => break
            }
        }
        Ok(())
    }

    async fn find_node(&mut self, capability_filter: task::CapabilityFilters) -> Option<Node> {
        let node_ids = {
            let capabilities = self.capabilities.lock().await;
            capabilities.get(&capability_filter.endpoint).cloned()
        };

        match node_ids {
            Some(node_ids) => {
                let index = self.node_indices.lock().await.get(&capability_filter.endpoint).unwrap().load(std::sync::atomic::Ordering::Relaxed) % node_ids.len();
                let node_id = node_ids.get(index).unwrap();
                let nodes = self.nodes.lock().await;
                let node = nodes.get(node_id);
                match node {
                    Some(node) => {
                        self.node_indices.lock().await.get(&capability_filter.endpoint).unwrap().fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        return Some(node.clone());
                    }
                    None => None
                }
            }
            None => None
        }
    }

    async fn accept_job(&mut self, stream: Stream) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (reader, mut writer) = stream.split();
        let job = read_prefix_size_message::<JobRequest>(reader).await?;

        let mut hasher = Sha256::new();
        hasher.update(&serialize_into_vec(&job).unwrap());
        let result = hasher.finalize();
        let job_id = hex::encode(result);
        println!("Job received: {:?}-{}", job.name, job_id);

        let mut job_handler = JobHandler {
            id: job_id.clone(),
            name: job.name.clone(),
            tasks: Arc::new(Mutex::new(HashMap::new())),
        };

        let resp = task::SubmitJobResponse {
            job_id: job_id.clone(),
            code: task::Code::Accepted,
            err_msg: "".to_string(),
        };
        writer.write_all(&prefix_size_message(&resp)).await?;
        writer.flush().await?;
        self.peer.client.subscribe(job_id.clone()).await?;

        for task_req in job.tasks.iter() {
            let mut task = task::Task {
                name: task_req.name.clone(),
                receiver: "".to_string(),
                endpoint: task_req.capability_filters.clone().unwrap().endpoint.clone(),
                access_token: "".to_string(), // TODO: generate access token
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
                    if !job_handler.contains_task(need).await {
                        panic!("Task not found: {:?}", need); // TODO: handle error
                    }
                }
            }
            if !task_req.receiver.is_empty() {
                // TODO: validate receiver
                task.receiver = task_req.receiver.clone();
                task.status = task::Status::PENDING;
            } else {
                let node = self.find_node(task_req.capability_filters.clone().unwrap()).await;
                match node {
                    Some(node) => {
                        task.receiver = node.id.clone();
                        task.status = task::Status::PENDING;
                    }
                    None => {
                        if task_handler.resource_recruitment.recruitment_policy == ResourceRecruitment::RecruitmentPolicy::FAIL {
                            eprintln!("No nodes found for task: {:?}", task_req); // TODO: handle error
                            break;
                        }
                    }
                }
            }
            self.task_req_queue.add(task_handler).await;
            
            if !job_handler.add_task(task.clone()).await {
                panic!("Task already exists: {:?}", task.name); // TODO: handle error
            }
        }

        self.jobs.lock().await.insert(job_id.clone(), job_handler);
        Ok(())
    }

    async fn run_task(&mut self, t: &task::Task, input: Option<task::Any>, dependencies: HashMap<String, task::Task>, timeout: u32) -> Result<(), Box<dyn Error + Send + Sync>> {
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
        
        let mut t_clone = t.clone();
        t_clone.access_token = encode_jwt(&self.domain_id, &t.job_id, &t.name, &t.sender, &t.receiver, "secret")?;
        t_clone.status = Status::PENDING;
        if t.sender == self.peer.id {
            let mut s = start_task(&t_clone, &mut self.peer.client).await?;
            let mut length_buf = [0u8; 4];
            let length = serialized_input.len() as u32;
            length_buf.copy_from_slice(&length.to_be_bytes());
            s.write_all(length_buf.to_vec().as_slice()).await?;
            s.write_all(&serialized_input).await?;
            s.flush().await?;
            println!("Sending task: {:?}", t_clone);
        } else {
            if let Err(e) = self.peer.client.publish(t.job_id.clone(), serialize_into_vec(&t_clone)?).await {
                println!("Error publishing message for task job {} {}: {:?}", t.job_id, t.name, e);
                return Err(e);
            }
        }
        Ok(())
    }

    // TODO: handle task failure
    async fn poll_task_req(&mut self, task_handler: &TaskHandler) -> Result<(), Box<dyn Error + Send + Sync>> {
        let try_job = self.jobs.lock().await.get(&task_handler.job_id).cloned();
        if try_job.is_none() {
            return Ok(());
        }
        let mut j = try_job.unwrap();
        println!("Polling task: {:?}", task_handler.name);

        if let Some(mut t) = j.get_task(&task_handler.name).await {
            println!("Polled task: {:?}", t);
            // TODO: adding sleep so we wont check the same task again and again too often
            if t.status == task::Status::WAITING_FOR_RESOURCE {
                let node = self.find_node(task_handler.capability_filters.clone()).await;
                match node {
                    Some(node) => {
                        t.receiver = node.id.clone();
                        t.status = task::Status::PENDING;
                        println!("Task {} status updated: {:?}, sender: {:?}, receiver: {:?}", t.name, t.status, t.sender, t.receiver);
                    }
                    None => {
                        self.task_req_queue.add(task_handler.clone()).await;
                        return Ok(());
                    }
                }
            }
            if t.status == task::Status::PENDING {
                let mut dependencies = HashMap::<String, task::Task>::new();
                for need in task_handler.needs.iter() {
                    if let Some(dependency) = j.get_task(need).await {
                        println!("Dependency: {} {:?}", dependency.name, dependency.status);
                        if dependency.status != task::Status::DONE && dependency.status != task::Status::FAILED {
                            let task_handler = task_handler.clone();
                            let mut task_req_queue = self.task_req_queue.clone();
                            spawn(async move {
                                sleep(std::time::Duration::from_secs(30)).await;
                                task_req_queue.add(task_handler).await;
                            });
                            return Ok(());
                        }
                        if dependency.status == task::Status::FAILED {
                            t.status = task::Status::FAILED;
                            self.peer.publish(t.job_id.clone(), serialize_into_vec(&t)?).await.expect("failed to publish task update");
                            println!("Task {} status updated: {:?}, sender: {:?}, receiver: {:?}", t.name, t.status, t.sender, t.receiver);
                        }
                        dependencies.insert(need.clone(), dependency.clone());
                    }
                }
                if t.status == task::Status::PENDING {
                    if let Err(e) = self.run_task(&t, task_handler.input.clone(), dependencies, task_handler.timeout).await {
                        println!("Error running task: {:?}", e);
                        // TODO: not every failure worths retry
                        t.status = task::Status::PENDING;
                        let mut task_req_queue = self.task_req_queue.clone();
                        let task_handler = task_handler.clone();
                        spawn(async move {
                            sleep(std::time::Duration::from_secs(5)).await;
                            task_req_queue.add(task_handler).await;
                        });
                    }
                }
                let _ = j.add_task(t.clone()).await;
                self.jobs.lock().await.insert(task_handler.job_id.clone(), j.clone());
                return Ok(());
            } else {
                self.task_req_queue.add(task_handler.clone()).await;
            }
        }
        self.task_req_queue.add(task_handler.clone()).await;
        Ok(())
    }
}

/*
    * This is a simple example of a domain_manager node. It will connect to a set of bootstraps and accept tasks.
    * Usage: cargo run --package domain-manager <port> <name> [private_key_path]
    * Example: cargo run --package domain-manager 18804 domain_manager 
 */
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} <port> <name> <domain_id> [private_key_path]", args[0]);
        return Ok(());
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
