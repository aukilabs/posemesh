use crate::client;
use crate::network::{Networking, NetworkingConfig};
use libp2p::Multiaddr;
use std::error::Error;

#[cfg(not(target_family="wasm"))]
use std::{ffi::CStr, os::raw::c_char, sync::Arc};
#[cfg(not(target_family="wasm"))]
use tokio::runtime::Runtime;

#[cfg(target_family="wasm")]
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};
#[cfg(target_family="wasm")]
use wasm_bindgen_futures::future_to_promise;

#[cfg(not(target_family="wasm"))]
#[repr(C)]
pub struct Config {
    pub serve_as_bootstrap: u8,
    pub serve_as_relay: u8,
    pub bootstraps: *const c_char,
}

#[cfg(target_family="wasm")]
#[wasm_bindgen(getter_with_clone)]
#[allow(non_snake_case)]
pub struct Config {
    pub bootstraps: String,
}

#[cfg(target_family="wasm")]
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
    #[cfg(feature="cpp")]
    runtime: Arc<Runtime>,
    client: client::Client,
}

impl Context {
    pub fn new(config: &Config) -> Result<Box<Context>, Box<dyn Error>> {
        // ************************
        // ** serve_as_bootstrap **
        // ************************

        #[cfg(not(target_family="wasm"))]
        let serve_as_bootstrap = config.serve_as_bootstrap != 0;

        #[cfg(target_family="wasm")]
        let serve_as_bootstrap = false;

        // ********************
        // ** serve_as_relay **
        // ********************

        #[cfg(not(target_family="wasm"))]
        let serve_as_relay = config.serve_as_relay != 0;

        #[cfg(target_family="wasm")]
        let serve_as_relay = false;

        // ****************
        // ** bootstraps **
        // ****************

        #[cfg(not(target_family="wasm"))]
        let bootstraps_raw = unsafe {
            assert!(!config.bootstraps.is_null(), "Context::new(): config.bootstraps is null");
            CStr::from_ptr(config.bootstraps)
        }.to_str()?;

        #[cfg(target_family="wasm")]
        let bootstraps_raw = &config.bootstraps;

        let bootstraps = bootstraps_raw
            .split(';')
            .map(|bootstrap| bootstrap.trim())
            .filter(|bootstrap| !bootstrap.is_empty())
            .map(|bootstrap|
                bootstrap.parse::<Multiaddr>().map_err(|error| Box::new(error) as Box<dyn Error>)
            ).collect::<Result<Vec<Multiaddr>, Box<dyn Error>>>()?;

        let _ = serve_as_bootstrap; // TODO: temp
        let _ = bootstraps; // TODO: temp

        let cfg = &NetworkingConfig{
            port: 0,
            bootstrap_nodes: vec![],
            enable_relay_server: serve_as_relay,
            enable_kdht: false,
            enable_mdns: false,
            relay_nodes: vec![],
            private_key: "".to_string(),
            private_key_path: "".to_string(),
            name: "my_name".to_string(),
            node_types: vec![],
            node_capabilities: vec![],
        };
        let ctx = context_create(cfg)?;
        Ok(Box::new(ctx))
    }

    pub async fn send(&mut self, msg: Vec<u8>, peer_id: String, protocol: String) -> Result<(), Box<dyn Error>> {
        let mut sender = self.client.clone();
        sender.send(msg, peer_id, protocol).await
    }
}

pub fn context_create(config: &NetworkingConfig) -> Result<Context, Box<dyn Error>> {
    #[cfg(feature="cpp")]
    let runtime = Arc::new(Runtime::new().unwrap());

    let (sender, receiver) = futures::channel::mpsc::channel::<client::Command>(8);
    let networking = Networking::new(config, receiver)?;
    let client = client::new_client(sender);

    #[cfg(feature="cpp")]
    runtime.spawn(networking.run());

    #[cfg(target_family="wasm")]
    wasm_bindgen_futures::spawn_local(networking.run());

    #[cfg(feature="rust")]
    tokio::spawn(networking.run());

    Ok(Context {
        #[cfg(feature="cpp")]
        runtime: runtime,
        client: client,
    })
}
