use std::{collections::{HashMap, VecDeque}, error::Error, sync::Arc, time::{Duration, SystemTime}};

use domain::protobuf::task::{self, mod_ResourceRecruitment as ResourceRecruitment, Status, Task, TaskRequest};
use futures::AsyncWriteExt;
use libp2p::Stream;
use quick_protobuf::{deserialize_from_slice, serialize_into_vec};
use tokio::{sync::mpsc::{self, Receiver}, task::JoinHandle};
use tokio::{sync::Mutex, spawn};
use crate::nodes_management::NodesManagement;

fn parse_duration_to_millis(input: &str) -> Result<u32, Box<dyn Error + Send + Sync>> {
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

#[derive(Debug)]
enum TaskAction {
    Start,
    Queue,
    Retry ,
    Done {
        node_mgmt: NodesManagement,
    },
    Fail,
    Add,
}

#[derive(Clone, Debug)]
pub struct TaskHandler {
    pub task: Task,
    pub dependencies: HashMap<String, bool>,
    in_degrees: usize,
    pub capability_filters: task::CapabilityFilters,
    pub resource_recruitment: task::ResourceRecruitment,
    pub job_id: String,
    pub timeout: u32,
    pub input: Option<task::Any>,
    retries: u32,
    pub updated_at: SystemTime,
    pub created_at: SystemTime,
    pub node_request: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl TaskHandler {
    pub fn ready(&self) -> bool {
        self.in_degrees == 0 && !self.task.receiver.is_empty()
    }
    pub async fn failed(&mut self, err_msg: &str) {
        self.task.status = Status::FAILED;
        self.task.output = Some(task::Any {
            type_url: "Error".to_string(),
            value: serialize_into_vec(&task::Error {
                message: err_msg.to_string(),
            }).unwrap(),
        });
        self.updated_at = SystemTime::now();
        if let Some(req) = self.node_request.lock().await.take() {
            req.abort();
        }
    }
    pub async fn done(&mut self) {
        self.task.status = Status::DONE;
        self.updated_at = SystemTime::now();
        if let Some(req) = self.node_request.lock().await.take() {
            req.abort();
        }
    }
    pub async fn resource_recruited(&mut self, node_id: &str) {
        self.task.receiver = node_id.to_string();
        self.updated_at = SystemTime::now();
        self.node_request.lock().await.take();
    }
}

#[derive(Clone, Debug)]
pub struct TasksManagement {
    pub tasks: Arc<Mutex<HashMap<String, TaskHandler>>>,
    pub task_queue: Arc<Mutex<VecDeque<String>>>,
    pub max_retries: u32,
}

pub fn task_id(job_id: &str, task_name: &str) -> String {
    format!("{}-{}", job_id, task_name)
}

#[derive(Debug)]
pub enum TaskManagementError {
    TaskNotFound(String),
    NodeNotFound(String),
    TaskAlreadyExists,
    RetryLimitReached,
    OtherError(Box<dyn Error + Send + Sync>),
}
impl Error for TaskManagementError {}
impl std::fmt::Display for TaskManagementError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TaskManagementError::TaskNotFound(err) => write!(f, "Task not found: {}", err),
            TaskManagementError::NodeNotFound(err) => write!(f, "Node not found: {}", err),
            TaskManagementError::TaskAlreadyExists => write!(f, "Task already exists"),
            TaskManagementError::OtherError(err) => write!(f, "Task Error: {}", err),
            TaskManagementError::RetryLimitReached => write!(f, "Retry limit reached"),
        }
    }
}

impl TasksManagement {
    pub fn new() -> Self {
        TasksManagement {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
            max_retries: 3,
        }
    }

