use crate::{ auth::{handshake, AuthError}, datastore::{common::{Datastore, CHUNK_SIZE}, remote::CONSUME_DATA_PROTOCOL_V1}, message::prefix_size_message, protobuf::{domain_data::{Metadata, UpsertMetadata}, task::{ConsumeDataInputV1, Status, Task}}};
use networking::{libp2p::{NetworkError, Networking}, AsyncStream};
use quick_protobuf::{deserialize_from_slice, serialize_into_vec};
use tokio::{self, select};
use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};

use super::common::DomainError;

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

pub async fn store_data_v1<S: AsyncStream, D: Datastore>(mut stream: S, mut c: Networking, mut datastore: D, secret: &Vec<u8>) -> Result<(), CapabilityError> {
    let claim = handshake(&mut stream, secret).await?;
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
        loop {
            // TODO: add timeout so stream wont be idle for too long
            let chunk_size = if data_size - read_size > CHUNK_SIZE { CHUNK_SIZE } else { data_size - read_size };
            
            let mut buffer = vec![0u8; chunk_size];
            stream.read_exact(&mut buffer).await?;

            read_size+=chunk_size;

            if chunk_size < CHUNK_SIZE {
                let hash = data_writer.next_chunk(&buffer, false).await?;
                tracing::info!("Stored data: {}, size: {}, hash: {:?}", metadata.name, metadata.size, hash);
                if metadata.size as usize != read_size {
                    return Err(CapabilityError::DomainError(DomainError::SizeMismatch(metadata.size as usize, read_size)));
                }
                stream.write_all(&prefix_size_message(&metadata)).await?;
                stream.flush().await?;
                break;
            }

            data_writer.next_chunk(&buffer, true).await.map_err(|e| CapabilityError::DomainError(e))?;
            tracing::debug!("Received chunk: {}/{}", read_size, metadata.size);
        }
    }
}

pub async fn serve_data_v1<S: AsyncStream, D: Datastore>(mut stream: S, mut c: Networking, mut datastore: D, secret: &Vec<u8>) -> Result<(), CapabilityError> {
    let header = handshake(&mut stream, secret).await?;
    c.client.subscribe(header.job_id.clone()).await?;
    
    let mut buf = Vec::<u8>::new();
    let res = stream.read_to_end(&mut buf).await;
    if res.is_err() {
        let err = res.err().unwrap();
        if err.kind() == std::io::ErrorKind::UnexpectedEof {
            return Ok(());
        } else {
            return Err(CapabilityError::StreamError(err));
        }
    }
    let input = deserialize_from_slice::<ConsumeDataInputV1>(&buf)?;
    let mut consumer = datastore.load(header.domain_id.clone(), input.query, input.keep_alive).await?;
    loop {
        select! {
            result = consumer.next() => {
                match result {
                    Some(Ok(data)) => {
                        stream.write_all(&prefix_size_message(&data.metadata)).await?;
                        for chunk in data.content.chunks(CHUNK_SIZE) {
                            stream.write_all(chunk).await?;
                        }
                        stream.flush().await?;
                        tracing::info!("Served data: {}, size: {}", data.metadata.name, data.metadata.size);
                    }
                    Some(Err(e)) => {
                        tracing::info!("Error: {:?}", e);
                        return Err(CapabilityError::DomainError(e));
                    }
                    None => break
                }
            }
        }
    }

    if !input.keep_alive {
        let task = Task {
            name: header.task_name.clone(),
            receiver: Some(header.receiver.clone()),
            sender: header.sender.clone(),
            endpoint: CONSUME_DATA_PROTOCOL_V1.to_string(),
            status: Status::DONE,
            access_token: None,
            job_id: header.job_id.clone(),
            output: None,
        };
        let buf = serialize_into_vec(&task)?;
        c.client.publish(header.job_id.clone(), buf).await?;
    }

    Ok(())
}
