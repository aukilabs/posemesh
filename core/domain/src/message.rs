use futures::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use libp2p::Stream;
use networking::client::Client;
use quick_protobuf::{deserialize_from_slice, serialize_into_vec, MessageRead, MessageWrite};

use crate::{datastore::common::{DomainError, CHUNK_SIZE}, protobuf::task};

pub fn prefix_size_message<M: MessageWrite>(message: &M) -> Vec<u8>
{
    let mut message_buffer = serialize_into_vec(message).expect("Failed to serialize message");
    let size = message_buffer.len() as u32;
    let size_buffer = size.to_be_bytes();
    let mut result = Vec::with_capacity(4 + message_buffer.len());
    result.extend_from_slice(&size_buffer);
    result.append(&mut message_buffer);
    result
}

pub async fn read_prefix_size_message<M: for<'a> MessageRead<'a>>(mut stream: impl AsyncRead + Unpin) -> Result<M, quick_protobuf::Error> {
    let mut size_buffer = [0u8; 4];
    stream.read_exact(&mut size_buffer).await?;
    let size = u32::from_be_bytes(size_buffer);
    let mut read: usize = 0;
    let mut message_buffer = vec![0u8; size as usize];
    while read < size as usize {
        let chunk_size = std::cmp::min(CHUNK_SIZE, size as usize - read);
        stream.read_exact(&mut message_buffer[read..read + chunk_size]).await?;
        read += chunk_size;
    }
    
    deserialize_from_slice(&message_buffer)
}

pub async fn handshake_then_prefixed_content<M: MessageWrite>(peer: Client, access_token: &str, receiver: &str, endpoint: &str, content: &M, timeout: u32) -> Result<Stream, DomainError> {
    let mut upload_stream = handshake(peer, access_token, receiver, endpoint, timeout).await?;

    let content_buffer = prefix_size_message(content);
    upload_stream.write_all(&content_buffer).await?;
    upload_stream.flush().await?;
    Ok(upload_stream)
}

pub async fn handshake_then_content<M: MessageWrite>(peer: Client, access_token: &str, receiver: &str, endpoint: &str, content: &M, timeout: u32) -> Result<Stream, DomainError> {
    let mut upload_stream = handshake(peer, access_token, receiver, endpoint, timeout).await?;

    upload_stream.write_all(&serialize_into_vec(content).unwrap()).await?;
    upload_stream.flush().await?;
    Ok(upload_stream)
}

pub async fn handshake_then_vec(peer: Client, access_token: &str, receiver: &str, endpoint: &str, vec: Vec<u8>, timeout: u32) -> Result<Stream, DomainError> {
    let mut upload_stream = handshake(peer, access_token, receiver, endpoint, timeout).await?;
    tracing::debug!("Sending vec");
    upload_stream.write_all(&vec).await?;
    tracing::debug!("Flushing");
    upload_stream.flush().await?;
    Ok(upload_stream)
}

pub async fn handshake(mut peer: Client, access_token: &str, receiver: &str, endpoint: &str, timeout: u32) -> Result<Stream, DomainError> {
    tracing::debug!("Sending handshake");
    let upload_stream = peer.send(prefix_size_message(&task::DomainClusterHandshake{
        access_token: access_token.to_string(),
    }), receiver.to_string(), endpoint.to_string(), timeout).await?;
    tracing::debug!("Handshake sent");
    Ok(upload_stream)
}

pub async fn request_response_with_handshake<Request: MessageWrite, Response: for<'a> MessageRead<'a>>(peer: Client, access_token: &str, receiver: &str, endpoint: &str, request: &Request, timeout: u32) -> Result<Response, DomainError> {
    let mut upload_stream = handshake(peer, access_token, receiver, endpoint, timeout).await?;
    let content_buffer = prefix_size_message(request);
    for chunk in content_buffer.chunks(CHUNK_SIZE) {
        upload_stream.write_all(chunk).await?;
    }
    upload_stream.flush().await?;
    let response = read_prefix_size_message::<Response>(&mut upload_stream).await?;
    Ok(response)
}

pub async fn request_response<Request: MessageWrite, Response: for<'a> MessageRead<'a>>(mut peer: Client, receiver: &str, endpoint: &str, request: &Request, timeout_millis: u32) -> Result<Response, DomainError> {
    let mut upload_stream = peer.send(prefix_size_message(request), receiver.to_string(), endpoint.to_string(), timeout_millis).await?;
    let response = read_prefix_size_message::<Response>(&mut upload_stream).await?;
    Ok(response)
}

pub async fn request_response_raw(mut peer:Client, receiver: &str, endpoint: &str, request: &[u8], timeout_millis: u32) -> Result<Vec<u8>, DomainError> {
    let mut upload_stream = peer.send(request.to_vec(), receiver.to_string(), endpoint.to_string(), timeout_millis).await?;
    let mut response = Vec::new();
    upload_stream.read_to_end(&mut response).await?;
    Ok(response)
}
