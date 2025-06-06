use async_trait::async_trait;
use libp2p::{PeerId, Stream, StreamProtocol};
use libp2p_stream::IncomingStreams;
use std::time::Duration;
use futures::{channel::{mpsc, oneshot}, SinkExt};
use std::str::FromStr;
#[cfg(not(target_family = "wasm"))]
use tokio::time::sleep;
#[cfg(target_family = "wasm")]
use posemesh_utils::sleep;
#[cfg(test)]
use mockall::automock;

use crate::libp2p::{NetworkError};

async fn retry_send(mut command_sender: mpsc::Sender<Command>, message: Vec<u8>, peer_id: PeerId, protocol: StreamProtocol, timeout_millis: u32, last: bool) -> Result<Stream, NetworkError> {
    let (sender, receiver) = oneshot::channel::<Result<Stream, NetworkError>>();
    command_sender
        .send(Command::Send { message: message.clone(), peer_id: peer_id.clone(), protocol: protocol.clone(), response: sender })
        .await?;

    let result = posemesh_utils::timeout(Duration::from_millis(timeout_millis as u64), async move {
        match receiver.await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(NetworkError::ChannelReceiverError(e)),
        }
    }).await?;

    match result {
        Ok(s) => Ok(s),
        Err(e) => {
            match e {
                NetworkError::DialError(e) => {
                    if !last {
                        tracing::warn!("find address the last time: {:?}", e);
                        sleep(Duration::from_millis(500)).await;
                        return Box::pin(retry_send(command_sender, message, peer_id, protocol, timeout_millis, true)).await;
                    }
                    Err(NetworkError::DialError(e))
                }
                _ => {
                    tracing::error!("send error: {:?}", e);
                    return Err(e);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Client {
    sender: mpsc::Sender<Command>,
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait TClient {
    async fn publish(&mut self, topic: String, message: Vec<u8>) -> Result<(), NetworkError>;
    async fn subscribe(&mut self, topic: String) -> Result<(), NetworkError>;
}

#[async_trait]
impl TClient for Client {
    async fn publish(&mut self, topic: String, message: Vec<u8>) -> Result<(), NetworkError> {
        let (sender, receiver) = oneshot::channel::<Result<(), NetworkError>>();
        self.sender
            .send(Command::Publish { topic, message, sender })
            .await?;

        match receiver.await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(NetworkError::ChannelReceiverError(e)),
        }
    }

    async fn subscribe(&mut self, topic: String) -> Result<(), NetworkError> {
        let (resp, req) = oneshot::channel::<Result<(), NetworkError>>();
        self.sender
            .send(Command::Subscribe { topic, resp })
            .await?;

        match req.await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(NetworkError::ChannelReceiverError(e)),
        }
    }
}

impl Client {
    pub fn new(sender: mpsc::Sender<Command>) -> Self {
        Self { sender }
    }

    pub async fn set_stream_handler(&mut self, endpoint: &str) -> Result<IncomingStreams, NetworkError> {
        let (sender, receiver) = oneshot::channel::<Result<IncomingStreams, NetworkError>>();
        self.sender
            .send(Command::SetStreamHandler { endpoint: endpoint.to_string(), sender })
            .await?;

        match receiver.await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => Err(e), 
            Err(e) => Err(NetworkError::ChannelReceiverError(e)), 
        }
    }

    pub async fn cancel(&mut self) -> Result<(), NetworkError> {
        let (sender, receiver) = oneshot::channel::<()>();
        self.sender
            .send(Command::Cancel { sender })
            .await?;

        receiver.await.map_err(|e| NetworkError::ChannelReceiverError(e))
    }
    
    // timeout is in milliseconds
    pub async fn send(&mut self, message: Vec<u8>, peer_id: String, protocol: String, timeout: u32) -> Result<Stream, NetworkError> {
        let peer_id = PeerId::from_str(&peer_id)?;
        let pro = StreamProtocol::try_from_owned(protocol)?; 
        
        retry_send(self.sender.clone(), message, peer_id, pro, timeout, false).await
    }
}

#[derive(Debug)]
pub enum Command {
    Send {
        message: Vec<u8>,
        peer_id: PeerId,
        protocol: StreamProtocol,
        response: oneshot::Sender<Result<Stream, NetworkError>>,
    },
    SetStreamHandler {
        endpoint: String,
        sender: oneshot::Sender<Result<IncomingStreams, NetworkError>>,
    },
    Publish {
        topic: String,
        message: Vec<u8>,
        sender: oneshot::Sender<Result<(), NetworkError>>,
    },
    Subscribe {
        topic: String,
        resp: oneshot::Sender<Result<(), NetworkError>>,
    },
    Cancel {
        sender: oneshot::Sender<()>,
    }
}
