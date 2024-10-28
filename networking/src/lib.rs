mod client;
mod context;
mod network;

use context::{Config, Context};
use std::ptr::null_mut;

#[cfg(feature = "web_platform")]
use wasm_bindgen::prelude::wasm_bindgen;

// ******************************************
// ** posemesh_networking_context_create() **
// ******************************************

fn posemesh_networking_context_create(config: &Config) -> *mut Context {
    match Context::new(config) {
        Ok(context) => Box::into_raw(context),
        Err(error) => {
            eprintln!("posemesh_networking_context_create(): {:?}", error);
            null_mut()
        }
    }
}

#[cfg(feature = "non_web_platform")]
#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_create(config: *const Config) -> *mut Context {
    assert!(!config.is_null(), "psm_posemesh_networking_context_create(): config is null");
    posemesh_networking_context_create(unsafe { &*config })
}

#[cfg(feature = "web_platform")]
#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn posemeshNetworkingContextCreate(config: &Config) -> *mut Context {
    posemesh_networking_context_create(config)
}

// *******************************************
// ** posemesh_networking_context_destroy() **
// *******************************************

fn posemesh_networking_context_destroy(context: *mut Context) {
    assert!(!context.is_null(), "posemesh_networking_context_destroy(): context is null");
    unsafe {
        let _ = Box::from_raw(context);
    }
}

#[cfg(feature = "non_web_platform")]
#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_destroy(context: *mut Context) {
    posemesh_networking_context_destroy(context);
}

#[cfg(feature = "web_platform")]
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
#[cfg(feature = "non_web_platform")]
#[no_mangle]
pub extern "C" fn psm_posemesh_networking_send_message(context: *mut Context, msg: Vec<u8>, peer_id: String, protocol: String, callback: extern "C" fn(i32)) {
    assert!(!context.is_null(), "psm_posemesh_networking_send_message(): context is null");
    let context = unsafe { &mut *context };
    context.send(callback, msg, peer_id, protocol);
}
