use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tokio::time::sleep as tokio_sleep;

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

pub async fn sleep(seconds: f64) {
    let duration = Duration::from_secs_f64(seconds);
    tokio_sleep(duration).await;
}

pub fn format(timestamp: i64, format: String) -> String {
    // Simple implementation - in a real system you'd use chrono or similar
    format!("Timestamp: {} (format: {})", timestamp, format)
}