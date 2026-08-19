use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use auki_p2p::{
    ApplicationProtocol, AuthenticatedStream, IncomingAuthenticatedStreams, Multiaddr, Node,
    PeerId, PeerRole, Protocol, SessionRequirements,
};
use chrono::{DateTime, Utc};
use compute_runner_api::{
    P2pDataset, P2pDatasetReference, P2pDatasetRegistration, P2P_DATASET_SCHEMA,
};
use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt as TokioAsyncReadExt, AsyncWriteExt as TokioAsyncWriteExt},
    sync::RwLock,
    task::{JoinHandle, JoinSet},
};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use uuid::Uuid;

use crate::dds::p2p::{DdsP2pError, P2pCredentialStore};

pub const DATASET_PROTOCOL: &str = "/auki-p2p/dataset/0";
const DATASET_REQUEST_VERSION: u8 = 0;
const MAX_REQUEST_BYTES: usize = 4 * 1024;
const MAX_RESPONSE_HEADER_BYTES: usize = 16 * 1024;
const MAX_DATASET_ID_BYTES: usize = 512;
const MAX_DATASET_NAME_BYTES: usize = 1024;
const TRANSFER_BUFFER_BYTES: usize = 64 * 1024;
const FETCH_ATTEMPTS: usize = 2;
const FETCH_RETRY_DELAY: Duration = Duration::from_millis(50);
const FETCH_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, thiserror::Error)]
pub enum P2pDatasetError {
    #[error("P2P dataset credential is unavailable or unauthorized")]
    Credential(#[source] DdsP2pError),
    #[error("P2P dataset transport failed")]
    Transport(#[source] auki_p2p::Error),
    #[error("P2P dataset file operation failed")]
    Io(#[source] std::io::Error),
    #[error("P2P dataset protocol payload is malformed")]
    Json(#[source] serde_json::Error),
    #[error("P2P dataset request version is unsupported")]
    UnsupportedRequestVersion,
    #[error("P2P dataset protocol frame exceeds its limit")]
    FrameTooLarge,
    #[error("P2P dataset identifier is invalid")]
    InvalidDatasetId,
    #[error("P2P dataset name is invalid")]
    InvalidDatasetName,
    #[error("P2P dataset reference is invalid")]
    InvalidReference,
    #[error("P2P dataset file is empty")]
    EmptyDataset,
    #[error("P2P dataset is not registered")]
    UnknownDataset,
    #[error("P2P dataset is no longer available")]
    ExpiredDataset,
    #[error("P2P dataset service is not serving the requested Domain")]
    ServingDomainMismatch,
    #[error("P2P dataset service has no explicit advertised multiaddr")]
    MissingAdvertisedAddress,
    #[error("P2P dataset response does not match its source reference")]
    ReferenceMismatch,
    #[error("P2P dataset transfer ended before the declared size")]
    InterruptedTransfer,
    #[error("P2P dataset response size does not match its source reference")]
    SizeMismatch,
    #[error("P2P dataset response hash does not match its source reference")]
    HashMismatch,
    #[error("P2P dataset transfer timed out")]
    TransferTimeout,
    #[error("P2P dataset server task failed")]
    ServerTask(#[source] tokio::task::JoinError),
}

pub type Result<T> = std::result::Result<T, P2pDatasetError>;

#[derive(Clone)]
pub struct P2pDatasetAdapter {
    inner: Arc<AdapterInner>,
}

struct AdapterInner {
    node: Node,
    credentials: P2pCredentialStore,
    advertised_multiaddrs: Vec<Multiaddr>,
    registry: RwLock<HashMap<String, RegisteredDataset>>,
    serving_domain: RwLock<Option<Uuid>>,
}

#[derive(Clone)]
struct RegisteredDataset {
    dataset_id: String,
    domain_id: Uuid,
    path: PathBuf,
    size_bytes: u64,
    sha256: String,
    available_until: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
struct DatasetRequest {
    version: u8,
    dataset_id: String,
}

#[derive(Serialize, Deserialize)]
struct DatasetResponseHeader {
    dataset_id: String,
    size_bytes: u64,
    sha256: String,
}

impl P2pDatasetAdapter {
    pub fn new(
        node: Node,
        credentials: P2pCredentialStore,
        advertised_multiaddrs: Vec<Multiaddr>,
    ) -> Self {
        Self {
            inner: Arc::new(AdapterInner {
                node,
                credentials,
                advertised_multiaddrs,
                registry: RwLock::new(HashMap::new()),
                serving_domain: RwLock::new(None),
            }),
        }
    }

    pub fn credentials(&self) -> P2pCredentialStore {
        self.inner.credentials.clone()
    }

    pub async fn start_serving(
        &self,
        domain_id: Uuid,
        shutdown: &CancellationToken,
    ) -> Result<P2pDatasetServer> {
        if self.inner.advertised_multiaddrs.is_empty() {
            return Err(P2pDatasetError::MissingAdvertisedAddress);
        }
        self.inner
            .credentials
            .require(PeerRole::Robot, domain_id)
            .await
            .map_err(P2pDatasetError::Credential)?;

        let mut serving_domain = self.inner.serving_domain.write().await;
        if serving_domain.is_some() {
            return Err(P2pDatasetError::ServingDomainMismatch);
        }
        let protocol =
            ApplicationProtocol::new(DATASET_PROTOCOL).map_err(P2pDatasetError::Transport)?;
        let requirements = SessionRequirements::new(domain_id.to_string(), PeerRole::Compute)
            .map_err(P2pDatasetError::Transport)?;
        let incoming = self
            .inner
            .node
            .accept(protocol, requirements)
            .map_err(P2pDatasetError::Transport)?;
        *serving_domain = Some(domain_id);
        drop(serving_domain);

        let stop = shutdown.child_token();
        let task_stop = stop.clone();
        let adapter = self.clone();
        let task = tokio::spawn(async move {
            adapter.run_server(incoming, task_stop).await;
        });
        Ok(P2pDatasetServer {
            stop,
            task: Some(task),
        })
    }

    pub async fn register_dataset(
        &self,
        registration: P2pDatasetRegistration,
    ) -> Result<P2pDatasetReference> {
        validate_dataset_id(&registration.dataset_id)?;
        validate_dataset_name(&registration.name)?;
        if registration.available_until <= Utc::now() {
            return Err(P2pDatasetError::ExpiredDataset);
        }
        if self.inner.advertised_multiaddrs.is_empty() {
            return Err(P2pDatasetError::MissingAdvertisedAddress);
        }
        let serving_domain = *self.inner.serving_domain.read().await;
        if serving_domain != Some(registration.domain_id) {
            return Err(P2pDatasetError::ServingDomainMismatch);
        }
        self.inner
            .credentials
            .require(PeerRole::Robot, registration.domain_id)
            .await
            .map_err(P2pDatasetError::Credential)?;

        let path = fs::canonicalize(&registration.path)
            .await
            .map_err(P2pDatasetError::Io)?;
        let metadata = fs::metadata(&path).await.map_err(P2pDatasetError::Io)?;
        if !metadata.is_file() {
            return Err(P2pDatasetError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "registered dataset is not a file",
            )));
        }
        let (size_bytes, sha256) = hash_file(&path).await?;
        if size_bytes == 0 {
            return Err(P2pDatasetError::EmptyDataset);
        }
        let registered = RegisteredDataset {
            dataset_id: registration.dataset_id.clone(),
            domain_id: registration.domain_id,
            path,
            size_bytes,
            sha256: sha256.clone(),
            available_until: registration.available_until,
        };

        let mut registry = self.inner.registry.write().await;
        prune_expired(&mut registry);
        registry.insert(registration.dataset_id.clone(), registered);
        drop(registry);

        Ok(P2pDatasetReference {
            schema: P2P_DATASET_SCHEMA.to_string(),
            dataset_id: registration.dataset_id,
            domain_id: registration.domain_id,
            name: registration.name,
            peer_id: self.inner.node.peer_id().to_string(),
            multiaddrs: self
                .inner
                .advertised_multiaddrs
                .iter()
                .map(ToString::to_string)
                .collect(),
            size_bytes,
            sha256,
            available_until: registration.available_until,
        })
    }

    pub async fn fetch_dataset(
        &self,
        reference: &P2pDatasetReference,
        destination: &Path,
    ) -> Result<()> {
        let (peer_id, multiaddrs) = validate_reference(reference)?;
        self.inner
            .credentials
            .require(PeerRole::Compute, reference.domain_id)
            .await
            .map_err(P2pDatasetError::Credential)?;

        let parent = destination.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)
            .await
            .map_err(P2pDatasetError::Io)?;

        let mut last_error = None;
        for attempt in 0..FETCH_ATTEMPTS {
            let temp_path = partial_path(destination);
            let transfer = self.fetch_once(
                reference,
                destination,
                &temp_path,
                peer_id,
                multiaddrs.clone(),
            );
            let result = match tokio::time::timeout(FETCH_ATTEMPT_TIMEOUT, transfer).await {
                Ok(result) => result,
                Err(_) => Err(P2pDatasetError::TransferTimeout),
            };
            match result {
                Ok(()) => return Ok(()),
                Err(error) => {
                    let _ = fs::remove_file(&temp_path).await;
                    let retry = error.is_retryable() && attempt + 1 < FETCH_ATTEMPTS;
                    last_error = Some(error);
                    if !retry {
                        break;
                    }
                    let _ = self.inner.node.disconnect(peer_id).await;
                    tokio::time::sleep(FETCH_RETRY_DELAY).await;
                }
            }
        }
        Err(last_error.unwrap_or(P2pDatasetError::InterruptedTransfer))
    }

    async fn fetch_once(
        &self,
        reference: &P2pDatasetReference,
        destination: &Path,
        temp_path: &Path,
        peer_id: PeerId,
        multiaddrs: Vec<Multiaddr>,
    ) -> Result<()> {
        let protocol =
            ApplicationProtocol::new(DATASET_PROTOCOL).map_err(P2pDatasetError::Transport)?;
        let requirements =
            SessionRequirements::new(reference.domain_id.to_string(), PeerRole::Robot)
                .map_err(P2pDatasetError::Transport)?
                .with_expected_remote_peer_id(peer_id);
        let mut stream = self
            .inner
            .node
            .open(peer_id, multiaddrs, protocol, requirements)
            .await
            .map_err(P2pDatasetError::Transport)?;

        write_json_frame(
            &mut stream,
            &DatasetRequest {
                version: DATASET_REQUEST_VERSION,
                dataset_id: reference.dataset_id.clone(),
            },
            MAX_REQUEST_BYTES,
        )
        .await?;
        stream.flush().await.map_err(P2pDatasetError::Io)?;
        let header: DatasetResponseHeader =
            read_json_frame(&mut stream, MAX_RESPONSE_HEADER_BYTES).await?;
        if header.dataset_id != reference.dataset_id
            || header.size_bytes != reference.size_bytes
            || !header.sha256.eq_ignore_ascii_case(&reference.sha256)
        {
            return Err(P2pDatasetError::ReferenceMismatch);
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temp_path)
            .await
            .map_err(P2pDatasetError::Io)?;
        let mut hasher = Sha256::new();
        let mut received = 0_u64;
        let mut buffer = vec![0_u8; TRANSFER_BUFFER_BYTES];
        while received < reference.size_bytes {
            let remaining = reference.size_bytes - received;
            let limit = usize::try_from(remaining)
                .unwrap_or(usize::MAX)
                .min(buffer.len());
            let count = stream
                .read(&mut buffer[..limit])
                .await
                .map_err(P2pDatasetError::Io)?;
            if count == 0 {
                return Err(P2pDatasetError::InterruptedTransfer);
            }
            TokioAsyncWriteExt::write_all(&mut file, &buffer[..count])
                .await
                .map_err(P2pDatasetError::Io)?;
            hasher.update(&buffer[..count]);
            received += count as u64;
        }
        let mut extra = [0_u8; 1];
        if stream.read(&mut extra).await.map_err(P2pDatasetError::Io)? != 0 {
            return Err(P2pDatasetError::SizeMismatch);
        }
        if !hex::encode(hasher.finalize()).eq_ignore_ascii_case(&reference.sha256) {
            return Err(P2pDatasetError::HashMismatch);
        }
        TokioAsyncWriteExt::flush(&mut file)
            .await
            .map_err(P2pDatasetError::Io)?;
        file.sync_all().await.map_err(P2pDatasetError::Io)?;
        drop(file);
        fs::rename(temp_path, destination)
            .await
            .map_err(P2pDatasetError::Io)?;
        Ok(())
    }

    async fn run_server(
        &self,
        mut incoming: IncomingAuthenticatedStreams,
        stop: CancellationToken,
    ) {
        let mut handlers = JoinSet::new();
        loop {
            tokio::select! {
                _ = stop.cancelled() => break,
                incoming_stream = incoming.accept() => {
                    match incoming_stream {
                        Some(Ok(stream)) => {
                            let adapter = self.clone();
                            handlers.spawn(async move {
                                if let Err(error) = adapter.serve_stream(stream).await {
                                    warn!(error = %error, "P2P dataset stream failed");
                                }
                            });
                        }
                        Some(Err(error)) => {
                            warn!(error = %error, "P2P dataset session authentication failed");
                        }
                        None => break,
                    }
                }
                joined = handlers.join_next(), if !handlers.is_empty() => {
                    if let Some(Err(error)) = joined {
                        warn!(error = %error, "P2P dataset stream task failed");
                    }
                }
            }
        }
        handlers.abort_all();
        while handlers.join_next().await.is_some() {}
        *self.inner.serving_domain.write().await = None;
    }

    async fn serve_stream(&self, mut stream: AuthenticatedStream) -> Result<()> {
        let request: DatasetRequest = read_json_frame(&mut stream, MAX_REQUEST_BYTES).await?;
        if request.version != DATASET_REQUEST_VERSION {
            return Err(P2pDatasetError::UnsupportedRequestVersion);
        }
        validate_dataset_id(&request.dataset_id)?;
        let entry = {
            let mut registry = self.inner.registry.write().await;
            prune_expired(&mut registry);
            registry
                .get(&request.dataset_id)
                .cloned()
                .ok_or(P2pDatasetError::UnknownDataset)?
        };
        self.inner
            .credentials
            .require(PeerRole::Robot, entry.domain_id)
            .await
            .map_err(P2pDatasetError::Credential)?;

        let header = DatasetResponseHeader {
            dataset_id: entry.dataset_id,
            size_bytes: entry.size_bytes,
            sha256: entry.sha256,
        };
        write_json_frame(&mut stream, &header, MAX_RESPONSE_HEADER_BYTES).await?;
        let mut file = File::open(&entry.path).await.map_err(P2pDatasetError::Io)?;
        let mut remaining = entry.size_bytes;
        let mut buffer = vec![0_u8; TRANSFER_BUFFER_BYTES];
        while remaining > 0 {
            let limit = usize::try_from(remaining)
                .unwrap_or(usize::MAX)
                .min(buffer.len());
            let count = TokioAsyncReadExt::read(&mut file, &mut buffer[..limit])
                .await
                .map_err(P2pDatasetError::Io)?;
            if count == 0 {
                return Err(P2pDatasetError::InterruptedTransfer);
            }
            stream
                .write_all(&buffer[..count])
                .await
                .map_err(P2pDatasetError::Io)?;
            remaining -= count as u64;
        }
        stream.flush().await.map_err(P2pDatasetError::Io)?;
        stream.close().await.map_err(P2pDatasetError::Io)?;
        Ok(())
    }
}

#[async_trait]
impl P2pDataset for P2pDatasetAdapter {
    async fn register(
        &self,
        registration: P2pDatasetRegistration,
    ) -> anyhow::Result<P2pDatasetReference> {
        Ok(self.register_dataset(registration).await?)
    }

    async fn fetch(
        &self,
        reference: &P2pDatasetReference,
        destination: &Path,
    ) -> anyhow::Result<()> {
        Ok(self.fetch_dataset(reference, destination).await?)
    }
}

pub struct P2pDatasetServer {
    stop: CancellationToken,
    task: Option<JoinHandle<()>>,
}

impl P2pDatasetServer {
    pub async fn shutdown(mut self) -> Result<()> {
        self.stop.cancel();
        if let Some(task) = self.task.take() {
            task.await.map_err(P2pDatasetError::ServerTask)?;
        }
        Ok(())
    }
}

impl Drop for P2pDatasetServer {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

impl P2pDatasetError {
    fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Transport(_)
                | Self::Io(_)
                | Self::Json(_)
                | Self::InterruptedTransfer
                | Self::TransferTimeout
        )
    }
}

fn validate_dataset_id(dataset_id: &str) -> Result<()> {
    let dataset_id = dataset_id.trim();
    if dataset_id.is_empty() || dataset_id.len() > MAX_DATASET_ID_BYTES {
        return Err(P2pDatasetError::InvalidDatasetId);
    }
    Ok(())
}

fn validate_dataset_name(name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() || name.len() > MAX_DATASET_NAME_BYTES {
        return Err(P2pDatasetError::InvalidDatasetName);
    }
    Ok(())
}

fn validate_reference(reference: &P2pDatasetReference) -> Result<(PeerId, Vec<Multiaddr>)> {
    validate_dataset_id(&reference.dataset_id)?;
    validate_dataset_name(&reference.name)?;
    if reference.schema != P2P_DATASET_SCHEMA
        || reference.available_until <= Utc::now()
        || reference.multiaddrs.is_empty()
        || reference.size_bytes == 0
    {
        return Err(if reference.available_until <= Utc::now() {
            P2pDatasetError::ExpiredDataset
        } else {
            P2pDatasetError::InvalidReference
        });
    }
    let hash = hex::decode(&reference.sha256).map_err(|_| P2pDatasetError::InvalidReference)?;
    if hash.len() != 32 {
        return Err(P2pDatasetError::InvalidReference);
    }
    let peer_id =
        PeerId::from_str(&reference.peer_id).map_err(|_| P2pDatasetError::InvalidReference)?;
    let multiaddrs = reference
        .multiaddrs
        .iter()
        .map(|address| Multiaddr::from_str(address).map_err(|_| P2pDatasetError::InvalidReference))
        .collect::<Result<Vec<_>>>()?;
    if multiaddrs.iter().any(|address| {
        !address
            .iter()
            .any(|protocol| matches!(protocol, Protocol::Tcp(_)))
    }) {
        return Err(P2pDatasetError::InvalidReference);
    }
    Ok((peer_id, multiaddrs))
}

async fn hash_file(path: &Path) -> Result<(u64, String)> {
    let mut file = File::open(path).await.map_err(P2pDatasetError::Io)?;
    let mut hasher = Sha256::new();
    let mut size_bytes = 0_u64;
    let mut buffer = vec![0_u8; TRANSFER_BUFFER_BYTES];
    loop {
        let count = TokioAsyncReadExt::read(&mut file, &mut buffer)
            .await
            .map_err(P2pDatasetError::Io)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        size_bytes += count as u64;
    }
    Ok((size_bytes, hex::encode(hasher.finalize())))
}

fn prune_expired(registry: &mut HashMap<String, RegisteredDataset>) {
    let now = Utc::now();
    registry.retain(|_, dataset| dataset.available_until > now);
}

fn partial_path(destination: &Path) -> PathBuf {
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("dataset");
    destination.with_file_name(format!(".{file_name}.{}.part", Uuid::new_v4()))
}

async fn write_json_frame<W, T>(writer: &mut W, value: &T, limit: usize) -> Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let encoded = serde_json::to_vec(value).map_err(P2pDatasetError::Json)?;
    if encoded.len() > limit || encoded.len() > u32::MAX as usize {
        return Err(P2pDatasetError::FrameTooLarge);
    }
    writer
        .write_all(&(encoded.len() as u32).to_be_bytes())
        .await
        .map_err(P2pDatasetError::Io)?;
    writer
        .write_all(&encoded)
        .await
        .map_err(P2pDatasetError::Io)?;
    Ok(())
}

async fn read_json_frame<R, T>(reader: &mut R, limit: usize) -> Result<T>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let mut length = [0_u8; 4];
    reader
        .read_exact(&mut length)
        .await
        .map_err(P2pDatasetError::Io)?;
    let length = u32::from_be_bytes(length) as usize;
    if length > limit {
        return Err(P2pDatasetError::FrameTooLarge);
    }
    let mut encoded = vec![0_u8; length];
    reader
        .read_exact(&mut encoded)
        .await
        .map_err(P2pDatasetError::Io)?;
    serde_json::from_slice(&encoded).map_err(P2pDatasetError::Json)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use auki_p2p::{
        DdsTokenVerifier, Identity, P2PAccessClaims, P2P_TOKEN_AUDIENCE, P2P_TOKEN_ISSUER,
        P2P_TOKEN_SCOPE, P2P_TOKEN_TTL, P2P_TOKEN_TYPE,
    };
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use tempfile::TempDir;

