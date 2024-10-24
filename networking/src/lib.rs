mod context;
use context::{Config, Context};

use std::ptr::null_mut;

#[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
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

#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_create(config: *const Config) -> *mut Context {
    assert!(!config.is_null(), "psm_posemesh_networking_context_create(): config is null");
    posemesh_networking_context_create(unsafe { &*config })
}

#[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
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

#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_destroy(context: *mut Context) {
    posemesh_networking_context_destroy(context);
}

#[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn posemeshNetworkingContextDestroy(context: *mut Context) {
    posemesh_networking_context_destroy(context);
}
