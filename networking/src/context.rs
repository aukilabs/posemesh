use crate::client;
use crate::network::{Networking, NetworkingConfig};
use std::error::Error;

#[cfg(any(feature="cpp", feature="wasm"))]
use std::{ffi::CStr, os::raw::{c_char, c_void}};
#[cfg(any(feature="cpp", feature="wasm"))]
use libp2p::Multiaddr;

#[cfg(target_family="wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature="cpp")]
#[repr(C)]
pub struct Config {
    pub serve_as_bootstrap: u8,
    pub serve_as_relay: u8,
    pub bootstraps: *const c_char,
}

#[cfg(feature="wasm")]
#[wasm_bindgen(getter_with_clone)]
#[allow(non_snake_case)]
pub struct Config {
    pub bootstraps: String,
}

#[cfg(feature="wasm")]
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
    runtime: tokio::runtime::Runtime,
    client: client::Client,
}

impl Context {
    #[cfg(any(feature="wasm", feature="cpp"))]
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
        }.to_str().map_err(|error| Box::new(error) as Box<dyn Error>)?;

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

    #[cfg(feature="cpp")]
    pub fn send_with_callback(
        &mut self,
        msg: Vec<u8>,
        peer_id: String,
        protocol: String,
        user_data: *mut c_void,
        callback: *const extern "C" fn(status: u8, user_data: *mut c_void)
    ) {
        let mut sender = self.client.clone();
        let user_data_safe = user_data as usize; // Rust is holding me hostage here
        let callback = unsafe { callback.as_ref() };
        self.runtime.spawn(async move {
            match sender.send(msg, peer_id, protocol).await {
                Ok(_) => {
                    if let Some(callback) = callback {
                        let user_data = user_data_safe as *mut c_void;
                        callback(1, user_data);
                    }
                },
                Err(error) => {
                    eprintln!("Context::send_with_callback(): {:?}", error);
                    if let Some(callback) = callback {
                        let user_data = user_data_safe as *mut c_void;
                        callback(0, user_data);
                    }
                }
            }
        });
    }
}

pub fn context_create(config: &NetworkingConfig) -> Result<Context, Box<dyn Error>> {
    #[cfg(feature="cpp")]
    let runtime = tokio::runtime::Runtime::new().map_err(|error| Box::new(error) as Box<dyn Error>)?;

    let (sender, receiver) = futures::channel::mpsc::channel::<client::Command>(8);
    let client = client::new_client(sender);
    let cfg = config.clone();

    #[cfg(any(target_family="wasm", feature="rust"))]
    let networking = Networking::new(&cfg, receiver)?;

    #[cfg(feature="cpp")]
    runtime.spawn(async move {
        let networking = Networking::new(&cfg, receiver).unwrap();
        networking.run().await;
    });

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
