use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_unix() -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}
