use posemesh_compute_node::heartbeat::{
    progress_channel, run_scheduler, shutdown_channel, HeartbeatData,
};
use serde_json::json;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration, Instant};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn coalesces_updates_and_sends_last() {
    let (ptx, prx) = progress_channel();
    let (stx, srx) = shutdown_channel();

    let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let seen_cl = seen.clone();
    let jh = tokio::spawn(async move {
        run_scheduler(prx, srx, 30, move |hb: HeartbeatData| {
            let seen = seen_cl.clone();
            async move {
                seen.lock()
                    .unwrap()
                    .push(hb.progress.as_str().unwrap_or_default().to_string());
            }
        })
        .await
        .unwrap();
    });

    ptx.update(json!("a"), json!({}));
    // Second update comes shortly after; should be coalesced into single heartbeat delivering "b"
    sleep(Duration::from_millis(5)).await;
    ptx.update(json!("b"), json!({}));

    // Wait for one heartbeat
    let start = Instant::now();
    loop {
        if start.elapsed() > Duration::from_millis(500) {
            break;
        }
        let has_values = {
            let guard = seen.lock().unwrap();
            !guard.is_empty()
        };
        if has_values {
            break;
        }
        sleep(Duration::from_millis(5)).await;
    }

    stx.shutdown();
    let _ = jh.await;

    let vals = seen.lock().unwrap().clone();
    assert_eq!(
        vals.len(),
        1,
        "expected single debounced heartbeat, got {:?}",
        vals
    );
    assert_eq!(vals[0], "b");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn jitter_within_upper_bound() {
    let (ptx, prx) = progress_channel();
    let (stx, srx) = shutdown_channel();
    let times: Arc<Mutex<Vec<u128>>> = Arc::new(Mutex::new(Vec::new()));
    let times_cl = times.clone();

    let jh = tokio::spawn(async move {
        run_scheduler(prx, srx, 20, move |_hb: HeartbeatData| {
            let times = times_cl.clone();
            async move {
                times
                    .lock()
                    .unwrap()
                    .push(Instant::now().elapsed().as_millis());
            }
        })
        .await
        .unwrap();
    });

    let t0 = Instant::now();
    ptx.update(json!("p"), json!({}));

    // Wait for first heartbeat
    loop {
        if t0.elapsed() > Duration::from_millis(500) {
            break;
        }
        let has_entry = {
            let guard = times.lock().unwrap();
            !guard.is_empty()
        };
        if has_entry {
            break;
        }
        sleep(Duration::from_millis(5)).await;
    }
    stx.shutdown();
    let _ = jh.await;

    let delay_ms = times.lock().unwrap().first().copied().unwrap_or(0);
    assert!(
        delay_ms <= 20 + 100,
        "delay {}ms exceeds jitter bound",
        delay_ms
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_stops_loop() {
    let (_ptx, prx) = progress_channel();
    let (stx, srx) = shutdown_channel();

    let jh = tokio::spawn(async move { run_scheduler(prx, srx, 10, |_hb| async {}).await });
    stx.shutdown();
    let res = tokio::time::timeout(Duration::from_millis(200), jh).await;
    assert!(res.is_ok(), "scheduler did not stop on cancel");
}
