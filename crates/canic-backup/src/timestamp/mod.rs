//! Module: timestamp
//!
//! Responsibility: generate compact wall-clock markers for backup metadata.
//! Does not own: clock synchronization, ordering guarantees, or persistence.
//! Boundary: supplies human-readable timestamps to backup journals and reports.

use std::time::{SystemTime, UNIX_EPOCH};

/// Return the current wall-clock timestamp as a compact unix-seconds marker.
#[must_use]
pub fn current_timestamp_marker() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());

    format!("unix:{seconds}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure generated timestamp markers use the stable unix-seconds prefix.
    #[test]
    fn current_timestamp_marker_uses_unix_prefix() {
        let marker = current_timestamp_marker();
        let seconds = marker
            .strip_prefix("unix:")
            .expect("timestamp marker should include prefix");

        assert!(seconds.parse::<u64>().is_ok());
    }
}
