use std::{collections::{HashMap, HashSet, VecDeque}, error::Error, sync::Arc};

use domain::{cluster::{TaskUpdateEvent, TaskUpdateResult}, protobuf::task::{self, mod_ResourceRecruitment as ResourceRecruitment, Status, Task, TaskRequest}};

use tokio::task::JoinHandle;
#[cfg(not(target_arch = "wasm32"))]
use tokio::{sync::{Mutex, mpsc::channel, oneshot}, spawn};
#[cfg(target_arch = "wasm32")]
use futures::{channel::oneshot, lock::Mutex, channel::mpsc::{channel, Receiver}};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local as spawn;

use crate::nodes_management::{self, NodesManagement};

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
}

pub struct TaskPendingRequest {
    pub task_id: String,
    pub node_request: Option<JoinHandle<()>>,
    pub dependencies_ready: bool,
}

#[derive(Clone, Debug)]
pub struct TasksManagement {
    pub tasks: Arc<Mutex<HashMap<String, TaskHandler>>>,
    pub task_queue: Arc<Mutex<VecDeque<String>>>,
}

pub fn task_id(job_id: &str, task_name: &str) -> String {
    format!("{}-{}", job_id, task_name)
}

#[derive(Debug)]
pub enum TaskManagementError {
    TaskNotFound(String),
    NodeNotFound(String),
    TaskAlreadyExists,
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
        }
    }
}

impl TasksManagement {
    pub fn new() -> Self {
        TasksManagement {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    #[tracing::instrument]
    pub async fn validate_task(&self, node_mgmt: &mut NodesManagement, job_id: &str, task_req: &TaskRequest) -> Result<TaskPendingRequest, TaskManagementError> {
        let mut task_handler = TaskHandler {
            task: Task {
                name: task_req.name.clone(),
                receiver: "".to_string(),
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
        };

        let id = task_id(job_id, &task_req.name);
        let mut ready = true;

        // validate task name
        {
            let tasks = self.tasks.lock().await;
            if tasks.contains_key(&id) {
                return Err(TaskManagementError::TaskAlreadyExists);
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
            ready = false;
        }

        let mut requester = TaskPendingRequest {
            task_id: id.clone(),
            node_request: None,
            dependencies_ready: ready,
        }; 

        // validate recruitment policy
        if !task_req.receiver.is_empty() {
            // TODO: validate receiver
            task_handler.task.receiver = task_req.receiver.clone();
        } else {
            let recruit_policy = task_handler.resource_recruitment.recruitment_policy;
            let capapbilities = task_handler.capability_filters.clone();
            let node = node_mgmt.find_node(capapbilities).await;
            match node {
                Some(node) => {
                    task_handler.task.receiver = node.id.clone();
                }
                None => {
                    if recruit_policy == ResourceRecruitment::RecruitmentPolicy::FAIL && ready {
                        return Err(TaskManagementError::NodeNotFound(format!("No nodes found for task: {}", task_req.name)));
                    }
                    if ready {
                        let mut node_mgmt = node_mgmt.clone();
                        let endpoint = task_handler.capability_filters.endpoint.clone();
                        let task_queue = self.task_queue.clone();
                        let tasks = self.tasks.clone();
                        let id = id.clone();
                        requester.node_request = Some(spawn(async move {
                            let rx = node_mgmt.request_node(&endpoint).await;
                            if let Ok(node_id) = rx.await {
                                let mut tasks = tasks.lock().await;
                                let task = tasks.get_mut(&id).expect("Task not found");
                                task.task.receiver = node_id;
                                let mut task_queue = task_queue.lock().await;
                                task_queue.push_back(id.clone());
                            } else {
                                let mut tasks = tasks.lock().await;
                                let task = tasks.get_mut(&id).expect("Task not found");
                                task.task.status = Status::FAILED; 
                            }
                        }));
                    }
                }
            }
        }

        let mut tasks = self.tasks.lock().await;
        tasks.insert(id.clone(), task_handler);

        Ok(requester)
    }

    #[tracing::instrument]
    pub async fn remove_job(&self, job_id: &str) {
        let mut tasks = self.tasks.lock().await;
        let mut to_remove = Vec::new();
        for (id, task) in tasks.iter() {
            if task.job_id == job_id {
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

    #[tracing::instrument]
    pub async fn update_task(&self, task: Task) {
        let key = task_id(&task.job_id, &task.name);
        let mut tasks = self.tasks.lock().await;
        match tasks.get_mut(&key) {
            Some(task_handler) => {
                task_handler.task = task.clone();
            }
            None => {
                tracing::warn!("Task {} not found", key);
            }
        }
        drop(tasks);

        if task.status == Status::RETRY {
            let mut task_queue = self.task_queue.lock().await;
            task_queue.push_back(key.clone());
            return;
        }

        if task.status == Status::DONE {
            let mut tasks = self.tasks.lock().await;
            for (_, handler) in tasks.iter_mut() {
                if let Some(completed) = handler.dependencies.get_mut(&key) {
                    if *completed == false {
                        *completed = true;
                        handler.in_degrees-=1;
                        if handler.in_degrees == 0 {
                            let mut task_queue = self.task_queue.lock().await;
                            let id = task_id(&handler.job_id, &handler.task.name);
                            task_queue.push_back(id);
                        }
                    }
                }
            }
        }
    }

    #[tracing::instrument]
    pub async fn push_tasks(&self, tasks: Vec<String>) {
        let mut task_queue = self.task_queue.lock().await;
        for t in tasks {
            task_queue.push_back(t);
        }
    }
}
