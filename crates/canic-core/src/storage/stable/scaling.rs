use crate::impl_storable_bounded;
use crate::{
    cdk::{
        candid::Principal,
        structures::{DefaultMemoryImpl, memory::VirtualMemory},
        types::BoundedString64,
    },
    eager_static,
    ids::CanisterRole,
    storage::stable::memory::placement::SCALING_REGISTRY_ID,
};
use ic_memory::stable_structures::btreemap::BTreeMap as StableBtreeMap;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

eager_static! {
    static SCALING_REGISTRY: RefCell<
        StableBtreeMap<Principal, WorkerEntryRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!("canic.core.scaling_registry.v1", ScalingRegistry, SCALING_REGISTRY_ID)),
    );
}

///
/// ScalingRegistryRecord
///

#[derive(Clone, Debug)]
pub struct ScalingRegistryRecord {
    pub entries: Vec<(Principal, WorkerEntryRecord)>,
}

///
/// ScalingRegistry
/// Registry of active scaling workers
///

pub struct ScalingRegistry;

impl ScalingRegistry {
    /// Insert or update a worker entry
    pub(crate) fn upsert(pid: Principal, entry: WorkerEntryRecord) {
        SCALING_REGISTRY.with_borrow_mut(|map| {
            map.insert(pid, entry);
        });
    }

    /// Count worker entries for one pool.
    #[must_use]
    #[expect(clippy::cast_possible_truncation)]
    pub(crate) fn count_by_pool(pool: &str) -> u32 {
        SCALING_REGISTRY.with_borrow(|map| {
            map.iter()
                .filter(|entry| entry.value().pool.as_ref() == pool)
                .count() as u32
        })
    }

    /// Export full registry
    #[must_use]
    pub(crate) fn export() -> ScalingRegistryRecord {
        ScalingRegistryRecord {
            entries: SCALING_REGISTRY
                .with_borrow(|map| map.iter().map(|e| (*e.key(), e.value())).collect()),
        }
    }
}

///
/// WorkerEntryRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkerEntryRecord {
    pub pool: BoundedString64,       // which scale pool this belongs to
    pub canister_role: CanisterRole, // canister role
    pub created_at_secs: u64,        // timestamp
}

impl WorkerEntryRecord {
    pub const STORABLE_MAX_SIZE: u32 = 160;
}

impl_storable_bounded!(
    WorkerEntryRecord,
    WorkerEntryRecord::STORABLE_MAX_SIZE,
    false
);
