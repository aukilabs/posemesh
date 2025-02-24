
use crate::{protobuf::domain_data, remote::{DataStream, DomainError}};
use async_trait::async_trait;

#[async_trait]
pub trait Datastore: Send + Sync {
    async fn find(self: &mut Self, domain_id: String, query: domain_data::Query) -> DataStream;
}
