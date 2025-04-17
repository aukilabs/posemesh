use libp2p::{PeerId, Stream, StreamProtocol};
use libp2p_stream::IncomingStreams;
use utils;
use std::{error::Error, time::Duration};
use futures::{channel::{mpsc, oneshot}, SinkExt};
use std::str::FromStr;
#[cfg(not(target_family = "wasm"))]
use tokio::time::sleep;
#[cfg(target_family = "wasm")]
use utils::sleep;

async fn retry_send(mut command_sender: mpsc::Sender<Command>, message: Vec<u8>, peer_id: PeerId, protocol: StreamProtocol, timeout: u32, last: bool) -> Result<Stream, Box<dyn Error + Send + Sync>> {
    let (sender, receiver) = oneshot::channel::<Result<Stream, Box<dyn Error + Send + Sync>>>();
    command_sender
        .send(Command::Send { message: message.clone(), peer_id: peer_id.clone(), protocol: protocol.clone(), response: sender })
        .await
        .map_err(|e| Box::new(e))?;

    let result = utils::timeout(Duration::from_millis(timeout as u64), async move {
        match receiver.await {
            Ok(result) => result,
            Err(e) => Err(Box::new(e) as Box<dyn Error + Send + Sync>),
        }
    }).await?;

    match result {
        Ok(s) => Ok(s),
        Err(e) => {
            if let Some(dial_error) = e.downcast_ref::<libp2p::swarm::DialError>() {
                if matches!(dial_error, libp2p::swarm::DialError::NoAddresses) && !last {
                    tracing::warn!("find address the last time: {:?}", dial_error);
                    sleep(Duration::from_millis(500)).await;
                    return Box::pin(retry_send(command_sender, message, peer_id, protocol, timeout, true)).await;
                }
            }
            tracing::error!("send error: {:?}", e);
            return Err(e);
        },
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
    pub async fn send(&mut self, message: Vec<u8>, peer_id: String, protocol: String, timeout: u32) -> Result<Stream, Box<dyn Error + Send + Sync>> {
        let peer_id = PeerId::from_str(&peer_id).map_err(|e| Box::new(e))?;
        let pro = StreamProtocol::try_from_owned(protocol).map_err(|e| Box::new(e))?; 
        
        retry_send(self.sender.clone(), message, peer_id, pro, timeout, false).await
    }

    pub async fn set_stream_handler(&mut self, protocol: String) -> Result<IncomingStreams, Box<dyn Error + Send + Sync>> {
        let (sender, receiver) = oneshot::channel::<Result<IncomingStreams, Box<dyn Error + Send + Sync>>>();
        let pro = StreamProtocol::try_from_owned(protocol).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
        self.sender
            .send(Command::SetStreamHandler { protocol: pro, sender })
            .await
            .map_err(|e| {
                tracing::error!("set stream handler error: {:?}", e);
                Box::new(e) as Box<dyn Error + Send + Sync>
            })?;

        match receiver.await {
            Ok(result) => result,
            Err(e) => Err(Box::new(e)), 
        }
    }

    pub async fn subscribe(&mut self, topic: String) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (resp, req) = oneshot::channel::<Box<dyn Error + Send + Sync>>();
        self.sender
            .send(Command::Subscribe { topic, resp })
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        if let Ok(e) = req.await {
            return Err(e);
        }

        Ok(())
    }

    pub async fn publish(&mut self, topic: String, message: Vec<u8>) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (sender, receiver) = oneshot::channel::<Result<(), Box<dyn Error + Send + Sync>>>();
        self.sender
            .send(Command::Publish { topic, message, sender })
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        if let Ok(Err(e)) = receiver.await {
            return Err(e);
        }
        Ok(())
    }

    pub async fn cancel(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (sender, receiver) = oneshot::channel::<()>();
        self.sender
            .send(Command::Cancel { sender })
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        receiver.await.map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}

#[derive(Debug)]
pub enum Command {
    Send {
        message: Vec<u8>,
        peer_id: PeerId,
        protocol: StreamProtocol,
        response: oneshot::Sender<Result<Stream, Box<dyn Error + Send + Sync>>>,
    },
    SetStreamHandler {
        protocol: StreamProtocol,
        sender: oneshot::Sender<Result<IncomingStreams, Box<dyn Error + Send + Sync>>>,
    },
    Publish {
        topic: String,
        message: Vec<u8>,
        sender: oneshot::Sender<Result<(), Box<dyn Error + Send + Sync>>>,
    },
    Subscribe {
        topic: String,
        resp: oneshot::Sender<Box<dyn Error + Send + Sync>>,
    },
    Cancel {
        sender: oneshot::Sender<()>,
    }
}
