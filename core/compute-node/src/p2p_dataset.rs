//! Compatibility re-exports for the extracted authenticated dataset protocol.
//!
//! New code should depend on `auki-p2p-dataset` directly. Keeping this module
//! avoids a flag-day migration for existing compute-node embedders.

pub use auki_p2p_dataset::*;
