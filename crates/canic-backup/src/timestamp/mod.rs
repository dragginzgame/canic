//! Module: timestamp
//!
//! Responsibility: derive and generate compact unix timestamp markers for backup metadata.
//! Does not own: clock synchronization, ordering guarantees, or persistence.
//! Boundary: supplies human-readable timestamps to backup journals and reports.

use std::time::{SystemTime, UNIX_EPOCH};

/// Return the current wall-clock timestamp as a compact unix-seconds marker.
#[must_use]
pub fn current_timestamp_marker() -> String {
    timestamp_marker(current_unix_seconds())
}

pub(crate) fn state_updated_at(updated_at: Option<&String>) -> String {
    updated_at.cloned().unwrap_or_else(current_timestamp_marker)
}

pub(crate) fn timestamp_seconds(marker: &str) -> u64 {
    marker
        .strip_prefix("unix:")
        .and_then(|seconds| seconds.parse::<u64>().ok())
        .unwrap_or_else(current_unix_seconds)
}

pub(crate) fn timestamp_marker(seconds: u64) -> String {
    format!("unix:{seconds}")
}

fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
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

    #[test]
    fn supplied_state_timestamp_is_preserved() {
        assert_eq!(state_updated_at(Some(&"unix:42".to_string())), "unix:42");
        assert_eq!(timestamp_seconds("unix:42"), 42);
        assert_eq!(timestamp_marker(42), "unix:42");
    }
}
