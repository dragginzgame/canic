//! Module: role_contract::allocation
//!
//! Responsibility: own canonical Canic stable-memory IDs and allocation definitions.
//! Does not own: stable records, descriptor metadata, migrations, or role selection.
//! Boundary: runtime storage imports IDs; pure role policy selects allocation keys.

use crate::role_contract::model::{
    AllocationDefinition, AllocationLifecycle, AllocationOwner, MemoryId, RoleContractFinding,
    StateAllocationKey,
};
use std::collections::{BTreeMap, BTreeSet};

pub const CANIC_CORE_MIN_ID: u8 = 11;
pub const CANIC_CORE_MAX_ID: u8 = 79;
pub const CANIC_CONTROL_PLANE_MIN_ID: u8 = 80;
pub const CANIC_CONTROL_PLANE_MAX_ID: u8 = 85;

/// Canonical stable-memory IDs grouped by record owner.
pub mod memory {
    pub mod topology {
        pub const CANISTER_CHILDREN_ID: u8 = 11;
        pub const APP_INDEX_ID: u8 = 12;
        pub const SUBNET_INDEX_ID: u8 = 13;
        pub const APP_REGISTRY_ID: u8 = 14;
        pub const SUBNET_REGISTRY_ID: u8 = 15;
    }

    pub mod env {
        pub const ENV_ID: u8 = 16;
        pub const SUBNET_STATE_ID: u8 = 17;
        pub const APP_STATE_ID: u8 = 18;
    }

    pub mod auth {
        pub const AUTH_STATE_ID: u8 = 19;
        pub const ROOT_REPLAY_ID: u8 = 20;
        pub const REPLAY_RECEIPTS_ID: u8 = 21;
    }

    pub mod observability {
        pub const CYCLE_TRACKER_ID: u8 = 29;
        pub const CYCLE_TOPUP_EVENTS_ID: u8 = 30;
        pub const LOG_INDEX_ID: u8 = 31;
        pub const LOG_DATA_ID: u8 = 32;
        pub const ICP_REFILL_RECORDS_ID: u8 = 33;
        pub const CYCLES_FUNDING_LEDGER_ID: u8 = 34;
    }

    pub mod intent {
        pub const INTENT_META_ID: u8 = 39;
        pub const INTENT_RECORDS_ID: u8 = 40;
        pub const INTENT_TOTALS_ID: u8 = 41;
        pub const INTENT_PENDING_ID: u8 = 42;
    }

    pub mod pool {
        pub const CANISTER_POOL_ID: u8 = 49;
    }

    pub mod placement {
        pub const SCALING_REGISTRY_ID: u8 = 52;
        pub const SHARDING_REGISTRY_ID: u8 = 53;
        pub const SHARDING_ASSIGNMENT_ID: u8 = 54;
        pub const DIRECTORY_REGISTRY_ID: u8 = 55;
        pub const SHARDING_ACTIVE_SET_ID: u8 = 56;
    }

    pub mod blob_storage {
        pub const STORED_BLOBS_ID: u8 = 62;
        pub const BLOB_DELETION_PENDING_ID: u8 = 63;
        pub const STORAGE_GATEWAY_PRINCIPALS_ID: u8 = 64;
        pub const BLOB_STORAGE_BILLING_ID: u8 = 65;
    }

    pub mod template {
        pub const TEMPLATE_MANIFESTS_ID: u8 = 80;
        pub const TEMPLATE_CHUNK_SETS_ID: u8 = 81;
        pub const TEMPLATE_CHUNK_REFS_ID: u8 = 82;
        pub const TEMPLATE_CHUNK_PAYLOADS_ID: u8 = 83;
        pub const CONTROL_PLANE_SUBNET_STATE_ID: u8 = 84;
        pub const WASM_STORE_GC_STATE_ID: u8 = 85;
    }
}

