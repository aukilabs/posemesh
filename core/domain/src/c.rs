use std::any::Any;
use std::os::raw::{c_char, c_void, c_int};
use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::{Arc, Mutex};
use networking::context::Context;
use futures::stream::StreamExt;
use runtime::get_runtime;
use tokio::sync::mpsc;

use crate::cluster::DomainCluster;
use crate::datastore::{common::Datastore, remote::RemoteDatastore};
use crate::binding_helper::init_r_remote_storage;
use crate::protobuf::{self, domain_data::{self, Data, Metadata, Query}};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DomainData {
    pub domain_id: *const c_char,
    pub id: *const c_char,
    pub name: *const c_char,           // Pointer to the null-terminated C string (no fixed size)
    pub data_type: *const c_char,      // Pointer to the null-terminated C string (no fixed size)
    pub metadata: *const c_char,
    pub content: *mut c_void,
    pub content_size: usize,
}

fn to_rust(c_domain_data: *const DomainData) -> Data {
    let c_domain_data_ref = unsafe { &*c_domain_data };

    let domain_id = unsafe { CStr::from_ptr(c_domain_data_ref.domain_id).to_string_lossy().into_owned() };
    let id = unsafe {
        if c_domain_data_ref.id.is_null() {
            None
        } else {
            Some(CStr::from_ptr(c_domain_data_ref.id).to_string_lossy().into_owned())
        }
    };
    let name = unsafe { CStr::from_ptr(c_domain_data_ref.name).to_string_lossy().into_owned() };
    let data_type = unsafe { CStr::from_ptr(c_domain_data_ref.data_type).to_string_lossy().into_owned() };
    let metadata = unsafe { CStr::from_ptr(c_domain_data_ref.metadata).to_string_lossy().into_owned() };

    let content = unsafe {
        if c_domain_data_ref.content.is_null() {
            Vec::new()
        } else {
            let content_ptr = c_domain_data_ref.content as *const u8;
            let content_size = c_domain_data_ref.content_size;
            let content_slice = std::slice::from_raw_parts(content_ptr, content_size);
            content_slice.to_vec()
        }
    };
    let content_size = content.len();

    Data {
        domain_id: domain_id,
        metadata: Metadata {
            name,
            data_type,
            properties: serde_json::from_str(&metadata).unwrap(),
            size: content_size as u32,
            id,
        },
        content,
    }
}

fn from_rust(r_domain_data: &Data) -> DomainData {
    let id = match &r_domain_data.metadata.id {
        Some(id) => CString::new(id.clone()).unwrap().into_raw(),
        None => ptr::null_mut(),
    };
    let metadata = &r_domain_data.metadata;
    let domain_id = CString::new(r_domain_data.domain_id.clone()).unwrap().into_raw();
    let name = CString::new(metadata.name.clone()).unwrap().into_raw();
    let data_type = CString::new(metadata.data_type.clone()).unwrap().into_raw();
    let metadata = CString::new(serde_json::to_string(&metadata.properties).unwrap()).unwrap().into_raw();

    let content = if r_domain_data.content.is_empty() {
        ptr::null_mut()
    } else {
        let content_ptr = r_domain_data.content.as_ptr() as *mut c_void;
        let content_ptr = content_ptr as *mut u8;
        let content_ptr = content_ptr as *mut c_void;
        content_ptr
    };
    let content_size = r_domain_data.metadata.size as usize;

    DomainData {
        domain_id,
        id,
        name,
        data_type,
        metadata,
        content,
        content_size,
    }
}

#[no_mangle]
pub extern "C" fn create_domain_data_query(ids_ptr: *const *const c_char, len: c_int, name: *const c_char, data_type: *const c_char) -> *mut domain_data::Query {
    let name = unsafe { 
        if name.is_null() {
            None
        } else {
            Some(CStr::from_ptr(name).to_string_lossy().into_owned())
        }
    };
    let data_type = unsafe { 
        if data_type.is_null() {
            None
        } else {
            Some(CStr::from_ptr(data_type).to_string_lossy().into_owned())
        }
    };
    let ids = unsafe {
        assert!(!ids_ptr.is_null());
        let ids_slice = std::slice::from_raw_parts(ids_ptr, len as usize);
        ids_slice.iter().map(|&id| CStr::from_ptr(id).to_string_lossy().into_owned()).collect()
    };

    let query = domain_data::Query {
        ids,
        name,
        data_type,
    };

    Box::into_raw(Box::new(query))
}

