use crate::client;
use crate::event;
use crate::network::Node;
use crate::network::{InnerNetworking, NetworkingConfig};
use core::time;
use std::collections::HashMap;
use std::error::Error;
use futures::{StreamExt, channel::mpsc::{Receiver, channel}};
use futures::lock::Mutex;
use libp2p::{Stream, PeerId};
use libp2p_stream::IncomingStreams;
use std::sync::Arc;

#[cfg(not(target_family="wasm"))]
use runtime::get_runtime;

use std::{ffi::CStr, os::raw::{c_char, c_uchar, c_void}};
#[cfg(any(feature="cpp", target_family="wasm"))]
use libp2p::Multiaddr;

#[cfg(any(feature="cpp", feature="py"))]
use tokio::runtime::Runtime;
#[cfg(feature="py")]
use crate::event::{PyStream, NewNodeRegisteredEvent, PubSubMessageReceivedEvent};

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
    pub private_key: *const c_uchar,
    pub private_key_size: u32,
    pub private_key_path: *const c_char,
}

#[cfg(target_family="wasm")]
#[wasm_bindgen(getter_with_clone)]
#[allow(non_snake_case)]
pub struct Config {
    pub bootstraps: String,
    pub relays: String,
    pub privateKey: Vec<u8>,
}

#[cfg(target_family="wasm")]
#[wasm_bindgen]
impl Config {
    #[wasm_bindgen(constructor)]
    pub fn new(
        bootstraps: String,
        relays: String,
        private_key: Vec<u8>
    ) -> Self {
        Self {
            bootstraps: bootstraps,
            relays: relays,
            privateKey: private_key,
        }
    }
}

#[cfg_attr(feature="py", pyclass)]
#[derive(Clone)]
pub struct Context {
    client: client::Client,
    receiver: Arc<Mutex<Receiver<event::Event>>>,
    pub id: String,
}

#[cfg_attr(feature="py", pymethods)]
impl Context {
    #[cfg(any(target_family="wasm", feature="cpp"))]
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

        // *****************
        // ** private_key **
        // *****************

        #[cfg(not(target_family="wasm"))]
        let private_key = unsafe {
            if config.private_key.is_null() {
                assert!((config.private_key_size as usize) == 0, "Context::new(): config.private_key is null and config.private_key_size is non-zero");
                vec![]
            } else {
                core::slice::from_raw_parts(config.private_key, config.private_key_size as usize).to_vec()
            }
        };

        #[cfg(target_family="wasm")]
        let private_key = config.privateKey.clone();

        // **********************
        // ** private_key_path **
        // **********************

        #[cfg(not(target_family="wasm"))]
        let private_key_path = unsafe {
            assert!(!config.private_key_path.is_null(), "Context::new(): config.private_key_path is null");
            CStr::from_ptr(config.private_key_path)
        }.to_str().map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?.to_string();

        #[cfg(target_family="wasm")]
        let private_key_path = "".to_string();

        let _ = serve_as_bootstrap; // TODO: temp

