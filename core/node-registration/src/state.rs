use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

pub const STATUS_REGISTERING: &str = "registering";
pub const STATUS_REGISTERED: &str = "registered";
pub const STATUS_DISCONNECTED: &str = "disconnected";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationState {
    pub status: String,
    #[serde(default, with = "chrono::serde::ts_seconds_option")]
    pub last_healthcheck: Option<DateTime<Utc>>,
}

impl Default for RegistrationState {
    fn default() -> Self {
        Self {
            status: STATUS_DISCONNECTED.to_string(),
            last_healthcheck: None,
        }
    }
}

#[derive(Default)]
struct InMemoryState {
    registration: Option<RegistrationState>,
    node_secret: Option<String>,
}

static STATE_STORE: OnceLock<Mutex<InMemoryState>> = OnceLock::new();

fn state_store() -> &'static Mutex<InMemoryState> {
    STATE_STORE.get_or_init(|| Mutex::new(InMemoryState::default()))
}

fn lock_state_store() -> Result<MutexGuard<'static, InMemoryState>> {
    state_store()
        .lock()
        .map_err(|_| anyhow!("registration state store poisoned"))
}

/// Store node secret bytes in memory.
pub fn write_node_secret(secret: &str) -> Result<()> {
    let mut store = lock_state_store()?;
    store.node_secret = Some(secret.to_owned());
    Ok(())
}

/// Read secret contents. Returns Ok(None) if missing.
pub fn read_node_secret() -> Result<Option<String>> {
    let store = lock_state_store()?;
    Ok(store.node_secret.clone())
}

/// Clear any stored secret. Intended for tests.
pub fn clear_node_secret() -> Result<()> {
    let mut store = lock_state_store()?;
    store.node_secret = None;
    Ok(())
}

pub fn read_state() -> Result<RegistrationState> {
    let store = lock_state_store()?;
    Ok(store.registration.clone().unwrap_or_default())
}

pub fn write_state(st: &RegistrationState) -> Result<()> {
    let mut store = lock_state_store()?;
    store.registration = Some(st.clone());
    Ok(())
}

pub fn set_status(new_status: &str) -> Result<()> {
    let mut store = lock_state_store()?;
    let st = store
        .registration
        .get_or_insert_with(RegistrationState::default);
    st.status = new_status.to_string();
    Ok(())
}

pub fn touch_healthcheck_now() -> Result<()> {
    let mut store = lock_state_store()?;
    let st = store
        .registration
        .get_or_insert_with(RegistrationState::default);
    st.last_healthcheck = Some(Utc::now());
    Ok(())
}

pub struct LockGuard;

impl LockGuard {
    pub fn try_acquire(stale_after: Duration) -> std::io::Result<Option<Self>> {
        let mut state = lock_lock_store()?;
        let now = Instant::now();

        if let Some(acquired_at) = state.acquired_at {
            if now.duration_since(acquired_at) <= stale_after {
                return Ok(None);
            }
        }

        state.acquired_at = Some(now);
        state.owner_pid = Some(std::process::id());

        Ok(Some(Self))
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = lock_lock_store() {
            state.acquired_at = None;
            state.owner_pid = None;
        }
    }
}

#[derive(Default)]
struct LockState {
    acquired_at: Option<Instant>,
    owner_pid: Option<u32>,
}

static LOCK_STATE: OnceLock<Mutex<LockState>> = OnceLock::new();

fn lock_store() -> &'static Mutex<LockState> {
    LOCK_STATE.get_or_init(|| Mutex::new(LockState::default()))
}

fn lock_lock_store() -> io::Result<MutexGuard<'static, LockState>> {
    lock_store()
        .lock()
        .map_err(|_| io::Error::other("registration lock store poisoned"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_disconnected() {
        let st = RegistrationState::default();
        assert_eq!(st.status, STATUS_DISCONNECTED);
        assert!(st.last_healthcheck.is_none());
    }

    #[test]
    fn write_and_read_secret_roundtrip() {
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
