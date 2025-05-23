pub mod cluster;
pub mod datastore;
mod binding_helper;
pub mod message;
pub mod protobuf {
    include!("protobuf/mod.rs");
}
pub mod spatial;
pub mod auth;

#[cfg(all(feature="c", not(target_family="wasm")))]
mod c;

#[cfg(target_family = "wasm")]
mod wasm;

pub mod capabilities;
