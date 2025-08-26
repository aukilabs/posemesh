use std::{io::{Error, ErrorKind}, time::Duration};
use futures::{self, Future, FutureExt};

#[cfg(not(target_family = "wasm"))]
use tokio::time::sleep;

#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_family = "wasm")]
use wasm_bindgen::{closure::Closure, JsCast};
#[cfg(target_family = "wasm")]
use js_sys::Promise;
#[cfg(target_family = "wasm")]
use wasm_bindgen::JsValue;

#[cfg(feature="crypto")]
pub mod crypto;

#[cfg(target_family = "wasm")]
pub async fn sleep(duration: Duration) {
    let window = web_sys::window().expect("no window");
    let promise = Promise::new(&mut |resolve, _| {
        window.set_timeout_with_callback_and_timeout_and_arguments_0(
            &Closure::once_into_js(move || {
                resolve.call0(&JsValue::null()).unwrap();
            }).unchecked_ref(),
            duration.as_millis() as i32,
        ).expect("failed to set timeout");
    });
    JsFuture::from(promise).await.expect("failed to wait");
}

pub const INFINITE_RETRIES: u32 = 0;

/// Retries an async operation with a delay between attempts.
/// 
/// # Arguments
/// * `f` - The async function to retry
/// * `max_attempts` - Maximum number of attempts, 0 means infinite retries, 1 means only one attempt
/// * `delay` - Duration to wait between retries
/// 
/// # Returns
/// * `Ok(T)` - The successful result
/// * `Err(E)` - The error from the last attempt if all attempts failed


pub async fn retry_with_delay<F, Fut, T, E>(mut f: F, max_attempts: u32, delay: Duration) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut retries = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                retries += 1;
                if max_attempts != INFINITE_RETRIES && retries >= max_attempts {
                    return Err(e);
                }
                tracing::warn!("Retry {}/{} after {:?}: {:?}", retries, max_attempts, delay, e);
                sleep(delay).await;
            }
        }
    }
}

pub async fn retry_with_increasing_delay<F, T, E>(mut f: F, max_retries: u32, initial_delay: Duration) -> Result<T, E>
where
    F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
    E: std::fmt::Debug,
{
    let mut retries = 0;
    let mut delay = initial_delay;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                retries += 1;
                if retries >= max_retries {
                    return Err(e);
                }
                tracing::warn!("Retry {}/{} after {:?}: {:?}", retries, max_retries, delay, e);
                sleep(delay).await;
                delay *= 2;
            }
        }
    }
}

#[cfg(target_family = "wasm")]
pub async fn timeout<F, T>(duration: Duration, future: F) -> Result<T, Error>
where
    F: Future<Output = T> + Send,
{
    if duration.is_zero() {
        return Ok(future.await);
    }
    let timeout_fut = gloo_timers::future::TimeoutFuture::new(duration.as_millis() as u32);
    futures::select! {
        result = future.fuse() => Ok(result),
        _ = timeout_fut.fuse() => Err(Error::new(ErrorKind::TimedOut, "Operation timed out")),
    }
}

#[cfg(not(target_family = "wasm"))]
pub async fn timeout<F, T>(duration: Duration, future: F) -> Result<T, Error>
where
    F: Future<Output = T>,
{
    if duration.is_zero() {
        return Ok(future.await);
    }
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| Error::new(ErrorKind::TimedOut, "Operation timed out"))
}
