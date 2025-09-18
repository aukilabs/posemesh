use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

pub const STATE_PATH: &str = "data/registration_state.json";

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

static STATE_STORE: OnceLock<Mutex<HashMap<PathBuf, RegistrationState>>> = OnceLock::new();

fn state_store() -> &'static Mutex<HashMap<PathBuf, RegistrationState>> {
    STATE_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_state_store() -> Result<MutexGuard<'static, HashMap<PathBuf, RegistrationState>>> {
    state_store()
        .lock()
        .map_err(|_| anyhow!("registration state store poisoned"))
}

pub fn read_state_from_path(path: &Path) -> Result<RegistrationState> {
    let store = lock_state_store()?;
    Ok(store.get(path).cloned().unwrap_or_default())
}

pub fn write_state_to_path(path: &Path, st: &RegistrationState) -> Result<()> {
    let mut store = lock_state_store()?;
    store.insert(path.to_path_buf(), st.clone());
    Ok(())
}

pub fn read_state() -> Result<RegistrationState> {
    read_state_from_path(Path::new(STATE_PATH))
}

pub fn write_state(st: &RegistrationState) -> Result<()> {
    write_state_to_path(Path::new(STATE_PATH), st)
}

pub fn set_status(new_status: &str) -> Result<()> {
    let mut st = read_state()?;
    st.status = new_status.to_string();
    write_state(&st)
}

pub fn touch_healthcheck_now() -> Result<()> {
    let mut st = read_state()?;
    st.last_healthcheck = Some(Utc::now());
    write_state(&st)
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
}
