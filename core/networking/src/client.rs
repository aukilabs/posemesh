use libp2p::{identity::ParseError, swarm::InvalidProtocol, PeerId, Stream, StreamProtocol};
use libp2p_stream::IncomingStreams;
use utils;
use std::{error::Error, time::Duration};
use futures::{channel::{mpsc, oneshot}, SinkExt};
use std::str::FromStr;
#[cfg(not(target_family = "wasm"))]
use tokio::time::sleep;
#[cfg(target_family = "wasm")]
use utils::sleep;

use crate::libp2p::NetworkError;

async fn retry_send(mut command_sender: mpsc::Sender<Command>, message: Vec<u8>, peer_id: PeerId, protocol: StreamProtocol, timeout_millis: u32, last: bool) -> Result<Stream, NetworkError> {
    let (sender, receiver) = oneshot::channel::<Result<Stream, NetworkError>>();
    command_sender
        .send(Command::Send { message: message.clone(), peer_id: peer_id.clone(), protocol: protocol.clone(), response: sender })
        .await?;

    let result = utils::timeout(Duration::from_millis(timeout_millis as u64), async move {
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

impl Client {
    pub fn new(sender: mpsc::Sender<Command>) -> Self {
        Self { sender }
    }
    
    // timeout is in milliseconds
    pub async fn send(&mut self, message: Vec<u8>, peer_id: String, protocol: String, timeout: u32) -> Result<Stream, NetworkError> {
        let peer_id = PeerId::from_str(&peer_id)?;
        let pro = StreamProtocol::try_from_owned(protocol)?; 
        
        retry_send(self.sender.clone(), message, peer_id, pro, timeout, false).await
    }

    pub async fn set_stream_handler(&mut self, protocol: String) -> Result<IncomingStreams, NetworkError> {
        let (sender, receiver) = oneshot::channel::<Result<IncomingStreams, NetworkError>>();
        let pro = StreamProtocol::try_from_owned(protocol)?;
        self.sender
            .send(Command::SetStreamHandler { protocol: pro, sender })
            .await?;

        match receiver.await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => Err(e), 
            Err(e) => Err(NetworkError::ChannelReceiverError(e)), 
        }
    }

    pub async fn subscribe(&mut self, topic: String) -> Result<(), NetworkError> {
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

    pub async fn publish(&mut self, topic: String, message: Vec<u8>) -> Result<(), NetworkError> {
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

    pub async fn cancel(&mut self) -> Result<(), NetworkError> {
        let (sender, receiver) = oneshot::channel::<()>();
        self.sender
            .send(Command::Cancel { sender })
            .await?;

        receiver.await.map_err(|e| NetworkError::ChannelReceiverError(e))
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
        protocol: StreamProtocol,
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
