/// Decide whether a canister should be upgraded based on module hashes.

#[must_use]
pub fn should_upgrade(current_hash: Option<&[u8]>, target_hash: &[u8]) -> bool {
    current_hash != Some(target_hash)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_target_hash_makes_repeated_upgrade_a_noop() {
        let target_hash = vec![1, 2, 3, 4];

        assert!(!should_upgrade(Some(target_hash.as_slice()), &target_hash));
    }

    #[test]
    fn missing_or_different_hash_requires_upgrade() {
        let target_hash = vec![1, 2, 3, 4];

        assert!(should_upgrade(None, &target_hash));
        assert!(should_upgrade(Some(&[4, 3, 2, 1]), &target_hash));
    }
}
