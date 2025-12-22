use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    eager_static, ic_memory,
    ids::CanisterRole,
    memory::impl_storable_bounded,
    model::memory::id::scaling::SCALING_REGISTRY_ID,
    types::BoundedString64,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

//
// SCALING REGISTRY
//

eager_static! {
    static SCALING_REGISTRY: RefCell<
        BTreeMap<Principal, WorkerEntry, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(ScalingRegistry, SCALING_REGISTRY_ID)),
    );
}

///
/// WorkerEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkerEntry {
    pub pool: BoundedString64,       // which scale pool this belongs to
    pub canister_role: CanisterRole, // canister role
    pub created_at_secs: u64,        // timestamp
}

impl WorkerEntry {
    pub const STORABLE_MAX_SIZE: u32 = 160;

    pub(crate) fn try_new(
        pool: &str,
        canister_role: CanisterRole,
        created_at_secs: u64,
    ) -> Result<Self, String> {
        let pool = BoundedString64::try_new(pool).map_err(|err| format!("pool name: {err}"))?;

        Ok(Self {
            pool,
            canister_role,
            created_at_secs,
        })
    }
}

impl_storable_bounded!(WorkerEntry, WorkerEntry::STORABLE_MAX_SIZE, false);

///
/// ScalingRegistryView
///

pub type ScalingRegistryView = Vec<(Principal, WorkerEntry)>;

///
/// ScalingRegistry
/// Registry of active scaling workers
///

#[derive(Clone, Copy, Debug, Default)]
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
    pub(crate) fn find_by_pool(pool: &str) -> Vec<(Principal, WorkerEntry)> {
        SCALING_REGISTRY.with_borrow(|map| {
            map.iter()
                .filter(|e| e.value().pool.as_ref() == pool)
                .map(|e| (*e.key(), e.value()))
                .collect()
        })
    }

    /// Export full registry
    #[must_use]
    pub(crate) fn export() -> Vec<(Principal, WorkerEntry)> {
        SCALING_REGISTRY.with_borrow(|map| map.iter().map(|e| (*e.key(), e.value())).collect())
    }
}
