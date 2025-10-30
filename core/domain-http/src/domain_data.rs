use bytes::Bytes;
use futures::{SinkExt, Stream, channel::mpsc, stream::StreamExt};
use reqwest::{Body, Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(not(target_family = "wasm"))]
use tokio::spawn;
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainDataMetadata {
    pub id: String,
    pub domain_id: String,
    pub name: String,
    pub data_type: String,
    pub size: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DomainData {
    // #[serde(flatten)] This doesn't work in serde_wasm_bindgen, it generates Map instead of a plain object
    pub metadata: DomainDataMetadata,
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct UpdateDomainData {
    pub id: String,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct CreateDomainData {
    pub name: String,
    pub data_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DomainAction {
    Create(CreateDomainData),
    Update(UpdateDomainData),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadDomainData {
    #[serde(flatten)]
    pub action: DomainAction,
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadQuery {
    pub ids: Vec<String>,
    pub name: Option<String>,
    pub data_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListDomainDataMetadata {
    pub data: Vec<DomainDataMetadata>,
}

pub async fn download_by_id(
    url: &str,
    client_id: &str,
    access_token: &str,
    domain_id: &str,
    id: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let response = Client::new()
        .get(format!(
            "{}/api/v1/domains/{}/data/{}?raw=true",
            url, domain_id, id
        ))
        .bearer_auth(access_token)
        .header("posemesh-client-id", client_id)
        .send()
        .await?;

    if response.status().is_success() {
        let data = response.bytes().await?;
        Ok(data.to_vec())
    } else {
        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!(
            "Failed to download data by id. Status: {} - {}",
            status, text
        )
        .into())
    }
}

/// Perform a direct absolute download request to the domain server.
/// Sets headers: Accept: multipart/form-data, Authorization: Bearer <token>, posemesh-client-id.
pub async fn request_download_absolute(
    url: &str,
    client_id: &str,
    access_token: &str,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let response = Client::new()
        .get(url)
        .bearer_auth(access_token)
        .header("posemesh-client-id", client_id)
        .header("Accept", "multipart/form-data")
        .send()
        .await?;
    Ok(response)
}

pub async fn download_metadata_v1(
    url: &str,
    client_id: &str,
    access_token: &str,
    domain_id: &str,
    query: &DownloadQuery,
) -> Result<Vec<DomainDataMetadata>, Box<dyn std::error::Error + Send + Sync>> {
    let response = download_v1(url, client_id, access_token, domain_id, query, false).await?;
    if response.status().is_success() {
        let data = response.json::<ListDomainDataMetadata>().await?;
        Ok(data.data)
    } else {
        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to download data. Status: {} - {}", status, text).into())
    }
}

pub async fn download_v1(
    url: &str,
    client_id: &str,
    access_token: &str,
    domain_id: &str,
    query: &DownloadQuery,
    with_data: bool,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let mut params = HashMap::new();

    if let Some(name) = &query.name {
        params.insert("name", name.clone());
    }
    if let Some(data_type) = &query.data_type {
        params.insert("data_type", data_type.clone());
    }
    let ids = if !query.ids.is_empty() {
        format!("?ids={}", query.ids.join(","))
    } else {
        String::new()
    };

    let response = Client::new()
        .get(format!("{}/api/v1/domains/{}/data{}", url, domain_id, ids))
        .bearer_auth(access_token)
        .header(
            "Accept",
            if with_data {
                "multipart/form-data"
            } else {
                "application/json"
            },
        )
        .header("posemesh-client-id", client_id)
        .query(&params)
        .send()
        .await?;

    if response.status().is_success() {
        Ok(response)
    } else {
        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to download data. Status: {} - {}", status, text).into())
    }
}

pub async fn download_v1_stream(
    url: &str,
    client_id: &str,
    access_token: &str,
    domain_id: &str,
    query: &DownloadQuery,
) -> Result<
    mpsc::Receiver<Result<DomainData, Box<dyn std::error::Error + Send + Sync>>>,
    Box<dyn std::error::Error + Send + Sync>,
> {
    let response = download_v1(url, client_id, access_token, domain_id, query, true).await?;

    let (mut tx, rx) =
        mpsc::channel::<Result<DomainData, Box<dyn std::error::Error + Send + Sync>>>(100);

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
        let stream = response.bytes_stream();
        handle_domain_data_stream(tx, stream, &boundary).await;
    });

    Ok(rx)
}

/// Build a stream from an HTTP response returned by the domain download endpoint.
/// Parses multipart boundaries and yields DomainData items as they arrive.
pub async fn stream_from_response(
    response: Response,
) -> Result<
    mpsc::Receiver<Result<DomainData, Box<dyn std::error::Error + Send + Sync>>>,
    Box<dyn std::error::Error + Send + Sync>,
> {
    let (mut tx, rx) =
        mpsc::channel::<Result<DomainData, Box<dyn std::error::Error + Send + Sync>>>(100);

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
        let stream = response.bytes_stream();
        handle_domain_data_stream(tx, stream, &boundary).await;
    });

    Ok(rx)
}

pub async fn delete_by_id(
    url: &str,
    access_token: &str,
    domain_id: &str,
    id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let endpoint = format!("{}/api/v1/domains/{}/data/{}", url, domain_id, id);
    let client = Client::new();
    let resp = client
        .delete(&endpoint)
        .bearer_auth(access_token)
        .send()
        .await?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let err = resp
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Delete failed with status: {}", err).into())
    }
}

#[cfg(not(target_family = "wasm"))]
pub async fn upload_v1_stream(
    url: &str,
    access_token: &str,
    domain_id: &str,
    mut rx: mpsc::Receiver<UploadDomainData>,
) -> Result<Vec<DomainDataMetadata>, Box<dyn std::error::Error + Send + Sync>> {
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

    let (create_signal, create_signal_rx) = oneshot::channel::<
        Result<Vec<DomainDataMetadata>, Box<dyn std::error::Error + Send + Sync>>,
    >();
    let (update_signal, update_signal_rx) = oneshot::channel::<
        Result<Vec<DomainDataMetadata>, Box<dyn std::error::Error + Send + Sync>>,
    >();

    spawn(async move {
        let create_response =
            create_v1(&url, &access_token, &domain_id, boundary, create_body).await;
        if let Err(Err(e)) = create_signal.send(create_response) {
            tracing::error!("Failed to send create response: {}", e);
        }
    });

    spawn(async move {
        let update_response =
            update_v1(&url_2, &access_token_2, &domain_id_2, boundary, update_body).await;
        if let Err(Err(e)) = update_signal.send(update_response) {
            tracing::error!("Failed to send update response: {}", e);
        }
    });

    while let Some(datum) = rx.next().await {
        match datum.action {
            DomainAction::Create(create) => {
                let create_data = write_create_body(boundary, &create, &datum.data);
                create_tx.clone().send(create_data).await?;
            }
            DomainAction::Update(update) => {
                let update_data = write_update_body(boundary, &update, &datum.data);
                update_tx.send(update_data).await?;
            }
        }
    }
    update_tx
        .send(format!("--{}--\r\n", boundary).as_bytes().to_vec())
        .await?;
    create_tx
        .send(format!("--{}--\r\n", boundary).as_bytes().to_vec())
        .await?;
    update_tx.close().await?;
    create_tx.close().await?;

    let mut data = {
        if let Ok(res) = create_signal_rx.await {
            match res {
                Ok(d) => d,
                Err(e) => return Err(e),
            }
        } else {
            return Err("create cancelled".into());
        }
    };

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

async fn update_v1(
    url: &str,
    access_token: &str,
    domain_id: &str,
    boundary: &str,
    body: Body,
) -> Result<Vec<DomainDataMetadata>, Box<dyn std::error::Error + Send + Sync>> {
    let update_response = Client::new()
        .put(format!("{}/api/v1/domains/{}/data", url, domain_id))
        .bearer_auth(access_token)
        .header(
            "Content-Type",
            &format!("multipart/form-data; boundary={}", boundary),
        )
        .body(body)
        .send()
        .await?;

    if update_response.status().is_success() {
        let data = update_response
            .json::<ListDomainDataMetadata>()
            .await
            .unwrap();
        Ok(data.data)
    } else {
        let err = update_response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Update failed with status: {}", err).into())
    }
}

async fn create_v1(
    url: &str,
    access_token: &str,
    domain_id: &str,
    boundary: &str,
    body: Body,
) -> Result<Vec<DomainDataMetadata>, Box<dyn std::error::Error + Send + Sync>> {
    let create_response = Client::new()
        .post(format!("{}/api/v1/domains/{}/data", url, domain_id))
        .bearer_auth(access_token)
        .header(
            "Content-Type",
            &format!("multipart/form-data; boundary={}", boundary),
        )
        .body(body)
        .send()
        .await?;

    if create_response.status().is_success() {
        let data = create_response
            .json::<ListDomainDataMetadata>()
            .await
            .unwrap();
        Ok(data.data)
    } else {
        let err = create_response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Create failed with status: {}", err).into())
    }
}

fn write_create_body(boundary: &str, data: &CreateDomainData, data_bytes: &[u8]) -> Vec<u8> {
    let create_bytes = format!(
        "--{}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: form-data; name=\"{}\"; data-type=\"{}\"\r\n\r\n",
        boundary, data.name, data.data_type
    );
    let mut create_data = create_bytes.into_bytes();
    create_data.extend_from_slice(data_bytes);
    create_data.extend_from_slice("\r\n".as_bytes());
    create_data
}

fn write_update_body(boundary: &str, data: &UpdateDomainData, data_bytes: &[u8]) -> Vec<u8> {
    let update_bytes = format!(
        "--{}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: form-data; id=\"{}\"\r\n\r\n",
        boundary, data.id
    );
    let mut update_data = update_bytes.into_bytes();
    update_data.extend_from_slice(data_bytes);
    update_data.extend_from_slice("\r\n".as_bytes());
    update_data
}

pub async fn upload_v1(
    url: &str,
    access_token: &str,
    domain_id: &str,
    data: Vec<UploadDomainData>,
) -> Result<Vec<DomainDataMetadata>, Box<dyn std::error::Error + Send + Sync>> {
    let boundary = "boundary";

    let mut create_body = Vec::new();
    let mut update_body = Vec::new();
    let mut to_update = false;
    let mut to_create = false;

    // Process the first item to get metadata for the form
    for datum in data {
        match datum.action {
            DomainAction::Create(create) => {
                to_create = true;
                let create_data = write_create_body(boundary, &create, &datum.data);
                create_body.extend_from_slice(&create_data);
            }
            DomainAction::Update(update) => {
                to_update = true;
                let update_data = write_update_body(boundary, &update, &datum.data);
                update_body.extend_from_slice(&update_data);
            }
        }
    }

    create_body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());
    update_body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let create_body = Body::from(create_body);
    let update_body = Body::from(update_body);
    let mut res = Vec::new();

    if to_create {
        res = create_v1(url, access_token, domain_id, boundary, create_body).await?;
    }
    if to_update {
        let update_response =
            update_v1(url, access_token, domain_id, boundary, update_body).await?;
        if !update_response.is_empty() {
            res.extend(update_response);
        }
    }

    Ok(res)
}

/// Upload a single domain data item (create or update) as one HTTP request.
/// Returns a list of created/updated DomainData entries (with empty data payloads).
#[cfg(not(target_family = "wasm"))]
pub async fn upload_one(
    url: &str,
    access_token: &str,
    domain_id: &str,
    data: UploadDomainData,
) -> Result<Vec<DomainData>, (StatusCode, String)> {
    let boundary = "boundary";
    let (method, body) = match &data.action {
        DomainAction::Create(create) => {
            let bytes = write_create_body(boundary, create, &data.data);
            (reqwest::Method::POST, Body::from(bytes))
        }
        DomainAction::Update(update) => {
            let bytes = write_update_body(boundary, update, &data.data);
            (reqwest::Method::PUT, Body::from(bytes))
        }
    };

    let endpoint = format!("{}/api/v1/domains/{}/data", url, domain_id);
    let response = Client::new()
        .request(method, endpoint)
        .bearer_auth(access_token)
        .header(
            "Content-Type",
            &format!("multipart/form-data; boundary={}", boundary),
        )
        .body(body)
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if response.status().is_success() {
        // Be tolerant to varying response shapes (full metadata or id-only)
        let text = response
            .text()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        // Try strict metadata first
        if let Ok(md) = serde_json::from_str::<ListDomainDataMetadata>(&text) {
            let items: Vec<DomainData> = md
                .data
                .into_iter()
                .map(|m| DomainData {
                    metadata: m,
                    data: Vec::new(),
                })
                .collect();
            return Ok(items);
        }
        // Fallback: extract minimal fields from generic JSON
        let v: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let mut out: Vec<DomainData> = Vec::new();
        if let Some(arr) = v.get("data").and_then(|d| d.as_array()) {
            for item in arr {
                let id = item.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let domain_id = item
                    .get("domain_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let name = item
                    .get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let data_type = item
                    .get("data_type")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                out.push(DomainData {
                    metadata: DomainDataMetadata {
                        id,
                        domain_id,
                        name,
                        data_type,
                        size: 0,
                        created_at: String::new(),
                        updated_at: String::new(),
                    },
                    data: Vec::new(),
                });
            }
            return Ok(out);
        }
        Err((StatusCode::INTERNAL_SERVER_ERROR, "invalid response".to_string()))
    } else {
        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err((status, text))
    }
}

fn parse_headers(
    headers_slice: &[u8],
) -> Result<DomainData, Box<dyn std::error::Error + Send + Sync>> {
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
                    metadata: DomainDataMetadata {
                        id: String::new(),
                        domain_id: String::new(),
                        name: String::new(),
                        data_type: String::new(),
                        size: 0,
                        created_at: String::new(),
                        updated_at: String::new(),
                    },
                    data: Vec::new(),
                };
                for part in value.split(';') {
                    let part = part.trim();
                    if let Some((key, value)) = part.split_once('=') {
                        let key = key.trim();
                        let value = value.trim().trim_matches('"');
                        match key {
                            "id" => parsed_domain_data.metadata.id = value.to_string(),
                            "domain-id" => {
                                parsed_domain_data.metadata.domain_id = value.to_string()
                            }
                            "name" => parsed_domain_data.metadata.name = value.to_string(),
                            "data-type" => {
                                parsed_domain_data.metadata.data_type = value.to_string()
                            }
                            "size" => parsed_domain_data.metadata.size = value.parse()?,
                            "created-at" => {
                                parsed_domain_data.metadata.created_at = value.to_string()
                            }
                            "updated-at" => {
                                parsed_domain_data.metadata.updated_at = value.to_string()
                            }
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
    data.windows(boundary.len())
        .position(|window| window == boundary)
}

fn find_headers_end(data: &[u8]) -> Option<usize> {
    if let Some(i) = data.windows(4).position(|w| w == b"\r\n\r\n") {
        return Some(i + 4);
    }
    data.windows(2)
        .position(|w| w == b"\n\n")
        .map(|i| i + 2)
}

async fn handle_domain_data_stream(
    mut tx: mpsc::Sender<Result<DomainData, Box<dyn std::error::Error + Send + Sync>>>,
    stream: impl Stream<Item = Result<Bytes, reqwest::Error>>,
    boundary: &str,
) {
    use futures::pin_mut;

    let mut buffer = Vec::new();
    let mut current_domain_data: Option<DomainData> = None;
    let boundary_bytes = format!("--{}", boundary).into_bytes();
    let keep_tail = boundary_bytes.len() + 4; // bytes to keep for boundary detection across chunks

    pin_mut!(stream);

    while let Some(chunk_result) = stream.next().await {
        let chunk = match chunk_result {
            Ok(c) if c.is_empty() => continue,
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(Err(e.into())).await;
                return;
            }
        };

        buffer.extend_from_slice(&chunk);

        'consume: loop {
            match &mut current_domain_data {
                None => {
                    let Some(boundary_pos) = find_boundary(&buffer, &boundary_bytes) else {
                        if buffer.len() > keep_tail {
                            buffer.drain(..buffer.len() - keep_tail);
                        }
                        break 'consume;
                    };
                    let Some(header_end_rel) = find_headers_end(&buffer[boundary_pos..]) else {
                        break 'consume;
                    };
                    let headers_slice = &buffer[boundary_pos..boundary_pos + header_end_rel];
                    let part_headers = parse_headers(headers_slice);
                    let domain_data = match part_headers {
                        Ok(d) => d,
                        Err(e) => {
                            tracing::error!("Failed to parse headers: {:?}", e);
                            return;
                        }
                    };
                    buffer.drain(..boundary_pos + header_end_rel);
                    current_domain_data = Some(domain_data);
                }
                Some(dd) => {
                    if let Some(next_boundary_pos) = find_boundary(&buffer, &boundary_bytes) {
                        let mut data_end = next_boundary_pos;
                        if data_end >= 2 && &buffer[data_end - 2..data_end] == b"\r\n" {
                            data_end -= 2;
                        } else if data_end >= 1 && buffer[data_end - 1] == b'\n' {
                            data_end -= 1;
                        }
                        dd.data.extend_from_slice(&buffer[..data_end]);
                        buffer.drain(..next_boundary_pos);
                        let finished = current_domain_data.take().unwrap();
                        if tx.send(Ok(finished)).await.is_err() {
                            return;
                        }
                    } else {
                        if buffer.len() > keep_tail {
                            let take = buffer.len() - keep_tail;
                            dd.data.extend_from_slice(&buffer[..take]);
                            buffer.drain(..take);
                        }
                        break 'consume;
                    }
                }
            }
        }
    }

    let _ = tx.close().await;
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use crate::{auth::TokenCache, config::Config, discovery::DiscoveryService};

    use super::*;

    fn get_config() -> (Config, String) {
        if std::path::Path::new("../.env.local").exists() {
            dotenvy::from_filename("../.env.local").ok();
            dotenvy::dotenv().ok();
        }
        let config = Config::from_env().unwrap();
        (config, std::env::var("DOMAIN_ID").unwrap())
    }

    #[test]
    fn test_find_boundary_found() {
        let data = b"random--boundary--data";
        let boundary = b"--boundary";
        assert_eq!(find_boundary(data, boundary), Some(6));
    }

    #[test]
    fn test_find_boundary_not_found() {
        let data = b"random-data";
        let boundary = b"--boundary";
        assert_eq!(find_boundary(data, boundary), None);
    }

    #[test]
    fn test_find_headers_end_crlf() {
        let data = b"header1: value1\r\nheader2: value2\r\n\r\nbody";
        assert_eq!(find_headers_end(data), Some(36));
    }

    #[test]
    fn test_find_headers_end_lf() {
        let data = b"header1: value1\nheader2: value2\n\nbody";
        assert_eq!(find_headers_end(data), Some(33));
    }

    #[test]
    fn test_find_headers_end_none() {
        let data = b"header1: value1\nheader2: value2\nbody";
        assert_eq!(find_headers_end(data), None);
    }

    #[test]
    fn test_parse_headers_success() {
        let headers = b"content-disposition: form-data; id=\"123\"; domain-id=\"abc\"; name=\"test\"; data-type=\"type\"; size=\"42\"; created-at=\"2024-01-01T00:00:00Z\"; updated-at=\"2024-01-02T00:00:00Z\"\r\n\r\n";
        let parsed = super::parse_headers(headers);
        assert!(parsed.is_ok());
        let domain_data = parsed.unwrap();
        assert_eq!(domain_data.metadata.id, "123");
        assert_eq!(domain_data.metadata.domain_id, "abc");
        assert_eq!(domain_data.metadata.name, "test");
        assert_eq!(domain_data.metadata.data_type, "type");
        assert_eq!(domain_data.metadata.size, 42);
        assert_eq!(domain_data.metadata.created_at, "2024-01-01T00:00:00Z");
        assert_eq!(domain_data.metadata.updated_at, "2024-01-02T00:00:00Z");
    }

    #[test]
    fn test_parse_headers_missing_content_disposition() {
        let headers = b"content-type: application/octet-stream\r\n\r\n";
        let parsed = super::parse_headers(headers);
        assert!(parsed.is_err());
    }

    #[tokio::test]
    async fn test_a_chunk_contains_multiple_data() {
        let (tx, rx) = mpsc::channel(10);

        let payload = br#"
        --0f336dec6f61e466706eb557cda40d8caa86c28df397bd7348766b5b5eda
Content-Disposition: form-data; name="to be deleted"; data-type="test"; id="3c5bbdbc-65b9-4f11-93b6-a3e535d63990"; domain-id="23d60e61-6978-4f6b-a59d-9ffa027755fc"; size="16"; created-at="2025-09-25T02:54:26.124336Z"; updated-at="2025-09-25T02:54:26.124336Z"
Content-Type: application/octet-stream

{"test": "test"}
--0f336dec6f61e466706eb557cda40d8caa86c28df397bd7348766b5b5eda
Content-Disposition: form-data; name="test"; data-type="test"; id="a84a36e5-312b-4f80-974a-06f5d19c1e16"; domain-id="23d60e61-6978-4f6b-a59d-9ffa027755fc"; size="24"; created-at="2025-08-05T10:29:56.448595Z"; updated-at="2025-09-25T02:54:26.154224Z"
Content-Type: application/octet-stream

{"test": "test updated"}
--0f336dec6f61e466706eb557cda40d8caa86c28df397bd7348766b5b5eda--
        "#;
        let stream = tokio_stream::iter(vec![Ok(Bytes::from_static(payload))]);

        handle_domain_data_stream(
            tx,
            stream,
            "0f336dec6f61e466706eb557cda40d8caa86c28df397bd7348766b5b5eda",
        )
        .await;

        let output: Vec<DomainData> = rx
            .collect::<Vec<Result<DomainData, Box<dyn std::error::Error + Send + Sync>>>>()
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(output.len(), 2);
        assert_eq!(output[1].data, b"{\"test\": \"test updated\"}");
        assert_eq!(output[0].data, b"{\"test\": \"test\"}");
    }

    #[tokio::test]
    async fn test_chunk_size_is_smaller_than_part() {
        let (tx, rx) = mpsc::channel(10);

        let payload = br#"
        --0f336dec6f61e466706eb557cda40d8caa86c28df397bd7348766b5b5eda
Content-Disposition: form-data; name="to be deleted"; data-type="test"; id="3c5bbdbc-65b9-4f11-93b6-a3e535d63990"; domain-id="23d60e61-6978-4f6b-a59d-9ffa027755fc"; size="16"; created-at="2025-09-25T02:54:26.124336Z"; updated-at="2025-09-25T02:54:26.124336Z"
Content-Type: application/octet-stream
        "#;
        let payload2 = br#"

{"test": "test"}
--0f336dec6f61e466706eb557cda40d8caa86c28df397bd7348766b5b5eda
Content-Disposition: form-data; name="test"; data-type="test"; id="a84a36e5-312b-4f80-974a-06f5d19c1e16"; domain-id="23d60e61-6978-4f6b-a59d-9ffa027755fc"; size="24"; created-at="2025-08-05T10:29:56.448595Z"; updated-at="2025-09-25T02:54:26.154224Z"
Content-Type: application/octet-stream

{"test": "test updated"}
--0f336dec6f61e466706eb557cda40d8caa86c28df397bd7348766b5b5eda--
"#;
        let stream = tokio_stream::iter(vec![
            Ok(Bytes::from_static(payload)),
            Ok(Bytes::from_static(payload2)),
        ]);

        handle_domain_data_stream(
            tx,
            stream,
            "0f336dec6f61e466706eb557cda40d8caa86c28df397bd7348766b5b5eda",
        )
        .await;

        let output: Vec<DomainData> = rx
            .collect::<Vec<Result<DomainData, Box<dyn std::error::Error + Send + Sync>>>>()
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(output.len(), 2);
        assert_eq!(output[1].data, b"{\"test\": \"test updated\"}");
        assert_eq!(output[0].data, b"{\"test\": \"test\"}");
    }

    #[tokio::test]
    async fn test_chunk_size_is_smaller_than_header() {
        let (tx, rx) = mpsc::channel(10);

        let payload = br#"
        --0f336dec6f61e466706eb557cda40d8caa86c28df397bd7348766b5b5eda
Content-Disposition: form-data; name="to be deleted"; data-type="test"; id="3c5bbdbc-65b9-4f11-93b6-a3e535d63990"; domain-id="23d60e61-6978-4f6b-a59d-9ffa027755fc"; size="16"; created-at="2025-09-25T02:54:26.124336Z"; updated-at="2025-09-25T02:54:26.124336Z"
Content-Type: application/octet-stream
        "#;
        let payload2 = br#"
e: application/octet-stream

{"test": "test"}
--0f336dec6f61e466706eb557cda40d8caa86c28df397bd7348766b5b5eda
Content-Disposition: form-data; name="test"; data-type="test"; id="a84a36e5-312b-4f80-974a-06f5d19c1e16"; domain-id="23d60e61-6978-4f6b-a59d-9ffa027755fc"; size="24"; created-at="2025-08-05T10:29:56.448595Z"; updated-at="2025-09-25T02:54:26.154224Z"
Content-Type: application/octet-stream

{"test": "test updated"}
--0f336dec6f61e466706eb557cda40d8caa86c28df397bd7348766b5b5eda--
"#;
        let stream = tokio_stream::iter(vec![
            Ok(Bytes::from_static(payload)),
            Ok(Bytes::from_static(payload2)),
        ]);

        handle_domain_data_stream(
            tx,
            stream,
            "0f336dec6f61e466706eb557cda40d8caa86c28df397bd7348766b5b5eda",
        )
        .await;

        let output: Vec<DomainData> = rx
            .collect::<Vec<Result<DomainData, Box<dyn std::error::Error + Send + Sync>>>>()
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(output.len(), 2);
        assert_eq!(output[1].data, b"{\"test\": \"test updated\"}");
        assert_eq!(output[0].data, b"{\"test\": \"test\"}");
    }

    #[tokio::test]
    async fn test_chunk_size_doesnt_cover_the_whole_data() {
        let (tx, rx) = mpsc::channel(10);

        let payload = br#"
        --0f336dec6f61e466706eb557cda40d8caa86c28df397bd7348766b5b5eda
Content-Disposition: form-data; name="to be deleted"; data-type="test"; id="3c5bbdbc-65b9-4f11-93b6-a3e535d63990"; domain-id="23d60e61-6978-4f6b-a59d-9ffa027755fc"; size="16"; created-at="2025-09-25T02:54:26.124336Z"; updated-at="2025-09-25T02:54:26.124336Z"
Content-Type: application/octet-stream

{"test": "test"#;
        let payload2 = br#""}
--0f336dec6f61e466706eb557cda40d8caa86c28df397bd7348766b5b5eda
Content-Disposition: form-data; name="test"; data-type="test"; id="a84a36e5-312b-4f80-974a-06f5d19c1e16"; domain-id="23d60e61-6978-4f6b-a59d-9ffa027755fc"; size="24"; created-at="2025-08-05T10:29:56.448595Z"; updated-at="2025-09-25T02:54:26.154224Z"
Content-Type: application/octet-stream

{"test": "test updated"}
--0f336dec6f61e466706eb557cda40d8caa86c28df397bd7348766b5b5eda--
"#;
        let stream = tokio_stream::iter(vec![
            Ok(Bytes::from_static(payload)),
            Ok(Bytes::from_static(payload2)),
        ]);

        handle_domain_data_stream(
            tx,
            stream,
            "0f336dec6f61e466706eb557cda40d8caa86c28df397bd7348766b5b5eda",
        )
        .await;

        let output: Vec<DomainData> = rx
            .collect::<Vec<Result<DomainData, Box<dyn std::error::Error + Send + Sync>>>>()
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(output.len(), 2);
        assert_eq!(
            std::str::from_utf8(&output[1].data).unwrap(),
            "{\"test\": \"test updated\"}"
        );
        assert_eq!(
            std::str::from_utf8(&output[0].data).unwrap(),
            "{\"test\": \"test\"}"
        );
    }

    #[tokio::test]
    async fn test_upload_v1_with_user_dds_access_token() {
        use crate::domain_data::{CreateDomainData, DomainAction, UploadDomainData};

        let (config, domain_id) = get_config();

        let mut discovery =
            DiscoveryService::new(&config.api_url, &config.dds_url, &config.client_id);
        discovery
            .sign_in_with_auki_account(&config.email.unwrap(), &config.password.unwrap(), false)
            .await
            .expect("sign_in_with_auki_account failed");
        let domain = discovery
            .auth_domain(&domain_id)
            .await
            .expect("get_domain failed");
        // 4. Prepare upload data
        let upload_data = vec![
            UploadDomainData {
                action: DomainAction::Create(CreateDomainData {
                    name: "test_upload".to_string(),
                    data_type: "test".to_string(),
                }),
                data: b"hello world".to_vec(),
            },
            UploadDomainData {
                action: DomainAction::Update(UpdateDomainData {
                    id: "a84a36e5-312b-4f80-974a-06f5d19c1e16".to_string(),
                }),
                data: b"{\"test\": \"test updated\"}".to_vec(),
            },
        ];

        // 5. Call upload_v1
        let result = upload_v1(
            &domain.domain.domain_server.url,
            &domain.get_access_token(),
            &domain_id,
            upload_data,
        )
        .await
        .expect("upload_v1 failed");

        assert_eq!(result.len(), 2, "No metadata returned from upload_v1");
        for data in result {
            if data.id != "a84a36e5-312b-4f80-974a-06f5d19c1e16" {
                assert_eq!(data.name, "test_upload");
                delete_by_id(
                    &domain.domain.domain_server.url,
                    &domain.get_access_token(),
                    &domain_id,
                    &data.id,
                )
                .await
                .expect("delete_by_id failed");
            }
        }
    }
}
