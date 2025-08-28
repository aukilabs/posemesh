use futures::{channel::mpsc, stream::StreamExt, SinkExt};
use reqwest::{Client, Response, Body};
use serde::{Deserialize, Serialize};
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::js_sys::Uint8Array;
use std::collections::HashMap;
#[cfg(not(target_family = "wasm"))]
use tokio::spawn;
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;
#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone)]
pub struct DomainServer {
    pub url: String,
}

#[cfg_attr(target_family = "wasm", wasm_bindgen(getter_with_clone))]
#[derive(Debug, Deserialize, Serialize)]
pub struct DomainData {
    pub id: String,
    pub domain_id: String,
    pub name: String,
    pub data_type: String,
    pub size: u64,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Vec::is_empty", skip_deserializing)]
    pub data: Vec<u8>,
}

#[cfg_attr(target_family = "wasm", wasm_bindgen)]
impl DomainData {
    #[cfg(target_family = "wasm")]
    #[wasm_bindgen]
    pub fn get_data_bytes(&self) -> Uint8Array {
        Uint8Array::from(self.data.as_slice())
    }
}

#[cfg_attr(target_family = "wasm", wasm_bindgen(getter_with_clone))]
#[derive(Debug, Serialize, Clone)]
pub struct UpdateDomainData {
    pub id: String,
}

#[cfg_attr(target_family = "wasm", wasm_bindgen(getter_with_clone))]
#[derive(Debug, Serialize, Clone)]
pub struct CreateDomainData {
    pub name: String,
    pub data_type: String,
}

#[cfg_attr(target_family = "wasm", wasm_bindgen(getter_with_clone))]
#[derive(Debug, Serialize)]
pub struct UploadDomainData {
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub create: Option<CreateDomainData>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub update: Option<UpdateDomainData>,
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadQuery {
    pub ids: Vec<String>,
    pub name: Option<String>,
    pub data_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListDomainData {
    pub data: Vec<DomainData>,
}

pub async fn download_metadata_v1(
    url: &str,
    client_id: &str,
    access_token: &str,
    domain_id: &str,
    query: &DownloadQuery,
) -> Result<Vec<DomainData>, Box<dyn std::error::Error + Send + Sync>> {
    let mut params = HashMap::new();
    
    if !query.ids.is_empty() {
        params.insert("ids", query.ids.join(","));
    }
    if let Some(name) = &query.name {
        params.insert("name", name.clone());
    }
    if let Some(data_type) = &query.data_type {
        params.insert("data_type", data_type.clone());
    }

    let response = Client::new()
        .get(&format!("{}/api/v1/domains/{}/data", url, domain_id))
        .bearer_auth(access_token)
        .header("Accept", "application/json")
        .header("posemesh-client-id", client_id)
        .query(&params)
        .send()
        .await?;

    if response.status().is_success() {
        let data = response.json::<ListDomainData>().await?;
        Ok(data.data)
    } else {
        let status = response.status();
        let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to download data. Status: {} - {}", status, text).into())
    }
}

pub async fn download_v1(
    url: &str,
    client_id: &str,
    access_token: &str,
    domain_id: &str,
    query: &DownloadQuery,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let mut params = HashMap::new();
    
    if !query.ids.is_empty() {
        params.insert("ids", query.ids.join(","));
    }
    if let Some(name) = &query.name {
        params.insert("name", name.clone());
    }
    if let Some(data_type) = &query.data_type {
        params.insert("data_type", data_type.clone());
    }

    let response = Client::new()
        .get(&format!("{}/api/v1/domains/{}/data", url, domain_id))
        .bearer_auth(access_token)
        .header("Accept", "multipart/form-data")
        .header("posemesh-client-id", client_id)
        .query(&params)
        .send()
        .await?;

    if response.status().is_success() {
        Ok(response)
    } else {
        let status = response.status();
        let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to download data. Status: {} - {}", status, text).into())
    }
}

pub async fn download_v1_stream(
    url: &str,
    client_id: &str,
    access_token: &str,
    domain_id: &str,
    query: &DownloadQuery,
) -> Result<mpsc::Receiver<Result<DomainData, Box<dyn std::error::Error + Send + Sync>>>, Box<dyn std::error::Error + Send + Sync>> {
    let response = download_v1(url, client_id, access_token, domain_id, query).await?;
    let (mut tx, rx) = mpsc::channel::<Result<DomainData, Box<dyn std::error::Error + Send + Sync>>>(100);

    let boundary = match response
        .headers()
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .and_then(|ct| {
            if ct.starts_with("multipart/form-data; boundary=") {
                Some(ct.split("boundary=").nth(1)?.to_string())
            } else {
                None
            }
        }) {
        Some(b) => b,
        None => {
            tracing::error!("Invalid content-type header");
            let _ = tx.close().await;
            return Err("Invalid content-type header".into());
        }
    };

    spawn(async move {
        let mut stream = response.bytes_stream();
        let mut buffer = Vec::new();
        let mut current_domain_data: Option<DomainData> = None;

        let boundary_bytes = format!("--{}", boundary).as_bytes().to_vec();

        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(c) if c.is_empty() => {
                    tx.close().await.ok();
                    return;
                },
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(Err(e.into())).await;
                    return;
                }
            };

            buffer.extend_from_slice(&chunk);

            if let Some(mut domain_data) = current_domain_data.take() {
                let expected_size = domain_data.size as usize;
                if buffer.len() >= expected_size {
                    domain_data.data.extend_from_slice(&buffer[..expected_size]);
                    buffer.drain(..expected_size);
                    if tx.send(Ok(domain_data)).await.is_err() {
                        return;
                    }
                } else {
                    current_domain_data = Some(domain_data);
                    continue;
                }
            }

            // Find boundary
            if let Some(boundary_pos) = find_boundary(&buffer, &boundary_bytes) {
                // Look for header end after boundary
                if let Some(header_end) = find_headers_end(&buffer[boundary_pos..]) {
                    let headers_slice = &buffer[boundary_pos..boundary_pos + header_end];
                    let part_headers = parse_headers(headers_slice);
                    if let Ok(domain_data) = part_headers {
                        current_domain_data = Some(domain_data);
                    } else {
                        tracing::error!("Failed to parse headers: {:?}", part_headers.err());
                        return;
                    }

                    // Remove processed data from buffer
                    buffer.drain(..boundary_pos + header_end);
                    continue;
                } else {
                    // Incomplete headers, wait for more chunks
                    continue;
                }
            }
        }
    });

    Ok(rx)
}