        let cfg = &NetworkingConfig{
            port: 0,
            bootstrap_nodes: bootstraps.iter().map(|bootstrap| bootstrap.to_string()).collect(),
            enable_relay_server: serve_as_relay,
            enable_kdht: true,
            enable_mdns: false,
            relay_nodes: relays.iter().map(|relay| relay.to_string()).collect(),
            private_key: private_key,
            private_key_path: private_key_path,
            name: "my_name".to_string(),
        };
        let ctx = context_create(cfg)?;
        Ok(Box::new(ctx))
    }

    #[cfg(feature="py")]
    #[new]
    pub fn new(mdns: bool, relay_nodes: Vec<String>, name: String, pkey_path: String, port: u16) -> PyResult<Self> {
        pyo3_log::init();
        let cfg = NetworkingConfig {
            port: port,
            bootstrap_nodes: relay_nodes.clone(),
            enable_relay_server: false,
            enable_kdht: true,
            enable_mdns: mdns,
            relay_nodes: relay_nodes.clone(),
            private_key: vec![],
            private_key_path: pkey_path.clone(),
            name: name.clone(),
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
        timeout: u32,
        callback: *const c_void
    ) {
        let mut sender = self.client.clone();
        let user_data_safe = user_data as usize; // Rust is holding me hostage here
        if callback.is_null() {
            get_runtime().spawn(async move {
                match sender.send(msg, peer_id, protocol, timeout).await {
                    Ok(_) => { },
                    Err(error) => {
                        eprintln!("Context::send_with_callback(): {:?}", error);

                        #[cfg(any(target_os = "macos", target_os = "ios", target_os = "tvos", target_os = "watchos"))]
                        eprintln!("Failed to send message: Apple platforms require 'com.apple.security.network.client' entitlement set to YES.");
                    }
                }
            });
        } else {
            let callback: extern "C" fn(status: u8, user_data: *mut c_void) = unsafe { std::mem::transmute(callback) };
            get_runtime().spawn(async move {
                match sender.send(msg, peer_id, protocol, timeout).await {
                    Ok(_) => {
                        let user_data = user_data_safe as *mut c_void;
                        callback(1, user_data);
                    },
                    Err(error) => {
                        eprintln!("Context::send_with_callback(): {:?}", error);

                        #[cfg(any(target_os = "macos", target_os = "ios", target_os = "tvos", target_os = "watchos"))]
                        eprintln!("Failed to send message: Apple platforms require 'com.apple.security.network.client' entitlement set to YES.");

                        let user_data = user_data_safe as *mut c_void;
                        callback(0, user_data);
                    }
                }
            });
        }
    }

    #[cfg(feature="cpp")]
    pub fn set_stream_handler_cpp(
        &mut self,
        protocol: String,
        callback: extern "C" fn(status: u8)
    ) {
        let mut sender = self.client.clone();
        get_runtime().spawn(async move {
            match sender.set_stream_handler(protocol).await {
                Ok(_) => {
                    if (callback as *const c_void) != std::ptr::null() {
                        callback(1);
                    }
                },
                Err(error) => {
                    eprintln!("Context::send_with_callback(): {:?}", error);
                    if (callback as *const c_void) != std::ptr::null() {
                        callback(0);
                    }
                }
            }
        });
    }

    #[cfg(feature="py")]
    pub fn send<'a>(&mut self, msg: Vec<u8>, peer_id: String, protocol: String, timeout: u32, py: Python<'a>) -> PyResult<&'a PyAny> {
        let mut sender = self.client.clone();

        let fut = async move {
            let result = sender.send(msg, peer_id.clone(), protocol.clone(), timeout).await.map_err(|e| PyValueError::new_err(e.to_string()))?;
            Ok(Python::with_gil(|py| event::PyStream::new(protocol, peer_id, result).into_py(py)))
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
                        event::Event::PubSubMessageReceivedEvent { topic, message, .. } => {
                            let py_message = Python::with_gil(|py| Py::new(py, PubSubMessageReceivedEvent {topic, message}).unwrap().into_py(py));
                            Ok(py_message) 
                        }
                    }
                },
                None => Ok(Python::with_gil(|py| py.None()))
            }
        };
        future_into_py(py, fut)
    }

    #[cfg(feature="py")]
    pub fn set_stream_handler<'a>(&mut self, protocol: String, py: Python<'a>) -> PyResult<&'a PyAny> {
        let mut sender = self.client.clone();

        let fut = async move {
            let result = sender.set_stream_handler(protocol).await;
            result.map_err(|e| PyValueError::new_err(e.to_string()))
        };
        future_into_py(py, fut)
    }
}

#[cfg(any(feature="rust", target_family="wasm"))]
impl Context {
    pub fn copy(&self) -> Context {
        Context {
            client: self.client.clone(),
            receiver: self.receiver.clone(),
            id: self.id.clone(),
        }
    }
    pub async fn send(&mut self, msg: Vec<u8>, peer_id: String, protocol: String, timeout: u32) -> Result<Stream, Box<dyn Error + Send + Sync>> {
        let mut sender = self.client.clone();
        sender.send(msg, peer_id, protocol, timeout).await
    }

    pub async fn poll(&mut self) -> Option<event::Event> {
        let mut receiver = self.receiver.lock().await;
        receiver.next().await
    }

    pub async fn set_stream_handler(&mut self, protocol: String) -> Result<IncomingStreams, Box<dyn Error + Send + Sync>> {
        let mut sender = self.client.clone();
        sender.set_stream_handler(protocol).await
    }

    pub async fn publish(&mut self, topic: String, message: Vec<u8>) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut sender = self.client.clone();
        sender.publish(topic, message).await
    }

    pub async fn subscribe(&mut self, topic: String) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut sender = self.client.clone();
        sender.subscribe(topic).await
    }
}

pub fn context_create(config: &NetworkingConfig) -> Result<Context, Box<dyn Error + Send + Sync>> {
    let (sender, receiver) = channel::<client::Command>(8);
    let (event_sender, event_receiver) = channel::<event::Event>(8);
    let client = client::new_client(sender);
    let cfg = config.clone();
    let mut id: String = PeerId::random().to_string();

    #[cfg(all(any(target_family="wasm", feature="rust"), not(any(feature="cpp", feature="py"))))]
    let networking = InnerNetworking::new(&cfg, receiver, event_sender)?;
    
    #[cfg(all(any(target_family="wasm", feature="rust"), not(any(feature="cpp", feature="py"))))]
    let id = networking.node.id.clone();

    #[cfg(any(feature="cpp", feature="py"))]
    get_runtime().block_on(async {
        let networking = InnerNetworking::new(&cfg, receiver, event_sender).unwrap();
        id = networking.node.id.clone();
        tokio::spawn(async move {
            let _ = networking.run().await.expect("Failed to run networking");
        });
    });

    #[cfg(target_family="wasm")]
    wasm_bindgen_futures::spawn_local(async move {
        let _ = networking.run().await.expect("Failed to run networking");
    });

    #[cfg(all(feature="rust", not(target_arch = "wasm32"), not(any(feature="cpp", feature="py"))))]
    tokio::spawn(async move {
        let _ = networking.run().await.expect("Failed to run networking");
    });

    Ok(Context {
        client,
        receiver: Arc::new(Mutex::new(event_receiver)),
        id,
    })
}