#[no_mangle]
pub extern "C" fn free_domain_data_query(query: *mut domain_data::Query) {
    if query.is_null() {
        return;
    }

    unsafe {
        let _ = Box::from_raw(query);
    }
}

#[repr(C)]
pub struct DomainError {
    pub message: *const c_char, // Error message
}

type FindCallback = extern "C" fn(*mut c_void, *const DomainData, *const DomainError);

/// Free a C string if it's not null
unsafe fn free_c_string(ptr: *const c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr as *mut c_char));
    }
}

/// Free a DomainData struct
#[no_mangle]
pub unsafe extern "C" fn free_domain_data(data: *mut DomainData) {
    if data.is_null() {
        return;
    }

    let data = &mut *data;

    free_c_string(data.domain_id);
    free_c_string(data.id);
    free_c_string(data.name);
    free_c_string(data.data_type);
    free_c_string(data.metadata);

    if !data.content.is_null() {
        drop(Vec::from_raw_parts(data.content as *mut u8, data.content_size, data.content_size));
    }

    // Finally, free the struct itself
    drop(Box::from_raw(data));
}

#[no_mangle]
pub extern "C" fn init_domain_cluster(domain_manager_id: *const c_char, peer: *mut Context) -> *mut DomainCluster {
    if domain_manager_id.is_null() || peer.is_null() {
        return ptr::null_mut();
    }
    let domain_manager_id = unsafe { CStr::from_ptr(domain_manager_id).to_string_lossy().into_owned() };
    let peer = unsafe { Box::from_raw(peer) };
    let cluster = DomainCluster::new(domain_manager_id, peer);
    Box::into_raw(Box::new(cluster))
}

#[no_mangle]
pub extern "C" fn free_domain_cluster(cluster: *mut DomainCluster) {
    if cluster.is_null() {
        return;
    }

    unsafe {
        let _ = Box::from_raw(cluster);
    }
}

#[no_mangle]
pub extern "C" fn init_remote_storage(cluster: *mut DomainCluster, peer: *mut Context) -> *mut DatastoreWrapper {
    Box::into_raw(Box::new(DatastoreWrapper::new(Box::new(init_r_remote_storage(cluster, peer)))))
}

#[no_mangle]
pub extern "C" fn free_datastore(store: *mut DatastoreWrapper) {
    if store.is_null() {
        return;
    }

    unsafe {
        let _ = Box::from_raw(store);
    }
}

pub struct DatastoreWrapper {
    pub store: Box<dyn Datastore>,
}

impl DatastoreWrapper {
    fn new(store: Box<dyn Datastore>) -> Self {
        DatastoreWrapper { store }
    }
}

#[no_mangle]
pub extern "C" fn find_domain_data(
    store: *mut DatastoreWrapper, // Pointer to Rust struct
    domain_id: *const c_char,    // C string
    query: *mut domain_data::Query, // Pointer to query struct
    callback: FindCallback,      // C function pointer
    user_data: *mut c_void       // Custom user data (optional)
) {
    // Convert domain_id to Rust string
    let domain_id = unsafe {
        assert!(!domain_id.is_null());
        std::ffi::CStr::from_ptr(domain_id).to_string_lossy().into_owned()
    };

    // Convert store pointer to Rust reference
    let store_wrapper = unsafe {
        assert!(!store.is_null());
        &mut *store
    };

    let query = query as *const domain_data::Query;
    let query_clone = unsafe { (*query).clone() };

    let user_data_clone = user_data as usize;

    // Spawn a Tokio task to process the receiver
    get_runtime().spawn(async move {
        let stream = store_wrapper.store.find(domain_id, query_clone).await;
        let mut stream = Box::pin(stream);

        while let Some(result) = stream.next().await {
            match result {
                Ok(data) => {
                    let c_data = from_rust(&data);
                    let user_data = user_data_clone as *mut c_void;
                    callback(user_data, &c_data, ptr::null());
                }
                Err(err) => {
                    let message = CString::new(err.to_string()).unwrap().into_raw();
                    let error = DomainError { message };
                    let user_data = user_data_clone as *mut c_void;
                    callback(user_data, ptr::null(), &error);
                }
            }
        }
    });
}
