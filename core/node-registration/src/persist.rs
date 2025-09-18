use anyhow::{anyhow, Result};
use std::sync::{Mutex, MutexGuard, OnceLock};

#[derive(Default)]
struct NodeSecretStore {
    secret: Option<String>,
}

static NODE_SECRET_STORE: OnceLock<Mutex<NodeSecretStore>> = OnceLock::new();

fn node_secret_store() -> &'static Mutex<NodeSecretStore> {
    NODE_SECRET_STORE.get_or_init(|| Mutex::new(NodeSecretStore::default()))
}

fn lock_node_secret_store() -> Result<MutexGuard<'static, NodeSecretStore>> {
    node_secret_store()
        .lock()
        .map_err(|_| anyhow!("node secret store poisoned"))
}

/// Store node secret bytes in memory.
pub fn write_node_secret(secret: &str) -> Result<()> {
    let mut store = lock_node_secret_store()?;
    store.secret = Some(secret.to_owned());
    Ok(())
}

/// Read secret contents. Returns Ok(None) if missing.
pub fn read_node_secret() -> Result<Option<String>> {
    let store = lock_node_secret_store()?;
    Ok(store.secret.clone())
}

/// Clear any stored secret. Intended for tests.
pub fn clear_node_secret() -> Result<()> {
    let mut store = lock_node_secret_store()?;
    store.secret = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_roundtrip() {
        clear_node_secret().unwrap();

        write_node_secret("first").unwrap();
        let got = read_node_secret().unwrap();
        assert_eq!(got.as_deref(), Some("first"));

        write_node_secret("second").unwrap();
        let got2 = read_node_secret().unwrap();
        assert_eq!(got2.as_deref(), Some("second"));

        clear_node_secret().unwrap();
        assert!(read_node_secret().unwrap().is_none());
    }
}
