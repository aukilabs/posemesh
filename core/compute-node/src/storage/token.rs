//! TokenRef for hot-swappable bearer tokens used by storage requests.

#[derive(Clone)]
pub struct TokenRef {
    value: std::sync::Arc<parking_lot::RwLock<String>>,
    task: Option<auki_sdk::TaskAccessToken>,
}

impl TokenRef {
    /// Create a new token reference with an initial value.
    pub fn new(initial: String) -> Self {
        Self {
            value: std::sync::Arc::new(parking_lot::RwLock::new(initial)),
            task: None,
        }
    }

    /// Get the current token. Managed task references return an empty string
    /// after lease expiry or revocation, preserving the legacy getter signature.
    pub fn get(&self) -> String {
        match &self.task {
            Some(task) => task
                .get()
                .map(|v| v.expose_secret().to_owned())
                .unwrap_or_default(),
            None => self.value.read().clone(),
        }
    }

    /// Swap the token value with a new one.
    pub fn swap(&self, v: String) {
        *self.value.write() = v;
    }

    pub(crate) fn from_task(task: auki_sdk::TaskAccessToken) -> Self {
        Self {
            task: Some(task),
            ..Self::new(String::new())
        }
    }
}

// Expose read-only access via runner API trait so runners can use the same token ref.
impl compute_runner_api::runner::AccessTokenProvider for TokenRef {
    fn get(&self) -> String {
        TokenRef::get(self)
    }
}
