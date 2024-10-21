use std::ffi::c_void;

#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_create() -> *mut c_void {
}

#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_destroy(context: *mut c_void) {
}
