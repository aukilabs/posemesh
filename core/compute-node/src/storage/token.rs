//! TokenRef for hot-swappable bearer tokens used by storage requests.

#[derive(Clone)]
pub struct TokenRef(std::sync::Arc<parking_lot::RwLock<String>>);

impl TokenRef {
    /// Create a new token reference with an initial value.
    pub fn new(initial: String) -> Self {
        Self(std::sync::Arc::new(parking_lot::RwLock::new(initial)))
    }

    /// Get a clone of the current token.
    pub fn get(&self) -> String {
        self.0.read().clone()
    }

    /// Swap the token value with a new one.
    pub fn swap(&self, v: String) {
        *self.0.write() = v;
    }
}
