use libp2p::{gossipsub::TopicHash, PeerId, Stream, StreamProtocol};
use std::error::Error;
#[cfg(feature="py")]
use std::sync::Arc;
#[cfg(feature="py")]
use tokio::sync::Mutex;
#[cfg(feature="py")]
use pyo3::prelude::*;
#[cfg(feature="py")]
use pyo3_asyncio::tokio::future_into_py;
#[cfg(feature="py")]
use futures::{AsyncReadExt, AsyncWriteExt};
#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

#[derive(Debug)]
pub enum Event {
    NewNodeRegistered {
        node: crate::network::Node,
    },
    StreamMessageReceivedEvent {
        protocol: StreamProtocol,
        msg_reader: Stream,
        peer: PeerId,
    },
    PubSubMessageReceivedEvent {
        topic: TopicHash,
        message: Vec<u8>,
        from: Option<PeerId>,
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

#[cfg(feature="py")]
impl PyStream {
    pub fn new(protocol: String, peer: String, stream: Stream) -> Self {
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

#[cfg(feature="py")]
#[pyclass]
pub(crate) struct PubSubMessageReceivedEvent {
    pub topic: TopicHash,
    pub result: PubsubResult,
}

#[cfg(feature="py")]
#[pymethods]
impl PubSubMessageReceivedEvent {
    #[getter]
    pub fn topic(&self) -> String {
        self.topic.to_string()
    }

    #[getter]
    pub fn message(&self) -> PyResult<Vec<u8>> {
        match &self.result {
            PubsubResult::Ok { message, .. } => Ok(message.clone()),
            PubsubResult::Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
        }
    }

    #[getter]
    pub fn from(&self) -> Option<String> {
        match &self.result {
            PubsubResult::Ok { from, .. } => from.as_ref().map(|p| p.to_string()),
            _ => None,
        }
    }
}

#[cfg(feature="py")]
#[pyclass]
pub struct PyStream {
    protocol: String,
    peer: String,
    stream: Arc<Mutex<Stream>>,
}

#[cfg(feature="py")]
#[pymethods]
impl PyStream {
    #[getter]
    pub fn protocol(&self) -> String {
        self.protocol.clone()
    }

    #[getter]
    pub fn peer(&self) -> String {
        self.peer.clone()
    }

    pub fn next<'a>(&self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let stream = self.stream.clone();
        
        future_into_py(py, async move {
            let mut s = stream.lock().await;
            let mut length_buf = [0u8; 4];
            s.read_exact(&mut length_buf).await?;

            // TODO: handle buffer overflow
            let length = u32::from_be_bytes(length_buf) as usize;
            let mut buffer = vec![0u8; length];
            s.read_exact(&mut buffer).await?;

            Ok(buffer)
        })
    }

    pub fn write<'a>(&self, data: Vec<u8>, py: Python<'a>) -> PyResult<&'a PyAny> {
        let stream = self.stream.clone();
        
        future_into_py(py, async move {
            let mut s = stream.lock().await;
            s.write_all(&data).await?;

            Ok(())
        })
    }

    pub fn close<'a>(&self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let stream = self.stream.clone();
        
        future_into_py(py, async move {
            let mut s = stream.lock().await;
            s.close().await?;

            Ok(())
        })
    }
}
