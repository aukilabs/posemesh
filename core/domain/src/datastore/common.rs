
use crate::{auth::AuthError, protobuf::domain_data::{self, Data}};
use async_trait::async_trait;
use networking::libp2p::NetworkError;
use uuid::Uuid;

use futures::channel::{mpsc::{Receiver, Sender}, oneshot::Canceled};
use sha2::{Digest, Sha256 as Sha256Hasher};

pub type Reader<T> = Receiver<Result<T, DomainError>>;
pub type Writer<T> = Sender<Result<T, DomainError>>;

pub type DataWriter = Writer<Data>;
pub type DataReader = Reader<Data>;

pub const CHUNK_SIZE: usize = 7 * 1024; // webrtc allows 8192 = 8KB the most

// Define a custom error type
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Not found")]
    NotFound(String),
    #[error("{0} Cancelled: {1}")]
    Cancelled(String, Canceled),
    #[error("IO error: {0}")]
    Io(#[from]std::io::Error),
    #[cfg(all(feature="fs", not(target_family="wasm")))]
    #[error("Postgres error: {0}")]
    #[cfg(all(feature="fs", not(target_family="wasm")))]
    PostgresError(#[from] tokio_postgres::Error),
    #[error("Internal error: {0}")]
    InternalError(Box<dyn std::error::Error + Send + Sync>),
    #[error("Invalid: {0} {1} {2}")]
    Invalid(String, String, String),
    #[error("Size mismatch: expected {0}, got {1}")]
    SizeMismatch(usize, usize),
    #[error("Network error: {0}")]
    NetworkError(#[from] NetworkError),
    #[error("Protobuf error: {0}")]
    ProtobufError(#[from] quick_protobuf::Error),
    #[error("Auth error: {0}")]
    AuthError(#[from] AuthError),
}

#[async_trait]
pub trait DomainData: Send + Sync {
    async fn next_chunk(&mut self, chunk: &[u8], more: bool) -> Result<String, DomainError>;
}

#[async_trait]
pub trait ReliableDataProducer: Send + Sync {
    async fn push(&mut self, metadata: &domain_data::UpsertMetadata) -> Result<Box<dyn DomainData>, DomainError>;
    async fn is_completed(&self) -> bool;
    async fn close(&mut self) -> ();
}


#[async_trait]
pub trait Datastore: Send + Sync + Clone {
    async fn load(self: &mut Self, domain_id: String, query: domain_data::Query, keep_alive: bool) -> Result<DataReader, DomainError>;
    async fn upsert(self: &mut Self, domain_id: String) -> Result<Box<dyn ReliableDataProducer>, DomainError>;
}

pub fn data_id_generator() -> String {
    Uuid::new_v4().to_string()
}

pub fn hash_chunk(chunk: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256Hasher::new();
    hasher.update(chunk);
    hasher.finalize().into()
}
