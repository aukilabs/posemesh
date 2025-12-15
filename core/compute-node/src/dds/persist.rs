//! Compatibility layer: delegate persistence of the DDS registration secret
//! to the canonical store provided by `posemesh-node-registration`.

use anyhow::Result;

/// Persist node secret in memory. Overwrites existing secret.
pub fn write_node_secret(secret: &str) -> Result<()> {
    posemesh_node_registration::state::write_node_secret(secret)
}

/// Read persisted secret. Returns `Ok(None)` if secret missing.
pub fn read_node_secret() -> Result<Option<String>> {
    posemesh_node_registration::state::read_node_secret()
}

/// Clear any persisted secret. Intended for tests.
pub fn clear_node_secret() -> Result<()> {
    posemesh_node_registration::state::clear_node_secret()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_read_cycle() {
        clear_node_secret().unwrap();
        assert!(read_node_secret().unwrap().is_none());

        write_node_secret("test-secret").unwrap();
        assert_eq!(read_node_secret().unwrap().as_deref(), Some("test-secret"));

        write_node_secret("new-secret").unwrap();
        assert_eq!(read_node_secret().unwrap().as_deref(), Some("new-secret"));

        clear_node_secret().unwrap();
        assert!(read_node_secret().unwrap().is_none());
    }
}
