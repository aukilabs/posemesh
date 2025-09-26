use crate::DomainClient as r_DomainClient;
use crate::domain_data::{
    DomainData, DownloadQuery as r_DownloadQuery, UploadDomainData as r_UploadDomainData,
};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsError, JsValue};
use wasm_bindgen_futures::{
    future_to_promise,
    js_sys::{Promise, Uint8Array},
};
use wasm_streams::readable::sys;

#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"

export type DownloadQuery = { ids: string[], name: string | null, data_type: string | null };
/**
 * UploadDomainData is a union of CreateDomainData and UpdateDomainData
 * CreateDomainData is an object with the following fields:
 * - name: string
 * - data_type: string
 * - data: Uint8Array
 * UpdateDomainData is an object with the following fields:
 * - id: string
 */
export type UploadDomainData = { id?: string, name?: string, data_type?: string, data: Uint8Array };
export type DomainDataMetadata = { id: string, name: string, data_type: string, size: number, created_at: string, updated_at: string };
export type DomainData = { metadata: DomainDataMetadata, data: Uint8Array };

/**
 * Signs in with application credentials to obtain a DomainClient instance. Make sure to call .free() to free the memory when you are done with the client.
 * 
 * @param api_url - The base URL for the API service.
 * @param dds_url - The URL for the Domain Discovery Service.
 * @param client_id - Unique identifier for this client.
 * @param app_key - Application key for authentication.
 * @param app_secret - Application secret for authentication.
 * @returns Promise that resolves to a DomainClient instance.
 * 
 * @example
 * const client = await signInWithAppCredential(
 *   "https://api.auki.network",
 *   "https://dds.auki.network",
 *   "my-client-id",
 *   "app-key-123",
 *   "app-secret-456"
 * );
 * client.free(); // free the memory when you are done with the client
 */
export function signInWithAppCredential(
    api_url: string,
    dds_url: string,
    client_id: string,
    app_key: string,
    app_secret: string
): Promise<DomainClient>;

/**
 * Signs in with user credentials to obtain a DomainClient instance. Make sure to call .free() to free the memory when you are done with the client.
 * 
 * @param api_url - The base URL for the API service.
 * @param dds_url - The URL for the Domain Discovery Service.
 * @param client_id - Unique identifier for this client.
 * @param email - User's email address.
 * @param password - User's password.
 * @param remember_password - Set to `true` if you want to automatically relogin with the same credentials after refreshtoken expires, it is NOT recommended to set to `true` in client side as storing credentials in the browser increases security risks (e.g., XSS attacks).
 * @returns Promise that resolves to a DomainClient instance.
 * 
 * @example
 * const client = await signInWithUserCredential(
 *   "https://api.auki.network",
 *   "https://dds.auki.network",
 *   "my-client-id",
 *   "user@example.com",
 *   "password123",
 *   false
 * );
 * client.free(); // free the memory when you are done with the client
 */
export function signInWithUserCredential(
    api_url: string,
    dds_url: string,
    client_id: string,
    email: string,
    password: string,
    logout: boolean
): Promise<DomainClient>;

"#;

/// WASM wrapper for DomainClient that provides JavaScript bindings
///
/// This struct wraps the Rust DomainClient and exposes its functionality
/// to JavaScript through WASM bindings. It handles authentication,
/// domain data upload/download, and metadata operations.
#[wasm_bindgen(getter_with_clone)]
pub struct DomainClient {
    domain_client: r_DomainClient,
}

#[wasm_bindgen(js_name = "signInWithAppCredential")]
pub fn sign_in_with_app_credential(
    api_url: String,
    dds_url: String,
    client_id: String,
    app_key: String,
    app_secret: String,
) -> Promise {
    let future = async move {
        let res = r_DomainClient::new_with_app_credential(
            &api_url,
            &dds_url,
            &client_id,
            &app_key,
            &app_secret,
        )
        .await;
        match res {
            Ok(domain_client) => Ok(JsValue::from(DomainClient {
                domain_client: domain_client,
            })),
            Err(e) => Err(JsError::new(&e.to_string()).into()),
        }
    };
    future_to_promise(future)
}