#[cfg(not(target_family = "wasm"))]
pub async fn upload_v1_stream(
    url: &str,
    access_token: &str,
    domain_id: &str,
    mut rx: mpsc::Receiver<UploadDomainData>,
) -> Result<Vec<DomainData>, Box<dyn std::error::Error + Send + Sync>> {
    use futures::channel::oneshot;

    let boundary = "boundary";

    let (mut create_tx, create_rx) = mpsc::channel(100);
    let (mut update_tx, update_rx) = mpsc::channel(100);

    let create_body = Body::wrap_stream(create_rx.map(Ok::<Vec<u8>, std::io::Error>));
    let update_body = Body::wrap_stream(update_rx.map(Ok::<Vec<u8>, std::io::Error>));

    let url = url.to_string();
    let url_2 = url.clone();
    let access_token = access_token.to_string();
    let domain_id = domain_id.to_string();
    let access_token_2 = access_token.clone();
    let domain_id_2 = domain_id.clone();

    let (create_signal, create_signal_rx) = oneshot::channel::<Result<Vec<DomainData>, Box<dyn std::error::Error + Send + Sync>>>();
    let (update_signal, update_signal_rx) = oneshot::channel::<Result<Vec<DomainData>, Box<dyn std::error::Error + Send + Sync>>>();

    spawn(async move {
        let create_response = Client::new()
            .post(&format!("{}/api/v1/domains/{}/data", url, domain_id))
            .bearer_auth(access_token)
            .header("Content-Type", &format!("multipart/form-data; boundary={}", boundary))
            .body(create_body)
            .send()
            .await;

        if let Err(e) = create_response {
            tracing::error!("Create failed with error: {}", e);
            create_signal.send(Err(e.into())).unwrap();
            return;
            // return Err(e.into());
        }

        let create_response = create_response.unwrap();
        if create_response.status().is_success() {
            let data = create_response.json::<ListDomainData>().await.unwrap();
            create_signal.send(Ok(data.data)).unwrap();
        } else {
            let err = create_response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            create_signal.send(Err(format!("Create failed with status: {}", err).into())).unwrap();
        }
    });

    spawn(async move {
        let update_response = Client::new()
            .put(&format!("{}/api/v1/domains/{}/data", url_2, domain_id_2))
            .bearer_auth(access_token_2)
            .header("Content-Type", &format!("multipart/form-data; boundary={}", boundary))
            .body(update_body)
            .send()
            .await;

        if let Err(e) = update_response {
            tracing::error!("Update failed with error: {}", e);
            update_signal.send(Err(e.into())).unwrap();
            return;
            // return Err(e.into());
        }
        let update_response = update_response.unwrap();
        if update_response.status().is_success() {
            let data = update_response.json::<ListDomainData>().await.unwrap();
            update_signal.send(Ok(data.data)).unwrap();
        } else {
            let err = update_response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            update_signal.send(Err(format!("Update failed with status: {}", err).into())).unwrap();
        }
    });

    while let Some(datum) = rx.next().await {
        // Process the first item based on whether it's create or update
        if let Some(update) = &datum.update {
            // Create update bytes with boundary format
            let update_bytes = format!(
                "--{}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: form-data; name=\"{}\"; id=\"{}\"\r\n\r\n",
                boundary, update.id, update.id
            );
            let mut update_data = update_bytes.into_bytes();
            update_data.extend_from_slice(&datum.data);
            update_data.extend_from_slice("\r\n".as_bytes());
            
            update_tx.send(update_data).await?;
        } else if let Some(create) = &datum.create {
            // Create create bytes with boundary format
            let create_bytes = format!(
                "--{}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: form-data; name=\"{}\"; data-type=\"{}\"\r\n\r\n",
                boundary, create.name, create.data_type
            );
            let mut create_data = create_bytes.into_bytes();
            create_data.extend_from_slice(&datum.data);
            create_data.extend_from_slice("\r\n".as_bytes());
            
            create_tx.clone().send(create_data).await?;
        }
    }
    update_tx.send(format!("--{}--\r\n", boundary).as_bytes().to_vec()).await?;
    create_tx.send(format!("--{}--\r\n", boundary).as_bytes().to_vec()).await?;
    update_tx.close().await?;
    create_tx.close().await?;

    let mut data = Vec::new();

    if let Ok(res) = create_signal_rx.await {
        match res {
            Ok(d) => data = d,
            Err(e) => return Err(e),
        }
    } else {
        return Err("create cancelled".into());
    }

    if let Ok(res) = update_signal_rx.await {
        match res {
            Ok(d) => data.extend(d),
            Err(e) => return Err(e),
        }
    } else {
        return Err("update cancelled".into());
    }

    Ok(data)
}

