use bytes::Bytes;
use futures::{SinkExt, Stream, channel::mpsc, stream::StreamExt};
use reqwest::{Body, Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(not(target_family = "wasm"))]
use tokio::spawn;
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

use crate::errors::{AukiErrorResponse, DomainError};

#[derive(Debug, Deserialize)]
struct InfoResponse {
    upload: InfoUpload,
}

#[derive(Debug, Deserialize)]
struct InfoUpload {
    request_max_bytes: i64,
    multipart: InfoMultipart,
}

#[derive(Debug, Deserialize)]
struct InfoMultipart {
    enabled: bool,
}

async fn try_get_info_v1(url: &str) -> Option<InfoResponse> {
    let resp = Client::new()
        .get(&format!("{}/api/v1/info", url))
        .send()
        .await
        .ok()?;

    if resp.status() == StatusCode::NOT_FOUND {
        return None;
    }
    if !resp.status().is_success() {
        return None;
    }
    resp.json::<InfoResponse>().await.ok()
}

fn is_unsupported_endpoint_status(status: StatusCode) -> bool {
    status == StatusCode::NOT_FOUND
        || status == StatusCode::METHOD_NOT_ALLOWED
        || status == StatusCode::NOT_IMPLEMENTED
}

fn is_unsupported_endpoint_error(err: &DomainError) -> bool {
    match err {
        DomainError::AukiErrorResponse(resp) => is_unsupported_endpoint_status(resp.status),
        _ => false,
    }
}

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
    Create {
        name: String,
        data_type: String,
    },
    Update {
        id: String,
    },
}

#[derive(Debug, Serialize)]
struct InitiateMultipartRequest {
    name: String,
    data_type: String,
    size: Option<i64>,
    content_type: Option<String>,
    existing_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InitiateMultipartResponse {
    upload_id: String,
    part_size: i64,
}

#[derive(Debug, Serialize)]
struct CompletedPart {
    part_number: i32,
    etag: String,
}

#[derive(Debug, Serialize)]
struct CompleteMultipartRequest {
    parts: Vec<CompletedPart>,
}

#[derive(Debug, Deserialize)]
struct UploadPartResult {
    etag: String,
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
) -> Result<Vec<u8>, DomainError> {
    let response = Client::new()
        .get(&format!(
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
        let error = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(AukiErrorResponse { status, error: format!("Failed to download data by id. {}", error) }.into())
    }
}

pub async fn download_metadata_v1(
    url: &str,
    client_id: &str,
    access_token: &str,
    domain_id: &str,
    query: &DownloadQuery,
) -> Result<Vec<DomainDataMetadata>, DomainError> {
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
        Err(AukiErrorResponse { status, error: format!("Failed to download metadata. {}", text) }.into())
    }
}

pub async fn download_v1(
    url: &str,
    client_id: &str,
    access_token: &str,
    domain_id: &str,
    query: &DownloadQuery,
    with_data: bool,
) -> Result<Response, DomainError> {
    let mut params = HashMap::new();

    if let Some(name) = &query.name {
        params.insert("name", name.clone());
    }
    if let Some(data_type) = &query.data_type {
        params.insert("data_type", data_type.clone());
    }
    let ids = {
        if !query.ids.is_empty() {
            let ids = query.ids.join(",");
            if params.is_empty() {
                &format!("?ids={}", ids)
            } else {
                &format!("?ids={}", ids)
            }
        } else {
            ""
        }
    };

    let response = Client::new()
        .get(&format!("{}/api/v1/domains/{}/data{}", url, domain_id, ids))
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
        Err(AukiErrorResponse { status, error: format!("Failed to download data. {}", text) }.into())
    }
}

pub async fn download_v1_stream(
    url: &str,
    client_id: &str,
    access_token: &str,
    domain_id: &str,
    query: &DownloadQuery,
) -> Result<
    mpsc::Receiver<Result<DomainData, DomainError>>,
    DomainError,
> {
    let response = download_v1(url, client_id, access_token, domain_id, query, true).await?;

    let (mut tx, rx) =
        mpsc::channel::<Result<DomainData, DomainError>>(100);

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
            return Err(DomainError::InvalidContentTypeHeader.into());
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
) -> Result<(), DomainError> {
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
        let status = resp.status();
        let err = resp
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(AukiErrorResponse { status, error: format!("Failed to delete data by id. {}", err) }.into())
    }
}