#[wasm_bindgen(js_name = "signInWithUserCredential")]
pub fn sign_in_with_user_credential(
    api_url: String,
    dds_url: String,
    client_id: String,
    email: String,
    password: String,
    remember_password: bool,
) -> Promise {
    let future = async move {
        let res = r_DomainClient::new_with_user_credential(
            &api_url,
            &dds_url,
            &client_id,
            &email,
            &password,
            remember_password,
        )
        .await;
        match res {
            Ok(domain_client) => Ok(JsValue::from(DomainClient {
                domain_client: domain_client,
            })),
            Err(e) => Err(JsError::new(&e.to_string()).into()),
        }
    };
    future_to_promise(future)
}

#[wasm_bindgen]
impl DomainClient {
    /// Constructs a new DomainClient instance. Make sure to call .free() to free the memory when you are done with the client.
    ///
    /// # Arguments
    /// * `api_url` - The base URL for the API service.
    /// * `dds_url` - The URL for the Domain Discovery Service.
    /// * `client_id` - Unique identifier for this client.
    ///
    /// # Returns
    /// * `Self` - A new DomainClient instance.
    ///
    /// # Example
    /// ```javascript
    /// const client = new DomainClient(
    ///     "https://api.example.com".to_string(),
    ///     "https://dds.example.com".to_string(),
    ///     "my-client-id".to_string()
    /// );
    /// 
    /// // free the memory when you are done with the client
    /// client.free();
    /// ```
    /// 
    #[wasm_bindgen(constructor)]
    pub fn new(api_url: String, dds_url: String, client_id: String) -> Self {
        Self {
            domain_client: r_DomainClient::new(&api_url, &dds_url, &client_id),
        }
    }

    /// Returns a new DomainClient instance with the given Zitadel token for authentication. Make sure to call .free() to free the memory when you are done with the client.
    ///
    /// # Arguments
    /// * `zitadel_token` - The Zitadel access token.
    ///
    /// # Returns
    /// * `Self` - A new DomainClient instance with the token applied.
    ///
    /// # Example
    /// ```javascript
    /// const client_with_token = client.withZitadelToken("your-zitadel-token");
    /// 
    /// // free the memory when you are done with the client
    /// client_with_token.free();
    /// ```
    #[wasm_bindgen(js_name = "withZitadelToken")]
    pub fn with_zitadel_token(&self, zitadel_token: String) -> Self {
        Self {
            domain_client: self.domain_client.with_zitadel_token(&zitadel_token),
        }
    }

    /// Downloads metadata for domain data matching the query.
    ///
    /// # Arguments
    /// * `domain_id` - The ID of the domain.
    /// * `query` - The query `DownloadQuery` parameters for filtering data.
    ///
    /// # Returns
    /// * `Promise<DomainDataMetadata[]>` - Resolves to an array of DomainDataMetadata.
    ///
    /// # Example
    /// ```javascript
    /// let metadata: DomainDataMetadata[] = await client.downloadDomainDataMetadata(
    ///     "domain-123",
    ///     { ids: [], name: null, data_type: "data type" }
    /// );
    /// ```
    #[wasm_bindgen(js_name = "downloadDomainDataMetadata")]
    pub fn download_domain_data_metadata(&self, domain_id: String, query: JsValue) -> Promise {
        let domain_client = self.domain_client.clone();
        let future = async move {
            let parse = from_value::<r_DownloadQuery>(query);
            if let Err(e) = parse {
                return Err(JsError::new(&e.to_string()).into());
            }
            let query = parse.unwrap();
            let res = domain_client.download_metadata(&domain_id, &query).await;
            match res {
                Ok(data) => match to_value(&data) {
                    Ok(value) => Ok(value),
                    Err(e) => Err(JsError::new(&e.to_string()).into()),
                },
                Err(e) => Err(JsError::new(&e.to_string()).into()),
            }
        };
        future_to_promise(future)
    }