use memory::{
    auth::{AUTH_STATE_ID, REPLAY_RECEIPTS_ID, ROOT_REPLAY_ID},
    blob_storage::{
        BLOB_DELETION_PENDING_ID, BLOB_STORAGE_BILLING_ID, STORAGE_GATEWAY_PRINCIPALS_ID,
        STORED_BLOBS_ID,
    },
    env::{APP_STATE_ID, ENV_ID, SUBNET_STATE_ID},
    intent::{INTENT_META_ID, INTENT_PENDING_ID, INTENT_RECORDS_ID, INTENT_TOTALS_ID},
    observability::{
        CYCLE_TOPUP_EVENTS_ID, CYCLE_TRACKER_ID, CYCLES_FUNDING_LEDGER_ID, ICP_REFILL_RECORDS_ID,
        LOG_DATA_ID, LOG_INDEX_ID,
    },
    placement::{
        DIRECTORY_REGISTRY_ID, SCALING_REGISTRY_ID, SHARDING_ACTIVE_SET_ID, SHARDING_ASSIGNMENT_ID,
        SHARDING_REGISTRY_ID,
    },
    pool::CANISTER_POOL_ID,
    template::{
        CONTROL_PLANE_SUBNET_STATE_ID, TEMPLATE_CHUNK_PAYLOADS_ID, TEMPLATE_CHUNK_REFS_ID,
        TEMPLATE_CHUNK_SETS_ID, TEMPLATE_MANIFESTS_ID, WASM_STORE_GC_STATE_ID,
    },
    topology::{
        APP_INDEX_ID, APP_REGISTRY_ID, CANISTER_CHILDREN_ID, SUBNET_INDEX_ID, SUBNET_REGISTRY_ID,
    },
};

const CORE_ROOT_TOPOLOGY_IDS: &[MemoryId] = &[
    MemoryId::new(CANISTER_CHILDREN_ID),
    MemoryId::new(APP_INDEX_ID),
    MemoryId::new(SUBNET_INDEX_ID),
    MemoryId::new(APP_REGISTRY_ID),
    MemoryId::new(SUBNET_REGISTRY_ID),
];
const CORE_ROOT_ENVIRONMENT_IDS: &[MemoryId] = &[
    MemoryId::new(ENV_ID),
    MemoryId::new(SUBNET_STATE_ID),
    MemoryId::new(APP_STATE_ID),
];
const CORE_ROOT_AUTH_IDS: &[MemoryId] = &[
    MemoryId::new(AUTH_STATE_ID),
    MemoryId::new(REPLAY_RECEIPTS_ID),
];
const RETIRED_ROOT_REPLAY_IDS: &[MemoryId] = &[MemoryId::new(ROOT_REPLAY_ID)];
const CORE_ROOT_OBSERVABILITY_IDS: &[MemoryId] = &[
    MemoryId::new(CYCLE_TRACKER_ID),
    MemoryId::new(CYCLE_TOPUP_EVENTS_ID),
    MemoryId::new(LOG_INDEX_ID),
    MemoryId::new(LOG_DATA_ID),
    MemoryId::new(ICP_REFILL_RECORDS_ID),
    MemoryId::new(CYCLES_FUNDING_LEDGER_ID),
];
const CORE_ROOT_INTENT_IDS: &[MemoryId] = &[
    MemoryId::new(INTENT_META_ID),
    MemoryId::new(INTENT_RECORDS_ID),
    MemoryId::new(INTENT_TOTALS_ID),
    MemoryId::new(INTENT_PENDING_ID),
];
const CORE_ROOT_CAPACITY_IDS: &[MemoryId] = &[
    MemoryId::new(CANISTER_POOL_ID),
    MemoryId::new(SCALING_REGISTRY_ID),
    MemoryId::new(DIRECTORY_REGISTRY_ID),
];
const SHARDING_REGISTRY_IDS: &[MemoryId] = &[MemoryId::new(SHARDING_REGISTRY_ID)];
const SHARDING_ASSIGNMENT_IDS: &[MemoryId] = &[MemoryId::new(SHARDING_ASSIGNMENT_ID)];
const SHARDING_ACTIVE_SET_IDS: &[MemoryId] = &[MemoryId::new(SHARDING_ACTIVE_SET_ID)];
const STORED_BLOBS_IDS: &[MemoryId] = &[MemoryId::new(STORED_BLOBS_ID)];
const BLOB_DELETION_PENDING_IDS: &[MemoryId] = &[MemoryId::new(BLOB_DELETION_PENDING_ID)];
const STORAGE_GATEWAY_PRINCIPALS_IDS: &[MemoryId] = &[MemoryId::new(STORAGE_GATEWAY_PRINCIPALS_ID)];
const BLOB_STORAGE_BILLING_IDS: &[MemoryId] = &[MemoryId::new(BLOB_STORAGE_BILLING_ID)];
const TEMPLATE_MANIFESTS_IDS: &[MemoryId] = &[MemoryId::new(TEMPLATE_MANIFESTS_ID)];
const TEMPLATE_CHUNK_SETS_IDS: &[MemoryId] = &[MemoryId::new(TEMPLATE_CHUNK_SETS_ID)];
const TEMPLATE_CHUNK_REFS_IDS: &[MemoryId] = &[MemoryId::new(TEMPLATE_CHUNK_REFS_ID)];
const TEMPLATE_CHUNK_PAYLOADS_IDS: &[MemoryId] = &[MemoryId::new(TEMPLATE_CHUNK_PAYLOADS_ID)];
const CONTROL_PLANE_SUBNET_STATE_IDS: &[MemoryId] = &[MemoryId::new(CONTROL_PLANE_SUBNET_STATE_ID)];
const WASM_STORE_GC_STATE_IDS: &[MemoryId] = &[MemoryId::new(WASM_STORE_GC_STATE_ID)];

