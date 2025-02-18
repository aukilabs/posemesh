use std::sync::{Arc, Mutex};
use networking::context::Context;
use quick_protobuf::serialize_into_vec;
use wasm_bindgen::prelude::*;
use crate::{binding_helper::init_r_remote_storage, cluster::DomainCluster as r_DomainCluster, datastore::Datastore, protobuf::domain_data, remote::{DataStream as r_DataStream, RemoteDatastore as r_RemoteDatastore}};
use wasm_bindgen_futures::{future_to_promise, js_sys};

#[derive(Clone)]
#[wasm_bindgen]
pub struct Query {
    inner: domain_data::Query,
}

#[wasm_bindgen]
impl Query {
    #[wasm_bindgen(constructor)]
    pub fn new(ids: Vec<String>, name: Option<String>, data_type: Option<String>) -> Self {
        Self {
            inner: domain_data::Query {
                ids,
                name,
                data_type,
            }
        }
    }
}


#[wasm_bindgen]
pub struct DomainData {
    // pub domain_id: String,
    // pub metadata: Metadata,
    // pub content: *const u8,
    // pub content_size: usize,
    pub data: *const u8,
    pub size: usize,
}

impl DomainData {
    pub fn new(r_data: &domain_data::Data) -> Self {
        let serialized = serialize_into_vec(r_data).expect("Failed to serialize metadata");
        DomainData {
            data: serialized.as_ptr(),
            size: serialized.len(),
        }
    }
}

#[wasm_bindgen]
pub struct DataStream {
    inner: Arc<Mutex<r_DataStream>>,
}

#[wasm_bindgen]
impl DataStream {
    #[wasm_bindgen]
    pub fn next(&mut self) -> js_sys::Promise {
        let inner = self.inner.clone();
        let future = async move {
            let mut inner = inner.lock().unwrap();
            // Attempt to get the next item from the stream
            match inner.try_next() {
                Ok(Some(Ok(data))) => {
                    // Convert the Rust struct into a JavaScript object
                    let data = DomainData::new(&data);
                    Ok(JsValue::from(data))
                }
                Ok(Some(Err(e))) => Err(JsValue::from_str(&format!("{}", e))),
                Ok(None) => Ok(JsValue::NULL),
                Err(e) => Err(JsValue::from_str(&format!("{}", e))),
            }
        };
        // Convert the Rust Future into a JavaScript Promise
        future_to_promise(future)
    }

    #[wasm_bindgen]
    pub fn close(&mut self) {
        let mut inner = self.inner.lock().unwrap();
        inner.close();
    }
}

#[wasm_bindgen]
pub struct DomainCluster {
    inner: Arc<Mutex<r_DomainCluster>>,
}

#[wasm_bindgen]
impl DomainCluster {
    #[wasm_bindgen(constructor)]
    pub fn new(domain_manager_id: String, context: *mut Context) -> Self {
        let context = Box::new(unsafe { (*context).clone() });
        Self { inner: Arc::new(Mutex::new(r_DomainCluster::new(domain_manager_id, context))) }   
    }
}

#[wasm_bindgen]
pub struct RemoteDatastore {
    inner: r_RemoteDatastore,
}

#[wasm_bindgen]
impl RemoteDatastore {
    #[wasm_bindgen(constructor)]
    pub fn new(cluster: DomainCluster, peer: *mut Context ) -> Self {
        let r_domain_cluster = cluster.inner.lock().unwrap();
        let cluster = r_domain_cluster.clone();

        Self { inner: init_r_remote_storage(Box::into_raw(Box::new(cluster)), peer) }
    }

    #[wasm_bindgen]
    pub fn find(
        &mut self,
        domain_id: String,
        query: Query,
    ) -> js_sys::Promise {
        let domain_id = domain_id.clone();
        // TODO: Convert query to Rust struct
        let query = query.clone();
        let mut inner = self.inner.clone();

        future_to_promise(async move {
            let stream = inner.find(domain_id, query.inner).await;
            let stream = DataStream { inner: Arc::new(Mutex::new(stream)) };
            
            Ok(JsValue::from(stream))
        })
    }
}
