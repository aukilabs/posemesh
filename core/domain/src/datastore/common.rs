
use std::error::Error;

use crate::protobuf::domain_data::{self, Data};
use async_trait::async_trait;
use uuid::Uuid;

use futures::{channel::mpsc::{self, Receiver, Sender}, lock::Mutex, SinkExt, StreamExt};
use sha2::{Digest, Sha256 as Sha256Hasher};

pub type Reader<T> = Receiver<Result<T, DomainError>>;
pub type Writer<T> = Sender<Result<T, DomainError>>;

pub type DataWriter = Writer<Data>;
pub type DataReader = Reader<Data>;

// Define a custom error type
#[derive(Debug)]
pub enum DomainError {
    NotFound,
    Interrupted,
    Cancelled(String),
    IoError(std::io::Error),
    PostgresError(tokio_postgres::Error),
    InternalError(Box<dyn Error + Send + Sync>),
}

impl Error for DomainError {}
impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DomainError::NotFound => write!(f, "Not found"),
            DomainError::Interrupted => write!(f, "Interrupted"),
            DomainError::Cancelled(s) => write!(f, "Cancelled: {}", s),
            DomainError::IoError(e) => write!(f, "IO error: {}", e),
            DomainError::PostgresError(e) => write!(f, "Postgres error: {}", e),
            DomainError::InternalError(e) => write!(f, "Internal error: {}", e),
        }
    }
}

#[async_trait]
pub trait DomainData: Send + Sync {
    async fn push_chunk(&mut self, chunk: &[u8], more: bool) -> Result<String, DomainError>;
}

#[async_trait]
pub trait ReliableDataProducer: Send + Sync {
    async fn push(&mut self, metadata: &domain_data::Metadata) -> Result<Box<dyn DomainData>, DomainError>;
    async fn is_completed(&self) -> bool;
    async fn close(&mut self) -> ();
}


#[async_trait]
pub trait Datastore: Send + Sync + Clone {
    async fn load(self: &mut Self, domain_id: String, query: domain_data::Query, keep_alive: bool) -> DataReader;
    async fn upsert(self: &mut Self, domain_id: String) -> Box<dyn ReliableDataProducer>;
}

pub fn data_id_generator() -> String {
    Uuid::new_v4().to_string()
}

pub fn hash_chunk(chunk: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256Hasher::new();
    hasher.update(chunk);
    hasher.finalize().into()
}
