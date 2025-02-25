use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}};
use futures::{SinkExt, StreamExt};
use networking::context::Context;
use quick_protobuf::serialize_into_vec;
use serde::de;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;
use crate::{binding_helper::init_r_remote_storage, cluster::DomainCluster as r_DomainCluster, datastore::{common::{data_id_generator, DataReader as r_DataReader, DataWriter as r_DataWriter, Datastore, DomainError, Reader as r_Reader, ReliableDataProducer as r_ReliableDataProducer}, remote::RemoteDatastore as r_RemoteDatastore}, protobuf::domain_data};
use wasm_bindgen_futures::{future_to_promise, js_sys::{self, Promise}, spawn_local};

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


#[wasm_bindgen(getter_with_clone)]
pub struct DomainData {
    pub domain_id: String,
    pub metadata: Metadata,
    pub content: *const u8,
}

#[wasm_bindgen]
impl DomainData {
    #[wasm_bindgen(constructor)]
    pub fn new(domain_id: String, metadata: Metadata, content: *const u8) -> Self {
        Self {
            domain_id,
            metadata,
            content,
        }
    }
}

#[derive(Clone)]
#[wasm_bindgen(getter_with_clone)]
pub struct Metadata {
    pub name: String,
    pub data_type: String,
    pub size: usize,
    pub properties: JsValue,
    pub id: String,
}

#[wasm_bindgen]
impl Metadata {
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, data_type: String, size: usize, properties: JsValue, id: String) -> Self {
        Self {
            name,
            data_type,
            size,
            properties,
            id,
        }
    }
}

fn from_r_metadata(r_metadata: &domain_data::Metadata) -> Metadata {
    Metadata {
        name: r_metadata.name.clone(),
        data_type: r_metadata.data_type.clone(),
        size: r_metadata.size as usize,
        properties: to_value(&r_metadata.properties).unwrap(),
        id: r_metadata.id.clone().unwrap_or("from".to_string()),
    }
}

fn to_r_metadata(metadata: Metadata) -> domain_data::Metadata {
    domain_data::Metadata {
        name: metadata.name,
        data_type: metadata.data_type,
        properties: from_value(metadata.properties).unwrap(),
        id: Some(metadata.id),
        size: metadata.size as u32,
    }
}

fn from_r_data(r_data: &domain_data::Data) -> DomainData {
    let content = r_data.content.as_slice();
    let content_size = content.len();
    DomainData {
        domain_id: r_data.domain_id.clone(),
        metadata: from_r_metadata(&r_data.metadata),
        content: content.as_ptr(),
    }
}

fn to_r_data(data: DomainData) -> domain_data::Data {
    let metadata = to_r_metadata(data.metadata);
    let content = Vec::from(unsafe { std::slice::from_raw_parts(data.content, metadata.size as usize) });
    domain_data::Data {
        domain_id: data.domain_id,
        metadata,
        content,
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
            match inner.next().await {
                Some(Ok(data)) => {
                    tracing::debug!("Got data: {:?}", data.metadata);
                    // Convert the Rust struct into a JavaScript object
                    let data = from_r_data(&data);
                    Ok(JsValue::from(data))
                }
                Some(Err(e)) => Err(JsValue::from_str(&format!("{}", e))),
                None => Ok(JsValue::NULL),
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
struct ReliableDataProducer {
    inner: r_ReliableDataProducer
}

#[wasm_bindgen]
impl ReliableDataProducer {
    #[wasm_bindgen]
    pub fn push(&mut self, data: DomainData) -> js_sys::Promise {
        let id = data.metadata.id.is_empty().then(|| data_id_generator());
        let data = domain_data::Data {
            domain_id: data.domain_id,
            metadata: domain_data::Metadata {
                name: data.metadata.name,
                data_type: data.metadata.data_type,
                properties: from_value(data.metadata.properties).unwrap(),
                id,
                size: data.metadata.size as u32,
            },
            content: Vec::from(unsafe { std::slice::from_raw_parts(data.content, data.metadata.size) }),
        };
        let mut writer = self.inner.clone();
        let future = async move {
            let res = writer.push(&data).await;
            match res {
                Ok(id) => Ok(JsValue::from_str(&id)),
                Err(e) => Err(JsValue::from_str(&format!("{}", e))),
            }
        };
        future_to_promise(future)
    }

    #[wasm_bindgen]
    pub fn is_completed(&self) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move{
            Ok(JsValue::from_bool(inner.is_completed().await))
        })
    }

    #[wasm_bindgen]
    pub fn close(&self) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async {
            inner.close().await;
            Ok(JsValue::NULL)
        })
    }
}

#[wasm_bindgen]
pub struct RemoteDatastore {
    inner: r_RemoteDatastore,
}

#[wasm_bindgen]
impl RemoteDatastore {
    #[wasm_bindgen(constructor)]
    pub fn new(cluster: DomainCluster, peer: *mut Context) -> Self {
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
            tracing::debug!("download stream");
            let stream = DataReader { inner: Arc::new(Mutex::new(stream)) };
            
            Ok(JsValue::from(stream))
        })
    }

    #[wasm_bindgen]
    pub fn produce(
        &mut self,
        domain_id: String,
    ) -> js_sys::Promise {
        let domain_id = domain_id.clone();
        let mut inner = self.inner.clone();

        future_to_promise(async move {
            let r = inner.produce(domain_id).await;
            Ok(JsValue::from(ReliableDataProducer {inner: r}))
        })
    }
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    // print pretty errors in wasm https://github.com/rustwasm/console_error_panic_hook
    // This is not needed for tracing_wasm to work, but it is a common tool for getting proper error line numbers for panics.
    console_error_panic_hook::set_once();

    // Add this line:
    tracing_wasm::set_as_global_default();

    Ok(())
}
