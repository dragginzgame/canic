//! Module: ops::storage::placement::sharding_lifecycle
//!
//! Responsibility: expose deterministic sharding lifecycle active-set operations.
//! Does not own: shard placement policy, workflow orchestration, or endpoint DTOs.
//! Boundary: storage ops facade over stable sharding lifecycle state.

use crate::{cdk::types::Principal, storage::stable::sharding::lifecycle::ShardingLifecycle};

///
/// ShardingLifecycleOps
///

pub struct ShardingLifecycleOps;

impl ShardingLifecycleOps {
    #[must_use]
    pub fn active_shards() -> Vec<Principal> {
        ShardingLifecycle::active_shards()
    }

    pub fn set_active(pid: Principal) {
        ShardingLifecycle::set_active(pid);
    }

    #[cfg(test)]
    #[expect(dead_code)]
    pub(crate) fn clear_for_test() {
        ShardingLifecycle::clear();
    }
}
