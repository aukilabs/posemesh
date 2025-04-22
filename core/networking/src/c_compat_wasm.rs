use wasm_bindgen::prelude::*;
use crate::{binding_helper::{posemesh_networking_context_destroy, posemesh_networking_get_commit_id}, libp2p::{Networking, NetworkingConfig}};
use wasm_bindgen_futures::{future_to_promise, js_sys::{Promise, Error}};

#[wasm_bindgen(getter_with_clone)]
#[allow(non_snake_case)]
pub struct Config {
    pub bootstraps: Vec<String>,
    pub relays: Vec<String>,
    pub privateKey: Option<Vec<u8>>,
    pub name: String,
}

#[wasm_bindgen]
impl Config {
    #[wasm_bindgen(constructor)]
    pub fn new(
        bootstraps: Vec<String>,
        relays: Vec<String>,
        private_key: Option<Vec<u8>>,
        name: String,
    ) -> Self {
        Self {
            bootstraps,
            relays,
            privateKey: private_key,
            name,
        }
    }
}

#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn posemeshNetworkingContextCreate(config: Config) -> *mut Networking {
    let config = NetworkingConfig {
        bootstrap_nodes: config.bootstraps.clone(),
        relay_nodes: config.relays.clone(),
        private_key: config.privateKey.clone(),
        private_key_path: None,
        enable_mdns: false,
        enable_kdht: true,
        enable_relay_server: false,
        name: config.name.clone(),
        port: 0,
        enable_websocket: true,
        enable_webrtc: true,
        domain: None,
    };
    let networking = Networking::new(&config).expect("posemeshNetworkingContextCreate(): failed to create networking context");
    Box::into_raw(Box::new(networking))
}

#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn posemeshNetworkingContextDestroy(context: *mut Networking) {
    posemesh_networking_context_destroy(context);
}

#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn posemeshNetworkingGetCommitId() -> String {
    posemesh_networking_get_commit_id()
}

#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn posemeshNetworkingContextSendMessage(context: *mut Networking, message: Vec<u8>, peer_id: String, protocol: String, timeout: u32) -> Promise {
    let networking = unsafe {
        assert!(!context.is_null(), "posemeshNetworkingContextSendMessage(): context is null");
        &mut *context
    };

    return future_to_promise(async move {
        match networking.client.send(message, peer_id, protocol, timeout).await {
            Ok(_) => { Ok(JsValue::from(true)) },
            Err(error) => {
                eprintln!("posemesh_networking_context_send_message(): {:?}", error);
                Err(JsValue::from(Error::new(error.to_string().as_str())))
            }
        }
    });
}
