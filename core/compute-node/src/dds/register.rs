use crate::config::NodeConfig;
use anyhow::Result;

/// Compatibility shim for hosts that call this before `engine::run_node`.
///
/// Registration now starts and stops with the SDK-managed worker, using the
/// capabilities in its runner registry. This call starts no background work;
/// hosts can remove it when convenient.
pub fn spawn_registration_if_configured(_cfg: &NodeConfig, _capabilities: &[String]) -> Result<()> {
    Ok(())
}
