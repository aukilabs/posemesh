use std::time::Duration;
use std::io;
use futures::{self, Future};

#[cfg(not(target_family = "wasm"))]
use tokio::time::sleep;

#[cfg(target_family = "wasm")]
use futures::FutureExt;

#[cfg(target_family = "wasm")]
pub async fn sleep(duration: Duration) {
    gloo_timers::future::TimeoutFuture::new(duration.as_millis() as u32).await;
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
pub async fn retry_with_delay<F, T, E>(mut f: F, max_attempts: u32, delay: Duration) -> Result<T, E>
where
    F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
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
pub async fn timeout<F, T>(duration: Duration, future: F) -> Result<T, io::Error>
where
    F: Future<Output = T>,
    T: Send + Sync,
{
    if duration.is_zero() {
        return Ok(future.await);
    }
    let timeout_fut = gloo_timers::future::TimeoutFuture::new(duration.as_millis() as u32);
    futures::select! {
        result = future.fuse() => Ok(result),
        _ = timeout_fut.fuse() => Err(io::Error::new(io::ErrorKind::TimedOut, "Operation timed out")),
    }
}

#[cfg(not(target_family = "wasm"))]
pub async fn timeout<F, T>(duration: Duration, future: F) -> Result<T, io::Error>
where
    F: Future<Output = T>,
{
    if duration.is_zero() {
        return Ok(future.await);
    }
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Operation timed out"))
}

#[cfg(target_family = "wasm")]
pub fn now_unix_secs() -> u64 {
    let millis: f64 = wasm_bindgen_futures::js_sys::Date::now(); // milliseconds since epoch
    // truncate/floor safely
    let secs_f64 = (millis / 1000.0).floor();
    // convert to u64 safely
    u64::try_from(secs_f64 as i128).unwrap_or(u64::MAX)
}

#[cfg(not(target_family = "wasm"))]
pub fn now_unix_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
