//! Helpers for formatting raw instruction counts into friendly strings.

/// Format an instruction count using engineering suffixes (K/M/B/T).
#[must_use]
#[expect(clippy::cast_precision_loss)]
pub fn format_instructions(n: u64) -> String {
    const TABLE: &[(u64, &str)] = &[
        (1_000_000_000_000, "T"),
        (1_000_000_000, "B"),
        (1_000_000, "M"),
        (1_000, "K"),
    ];

    for &(div, suffix) in TABLE {
        if n >= div {
            return format!("{:.2}{}", n as f64 / div as f64, suffix);
        }
    }

    n.to_string()
}
