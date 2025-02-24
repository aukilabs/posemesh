use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    Runtime::new().expect("Failed to create Tokio runtime")
});

/// Expose a function to get the global Tokio runtime.
pub fn get_runtime() -> &'static Runtime {
    &*RUNTIME
}
