#[inline]
pub fn bytes_to_gb(bytes: u64) -> f32 {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    ((bytes as f64 / GB) * 10.0).round() as f32 / 10.0
}
