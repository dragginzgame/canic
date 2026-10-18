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

/// Parse a Unix-seconds marker without consulting the clock.
#[must_use]
pub fn timestamp_seconds(marker: &str) -> Option<u64> {
    marker
        .strip_prefix("unix:")
        .and_then(|seconds| seconds.parse::<u64>().ok())
}

pub(crate) fn timestamp_marker(seconds: u64) -> String {
    format!("unix:{seconds}")
}

pub(crate) fn current_unix_seconds() -> u64 {
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
        assert_eq!(timestamp_seconds("unix:42"), Some(42));
        assert_eq!(timestamp_marker(42), "unix:42");
    }
}

#[cfg(test)]
mod parse_tests {
    use super::timestamp_seconds;

    #[test]
    fn parser_preserves_boundaries_and_rejects_invalid_markers() {
        assert_eq!(timestamp_seconds("unix:0"), Some(0));
        assert_eq!(
            timestamp_seconds("unix:18446744073709551615"),
            Some(u64::MAX)
        );
        for marker in [
            "",
            "42",
            "unix:",
            "unix:-1",
            "unix: 42",
            "unix:42 ",
            "unix:18446744073709551616",
        ] {
            assert_eq!(timestamp_seconds(marker), None);
        }
    }
}
