use libp2p::Multiaddr;
use std::error::Error;
use crate::network::{NetworkingConfig, RNetworking}; // TODO: review

#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
use std::{ffi::CStr, os::raw::c_char};

#[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
use wasm_bindgen::prelude::wasm_bindgen;

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
    networking: Box<RNetworking>, // TODO: review
}

impl Context {
    pub fn new(config: &Config) -> Result<Box<Self>, Box<dyn Error>> {
        // ************************
        // ** serve_as_bootstrap **
        // ************************

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

        Ok(Box::new(Self {
            networking: Box::new(RNetworking::new(&NetworkingConfig{ // TODO: review
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
            }).unwrap()),
        }))
    }
}
