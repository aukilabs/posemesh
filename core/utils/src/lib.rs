pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

use std::time::Duration;
use tracing;

#[cfg(not(target_family = "wasm"))]
use tokio::time::sleep;

#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_family = "wasm")]
use web_sys::Window;
#[cfg(target_family = "wasm")]
use wasm_bindgen::{closure::Closure, JsCast};

#[cfg(target_family = "wasm")]
async fn sleep(duration: Duration) {
    let window = web_sys::window().expect("no window");
    let promise = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        &Closure::once_into_js(move || {}).unchecked_ref(),
        duration.as_millis() as i32,
    ).expect("failed to set timeout");
    JsFuture::from(promise).await.expect("failed to wait");
}

/// Retries an async operation with a delay between attempts.
/// 
/// # Arguments
/// * `f` - The async function to retry
/// * `max_retries` - Maximum number of retry attempts
/// * `delay` - Duration to wait between retries
/// 
/// # Returns
/// * `Ok(T)` - The successful result
/// * `Err(E)` - The error from the last attempt if all retries failed
pub async fn retry_with_delay<F, T, E>(mut f: F, max_retries: u32, delay: Duration) -> Result<T, E>
where
    F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
    E: std::fmt::Debug,
{
    let mut retries = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if retries >= max_retries {
                    return Err(e);
                }
                tracing::warn!("Retry {}/{} after {:?}: {:?}", retries + 1, max_retries, delay, e);
                sleep(delay).await;
                retries += 1;
            }
        }
    }
}
