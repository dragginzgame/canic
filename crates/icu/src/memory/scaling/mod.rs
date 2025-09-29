use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    icu_eager_static, icu_memory, impl_storable_bounded,
    memory::id::capability::SCALING_REGISTRY_ID,
    types::CanisterType,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// SCALING REGISTRY
//

icu_eager_static! {
    static SCALING_REGISTRY: RefCell<
        BTreeMap<Principal, WorkerEntry, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(icu_memory!(ScalingRegistry, SCALING_REGISTRY_ID)),
    );
}

///
/// ScalingRegistryError
///

#[derive(Debug, ThisError)]
pub enum ScalingRegistryError {
    #[error("scaling pool not found")]
    PoolNotFound,

    #[error("worker not found: {0}")]
    WorkerNotFound(Principal),
}

///
/// WorkerEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkerEntry {
    pub pool: String,                // which scale pool this belongs to
    pub canister_type: CanisterType, // canister type
    pub created_at_secs: u64,        // timestamp
}

impl WorkerEntry {
    pub const STORABLE_MAX_SIZE: u32 = 128;
}

impl_storable_bounded!(WorkerEntry, WorkerEntry::STORABLE_MAX_SIZE, false);

///
/// ScalingRegistry
/// Registry of active scaling workers
///

#[derive(Clone, Copy, Debug, Default)]
pub struct ScalingRegistry;

pub type ScalingRegistryView = Vec<(Principal, WorkerEntry)>;

impl ScalingRegistry {
    /// Insert or update a worker entry
    pub fn insert(pid: Principal, entry: WorkerEntry) {
        SCALING_REGISTRY.with_borrow_mut(|map| {
            map.insert(pid, entry);
        });
    }

    /// Remove a worker by PID
    pub fn remove(pid: &Principal) -> Result<(), ScalingRegistryError> {
        SCALING_REGISTRY.with_borrow_mut(|map| {
            map.remove(pid)
                .ok_or(ScalingRegistryError::WorkerNotFound(*pid))?;
            Ok(())
        })
    }

    /// Lookup a worker by PID
    #[must_use]
    pub fn find_by_pid(pid: &Principal) -> Option<WorkerEntry> {
        SCALING_REGISTRY.with_borrow(|map| map.get(pid))
    }

    /// Lookup all workers in a given pool
    #[must_use]
    pub fn find_by_pool(pool: &str) -> Vec<(Principal, WorkerEntry)> {
        SCALING_REGISTRY.with_borrow(|map| {
            map.iter()
                .filter(|e| e.value().pool == pool)
                .map(|e| (*e.key(), e.value()))
                .collect()
        })
    }

    /// Export full registry
    #[must_use]
    pub fn export() -> Vec<(Principal, WorkerEntry)> {
        SCALING_REGISTRY.with_borrow(|map| map.iter().map(|e| (*e.key(), e.value())).collect())
    }

    /// Clear registry
    pub fn clear() {
        SCALING_REGISTRY.with_borrow_mut(BTreeMap::clear);
    }
}
