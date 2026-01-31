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
    pub(crate) fn clear_for_test() {
        ShardingLifecycle::clear();
    }
}