    #[tracing::instrument]
    pub async fn validate_task(&self, node_mgmt: &mut NodesManagement, job_id: &str, task_req: &TaskRequest) -> Result<bool, TaskManagementError> {
        let mut task_handler = TaskHandler {
            task: Task {
                name: task_req.name.clone(),
                receiver: task_req.receiver.clone(),
                endpoint: task_req.capability_filters.clone().unwrap().endpoint.clone(),
                access_token: "".to_string(), // TODO: generate access token
                job_id: job_id.to_string(),
                sender: task_req.sender.clone(),
                status: Status::WAITING_FOR_RESOURCE,
                output: None,
            },
            capability_filters: task_req.capability_filters.clone().get_or_insert(task::CapabilityFilters {
                endpoint: "/".to_string(),
                min_cpu: 0,
                min_gpu: 0,
            }).clone(),
            resource_recruitment: task_req.resource_recruitment.clone().get_or_insert(task::ResourceRecruitment {
                recruitment_policy: ResourceRecruitment::RecruitmentPolicy::FAIL,
                termination_policy: ResourceRecruitment::TerminationPolicy::KEEP,
            }).clone(),
            job_id: job_id.to_string(),
            timeout: parse_duration_to_millis(&task_req.timeout).map_err(|e| TaskManagementError::OtherError(e))?,
            input: task_req.data.clone(),
            dependencies: HashMap::new(),
            in_degrees: 0,
            retries: 0,
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            node_request: Arc::new(Mutex::new(None)),
        };

        let id = task_id(job_id, &task_req.name);

        // validate task name
        {
            let tasks = self.tasks.lock().await;
            if tasks.contains_key(&id) {
                tracing::warn!("Task {} already exists", id);
                let th = tasks.get(&id).unwrap();
                if th.task.status == Status::WAITING_FOR_RESOURCE {
                    return Ok(th.ready());
                }
                drop(tasks);
                if let Err(e) = self.task_state_machine(&id, TaskAction::Add).await {
                    return Err(e);
                }
                let tasks = self.tasks.lock().await;
                task_handler = tasks.get(&id).unwrap().clone();
            }
        }

        // validate dependencies
        if !task_req.needs.is_empty() {
            let mut dependencies = HashMap::new();
            let needs_clone = task_req.needs.clone();
            let tasks = self.tasks.lock().await;
            for need in needs_clone.iter() {
                let need_id = task_id(job_id, need);
                if !tasks.contains_key(&need_id) {
                    return Err(TaskManagementError::TaskNotFound(format!("Invalid dependency: {}", need_id)));
                }
                dependencies.insert(need_id.clone(), false);
            }
            task_handler.in_degrees = dependencies.len();
            task_handler.dependencies = dependencies;
        }

        // validate recruitment policy
        if task_handler.in_degrees == 0 {
            if let Err(e) = self.recruit_node(&mut task_handler, node_mgmt).await {
                return Err(e);
            }
        }
        let ready = task_handler.ready();
        let mut tasks = self.tasks.lock().await;
        tasks.insert(id.clone(), task_handler);

        Ok(ready)
    }

    #[tracing::instrument]
    pub async fn remove_job(&self, job_id: &str) {
        let mut tasks = self.tasks.lock().await;
        let mut to_remove = Vec::new();
        for (id, task) in tasks.iter_mut() {
            if task.job_id == job_id {
                task.failed("Job cancelled").await;
                to_remove.push(id.clone());
            }
        }
        for id in to_remove {
            tasks.remove(&id);
        }
    }

    #[tracing::instrument]
    pub async fn get_task(&self, task_id: &str) -> Option<TaskHandler> {
        let tasks = self.tasks.lock().await;
        tasks.get(task_id).cloned()
    }

    #[tracing::instrument]
    pub async fn get_next_task(&self) -> Option<TaskHandler> {
        let mut task_queue = self.task_queue.lock().await;
        let task_id = task_queue.pop_front()?;
        self.get_task(&task_id).await
    }

    // TODO: change TaskUpdateEvent to TaskAction
    #[tracing::instrument]
    pub async fn update_task(&self, task: &Task, node_mgmt: NodesManagement) {
        let key = task_id(&task.job_id, &task.name);
        
        let mut tasks = self.tasks.lock().await;
        match tasks.get_mut(&key) {
            Some(task_handler) => {
                task_handler.task = task.clone();
                let status = task.status;
                drop(tasks);
                match status {
                    Status::RETRY => {
                        let _ = self.task_state_machine(&key, TaskAction::Retry).await;
                    }
                    Status::FAILED => {
                        let _ = self.task_state_machine(&key, TaskAction::Fail).await;
                    }
                    Status::DONE => {
                        let _ = self.task_state_machine(&key, TaskAction::Done {node_mgmt}).await;
                    }
                    _ => {}
                }
            }
            None => {
                tracing::warn!("Task {} not found", key);
            }
        }
    }

