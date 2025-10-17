//! DDS router/client skeleton (no HTTP).

pub mod persist;
pub mod register;

use url::Url;

/// Placeholder router for DDS interactions (e.g., SIWE auth flow).
#[derive(Clone, Debug)]
pub struct DdsRouter {
    pub base: Url,
}
impl DdsRouter {
    /// Create a new DDS router from base URL.
    pub fn new(base: Url) -> Self {
        Self { base }
    }
}
