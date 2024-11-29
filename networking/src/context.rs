use crate::client;
use crate::event;
use crate::network::{Networking, NetworkingConfig};
use std::error::Error;
use futures::{StreamExt, channel::mpsc::{Receiver, channel}};
use futures::lock::Mutex;
use std::sync::Arc;

#[cfg(any(feature="cpp", feature="wasm"))]
use std::{ffi::CStr, os::raw::{c_char, c_void}};
#[cfg(any(feature="cpp", feature="wasm"))]
use libp2p::Multiaddr;

#[cfg(any(feature="cpp", feature="py"))]
use tokio::runtime::Runtime;
#[cfg(feature="py")]
use crate::event::{MessageReceivedEvent, NewNodeRegisteredEvent};

#[cfg(target_family="wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature="py")]
use pyo3::prelude::*;
#[cfg(feature="py")]
use pyo3::exceptions::PyValueError;
#[cfg(feature="py")]
use pyo3_asyncio::tokio::future_into_py;

#[cfg(feature="cpp")]
#[repr(C)]
pub struct Config {
    pub serve_as_bootstrap: u8,
    pub serve_as_relay: u8,
    pub bootstraps: *const c_char,
    pub relays: *const c_char,
}

#[cfg(feature="wasm")]
#[wasm_bindgen(getter_with_clone)]
#[allow(non_snake_case)]
pub struct Config {
    pub bootstraps: String,
    pub relays: String,
}

#[cfg(feature="wasm")]
#[wasm_bindgen]
impl Config {
    #[wasm_bindgen(constructor)]
    pub fn new(
        bootstraps: String,
        relays: String
    ) -> Self {
        Self {
            bootstraps: bootstraps,
            relays: relays,
        }
    }
}

#[cfg_attr(feature="py", pyclass)]
pub struct Context {
    #[cfg(any(feature="cpp", feature="py"))]
    runtime: Arc<Runtime>,
    client: client::Client,
    receiver: Arc<Mutex<Receiver<event::Event>>>,
}

#[cfg_attr(feature="py", pymethods)]
impl Context {
    #[cfg(any(feature="wasm", feature="cpp"))]
    pub fn new(config: &Config) -> Result<Box<Context>, Box<dyn Error + Send + Sync>> {
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
        }.to_str().map_err(|error| Box::new(error))?;

        #[cfg(target_family="wasm")]
        let bootstraps_raw = &config.bootstraps;

        let bootstraps = bootstraps_raw
            .split(';')
            .map(|bootstrap| bootstrap.trim())
            .filter(|bootstrap| !bootstrap.is_empty())
            .map(|bootstrap|
                bootstrap.parse::<Multiaddr>().map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
            ).collect::<Result<Vec<Multiaddr>, Box<dyn Error + Send + Sync>>>()?;

        // ************
        // ** relays **
        // ************

        #[cfg(not(target_family="wasm"))]
        let relays_raw = unsafe {
            assert!(!config.relays.is_null(), "Context::new(): config.relays is null");
            CStr::from_ptr(config.relays)
        }.to_str().map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;

        #[cfg(target_family="wasm")]
        let relays_raw = &config.relays;

        let relays = relays_raw
            .split(';')
            .map(|relay| relay.trim())
            .filter(|relay| !relay.is_empty())
            .map(|relay|
                relay.parse::<Multiaddr>().map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
            ).collect::<Result<Vec<Multiaddr>, Box<dyn Error + Send + Sync>>>()?;

        let _ = serve_as_bootstrap; // TODO: temp

