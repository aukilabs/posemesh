use libp2p::Multiaddr;
use std::error::Error;
use crate::network::{NetworkingConfig, Networking};
use crate::client;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::Runtime;
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
use std::{ffi::CStr, os::raw::c_char};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::future_to_promise;

#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
#[repr(C)]
pub struct Config {
    pub serve_as_bootstrap: u8,
    pub serve_as_relay: u8,
    pub bootstraps: *const c_char,
}

#[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
#[wasm_bindgen(getter_with_clone)]
#[allow(non_snake_case)]
pub struct Config {
    pub bootstraps: String,
}

#[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
#[wasm_bindgen]
impl Config {
    #[wasm_bindgen(constructor)]
    pub fn new(bootstraps: String) -> Self {
        Self {
            bootstraps: bootstraps,
        }
    }
}

pub struct Context {
    #[cfg(not(target_arch = "wasm32"))]
    runtime: Arc<Runtime>,
    client: client::Client,
}

impl Context {
    pub fn new(config: &Config) -> Result<Box<Context>, Box<dyn Error>> {
        #[cfg(not(target_arch = "wasm32"))]
        let rt = Arc::new(Runtime::new().unwrap());


        #[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
        let serve_as_bootstrap = config.serve_as_bootstrap != 0;

        #[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
        let serve_as_bootstrap = false;

        // ********************
        // ** serve_as_relay **
        // ********************

        #[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
        let serve_as_relay = config.serve_as_relay != 0;

        #[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
        let serve_as_relay = false;

        // ****************
        // ** bootstraps **
        // ****************

        #[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
        let bootstraps_raw = unsafe {
            assert!(!config.bootstraps.is_null(), "Context::new(): config.bootstraps is null");
            CStr::from_ptr(config.bootstraps)
        }.to_str()?;

        #[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
        let bootstraps_raw = &config.bootstraps;

        let bootstraps = bootstraps_raw
            .split(';')
            .map(|bootstrap| bootstrap.trim())
            .filter(|bootstrap| !bootstrap.is_empty())
            .map(|bootstrap|
                bootstrap.parse::<Multiaddr>().map_err(|error| Box::new(error) as Box<dyn Error>)
            ).collect::<Result<Vec<Multiaddr>, Box<dyn Error>>>()?;

        let _ = serve_as_bootstrap; // TODO: temp
        let _ = serve_as_relay; // TODO: temp
        let _ = bootstraps; // TODO: temp

        let (sender, receiver) = futures::channel::mpsc::channel::<client::Command>(8);
        let n = Networking::new(&NetworkingConfig{
            port: 0,
            bootstrap_nodes: vec![],
            enable_relay_server: false,
            enable_kdht: false,
            enable_mdns: false,
            relay_nodes: vec![],
            private_key: "".to_string(),
            private_key_path: "".to_string(),
            name: "my_name".to_string(),
            node_types: vec![],
            node_capabilities: vec![],
        }, receiver)?;
        let c = client::new_client(sender);

        #[cfg(not(target_arch = "wasm32"))]
        rt.spawn(n.run());

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(n.run());

        Ok(Box::new(Context {
            #[cfg(not(target_arch = "wasm32"))]
            runtime: rt,
            client: c,
        }))
    }

    #[cfg(not(target_arch = "wasm32"))] 
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
    
    #[cfg(target_arch = "wasm32")]
    pub fn send(&mut self, msg: Vec<u8>, peer_id: String, protocol: String) -> js_sys::Promise {
        let mut c = self.client.clone();
        future_to_promise(async move {
            c.send(msg, peer_id, protocol).await;
            Ok(JsValue::NULL)
        })
    }
}
