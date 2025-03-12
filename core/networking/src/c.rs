use crate::libp2p::{Networking, NetworkingConfig};
use crate::binding_helper::{posemesh_networking_get_commit_id, posemesh_networking_context_destroy};
use std::os::raw::{c_char, c_uchar, c_void, c_uint};
use std::ffi::CStr;
use std::slice;
use runtime::get_runtime;

#[repr(C)]
pub struct Config {
    pub bootstraps: *const c_char, // a list of bootstrap nodes separated by comma
    pub relays: *const c_char,
    pub private_key: *const c_uchar, // private key can be null
    pub private_key_size: u32,
    pub private_key_path: *const c_char, // private key path can be null, but if private key is null, private key path must be provided
    pub enable_mdns: u8,
    pub name: *const c_char,
}

pub fn to_rust(config: &Config) -> NetworkingConfig {
    let bootstraps_raw = unsafe {
        assert!(!config.bootstraps.is_null(), "Context::new(): config.bootstraps is null");
        CStr::from_ptr(config.bootstraps)
    }.to_str().expect("Context::new(): config.bootstraps is not a valid UTF-8 string");
    let bootstraps = bootstraps_raw.split(';').map(|s| s.to_string()).filter(|s| !s.is_empty()).collect::<Vec<String>>();

    let relays_raw = unsafe {
        assert!(!config.relays.is_null(), "Context::new(): config.relays is null");
        CStr::from_ptr(config.relays)
    }.to_str().expect("Context::new(): config.relays is not a valid UTF-8 string");
    let relays = relays_raw.split(';').map(|s| s.to_string()).filter(|s| !s.is_empty()).collect::<Vec<String>>();

    let private_key = if config.private_key.is_null() {
        None
    } else {
        let private_key = unsafe {
            std::slice::from_raw_parts(config.private_key, config.private_key_size as usize)
        };
        Some(private_key.to_vec())
    };

    let private_key_path = if config.private_key_path.is_null() {
        None
    } else {
        let private_key_path = unsafe {
            CStr::from_ptr(config.private_key_path)
        }.to_str().expect("Context::new(): config.private_key_path is not a valid UTF-8 string");
        Some(private_key_path.to_string())
    };

    let name = unsafe {
        assert!(!config.name.is_null(), "Context::new(): config.name is null");
        CStr::from_ptr(config.name)
    }.to_str().expect("Context::new(): config.name is not a valid UTF-8 string");

    NetworkingConfig {
        bootstrap_nodes: bootstraps,
        relay_nodes: relays,
        private_key,
        private_key_path,
        enable_mdns: config.enable_mdns != 0,
        enable_kdht: true,
        enable_relay_server: false,
        port: 0,
        name: name.to_string(),
    }
}


#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_create(config: *const Config) -> *mut Networking {
    assert!(!config.is_null(), "psm_posemesh_networking_context_create(): config is null");
    let config = unsafe { &*config };
    let config = to_rust(&config);
    get_runtime().block_on(async move {
        let networking = Networking::new(&config).expect("psm_posemesh_networking_context_create(): failed to create networking context");
        return Box::into_raw(Box::new(networking))
    })
}

#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_destroy(context: *mut Networking) {
    posemesh_networking_context_destroy(context);
}

#[no_mangle]
pub extern "C" fn psm_posemesh_networking_context_send_message(
    context: *mut Networking,
    message: *const c_void,
    message_size: u32,
    peer_id: *const c_char,
    protocol: *const c_char,
    user_data: *mut c_void,
    timeout: u32,
    callback: *const c_void
) -> u8 {
    send_message(context, message, message_size, peer_id, protocol, user_data, timeout, callback)
}

#[no_mangle]
pub extern "C" fn psm_posemesh_networking_get_commit_id(buffer: *mut c_char, size: *mut c_uint) {
    assert!(!buffer.is_null(), "psm_posemesh_networking_get_commit_id(): buffer is null");
    assert!(!size.is_null(), "psm_posemesh_networking_get_commit_id(): size is null");
    let max_size = unsafe { *size };
    if max_size == 0 {
        return;
    }
    let commit_id = posemesh_networking_get_commit_id();
    let commit_id_bytes = commit_id.as_bytes();
    let copy_size = if max_size > 1 {
        std::cmp::min(commit_id_bytes.len(), (max_size - 1) as usize)
    } else { 0 };
    unsafe {
        std::ptr::copy_nonoverlapping(commit_id_bytes.as_ptr(), buffer as *mut u8, copy_size);
        *buffer.add(copy_size) = 0;
        *size = (copy_size + 1) as c_uint;
    };
}

fn send_message(
    context: *mut Networking,
    message: *const c_void,
    message_size: u32,
    peer_id: *const c_char,
    protocol: *const c_char,
    user_data: *mut c_void,
    timeout: u32,
    callback: *const c_void
) -> u8 {
    let message = unsafe {
        assert!(!message.is_null(), "send_message(): message is null");
        assert!(message_size != 0, "send_message(): message_size is zero");
        slice::from_raw_parts(message as *const c_uchar, message_size as usize)
    }.to_vec();

    let peer_id = match unsafe {
        assert!(!peer_id.is_null(), "send_message(): peer_id is null");
        CStr::from_ptr(peer_id)
    }.to_str() {
        Ok(peer_id) => peer_id,
        Err(error) => {
            eprintln!("send_message(): {:?}", error);
            return 0;
        }
    }.to_string();

    let protocol = match unsafe {
        assert!(!protocol.is_null(), "send_message(): protocol is null");
        CStr::from_ptr(protocol)
    }.to_str() {
        Ok(protocol) => protocol,
        Err(error) => {
            eprintln!("send_message(): {:?}", error);
            return 0;
        }
    }.to_string();
    let context = unsafe {
        assert!(!context.is_null(), "send_message(): context is null");
        &mut *context
    };
    let mut sender = context.client.clone();
    let user_data_safe = user_data as usize; // Rust is holding me hostage here
    if callback.is_null() {
        get_runtime().spawn(async move {
            match sender.send(message, peer_id, protocol, timeout).await {
                Ok(_) => { },
                Err(error) => {
                    eprintln!("send_message(): {:?}", error);

                    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "tvos", target_os = "watchos"))]
                    eprintln!("Failed to send message: Apple platforms require 'com.apple.security.network.client' entitlement set to YES.");
                }
            }
        });
    } else {
        let callback: extern "C" fn(status: u8, user_data: *mut c_void) = unsafe { std::mem::transmute(callback) };
        get_runtime().spawn(async move {
            match sender.send(message, peer_id, protocol, timeout).await {
                Ok(_) => {
                    let user_data = user_data_safe as *mut c_void;
                    callback(1, user_data);
                },
                Err(error) => {
                    eprintln!("send_message(): {:?}", error);

                    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "tvos", target_os = "watchos"))]
                    eprintln!("Failed to send message: Apple platforms require 'com.apple.security.network.client' entitlement set to YES.");

                    let user_data = user_data_safe as *mut c_void;
                    callback(0, user_data);
                }
            }
        });
    }
    return 1;
}
