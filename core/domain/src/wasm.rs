use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}};
use futures::{executor::block_on, SinkExt, StreamExt};
use networking::libp2p::Networking;
use quick_protobuf::serialize_into_vec;
use serde::de;
use js_sys::Function;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;
use crate::{binding_helper::init_r_remote_storage, cluster::{DomainCluster as r_DomainCluster, TaskUpdateResult}, datastore::{common::{data_id_generator, DataReader as r_DataReader, DataWriter as r_DataWriter, Datastore, DomainError, Reader as r_Reader, ReliableDataProducer as r_ReliableDataProducer}, remote::RemoteDatastore as r_RemoteDatastore}, protobuf::domain_data, spatial::reconstruction::reconstruction_job as r_reconstruction_job};
use wasm_bindgen_futures::{future_to_promise, js_sys::{self, Promise, Uint8Array}, spawn_local};

#[derive(Clone)]
#[wasm_bindgen]
pub struct Query {
    inner: domain_data::Query,
}

#[wasm_bindgen]
impl Query {
    #[wasm_bindgen(constructor)]
    pub fn new(ids: Vec<String>, names: Vec<String>, data_types: Vec<String>, name_regexp: Option<String>, data_type_regexp: Option<String>) -> Self {
        Self {
            inner: domain_data::Query {
                ids,
                names,
                data_types,
                name_regexp,
                data_type_regexp,
            }
        }
    }
}


#[wasm_bindgen(getter_with_clone)]
pub struct DomainData {
    pub domain_id: String,
    pub metadata: Metadata,
    pub content: js_sys::Uint8Array,
}

#[wasm_bindgen]
impl DomainData {
    #[wasm_bindgen(constructor)]
    pub fn new(domain_id: String, metadata: Metadata, content: js_sys::Uint8Array) -> Self {
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

fn to_r_metadata(metadata: &Metadata) -> domain_data::Metadata {
    domain_data::Metadata {
        name: metadata.name.clone(),
        data_type: metadata.data_type.clone(),
        properties: from_value(metadata.properties.clone()).unwrap(),
        id: Some(metadata.id.clone()),
        size: metadata.size as u32,
    }
}

fn from_r_data(r_data: &domain_data::Data) -> DomainData {
    let content = r_data.content.as_slice();
    DomainData {
        domain_id: r_data.domain_id.clone(),
        metadata: from_r_metadata(&r_data.metadata),
        content: js_sys::Uint8Array::from(content),
    }
}

fn to_r_data(data: &DomainData) -> domain_data::Data {
    let metadata = to_r_metadata(&data.metadata);

    let content = data.content.to_vec();
    
    domain_data::Data {
        domain_id: data.domain_id.clone(),
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
    pub fn new(domain_manager_addr: String, name: String, private_key: Option<Vec<u8>>, private_key_path: Option<String>) -> Self {
        Self { inner: Arc::new(Mutex::new(r_DomainCluster::new(domain_manager_addr, name, false, 0, false, false, private_key, private_key_path))) }   
    }

    #[wasm_bindgen]
    pub fn monitor(&self, callback: Function) {
        let inner = self.inner.clone();
        block_on(async move {
            let mut rx = inner.lock().unwrap().monitor_jobs().await;
            while let Some(job) = rx.next().await {
                let job_bytes = serialize_into_vec(&job).unwrap();
                let js_arr = Uint8Array::from(&job_bytes[..]);
                callback.call1(&JsValue::NULL, &js_arr).unwrap();
            }
        });
    }
}

#[wasm_bindgen]
struct ReliableDataProducer {
    inner: r_ReliableDataProducer
}

#[wasm_bindgen]
impl ReliableDataProducer {
    #[wasm_bindgen]
    pub fn push(&mut self, mut data: DomainData) -> js_sys::Promise {
        let id = data.metadata.id.is_empty().then(|| data_id_generator());
        data.metadata.id = id.unwrap();
        let data = to_r_data(&data);
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
    pub fn new(cluster: &DomainCluster) -> Self {
        let r_domain_cluster = cluster.inner.lock().unwrap();
        let cluster = r_domain_cluster.clone();

        Self { inner: init_r_remote_storage(Box::into_raw(Box::new(cluster))) }
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
            let stream = inner.consume(domain_id, query.inner, false).await;
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

#[wasm_bindgen]
pub fn reconstruction_job(cluster: &DomainCluster, scans: Vec<String>, callback: Function) -> js_sys::Promise {
    let cluster = cluster.inner.lock().unwrap();
    let cluster_clone = cluster.clone();
    drop(cluster);

    future_to_promise(async move {
        let mut r = r_reconstruction_job(cluster_clone, scans).await;
        spawn_local(async move {
            while let Some(task_update) = r.next().await {
                match task_update.result {
                    TaskUpdateResult::Ok(task) => {
                        let task_update_bytes = serialize_into_vec(&task).unwrap();
                        let js_arr = Uint8Array::from(&task_update_bytes[..]);
                        callback.call1(&JsValue::NULL, &js_arr).unwrap();
                    }
                    TaskUpdateResult::Err(e) => {
                        tracing::error!("Error: {}", e);
                    }
                }
            }
        });
        Ok(JsValue::NULL)
    })
}
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    // print pretty errors in wasm https://github.com/rustwasm/console_error_panic_hook
    // This is not needed for tracing_wasm to work, but it is a common tool for getting proper error line numbers for panics.
    console_error_panic_hook::set_once();

    // Add this line:
    tracing_wasm::set_as_global_default();

    tracing::info!("Starting domain-core");

    Ok(())
}
