use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    ids::ShardLifecycleState,
    storage::{
        prelude::*,
        stable::memory::placement::{
            SHARDING_ACTIVE_SET_ID, SHARDING_LIFECYCLE_ID, SHARDING_ROTATION_TARGETS_ID,
        },
    },
};
use std::cell::RefCell;

const PRESENT: u8 = 1;

//
// SHARDING_LIFECYCLE CORE
//

eager_static! {
    static SHARDING_LIFECYCLE: RefCell<ShardingLifecycleCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(ShardingLifecycleCore::new(
            BTreeMap::init(ic_memory!(ShardingLifecycle, SHARDING_LIFECYCLE_ID)),
            BTreeMap::init(ic_memory!(ShardingActiveSet, SHARDING_ACTIVE_SET_ID)),
            BTreeMap::init(ic_memory!(ShardingRotationTargets, SHARDING_ROTATION_TARGETS_ID)),
        ));
}

///
/// ShardingLifecycle
///

pub struct ShardingLifecycle;

///
/// ShardingActiveSet
///

pub struct ShardingActiveSet;

///
/// ShardingRotationTargets
///

pub struct ShardingRotationTargets;

impl ShardingLifecycle {
    pub(crate) fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&ShardingLifecycleCore<VirtualMemory<DefaultMemoryImpl>>) -> R,
    {
        SHARDING_LIFECYCLE.with_borrow(f)
    }

    pub(crate) fn with_mut<F, R>(f: F) -> R
    where
        F: FnOnce(&mut ShardingLifecycleCore<VirtualMemory<DefaultMemoryImpl>>) -> R,
    {
        SHARDING_LIFECYCLE.with_borrow_mut(f)
    }

    // ---------------------------------------------------------------------
    // Queries
    // ---------------------------------------------------------------------

    #[must_use]
    pub(crate) fn state(pid: &Principal) -> Option<ShardLifecycleState> {
        Self::with(|core| core.lifecycle.get(pid))
    }

    #[must_use]
    pub(crate) fn active_shards() -> Vec<Principal> {
        Self::with(|core| core.active.iter().map(|entry| *entry.key()).collect())
    }

    #[must_use]
    pub(crate) fn rotation_targets() -> Vec<Principal> {
        Self::with(|core| {
            core.rotation_targets
                .iter()
                .map(|entry| *entry.key())
                .collect()
        })
    }

    // ---------------------------------------------------------------------
    // Mutations
    // ---------------------------------------------------------------------

    pub(crate) fn set_state(pid: Principal, state: ShardLifecycleState) {
        Self::with_mut(|core| {
            core.lifecycle.insert(pid, state);
        });
    }

    pub(crate) fn set_active(pid: Principal) {
        Self::with_mut(|core| {
            core.active.insert(pid, PRESENT);
        });
    }

    pub(crate) fn clear_active(pid: &Principal) {
        Self::with_mut(|core| {
            core.active.remove(pid);
        });
    }

    pub(crate) fn set_rotation_target(pid: Principal) {
        Self::with_mut(|core| {
            core.rotation_targets.insert(pid, PRESENT);
        });
    }

    pub(crate) fn clear_rotation_target(pid: &Principal) {
        Self::with_mut(|core| {
            core.rotation_targets.remove(pid);
        });
    }

    // ---------------------------------------------------------------------
    // Lifecycle
    // ---------------------------------------------------------------------

    #[cfg(test)]
    pub(crate) fn clear() {
        Self::with_mut(|core| {
            core.lifecycle.clear();
            core.active.clear();
            core.rotation_targets.clear();
        });
    }
}

//
// ShardingLifecycleCore
//

pub struct ShardingLifecycleCore<M: Memory> {
    lifecycle: BTreeMap<Principal, ShardLifecycleState, M>,
    active: BTreeMap<Principal, u8, M>,
    rotation_targets: BTreeMap<Principal, u8, M>,
}

impl<M: Memory> ShardingLifecycleCore<M> {
    pub const fn new(
        lifecycle: BTreeMap<Principal, ShardLifecycleState, M>,
        active: BTreeMap<Principal, u8, M>,
        rotation_targets: BTreeMap<Principal, u8, M>,
    ) -> Self {
        Self {
            lifecycle,
            active,
            rotation_targets,
        }
    }
}
