use crate::network::{NetworkingConfig, Networking};
use crate::client;
use std::sync::Arc;
use std::error::Error;

#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::Runtime;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::future_to_promise;

#[cfg(not(target_arch = "wasm32"))]
pub struct Context {
    runtime: Arc<Runtime>,
    client: client::Client,
}

#[cfg(not(target_arch = "wasm32"))]
impl Context {
    pub fn new(cfg: &NetworkingConfig) -> Result<Box<Context>, Box<dyn Error>> {
        let rt = Arc::new(Runtime::new().unwrap());
        let (sender, receiver) = futures::channel::mpsc::channel::<client::Command>(8);
        let n = Networking::new(cfg, receiver)?;
        let c = client::new_client(sender);

        rt.spawn(n.run());
        Ok(Box::new(Context {
            runtime: rt,
            client: c,
        }))
    }

    pub fn send(&mut self, callback: extern "C" fn(i32), msg: Vec<u8>, peer_id: String, protocol: String) {
        let rt = self.runtime.clone();
        let mut sender = self.client.clone();
        rt.spawn(async move {
            let res = sender.send(msg, peer_id, protocol).await;
            if let Err(e) = res {
                eprintln!("Error sending message: {:?}", e);
                callback(-1);
            } else {
                callback(0);
            }
        });
    }

    // pub fn poll(self: &mut Self) -> Vec<String> {
    //     let nt = self.networking.as_mut();
    //     self.runtime.as_mut().unwrap().block_on(async {
    //         nt.poll_messages().into_iter().map(|msg| String::from_utf8(msg).unwrap()).collect()
    //     })
    // }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct Context {
    client: client::Client,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl Context {
    #[wasm_bindgen(constructor)]
    pub fn new(bootstrap_nodes: Vec<String>, relay_nodes: Vec<String>, enable_kdht: bool, name: String) -> Context {
        let (sender, receiver) = futures::channel::mpsc::channel::<client::Command>(8);
        let c = client::new_client(sender); 
        let nt = Networking::new(&NetworkingConfig{
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
        }, receiver).unwrap();
        let ct = Context {
            client: c,
        };
        wasm_bindgen_futures::spawn_local(nt.run());
        ct
    }

    pub fn send_message(&mut self, msg: Vec<u8>, peer_id: String, protocol: String) -> js_sys::Promise {
        let mut c = self.client.clone();
        future_to_promise(async move {
            c.send(msg, peer_id, protocol).await;
            Ok(JsValue::NULL)
        })
    }

    // pub fn nodes(&self) -> Vec<JsValue> {
    //     let nodes = self.networking.nodes_map.lock().unwrap();
    //     return nodes.iter().map(|(peer_id, node)| {
    //         let node = serde_wasm_bindgen::to_value(node).unwrap();
    //         node
    //     }).collect();
    // }

    // pub fn poll_messages(&mut self) -> Vec<String> {
    //     self.networking.poll_messages().into_iter().map(|msg| String::from_utf8(msg).unwrap()).collect()
    // }
}
