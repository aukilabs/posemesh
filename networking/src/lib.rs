mod context;
use context::Context;
mod network;

#[cfg(not(target_arch = "wasm32"))]
#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_create(cfg: &network::NetworkingConfig) -> *mut Context {
    let context = Context::new(cfg);
    Box::into_raw(context)
}

#[cfg(target_arch = "wasm32")]
pub extern "C" fn psm_posemesh_networking_context_create(bootstrap_nodes: Vec<String>, relay_nodes: Vec<String>, enable_kdht: bool, name: String) -> *mut Context {
    let context = Context::new(bootstrap_nodes, relay_nodes, enable_kdht, name);
    Box::into_raw(Box::new(context))
}

#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_destroy(context: *mut Context) {
    assert!(!context.is_null(), "psm_posemesh_networking_context_destroy(): context is null");
    unsafe {
        let _ = Box::from_raw(context);
    }
}
