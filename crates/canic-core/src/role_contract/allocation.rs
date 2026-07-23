//! Module: role_contract::allocation
//!
//! Responsibility: own canonical Canic stable-memory IDs and allocation definitions.
//! Does not own: stable records, descriptor metadata, migrations, or role selection.
//! Boundary: runtime storage imports IDs; pure role policy selects allocation keys.

use crate::role_contract::model::{
    AllocationDefinition, AllocationOwner, MemoryId, RoleContractFinding, StateAllocationKey,
};
use std::collections::{BTreeMap, BTreeSet};

pub const CANIC_CORE_MIN_ID: u8 = 11;
pub const CANIC_CORE_MAX_ID: u8 = 79;
pub const CANIC_CONTROL_PLANE_MIN_ID: u8 = 80;
pub const CANIC_CONTROL_PLANE_MAX_ID: u8 = 99;

/// Canonical stable-memory IDs grouped by record owner.
pub mod memory {
    pub mod topology {
        pub const CANISTER_CHILDREN_ID: u8 = 11;
        pub const APP_INDEX_ID: u8 = 12;
        pub const SUBNET_INDEX_ID: u8 = 13;
        pub const RETIRED_APP_REGISTRY_ID: u8 = 14;
        pub const SUBNET_REGISTRY_ID: u8 = 15;
    }

    pub mod env {
        pub const ENV_ID: u8 = 16;
        pub const RETIRED_SUBNET_STATE_ID: u8 = 17;
        pub const APP_STATE_ID: u8 = 18;
    }

    pub mod auth {
        pub const AUTH_STATE_ID: u8 = 19;
        pub const REPLAY_RECEIPTS_ID: u8 = 20;
    }

    pub mod observability {
        pub const CYCLE_TRACKER_ID: u8 = 29;
        pub const CYCLE_TOPUP_EVENTS_ID: u8 = 30;
        pub const ICP_REFILL_RECORDS_ID: u8 = 33;
        pub const CYCLES_FUNDING_LEDGER_ID: u8 = 34;
        pub const LOG_ENTRIES_ID: u8 = 35;
    }

