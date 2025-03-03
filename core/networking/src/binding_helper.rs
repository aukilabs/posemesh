use crate::network::Networking;

pub fn posemesh_networking_get_commit_id() -> String {
    return env!("COMMIT_ID").to_string();
}

pub fn posemesh_networking_context_destroy(context: *mut Networking) {
    assert!(!context.is_null(), "posemesh_networking_context_destroy(): context is null");
    unsafe {
        let _ = Box::from_raw(context);
    }
}
