use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, Memory, memory::VirtualMemory},
    storage::{prelude::*, stable::memory::placement::SHARDING_ACTIVE_SET_ID},
};
use std::cell::RefCell;

const PRESENT: u8 = 1;

//
// SHARDING_LIFECYCLE CORE
//

eager_static! {
    static SHARDING_LIFECYCLE: RefCell<ShardingLifecycleCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(ShardingLifecycleCore::new(
            BTreeMap::init(ic_memory!(ShardingActiveSet, SHARDING_ACTIVE_SET_ID)),
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
    pub(crate) fn active_shards() -> Vec<Principal> {
        Self::with(|core| core.active.iter().map(|entry| *entry.key()).collect())
    }

    // ---------------------------------------------------------------------
    // Mutations
    // ---------------------------------------------------------------------

    pub(crate) fn set_active(pid: Principal) {
        Self::with_mut(|core| {
            core.active.insert(pid, PRESENT);
        });
    }

    // ---------------------------------------------------------------------
    // Lifecycle
    // ---------------------------------------------------------------------

    #[cfg(test)]
    pub(crate) fn clear() {
        Self::with_mut(|core| {
            core.active.clear();
        });
    }
}

//
// ShardingLifecycleCore
//

pub struct ShardingLifecycleCore<M: Memory> {
    active: BTreeMap<Principal, u8, M>,
}

impl<M: Memory> ShardingLifecycleCore<M> {
    pub const fn new(active: BTreeMap<Principal, u8, M>) -> Self {
        Self { active }
    }
}
