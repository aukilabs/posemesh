use std::time::Duration;
use std::{error::Error, io};
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
                if retries >= max_retries {
                    return Err(e);
                }
                tracing::warn!("Retry {}/{} after {:?}: {:?}", retries + 1, max_retries, delay, e);
                sleep(delay).await;
                delay *= 2;
                retries += 1;
            }
        }
    }
}

#[cfg(target_family = "wasm")]
pub async fn timeout<F, T>(duration: Duration, future: F) -> Result<T, Box<dyn Error + Send + Sync>>
where
    F: Future<Output = T> + 'static + Send,
    T: 'static + Send,
{
    if duration.is_zero() {
        return Ok(future.await);
    }
    let timeout_fut = gloo_timers::future::TimeoutFuture::new(duration.as_millis() as u32);
    futures::select! {
        result = future.fuse() => Ok(result),
        _ = timeout_fut.fuse() => Err(Box::new(io::Error::new(io::ErrorKind::TimedOut, "Operation timed out")) as Box<dyn Error + Send + Sync>),
    }
}

#[cfg(not(target_family = "wasm"))]
pub async fn timeout<F, T>(duration: Duration, future: F) -> Result<T, Box<dyn Error + Send + Sync>>
where
    F: Future<Output = T>,
{
    if duration.is_zero() {
        return Ok(future.await);
    }
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| Box::new(io::Error::new(io::ErrorKind::TimedOut, "Operation timed out")) as Box<dyn Error + Send + Sync>)
}
