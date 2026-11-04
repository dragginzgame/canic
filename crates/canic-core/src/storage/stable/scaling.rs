use crate::{
    cdk::{
        structures::{DefaultMemoryImpl, StableBTreeMap as StableBtreeMap},
        types::BoundedString64,
    },
    impl_storable_bounded,
};
use canic_contracts::ids::CanisterRole;
use ic_platform::principal::ExternalPrincipal as Principal;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

std::thread_local! {
    static SCALING_REGISTRY: RefCell<
        StableBtreeMap<crate::cdk::types::PrincipalKeyRecord, WorkerEntryRecord, DefaultMemoryImpl>
    > = RefCell::new(
        {
            let allocation = crate::memory::open(crate::memory::keys::PLACEMENT_SCALING_REGISTRY_V1);
            if ic_storage::Memory::size(&allocation) == 0 {
                StableBtreeMap::create(allocation).expect("open admitted stable collection")
            } else {
                StableBtreeMap::load(allocation).expect("open admitted stable collection")
            }
        },
    );
}

///
/// ScalingRegistryEntryRecord
///
/// One logical scaling-registry snapshot row.
///

#[derive(Clone, Debug)]
pub struct ScalingRegistryEntryRecord {
    pub pid: Principal,
    pub entry: WorkerEntryRecord,
}

impl ScalingRegistryEntryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "ScalingRegistryEntryRecord";
}

///
/// ScalingRegistryData
///
/// Canonical scaling-registry export snapshot.
///

#[derive(Clone, Debug)]
pub struct ScalingRegistryData {
    pub entries: Vec<ScalingRegistryEntryRecord>,
}

impl ScalingRegistryData {
    pub const STATE_CONTRACT_NAME: &'static str = "ScalingRegistryData";
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
            map.insert(pid.into(), entry)
                .expect("Storage operation failed");
        });
    }

    /// Export full registry
    #[must_use]
    pub(crate) fn export() -> ScalingRegistryData {
        ScalingRegistryData {
            entries: SCALING_REGISTRY.with_borrow(|map| {
                map.iter()
                    .map(|entry| entry.expect("Storage iteration failed"))
                    .map(|entry| ScalingRegistryEntryRecord {
                        pid: entry.key().principal(),
                        entry: entry.value().expect("decode Storage entry"),
                    })
                    .collect()
            }),
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
    pub allocation_operation_id: [u8; 32],
    pub created_at_secs: u64, // timestamp
}

impl WorkerEntryRecord {
    pub const STORABLE_MAX_SIZE: u32 = 256;
}

impl_storable_bounded!(
    WorkerEntryRecord,
    WorkerEntryRecord::STORABLE_MAX_SIZE,
    false
);