    /// Downloads domain data matching the query, including the data bytes.
    ///
    /// # Arguments
    /// * `domain_id` - The ID of the domain.
    /// * `query` - The query `DownloadQuery` parameters for filtering data.
    ///
    /// # Returns
    /// * `Promise<DomainData[]>` - Resolves to an array of DomainData.
    ///
    /// # Example
    /// ```javascript
    /// let data: DomainData[] = await client.downloadDomainData(
    ///     "domain-123",
    ///     { ids: [], name: null, data_type: "data type" }
    /// );
    /// ```
    #[wasm_bindgen(js_name = "downloadDomainData")]
    pub fn download_domain_data(&self, domain_id: String, query: JsValue) -> Promise {
        let domain_client = self.domain_client.clone();
        let future = async move {
            use futures::StreamExt;
            let parse = from_value::<r_DownloadQuery>(query);
            if let Err(e) = parse {
                return Err(JsError::new(&e.to_string()).into());
            }
            let query = parse.unwrap();
            let res = domain_client.download_domain_data(&domain_id, &query).await;
            if let Err(e) = res {
                return Err(JsError::new(&e.to_string()).into());
            }
            let mut rx = res.unwrap();
            let mut response: Vec<DomainData> = vec![];
            while let Some(result) = rx.next().await {
                match result {
                    Ok(data) => {
                        response.push(data);
                    }
                    Err(e) => {
                        return Err(JsError::new(&e.to_string()).into());
                    }
                }
            }

            to_value(&response).map_err(|e| JsError::new(&e.to_string()).into())
        };
        future_to_promise(future)
    }

    /// Downloads domain data as a readable stream, matching the query.
    ///
    /// # Arguments
    /// * `domain_id` - The ID of the domain.
    /// * `query` - The query `DownloadQuery` parameters for filtering data.
    ///
    /// # Returns
    /// * `ReadableStream<DomainData>` - A JavaScript ReadableStream of DomainData objects.
    ///
    /// # Example
    /// ```javascript
    /// let stream: ReadableStream<DomainData> = client.downloadDomainDataStream(
    ///     "domain-123",
    ///     query_js_value
    /// );
    /// ```
    #[wasm_bindgen(js_name = "downloadDomainDataStream")]
    pub fn download_domain_data_stream(
        &self,
        domain_id: String,
        query: JsValue,
    ) -> sys::ReadableStream {
        use futures::{SinkExt, StreamExt};
        use wasm_bindgen_futures::spawn_local;
        // We'll use a futures channel to push items into JS
        let (mut tx, rx) = futures::channel::mpsc::unbounded::<Result<JsValue, JsValue>>();
        let domain_client = self.domain_client.clone();
        // Spawn a Rust async task that sends data
        spawn_local(async move {
            let query = match from_value::<r_DownloadQuery>(query) {
                Ok(q) => q,
                Err(e) => {
                    tx.send(Err(JsError::new(&e.to_string()).into())).await.ok();
                    return;
                }
            };
            let res = domain_client.download_domain_data(&domain_id, &query).await;
            if let Ok(mut download_rx) = res {
                while let Some(result) = download_rx.next().await {
                    match result {
                        Ok(data) => match to_value(&data) {
                            Ok(value) => {
                                tx.send(Ok(value)).await.ok();
                            }
                            Err(e) => {
                                tx.send(Err(JsError::new(&e.to_string()).into())).await.ok();
                            }
                        },
                        Err(e) => {
                            tx.send(Err(JsError::new(&e.to_string()).into())).await.ok();
                        }
                    }
                }
            }
        });

        wasm_streams::ReadableStream::into_raw(wasm_streams::ReadableStream::from_stream(rx))
    }

