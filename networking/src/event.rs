#[derive(Debug)]
pub enum Event {
    NewNodeRegistered {
        node: crate::network::Node,
    },
    MessageReceived {
        message: Vec<u8>,
    },
}
