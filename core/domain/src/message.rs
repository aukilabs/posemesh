use futures::{AsyncRead, AsyncReadExt};
use quick_protobuf::{deserialize_from_slice, serialize_into_vec, MessageRead, MessageWrite};

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