pub async fn upload_v1(
    url: &str,
    access_token: &str,
    domain_id: &str,
    data: Vec<UploadDomainData>,
) -> Result<Vec<DomainData>, Box<dyn std::error::Error + Send + Sync>> {
    let boundary = "boundary";

    let mut create_body = Vec::new();
    let mut update_body = Vec::new();

    let url = url.to_string();
    let url_2 = url.clone();
    let access_token = access_token.to_string();
    let domain_id = domain_id.to_string();
    let access_token_2 = access_token.clone();
    let domain_id_2 = domain_id.clone();

    // Process the first item to get metadata for the form
    for datun in data {
    // Process the first item based on whether it's create or update
        if let Some(update) = &datun.update {
            // Create update bytes with boundary format
            let update_bytes = format!(
                "--{}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: form-data; name=\"{}\"; id=\"{}\"\r\n\r\n",
                boundary, update.id, update.id
            );
            let mut update_data = update_bytes.into_bytes();
            update_data.extend_from_slice(&datun.data);
            update_data.extend_from_slice("\r\n".as_bytes());
            
            update_body.extend_from_slice(&update_data);
        } else if let Some(create) = &datun.create {
            // Create create bytes with boundary format
            let create_bytes = format!(
                "--{}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: form-data; name=\"{}\"; data-type=\"{}\"\r\n\r\n",
                boundary, create.name, create.data_type
            );
            let mut create_data = create_bytes.into_bytes();
            create_data.extend_from_slice(&datun.data);
            create_data.extend_from_slice("\r\n".as_bytes());
            
            create_body.extend_from_slice(&create_data);
        }
    }

    create_body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());
    update_body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let create_body = Body::from(create_body);
    let update_body = Body::from(update_body);

    let create_response = Client::new()
        .post(&format!("{}/api/v1/domains/{}/data", url, domain_id))
        .bearer_auth(access_token)
        .header("Content-Type", "multipart/form-data")
        .body(create_body)
        .send()
        .await.unwrap();

    let mut res = Vec::new();
    if create_response.status().is_success() {
        let data = create_response.json::<ListDomainData>().await.unwrap();
        res = data.data;
    } else {
        let err = create_response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Create failed with status: {}", err).into());
    }

    let update_response = Client::new()
        .post(&format!("{}/api/v1/domains/{}/data", url_2, domain_id_2))
        .bearer_auth(access_token_2)
        .header("Content-Type", "multipart/form-data")
        .body(update_body)
        .send()
        .await.unwrap();

    if update_response.status().is_success() {
        let data = update_response.json::<ListDomainData>().await.unwrap();
        res.extend(data.data);
    } else {
        let err = update_response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Update failed with status: {}", err).into());
    }

    Ok(res)
} 
    
