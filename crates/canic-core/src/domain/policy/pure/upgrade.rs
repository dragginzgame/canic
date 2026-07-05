///
/// UpgradePlan
///

#[derive(Clone, Copy, Debug)]
pub struct UpgradePlan {
    pub should_upgrade: bool,
}

///
/// plan_upgrade
/// Decide whether a canister should be upgraded based on module hashes.
///

#[must_use]
pub fn plan_upgrade(current_hash: Option<Vec<u8>>, target_hash: Vec<u8>) -> UpgradePlan {
    UpgradePlan {
        should_upgrade: current_hash != Some(target_hash),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_target_hash_makes_repeated_upgrade_a_noop() {
        let target_hash = vec![1, 2, 3, 4];
        let plan = plan_upgrade(Some(target_hash.clone()), target_hash);

        assert!(!plan.should_upgrade);
    }

    #[test]
    fn missing_or_different_hash_requires_upgrade() {
        let target_hash = vec![1, 2, 3, 4];

        assert!(plan_upgrade(None, target_hash.clone()).should_upgrade);
        assert!(plan_upgrade(Some(vec![4, 3, 2, 1]), target_hash).should_upgrade);
    }
}
