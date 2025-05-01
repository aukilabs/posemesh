use networking::AsyncStream;
use futures::AsyncWriteExt;
use crate::datastore::common::DomainError;

pub const PUBLIC_KEY_PROTOCOL_V1: &str = "/public-key/v1.0.0";

pub async fn serve_public_key_v1<S: AsyncStream>(mut stream: S, public_key: Vec<u8>) -> Result<(), DomainError> {
    stream.write_all(&public_key).await?;
    stream.close().await?;
    Ok(())
}
