use libp2p::{swarm::DialError, PeerId, StreamProtocol};
use std::{error::Error, io};
use futures::{channel::{mpsc::{self}, oneshot}, SinkExt};
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
        println!("Sending message to peer: {:?}", peer_id);
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
        
        match receiver.await {
            Ok(result) => result,
            Err(e) => Err(Box::new(e)),
        }
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
    }
}
