//! In-memory persistence helpers for DDS registration secret.

use anyhow::{anyhow, Result};
use std::sync::{Mutex, MutexGuard, OnceLock};

#[derive(Default)]
struct NodeSecretStore {
    secret: Option<String>,
}

static NODE_SECRET_STORE: OnceLock<Mutex<NodeSecretStore>> = OnceLock::new();

fn store() -> &'static Mutex<NodeSecretStore> {
    NODE_SECRET_STORE.get_or_init(|| Mutex::new(NodeSecretStore::default()))
}

fn lock_store() -> Result<MutexGuard<'static, NodeSecretStore>> {
    store()
        .lock()
        .map_err(|_| anyhow!("node secret store poisoned"))
}

/// Persist node secret in memory. Overwrites existing secret.
pub fn write_node_secret(secret: &str) -> Result<()> {
    let mut guard = lock_store()?;
    guard.secret = Some(secret.to_owned());
    Ok(())
}

/// Read persisted secret. Returns `Ok(None)` if secret missing.
pub fn read_node_secret() -> Result<Option<String>> {
    let guard = lock_store()?;
    Ok(guard.secret.clone())
}

/// Clear any persisted secret. Intended for tests.
pub fn clear_node_secret() -> Result<()> {
    let mut guard = lock_store()?;
    guard.secret = None;
    Ok(())
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
