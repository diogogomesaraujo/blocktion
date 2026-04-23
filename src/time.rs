use std::{
    error::Error,
    time::{SystemTime, UNIX_EPOCH},
};

pub type Timestamp = u64;

pub fn now_unix() -> Result<Timestamp, Box<dyn Error + Send + Sync>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}
