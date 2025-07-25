use once_cell::sync::Lazy;
use std::{sync::Mutex, time::Instant};
use sysinfo::Networks;

static NETWORKS: Lazy<Mutex<Networks>> =
    Lazy::new(|| Mutex::new(Networks::new_with_refreshed_list()));

static LAST_TIME: Lazy<Mutex<Option<Instant>>> =
    Lazy::new(|| Mutex::new(None));

pub fn net_usage_bps() -> (f32, f32) {
    let mut nets = NETWORKS.lock().unwrap();
    // sysinfo 0.36.x: `refresh(remove_not_listed_interfaces: bool)`
    nets.refresh(false);

    let now = Instant::now();
    // `received()/transmitted()` are BYTES **since previous refresh**. Not totals!
    // docs: "Returns the number of ... since the last refresh". :contentReference[oaicite:1]{index=1}
    let (rx_delta, tx_delta): (u64, u64) = nets
        .iter()
        .map(|(_, d)| (d.received(), d.transmitted()))
        .fold((0, 0), |(r, t), (rx, tx)| (r + rx, t + tx));

    let mut prev = LAST_TIME.lock().unwrap();
    let dt = if let Some(t) = *prev {
        now.duration_since(t).as_secs_f32().max(1e-3)
    } else {
        1.0
    };
    *prev = Some(now);

    (rx_delta as f32 / dt, tx_delta as f32 / dt)
}