const ALLOCATION_DEFINITIONS: &[AllocationDefinition] = &[
    definition(
        StateAllocationKey::CoreRootTopology,
        AllocationOwner::CanicCore,
        AllocationLifecycle::Active,
        CORE_ROOT_TOPOLOGY_IDS,
    ),
    definition(
        StateAllocationKey::CoreRootEnvironment,
        AllocationOwner::CanicCore,
        AllocationLifecycle::Active,
        CORE_ROOT_ENVIRONMENT_IDS,
    ),
    definition(
        StateAllocationKey::CoreRootAuth,
        AllocationOwner::CanicCore,
        AllocationLifecycle::Active,
        CORE_ROOT_AUTH_IDS,
    ),
    definition(
        StateAllocationKey::RetiredRootReplay,
        AllocationOwner::CanicCore,
        AllocationLifecycle::RetiredNeverReuse,
        RETIRED_ROOT_REPLAY_IDS,
    ),
    definition(
        StateAllocationKey::CoreRootObservability,
        AllocationOwner::CanicCore,
        AllocationLifecycle::Active,
        CORE_ROOT_OBSERVABILITY_IDS,
    ),
    definition(
        StateAllocationKey::CoreRootIntent,
        AllocationOwner::CanicCore,
        AllocationLifecycle::Active,
        CORE_ROOT_INTENT_IDS,
    ),
    definition(
        StateAllocationKey::CoreRootCapacity,
        AllocationOwner::CanicCore,
        AllocationLifecycle::Active,
        CORE_ROOT_CAPACITY_IDS,
    ),
    definition(
        StateAllocationKey::ShardingRegistry,
        AllocationOwner::CanicCore,
        AllocationLifecycle::Active,
        SHARDING_REGISTRY_IDS,
    ),
    definition(
        StateAllocationKey::ShardingAssignments,
        AllocationOwner::CanicCore,
        AllocationLifecycle::Active,
        SHARDING_ASSIGNMENT_IDS,
    ),
    definition(
        StateAllocationKey::ShardingActiveSet,
        AllocationOwner::CanicCore,
        AllocationLifecycle::Active,
        SHARDING_ACTIVE_SET_IDS,
    ),
    definition(
        StateAllocationKey::StoredBlobs,
        AllocationOwner::CanicCore,
        AllocationLifecycle::Active,
        STORED_BLOBS_IDS,
    ),
    definition(
        StateAllocationKey::BlobDeletionPending,
        AllocationOwner::CanicCore,
        AllocationLifecycle::Active,
        BLOB_DELETION_PENDING_IDS,
    ),
    definition(
        StateAllocationKey::StorageGatewayPrincipals,
        AllocationOwner::CanicCore,
        AllocationLifecycle::Active,
        STORAGE_GATEWAY_PRINCIPALS_IDS,
    ),
    definition(
        StateAllocationKey::BlobStorageBilling,
        AllocationOwner::CanicCore,
        AllocationLifecycle::Active,
        BLOB_STORAGE_BILLING_IDS,
    ),
    definition(
        StateAllocationKey::TemplateManifests,
        AllocationOwner::CanicControlPlane,
        AllocationLifecycle::Active,
        TEMPLATE_MANIFESTS_IDS,
    ),
    definition(
        StateAllocationKey::TemplateChunkSets,
        AllocationOwner::CanicControlPlane,
        AllocationLifecycle::Active,
        TEMPLATE_CHUNK_SETS_IDS,
    ),
    definition(
        StateAllocationKey::TemplateChunkRefs,
        AllocationOwner::CanicControlPlane,
        AllocationLifecycle::Active,
        TEMPLATE_CHUNK_REFS_IDS,
    ),
    definition(
        StateAllocationKey::TemplateChunkPayloads,
        AllocationOwner::CanicControlPlane,
        AllocationLifecycle::Active,
        TEMPLATE_CHUNK_PAYLOADS_IDS,
    ),
    definition(
        StateAllocationKey::ControlPlaneSubnetState,
        AllocationOwner::CanicControlPlane,
        AllocationLifecycle::Active,
        CONTROL_PLANE_SUBNET_STATE_IDS,
    ),
    definition(
        StateAllocationKey::WasmStoreGcState,
        AllocationOwner::CanicControlPlane,
        AllocationLifecycle::Active,
        WASM_STORE_GC_STATE_IDS,
    ),
];

