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
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::truncate;

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
}
