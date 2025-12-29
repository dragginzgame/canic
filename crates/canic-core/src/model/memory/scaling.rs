use crate::{
    cdk::{
        structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
        types::BoundedString64,
    },
    eager_static, ic_memory,
    ids::CanisterRole,
    memory::impl_storable_bounded,
    model::memory::id::scaling::SCALING_REGISTRY_ID,
};
use candid::Principal;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

eager_static! {
    static SCALING_REGISTRY: RefCell<
        BTreeMap<Principal, WorkerEntry, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(ScalingRegistry, SCALING_REGISTRY_ID)),
    );
}

///
/// ScalingRegistryData
///

pub type ScalingRegistryData = Vec<(Principal, WorkerEntry)>;

///
/// ScalingRegistry
/// Registry of active scaling workers
///

pub struct ScalingRegistry;

impl ScalingRegistry {
    /// Insert or update a worker entry
    pub(crate) fn insert(pid: Principal, entry: WorkerEntry) {
        SCALING_REGISTRY.with_borrow_mut(|map| {
            map.insert(pid, entry);
        });
    }

    /// Lookup all workers in a given pool
    #[must_use]
    pub(crate) fn find_by_pool(pool: &str) -> ScalingRegistryData {
        SCALING_REGISTRY.with_borrow(|map| {
            map.iter()
                .filter(|e| e.value().pool.as_ref() == pool)
                .map(|e| (*e.key(), e.value()))
                .collect()
        })
    }

    /// Export full registry
    #[must_use]
    pub(crate) fn export() -> ScalingRegistryData {
        SCALING_REGISTRY.with_borrow(|map| map.iter().map(|e| (*e.key(), e.value())).collect())
    }
}

///
/// WorkerEntry
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkerEntry {
    pub pool: BoundedString64,       // which scale pool this belongs to
    pub canister_role: CanisterRole, // canister role
    pub created_at_secs: u64,        // timestamp
}

impl WorkerEntry {
    pub const STORABLE_MAX_SIZE: u32 = 160;
}

impl_storable_bounded!(WorkerEntry, WorkerEntry::STORABLE_MAX_SIZE, false);
