use std::{collections::HashMap, sync::{Arc, Mutex}};
use futures::SinkExt;
use networking::context::Context;
use quick_protobuf::serialize_into_vec;
use wasm_bindgen::prelude::*;
use crate::{binding_helper::init_r_remote_storage, cluster::DomainCluster as r_DomainCluster, datastore::{common::{Datastore, DataReader as r_DataReader, DataWriter as r_DataWriter}, remote::RemoteDatastore as r_RemoteDatastore}, protobuf::domain_data};
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
    pub domain_id: String,
    pub metadata: Metadata,
    pub content: *const u8,
}

#[wasm_bindgen]
pub struct Metadata {
    pub name: String,
    pub data_type: String,
    pub size: usize,
    pub properties: HashMap<String, String>,
    pub id: Option<String>,
}

impl DomainData {
    pub fn new(r_data: &domain_data::Data) -> Self {
        let content = r_data.content.as_slice();
        let content_size = content.len();
        Self {
            domain_id: r_data.domain_id.clone(),
            metadata: Metadata {
                name: r_data.metadata.name.clone(),
                data_type: r_data.metadata.data_type.clone(),
                size: content_size,
                properties: r_data.metadata.properties.clone(),
                id: r_data.metadata.id.clone(),
            },
            content: content.as_ptr(),
        }
    }
}

#[wasm_bindgen]
pub struct DataReader {
    inner: Arc<Mutex<r_DataReader>>,
}

#[wasm_bindgen]
impl DataReader {
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
pub struct DataWriter {
    inner: r_DataWriter,
}

#[wasm_bindgen]
impl DataWriter {
    #[wasm_bindgen]
    pub fn push(&mut self, data: DomainData) {
        let data = domain_data::Data {
            domain_id: data.domain_id,
            metadata: domain_data::Metadata {
                name: data.metadata.name,
                data_type: data.metadata.data_type,
                properties: data.metadata.properties,
                id: data.metadata.id,
                size: data.metadata.size as u32,
            },
            content: Vec::from(unsafe { std::slice::from_raw_parts(data.content, data.metadata.size) }),
        };

        future_to_promise(async move {
            let res = self.inner.send(Ok(data)).await;
            match res {
                Ok(_) => Ok(JsValue::NULL),
                Err(e) => Err(JsValue::from_str(&format!("{}", e))),
            }
        });
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
    pub fn consume(
        &mut self,
        domain_id: String,
        query: Query,
    ) -> js_sys::Promise {
        let domain_id = domain_id.clone();
        // TODO: Convert query to Rust struct
        let query = query.clone();
        let mut inner = self.inner.clone();

        future_to_promise(async move {
            let stream = inner.consume(domain_id, query.inner).await;
            let stream = DataReader { inner: Arc::new(Mutex::new(stream)) };
            
            Ok(JsValue::from(stream))
        })
    }

    #[wasm_bindgen]
    pub fn produce(
        &mut self,
        domain_id: String,
        data: *const u8,
        size: usize,
    ) -> js_sys::Promise {
        let domain_id = domain_id.clone();
        let data = unsafe { std::slice::from_raw_parts(data, size) };
        let data = data.to_vec();
        let mut inner = self.inner.clone();

        future_to_promise(async move {
            let writer = inner.produce(domain_id).await;
            Ok(JsValue::NULL)
        })
    }
}
