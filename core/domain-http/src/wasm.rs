use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsValue, JsError};
use crate::domain_data::{DownloadQuery as r_DownloadQuery, UploadDomainData};
use wasm_bindgen_futures::{future_to_promise, js_sys::{Promise, Function, Uint8Array}};
use crate::DomainClient as r_DomainClient;

/// WASM wrapper for DomainClient that provides JavaScript bindings
/// 
/// This struct wraps the Rust DomainClient and exposes its functionality
/// to JavaScript through WASM bindings. It handles authentication,
/// domain data upload/download, and metadata operations.
#[wasm_bindgen(getter_with_clone)]
pub struct DomainClient {
    domain_client: r_DomainClient,
}

/// WASM wrapper for DownloadQuery that provides JavaScript bindings
/// 
/// Represents a query for downloading domain data with optional filters
/// for specific IDs, names, or data types.
#[wasm_bindgen(getter_with_clone)]
pub struct DownloadQuery {
    inner: r_DownloadQuery,
}

#[wasm_bindgen]
impl DownloadQuery {
    /// Creates a new DownloadQuery with optional filters
    /// 
    /// # Arguments
    /// * `ids` - Vector of specific data IDs to download (empty for all)
    /// * `name` - Optional name filter for the data
    /// * `data_type` - Optional data type filter
    /// 
    /// # Example
    /// ```javascript
    /// // Download all data
    /// const query = new DownloadQuery([], null, null);
    /// 
    /// // Download specific data by IDs
    /// const query = new DownloadQuery(["id1", "id2"], null, null);
    /// 
    /// // Download data with name filter
    /// const query = new DownloadQuery([], "my-data", null);
    /// 
    /// // Download data with type filter
    /// const query = new DownloadQuery([], null, "image");
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(ids: Vec<String>, name: Option<String>, data_type: Option<String>) -> Self {
        Self { inner: r_DownloadQuery { ids, name, data_type } }
    }
}

/// Signs in with app credentials to have read access to all domains in the app organization
/// 
/// Authenticates with the Auki Network using application-level credentials
/// (app key and secret). This is typically used for server-to-server communication
/// or automated processes.
/// 
/// # Arguments
/// * `api_url` - Base URL for the API service
/// * `dds_url` - URL for the Domain Discovery Service
/// * `client_id` - Unique identifier for this client
/// * `app_key` - Application key for authentication
/// * `app_secret` - Application secret for authentication
/// 
/// # Returns
/// Promise that resolves to a DomainClient instance
/// 
/// # Example
/// ```javascript
/// const client = await signInWithAppCredential(
///     "https://api.example.com",
///     "https://dds.example.com", 
///     "my-client-id",
///     "app-key-123",
///     "app-secret-456"
/// );
/// 
/// // Use the client for operations
/// const metadata = await client.downloadDomainDataMetadata("domain-123", query);
/// ```
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

