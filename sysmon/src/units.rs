#[inline]
pub fn bytes_to_gb(bytes: u64) -> f32 {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    ((bytes as f64 / GB) * 10.0).round() as f32 / 10.0
}

pub fn fmt_rate(bps: f32) -> String {
    const KB: f32 = 1_024.0;
    const MB: f32 = KB * 1_024.0;
    const GB: f32 = MB * 1_024.0;
    if bps < KB {
        format!("{:.0} B/s", bps)
    } else if bps < MB {
        format!("{:.1} KB/s", bps / KB)
    } else if bps < GB {
        format!("{:.1} MB/s", bps / MB)
    } else {
        format!("{:.2} GB/s", bps / GB)
    }
}
