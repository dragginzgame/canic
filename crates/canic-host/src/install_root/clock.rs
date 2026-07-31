use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn current_unix_timestamp_label() -> Result<String, Box<dyn std::error::Error>> {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    Ok(format!("unix:{seconds}"))
}
