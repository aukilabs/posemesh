mod context;
use context::Context;
mod network;
use network::NetworkingConfig;

#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_create(cfg: &NetworkingConfig) -> *mut Context {
    let context = Context::new(cfg);
    Box::into_raw(context)
}

#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_destroy(context: *mut Context) {
    assert!(!context.is_null(), "psm_posemesh_networking_context_destroy(): context is null");
    unsafe {
        let _ = Box::from_raw(context);
    }
}
