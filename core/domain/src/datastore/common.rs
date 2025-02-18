
use std::error::Error;

use crate::protobuf::domain_data::{self, Data};
use async_trait::async_trait;
use futures::channel::mpsc::{Receiver, Sender};

pub type DataWriter = Sender<Result<Data, DomainError>>;
pub type DataReader = Receiver<Result<Data, DomainError>>;

// Define a custom error type
#[derive(Debug)]
pub enum DomainError {
    NotFound,
    Interrupted,
    Cancelled,
}

impl Error for DomainError {}
impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DomainError::NotFound => write!(f, "Not found"),
            DomainError::Interrupted => write!(f, "Interrupted"),
            DomainError::Cancelled => write!(f, "Cancelled"),
        }
    }
}


#[async_trait]
pub trait Datastore: Send + Sync {
    async fn consume(self: &mut Self, domain_id: String, query: domain_data::Query) -> DataReader;
    async fn produce(self: &mut Self, domain_id: String) -> DataWriter;
}