async fn initiate_domain_data_multipart_upload(
    client: &Client,
    url: &str,
    access_token: &str,
    domain_id: &str,
    req: &InitiateMultipartRequest,
) -> Result<InitiateMultipartResponse, DomainError> {
    let resp = client
        .post(&format!(
            "{}/api/v1/domains/{}/data/multipart?uploads",
            url, domain_id
        ))
        .bearer_auth(access_token)
        .header("Content-Type", "application/json")
        .json(req)
        .send()
        .await?;

    if resp.status().is_success() {
        Ok(resp.json::<InitiateMultipartResponse>().await?)
    } else {
        let status = resp.status();
        let err = resp
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(AukiErrorResponse { status, error: format!("Failed to initiate multipart upload. {}", err) }.into())
    }
}

async fn upload_domain_data_multipart_part(
    client: &Client,
    url: &str,
    access_token: &str,
    domain_id: &str,
    upload_id: &str,
    part_number: i32,
    bytes: Bytes,
) -> Result<UploadPartResult, DomainError> {
    let resp = client
        .put(&format!(
            "{}/api/v1/domains/{}/data/multipart?uploadId={}&partNumber={}",
            url, domain_id, upload_id, part_number
        ))
        .bearer_auth(access_token)
        .header("Content-Type", "application/octet-stream")
        .body(bytes)
        .send()
        .await?;

    if resp.status().is_success() {
        Ok(resp.json::<UploadPartResult>().await?)
    } else {
        let status = resp.status();
        let err = resp
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(AukiErrorResponse { status, error: format!("Failed to upload multipart part. {}", err) }.into())
    }
}

async fn complete_domain_data_multipart_upload(
    client: &Client,
    url: &str,
    access_token: &str,
    domain_id: &str,
    upload_id: &str,
    parts: Vec<CompletedPart>,
) -> Result<DomainDataMetadata, DomainError> {
    let resp = client
        .post(&format!(
            "{}/api/v1/domains/{}/data/multipart?uploadId={}",
            url, domain_id, upload_id
        ))
        .bearer_auth(access_token)
        .header("Content-Type", "application/json")
        .json(&CompleteMultipartRequest { parts })
        .send()
        .await?;

    if resp.status().is_success() {
        Ok(resp.json::<DomainDataMetadata>().await?)
    } else {
        let status = resp.status();
        let err = resp
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(AukiErrorResponse { status, error: format!("Failed to complete multipart upload. {}", err) }.into())
    }
}

async fn abort_domain_data_multipart_upload(
    client: &Client,
    url: &str,
    access_token: &str,
    domain_id: &str,
    upload_id: &str,
) -> Result<(), DomainError> {
    let resp = client
        .delete(&format!(
            "{}/api/v1/domains/{}/data/multipart?uploadId={}",
            url, domain_id, upload_id
        ))
        .bearer_auth(access_token)
        .send()
        .await?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let err = resp
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(AukiErrorResponse { status, error: format!("Failed to abort multipart upload. {}", err) }.into())
    }
}

