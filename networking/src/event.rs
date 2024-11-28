use futures::AsyncReadExt;
use libp2p::{PeerId, Stream, StreamProtocol};
#[cfg(feature="py")]
use std::sync::Arc;
#[cfg(feature="py")]
use tokio::sync::Mutex;
#[cfg(feature="py")]
use pyo3::prelude::*;
#[cfg(feature="py")]
use pyo3_asyncio::tokio::future_into_py;

#[derive(Debug)]
pub enum Event {
    NewNodeRegistered {
        node: crate::network::Node,
    },
    MessageReceived {
        protocol: StreamProtocol,
        stream: Stream,
        peer: PeerId,
    },
}

#[cfg(feature="py")]
#[pyclass]
pub(crate) struct MessageReceivedEvent {
    protocol: StreamProtocol,
    peer: PeerId,
    stream: Arc<Mutex<Stream>>,
}

#[cfg(feature="py")]
#[pymethods]
impl MessageReceivedEvent {
    #[getter]
    pub fn protocol(&self) -> String {
        self.protocol.to_string()
    }

    #[getter]
    pub fn peer(&self) -> String {
        self.peer.to_string()
    }

    #[getter]
    pub fn message<'a>(&self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let mut buf = Vec::new();
        let stream = self.stream.clone();
        
        future_into_py(py, async move {
            let mut s = stream.lock().await;
            s.read_to_end(&mut buf).await.unwrap();
            Ok(buf)
        })
    }
}

#[cfg(feature="py")]
impl MessageReceivedEvent {
    pub fn new(protocol: StreamProtocol, peer: PeerId, stream: Stream) -> Self {
        Self { protocol, peer, stream: Arc::new(Mutex::new(stream)) }
    }
}

#[cfg(feature="py")]
#[pyclass]
pub(crate) struct NewNodeRegisteredEvent {
    node: crate::network::Node,
}

#[cfg(feature="py")]
#[pymethods]
impl NewNodeRegisteredEvent {
    #[getter]
    pub fn node(&self) -> crate::network::Node {
        self.node.clone()
    }
}

#[cfg(feature="py")]
impl NewNodeRegisteredEvent {
    pub fn new(node: crate::network::Node) -> Self {
        Self { node }
    }
}
