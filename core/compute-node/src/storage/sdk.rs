use super::client::{
    extract_timestamp, map_filename, sanitize_component, DownloadedPart, UploadFileRequest,
    UploadRequest,
};
use crate::errors::StorageError;
use auki_sdk::{
    AukiDomainData, DataError, DataLimits, DataListQuery, DataWrite, DomainDataClient, TaskContext,
    TransferOptions,
};
use std::sync::Arc;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const BUFFER_LIMIT: usize = 64 * 1024 * 1024;

#[derive(Clone)]
pub(super) struct SdkDomainClient {
    data: DomainDataClient,
    cancellation: CancellationToken,
}

impl SdkDomainClient {
    pub(super) fn new(task: &TaskContext) -> anyhow::Result<Self> {
        Ok(Self {
            data: AukiDomainData::with_limits(
                task.credential.clone(),
                DataLimits {
                    max_data_bytes: BUFFER_LIMIT,
                    ..DataLimits::default()
                },
            )?
            .in_domain(task.credential.domain_id()),
            cancellation: task.cancellation(),
        })
    }

    pub(super) async fn close(&self) {
        self.data.close().await;
    }

    fn domain(&self, id: &str) -> Result<(), StorageError> {
        if Uuid::parse_str(id).ok() != Some(self.data.domain_id()) {
            return Err(StorageError::Unauthorized);
        }
        Ok(())
    }

    pub(super) async fn find(
        &self,
        domain: &str,
        name: &str,
        data_type: &str,
    ) -> Result<Option<String>, StorageError> {
        self.domain(domain)?;
        match self
            .data
            .list_with_cancellation(
                &DataListQuery {
                    name: Some(name.into()),
                    data_type: Some(data_type.into()),
                    ..Default::default()
                },
                &self.cancellation,
            )
            .await
        {
            Ok(items) => Ok(items
                .into_iter()
                .find(|v| v.name == name && v.data_type == data_type)
                .map(|v| v.id.to_string())),
            Err(error) if error.status() == Some(404) => Ok(None),
            Err(error) => Err(map_error(error)),
        }
    }

