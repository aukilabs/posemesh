use std::{collections::{HashMap, VecDeque}, fmt::Debug, sync::Arc};
use domain::protobuf::task::CapabilityFilters;
use async_trait::async_trait;
use networking::libp2p::Node;
use tokio::sync::{Mutex, oneshot};

#[async_trait]
trait LoadBalancer: Send + Sync + Debug {
    async fn find_key(&mut self, nodes: HashMap<String, Node>, key: &str) -> Option<Node>;
    async fn add_key(&mut self, key: &str, value: &str);
}

#[derive(Debug)]
struct RoundRobin {
    capabilities: HashMap<String, Vec<String>>,
    node_indices: HashMap<String, std::sync::atomic::AtomicUsize>,
}

impl RoundRobin {
    pub fn new() -> Self {
        RoundRobin {
            capabilities:HashMap::new(),
            node_indices: HashMap::new(),
        }
    }
}

#[async_trait]
impl LoadBalancer for RoundRobin {
    #[tracing::instrument]
    async fn find_key(&mut self, nodes: HashMap<String, Node>, endpoint: &str) -> Option<Node> {
        let node_ids = self.capabilities.get(endpoint);

        match node_ids {
            Some(node_ids) => {
                let index = self.node_indices.get(endpoint).unwrap();
                let node_id = node_ids.get(index.load(std::sync::atomic::Ordering::Relaxed) % node_ids.len()).unwrap();
                let node = nodes.get(node_id);
                match node {
                    Some(node) => {
                        index.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        return Some(node.clone());
                    }
                    None => None
                }
            }
            None => None
        }
    }
    #[tracing::instrument]
    async fn add_key(&mut self, endpoint: &str, node_id: &str) {
        match self.capabilities.get_mut(endpoint) {
            Some(node_ids) => {
                node_ids.push(node_id.to_string());
            }
            None => {
                self.capabilities.insert(endpoint.to_string(), vec![node_id.to_string()]);
                self.node_indices.insert(endpoint.to_string(), std::sync::atomic::AtomicUsize::new(0));
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct NodesManagement {
    nodes: Arc<Mutex<HashMap<String, Node>>>,
    load_balancer: Arc<Mutex<dyn LoadBalancer>>,
    requests: Arc<Mutex<HashMap<String, VecDeque<oneshot::Sender<String>>>>>,
}

impl NodesManagement {
    pub fn new() -> Self {
        NodesManagement {
            nodes: Arc::new(Mutex::new(HashMap::new())),
            load_balancer: Arc::new(Mutex::new(RoundRobin::new())),
            requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[tracing::instrument]
    pub async fn register_node(&mut self, node: Node) {
        let node_id = node.id.clone();
        let mut exists = false;

        let mut nodes = self.nodes.lock().await;
        if nodes.insert(node_id.clone(), node.clone()).is_some() {
            exists = true;
            tracing::warn!("Node {} already registered", node_id);
        }
        drop(nodes);

        for capability in node.capabilities.iter() {
            let mut requests = self.requests.lock().await;
            if let Some(requests) = requests.get_mut(capability) {
                if let Some(sender) = requests.pop_front() {
                    if let Err(err) = sender.send(node_id.clone()) {
                        tracing::error!("Failed to send node to requestor: {:?}", err);
                    } else {
                        tracing::debug!("Sent node to requestor");
                        continue;
                    }
                }
            }
            drop(requests);
            if !exists {
                let mut load_balancer = self.load_balancer.lock().await;
                load_balancer.add_key(capability, &node_id.clone()).await;
            }
        }
    }

    #[tracing::instrument]
    pub async fn find_node(&mut self, capability_filter: CapabilityFilters) -> Option<Node> {
        let nodes = self.nodes.lock().await;
        let mut load_balancer = self.load_balancer.lock().await;
        load_balancer.find_key(nodes.clone(), &capability_filter.endpoint).await
    }

    #[tracing::instrument]
    pub async fn request_node(&mut self, endpoint: &str) -> oneshot::Receiver<String> {
        let (tx, rx) = oneshot::channel::<String>();
        let mut requests = self.requests.lock().await;
        if let Some(requests) = requests.get_mut(endpoint) {
            requests.push_back(tx);
        } else {
            let mut request_list = VecDeque::new();
            request_list.push_back(tx);
            requests.insert(endpoint.to_string(), request_list);
        }
        rx
    }
}
