use crate::network::{NetworkingConfig, RNetworking};
#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::Runtime;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
pub struct Context {
    runtime: Option<Box<Runtime>>,
    networking: Box<RNetworking>,
}
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct Context {
    networking: Box<RNetworking>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Context {
    pub fn new(cfg: &NetworkingConfig) -> Box<Context> {
        let rt = Box::new(Runtime::new().unwrap());
        let mut nt = rt.block_on(async {
            Box::new(Context {
                runtime: None,
                networking: Box::new(RNetworking::new(&cfg).unwrap()),
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

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl Context {
    #[wasm_bindgen(constructor)]
    pub fn new(bootstrap_nodes: Vec<String>, relay_nodes: Vec<String>, enable_kdht: bool, name: String) -> Context {
        Context {
            networking: Box::new(RNetworking::new(&NetworkingConfig{
                port: 0,
                bootstrap_nodes: bootstrap_nodes.clone(),
                enable_relay_server: false,
                enable_kdht: enable_kdht,
                enable_mdns: false,
                relay_nodes: relay_nodes.clone(),
                private_key: "".to_string(),
                private_key_path: "".to_string(),
                name: name.clone(),
                node_types: vec![],
                node_capabilities: vec![],
            }).unwrap()),
        }
    }
}
