// #[cfg(feature="py")]
// use pyo3::prelude::*;

// #[cfg_attr(feature = "py", pyclass)]
#[derive(Debug)]
pub enum Event {
    NewNodeRegistered {
        node: crate::network::Node,
    },
    MessageReceived {
        message: Vec<u8>,
    },
    // #[cfg(feature="py")]
    None {},
}