        let cfg = &NetworkingConfig{
            port: 0,
            bootstrap_nodes: bootstraps.iter().map(|bootstrap| bootstrap.to_string()).collect(),
            enable_relay_server: serve_as_relay,
            enable_kdht: true,
            enable_mdns: false,
            relay_nodes: relays.iter().map(|relay| relay.to_string()).collect(),
            private_key: "".to_string(),
            private_key_path: "".to_string(),
            name: "my_name".to_string(),
            node_types: vec![],
            node_capabilities: vec![],
        };
        let ctx = context_create(cfg)?;
        Ok(Box::new(ctx))
    }

    #[cfg(feature="py")]
    #[new]
    pub fn new(mdns: bool, relay_nodes: Vec<String>, name: String, node_types: Vec<String>, capabilities: Vec<String>, pkey_path: String, port: u16) -> PyResult<Self> {
        pyo3_log::init();
        let cfg = NetworkingConfig {
            port: port,
            bootstrap_nodes: relay_nodes.clone(),
            enable_relay_server: false,
            enable_kdht: true,
            enable_mdns: mdns,
            relay_nodes: relay_nodes.clone(),
            private_key: "".to_string(),
            private_key_path: pkey_path.clone(),
            name: name.clone(),
            node_types: node_types.clone(),
            node_capabilities: capabilities.clone(),
        };
        let ctx = context_create(&cfg).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(ctx)
    }

    #[cfg(feature="cpp")]
    pub fn send_with_callback(
        &mut self,
        msg: Vec<u8>,
        peer_id: String,
        protocol: String,
        user_data: *mut c_void,
        callback: extern "C" fn(status: u8, user_data: *mut c_void)
    ) {
        let mut sender = self.client.clone();
        let user_data_safe = user_data as usize; // Rust is holding me hostage here
        self.runtime.spawn(async move {
            match sender.send(msg, peer_id, protocol).await {
                Ok(_) => {
                    if (callback as *const c_void) != std::ptr::null() {
                        let user_data = user_data_safe as *mut c_void;
                        callback(1, user_data);
                    }
                },
                Err(error) => {
                    eprintln!("Context::send_with_callback(): {:?}", error);
                    if (callback as *const c_void) != std::ptr::null() {
                        let user_data = user_data_safe as *mut c_void;
                        callback(0, user_data);
                    }
                }
            }
        });
    }

    #[cfg(feature="py")]
    pub fn send<'a>(&mut self, msg: Vec<u8>, peer_id: String, protocol: String, py: Python<'a>) -> PyResult<&'a PyAny> {
        let mut sender = self.client.clone();

        let fut = async move {
            let result = sender.send(msg, peer_id, protocol).await;
            result.map_err(|e| PyValueError::new_err(e.to_string()))
        };
        future_into_py(py, fut)
    }

    #[cfg(feature="py")]
    pub fn poll<'a>(&mut self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let receiver = self.receiver.clone();
        let fut = async move {
            let mut receiver = receiver.lock().await;
            let event = receiver.next().await;
            match event {
                Some(event) => {
                    match event {
                        event::Event::NewNodeRegistered { node } => {
                            let py_message = Python::with_gil(|py| Py::new(py, NewNodeRegisteredEvent::new(node)).unwrap().into_py(py));
                            Ok(py_message)
                        }
                        event::Event::MessageReceived { protocol, stream, peer } => {
                            let py_message = Python::with_gil(|py| Py::new(py, MessageReceivedEvent::new(protocol, peer, stream)).unwrap().into_py(py));
                            Ok(py_message)
                        }
                    }
                },
                None => Ok(Python::with_gil(|py| py.None()))
            }
        };
        future_into_py(py, fut)
    }
}

#[cfg(any(feature="rust", target_family="wasm"))]
impl Context {
    pub async fn send(&mut self, msg: Vec<u8>, peer_id: String, protocol: String) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut sender = self.client.clone();
        sender.send(msg, peer_id, protocol).await
    }

    pub async fn poll(&mut self) -> Option<event::Event> {
        let mut receiver = self.receiver.lock().await;
        receiver.next().await
    }
}

pub fn context_create(config: &NetworkingConfig) -> Result<Context, Box<dyn Error + Send + Sync>> {
    #[cfg(any(feature="cpp", feature="py"))]
    let runtime = Runtime::new()?;

    let (sender, receiver) = channel::<client::Command>(8);
    let (event_sender, event_receiver) = channel::<event::Event>(8);
    let client = client::new_client(sender);
    let cfg = config.clone();

    #[cfg(any(target_family="wasm", feature="rust"))]
    let networking = Networking::new(&cfg, receiver, event_sender)?;

    #[cfg(any(feature="cpp", feature="py"))]
    runtime.block_on(async {
        let networking = Networking::new(&cfg, receiver, event_sender).unwrap();
        runtime.spawn(async move {
            let _ = networking.run().await.expect("Failed to run networking");
        });
    });

    #[cfg(target_family="wasm")]
    wasm_bindgen_futures::spawn_local(async move {
        let _ = networking.run().await.expect("Failed to run networking");
    });

    #[cfg(feature="rust")]
    tokio::spawn(async move {
        let _ = networking.run().await.expect("Failed to run networking");
    });

    Ok(Context {
        #[cfg(any(feature="cpp", feature="py"))]
        runtime: Arc::new(runtime),
        client,
        receiver: Arc::new(Mutex::new(event_receiver)),
    })
}
