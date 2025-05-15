use std::os::raw::{c_char, c_void, c_int};
use std::ffi::{CStr, CString};
use std::ptr;
use futures::stream::StreamExt;
use runtime::get_runtime;

use crate::cluster::DomainCluster;
use crate::datastore::common::{self, data_id_generator, Datastore, ReliableDataProducer as r_ReliableDataProducer};
use crate::binding_helper::init_r_remote_storage;
use crate::datastore::remote::RemoteDatastore;
use crate::protobuf::domain_data::UpsertMetadata;
use crate::protobuf::domain_data::{self, Data};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DomainData {
    pub domain_id: *const c_char,
    pub id: *const c_char,
    pub name: *const c_char,           // Pointer to the null-terminated C string (no fixed size)
    pub data_type: *const c_char,      // Pointer to the null-terminated C string (no fixed size)
    pub properties: *const c_char,
    pub content: *mut c_void,
    pub content_size: usize,
}

#[no_mangle]
pub extern "C" fn free_domain_data(data: *mut DomainData) {
    if data.is_null() {
        return;
    }
    
    unsafe {
        let data = &mut *data;
        free_c_string(data.domain_id);
        free_c_string(data.id);
        free_c_string(data.name);
        free_c_string(data.data_type);
        free_c_string(data.properties);

        if !data.content.is_null() {
            drop(Vec::from_raw_parts(data.content as *mut u8, data.content_size, data.content_size));
        }

        let _ = Box::from_raw(data);
    }
}

fn to_rust(c_domain_data: *const DomainData) -> (UpsertMetadata, Vec<u8>) {
    let c_domain_data_ref = unsafe { &*c_domain_data };

    let id = unsafe {
        if c_domain_data_ref.id.is_null() {
            None
        } else {
            Some(CStr::from_ptr(c_domain_data_ref.id).to_string_lossy().into_owned())
        }
    };
    let name = unsafe { CStr::from_ptr(c_domain_data_ref.name).to_string_lossy().into_owned() };
    let data_type = unsafe { CStr::from_ptr(c_domain_data_ref.data_type).to_string_lossy().into_owned() };
    let properties = unsafe { CStr::from_ptr(c_domain_data_ref.properties).to_string_lossy().into_owned() };

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

    (UpsertMetadata {
        name,
        data_type,
        size: content_size as u32,
        is_new: id.is_none(),
        id: id.clone().unwrap_or_else(|| data_id_generator()),
        properties: serde_json::from_str(&properties).unwrap(),
    }, content)
}

fn from_rust(r_domain_data: &Data) -> DomainData {
    let id = CString::new(r_domain_data.metadata.id.clone()).unwrap().into_raw();
    let metadata = &r_domain_data.metadata;
    let domain_id = CString::new(r_domain_data.domain_id.clone()).unwrap().into_raw();
    let name = CString::new(metadata.name.clone()).unwrap().into_raw();
    let data_type = CString::new(metadata.data_type.clone()).unwrap().into_raw();
    let properties = CString::new(serde_json::to_string(&metadata.properties).unwrap()).unwrap().into_raw();

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
        properties,
        content,
        content_size,
    }
}

#[no_mangle]
pub extern "C" fn create_domain_data_query(ids_ptr: *const *const c_char, len: c_int, name_regexp: *const c_char, data_type_regexp: *const c_char, names: *const *const c_char, names_len: c_int, data_types: *const *const c_char, data_types_len: c_int) -> *mut domain_data::Query {
    let ids = unsafe {
        assert!(!ids_ptr.is_null());
        std::slice::from_raw_parts(ids_ptr, len as usize)
            .iter()
            .map(|&id| {
                assert!(!id.is_null());
                CStr::from_ptr(id).to_string_lossy().into_owned()
            })
            .collect()
    };

    let names = unsafe {
        assert!(!names.is_null());
        std::slice::from_raw_parts(names, names_len as usize)
            .iter()
            .map(|&name| {
                assert!(!name.is_null());
                CStr::from_ptr(name).to_string_lossy().into_owned()
            })
            .collect()
    };

    let data_types = unsafe {
        assert!(!data_types.is_null());
        std::slice::from_raw_parts(data_types, data_types_len as usize)
            .iter()
            .map(|&data_type| {
                assert!(!data_type.is_null());
                CStr::from_ptr(data_type).to_string_lossy().into_owned()
            })
            .collect()
    };

    let name_regexp = unsafe {
        assert!(!name_regexp.is_null());
        CStr::from_ptr(name_regexp).to_string_lossy().into_owned()
    };

    let data_type_regexp = unsafe {
        assert!(!data_type_regexp.is_null());
        CStr::from_ptr(data_type_regexp).to_string_lossy().into_owned()
    };

    let query = domain_data::Query {
        ids,
        name_regexp: Some(name_regexp),
        data_type_regexp: Some(data_type_regexp),
        names,
        data_types,
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

#[no_mangle]
pub extern "C" fn free_domain_error(error: *mut DomainError) {
    if error.is_null() {
        return;
    }

    unsafe {
        let error = &mut *error;
        if !error.message.is_null() {
            free_c_string(error.message);
        }

        let _ = Box::from_raw(error);
    }
}

type FindCallback = extern "C" fn(*mut c_void, *const DomainData, *const DomainError);

/// Free a C string if it's not null
unsafe fn free_c_string(ptr: *const c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr as *mut c_char));
    }
}

