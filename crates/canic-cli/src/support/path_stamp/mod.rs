//! Nanosecond display precision over Host Text's canonical UTC conversion.

use ic_host_text::timestamp::format_utc_timestamp_secs;

/// Render an observed Unix-nanosecond deadline without consulting the local clock.
pub fn utc_timestamp_ns(nanoseconds: u64) -> String {
    let seconds = format_utc_timestamp_secs(nanoseconds / 1_000_000_000);
    let fraction = nanoseconds % 1_000_000_000;
    format!("{}.{fraction:09}Z", seconds.trim_end_matches('Z'))
}
