use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    icu_eager_static, icu_memory, impl_storable_bounded,
    memory::id::capability::ELASTIC_REGISTRY_ID,
    types::CanisterType,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// ELASTIC REGISTRY
//

icu_eager_static! {
    static ELASTIC_REGISTRY: RefCell<
        BTreeMap<Principal, ElasticEntry, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(icu_memory!(ElasticRegistry, ELASTIC_REGISTRY_ID)),
    );
}

///
/// ElasticRegistryError
///

#[derive(Debug, ThisError)]
pub enum ElasticRegistryError {
    #[error("shard pool not found")]
    PoolNotFound,

    #[error("worker not found: {0}")]
    WorkerNotFound(Principal),
}

///
/// ElasticEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ElasticEntry {
    pub pool: String,                // which elastic pool this belongs to
    pub canister_type: CanisterType, // canister type
    pub created_at_secs: u64,        // timestamp
}

impl ElasticEntry {
    pub const STORABLE_MAX_SIZE: u32 = 128;
}

impl_storable_bounded!(ElasticEntry, ElasticEntry::STORABLE_MAX_SIZE, false);

///
/// ElasticRegistry
/// Registry of active elastic workers
///

#[derive(Clone, Copy, Debug, Default)]
pub struct ElasticRegistry;

pub type ElasticRegistryView = Vec<(Principal, ElasticEntry)>;

impl ElasticRegistry {
    /// Insert or update a worker entry
    pub fn insert(pid: Principal, entry: ElasticEntry) {
        ELASTIC_REGISTRY.with_borrow_mut(|map| {
            map.insert(pid, entry);
        });
    }

    /// Remove a worker by PID
    pub fn remove(pid: &Principal) -> Result<(), ElasticRegistryError> {
        ELASTIC_REGISTRY.with_borrow_mut(|map| {
            map.remove(pid)
                .ok_or(ElasticRegistryError::WorkerNotFound(*pid))?;
            Ok(())
        })
    }

    /// Lookup a worker by PID
    #[must_use]
    pub fn find_by_pid(pid: &Principal) -> Option<ElasticEntry> {
        ELASTIC_REGISTRY.with_borrow(|map| map.get(pid))
    }

    /// Lookup all workers in a given pool
    #[must_use]
    pub fn find_by_pool(pool: &str) -> Vec<(Principal, ElasticEntry)> {
        ELASTIC_REGISTRY.with_borrow(|map| {
            map.iter()
                .filter(|e| e.value().pool == pool)
                .map(|e| (*e.key(), e.value()))
                .collect()
        })
    }

    /// Export full registry
    #[must_use]
    pub fn export() -> Vec<(Principal, ElasticEntry)> {
        ELASTIC_REGISTRY.with_borrow(|map| map.iter().map(|e| (*e.key(), e.value())).collect())
    }

    /// Clear registry
    pub fn clear() {
        ELASTIC_REGISTRY.with_borrow_mut(BTreeMap::clear);
    }
}
