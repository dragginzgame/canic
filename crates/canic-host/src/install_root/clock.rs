use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn current_unix_timestamp_secs() -> Result<u64, Box<dyn std::error::Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

pub(super) fn current_unix_timestamp_label() -> Result<String, Box<dyn std::error::Error>> {
    let seconds = current_unix_timestamp_secs()?;
    Ok(format!("unix:{seconds}"))
}