fn parse_headers(headers_slice: &[u8]) -> Result<DomainData, Box<dyn std::error::Error + Send + Sync>> {
    let headers_str = String::from_utf8_lossy(headers_slice);
    let mut domain_data = None;
    
    for line in headers_str.lines() {
        if line.trim().is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_lowercase();
            if key == "content-disposition" {
                let mut parsed_domain_data = DomainData {
                    id: String::new(),
                    domain_id: String::new(),
                    name: String::new(),
                    data_type: String::new(),
                    size: 0,
                    created_at: String::new(),
                    updated_at: String::new(),
                    data: Vec::new(),
                };
                for part in value.split(';') {
                    let part = part.trim();
                    if let Some((key, value)) = part.split_once('=') {
                        let key = key.trim();
                        let value = value.trim().trim_matches('"');
                        match key {
                            "id" => parsed_domain_data.id = value.to_string(),
                            "domain-id" => parsed_domain_data.domain_id = value.to_string(),
                            "name" => parsed_domain_data.name = value.to_string(),
                            "data-type" => parsed_domain_data.data_type = value.to_string(),
                            "size" => parsed_domain_data.size = value.parse()?,
                            "created-at" => parsed_domain_data.created_at = value.to_string(),
                            "updated-at" => parsed_domain_data.updated_at = value.to_string(),
                            _ => {}
                        }
                    }
                } 
                domain_data = Some(parsed_domain_data);
            }
        }
    }

    if let Some(domain_data) = domain_data {
        Ok(domain_data)
    } else {
        Err("Missing content-disposition header".into())
    }
}

fn find_boundary(data: &[u8], boundary: &[u8]) -> Option<usize> {
    let _data = String::from_utf8_lossy(data);
    let _boundary = String::from_utf8_lossy(boundary);
    data.windows(boundary.len()).position(|window| window == boundary)
}

fn find_headers_end(data: &[u8]) -> Option<usize> {
    if let Some(i) = data.windows(4).position(|w| w == b"\r\n\r\n") {
        Some(i + 4) // body starts after \r\n\r\n
    } else if let Some(i) = data.windows(2).position(|w| w == b"\n\n") {
        Some(i + 2) // body starts after \n\n
    } else {
        None
    }
}