    #[tracing::instrument]
    pub async fn push_tasks(&self, tasks: Vec<String>) {
        let mut task_queue = self.task_queue.lock().await;
        task_queue.extend(tasks);
    }

    #[tracing::instrument]
    pub async fn retry_task(&self, task_id: &str) {
        let _ = self.task_state_machine(task_id, TaskAction::Retry).await;
    }

    #[tracing::instrument]
    pub async fn monitor_tasks(&self, mut stream: Stream) {
        let tasks = self.tasks.clone();
        spawn(async move {
            let tasks = tasks.lock().await;
            for (_, task) in tasks.iter() {
                let mut err_msg = "".to_string();
                if task.task.status == Status::FAILED {
                    if let Some(output) = task.task.output.as_ref() {
                        if output.type_url == "Error" {
                            let err = deserialize_from_slice::<task::Error>(output.value.as_ref()).unwrap();
                            err_msg = err.message;
                        }
                    }
                }
                let task_handler = task::TaskHandler {
                    task: task.task.clone(),
                    dependencies: task.dependencies.clone(),
                    job_id: task.job_id.clone(),
                    retries: task.retries,
                    updated_at: task.updated_at.elapsed().unwrap().as_millis() as u64,
                    created_at: task.created_at.elapsed().unwrap().as_millis() as u64,
                    err_msg,
                };
                stream.write_all(&serialize_into_vec(&task_handler).unwrap()).await.expect("Failed to send task handler");
            }
        });
    }

    async fn task_state_machine(&self, key: &str, action: TaskAction) -> Result<(), TaskManagementError> {
        let mut tasks = self.tasks.lock().await;
        if !tasks.contains_key(key) {
            return Err(TaskManagementError::TaskNotFound(format!("Task {} not found", key)));
        }
        let task = tasks.get_mut(key).unwrap();
        let ready = task.in_degrees == 0;
        println!("Task {} state machine: {:?}, {:?}", key, action, task.task.status);
        match action {
            TaskAction::Add => {
                match task.task.status {
                    Status::STARTED | Status::PROCESSING | Status::PENDING => {
                        return Err(TaskManagementError::TaskAlreadyExists);
                    }
                    Status::WAITING_FOR_RESOURCE => {
                        return Ok(());
                    }
                    _ => {
                        drop(tasks);
                        return Box::pin(self.task_state_machine(key, TaskAction::Retry)).await;
                    }
                }
            }
            TaskAction::Retry => {
                match task.task.status {
                    Status::RETRY | Status::FAILED => {
                        if task.retries >= self.max_retries {
                            task.task.status = Status::FAILED;
                            task.updated_at = SystemTime::now();
                            task.task.output = Some(task::Any {
                                type_url: "Error".to_string(),
                                value: b"Task failed after 3 retries".to_vec(),
                            });
                            return Err(TaskManagementError::RetryLimitReached);
                        }
                        if ready && task.task.receiver.is_empty() {
                            task.task.status = Status::WAITING_FOR_RESOURCE;
                            task.updated_at = SystemTime::now();
                            return Ok(());
                        }
                        if ready && !task.task.receiver.is_empty(){
                            task.task.status = Status::PENDING;
                            task.updated_at = SystemTime::now();
                            drop(tasks);
                            self.task_queue.lock().await.push_back(key.to_string());
                            return Ok(());
                        }
                        if !ready {
                            task.task.status = Status::WAITING_FOR_RESOURCE;
                            task.updated_at = SystemTime::now();
                            return Ok(());
                        }
                        return Err(TaskManagementError::TaskAlreadyExists);
                    }
                    _ => {
                        return Err(TaskManagementError::TaskAlreadyExists);
                    }
                }
            }
            TaskAction::Queue => {
                if task.task.status == Status::STARTED || task.task.status == Status::PROCESSING || task.task.status == Status::PENDING {
                    return Err(TaskManagementError::TaskAlreadyExists);
                }
                if ready && !task.task.receiver.is_empty() {
                    task.updated_at = SystemTime::now();
                    drop(tasks);
                    self.task_queue.lock().await.push_back(key.to_string());
                    return Ok(());
                }
                if ready && task.task.receiver.is_empty() && task.task.status != Status::WAITING_FOR_RESOURCE {
                    task.task.status = Status::WAITING_FOR_RESOURCE;
                    task.updated_at = SystemTime::now();
                    return Ok(());
                }
                return Err(TaskManagementError::TaskAlreadyExists);
            }
            TaskAction::Start => {
                if task.task.status != Status::PENDING {
                    return Err(TaskManagementError::TaskAlreadyExists);
                }
                task.task.status = Status::STARTED;
                task.updated_at = SystemTime::now();
                return Ok(());
            }
            TaskAction::Done {mut node_mgmt} => {
                if task.task.status != Status::PROCESSING && task.task.status != Status::STARTED {
                    return Err(TaskManagementError::TaskAlreadyExists);
                }
                task.task.status = Status::DONE;
                task.updated_at = SystemTime::now();
                drop(tasks);
                let mut tasks = self.tasks.lock().await;
                for (_, handler) in tasks.iter_mut() {
                    if let Some(completed) = handler.dependencies.get_mut(key) {
                        if *completed == false {
                            *completed = true;
                            handler.in_degrees-=1;
                            if handler.in_degrees == 0 {
                                if let Err(e) = self.recruit_node(handler, &mut node_mgmt).await {
                                    handler.failed(&e.to_string()).await;
                                    continue;
                                }
                                if handler.ready() {
                                    let mut task_queue = self.task_queue.lock().await;
                                    task_queue.push_back(task_id(&handler.job_id, &handler.task.name));
                                }
                            }
                        }
                    }
                }
                return Ok(());
            }
            TaskAction::Fail => {
                if task.task.status == Status::FAILED {
                    return Err(TaskManagementError::TaskAlreadyExists);
                }
                task.task.status = Status::FAILED;
                task.updated_at = SystemTime::now();
                for (_, handler) in tasks.iter_mut() {
                    if handler.dependencies.contains_key(key) {
                        let _ = Box::pin(self.task_state_machine(&task_id(&handler.job_id, &handler.task.name), TaskAction::Fail)).await;
                    }
                }
                return Ok(());
            }
        }
    }

