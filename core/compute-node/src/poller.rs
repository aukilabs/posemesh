use anyhow::Result;
use tokio::sync::watch;
use tokio::time::{sleep, Duration};

/// Poller backoff configuration.
#[derive(Clone, Copy, Debug)]
pub struct PollerConfig {
    pub backoff_ms_min: u64,
    pub backoff_ms_max: u64,
}

/// Compute a jittered delay within [min, max] inclusive.
pub fn jittered_delay_ms(cfg: PollerConfig) -> u64 {
    let min = cfg.backoff_ms_min.min(cfg.backoff_ms_max);
    let max = cfg.backoff_ms_max.max(cfg.backoff_ms_min);
    if min == max {
        return min;
    }
    let span = max - min + 1;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    min + ((now.subsec_millis() as u64) % span)
}

/// Shutdown signal for poller loop.
#[derive(Clone, Debug)]
pub struct ShutdownTx(watch::Sender<bool>);
#[derive(Debug)]
pub struct ShutdownRx(watch::Receiver<bool>);

/// Create shutdown channel; set to true to stop loop.
pub fn shutdown_channel() -> (ShutdownTx, ShutdownRx) {
    let (tx, rx) = watch::channel(false);
    (ShutdownTx(tx), ShutdownRx(rx))
}

impl ShutdownTx {
    pub fn shutdown(&self) {
        let _ = self.0.send(true);
    }
}

/// Run a cancellable poller loop invoking `on_tick` between backoff periods.
pub async fn run_poller<F, Fut>(
    cfg: PollerConfig,
    mut shutdown_rx: ShutdownRx,
    mut on_tick: F,
) -> Result<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    loop {
        on_tick().await;
        let delay = jittered_delay_ms(cfg);
        let mut remain = Duration::from_millis(delay);
        // Sleep in small chunks to observe cancellation quickly.
        while remain > Duration::from_millis(0) {
            let step = remain.min(Duration::from_millis(50));
            tokio::select! {
                changed = shutdown_rx.0.changed() => {
                    if changed.is_err() || *shutdown_rx.0.borrow() { return Ok(()); }
                }
                _ = sleep(step) => {
                    remain -= step;
                }
            }
        }
    }
}