/// Signs in with user credentials to have read and write access to all domains in the user organization
/// 
/// Authenticates with the Auki Network using user-level credentials
/// (email and password). This is typically used for end-user applications.
/// It is not RECOMMENDED to use this method with logout set to false when this is used in a browser environment.
/// 
/// # Arguments
/// * `api_url` - Base URL for the API service
/// * `dds_url` - URL for the Domain Discovery Service
/// * `client_id` - Unique identifier for this client
/// * `email` - User's email address
/// * `password` - User's password
/// * `logout` - Whether to logout when the token is expired, true to logout, false to not logout and start a periodic login with the same credentials to refresh the token
/// 
/// # Returns
/// Promise that resolves to a DomainClient instance
/// 
/// # Example
/// ```javascript
/// const client = await signInWithUserCredential(
///     "https://api.example.com",
///     "https://dds.example.com",
///     "my-client-id", 
///     "user@example.com",
///     "password123",
///     false
/// );
/// 
/// // Use the client for operations
/// const metadata = await client.downloadDomainDataMetadata("domain-123", query);
/// ```
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
    /// Downloads metadata for domain data without downloading the actual content
    /// 
    /// Retrieves metadata information (name, type, size, properties, etc.) for
    /// domain data that matches the specified query. This is useful for listing
    /// available data or getting information before downloading.
    /// 
    /// # Arguments
    /// * `domain_id` - The domain identifier
    /// * `query` - DownloadQuery object specifying what data to query
    /// 
    /// # Returns
    /// Promise that resolves to an array of metadata objects
    /// 
    /// # Example
    /// ```javascript
    /// const query = new DownloadQuery([], "my-data", "image");
    /// const metadata = await client.downloadDomainDataMetadata("domain-123", query);
    /// 
    /// // metadata contains array of objects with:
    /// // { id, name, data_type, size }
    /// console.log(`Found ${metadata.length} items`);
    /// metadata.forEach(item => {
    ///     console.log(`${item.name}: ${item.size} bytes`);
    /// });
    /// ```
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

    /// Downloads domain data with streaming support
    /// 
    /// Downloads the actual domain data content in chunks, calling the provided
    /// callback function for each chunk received. This method supports large
    /// file downloads by streaming data instead of loading everything into memory.
    /// 
    /// # Arguments
    /// * `domain_id` - The domain identifier
    /// * `query` - DownloadQuery object specifying what data to download
    /// * `callback` - Function called for each data chunk received
    /// 
    /// # Returns
    /// Promise that resolves when download is complete
    /// 
    /// # Example
    /// ```javascript
    /// const query = new DownloadQuery(["data-id-123"], null, null);
    /// 
    /// await client.downloadDomainData("domain-123", query, (datum) => {
    ///     // datum is a { id, name, data_type, size }
    ///     const chunk = datum.get_data_bytes();
    ///     // chunk is a Uint8Array containing the data
    ///     console.log(`Received chunk of ${chunk.length} bytes`);
    ///     
    ///     // Process the chunk (e.g., append to file, display, etc.)
    ///     processChunk(chunk);
    /// });
    /// 
    /// console.log("Download complete!");
    /// ```
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

    /// Uploads domain data to the specified domain
    /// 
    /// Uploads one or more pieces of domain data to the specified domain.
    /// The data is processed and stored according to the domain's configuration.
    /// 
    /// # Arguments
    /// * `domain_id` - The domain identifier where data will be uploaded
    /// * `data` - Array of UploadDomainData objects containing the data to upload
    /// 
    /// # Returns
    /// Promise that resolves to upload result information
    /// 
    /// # Example
    /// ```javascript
    /// // Create data
    /// const createData = new CreateDomainData("my-image.jpg", "image/jpeg");
    /// const uploadData = new UploadDomainData(createData, null, new Uint8Array(data));
    /// 
    /// // Update data
    /// const updateData = new UpdateDomainData("id");
    /// const uploadData2 = new UploadDomainData(null, updateData, new Uint8Array(data));
    /// 
    /// const result = await client.uploadDomainData("domain-123", [uploadData, uploadData2]);
    /// console.log(`Uploaded ${result.length} items successfully`);
    /// // result is an array of objects with:
    /// // { id, name, data_type, size }
    /// ```
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

    /// Downloads a specific piece of domain data by its ID
    /// 
    /// Downloads the complete content of a specific piece of domain data
    /// identified by its unique ID. This method loads the entire data into memory
    /// and returns it as a Uint8Array.
    /// 
    /// # Arguments
    /// * `domain_id` - The domain identifier
    /// * `id` - The unique identifier of the specific data to download
    /// 
    /// # Returns
    /// Promise that resolves to a Uint8Array containing the data
    /// 
    /// # Example
    /// ```javascript
    /// const data = await client.downloadDomainDataById("domain-123", "data-id-456");
    /// 
    /// // data is a Uint8Array containing the complete file content
    /// console.log(`Downloaded ${data.length} bytes`);
    /// 
    /// // Convert to string if it's text data
    /// const text = new TextDecoder().decode(data);
    /// console.log("File content:", text);
    /// 
    /// // Or save to file
    /// const blob = new Blob([data]);
    /// const url = URL.createObjectURL(blob);
    /// const a = document.createElement('a');
    /// a.href = url;
    /// a.download = "downloaded-file";
    /// a.click();
    /// ```
    #[wasm_bindgen(js_name = "downloadDomainDataById")]
    pub fn download_domain_data_by_id(
        &self,
        domain_id: String,
        id: String,
    ) -> Promise {
        let domain_client = self.domain_client.clone();
        let future = async move {
            let res = domain_client.download_domain_data_by_id(&domain_id, &id).await;
            match res {
                Ok(data) => Ok(JsValue::from(Uint8Array::from(data.as_slice()))),
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