const fn definition(
    key: StateAllocationKey,
    owner: AllocationOwner,
    lifecycle: AllocationLifecycle,
    memory_ids: &'static [MemoryId],
) -> AllocationDefinition {
    AllocationDefinition {
        key,
        owner,
        lifecycle,
        memory_ids,
    }
}

#[must_use]
pub const fn allocation_definitions() -> &'static [AllocationDefinition] {
    ALLOCATION_DEFINITIONS
}

#[must_use]
pub fn allocation_definition(key: StateAllocationKey) -> Option<&'static AllocationDefinition> {
    ALLOCATION_DEFINITIONS
        .iter()
        .find(|definition| definition.key == key)
}

pub fn validate_allocation_definitions(
    definitions: &[AllocationDefinition],
) -> Result<(), RoleContractFinding> {
    let mut keys = BTreeSet::new();
    let mut memory_owners = BTreeMap::new();

    for definition in definitions {
        if !keys.insert(definition.key) {
            return Err(RoleContractFinding::CatalogInvalid {
                reason: format!("duplicate allocation definition: {:?}", definition.key),
            });
        }
        if definition.memory_ids.is_empty() {
            return Err(RoleContractFinding::CatalogInvalid {
                reason: format!("allocation has no memory IDs: {:?}", definition.key),
            });
        }

        for memory_id in definition.memory_ids {
            if let Some(first) = memory_owners.insert(*memory_id, definition.key) {
                return Err(RoleContractFinding::MemoryIdCollision {
                    memory_id: *memory_id,
                    first,
                    second: definition.key,
                });
            }
        }
    }

    Ok(())
}

pub fn validate_canonical_allocations() -> Result<(), RoleContractFinding> {
    validate_allocation_definitions(ALLOCATION_DEFINITIONS)
}
