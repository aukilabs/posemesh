mod context;
use context::Context;

#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_create() -> *mut Context {
    let context = Box::new(Context { });
    Box::into_raw(context)
}

#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_destroy(context: *mut Context) {
    assert!(!context.is_null(), "psm_posemesh_networking_context_destroy(): context is null");
    unsafe {
        let _ = Box::from_raw(context);
    }
}