    pub mod intent {
        pub const INTENT_META_ID: u8 = 39;
        pub const INTENT_RECORDS_ID: u8 = 40;
        pub const INTENT_TOTALS_ID: u8 = 41;
        pub const INTENT_PENDING_ID: u8 = 42;
        pub const RECEIPT_BACKED_INTENT_RECORDS_ID: u8 = 43;
        pub const INTENT_EXPIRY_INDEX_ID: u8 = 44;
        pub const PLACEMENT_ACKNOWLEDGEMENT_INDEX_ID: u8 = 45;
        pub const APPLICATION_RECEIPT_REPLAY_ID: u8 = 46;
        pub const APPLICATION_RECEIPT_ELIGIBILITY_ID: u8 = 47;
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
    auth::{AUTH_STATE_ID, REPLAY_RECEIPTS_ID},
    blob_storage::{
        BLOB_DELETION_PENDING_ID, BLOB_STORAGE_BILLING_ID, STORAGE_GATEWAY_PRINCIPALS_ID,
        STORED_BLOBS_ID,
    },
    env::{APP_STATE_ID, ENV_ID, RETIRED_SUBNET_STATE_ID},
    intent::{
        APPLICATION_RECEIPT_ELIGIBILITY_ID, APPLICATION_RECEIPT_REPLAY_ID, INTENT_EXPIRY_INDEX_ID,
        INTENT_META_ID, INTENT_PENDING_ID, INTENT_RECORDS_ID, INTENT_TOTALS_ID,
        PLACEMENT_ACKNOWLEDGEMENT_INDEX_ID, RECEIPT_BACKED_INTENT_RECORDS_ID,
    },
    observability::{
        CYCLE_TOPUP_EVENTS_ID, CYCLE_TRACKER_ID, CYCLES_FUNDING_LEDGER_ID, ICP_REFILL_RECORDS_ID,
        LOG_ENTRIES_ID,
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
        APP_INDEX_ID, CANISTER_CHILDREN_ID, RETIRED_APP_REGISTRY_ID, SUBNET_INDEX_ID,
        SUBNET_REGISTRY_ID,
    },
};

const CORE_RUNTIME_TOPOLOGY_IDS: &[MemoryId] = &[
    MemoryId::new(CANISTER_CHILDREN_ID),
    MemoryId::new(APP_INDEX_ID),
    MemoryId::new(SUBNET_INDEX_ID),
    MemoryId::new(SUBNET_REGISTRY_ID),
];
const CORE_RUNTIME_ENVIRONMENT_IDS: &[MemoryId] =
    &[MemoryId::new(ENV_ID), MemoryId::new(APP_STATE_ID)];
const RETIRED_MEMORY_IDS: &[MemoryId] = &[
    MemoryId::new(RETIRED_APP_REGISTRY_ID),
    MemoryId::new(RETIRED_SUBNET_STATE_ID),
];
const CORE_AUTH_STATE_IDS: &[MemoryId] = &[MemoryId::new(AUTH_STATE_ID)];
const CORE_REPLAY_RECEIPTS_IDS: &[MemoryId] = &[MemoryId::new(REPLAY_RECEIPTS_ID)];
const CORE_RUNTIME_OBSERVABILITY_IDS: &[MemoryId] = &[
    MemoryId::new(CYCLE_TRACKER_ID),
    MemoryId::new(CYCLE_TOPUP_EVENTS_ID),
    MemoryId::new(CYCLES_FUNDING_LEDGER_ID),
    MemoryId::new(LOG_ENTRIES_ID),
];
const CORE_ICP_REFILL_RECORDS_IDS: &[MemoryId] = &[MemoryId::new(ICP_REFILL_RECORDS_ID)];
const CORE_RUNTIME_INTENT_IDS: &[MemoryId] = &[
    MemoryId::new(INTENT_META_ID),
    MemoryId::new(INTENT_RECORDS_ID),
    MemoryId::new(INTENT_TOTALS_ID),
    MemoryId::new(INTENT_PENDING_ID),
    MemoryId::new(RECEIPT_BACKED_INTENT_RECORDS_ID),
    MemoryId::new(INTENT_EXPIRY_INDEX_ID),
    MemoryId::new(PLACEMENT_ACKNOWLEDGEMENT_INDEX_ID),
    MemoryId::new(APPLICATION_RECEIPT_REPLAY_ID),
    MemoryId::new(APPLICATION_RECEIPT_ELIGIBILITY_ID),
];
const CANISTER_POOL_IDS: &[MemoryId] = &[MemoryId::new(CANISTER_POOL_ID)];
const SCALING_REGISTRY_IDS: &[MemoryId] = &[MemoryId::new(SCALING_REGISTRY_ID)];
const DIRECTORY_REGISTRY_IDS: &[MemoryId] = &[MemoryId::new(DIRECTORY_REGISTRY_ID)];
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
        StateAllocationKey::CoreRuntimeTopology,
        AllocationOwner::CanicCore,
        CORE_RUNTIME_TOPOLOGY_IDS,
    ),
    definition(
        StateAllocationKey::CoreRuntimeEnvironment,
        AllocationOwner::CanicCore,
        CORE_RUNTIME_ENVIRONMENT_IDS,
    ),
    definition(
        StateAllocationKey::CoreAuthState,
        AllocationOwner::CanicCore,
        CORE_AUTH_STATE_IDS,
    ),
    definition(
        StateAllocationKey::CoreReplayReceipts,
        AllocationOwner::CanicCore,
        CORE_REPLAY_RECEIPTS_IDS,
    ),
    definition(
        StateAllocationKey::CoreRuntimeObservability,
        AllocationOwner::CanicCore,
        CORE_RUNTIME_OBSERVABILITY_IDS,
    ),
    definition(
        StateAllocationKey::CoreIcpRefillRecords,
        AllocationOwner::CanicCore,
        CORE_ICP_REFILL_RECORDS_IDS,
    ),
    definition(
        StateAllocationKey::CoreRuntimeIntent,
        AllocationOwner::CanicCore,
        CORE_RUNTIME_INTENT_IDS,
    ),
    definition(
        StateAllocationKey::CanisterPool,
        AllocationOwner::CanicCore,
        CANISTER_POOL_IDS,
    ),
    definition(
        StateAllocationKey::ScalingRegistry,
        AllocationOwner::CanicCore,
        SCALING_REGISTRY_IDS,
    ),
    definition(
        StateAllocationKey::DirectoryRegistry,
        AllocationOwner::CanicCore,
        DIRECTORY_REGISTRY_IDS,
    ),
    definition(
        StateAllocationKey::ShardingRegistry,
        AllocationOwner::CanicCore,
        SHARDING_REGISTRY_IDS,
    ),
    definition(
        StateAllocationKey::ShardingAssignments,
        AllocationOwner::CanicCore,
        SHARDING_ASSIGNMENT_IDS,
    ),
    definition(
        StateAllocationKey::ShardingActiveSet,
        AllocationOwner::CanicCore,
        SHARDING_ACTIVE_SET_IDS,
    ),
    definition(
        StateAllocationKey::StoredBlobs,
        AllocationOwner::CanicCore,
        STORED_BLOBS_IDS,
    ),
    definition(
        StateAllocationKey::BlobDeletionPending,
        AllocationOwner::CanicCore,
        BLOB_DELETION_PENDING_IDS,
    ),
    definition(
        StateAllocationKey::StorageGatewayPrincipals,
        AllocationOwner::CanicCore,
        STORAGE_GATEWAY_PRINCIPALS_IDS,
    ),
    definition(
        StateAllocationKey::BlobStorageBilling,
        AllocationOwner::CanicCore,
        BLOB_STORAGE_BILLING_IDS,
    ),
    definition(
        StateAllocationKey::TemplateManifests,
        AllocationOwner::CanicControlPlane,
        TEMPLATE_MANIFESTS_IDS,
    ),
    definition(
        StateAllocationKey::TemplateChunkSets,
        AllocationOwner::CanicControlPlane,
        TEMPLATE_CHUNK_SETS_IDS,
    ),
    definition(
        StateAllocationKey::TemplateChunkRefs,
        AllocationOwner::CanicControlPlane,
        TEMPLATE_CHUNK_REFS_IDS,
    ),
    definition(
        StateAllocationKey::TemplateChunkPayloads,
        AllocationOwner::CanicControlPlane,
        TEMPLATE_CHUNK_PAYLOADS_IDS,
    ),
    definition(
        StateAllocationKey::ControlPlaneSubnetState,
        AllocationOwner::CanicControlPlane,
        CONTROL_PLANE_SUBNET_STATE_IDS,
    ),
    definition(
        StateAllocationKey::WasmStoreGcState,
        AllocationOwner::CanicControlPlane,
        WASM_STORE_GC_STATE_IDS,
    ),
];

