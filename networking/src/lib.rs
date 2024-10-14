mod network;

use libp2p::PeerId;
#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::Runtime;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

type Node = network::Node;
type NetworkingConfig = network::NetworkingConfig;

#[cfg(not(target_arch = "wasm32"))]
pub struct Networking {
    runtime: Option<Box<Runtime>>,
    networking: Box<network::RNetworking>,
}

#[cfg(not(target_arch = "wasm32"))]
#[cxx::bridge(namespace = "posemesh::networking")]
mod networking {
    extern "Rust" {
        type Networking;
        type Node;
        type NetworkingConfig;

        fn new_networking() -> Box<Networking>;
        fn send_message(context: &mut Networking, msg: &String);
        fn poll_messages(context: &mut Networking) -> Vec<String>;
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Networking {
    pub fn new(cfg: &NetworkingConfig) -> Box<Networking> {
        let rt = Box::new(Runtime::new().unwrap());
        let mut nt = rt.block_on(async {
            Box::new(Networking {
                runtime: None,
                networking: Box::new(network::RNetworking::new(cfg).unwrap()),
            })
        });
        nt.runtime = Some(rt);
        nt
    }

    pub fn send(self: &mut Self, msg: &str) {
        let nt = self.networking.as_mut();
        self.runtime.as_mut().unwrap().block_on(async {
            nt.send_message(msg.into());
        });
    }

    pub fn poll(self: &mut Self) -> Vec<String> {
        let nt = self.networking.as_mut();
        self.runtime.as_mut().unwrap().block_on(async {
            nt.poll_messages().into_iter().map(|msg| String::from_utf8(msg).unwrap()).collect()
        })
    }
}


#[cfg(not(target_arch = "wasm32"))]
fn new_networking() -> Box<Networking> {
    Networking::new(&NetworkingConfig{
        port: 0,
        bootstrap_nodes: vec![],
        enable_relay_server: false,
        enable_kdht: false,
        enable_mdns: true,
        relay_nodes: vec![],
        private_key: "".to_string(),
        private_key_path: "./volume/pkey".to_string(),
        name: "c++ server".to_string(), // placeholder
        node_capabilities: vec![], // placeholder
        node_types: vec!["c++ server".to_string()], // placeholder
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn send_message(networking: &mut Networking, msg: &String) {
    networking.send(msg);
}

#[cfg(not(target_arch = "wasm32"))]
fn poll_messages(networking: &mut Networking) -> Vec<String> {
    networking.poll()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct Networking {
    networking: Box<network::RNetworking>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl Networking {
    #[wasm_bindgen(constructor)]
    pub fn new(bootstrap_nodes: Vec<String>, relay_nodes: Vec<String>, private_key: String, enable_kdht: bool, name: String) -> Networking {
        Networking {
            networking: Box::new(network::RNetworking::new(&NetworkingConfig{
                port: 0,
                bootstrap_nodes: bootstrap_nodes.clone(),
                enable_relay_server: false,
                enable_kdht: enable_kdht,
                enable_mdns: false,
                relay_nodes: relay_nodes.clone(),
                private_key: private_key.clone(),
                private_key_path: "".to_string(),
                name: name.clone(),
                node_types: vec![],
                node_capabilities: vec![],
            }).unwrap()),
        }
    }

    pub fn send_message(&mut self, msg: &str) {
        self.networking.send_message(msg.into());
    }

    pub fn nodes(&self) -> Vec<JsValue> {
        let nodes = self.networking.nodes_map.lock().unwrap();
        return nodes.iter().map(|(peer_id, node)| {
            let node = serde_wasm_bindgen::to_value(node).unwrap();
            node
        }).collect();
    }

    pub fn poll_messages(&mut self) -> Vec<String> {
        self.networking.poll_messages().into_iter().map(|msg| String::from_utf8(msg).unwrap()).collect()
    }
}
