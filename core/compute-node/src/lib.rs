//! posemesh-compute-node: Node engine and shared functionality.

/// Public crate identifier used by workspace smoke tests.
pub const CRATE_NAME: &str = "posemesh-compute-node";

pub mod auth;
pub mod config;
pub mod dds;
pub mod dms;
pub mod engine;
pub mod errors;
pub mod heartbeat;
pub mod http;
pub mod p2p_dataset;
pub mod poller;
mod relay_booking;
pub mod session;
pub mod storage;
pub mod telemetry;
