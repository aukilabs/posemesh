mod client;
mod event;
pub mod context;
pub mod network;

#[cfg(any(feature="cpp", feature="wasm"))]
use context::{Config, Context};

#[cfg(feature="py")]
use context::Context;

#[cfg(target_family="wasm")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(feature="py")]
use pyo3::prelude::*;

#[cfg(feature="py")]
use pyo3::exceptions::PyValueError;

// ******************************************
// ** posemesh_networking_context_create() **
// ******************************************

#[cfg(any(feature="cpp", feature="wasm"))]
fn posemesh_networking_context_create(config: &Config) -> *mut Context {
    match Context::new(config) {
        Ok(context) => Box::into_raw(context),
        Err(error) => {
            eprintln!("posemesh_networking_context_create(): {:?}", error);
            std::ptr::null_mut()
        }
    }
}

#[cfg(feature="cpp")]
#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_create(config: *const Config) -> *mut Context {
    assert!(!config.is_null(), "psm_posemesh_networking_context_create(): config is null");
    posemesh_networking_context_create(unsafe { &*config })
}

#[cfg(feature="wasm")]
#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn posemeshNetworkingContextCreate(config: &Config) -> *mut Context {
    posemesh_networking_context_create(config)
}

// *******************************************
// ** posemesh_networking_context_destroy() **
// *******************************************
#[cfg(any(feature="cpp", feature="wasm"))]
fn posemesh_networking_context_destroy(context: *mut Context) {
    assert!(!context.is_null(), "posemesh_networking_context_destroy(): context is null");
    unsafe {
        let _ = Box::from_raw(context);
    }
}

#[cfg(feature="cpp")]
#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_destroy(context: *mut Context) {
    posemesh_networking_context_destroy(context);
}

#[cfg(feature="wasm")]
#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn posemeshNetworkingContextDestroy(context: *mut Context) {
    posemesh_networking_context_destroy(context);
}

// ********************************************
// ** psm_posemesh_networking_send_message() **
// ********************************************

// TODO: C++ needs a shallow Promise/Task impl
// TODO: Vec<u8> should use raw ptr and size (also perf optimization: use some sort of custom "stream" type for large messages)
// TODO: String needs to change to c_char most likely
#[cfg(feature="cpp")]
#[no_mangle]
pub async extern "C" fn psm_posemesh_networking_send_message(context: *mut Context, msg: Vec<u8>, peer_id: String, protocol: String, callback: extern "C" fn(i32)) {
    assert!(!context.is_null(), "psm_posemesh_networking_send_message(): context is null");
    let context = unsafe { &mut *context };
    match context.send(msg, peer_id, protocol).await {
        Ok(context) => callback(0),
        Err(error) => {
            eprintln!("posemesh_networking_context_create(): {:?}", error);
            callback(1);
        }
    }
}

#[cfg(feature="py")]
#[pyfunction]
pub fn start(py: Python, mdns: bool, relay_nodes: Vec<String>, name: String, node_types: Vec<String>, capabilities: Vec<String>, pkey_path: String, port: u16) -> PyResult<Context> {
    pyo3_log::init();
    let cfg = network::NetworkingConfig {
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
    let ctx = context::context_create(&cfg).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(ctx)
}



#[cfg(feature="py")]
#[pymodule]
fn posemesh_networking(_: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<Context>()?;
    m.add_function(wrap_pyfunction!(start, m)?)?;
    Ok(())
}
