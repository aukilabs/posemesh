use std::error::Error;

use futures::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use libp2p::Stream;
use networking::client::Client;
use quick_protobuf::{deserialize_from_slice, serialize_into_vec, MessageRead, MessageWrite};

use crate::protobuf::task;

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
    let mut message_buffer = vec![0u8; size as usize];
    stream.read_exact(&mut message_buffer).await?;
    deserialize_from_slice(&message_buffer)
}

pub async fn handshake_then_prefixed_content<M: MessageWrite>(peer: Client, access_token: &str, receiver: &str, endpoint: &str, content: &M, timeout: u32) -> Result<Stream, Box<dyn Error + Send + Sync>> {
    let mut upload_stream = handshake(peer, access_token, receiver, endpoint, timeout).await?;

    let content_buffer = prefix_size_message(content);
    upload_stream.write_all(&content_buffer).await?;
    upload_stream.flush().await?;
    Ok(upload_stream)
}

pub async fn handshake_then_content<M: MessageWrite>(peer: Client, access_token: &str, receiver: &str, endpoint: &str, content: &M, timeout: u32) -> Result<Stream, Box<dyn Error + Send + Sync>> {
    let mut upload_stream = handshake(peer, access_token, receiver, endpoint, timeout).await?;

    upload_stream.write_all(&serialize_into_vec(content).unwrap()).await?;
    upload_stream.flush().await?;
    Ok(upload_stream)
}

pub async fn handshake_then_vec(peer: Client, access_token: &str, receiver: &str, endpoint: &str, vec: Vec<u8>, timeout: u32) -> Result<Stream, Box<dyn Error + Send + Sync>> {
    let mut upload_stream = handshake(peer, access_token, receiver, endpoint, timeout).await?;
    upload_stream.write_all(&vec).await?;
    upload_stream.flush().await?;
    Ok(upload_stream)
}

pub async fn handshake(mut peer: Client, access_token: &str, receiver: &str, endpoint: &str, timeout: u32) -> Result<Stream, Box<dyn Error + Send + Sync>> {
    let upload_stream = peer.send(prefix_size_message(&task::DomainClusterHandshake{
        access_token: access_token.to_string(),
    }), receiver.to_string(), endpoint.to_string(), timeout).await?;

    Ok(upload_stream)
}
