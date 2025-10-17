use posemesh_compute_node::poller::{
    jittered_delay_ms, run_poller, shutdown_channel, PollerConfig,
};
use tokio::time::{timeout, Duration};

#[test]
fn jitter_within_bounds() {
    let cfg = PollerConfig {
        backoff_ms_min: 10,
        backoff_ms_max: 25,
    };
    for _ in 0..100 {
        let d = jittered_delay_ms(cfg);
        assert!((10..=25).contains(&d), "delay {} out of bounds", d);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancellation_stops_poller() {
    let cfg = PollerConfig {
        backoff_ms_min: 100,
        backoff_ms_max: 150,
    };
    let (tx, rx) = shutdown_channel();

    let jh = tokio::spawn(async move {
        run_poller(cfg, rx, || async {}).await.unwrap();
    });

    // Cancel shortly; ensure poller returns quickly
    tx.shutdown();
    let res = timeout(Duration::from_millis(200), jh).await;
    assert!(res.is_ok(), "poller did not stop on cancellation");
}