#[no_mangle]
pub extern "C" fn init_domain_cluster(domain_manager_addr: *const c_char, name: *const c_char, static_relay_nodes: *const *const c_char, static_relay_nodes_len: usize) -> *mut DomainCluster {
    let name = unsafe { CStr::from_ptr(name).to_string_lossy().into_owned() };
    let domain_manager_addr = unsafe { CStr::from_ptr(domain_manager_addr).to_string_lossy().into_owned() };
    let nodes = unsafe {
        if static_relay_nodes.is_null() {
            Vec::new()
        } else {  
            std::slice::from_raw_parts(static_relay_nodes, static_relay_nodes_len)
                .iter()
                .map(|&node| CStr::from_ptr(node).to_string_lossy().into_owned())
                .collect::<Vec<String>>()
        }
    };
    let cluster = DomainCluster::new(domain_manager_addr, name, false, 0, false, false, None, None, nodes);
    Box::into_raw(Box::new(cluster))
}

#[no_mangle]
pub extern "C" fn free_domain_cluster(cluster: *mut DomainCluster) {
    if cluster.is_null() {
        return;
    }

    let cluster = unsafe { &mut *cluster };
    let _ = get_runtime().block_on(cluster.peer.client.cancel());

    unsafe {
        let _ = Box::from_raw(cluster);
    }
}

#[no_mangle]
pub extern "C" fn init_remote_storage(cluster: *mut DomainCluster) -> *mut DatastoreWrapper<RemoteDatastore> {
    Box::into_raw(Box::new(DatastoreWrapper::new(Box::new(init_r_remote_storage(cluster)))))
}

#[no_mangle]
pub extern "C" fn free_datastore(store: *mut DatastoreWrapper<RemoteDatastore>) {
    if store.is_null() {
        return;
    }

    unsafe {
        let _ = Box::from_raw(store);
    }
}

pub struct DatastoreWrapper<D: Datastore> {
    pub inner: Box<D>,
}

impl<D: Datastore> DatastoreWrapper<D> {
    fn new(store: Box<D>) -> Self {
        DatastoreWrapper { inner: store }
    }
}

#[no_mangle]
pub extern "C" fn find_domain_data(
    store: *mut DatastoreWrapper<RemoteDatastore>, // Pointer to Rust struct
    domain_id: *const c_char,    // C string
    query: *mut domain_data::Query, // Pointer to query struct
    keep_alive: bool,
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
        let stream = store_wrapper.inner.load(domain_id, query_clone, keep_alive).await.expect("Failed to load domain data");
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

pub struct ReliableDataProducer {
    pub inner: Box<dyn r_ReliableDataProducer>,
}

#[no_mangle]
pub extern "C" fn free_reliable_data_producer(producer: *mut ReliableDataProducer) {
    if producer.is_null() {
        return;
    }

    unsafe {
        let _ = Box::from_raw(producer);
    }
}

#[no_mangle]
pub extern "C" fn initialize_reliable_data_producer(
    store: *mut DatastoreWrapper<RemoteDatastore>,
    domain_id: *const c_char,
) -> *mut ReliableDataProducer {
    // Convert domain_id to Rust string
    let domain_id = unsafe {
        assert!(!domain_id.is_null());
        std::ffi::CStr::from_ptr(domain_id).to_string_lossy().into_owned()
    };

    let store_wrapper = unsafe {
        assert!(!store.is_null());
        &mut *store
    };

    let res = get_runtime().block_on(async move {
        store_wrapper.inner.upsert(domain_id).await
    });

    if res.is_err() {
        return ptr::null_mut();
    }

    let uploader = res.unwrap();
    let producer = ReliableDataProducer { inner: uploader };

    Box::into_raw(Box::new(producer))
}

#[repr(C)]
pub struct UploadResult {
    pub id: *const c_char,
    pub error: *mut DomainError,
}

#[no_mangle]
pub extern "C" fn free_upload_result(result: *mut UploadResult) {
    if result.is_null() {
        return;
    }

    unsafe {
        let result = &mut *result;
        if !result.id.is_null() {
            free_c_string(result.id);
        }

        if !result.error.is_null() {
            free_domain_error(result.error);
        }

        let _ = Box::from_raw(result);
    }
}

#[no_mangle]
pub extern "C" fn upload_domain_data(
    producer: *mut ReliableDataProducer,
    data: *const DomainData,
) -> *const UploadResult {
    let producer = unsafe {
        assert!(!producer.is_null());
        &mut *producer
    };

    let (metadata, content) = to_rust(data);

    let res: Result<String, common::DomainError> = get_runtime().block_on(async move {
        let mut chunker = producer.inner.push(&metadata).await?;
        chunker.next_chunk(&content, false).await
    });

    if res.is_err() {
        return &UploadResult { id: ptr::null(), error: &mut DomainError { message: CString::new(res.err().unwrap().to_string()).unwrap().into_raw() } };
    }

    let id = res.unwrap();
    &mut UploadResult { id: CString::new(id).unwrap().into_raw(), error: ptr::null_mut() }
}
