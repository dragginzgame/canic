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
