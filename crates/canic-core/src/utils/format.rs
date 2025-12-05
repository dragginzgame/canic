//!
//! Small formatting helpers shared across logs and UI responses.
//!

///
/// Ellipsize a string in the middle when it exceeds the `threshold`.
///
/// Produces: first `head` chars, a Unicode ellipsis '…', then last `tail` chars.
/// Returns the original string if it does not exceed the threshold, or if the
/// requested head/tail slice would not shorten it.
///
#[must_use]
pub fn ellipsize_middle(s: &str, threshold: usize, head: usize, tail: usize) -> String {
    let len = s.chars().count();
    // Only ellipsize if strictly longer than threshold and we have space to shorten.
    if len > threshold && head + 1 + tail < len {
        let mut it = s.chars();
        let prefix: String = it.by_ref().take(head).collect();
        let suffix: String = s
            .chars()
            .rev()
            .take(tail)
            .collect::<String>()
            .chars()
            .rev()
            .collect();

        format!("{prefix}…{suffix}")
    } else {
        s.to_string()
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::ellipsize_middle;

    #[test]
    fn keeps_short_strings() {
        assert_eq!(ellipsize_middle("root", 9, 4, 4), "root");
        assert_eq!(ellipsize_middle("abcdefgh", 9, 4, 4), "abcdefgh");
        assert_eq!(ellipsize_middle("abcdefghi", 9, 4, 4), "abcdefghi");
    }

    #[test]
    fn ellipsizes_long_strings() {
        assert_eq!(ellipsize_middle("abcdefghijkl", 9, 4, 4), "abcd…ijkl");
        assert_eq!(
            ellipsize_middle("abcdefghijklmnopqrstuvwxyz", 9, 4, 4),
            "abcd…wxyz"
        );
    }
}
