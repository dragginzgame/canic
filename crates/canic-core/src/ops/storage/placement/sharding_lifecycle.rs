use crate::{
    cdk::types::Principal, ids::ShardLifecycleState,
    storage::stable::sharding::lifecycle::ShardingLifecycle,
};

///
/// ShardingLifecycleOps
///

pub struct ShardingLifecycleOps;

impl ShardingLifecycleOps {
    #[must_use]
    pub fn state(pid: Principal) -> Option<ShardLifecycleState> {
        ShardingLifecycle::state(&pid)
    }

    pub fn set_state(pid: Principal, state: ShardLifecycleState) {
        ShardingLifecycle::set_state(pid, state);
    }

    #[must_use]
    pub fn active_shards() -> Vec<Principal> {
        ShardingLifecycle::active_shards()
    }

    pub fn set_active(pid: Principal) {
        ShardingLifecycle::set_active(pid);
    }

    pub fn clear_active(pid: Principal) {
        ShardingLifecycle::clear_active(&pid);
    }

    #[must_use]
    pub fn rotation_targets() -> Vec<Principal> {
        ShardingLifecycle::rotation_targets()
    }

    pub fn set_rotation_target(pid: Principal) {
        ShardingLifecycle::set_rotation_target(pid);
    }

    pub fn clear_rotation_target(pid: Principal) {
        ShardingLifecycle::clear_rotation_target(&pid);
    }

    #[cfg(test)]
    pub(crate) fn clear_for_test() {
        ShardingLifecycle::clear();
    }
}
