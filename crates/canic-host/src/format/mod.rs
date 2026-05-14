//! Host-side formatting helpers shared by operator tools.

pub use canic_core::__control_plane_core::format::{byte_size, cycles_tc};

/// Format the IC-relevant uncompressed wasm size, with gzip size as secondary.
#[must_use]
pub fn wasm_size_label(wasm_bytes: Option<u64>, gzip_bytes: Option<u64>) -> String {
    match (wasm_bytes, gzip_bytes) {
        (Some(wasm), Some(gzip)) => format!("{} (gz {})", byte_size(wasm), byte_size(gzip)),
        (Some(wasm), None) => byte_size(wasm),
        (None, Some(gzip)) => format!("n/a (gz {})", byte_size(gzip)),
        (None, None) => "-".to_string(),
    }
}

/// Format a duration in compact largest units for CLI tables and summaries.
#[must_use]
pub fn compact_duration(seconds: u64) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    const WEEK: u64 = 7 * DAY;

    match seconds {
        0 => "0s".to_string(),
        1..MINUTE => format!("{seconds}s"),
        MINUTE..HOUR => compact_duration_pair(seconds, MINUTE, "m", 1, "s"),
        HOUR..DAY => compact_duration_pair(seconds, HOUR, "h", MINUTE, "m"),
        DAY..WEEK => compact_duration_pair(seconds, DAY, "d", HOUR, "h"),
        _ => compact_duration_pair(seconds, WEEK, "w", DAY, "d"),
    }
}

fn compact_duration_pair(
    seconds: u64,
    major_seconds: u64,
    major_unit: &str,
    minor_seconds: u64,
    minor_unit: &str,
) -> String {
    let major = seconds / major_seconds;
    let minor = (seconds % major_seconds) / minor_seconds;
    if minor == 0 {
        format!("{major}{major_unit}")
    } else {
        format!("{major}{major_unit} {minor}{minor_unit}")
    }
}

#[cfg(test)]
mod tests {
    use super::{compact_duration, wasm_size_label};

    // Prefer the IC-installed wasm size while retaining gzip as optional context.
    #[test]
    fn formats_wasm_size_labels() {
        assert_eq!(
            wasm_size_label(Some(2 * 1024 * 1024), Some(512 * 1024)),
            "2.00 MiB (gz 512.00 KiB)"
        );
        assert_eq!(wasm_size_label(Some(2 * 1024 * 1024), None), "2.00 MiB");
        assert_eq!(
            wasm_size_label(None, Some(512 * 1024)),
            "n/a (gz 512.00 KiB)"
        );
        assert_eq!(wasm_size_label(None, None), "-");
    }

    // Keep human duration labels compact for CLI tables.
    #[test]
    fn formats_compact_durations() {
        assert_eq!(compact_duration(0), "0s");
        assert_eq!(compact_duration(45), "45s");
        assert_eq!(compact_duration(90), "1m 30s");
        assert_eq!(compact_duration(7_230), "2h");
        assert_eq!(compact_duration(9_000), "2h 30m");
        assert_eq!(compact_duration(97_200), "1d 3h");
        assert_eq!(compact_duration(1_555_200), "2w 4d");
    }
}
