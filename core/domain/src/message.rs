use std::time::Duration;

use futures::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use libp2p::Stream;
use posemesh_networking::{client::Client, libp2p::NetworkError};
use quick_protobuf::{deserialize_from_slice, serialize_into_vec, MessageRead, MessageWrite};

use crate::{auth::AuthError, datastore::common::{DomainError, CHUNK_SIZE}, protobuf::task};

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

pub async fn read_prefix_size_message<M: for<'a> MessageRead<'a>, S: AsyncRead + Unpin>(stream: &mut S) -> Result<M, quick_protobuf::Error> {
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

pub async fn handshake_then_prefixed_content<M: MessageWrite>(peer: Client, domain_id: &str, access_token: &str, receiver: &str, endpoint: &str, content: &M, timeout: u32) -> Result<Stream, DomainError> {
    let mut upload_stream = handshake(peer, domain_id, access_token, receiver, endpoint, timeout).await?;

    let content_buffer = prefix_size_message(content);
    upload_stream.write_all(&content_buffer).await?;
    upload_stream.flush().await?;
    Ok(upload_stream)
}

pub async fn handshake_then_content<M: MessageWrite>(peer: Client, domain_id: &str, access_token: &str, receiver: &str, endpoint: &str, content: &M, timeout: u32) -> Result<Stream, DomainError> {
    let mut upload_stream = handshake(peer, domain_id, access_token, receiver, endpoint, timeout).await?;

    upload_stream.write_all(&serialize_into_vec(content).unwrap()).await?;
    upload_stream.flush().await?;
    Ok(upload_stream)
}

pub async fn handshake_then_vec(peer: Client, domain_id: &str, access_token: &str, receiver: &str, endpoint: &str, vec: Vec<u8>, timeout: u32) -> Result<Stream, DomainError> {
    let mut upload_stream = handshake(peer, domain_id, access_token, receiver, endpoint, timeout).await?;
    upload_stream.write_all(&vec).await?;
    upload_stream.flush().await?;
    Ok(upload_stream)
}

pub async fn handshake(mut peer: Client, domain_id: &str, access_token: &str, receiver: &str, endpoint: &str, timeout: u32) -> Result<Stream, DomainError> {
    let mut upload_stream = peer.send(prefix_size_message(&task::DomainClusterHandshakeRequest{
        access_token: access_token.to_string(),
        domain_id: domain_id.to_string(),
    }), receiver.to_string(), endpoint.to_string(), timeout).await?;

    let response = read_prefix_size_message::<task::DomainClusterHandshakeResponse, _>(&mut upload_stream).await?;
    match response.code {
        task::Code::OK => Ok(upload_stream),
        _ => Err(DomainError::AuthError(AuthError::HandshakeFailed(response.err_msg))),
    }
}

pub async fn request_response_with_handshake<Request: MessageWrite, Response: for<'a> MessageRead<'a>>(peer: Client, domain_id: &str, access_token: &str, receiver: &str, endpoint: &str, request: &Request, timeout: u32) -> Result<Response, DomainError> {
    let mut upload_stream = handshake(peer, domain_id, access_token, receiver, endpoint, timeout).await?;
    let content_buffer = prefix_size_message(request);
    for chunk in content_buffer.chunks(CHUNK_SIZE) {
        upload_stream.write_all(chunk).await?;
    }
    upload_stream.flush().await?;
    let response = read_prefix_size_message::<Response, _>(&mut upload_stream).await?;
    Ok(response)
}

pub async fn request_response<Request: MessageWrite, Response: for<'a> MessageRead<'a> + Send + >(mut peer: Client, receiver: &str, endpoint: &str, request: &Request, timeout_millis: u32) -> Result<Response, DomainError> {
    let mut upload_stream = peer.send(prefix_size_message(request), receiver.to_string(), endpoint.to_string(), timeout_millis).await?;
    
    posemesh_utils::timeout(Duration::from_millis(timeout_millis as u64), async move {
        let response = read_prefix_size_message::<Response, _>(&mut upload_stream).await.expect("Failed to read response");
        Ok(response)
    }).await?
}

pub async fn request_response_raw(mut peer:Client, receiver: &str, endpoint: &str, request: &[u8], timeout_millis: u32) -> Result<Vec<u8>, NetworkError> {
    let mut upload_stream = peer.send(request.to_vec(), receiver.to_string(), endpoint.to_string(), timeout_millis).await?;
    upload_stream.close().await?;
    let mut response = Vec::new();
    upload_stream.read_to_end(&mut response).await?;
    Ok(response)
}
