use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsValue, JsError};
use crate::domain_data::{DownloadQuery as r_DownloadQuery, UploadDomainData};
use wasm_bindgen_futures::{future_to_promise, js_sys::{Promise, Function}};
use crate::DomainClient as r_DomainClient;

#[wasm_bindgen(getter_with_clone)]
pub struct DomainClient {
    domain_client: r_DomainClient,
}

#[wasm_bindgen(getter_with_clone)]
pub struct DownloadQuery {
    inner: r_DownloadQuery,
}

#[wasm_bindgen]
impl DownloadQuery {
    #[wasm_bindgen(constructor)]
    pub fn new(ids: Vec<String>, name: Option<String>, data_type: Option<String>) -> Self {
        Self { inner: r_DownloadQuery { ids, name, data_type } }
    }
}

// Sign in with app credential, return a DomainClient
#[wasm_bindgen(js_name = "signInWithAppCredential")]
pub fn sign_in_with_app_credential(api_url: String, dds_url: String, client_id: String, app_key: String, app_secret: String) -> Promise {
    let future = async move {
        let res = r_DomainClient::new_with_app_credential(&api_url, &dds_url, &client_id, &app_key, &app_secret).await;
        match res {
            Ok(domain_client) => Ok(JsValue::from(DomainClient { domain_client: domain_client })),
            Err(e) => Err(JsError::new(&e.to_string()).into()),
        }
    };
    future_to_promise(future)
}

// Sign in with user credential, return a DomainClient
#[wasm_bindgen(js_name = "signInWithUserCredential")]
pub fn sign_in_with_user_credential(api_url: String, dds_url: String, client_id: String, email: String, password: String, logout: bool) -> Promise {    
    let future = async move {
        let res = r_DomainClient::new_with_user_credential(&api_url, &dds_url, &client_id, &email, &password, logout).await;
        match res {
            Ok(domain_client) => Ok(JsValue::from(DomainClient { domain_client: domain_client })),
            Err(e) => Err(JsError::new(&e.to_string()).into()),
        }
    };
    future_to_promise(future)
}

#[wasm_bindgen]
impl DomainClient {
    #[wasm_bindgen(js_name = "downloadDomainDataMetadata")]
    pub fn download_domain_data_metadata(
        &self,
        domain_id: String,
        query: DownloadQuery,
    ) -> Promise {
        tracing::info!("download_domain_data_metadata: {:?}, {:?}", query.inner, self.domain_client.clone());
        let domain_client = self.domain_client.clone();
        let future = async move {
            let res = domain_client.download_metadata(&domain_id, &query.inner).await;
            match res {
                Ok(data) => Ok(JsValue::from(data)),
                Err(e) => Err(JsError::new(&e.to_string()).into()),
            }
        };
        future_to_promise(future)
    }

    #[wasm_bindgen(js_name = "downloadDomainData")]
    pub fn download_domain_data(
        &self,
        domain_id: String,
        query: DownloadQuery,
        callback: Function,
    ) -> Promise {
        let domain_client = self.domain_client.clone();
        let future = async move {
            use futures::StreamExt;
            let res = domain_client.download_domain_data(&domain_id, &query.inner).await;
            match res {
                Ok(mut rx) => {
                    while let Some(result) = rx.next().await {
                        match result {
                            Ok(data) => {
                                match callback.call1(&JsValue::NULL, &JsValue::from(data)) {
                                    Ok(_) => {
                                        continue;
                                    }
                                    Err(e) => {
                                        return Err(e);
                                    }
                                }
                            }
                            Err(e) => {
                                return Err(JsError::new(&e.to_string()).into());
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(JsError::new(&e.to_string()).into());
                }
            }
            Ok(JsValue::NULL)
        };
        future_to_promise(future)
    }

    #[wasm_bindgen(js_name = "uploadDomainData")]
    pub fn upload_domain_data(
        &self,
        domain_id: String,
        data: Vec<UploadDomainData>,
    ) -> Promise {
        let domain_client = self.domain_client.clone();
        let future = async move {
            let res = domain_client.upload_domain_data(&domain_id, data).await;
            match res {
                Ok(data) => Ok(JsValue::from(data)),
                Err(e) => Err(JsError::new(&e.to_string()).into()),
            }
        };
        future_to_promise(future)
    }
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    // print pretty errors in wasm https://github.com/rustwasm/console_error_panic_hook
    // This is not needed for tracing_wasm to work, but it is a common tool for getting proper error line numbers for panics.
    console_error_panic_hook::set_once();

    // Configure tracing for WASM with comprehensive settings for async operations
    let config = tracing_wasm::WASMLayerConfigBuilder::new()
        .set_max_level(tracing::Level::DEBUG)
        .build();
    
    tracing_wasm::set_as_global_default_with_config(config);

    // Ensure tracing is properly initialized
    tracing::info!("Starting log for DOMAIN-HTTP");

    Ok(())
}
