//! Module: storage::stable::sharding::lifecycle
//!
//! Responsibility: persist the active shard set in stable memory.
//! Does not own: sharding placement policy, workflow orchestration, or DTOs.
//! Boundary: stable-memory schema and mutation primitives for shard lifecycle state.

use crate::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
use crate::{
    cdk::structures::{DefaultMemoryImpl, Memory, memory::VirtualMemory},
    role_contract::allocation::memory::placement::SHARDING_ACTIVE_SET_ID,
    storage::{
        prelude::*,
        stable::sharding::{ShardingActiveSetData, ShardingActiveSetRecord},
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
            StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.sharding_active_set.v1", ty = ShardingActiveSet, id = SHARDING_ACTIVE_SET_ID)),
        ));
}

///
/// ShardingLifecycle
///
/// Stable storage accessor for the active shard set.
///

pub struct ShardingLifecycle;

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
    pub(crate) fn export() -> ShardingActiveSetData {
        ShardingActiveSetData {
            entries: Self::with(|core| {
                core.active
                    .iter()
                    .map(|entry| ShardingActiveSetRecord { pid: *entry.key() })
                    .collect()
            }),
        }
    }

    // ---------------------------------------------------------------------
    // Mutations
    // ---------------------------------------------------------------------

    pub(crate) fn set_active(pid: Principal) {
        Self::with_mut(|core| {
            core.active.insert(pid, PRESENT);
        });
    }
}

///
/// ShardingActiveSet
///
/// Stable-memory marker type for the active shard set memory region.
///

pub struct ShardingActiveSet;

//
// ShardingLifecycleCore
//

///
/// ShardingLifecycleCore
///
/// Stable-memory core containing active shard records.
///

pub struct ShardingLifecycleCore<M: Memory> {
    active: StableBtreeMap<Principal, u8, M>,
}

impl<M: Memory> ShardingLifecycleCore<M> {
    pub const fn new(active: StableBtreeMap<Principal, u8, M>) -> Self {
        Self { active }
    }
}
