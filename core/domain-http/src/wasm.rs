use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsValue, JsError};
use crate::domain_data::{DownloadQuery, UploadDomainData};
use wasm_bindgen_futures::{future_to_promise, js_sys::{Promise, Function}};
use crate::DomainClient as r_DomainClient;

#[wasm_bindgen(getter_with_clone)]
pub struct DomainClient {
    domain_client: r_DomainClient,
}

// Sign in with app credential, return a DomainClient
#[wasm_bindgen(js_name = "signInWithAppCredential")]
pub fn sign_in_with_app_credential(api_url: String, dds_url: String, client_id: String, app_key: String, app_secret: String) -> Promise {
    let future = async move {
        let res = r_DomainClient::new_with_app_credential(&api_url, &dds_url, &client_id, &app_key, &app_secret).await;
        match res {
            Ok(domain_client) => Ok(JsValue::from(DomainClient { domain_client })),
            Err(e) => Err(JsError::new(&e.to_string()).into()),
        }
    };
    future_to_promise(future)
}

// Sign in with user credential, return a DomainClient
#[wasm_bindgen(js_name = "signInWithUserCredential")]
pub fn sign_in_with_user_credential(api_url: String, dds_url: String, client_id: String, email: String, password: String) -> Promise {
    let future = async move {
        let res = r_DomainClient::new_with_user_credential(&api_url, &dds_url, &client_id, &email, &password).await;
        match res {
            Ok(domain_client) => Ok(JsValue::from(DomainClient { domain_client })),
            Err(e) => Err(JsError::new(&e.to_string()).into()),
        }
    };
    future_to_promise(future)
}

#[wasm_bindgen]
impl DomainClient {
    #[wasm_bindgen(js_name = "downloadDomainDataWithAppCredential")]
    pub fn download_domain_data(
        self,
        domain_id: String,
        query: DownloadQuery,
        callback: Function,
    ) -> Promise {
        let future = async move {
            use futures::StreamExt;
            let res = self.domain_client.download_domain_data(&domain_id, &query).await;
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
        self,
        domain_id: String,
        data: Vec<UploadDomainData>,
    ) -> Promise {
        let future = async move {
            let res = self.domain_client.upload_domain_data(&domain_id, data).await;
            match res {
                Ok(_) => Ok(JsValue::NULL),
                Err(e) => Err(JsError::new(&e.to_string()).into()),
            }
        };
        future_to_promise(future)
    }
}