    pub(super) async fn download(
        &self,
        domain: &str,
        query: posemesh_domain_http::domain_data::DownloadQuery,
    ) -> Result<Vec<DownloadedPart>, StorageError> {
        self.domain(domain)?;
        let ids = query
            .ids
            .iter()
            .map(|id| Uuid::parse_str(id).map_err(|_| StorageError::BadRequest))
            .collect::<Result<_, _>>()?;
        let items = self
            .data
            .list_with_cancellation(
                &DataListQuery {
                    ids,
                    name: query.name,
                    data_type: query.data_type,
                },
                &self.cancellation,
            )
            .await
            .map_err(map_error)?;
        if items.is_empty() {
            return Err(StorageError::NotFound);
        }
        let root = std::env::temp_dir().join(format!("domain-input-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).await.map_err(io_error)?;
        let result = async {
            let mut parts = Vec::new();
            for item in items {
                let scan = extract_timestamp(&item.name).unwrap_or_else(|| item.name.clone());
                let path = root
                    .join("datasets")
                    .join(sanitize_component(&scan))
                    .join(map_filename(&item.data_type, &item.name));
                fs::create_dir_all(path.parent().expect("path under download root"))
                    .await
                    .map_err(io_error)?;
                let file = Arc::new(Mutex::new(fs::File::create(&path).await.map_err(io_error)?));
                self.data
                    .read_to(
                        item.id,
                        transfer_options(item.size.max(1)),
                        &self.cancellation,
                        move |bytes| {
                            let file = file.clone();
                            async move {
                                file.lock()
                                    .await
                                    .write_all(&bytes)
                                    .await
                                    .map_err(|_| DataError::Callback)
                            }
                        },
                    )
                    .await
                    .map_err(map_error)?;
                parts.push(DownloadedPart {
                    id: Some(item.id.to_string()),
                    name: Some(item.name),
                    data_type: Some(item.data_type),
                    domain_id: Some(item.domain_id.to_string()),
                    relative_path: path
                        .strip_prefix(&root)
                        .expect("path under download root")
                        .to_path_buf(),
                    path,
                    root: root.clone(),
                    extracted_paths: Vec::new(),
                });
            }
            Ok(parts)
        }
        .await;
        if result.is_err() {
            let _ = fs::remove_dir_all(&root).await;
        }
        result
    }

    pub(super) async fn upload(
        &self,
        request: UploadRequest<'_>,
    ) -> Result<Option<String>, StorageError> {
        self.domain(request.domain_id)?;
        let target = target(request.existing_id, request.name, request.data_type)?;
        let result = self
            .data
            .write_with_cancellation(target, request.bytes, &self.cancellation)
            .await;
        let saved = match result {
            Ok(saved) => saved,
            Err(DataError::TooLarge { .. }) if !request.bytes.is_empty() => {
                let mut offset = 0;
                self.data
                    .write_stream(
                        target,
                        request.bytes.len() as u64,
                        transfer_options(request.bytes.len() as u64),
                        &self.cancellation,
                        |maximum| {
                            let end = (offset + maximum).min(request.bytes.len());
                            let bytes = request.bytes[offset..end].to_vec();
                            offset = end;
                            std::future::ready(Ok(bytes))
                        },
                    )
                    .await
                    .map_err(map_error)?
            }
            Err(error) => return Err(map_error(error)),
        };
        Ok(Some(saved.id.to_string()))
    }

    pub(super) async fn upload_file(
        &self,
        request: UploadFileRequest<'_>,
    ) -> Result<Option<String>, StorageError> {
        self.domain(request.domain_id)?;
        let mut file = fs::File::open(request.path).await.map_err(io_error)?;
        let size = file.metadata().await.map_err(io_error)?.len();
        if size == 0 {
            return Err(StorageError::BadRequest);
        }
        if size <= BUFFER_LIMIT as u64 {
            let mut bytes = Vec::with_capacity(size as usize);
            (&mut file)
                .take(BUFFER_LIMIT as u64 + 1)
                .read_to_end(&mut bytes)
                .await
                .map_err(io_error)?;
            if bytes.len() > BUFFER_LIMIT {
                return Err(StorageError::Other(
                    "upload file grew beyond buffered limit".into(),
                ));
            }
            return self
                .upload(UploadRequest {
                    domain_id: request.domain_id,
                    name: request.name,
                    data_type: request.data_type,
                    logical_path: request.logical_path,
                    existing_id: request.existing_id,
                    bytes: &bytes,
                })
                .await;
        }
        let file = Arc::new(Mutex::new(file));
        let saved = self
            .data
            .write_stream(
                target(request.existing_id, request.name, request.data_type)?,
                size,
                transfer_options(size),
                &self.cancellation,
                move |maximum| {
                    let file = file.clone();
                    async move {
                        let mut bytes = vec![0; maximum];
                        let mut read = 0;
                        let mut file = file.lock().await;
                        while read < maximum {
                            let n = file
                                .read(&mut bytes[read..])
                                .await
                                .map_err(|_| DataError::Callback)?;
                            if n == 0 {
                                break;
                            }
                            read += n;
                        }
                        bytes.truncate(read);
                        Ok(bytes)
                    }
                },
            )
            .await
            .map_err(map_error)?;
        Ok(Some(saved.id.to_string()))
    }
}

fn target<'a>(
    id: Option<&str>,
    name: &'a str,
    data_type: &'a str,
) -> Result<DataWrite<'a>, StorageError> {
    Ok(match id {
        Some(id) => DataWrite::ById(Uuid::parse_str(id).map_err(|_| StorageError::BadRequest)?),
        None => DataWrite::Named { name, data_type },
    })
}

fn transfer_options(size: u64) -> TransferOptions {
    TransferOptions {
        max_bytes: size,
        ..TransferOptions::default()
    }
}

fn io_error(error: std::io::Error) -> StorageError {
    StorageError::Other(error.to_string())
}

fn map_error(error: DataError) -> StorageError {
    match error.status() {
        Some(400) => StorageError::BadRequest,
        Some(401) => StorageError::Unauthorized,
        Some(404) => StorageError::NotFound,
        Some(409) => StorageError::Conflict,
        Some(status @ 500..=599) => StorageError::Server(status),
        _ => StorageError::Other(error.to_string()),
    }
}
