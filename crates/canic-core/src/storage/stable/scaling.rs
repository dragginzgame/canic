use crate::{
    cdk::{
        candid::Principal,
        structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
        types::BoundedString64,
    },
    eager_static, ic_memory,
    ids::CanisterRole,
    memory::impl_storable_bounded,
    storage::stable::memory::placement::SCALING_REGISTRY_ID,
};
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

#[derive(Clone, Debug)]
pub struct ScalingRegistryData {
    pub entries: Vec<(Principal, WorkerEntry)>,
}

///
/// ScalingRegistry
/// Registry of active scaling workers
///

pub struct ScalingRegistry;

impl ScalingRegistry {
    /// Insert or update a worker entry
    pub(crate) fn upsert(pid: Principal, entry: WorkerEntry) {
        SCALING_REGISTRY.with_borrow_mut(|map| {
            map.insert(pid, entry);
        });
    }

    /// Export full registry
    #[must_use]
    pub(crate) fn export() -> ScalingRegistryData {
        ScalingRegistryData {
            entries: SCALING_REGISTRY
                .with_borrow(|map| map.iter().map(|e| (*e.key(), e.value())).collect()),
        }
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
