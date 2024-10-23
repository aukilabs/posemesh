mod context;
use context::Context;

#[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
use wasm_bindgen::prelude::wasm_bindgen;

// ******************************************
// ** posemesh_networking_context_create() **
// ******************************************

fn posemesh_networking_context_create() -> *mut Context {
    let context = Box::new(Context { });
    Box::into_raw(context)
}

#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_create() -> *mut Context {
    posemesh_networking_context_create()
}

#[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn posemeshNetworkingContextCreate() -> *mut Context {
    posemesh_networking_context_create()
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
