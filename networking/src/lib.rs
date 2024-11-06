mod client;
pub mod context;
pub mod network;

#[cfg(any(feature="cpp", feature="wasm"))]
use context::{Config, Context};
#[cfg(any(feature="cpp", feature="wasm"))]
use std::{ffi::{c_char, c_uchar, CStr}, os::raw::c_void, slice};

#[cfg(feature="py")]
use context::Context;

#[cfg(target_family="wasm")]
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};
#[cfg(target_family="wasm")]
use wasm_bindgen_futures::{future_to_promise, js_sys::{Error, Promise}};

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

// ************************************************
// ** posemesh_networking_context_send_message() **
// ************************************************

#[cfg(feature="cpp")]
type SendMessageReturnType = u8;

#[cfg(feature="wasm")]
type SendMessageReturnType = Promise;

#[cfg(any(feature="cpp", feature="wasm"))]
fn posemesh_networking_context_send_message(
    context: *mut Context,
    message: Vec<u8>,
    peer_id: String,
    protocol: String,
    #[cfg(feature="cpp")]
    user_data: *mut c_void,
    #[cfg(feature="cpp")]
    callback: extern "C" fn(status: u8, user_data: *mut c_void)
) -> SendMessageReturnType {
    let context = unsafe {
        assert!(!context.is_null(), "posemesh_networking_context_send_message(): context is null");
        &mut *context
    };

    #[cfg(feature="wasm")]
    return future_to_promise(async move {
        match context.send(message, peer_id, protocol).await {
            Ok(_) => { Ok(JsValue::from(true)) },
            Err(error) => {
                eprintln!("posemesh_networking_context_send_message(): {:?}", error);
                Err(JsValue::from(Error::new(error.to_string().as_str())))
            }
        }
    });

    #[cfg(feature="cpp")]
    {
        context.send_with_callback(message, peer_id, protocol, user_data, callback);
        return 1;
    }
}

#[cfg(any(feature="cpp", feature="wasm"))]
fn posemesh_networking_context_send_message_2(
    context: *mut Context,
    message: *const c_void,
    message_size: u32,
    peer_id: *const c_char,
    protocol: *const c_char,
    #[cfg(feature="cpp")]
    user_data: *mut c_void,
    #[cfg(feature="cpp")]
    callback: extern "C" fn(status: u8, user_data: *mut c_void)
) -> SendMessageReturnType {
    let message = unsafe {
        assert!(!message.is_null(), "posemesh_networking_context_send_message_2(): message is null");
        assert!(message_size != 0, "posemesh_networking_context_send_message_2(): message_size is zero");
        slice::from_raw_parts(message as *const c_uchar, message_size as usize)
    }.to_vec();

    let peer_id = match unsafe {
        assert!(!peer_id.is_null(), "posemesh_networking_context_send_message_2(): peer_id is null");
        CStr::from_ptr(peer_id)
    }.to_str() {
        Ok(peer_id) => peer_id,
        Err(error) => {
            eprintln!("posemesh_networking_context_send_message_2(): {:?}", error);
            
            #[cfg(feature="wasm")]
            return future_to_promise(async move {
                Err(JsValue::from(Error::new(error.to_string().as_str())))
            });

            #[cfg(feature="cpp")]
            return 0;
        }
    }.to_string();

    let protocol = match unsafe {
        assert!(!protocol.is_null(), "posemesh_networking_context_send_message_2(): protocol is null");
        CStr::from_ptr(protocol)
    }.to_str() {
        Ok(protocol) => protocol,
        Err(error) => {
            eprintln!("posemesh_networking_context_send_message_2(): {:?}", error);

            #[cfg(feature="wasm")]
            return future_to_promise(async move {
                Err(JsValue::from(Error::new(error.to_string().as_str())))
            });

            #[cfg(feature="cpp")]
            return 0;
        }
    }.to_string();

    return posemesh_networking_context_send_message(
        context,
        message,
        peer_id,
        protocol,
        #[cfg(feature="cpp")]
        user_data,
        #[cfg(feature="cpp")]
        callback
    );
}

#[cfg(feature="cpp")]
#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_send_message(
    context: *mut Context,
    message: *const c_void,
    message_size: u32,
    peer_id: *const c_char,
    protocol: *const c_char,
    user_data: *mut c_void,
    callback: extern "C" fn(status: u8, user_data: *mut c_void)
) -> u8 {
    posemesh_networking_context_send_message_2(context, message, message_size, peer_id, protocol, user_data, callback)
}

#[cfg(feature="wasm")]
#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn posemeshNetworkingContextSendMessage(context: *mut Context, message: Vec<u8>, peer_id: String, protocol: String) -> Promise {
    posemesh_networking_context_send_message(context, message, peer_id, protocol)
}

#[cfg(feature="wasm")]
#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn posemeshNetworkingContextSendMessage2(
    context: *mut Context,
    message: *const c_void,
    message_size: u32,
    peer_id: *const c_char,
    protocol: *const c_char
) -> Promise {
    posemesh_networking_context_send_message_2(context, message, message_size, peer_id, protocol)
}

#[cfg(feature="py")]
#[pyfunction]
pub fn start(py: Python, relay_nodes: Vec<String>, name: String, node_types: Vec<String>, capabilities: Vec<String>, pkey_path: String, port: u16) -> PyResult<Context> {
    pyo3_log::init();
    let cfg = network::NetworkingConfig {
        port: port,
        bootstrap_nodes: relay_nodes.clone(),
        enable_relay_server: false,
        enable_kdht: true,
        enable_mdns: false,
        relay_nodes: relay_nodes.clone(),
        private_key: "".to_string(),
        private_key_path: pkey_path.clone(),
        name: name.clone(),
        node_types: node_types.clone(),
        node_capabilities: capabilities.clone(),
    };
    let (sender, receiver) = futures::channel::mpsc::channel::<client::Command>(8);
    pyo3_asyncio::tokio::future_into_py(py, async move {
        let mut networking = network::Networking::new(&cfg, receiver).map_err(|e| PyValueError::new_err(e.to_string()))?;
        networking.run().await;
        Ok(())
    })?;

    Ok(Context{client: client::new_client(sender)})
}

#[cfg(feature="py")]
#[pymodule]
fn posemesh_networking(_: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<Context>()?;
    m.add_function(wrap_pyfunction!(start, m)?)?;
    Ok(())
}