async fn upload_domain_data_multipart_bytes(
    url: &str,
    access_token: &str,
    domain_id: &str,
    action: DomainAction,
    bytes: Bytes,
) -> Result<DomainDataMetadata, DomainError> {
    if bytes.is_empty() {
        return Err(DomainError::InvalidRequest("multipart upload requires non-empty data"));
    }

    let (name, data_type, existing_id) = match action {
        DomainAction::Create { name, data_type } => (name, data_type, None),
        DomainAction::Update { id } => ("".to_string(), "".to_string(), Some(id)),
    };

    let client = Client::new();
    let init_res = initiate_domain_data_multipart_upload(
        &client,
        url,
        access_token,
        domain_id,
        &InitiateMultipartRequest {
            name,
            data_type,
            size: Some(bytes.len() as i64),
            content_type: Some("application/octet-stream".to_string()),
            existing_id,
        },
    )
    .await?;

    let part_size = usize::try_from(init_res.part_size)
        .map_err(|_| DomainError::InvalidRequest("invalid multipart part_size"))?;
    if part_size == 0 {
        return Err(DomainError::InvalidRequest("invalid multipart part_size"));
    }

    let upload_id = init_res.upload_id;
    let mut parts = Vec::new();

    let upload_res = async {
        let mut offset = 0usize;
        let mut part_number: i32 = 1;

        while offset < bytes.len() {
            let end = std::cmp::min(offset + part_size, bytes.len());
            let chunk = bytes.slice(offset..end);

            let res = upload_domain_data_multipart_part(
                &client,
                url,
                access_token,
                domain_id,
                &upload_id,
                part_number,
                chunk,
            )
            .await?;

            parts.push(CompletedPart {
                part_number,
                etag: res.etag,
            });

            offset = end;
            part_number = part_number
                .checked_add(1)
                .ok_or(DomainError::InvalidRequest("multipart upload too many parts"))?;
        }

        complete_domain_data_multipart_upload(
            &client,
            url,
            access_token,
            domain_id,
            &upload_id,
            parts,
        )
        .await
    }
    .await;

    if upload_res.is_err() {
        let _ = abort_domain_data_multipart_upload(&client, url, access_token, domain_id, &upload_id).await;
    }

    upload_res
}

