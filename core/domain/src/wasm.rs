use std::sync::{Arc, Mutex};
use futures::{executor::block_on, SinkExt, StreamExt};
use quick_protobuf::serialize_into_vec;
use js_sys::Function;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;
use crate::{binding_helper::init_r_remote_storage, cluster::{DomainCluster as r_DomainCluster, TaskUpdateResult}, datastore::{common::{data_id_generator, DataReader as r_DataReader, DataWriter as r_DataWriter, Datastore, DomainError, Reader as r_Reader, ReliableDataProducer as r_ReliableDataProducer}, remote::{RemoteDatastore as r_RemoteDatastore}}, protobuf::domain_data, spatial::reconstruction::reconstruction_job as r_reconstruction_job};
use wasm_bindgen_futures::{future_to_promise, js_sys::{self, Promise, Uint8Array}, spawn_local};

#[derive(Clone)]
#[wasm_bindgen]
pub struct Query {
    inner: domain_data::Query,
}

#[wasm_bindgen]
impl Query {
    #[wasm_bindgen(constructor)]
    pub fn new(ids: Vec<String>, names: Vec<String>, data_types: Vec<String>, name_regexp: Option<String>, data_type_regexp: Option<String>, metadata_only: bool) -> Self {
        Self {
            inner: domain_data::Query {
                ids,
                names,
                data_types,
                name_regexp,
                data_type_regexp,
                metadata_only,
            }
        }
    }
}


#[wasm_bindgen(getter_with_clone)]
pub struct DomainData {
    pub metadata: Metadata,
    pub content: js_sys::Uint8Array,
}

#[wasm_bindgen]
impl DomainData {
    #[wasm_bindgen(constructor)]
    pub fn new(metadata: Metadata, content: js_sys::Uint8Array) -> Self {
        Self {
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
    pub id: Option<String>,
    pub hash: Option<String>,
}

#[wasm_bindgen]
impl Metadata {
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, data_type: String, size: usize, properties: JsValue, id: Option<String>) -> Self {
        Self {
            name,
            data_type,
            size,
            properties,
            id,
            hash: None,
        }
    }
}

fn from_r_metadata(r_metadata: &domain_data::Metadata) -> Metadata {
    Metadata {
        name: r_metadata.name.clone(),
        data_type: r_metadata.data_type.clone(),
        size: r_metadata.size as usize,
        properties: to_value(&r_metadata.properties).unwrap(),
        id: Some(r_metadata.id.clone()),
        hash: r_metadata.hash.clone(),
    }
}

fn to_r_metadata(metadata: &Metadata) -> domain_data::UpsertMetadata {
    domain_data::UpsertMetadata {
        name: metadata.name.clone(),
        data_type: metadata.data_type.clone(),
        properties: from_value(metadata.properties.clone()).unwrap(),
        id: metadata.id.clone().unwrap_or_else(|| data_id_generator()),
        size: metadata.size as u32,
        is_new: metadata.id.is_none(),
    }
}

fn from_r_data(r_data: &domain_data::Data) -> DomainData {
    let content = r_data.content.as_slice();
    DomainData {
        metadata: from_r_metadata(&r_data.metadata),
        content: js_sys::Uint8Array::from(content),
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
        Self { inner: Arc::new(Mutex::new(r_DomainCluster::new(domain_manager_addr, name, false, 0, false, false, private_key, private_key_path, vec![]))) }   
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
    inner: Arc<Mutex<Box<dyn r_ReliableDataProducer>>>
}

#[wasm_bindgen]
impl ReliableDataProducer {
    pub fn push(&mut self, data: DomainData) -> Promise {
        let metadata = to_r_metadata(&data.metadata);
        let content = data.content.to_vec();
        let writer = self.inner.clone();
        let future = async move {
            let mut writer = writer.lock().unwrap();
            let res = writer.push(&metadata).await;
            drop(writer);
            match res {
                Ok(mut data_push) => {
                    match data_push.next_chunk(&content, false).await {
                        Ok(hash) => {
                            Ok(JsValue::from(hash))
                        },
                        Err(e) => Err(JsValue::from_str(&format!("{}", e))),
                    }
                },
                Err(e) => Err(JsValue::from_str(&format!("{}", e))),
            }
        };
        future_to_promise(future)
    }

    #[wasm_bindgen]
    pub fn is_completed(&self) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move{
            let inner = inner.lock().unwrap();
            Ok(JsValue::from_bool(inner.is_completed().await))
        })
    }

    #[wasm_bindgen]
    pub fn close(&self) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let mut inner = inner.lock().unwrap();
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
        callback: Function,
        keep_alive: bool
    ) -> js_sys::Promise {
        let query = query.clone();
        let mut inner = self.inner.clone();

        future_to_promise(async move {
            let res = inner.load(domain_id, query.inner, keep_alive).await;
            if let Err(e) = res {
                return Err(JsValue::from_str(&format!("{}", e)));
            }
            
            let mut res = res.unwrap();
            spawn_local(async move {
                while let Some(data) = res.next().await {
                    match data {
                        Ok(data) => {
                            let data = from_r_data(&data);
                            tracing::debug!("Consumed data: {}", data.metadata.name);
                            let js_data = JsValue::from(data);
                            callback.call2(&JsValue::NULL, &js_data, &JsValue::NULL).expect("Failed to call callback");
                        }
                        Err(e) => {
                            callback.call2(&JsValue::NULL, &JsValue::NULL, &JsValue::from_str(&format!("{}", e))).unwrap();
                        }
                    }
                }
                tracing::debug!("Consumed all data");
                callback.call2(&JsValue::NULL, &JsValue::NULL, &JsValue::NULL).unwrap();
            });
            
            Ok(JsValue::NULL)
        })
    }

    #[wasm_bindgen]
    pub fn produce(
        &mut self,
        domain_id: String,
    ) -> js_sys::Promise {
        let mut inner = self.inner.clone();

        future_to_promise(async move {
            let r = inner.upsert(domain_id).await;
            if let Err(e) = r {
                return Err(JsValue::from_str(&format!("{}", e)));
            }
            Ok(JsValue::from(ReliableDataProducer {inner: Arc::new(Mutex::new(r.unwrap()))}))
        })
    }
}

#[wasm_bindgen]
pub fn reconstruction_job(cluster: &DomainCluster, domain_id: String, scans: Vec<String>, callback: Function) -> js_sys::Promise {
    let cluster = cluster.inner.lock().unwrap();
    let cluster_clone = cluster.clone();
    drop(cluster);

    future_to_promise(async move {
        let mut r = r_reconstruction_job(cluster_clone, &domain_id, scans).await;
        spawn_local(async move {
            while let Some(task_update) = r.next().await {
                match task_update.result {
                    TaskUpdateResult::Ok(task) => {
                        tracing::debug!("Task {}-{} update status {:?}", task.job_id, task.name, task.status);
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

    tracing::info!("Starting log for domain");

    Ok(())
}
