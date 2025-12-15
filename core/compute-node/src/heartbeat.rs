use anyhow::Result;
use serde_json::Value;
use tokio::sync::watch;
use tokio::time::{sleep, Duration};

/// Payload carried by heartbeats: last progress and small event map.
#[derive(Clone, Debug, Default)]
pub struct HeartbeatData {
    pub progress: Value,
    pub events: Value,
}

/// Sender side of the progress channel.
#[derive(Clone, Debug)]
pub struct ProgressSender(watch::Sender<Option<HeartbeatData>>);

/// Receiver side of the progress channel.
#[derive(Debug)]
pub struct ProgressReceiver(watch::Receiver<Option<HeartbeatData>>);

/// Create a new progress channel. Only the latest value is relevant (watch).
pub fn progress_channel() -> (ProgressSender, ProgressReceiver) {
    let (tx, rx) = watch::channel::<Option<HeartbeatData>>(None);
    (ProgressSender(tx), ProgressReceiver(rx))
}

impl ProgressSender {
    /// Replace the current progress/events state.
    pub fn update(&self, progress: Value, events: Value) {
        let _ = self
            .0
            .send_replace(Some(HeartbeatData { progress, events }));
    }
}

impl ProgressReceiver {
    pub(crate) async fn recv(&mut self) -> Option<HeartbeatData> {
        if self.0.changed().await.is_err() {
            return None;
        }
        self.0.borrow().clone()
    }
}

/// Shutdown signal for the heartbeat loop.
#[derive(Clone, Debug)]
pub struct ShutdownTx(watch::Sender<bool>);
#[derive(Debug)]
pub struct ShutdownRx(watch::Receiver<bool>);

/// Create a shutdown channel (false by default). Set to true to stop the loop.
pub fn shutdown_channel() -> (ShutdownTx, ShutdownRx) {
    let (tx, rx) = watch::channel(false);
    (ShutdownTx(tx), ShutdownRx(rx))
}

impl ShutdownTx {
    pub fn shutdown(&self) {
        let _ = self.0.send(true);
    }
}

/// Debounced heartbeat scheduler.
///
/// - Listens for progress updates on a watch channel.
/// - On change, waits a small jitter (0..=heartbeat_jitter_ms) and invokes `on_heartbeat`
///   with the latest progress/events, coalescing multiple rapid updates into a single call.
/// - Stops when `shutdown_rx` becomes true or all senders are dropped.
pub async fn run_scheduler<F, Fut>(
    mut progress_rx: ProgressReceiver,
    mut shutdown_rx: ShutdownRx,
    heartbeat_jitter_ms: u64,
    mut on_heartbeat: F,
) -> Result<()>
where
    F: FnMut(HeartbeatData) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    loop {
        tokio::select! {
            changed = shutdown_rx.0.changed() => {
                if changed.is_err() || *shutdown_rx.0.borrow() { break; }
            }
            changed = progress_rx.0.changed() => {
                if changed.is_err() { break; }
                // Mark current as observed to coalesce rapid updates during jitter.
                {
                    let _ = progress_rx.0.borrow_and_update();
                }
                let jitter = jitter_delay_ms(heartbeat_jitter_ms);
                if jitter > 0 { sleep(Duration::from_millis(jitter)).await; }
                let latest: Option<HeartbeatData> = {
                    progress_rx.0.borrow_and_update().clone()
                };
                if let Some(data) = latest {
                    on_heartbeat(data).await;
                }
            }
        }
    }
    Ok(())
}

fn jitter_delay_ms(max_ms: u64) -> u64 {
    if max_ms == 0 {
        return 0;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    let min = std::cmp::max(1, max_ms / 2);
    let span = max_ms - min + 1;
    min + ((now.subsec_millis() as u64) % span)
}
