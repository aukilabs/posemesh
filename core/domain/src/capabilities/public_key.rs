use async_trait::async_trait;
use networking::AsyncStream;
use futures::AsyncWriteExt;
use futures::AsyncReadExt;
use crate::auth::AuthError;

pub const PUBLIC_KEY_PROTOCOL_V1: &str = "/public-key/v1";

#[async_trait]
pub trait PublicKeyStorage {
    async fn get_by_domain_id(&self, domain_id: String) -> Result<Vec<u8>, AuthError>;
}

pub async fn serve_public_key_v1<S: AsyncStream, Store: PublicKeyStorage>(mut stream: S, storage: Store) -> Result<(), AuthError> {
    let mut domain_id = String::new();
    stream.read_to_string(&mut domain_id).await?;
    let public_key = storage.get_by_domain_id(domain_id).await?;
    stream.write_all(&public_key).await?;
    stream.close().await?;
    Ok(())
}