    async fn recruit_node(&self, task_handler: &mut TaskHandler, node_mgmt: &mut NodesManagement) -> Result<(), TaskManagementError> {
        let recruit_policy = task_handler.resource_recruitment.recruitment_policy;
        let capapbilities = task_handler.capability_filters.clone();
        let node = node_mgmt.find_node(capapbilities).await;
        match node {
            Some(node) => {
                task_handler.task.receiver = node.id.clone();
            }
            None => {
                if recruit_policy == ResourceRecruitment::RecruitmentPolicy::FAIL {
                    let err_msg = format!("No nodes found for task: {}", task_handler.task.name);
                    task_handler.failed(&err_msg).await;
                    return Err(TaskManagementError::NodeNotFound(err_msg));
                }
                let mut node_mgmt = node_mgmt.clone();
                let endpoint = task_handler.capability_filters.endpoint.clone();
                let task_queue = self.task_queue.clone();
                let tasks = self.tasks.clone();
                let id = task_id(&task_handler.job_id, &task_handler.task.name);
                task_handler.task.receiver = "".to_string();
                task_handler.node_request = Arc::new(Mutex::new(Some(spawn(async move {
                    let rx = node_mgmt.request_node(&endpoint).await;
                    if let Ok(node_id) = rx.await {
                        let mut tasks = tasks.lock().await;
                        let task = tasks.get_mut(&id).expect("Task not found");
                        task.resource_recruited(&node_id).await;
                        let mut task_queue = task_queue.lock().await;
                        task_queue.push_back(id.clone());
                    } else {
                        let mut tasks = tasks.lock().await;
                        let task = tasks.get_mut(&id).expect("Task not found");
                        task.failed("Failed to recruit node").await;
                    }
                }))));
            }
        }
        Ok(())
    }
}
