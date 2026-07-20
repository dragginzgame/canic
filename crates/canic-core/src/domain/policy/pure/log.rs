/// Return the strict age cutoff used by runtime-log retention.
///
/// Entries created before the cutoff are expired. An entry remains available
/// through the full configured age second.
#[must_use]
pub const fn age_cutoff(now_secs: u64, max_age_secs: u64) -> u64 {
    now_secs.saturating_sub(max_age_secs)
}

/// Return the first whole second at which an entry is older than the limit.
#[must_use]
pub const fn age_expiry_at(created_at: u64, max_age_secs: u64) -> Option<u64> {
    match created_at.checked_add(max_age_secs) {
        Some(last_retained_at) => last_retained_at.checked_add(1),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn age_expiry_preserves_the_existing_strict_cutoff_boundary() {
        assert_eq!(age_expiry_at(10, 5), Some(16));
        assert_eq!(age_cutoff(15, 5), 10);
        assert_eq!(age_cutoff(16, 5), 11);
    }

    #[test]
    fn unreachable_age_expiry_is_not_wrapped() {
        assert_eq!(age_expiry_at(u64::MAX, 0), None);
        assert_eq!(age_expiry_at(1, u64::MAX), None);
    }
}
