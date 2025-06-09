use libp2p::{gossipsub::TopicHash, Multiaddr, PeerId};
use std::error::Error;

#[derive(Debug)]
pub enum Event {
    NodeUnregistered {
        node_id: String
    },
    PubSubMessageReceivedEvent {
        topic: TopicHash,
        message: Vec<u8>,
        from: Option<PeerId>,
    },
    NewAddress {
        address: Multiaddr,
    },
}

#[derive(Debug)]
pub enum PubsubResult {
    Ok {
        message: Vec<u8>,
        from: Option<PeerId>,
    },
    Err(Box<dyn Error + Send + Sync>),
}