    /// Uploads domain data (create or update):
    ///
    /// # Arguments
    /// * `domain_id` - The ID of the domain.
    /// * `data`: `UploadDomainData[]` - The array of UploadDomainData objects.
    ///
    /// # Returns
    /// * `Promise<DomainDataMetadata[]>` - Resolves to an array of DomainDataMetadata.
    ///
    /// # Example
    /// ```javascript
    /// let result: DomainDataMetadata[] = await client.uploadDomainData(
    ///     "domain-123",
    ///     [{
    ///         name: "test",
    ///         data_type: "test",
    ///         data: new Uint8Array([1, 2, 3])
    ///     }, {
    ///         id: "data-id-456",
    ///         data: new Uint8Array([1, 2, 3])
    ///     }]
    /// ] as UploadDomainData[]
    /// );
    /// ```
    #[wasm_bindgen(js_name = "uploadDomainData")]
    pub fn upload_domain_data(&self, domain_id: String, data: JsValue) -> Promise {
        let domain_client = self.domain_client.clone();
        let future = async move {
            match from_value::<Vec<r_UploadDomainData>>(data) {
                Ok(upload) => {
                    let res = domain_client.upload_domain_data(&domain_id, upload).await;
                    match res {
                        Ok(data) => match to_value(&data) {
                            Ok(value) => Ok(value),
                            Err(e) => Err(JsError::new(&e.to_string()).into()),
                        },
                        Err(e) => Err(JsError::new(&e.to_string()).into()),
                    }
                }
                Err(e) => Err(JsError::new(&e.to_string()).into()),
            }
        };
        future_to_promise(future)
    }

    /// Downloads the raw data bytes for a specific domain data object by its ID.
    ///
    /// # Arguments
    /// * `domain_id` - The ID of the domain.
    /// * `id` - The ID of the data object to download.
    ///
    /// # Returns
    /// * `Promise<Uint8Array>` - Resolves to a Uint8Array containing the data bytes.
    ///
    /// # Example
    /// ```javascript
    /// let bytes: Uint8Array = await client.downloadDomainDataById(
    ///     "domain-123",
    ///     "data-id-456"
    /// );
    /// ```
    #[wasm_bindgen(js_name = "downloadDomainDataById")]
    pub fn download_domain_data_by_id(&self, domain_id: String, id: String) -> Promise {
        let domain_client = self.domain_client.clone();
        let future = async move {
            let res = domain_client
                .download_domain_data_by_id(&domain_id, &id)
                .await;
            match res {
                Ok(data) => Ok(JsValue::from(Uint8Array::from(data.as_slice()))),
                Err(e) => Err(JsError::new(&e.to_string()).into()),
            }
        };
        future_to_promise(future)
    }

    /// Deletes a domain data object by ID.
    ///
    /// # Arguments
    /// * `domain_id` - The ID of the domain.
    /// * `id` - The ID of the data object to delete.
    ///
    /// # Returns
    /// * `Promise<void>` - Resolves when the deletion is complete.
    ///
    /// # Example
    /// ```javascript
    /// await client.deleteDomainDataById(
    ///     "domain-123",
    ///     "data-id-456"
    /// );
    /// ```
    #[wasm_bindgen(js_name = "deleteDomainDataById")]
    pub fn delete_domain_data_by_id(&self, domain_id: String, id: String) -> Promise {
        let domain_client = self.domain_client.clone();
        let future = async move {
            let res = domain_client
                .delete_domain_data_by_id(&domain_id, &id)
                .await;
            match res {
                Ok(()) => Ok(JsValue::undefined()),
                Err(e) => Err(JsError::new(&e.to_string()).into()),
            }
        };
        future_to_promise(future)
    }
}

/// Initializes the WASM module with logging and error handling
///
/// This function is automatically called when the WASM module is loaded.
/// It sets up console error handling for panics and configures tracing
/// for comprehensive logging in the browser environment.
///
/// # Example
/// ```javascript
/// // This is automatically called when the WASM module loads
/// // No manual call needed
/// ```
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
