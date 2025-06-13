#[cfg(target_family="wasm")]
mod wasm;

mod protobuf;
pub mod error;
mod utils;

#[cfg(not(target_family="wasm"))]
pub mod client;
