//!
//! Small formatting helpers shared across logs and UI responses.
//!

///
/// Truncate a string to at most `max_chars` Unicode scalar values.
///
/// Returns the original string when it already fits.
///
#[must_use]
pub fn truncate(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();

    if chars.next().is_some() {
        truncated
    } else {
        s.to_string()
    }
}

///
/// Format a byte size using IEC units with two decimal places.
///
/// Examples: `512.00 B`, `720.79 KiB`, `13.61 MiB`.
///
#[must_use]
#[expect(clippy::cast_precision_loss)]
pub fn byte_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    let mut value = bytes as f64;
    let mut unit_index = 0usize;

    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    format!("{value:.2} {}", UNITS[unit_index])
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::{byte_size, truncate};

    #[test]
    fn keeps_short_strings() {
        assert_eq!(truncate("root", 9), "root");
        assert_eq!(truncate("abcdefgh", 9), "abcdefgh");
        assert_eq!(truncate("abcdefghi", 9), "abcdefghi");
    }

    #[test]
    fn truncates_long_strings() {
        assert_eq!(truncate("abcdefghijkl", 9), "abcdefghi");
        assert_eq!(truncate("abcdefghijklmnopqrstuvwxyz", 9), "abcdefghi");
    }

    #[test]
    fn formats_small_byte_sizes() {
        assert_eq!(byte_size(0), "0.00 B");
        assert_eq!(byte_size(512), "512.00 B");
        assert_eq!(byte_size(1024), "1.00 KiB");
    }

    #[test]
    fn formats_larger_byte_sizes() {
        assert_eq!(byte_size(720_795), "703.90 KiB");
        assert_eq!(byte_size(13_936_529), "13.29 MiB");
        assert_eq!(byte_size(9_102_643), "8.68 MiB");
    }
}
