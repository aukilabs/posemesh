use crate::{auth::{handshake, AuthError}, datastore::common::{Datastore, DomainError}, message::{prefix_size_message, read_prefix_size_message}, protobuf::{domain_data::{Metadata, UpsertMetadata}, task::{ConsumeDataInputV1, Status, Task, UnsubscribeDataQueryV1}}};
use posemesh_networking::{libp2p::{NetworkError, Networking}, AsyncStream};
use quick_protobuf::{deserialize_from_slice, serialize_into_vec};
use futures::{channel::oneshot, select, AsyncReadExt, AsyncWriteExt, FutureExt};
use posemesh_networking::client::TClient;
use super::public_key::PublicKeyStorage;
#[cfg(not(target_family = "wasm"))]
use tokio::spawn;
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

#[derive(Debug, thiserror::Error)]
pub enum CapabilityError {
    #[error("Handshake failed: {0}")]
    HandshakeFailed(#[from] AuthError),
    #[error("Stream error: {0}")]
    StreamError(#[from] std::io::Error),
    #[error("Protobuf error: {0}")]
    ProtobufError(#[from] quick_protobuf::Error),
    #[error("Domain error: {0}")]
    DomainError(#[from] DomainError),
    #[error("Network error: {0}")]
    NetworkError(#[from] NetworkError),
}

pub const CONSUME_DATA_PROTOCOL_V1: &str = "/load/v1";
pub const PRODUCE_DATA_PROTOCOL_V1: &str = "/store/v1";

pub async fn store_data_v1<S: AsyncStream, D: Datastore, P: PublicKeyStorage>(mut stream: S, mut c: Networking, mut datastore: D, key_loader: P) -> Result<(), CapabilityError> {
    let claim = handshake(&mut stream, key_loader).await?;
    let job_id = claim.job_id.clone();
    c.client.subscribe(job_id.clone()).await?;
    let domain_id = claim.domain_id.clone();

    let mut producer = datastore.upsert(domain_id.clone()).await?;

    loop {
        let mut length_buf = [0u8; 4];
        let res = stream.read_exact(&mut length_buf).await;
        if res.is_err() {
            let err = res.err().unwrap();
            if err.kind() == std::io::ErrorKind::UnexpectedEof {
                let task = Task {
                    name: claim.task_name.clone(),
                    receiver: Some(claim.receiver.clone()),
                    sender: claim.sender.clone(),
                    endpoint: PRODUCE_DATA_PROTOCOL_V1.to_string(),
                    status: Status::DONE,
                    access_token: None,
                    job_id: job_id.clone(),
                    output: None,
                };
                let buf = serialize_into_vec(&task)?;
                c.client.publish(job_id.clone(), buf).await.map_err(|e| CapabilityError::NetworkError(e))?;
                return Ok(());
            } else {
                return Err(CapabilityError::StreamError(err));
            }
        }
        let length = u32::from_be_bytes(length_buf) as usize;

        // Read the data in chunks
        let mut buffer = vec![0u8; length];
        stream.read_exact(&mut buffer).await?;
        let metadata = deserialize_from_slice::<UpsertMetadata>(&buffer)?;
        tracing::debug!("Received buffer: {:?}", metadata);

        let mut read_size: usize = 0;
        let data_size = metadata.size as usize;
        let mut data_writer = producer.push(&metadata).await?;
        let default_chunk_size = 10 * 1024 * 1024; // 10MB
        loop {
            // TODO: add timeout so stream wont be idle for too long
            let chunk_size = if data_size - read_size > default_chunk_size { default_chunk_size } else { data_size - read_size };
            tracing::debug!("chunk_size: {}", chunk_size);
            let mut buffer = vec![0u8; chunk_size];
            stream.read_exact(&mut buffer).await?;

            read_size+=chunk_size;

            if chunk_size < default_chunk_size {
                let hash = data_writer.next_chunk(&buffer, false).await?;
                tracing::info!("Stored data: {}, size: {}, hash: {:?}", metadata.name, metadata.size, hash);
                if metadata.size as usize != read_size {
                    return Err(CapabilityError::DomainError(DomainError::SizeMismatch(metadata.size as usize, read_size)));
                }
                let metadata = Metadata {
                    hash: Some(hash),
                    name: metadata.name,
                    data_type: metadata.data_type,
                    size: metadata.size,
                    id: metadata.id,
                    properties: metadata.properties,
                };
                stream.write_all(&prefix_size_message(&metadata)).await?;
                stream.flush().await?;
                break;
            }

            data_writer.next_chunk(&buffer, true).await?;
            tracing::debug!("Received chunk: {}/{}", read_size, metadata.size);
        }
    }
}

pub async fn serve_data_v1<S: AsyncStream + 'static, D: Datastore, P: PublicKeyStorage>(mut stream: S, mut c: Networking, mut datastore: D, key_loader: P) -> Result<(), CapabilityError> {    
    let header = handshake(&mut stream, key_loader).await?;
    let (mut read, write) = stream.split();
    c.client.subscribe(header.job_id.clone()).await?;
    let input = read_prefix_size_message::<ConsumeDataInputV1, _>(&mut read).await?;
    let (cancel_tx, mut cancel_rx) = oneshot::channel();

    let mut consumer = datastore.load::<_>(header.domain_id.clone(), input.query, input.keep_alive, write).await?;
    spawn(async move {
        let res = read_prefix_size_message::<UnsubscribeDataQueryV1, _>(&mut read).await;
        let _ = cancel_tx.send(res);
    });

    select! {
        res = consumer.wait_for_done().fuse() => res.map_err(|e| CapabilityError::DomainError(e)),
        res = cancel_rx => {
            consumer.close().await;
            match res {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(e)) => Err(CapabilityError::ProtobufError(e)),
                Err(e) => Err(CapabilityError::DomainError(DomainError::Cancelled("client cancelled".to_string(), e))),
            }
        }
    }
}
