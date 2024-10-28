use crate::client;
use crate::network::{Networking, NetworkingConfig};
use libp2p::Multiaddr;
use std::error::Error;

#[cfg(feature = "non_web_platform")]
use std::{ffi::CStr, os::raw::c_char, sync::Arc};
#[cfg(feature = "non_web_platform")]
use tokio::runtime::Runtime;

#[cfg(feature = "web_platform")]
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};
#[cfg(feature = "web_platform")]
use wasm_bindgen_futures::future_to_promise;

#[cfg(feature = "non_web_platform")]
#[repr(C)]
pub struct Config {
    pub serve_as_bootstrap: u8,
    pub serve_as_relay: u8,
    pub bootstraps: *const c_char,
}

#[cfg(feature = "web_platform")]
#[wasm_bindgen(getter_with_clone)]
#[allow(non_snake_case)]
pub struct Config {
    pub bootstraps: String,
}

#[cfg(feature = "web_platform")]
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
    #[cfg(feature = "non_web_platform")]
    runtime: Arc<Runtime>,
    client: client::Client,
}

impl Context {
    pub fn new(config: &Config) -> Result<Box<Context>, Box<dyn Error>> {
        // ************************
        // ** serve_as_bootstrap **
        // ************************

        #[cfg(feature = "non_web_platform")]
        let serve_as_bootstrap = config.serve_as_bootstrap != 0;

        #[cfg(feature = "web_platform")]
        let serve_as_bootstrap = false;

        // ********************
        // ** serve_as_relay **
        // ********************

        #[cfg(feature = "non_web_platform")]
        let serve_as_relay = config.serve_as_relay != 0;

        #[cfg(feature = "web_platform")]
        let serve_as_relay = false;

        // ****************
        // ** bootstraps **
        // ****************

        #[cfg(feature = "non_web_platform")]
        let bootstraps_raw = unsafe {
            assert!(!config.bootstraps.is_null(), "Context::new(): config.bootstraps is null");
            CStr::from_ptr(config.bootstraps)
        }.to_str()?;

        #[cfg(feature = "web_platform")]
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

        #[cfg(feature = "non_web_platform")]
        let runtime = Arc::new(Runtime::new().unwrap());

        let (sender, receiver) = futures::channel::mpsc::channel::<client::Command>(8);
        let networking = Networking::new(&NetworkingConfig{
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
        let client = client::new_client(sender);

        #[cfg(feature = "non_web_platform")]
        runtime.spawn(networking.run());

        #[cfg(feature = "web_platform")]
        wasm_bindgen_futures::spawn_local(networking.run());

        Ok(Box::new(Context {
            #[cfg(feature = "non_web_platform")]
            runtime: runtime,
            client: client,
        }))
    }

    #[cfg(feature = "non_web_platform")]
    pub fn send(&mut self, callback: extern "C" fn(i32), msg: Vec<u8>, peer_id: String, protocol: String) {
        let runtime = self.runtime.clone();
        let mut sender = self.client.clone();
        runtime.spawn(async move {
            let res = sender.send(msg, peer_id, protocol).await;
            if let Err(e) = res {
                eprintln!("Error sending message: {:?}", e);
                callback(-1);
            } else {
                callback(0);
            }
        });
    }

    #[cfg(feature = "web_platform")]
    pub fn send(&mut self, msg: Vec<u8>, peer_id: String, protocol: String) -> js_sys::Promise {
        let mut client = self.client.clone();
        future_to_promise(async move {
            client.send(msg, peer_id, protocol).await;
            Ok(JsValue::NULL)
        })
    }
}
