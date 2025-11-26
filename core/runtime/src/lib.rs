use once_cell::sync::Lazy;
use tokio::runtime::{Handle, Runtime};

static GLOBAL_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    Runtime::new().expect("Failed to create Tokio runtime")
});

/// Expose a function to get the global Tokio runtime.
pub fn get_runtime() -> &'static Runtime {
    &*GLOBAL_RUNTIME
}

/// Run code inside a Tokio runtime context.
/// Reuses current runtime if present.
pub fn with_runtime<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    if let Ok(handle) = Handle::try_current() {
        let _guard = handle.enter();
        f()
    } else {
        let handle = GLOBAL_RUNTIME.handle().clone();
        let _guard = handle.enter();
        f()
    }
}