    use super::*;

    const TEST_DDS_PRIVATE_KEY: &[u8] = br#"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQggm4twpf4y/yNNw/k
fqecEEl4zBTwZdRDFUFp/fSxV8qhRANCAARUxrDWJ0AtEGTAYZ4412VPHqMCKoPw
UphDkcOIk7SODsKwUvTIiUr11NbXBJmbBRfhERczsuK4PVha5eg0fVqo
-----END PRIVATE KEY-----"#;

    const TEST_DDS_PUBLIC_KEY: &[u8] = br#"-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEVMaw1idALRBkwGGeONdlTx6jAiqD
8FKYQ5HDiJO0jg7CsFL0yIlK9dTW1wSZmwUX4REXM7LiuD1YWuXoNH1aqA==
-----END PUBLIC KEY-----"#;

    #[tokio::test(flavor = "multi_thread")]
    async fn registered_dataset_survives_registration_scope_and_streams_without_buffering() {
        let domain_id = Uuid::new_v4();
        let robot = listening_node();
        let robot_address = listen_address(&robot).await;
        let robot_credentials = P2pCredentialStore::new(robot.clone());
        install_current_token(
            &robot_credentials,
            &robot,
            PeerRole::Robot,
            domain_id,
            unix_time(),
        )
        .await;
        let robot_adapter =
            P2pDatasetAdapter::new(robot.clone(), robot_credentials, vec![robot_address]);
        let shutdown = CancellationToken::new();
        let server = robot_adapter
            .start_serving(domain_id, &shutdown)
            .await
            .unwrap();

        let compute = unbound_node();
        let compute_credentials = P2pCredentialStore::new(compute.clone());
        install_current_token(
            &compute_credentials,
            &compute,
            PeerRole::Compute,
            domain_id,
            unix_time(),
        )
        .await;
        let compute_adapter = P2pDatasetAdapter::new(compute.clone(), compute_credentials, vec![]);

        let temp = TempDir::new().unwrap();
        let first_bytes = zip_like_bytes(3 * TRANSFER_BUFFER_BYTES + 17);
        let first_path = temp.path().join("first.zip");
        fs::write(&first_path, &first_bytes).await.unwrap();
        let first_reference = robot_adapter
            .register_dataset(registration("first", domain_id, first_path))
            .await
            .unwrap();

        let second_path = temp.path().join("second.zip");
        fs::write(&second_path, zip_like_bytes(128)).await.unwrap();
        robot_adapter
            .register_dataset(registration("second", domain_id, second_path))
            .await
            .unwrap();

        let destination = temp.path().join("downloaded.zip");
        compute_adapter
            .fetch_dataset(&first_reference, &destination)
            .await
            .unwrap();
        assert_eq!(fs::read(&destination).await.unwrap(), first_bytes);
        assert_no_partial_files(&temp).await;

        tokio::time::timeout(Duration::from_secs(2), server.shutdown())
            .await
            .expect("dataset server shutdown timed out")
            .unwrap();
        robot.shutdown().await.unwrap();
        compute.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unknown_expired_and_mismatched_references_fail_without_partial_files() {
        let domain_id = Uuid::new_v4();
        let robot = listening_node();
        let robot_address = listen_address(&robot).await;
        let robot_credentials = P2pCredentialStore::new(robot.clone());
        install_current_token(
            &robot_credentials,
            &robot,
            PeerRole::Robot,
            domain_id,
            unix_time(),
        )
        .await;
        let robot_adapter =
            P2pDatasetAdapter::new(robot.clone(), robot_credentials, vec![robot_address]);
        let shutdown = CancellationToken::new();
        let server = robot_adapter
            .start_serving(domain_id, &shutdown)
            .await
            .unwrap();

        let compute = unbound_node();
        let compute_credentials = P2pCredentialStore::new(compute.clone());
        install_current_token(
            &compute_credentials,
            &compute,
            PeerRole::Compute,
            domain_id,
            unix_time(),
        )
        .await;
        let compute_adapter = P2pDatasetAdapter::new(compute.clone(), compute_credentials, vec![]);
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.zip");
        fs::write(&source, zip_like_bytes(4096)).await.unwrap();
        let reference = robot_adapter
            .register_dataset(registration("known", domain_id, source))
            .await
            .unwrap();

        let mut unknown = reference.clone();
        unknown.dataset_id = "unknown".into();
        assert!(compute_adapter
            .fetch_dataset(&unknown, &temp.path().join("unknown.zip"))
            .await
            .is_err());

        let mut expired = reference.clone();
        expired.available_until = Utc::now() - chrono::Duration::seconds(1);
        assert!(matches!(
            compute_adapter
                .fetch_dataset(&expired, &temp.path().join("expired.zip"))
                .await,
            Err(P2pDatasetError::ExpiredDataset)
        ));

        let mut wrong_size = reference.clone();
        wrong_size.size_bytes += 1;
        assert!(matches!(
            compute_adapter
                .fetch_dataset(&wrong_size, &temp.path().join("wrong-size.zip"))
                .await,
            Err(P2pDatasetError::ReferenceMismatch)
        ));

        let mut wrong_hash = reference;
        wrong_hash.sha256 = "00".repeat(32);
        assert!(matches!(
            compute_adapter
                .fetch_dataset(&wrong_hash, &temp.path().join("wrong-hash.zip"))
                .await,
            Err(P2pDatasetError::ReferenceMismatch)
        ));
        assert_no_partial_files(&temp).await;

        server.shutdown().await.unwrap();
        robot.shutdown().await.unwrap();
        compute.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn receiver_rejects_body_size_and_hash_mismatches_without_completion() {
        let domain_id = Uuid::new_v4();
        let expected = zip_like_bytes(TRANSFER_BUFFER_BYTES + 29);
        let expected_sha256 = hex::encode(Sha256::digest(&expected));

        let robot = listening_node();
        let robot_address = listen_address(&robot).await;
        let robot_credentials = P2pCredentialStore::new(robot.clone());
        install_current_token(
            &robot_credentials,
            &robot,
            PeerRole::Robot,
            domain_id,
            unix_time(),
        )
        .await;
        let mut incoming = robot
            .accept(
                ApplicationProtocol::new(DATASET_PROTOCOL).unwrap(),
                SessionRequirements::new(domain_id.to_string(), PeerRole::Compute).unwrap(),
            )
            .unwrap();

        let compute = unbound_node();
        let compute_credentials = P2pCredentialStore::new(compute.clone());
        install_current_token(
            &compute_credentials,
            &compute,
            PeerRole::Compute,
            domain_id,
            unix_time(),
        )
        .await;
        let compute_adapter = P2pDatasetAdapter::new(compute.clone(), compute_credentials, vec![]);
        let reference = |dataset_id: &str| P2pDatasetReference {
            schema: P2P_DATASET_SCHEMA.into(),
            dataset_id: dataset_id.into(),
            domain_id,
            name: format!("{dataset_id}.zip"),
            peer_id: robot.peer_id().to_string(),
            multiaddrs: vec![robot_address.to_string()],
            size_bytes: expected.len() as u64,
            sha256: expected_sha256.clone(),
            available_until: Utc::now() + chrono::Duration::minutes(10),
        };
        let hash_reference = reference("wrong-body-hash");
        let size_reference = reference("wrong-body-size");

        let server_bytes = expected.clone();
        let server_sha256 = expected_sha256.clone();
        let server = tokio::spawn(async move {
            let mut hash_stream = incoming.accept().await.unwrap().unwrap();
            let hash_request: DatasetRequest = read_json_frame(&mut hash_stream, MAX_REQUEST_BYTES)
                .await
                .unwrap();
            write_json_frame(
                &mut hash_stream,
                &DatasetResponseHeader {
                    dataset_id: hash_request.dataset_id,
                    size_bytes: server_bytes.len() as u64,
                    sha256: server_sha256.clone(),
                },
                MAX_RESPONSE_HEADER_BYTES,
            )
            .await
            .unwrap();
            let mut corrupted = server_bytes.clone();
            *corrupted.last_mut().unwrap() ^= 0xff;
            hash_stream.write_all(&corrupted).await.unwrap();
            hash_stream.close().await.unwrap();

            let mut size_stream = incoming.accept().await.unwrap().unwrap();
            let size_request: DatasetRequest = read_json_frame(&mut size_stream, MAX_REQUEST_BYTES)
                .await
                .unwrap();
            write_json_frame(
                &mut size_stream,
                &DatasetResponseHeader {
                    dataset_id: size_request.dataset_id,
                    size_bytes: server_bytes.len() as u64,
                    sha256: server_sha256,
                },
                MAX_RESPONSE_HEADER_BYTES,
            )
            .await
            .unwrap();
            size_stream.write_all(&server_bytes).await.unwrap();
            size_stream.write_all(&[0xff]).await.unwrap();
            size_stream.close().await.unwrap();
        });

        let temp = TempDir::new().unwrap();
        let hash_destination = temp.path().join("wrong-body-hash.zip");
        assert!(matches!(
            compute_adapter
                .fetch_dataset(&hash_reference, &hash_destination)
                .await,
            Err(P2pDatasetError::HashMismatch)
        ));
        assert!(fs::metadata(&hash_destination).await.is_err());

        let size_destination = temp.path().join("wrong-body-size.zip");
        assert!(matches!(
            compute_adapter
                .fetch_dataset(&size_reference, &size_destination)
                .await,
            Err(P2pDatasetError::SizeMismatch)
        ));
        assert!(fs::metadata(&size_destination).await.is_err());
        assert_no_partial_files(&temp).await;

        server.await.unwrap();
        robot.shutdown().await.unwrap();
        compute.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn interrupted_transfer_reconnects_from_zero_and_completes_atomically() {
        let domain_id = Uuid::new_v4();
        let bytes = zip_like_bytes(2 * TRANSFER_BUFFER_BYTES + 33);
        let sha256 = hex::encode(Sha256::digest(&bytes));

        let robot = listening_node();
        let robot_address = listen_address(&robot).await;
        let robot_credentials = P2pCredentialStore::new(robot.clone());
        install_current_token(
            &robot_credentials,
            &robot,
            PeerRole::Robot,
            domain_id,
            unix_time(),
        )
        .await;
        let protocol = ApplicationProtocol::new(DATASET_PROTOCOL).unwrap();
        let mut incoming = robot
            .accept(
                protocol,
                SessionRequirements::new(domain_id.to_string(), PeerRole::Compute).unwrap(),
            )
            .unwrap();

        let compute = unbound_node();
        let compute_credentials = P2pCredentialStore::new(compute.clone());
        install_current_token(
            &compute_credentials,
            &compute,
            PeerRole::Compute,
            domain_id,
            unix_time(),
        )
        .await;
        let compute_adapter = P2pDatasetAdapter::new(compute.clone(), compute_credentials, vec![]);
        let reference = P2pDatasetReference {
            schema: P2P_DATASET_SCHEMA.into(),
            dataset_id: "retry-dataset".into(),
            domain_id,
            name: "retry.zip".into(),
            peer_id: robot.peer_id().to_string(),
            multiaddrs: vec![robot_address.to_string()],
            size_bytes: bytes.len() as u64,
            sha256: sha256.clone(),
            available_until: Utc::now() + chrono::Duration::minutes(10),
        };
        let server_bytes = bytes.clone();
        let server = tokio::spawn(async move {
            for attempt in 0..FETCH_ATTEMPTS {
                let mut stream = incoming.accept().await.unwrap().unwrap();
                let request: DatasetRequest = read_json_frame(&mut stream, MAX_REQUEST_BYTES)
                    .await
                    .unwrap();
                assert_eq!(request.version, DATASET_REQUEST_VERSION);
                assert_eq!(request.dataset_id, "retry-dataset");
                write_json_frame(
                    &mut stream,
                    &DatasetResponseHeader {
                        dataset_id: request.dataset_id,
                        size_bytes: server_bytes.len() as u64,
                        sha256: sha256.clone(),
                    },
                    MAX_RESPONSE_HEADER_BYTES,
                )
                .await
                .unwrap();
                let body = if attempt == 0 {
                    &server_bytes[..server_bytes.len() / 2]
                } else {
                    server_bytes.as_slice()
                };
                stream.write_all(body).await.unwrap();
                stream.close().await.unwrap();
            }
        });

        let temp = TempDir::new().unwrap();
        let destination = temp.path().join("retry.zip");
        tokio::time::timeout(
            Duration::from_secs(10),
            compute_adapter.fetch_dataset(&reference, &destination),
        )
        .await
        .expect("retry transfer timed out")
        .unwrap();
        server.await.unwrap();
        assert_eq!(fs::read(&destination).await.unwrap(), bytes);
        assert_no_partial_files(&temp).await;

        robot.shutdown().await.unwrap();
        compute.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn missing_and_expired_compute_credentials_fail_before_dial() {
        let domain_id = Uuid::new_v4();
        let compute = unbound_node();
        let credentials = P2pCredentialStore::new(compute.clone());
        let adapter = P2pDatasetAdapter::new(compute.clone(), credentials.clone(), vec![]);
        let reference = unreachable_reference(domain_id);
        let temp = TempDir::new().unwrap();

        assert!(matches!(
            adapter
                .fetch_dataset(&reference, &temp.path().join("missing.zip"))
                .await,
            Err(P2pDatasetError::Credential(DdsP2pError::MissingCredential))
        ));

        let issued_at = unix_time() - P2P_TOKEN_TTL.as_secs() + 2;
        install_current_token(
            &credentials,
            &compute,
            PeerRole::Compute,
            domain_id,
            issued_at,
        )
        .await;
        tokio::time::sleep(Duration::from_secs(3)).await;
        assert!(matches!(
            adapter
                .fetch_dataset(&reference, &temp.path().join("expired.zip"))
                .await,
            Err(P2pDatasetError::Credential(DdsP2pError::ExpiredCredential))
        ));

        compute.shutdown().await.unwrap();
    }

    fn listening_node() -> Node {
        Node::start(
            Identity::generate(),
            verifier(),
            ["/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap()],
        )
        .unwrap()
    }

    fn unbound_node() -> Node {
        Node::start(
            Identity::generate(),
            verifier(),
            std::iter::empty::<Multiaddr>(),
        )
        .unwrap()
    }

    async fn listen_address(node: &Node) -> Multiaddr {
        tokio::time::timeout(Duration::from_secs(5), node.first_listen_address())
            .await
            .expect("listener did not start")
            .unwrap()
    }

    async fn install_current_token(
        credentials: &P2pCredentialStore,
        node: &Node,
        role: PeerRole,
        domain_id: Uuid,
        issued_at: u64,
    ) {
        let expires_at_unix = issued_at + P2P_TOKEN_TTL.as_secs();
        let claims = P2PAccessClaims {
            token_type: P2P_TOKEN_TYPE.into(),
            iss: P2P_TOKEN_ISSUER.into(),
            aud: vec![P2P_TOKEN_AUDIENCE.into()],
            sub: Uuid::new_v4().to_string(),
            peer_type: role,
            peer_id: node.peer_id().to_string(),
            domain_ids: vec![domain_id.to_string()],
            scopes: vec![P2P_TOKEN_SCOPE.into()],
            iat: issued_at,
            exp: expires_at_unix,
        };
        let token = encode(
            &Header::new(Algorithm::ES256),
            &claims,
            &EncodingKey::from_ec_pem(TEST_DDS_PRIVATE_KEY).unwrap(),
        )
        .unwrap();
        credentials
            .install(
                token,
                DateTime::from_timestamp(expires_at_unix as i64, 0).unwrap(),
            )
            .await
            .unwrap();
    }

    fn verifier() -> DdsTokenVerifier {
        DdsTokenVerifier::from_es256_pem(TEST_DDS_PUBLIC_KEY).unwrap()
    }

    fn registration(dataset_id: &str, domain_id: Uuid, path: PathBuf) -> P2pDatasetRegistration {
        P2pDatasetRegistration {
            dataset_id: dataset_id.into(),
            domain_id,
            name: format!("{dataset_id}.zip"),
            path,
            available_until: Utc::now() + chrono::Duration::minutes(10),
        }
    }

    fn unreachable_reference(domain_id: Uuid) -> P2pDatasetReference {
        P2pDatasetReference {
            schema: P2P_DATASET_SCHEMA.into(),
            dataset_id: "unreachable".into(),
            domain_id,
            name: "unreachable.zip".into(),
            peer_id: Identity::generate().peer_id().to_string(),
            multiaddrs: vec!["/ip4/127.0.0.1/tcp/9".into()],
            size_bytes: 1,
            sha256: "00".repeat(32),
            available_until: Utc::now() + chrono::Duration::minutes(10),
        }
    }

    fn zip_like_bytes(length: usize) -> Vec<u8> {
        let mut bytes = vec![0_u8; length.max(4)];
        bytes[..4].copy_from_slice(b"PK\x03\x04");
        for (index, byte) in bytes[4..].iter_mut().enumerate() {
            *byte = (index % 251) as u8;
        }
        bytes
    }

    async fn assert_no_partial_files(temp: &TempDir) {
        let mut entries = fs::read_dir(temp.path()).await.unwrap();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            assert!(
                !entry.file_name().to_string_lossy().ends_with(".part"),
                "partial transfer file was not cleaned up"
            );
        }
    }

    fn unix_time() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}
