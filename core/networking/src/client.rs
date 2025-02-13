use libp2p::{PeerId, Stream, StreamProtocol};
use core::time;
use std::error::Error;
use futures::{channel::{mpsc::{self, Receiver}, oneshot}, future::{select, Either::{Left, Right}}, SinkExt};
use std::str::FromStr;
use crate::event;
#[cfg(target_family = "wasm")]
use gloo_timers::future::TimeoutFuture;

#[derive(Clone)]
pub struct Client {
    sender: mpsc::Sender<Command>,
}

pub fn new_client(sender: mpsc::Sender<Command>) -> Client {
    return Client { sender };
}

impl Client {
    // timeout is in milliseconds
    pub async fn send(&mut self, message: Vec<u8>, peer_id: String, protocol: String, timeout: u32) -> Result<Stream, Box<dyn Error + Send + Sync>> {
        let (sender, receiver) = oneshot::channel::<Result<Stream, Box<dyn Error + Send + Sync>>>();
        let peer_id = PeerId::from_str(&peer_id).map_err(|e| Box::new(e))?;
        let pro = StreamProtocol::try_from_owned(protocol).map_err(|e| Box::new(e))?; 
        self.sender
            .send(Command::Send { message, peer_id, protocol: pro, response: sender })
            .await
            .map_err(|e| Box::new(e))?;

        if timeout == 0 {
            return match receiver.await {
                Ok(result) => result,
                Err(e) => Err(Box::new(e)), 
            }
        }

        #[cfg(target_family = "wasm")]
        match select(TimeoutFuture::new(timeout), receiver).await {
            Left(_) => {
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::TimedOut, "Timed out")));
            }
            Right((result, timer)) => {
                drop(timer);
                return match result {
                    Ok(result) => result,
                    Err(e) => Err(Box::new(e)), 
                }
            }
        };

        #[cfg(not(target_family = "wasm"))]
        match tokio::time::timeout(time::Duration::from_millis(timeout as u64), receiver).await {
            Err(_) => {
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::TimedOut, "Timed out")));
            }
            Ok(result) => {
                return match result {
                    Ok(result) => result,
                    Err(e) => Err(Box::new(e)), 
                }
            }
        };
    }

    pub async fn set_stream_handler(&mut self, protocol: String) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (sender, receiver) = oneshot::channel::<Result<(), Box<dyn Error + Send + Sync>>>();
        let pro = StreamProtocol::try_from_owned(protocol).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
        self.sender
            .send(Command::SetStreamHandler { protocol: pro, sender })
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

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
        sender: oneshot::Sender<Result<(), Box<dyn Error + Send + Sync>>>,
    },
    Publish {
        topic: String,
        message: Vec<u8>,
        sender: oneshot::Sender<Result<(), Box<dyn Error + Send + Sync>>>,
    },
    Subscribe {
        topic: String,
        resp: oneshot::Sender<Box<dyn Error + Send + Sync>>,
    }
}