#[cfg(not(target_family = "wasm"))]
pub async fn upload_v1_stream(
    url: &str,
    access_token: &str,
    domain_id: &str,
    mut rx: mpsc::Receiver<UploadDomainData>,
) -> Result<Vec<DomainDataMetadata>, DomainError> {
    use futures::channel::oneshot;

    let boundary = "boundary";

    let info = try_get_info_v1(url).await;
    let request_max_bytes = info
        .as_ref()
        .map(|i| i.upload.request_max_bytes)
        .unwrap_or(0);
    let multipart_enabled = info
        .as_ref()
        .map(|i| i.upload.multipart.enabled)
        .unwrap_or(false);

    // If we can't determine a meaningful request size limit, keep the existing streaming behavior.
    if request_max_bytes <= 0 {
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
            Result<Vec<DomainDataMetadata>, DomainError>,
        >();
        let (update_signal, update_signal_rx) = oneshot::channel::<
            Result<Vec<DomainDataMetadata>, DomainError>,
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
                DomainAction::Create { name, data_type } => {
                    let create_data = write_create_body(boundary, &CreateDomainData { name, data_type }, &datum.data);
                    create_tx.clone().send(create_data).await?;
                }
                DomainAction::Update { id } => {
                    let update_data = write_update_body(boundary, &UpdateDomainData { id }, &datum.data);
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
            match create_signal_rx.await {
                Ok(res) => match res {
                    Ok(d) => d,
                    Err(e) => return Err(e),
                },
                Err(e) => return Err(DomainError::StreamCancelled(e)),
            }
        };

        match update_signal_rx.await {
            Ok(res) => match res {
                Ok(d) => data.extend(d),
                Err(e) => return Err(e),
            },
            Err(e) => return Err(DomainError::StreamCancelled(e)),
        }

        return Ok(data);
    }

    let closing = format!("--{}--\r\n", boundary).into_bytes();
    let closing_len = closing.len();

    struct Batch {
        tx: mpsc::Sender<Vec<u8>>,
        done: oneshot::Receiver<Result<Vec<DomainDataMetadata>, DomainError>>,
        size: usize,
    }

    let mut create_batch: Option<Batch> = None;
    let mut update_batch: Option<Batch> = None;
    let mut create_done = Vec::new();
    let mut update_done = Vec::new();
    let mut create_res = Vec::new();
    let mut update_res = Vec::new();

    let spawn_create_batch = |url: String, access_token: String, domain_id: String| {
        let (tx, rx) = mpsc::channel(100);
        let body = Body::wrap_stream(rx.map(Ok::<Vec<u8>, std::io::Error>));
        let (signal, signal_rx) = oneshot::channel::<Result<Vec<DomainDataMetadata>, DomainError>>();
        spawn(async move {
            let create_response = create_v1(&url, &access_token, &domain_id, boundary, body).await;
            if let Err(Err(e)) = signal.send(create_response) {
                tracing::error!("Failed to send create response: {}", e);
            }
        });
        Batch { tx, done: signal_rx, size: 0 }
    };

    let spawn_update_batch = |url: String, access_token: String, domain_id: String| {
        let (tx, rx) = mpsc::channel(100);
        let body = Body::wrap_stream(rx.map(Ok::<Vec<u8>, std::io::Error>));
        let (signal, signal_rx) = oneshot::channel::<Result<Vec<DomainDataMetadata>, DomainError>>();
        spawn(async move {
            let update_response = update_v1(&url, &access_token, &domain_id, boundary, body).await;
            if let Err(Err(e)) = signal.send(update_response) {
                tracing::error!("Failed to send update response: {}", e);
            }
        });
        Batch { tx, done: signal_rx, size: 0 }
    };

    let base_url = url.to_string();
    let token = access_token.to_string();
    let did = domain_id.to_string();

    while let Some(datum) = rx.next().await {
        let bytes = Bytes::from(datum.data);
        match datum.action {
            DomainAction::Create { name, data_type } => {
                let header = format!(
                    "--{}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: form-data; name=\"{}\"; data-type=\"{}\"\r\n\r\n",
                    boundary, name, data_type
                );
                let part_len = header.as_bytes().len() + bytes.len() + 2;

                let fits_alone = (part_len + closing_len) as i64 <= request_max_bytes;
                if multipart_enabled && !fits_alone {
                    match upload_domain_data_multipart_bytes(
                        &base_url,
                        &token,
                        &did,
                        DomainAction::Create {
                            name: name.clone(),
                            data_type: data_type.clone(),
                        },
                        bytes.clone(),
                    )
                    .await
                    {
                        Ok(meta) => {
                            create_res.push(meta);
                            continue;
                        }
                        Err(e) => {
                            if is_unsupported_endpoint_error(&e) {
                                // Endpoint not supported: fall back to single upload (will likely 413).
                                let mut body = Vec::with_capacity(part_len + closing.len());
                                body.extend_from_slice(header.as_bytes());
                                body.extend_from_slice(bytes.as_ref());
                                body.extend_from_slice("\r\n".as_bytes());
                                body.extend_from_slice(&closing);
                                let res = create_v1(&base_url, &token, &did, boundary, Body::from(body)).await?;
                                create_res.extend(res);
                                continue;
                            }
                            return Err(e);
                        }
                    }
                }

                if create_batch.is_none() {
                    create_batch = Some(spawn_create_batch(base_url.clone(), token.clone(), did.clone()));
                }
                let mut batch = create_batch.take().unwrap();
                if batch.size > 0 && (batch.size + part_len + closing_len) as i64 > request_max_bytes {
                    batch.tx.send(closing.clone()).await?;
                    batch.tx.close().await?;
                    create_done.push(batch.done);
                    batch = spawn_create_batch(base_url.clone(), token.clone(), did.clone());
                }
                let mut part = Vec::with_capacity(part_len);
                part.extend_from_slice(header.as_bytes());
                part.extend_from_slice(bytes.as_ref());
                part.extend_from_slice("\r\n".as_bytes());
                batch.size += part.len();
                batch.tx.send(part).await?;
                create_batch = Some(batch);
            }
            DomainAction::Update { id } => {
                let header = format!(
                    "--{}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: form-data; id=\"{}\"\r\n\r\n",
                    boundary, id
                );
                let part_len = header.as_bytes().len() + bytes.len() + 2;

                let fits_alone = (part_len + closing_len) as i64 <= request_max_bytes;
                if multipart_enabled && !fits_alone {
                    match upload_domain_data_multipart_bytes(
                        &base_url,
                        &token,
                        &did,
                        DomainAction::Update { id: id.clone() },
                        bytes.clone(),
                    )
                    .await
                    {
                        Ok(meta) => {
                            update_res.push(meta);
                            continue;
                        }
                        Err(e) => {
                            if is_unsupported_endpoint_error(&e) {
                                let mut body = Vec::with_capacity(part_len + closing.len());
                                body.extend_from_slice(header.as_bytes());
                                body.extend_from_slice(bytes.as_ref());
                                body.extend_from_slice("\r\n".as_bytes());
                                body.extend_from_slice(&closing);
                                let res = update_v1(&base_url, &token, &did, boundary, Body::from(body)).await?;
                                update_res.extend(res);
                                continue;
                            }
                            return Err(e);
                        }
                    }
                }

                if update_batch.is_none() {
                    update_batch = Some(spawn_update_batch(base_url.clone(), token.clone(), did.clone()));
                }
                let mut batch = update_batch.take().unwrap();
                if batch.size > 0 && (batch.size + part_len + closing_len) as i64 > request_max_bytes {
                    batch.tx.send(closing.clone()).await?;
                    batch.tx.close().await?;
                    update_done.push(batch.done);
                    batch = spawn_update_batch(base_url.clone(), token.clone(), did.clone());
                }
                let mut part = Vec::with_capacity(part_len);
                part.extend_from_slice(header.as_bytes());
                part.extend_from_slice(bytes.as_ref());
                part.extend_from_slice("\r\n".as_bytes());
                batch.size += part.len();
                batch.tx.send(part).await?;
                update_batch = Some(batch);
            }
        }
    }

    if let Some(mut batch) = create_batch {
        batch.tx.send(closing.clone()).await?;
        batch.tx.close().await?;
        create_done.push(batch.done);
    }
    if let Some(mut batch) = update_batch {
        batch.tx.send(closing.clone()).await?;
        batch.tx.close().await?;
        update_done.push(batch.done);
    }

    for done in create_done {
        match done.await {
            Ok(Ok(v)) => create_res.extend(v),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(DomainError::StreamCancelled(e)),
        }
    }

    for done in update_done {
        match done.await {
            Ok(Ok(v)) => update_res.extend(v),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(DomainError::StreamCancelled(e)),
        }
    }

    let mut out = Vec::new();
    out.extend(create_res);
    out.extend(update_res);
    Ok(out)
}

