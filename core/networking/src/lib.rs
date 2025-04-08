pub mod client;
pub mod event;
pub mod libp2p;

#[cfg(feature="c")]
mod binding_helper;

#[cfg(all(feature="c", not(target_family="wasm")))]
mod c;

#[cfg(all(target_family="wasm", feature="c"))]
mod c_compat_wasm;

use std::fmt::Debug;

use futures::io::{AsyncRead, AsyncWrite};

pub trait AsyncStream: AsyncRead + AsyncWrite + Send + Unpin + Debug {}
impl<T: AsyncRead + AsyncWrite + Send + Unpin + Debug> AsyncStream for T {}
