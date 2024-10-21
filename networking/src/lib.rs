mod network;
mod client;
mod context;
use network::NetworkingConfig;
use context::Context;

#[cfg(not(target_arch = "wasm32"))]
#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_create(cfg: &NetworkingConfig) -> *mut Context {
    let context = Context::new(cfg);
    match context {
        Ok(c) => Box::into_raw(c),
        Err(e) => {
            eprintln!("Error creating networking context: {:?}", e);
            std::ptr::null_mut()
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub extern "C" fn psm_posemesh_networking_context_create(bootstrap_nodes: Vec<String>, relay_nodes: Vec<String>, enable_kdht: bool, name: String) -> *mut Context {
    let c = Context::new(bootstrap_nodes, relay_nodes, enable_kdht, name);
    Box::into_raw(Box::new(c))
}

#[cfg(not(target_arch = "wasm32"))]
#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_destroy(context: *mut Context) {
    assert!(!context.is_null(), "psm_posemesh_networking_context_destroy(): context is null");
    unsafe {
        let _ = Box::from_raw(context);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[no_mangle]
pub extern "C" fn psm_posemesh_networking_send_message(context: *mut Context, msg: Vec<u8>, peer_id: String, protocol: String, callback: extern "C" fn(i32)) {
    assert!(!context.is_null(), "psm_posemesh_networking_send_message(): context is null");
    let context = unsafe { &mut *context };
    context.send(callback, msg, peer_id, protocol);
}
