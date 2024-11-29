use libp2p::{PeerId, StreamProtocol};
use std::error::Error;
use futures::{channel::{mpsc, oneshot}, SinkExt};
use std::str::FromStr;

#[derive(Clone)]
pub struct Client {
    sender: mpsc::Sender<Command>,
}

pub fn new_client(sender: mpsc::Sender<Command>) -> Client {
    return Client { sender };
}

impl Client {
    pub async fn send(&mut self, message: Vec<u8>, peer_id: String, protocol: String) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (sender, receiver) = oneshot::channel::<Result<(), Box<dyn Error + Send + Sync>>>();
        let peer_id = PeerId::from_str(&peer_id).map_err(|e| Box::new(e))?;
        let pro = StreamProtocol::try_from_owned(protocol).map_err(|e| Box::new(e))?; 
        self.sender
            .send(Command::Send { message, peer_id, protocol: pro, response: sender })
            .await
            .map_err(|e| Box::new(e))?;

        match receiver.await {
            Ok(result) => result,
            Err(e) => Err(Box::new(e)), 
        }
    }

    // TODO: it should return the found peer info if there is
    pub async fn find(&mut self, peer_id: String) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (sender, receiver) = oneshot::channel::<Result<(), Box<dyn Error + Send + Sync>>>();
        let peer_id = PeerId::from_str(&peer_id).map_err(|e| Box::new(e))?;
        println!("Finding peer: {:?}", peer_id);
        self.sender
            .send(Command::Find { peer_id, response: sender })
            .await
            .map_err(|e| Box::new(e))?;
        
        if let Ok(Err(e)) = receiver.await {
            return Err(e);
        }
        Ok(())
    }

    pub async fn set_stream_handler(&mut self, protocol: String) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (sender, receiver) = oneshot::channel::<Result<(), Box<dyn Error + Send + Sync>>>();
        let pro = StreamProtocol::try_from_owned(protocol).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
        self.sender
            .send(Command::SetStreamHandler { protocol: pro, sender })
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
        response: oneshot::Sender<Result<(), Box<dyn Error + Send + Sync>>>,
    },
    Find {
        peer_id: PeerId,
        response: oneshot::Sender<Result<(), Box<dyn Error + Send + Sync>>>,
    },
    SetStreamHandler {
        protocol: StreamProtocol,
        sender: oneshot::Sender<Result<(), Box<dyn Error + Send + Sync>>>,
    }
}
