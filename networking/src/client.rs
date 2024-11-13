use libp2p::{PeerId, StreamProtocol};
use std::error::Error;
use futures::{channel::mpsc::{self}, SinkExt};
use std::str::FromStr;

#[derive(Clone)]
pub struct Client {
    sender: mpsc::Sender<Command>,
}

pub fn new_client(sender: mpsc::Sender<Command>) -> Client {
    return Client { sender };
}

impl Client {
    pub async fn send(&mut self, message: Vec<u8>, peer_id: String, protocol: String) -> Result<(), Box<dyn Error>> {
        let peer_id = PeerId::from_str(&peer_id).map_err(|e| Box::new(e) as Box<dyn Error>)?;
        let pro = StreamProtocol::try_from_owned(protocol).map_err(|e| Box::new(e) as Box<dyn Error>)?;
        return self.sender
            .send(Command::Send { message, peer_id, protocol: pro})
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>);
    }
}

#[derive(Debug)]
pub enum Command {
    Send {
        message: Vec<u8>,
        peer_id: PeerId,
        protocol: StreamProtocol,
    },
}
