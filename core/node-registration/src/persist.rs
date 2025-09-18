use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub const NODE_SECRET_PATH: &str = "data/node_secret";
static NODE_SECRET_STORE: OnceLock<Mutex<HashMap<PathBuf, String>>> = OnceLock::new();

fn node_secret_store() -> &'static Mutex<HashMap<PathBuf, String>> {
    NODE_SECRET_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_node_secret_store() -> Result<MutexGuard<'static, HashMap<PathBuf, String>>> {
    node_secret_store()
        .lock()
        .map_err(|_| anyhow!("node secret store poisoned"))
}

/// Store node secret bytes in memory under the provided `path` identifier.
pub fn write_node_secret_to_path(path: &Path, secret: &str) -> Result<()> {
    let mut store = lock_node_secret_store()?;
    store.insert(path.to_path_buf(), secret.to_owned());
    Ok(())
}

/// Read secret contents from `path`. Returns Ok(None) if missing.
pub fn read_node_secret_from_path(path: &Path) -> Result<Option<String>> {
    let store = lock_node_secret_store()?;
    Ok(store.get(path).cloned())
}

/// Convenience: write to default path `data/node_secret`.
pub fn write_node_secret(secret: &str) -> Result<()> {
    write_node_secret_to_path(Path::new(NODE_SECRET_PATH), secret)
}

/// Convenience: read from default path `data/node_secret`.
pub fn read_node_secret() -> Result<Option<String>> {
    read_node_secret_from_path(Path::new(NODE_SECRET_PATH))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_roundtrip() {
        let path = PathBuf::from(format!("node_secret_{}", uuid::Uuid::new_v4()));

        write_node_secret_to_path(&path, "first").unwrap();
        let got = read_node_secret_from_path(&path).unwrap();
        assert_eq!(got.as_deref(), Some("first"));

        write_node_secret_to_path(&path, "second").unwrap();
        let got2 = read_node_secret_from_path(&path).unwrap();
        assert_eq!(got2.as_deref(), Some("second"));

        let other_path = PathBuf::from(format!("node_secret_other_{}", uuid::Uuid::new_v4()));
        assert!(read_node_secret_from_path(&other_path).unwrap().is_none());

        let mut store = lock_node_secret_store().unwrap();
        store.remove(&path);
        store.remove(&other_path);
    }
}
