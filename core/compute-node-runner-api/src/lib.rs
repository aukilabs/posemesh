//! posemesh-compute-node-runner-api: Stable seam for runners (no HTTP).
//!
//! Exposes:
//! - Data contracts: `LeaseEnvelope`, `TaskSpec`.
//! - Runner ports: `InputSource`, `ArtifactSink`, `ControlPlane`.
//! - Execution: `TaskCtx`, `Runner`.

/// Public crate identifier used by workspace smoke tests.
pub const CRATE_NAME: &str = "posemesh-compute-node-runner-api";

pub mod runner;
pub mod types;

pub use runner::{ArtifactSink, ControlPlane, InputSource, MaterializedInput, Runner, TaskCtx};
pub use types::{LeaseEnvelope, TaskSpec};