async fn update_v1(
    url: &str,
    access_token: &str,
    domain_id: &str,
    boundary: &str,
    body: Body,
) -> Result<Vec<DomainDataMetadata>, DomainError> {
    let update_response = Client::new()
        .put(&format!("{}/api/v1/domains/{}/data", url, domain_id))
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
        let status = update_response.status();
        let err = update_response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(AukiErrorResponse { status, error: format!("Failed to update data. {}", err) }.into())
    }
}

async fn create_v1(
    url: &str,
    access_token: &str,
    domain_id: &str,
    boundary: &str,
    body: Body,
) -> Result<Vec<DomainDataMetadata>, DomainError> {
    let create_response = Client::new()
        .post(&format!("{}/api/v1/domains/{}/data", url, domain_id))
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
        let status = create_response.status();
        let err = create_response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(AukiErrorResponse { status, error: format!("Failed to create data. {}", err) }.into())
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
) -> Result<Vec<DomainDataMetadata>, DomainError> {
    let boundary = "boundary";

    let info = try_get_info_v1(url).await;
    let request_max_bytes = info
        .as_ref()
        .map(|i| i.upload.request_max_bytes)
        .unwrap_or(0);
    let multipart_enabled = info
        .as_ref()
        .map(|i| i.upload.multipart.enabled)
        .unwrap_or(false);

    // If we can't determine a meaningful request size limit, keep existing single-request behavior.
    if request_max_bytes <= 0 || !multipart_enabled {
        let mut create_body = Vec::new();
        let mut update_body = Vec::new();
        let mut to_update = false;
        let mut to_create = false;

        for datum in data {
            match datum.action {
                DomainAction::Create { name, data_type } => {
                    to_create = true;
                    let create_data = write_create_body(boundary, &CreateDomainData { name, data_type }, &datum.data);
                    create_body.extend_from_slice(&create_data);
                }
                DomainAction::Update { id } => {
                    to_update = true;
                    let update_data = write_update_body(boundary, &UpdateDomainData { id }, &datum.data);
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

        return Ok(res);
    }

    let closing = format!("--{}--\r\n", boundary).into_bytes();
    let closing_len = closing.len();

    let mut create_res = Vec::new();
    let mut update_res = Vec::new();

    let mut create_batch = Vec::new();
    let mut update_batch = Vec::new();

    let mut create_size = 0usize;
    let mut update_size = 0usize;

    for datum in data {
        let bytes = Bytes::from(datum.data);
        match datum.action {
            DomainAction::Create { name, data_type } => {
                let header = format!(
                    "--{}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: form-data; name=\"{}\"; data-type=\"{}\"\r\n\r\n",
                    boundary, name, data_type
                );
                let part_len = header.as_bytes().len() + bytes.len() + 2;
                let fits_alone = (part_len + closing_len) as i64 <= request_max_bytes;

                if multipart_enabled && !fits_alone {
                    if !create_batch.is_empty() {
                        let mut body = std::mem::take(&mut create_batch);
                        body.extend_from_slice(&closing);
                        create_res.extend(create_v1(url, access_token, domain_id, boundary, Body::from(body)).await?);
                        create_size = 0;
                    }
                    match upload_domain_data_multipart_bytes(
                        url,
                        access_token,
                        domain_id,
                        DomainAction::Create {
                            name: name.clone(),
                            data_type: data_type.clone(),
                        },
                        bytes.clone(),
                    )
                    .await
                    {
                        Ok(meta) => {
                            create_res.push(meta);
                        }
                        Err(e) => {
                            if is_unsupported_endpoint_error(&e) {
                                // Fall back to single upload (will likely 413).
                                let mut body = Vec::with_capacity(part_len + closing.len());
                                body.extend_from_slice(header.as_bytes());
                                body.extend_from_slice(bytes.as_ref());
                                body.extend_from_slice("\r\n".as_bytes());
                                body.extend_from_slice(&closing);
                                create_res.extend(create_v1(url, access_token, domain_id, boundary, Body::from(body)).await?);
                            } else {
                                return Err(e);
                            }
                        }
                    }
                    continue;
                }

                if !create_batch.is_empty()
                    && (create_size + part_len + closing_len) as i64 > request_max_bytes
                {
                    let mut body = std::mem::take(&mut create_batch);
                    body.extend_from_slice(&closing);
                    create_res.extend(create_v1(url, access_token, domain_id, boundary, Body::from(body)).await?);
                    create_size = 0;
                }
                create_batch.extend_from_slice(header.as_bytes());
                create_batch.extend_from_slice(bytes.as_ref());
                create_batch.extend_from_slice("\r\n".as_bytes());
                create_size += part_len;
            }
            DomainAction::Update { id } => {
                let header = format!(
                    "--{}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: form-data; id=\"{}\"\r\n\r\n",
                    boundary, id
                );
                let part_len = header.as_bytes().len() + bytes.len() + 2;
                let fits_alone = (part_len + closing_len) as i64 <= request_max_bytes;

                if multipart_enabled && !fits_alone {
                    if !update_batch.is_empty() {
                        let mut body = std::mem::take(&mut update_batch);
                        body.extend_from_slice(&closing);
                        update_res.extend(update_v1(url, access_token, domain_id, boundary, Body::from(body)).await?);
                        update_size = 0;
                    }
                    match upload_domain_data_multipart_bytes(
                        url,
                        access_token,
                        domain_id,
                        DomainAction::Update { id: id.clone() },
                        bytes.clone(),
                    )
                    .await
                    {
                        Ok(meta) => {
                            update_res.push(meta);
                        }
                        Err(e) => {
                            if is_unsupported_endpoint_error(&e) {
                                let mut body = Vec::with_capacity(part_len + closing.len());
                                body.extend_from_slice(header.as_bytes());
                                body.extend_from_slice(bytes.as_ref());
                                body.extend_from_slice("\r\n".as_bytes());
                                body.extend_from_slice(&closing);
                                update_res.extend(update_v1(url, access_token, domain_id, boundary, Body::from(body)).await?);
                            } else {
                                return Err(e);
                            }
                        }
                    }
                    continue;
                }

                if !update_batch.is_empty()
                    && (update_size + part_len + closing_len) as i64 > request_max_bytes
                {
                    let mut body = std::mem::take(&mut update_batch);
                    body.extend_from_slice(&closing);
                    update_res.extend(update_v1(url, access_token, domain_id, boundary, Body::from(body)).await?);
                    update_size = 0;
                }
                update_batch.extend_from_slice(header.as_bytes());
                update_batch.extend_from_slice(bytes.as_ref());
                update_batch.extend_from_slice("\r\n".as_bytes());
                update_size += part_len;
            }
        }
    }

    if !create_batch.is_empty() {
        let mut body = create_batch;
        body.extend_from_slice(&closing);
        create_res.extend(create_v1(url, access_token, domain_id, boundary, Body::from(body)).await?);
    }
    if !update_batch.is_empty() {
        let mut body = update_batch;
        body.extend_from_slice(&closing);
        update_res.extend(update_v1(url, access_token, domain_id, boundary, Body::from(body)).await?);
    }

    let mut res = Vec::new();
    res.extend(create_res);
    res.extend(update_res);
    Ok(res)
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
        Some(i + 4) // body starts after \r\n\r\n
    } else if let Some(i) = data.windows(2).position(|w| w == b"\n\n") {
        Some(i + 2) // body starts after \n\n
    } else {
        None
    }
}

async fn handle_domain_data_stream(
    mut tx: mpsc::Sender<Result<DomainData, DomainError>>,
    stream: impl Stream<Item = Result<Bytes, reqwest::Error>>,
    boundary: &str,
) {
    use futures::pin_mut;

    let mut buffer = Vec::new();
    let mut current_domain_data: Option<DomainData> = None;
    let boundary_bytes = format!("--{}", boundary).as_bytes().to_vec();

    pin_mut!(stream);

    while let Some(chunk_result) = stream.next().await {
        // Handle chunk result
        let chunk = match chunk_result {
            Ok(c) if c.is_empty() => {
                tx.close().await.ok();
                return;
            }
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(Err(e.into())).await;
                return;
            }
        };

        buffer.extend_from_slice(&chunk);

        // If we are in the middle of reading a domain_data part, continue filling it
        if let Some(mut domain_data) = current_domain_data.take() {
            let expected_size = domain_data.metadata.size as usize - domain_data.data.len();
            if buffer.len() >= expected_size {
                domain_data.data.extend_from_slice(&buffer[..expected_size]);
                buffer.drain(..expected_size);
                if tx.send(Ok(domain_data)).await.is_err() {
                    return;
                }
            } else {
                domain_data.data.extend_from_slice(&buffer);
                buffer.clear();
                current_domain_data = Some(domain_data);
                continue;
            }
        }

        // Process all boundaries in the current buffer
        loop {
            // Find the next boundary in the buffer
            let boundary_pos = match find_boundary(&buffer, &boundary_bytes) {
                Some(pos) => pos,
                None => break, // No more boundaries found in current buffer
            };

            // Look for header end after boundary
            let header_end = match find_headers_end(&buffer[boundary_pos..]) {
                Some(end) => end,
                None => break, // Incomplete headers, wait for more chunks
            };

            let headers_slice = &buffer[boundary_pos..boundary_pos + header_end];
            let part_headers = parse_headers(headers_slice);

            let mut domain_data = match part_headers {
                Ok(data) => data,
                Err(e) => {
                    tracing::error!("Failed to parse headers: {:?}", e);
                    return;
                }
            };

            // Remove processed data (boundary + headers) from buffer
            buffer.drain(..boundary_pos + header_end);

            let expected_size = domain_data.metadata.size as usize - domain_data.data.len();
            if buffer.len() >= expected_size {
                domain_data.data.extend_from_slice(&buffer[..expected_size]);
                buffer.drain(..expected_size);
                if tx.send(Ok(domain_data)).await.is_err() {
                    return;
                }
            } else {
                domain_data.data.extend_from_slice(&buffer);
                buffer.clear();
                current_domain_data = Some(domain_data);
                break;
            }

            // Continue to process the next boundary in the same buffer
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use super::*;

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
            .collect::<Vec<Result<DomainData, DomainError>>>()
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
            .collect::<Vec<Result<DomainData, DomainError>>>()
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
            .collect::<Vec<Result<DomainData, DomainError>>>()
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
            .collect::<Vec<Result<DomainData, DomainError>>>()
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
}