const fn definition(
    key: StateAllocationKey,
    owner: AllocationOwner,
    memory_ids: &'static [MemoryId],
) -> AllocationDefinition {
    AllocationDefinition {
        key,
        owner,
        memory_ids,
    }
}

#[must_use]
pub const fn allocation_definitions() -> &'static [AllocationDefinition] {
    ALLOCATION_DEFINITIONS
}

/// Stable-memory IDs permanently excluded from active allocation.
#[must_use]
pub const fn retired_memory_ids() -> &'static [MemoryId] {
    RETIRED_MEMORY_IDS
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
            if RETIRED_MEMORY_IDS.contains(memory_id) {
                return Err(RoleContractFinding::CatalogInvalid {
                    reason: format!(
                        "allocation {:?} reuses retired memory ID {}",
                        definition.key,
                        memory_id.get(),
                    ),
                });
            }
            let (owner_min_id, owner_max_id) = match definition.owner {
                AllocationOwner::CanicCore => (CANIC_CORE_MIN_ID, CANIC_CORE_MAX_ID),
                AllocationOwner::CanicControlPlane => {
                    (CANIC_CONTROL_PLANE_MIN_ID, CANIC_CONTROL_PLANE_MAX_ID)
                }
            };
            if !(owner_min_id..=owner_max_id).contains(&memory_id.get()) {
                return Err(RoleContractFinding::CatalogInvalid {
                    reason: format!(
                        "allocation {:?} assigns memory ID {} outside owner {} range {owner_min_id}-{owner_max_id}",
                        definition.key,
                        memory_id.get(),
                        definition.owner.as_str(),
                    ),
                });
            }
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
