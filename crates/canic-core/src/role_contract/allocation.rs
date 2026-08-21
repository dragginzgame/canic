//! Module: role_contract::allocation
//!
//! Responsibility: own canonical Canic stable-memory IDs and allocation definitions.
//! Does not own: stable records, descriptor metadata, migrations, or role selection.
//! Boundary: runtime storage imports IDs; pure role policy selects allocation keys.

use crate::role_contract::model::{
    AllocationDefinition, AllocationOwner, MemoryId, RoleContractFinding, StateAllocationKey,
};
use std::collections::{BTreeMap, BTreeSet};

pub const CANIC_CONTROL_PLANE_MIN_ID: u8 = 10;
pub const CANIC_CONTROL_PLANE_MAX_ID: u8 = 29;
pub const CANIC_CORE_MIN_ID: u8 = 30;
pub const CANIC_CORE_MAX_ID: u8 = 99;
pub const CANIC_CORE_LOWER_MAX_ID: u8 = 61;
pub const CANIC_CORE_UPPER_MIN_ID: u8 = 63;

/// Canonical stable-memory IDs grouped by record owner.
pub mod memory {
    pub mod control_plane {
        // Shared template state.
        pub const TEMPLATE_MANIFESTS_ID: u8 = 10;
        pub const TEMPLATE_CHUNK_SETS_ID: u8 = 11;
        pub const TEMPLATE_CHUNK_REFS_ID: u8 = 12;
        pub const TEMPLATE_CHUNK_PAYLOADS_ID: u8 = 13;

        // Wasm Store state.
        pub const WASM_STORE_GC_STATE_ID: u8 = 14;

        // Fleet Coordinator state.
        pub const FLEET_COORDINATOR_REGISTRY_ID: u8 = 15;
        pub const FLEET_COORDINATOR_FUNDING_ID: u8 = 62;

        // Fleet Subnet Root state.
        pub const ROOT_WASM_STORE_STATE_ID: u8 = 16;
        pub const ROOT_FLEET_REGISTRY_MIRROR_ID: u8 = 17;
        pub const ROOT_COMPONENT_REGISTRY_STATE_ID: u8 = 18;
        pub const ROOT_COMPONENT_ALLOCATIONS_ID: u8 = 19;
        pub const ROOT_COMPONENT_REGISTRY_ENTRIES_ID: u8 = 20;
        pub const ROOT_COMPONENT_PRINCIPAL_INDEX_ID: u8 = 21;
        pub const ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY_ID: u8 = 22;
        pub const ROOT_COMPONENT_DRAINING_ID: u8 = 23;

        // Fleet Subnet Root prepaid empty-Canister inventory.
        pub const ROOT_CANISTER_INVENTORY_ASSETS_ID: u8 = 24;
        pub const ROOT_CANISTER_POOL_STATE_ID: u8 = 25;
        pub const ROOT_CANISTER_POOL_HANDOFF_RECEIPTS_ID: u8 = 26;

        // Fleet Subnet Root aggregate Component Group provisioning authority.
        pub const ROOT_COMPONENT_PROVISIONING_OPERATIONS_ID: u8 = 27;
        pub const ROOT_COMPONENT_PROVISIONING_PLACEMENTS_ID: u8 = 28;
        pub const ROOT_COMPONENT_PROVISIONING_STATE_ID: u8 = 29;
    }

    pub mod runtime {
        pub const RUNTIME_CANISTER_CHILDREN_ID: u8 = 30;
        pub const RUNTIME_BINDINGS_ID: u8 = 31;
    }

    pub mod fleet {
        pub const FLEET_STATE_ID: u8 = 32;
        pub const FLEET_ACTIVATION_ID: u8 = 33;
    }

    pub mod auth {
        pub const AUTH_STATE_ID: u8 = 34;
    }

    pub mod replay {
        pub const REPLAY_RECEIPTS_ID: u8 = 35;
    }

    pub mod cycles {
        pub const CYCLES_TRACKER_ID: u8 = 36;
        pub const CYCLES_TOPUP_EVENTS_ID: u8 = 37;
        pub const CYCLES_FUNDING_LEDGER_ID: u8 = 38;
        pub const CYCLES_ICP_REFILL_RECORDS_ID: u8 = 39;
    }

    pub mod log {
        pub const LOG_ENTRIES_ID: u8 = 40;
    }

    pub mod intent {
        pub const INTENT_META_ID: u8 = 41;
        pub const INTENT_RECORDS_ID: u8 = 42;
        pub const INTENT_TOTALS_ID: u8 = 43;
        pub const INTENT_PENDING_ID: u8 = 44;
        pub const INTENT_RECEIPT_BACKED_RECORDS_ID: u8 = 45;
        pub const INTENT_EXPIRY_INDEX_ID: u8 = 46;
    }

    pub mod application_receipt {
        pub const APPLICATION_RECEIPT_REPLAY_ID: u8 = 47;
        pub const APPLICATION_RECEIPT_ELIGIBILITY_ID: u8 = 48;
    }

    pub mod placement {
        pub const PLACEMENT_ACKNOWLEDGEMENT_INDEX_ID: u8 = 49;
        pub const PLACEMENT_SCALING_REGISTRY_ID: u8 = 50;
        pub const PLACEMENT_INDEX_REGISTRY_ID: u8 = 51;
    }

    pub mod sharding {
        pub const SHARDING_REGISTRY_ID: u8 = 52;
        pub const SHARDING_ASSIGNMENTS_ID: u8 = 53;
        pub const SHARDING_ACTIVE_SET_ID: u8 = 54;
    }

    pub mod blob_storage {
        pub const BLOB_STORAGE_ROOTS_ID: u8 = 55;
        pub const BLOB_STORAGE_PENDING_DELETIONS_ID: u8 = 56;
        pub const BLOB_STORAGE_GATEWAY_PRINCIPALS_ID: u8 = 57;
        pub const BLOB_STORAGE_BILLING_ID: u8 = 58;
    }

    pub mod authority_restore {
        pub const AUTHORITY_RESTORE_FENCE_ID: u8 = 59;
    }

    pub mod async_job_recovery {
        pub const ASYNC_JOB_RECOVERY_ID: u8 = 60;
    }

    pub mod runtime_whitelist {
        pub const RUNTIME_WHITELIST_ID: u8 = 61;
    }
}

use memory::{
    application_receipt::{APPLICATION_RECEIPT_ELIGIBILITY_ID, APPLICATION_RECEIPT_REPLAY_ID},
    async_job_recovery::ASYNC_JOB_RECOVERY_ID,
    auth::AUTH_STATE_ID,
    authority_restore::AUTHORITY_RESTORE_FENCE_ID,
    blob_storage::{
        BLOB_STORAGE_BILLING_ID, BLOB_STORAGE_GATEWAY_PRINCIPALS_ID,
        BLOB_STORAGE_PENDING_DELETIONS_ID, BLOB_STORAGE_ROOTS_ID,
    },
    control_plane::{
        FLEET_COORDINATOR_FUNDING_ID, FLEET_COORDINATOR_REGISTRY_ID,
        ROOT_CANISTER_INVENTORY_ASSETS_ID, ROOT_CANISTER_POOL_HANDOFF_RECEIPTS_ID,
        ROOT_CANISTER_POOL_STATE_ID, ROOT_COMPONENT_ALLOCATIONS_ID, ROOT_COMPONENT_DRAINING_ID,
        ROOT_COMPONENT_PRINCIPAL_INDEX_ID, ROOT_COMPONENT_PROVISIONING_OPERATIONS_ID,
        ROOT_COMPONENT_PROVISIONING_PLACEMENTS_ID, ROOT_COMPONENT_PROVISIONING_STATE_ID,
        ROOT_COMPONENT_REGISTRY_ENTRIES_ID, ROOT_COMPONENT_REGISTRY_STATE_ID,
        ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY_ID, ROOT_FLEET_REGISTRY_MIRROR_ID,
        ROOT_WASM_STORE_STATE_ID, TEMPLATE_CHUNK_PAYLOADS_ID, TEMPLATE_CHUNK_REFS_ID,
        TEMPLATE_CHUNK_SETS_ID, TEMPLATE_MANIFESTS_ID, WASM_STORE_GC_STATE_ID,
    },
    cycles::{
        CYCLES_FUNDING_LEDGER_ID, CYCLES_ICP_REFILL_RECORDS_ID, CYCLES_TOPUP_EVENTS_ID,
        CYCLES_TRACKER_ID,
    },
    fleet::{FLEET_ACTIVATION_ID, FLEET_STATE_ID},
    intent::{
        INTENT_EXPIRY_INDEX_ID, INTENT_META_ID, INTENT_PENDING_ID,
        INTENT_RECEIPT_BACKED_RECORDS_ID, INTENT_RECORDS_ID, INTENT_TOTALS_ID,
    },
    log::LOG_ENTRIES_ID,
    placement::{
        PLACEMENT_ACKNOWLEDGEMENT_INDEX_ID, PLACEMENT_INDEX_REGISTRY_ID,
        PLACEMENT_SCALING_REGISTRY_ID,
    },
    replay::REPLAY_RECEIPTS_ID,
    runtime::{RUNTIME_BINDINGS_ID, RUNTIME_CANISTER_CHILDREN_ID},
    runtime_whitelist::RUNTIME_WHITELIST_ID,
    sharding::{SHARDING_ACTIVE_SET_ID, SHARDING_ASSIGNMENTS_ID, SHARDING_REGISTRY_ID},
};

const TEMPLATE_MANIFESTS_IDS: &[MemoryId] = &[MemoryId::new(TEMPLATE_MANIFESTS_ID)];
const TEMPLATE_CHUNK_SETS_IDS: &[MemoryId] = &[MemoryId::new(TEMPLATE_CHUNK_SETS_ID)];
const TEMPLATE_CHUNK_REFS_IDS: &[MemoryId] = &[MemoryId::new(TEMPLATE_CHUNK_REFS_ID)];
const TEMPLATE_CHUNK_PAYLOADS_IDS: &[MemoryId] = &[MemoryId::new(TEMPLATE_CHUNK_PAYLOADS_ID)];
const WASM_STORE_GC_STATE_IDS: &[MemoryId] = &[MemoryId::new(WASM_STORE_GC_STATE_ID)];
const FLEET_COORDINATOR_REGISTRY_IDS: &[MemoryId] = &[MemoryId::new(FLEET_COORDINATOR_REGISTRY_ID)];
const FLEET_COORDINATOR_FUNDING_IDS: &[MemoryId] = &[MemoryId::new(FLEET_COORDINATOR_FUNDING_ID)];
const ROOT_WASM_STORE_STATE_IDS: &[MemoryId] = &[MemoryId::new(ROOT_WASM_STORE_STATE_ID)];
const ROOT_FLEET_REGISTRY_MIRROR_IDS: &[MemoryId] = &[MemoryId::new(ROOT_FLEET_REGISTRY_MIRROR_ID)];
const ROOT_COMPONENT_REGISTRY_IDS: &[MemoryId] = &[
    MemoryId::new(ROOT_COMPONENT_REGISTRY_STATE_ID),
    MemoryId::new(ROOT_COMPONENT_ALLOCATIONS_ID),
    MemoryId::new(ROOT_COMPONENT_REGISTRY_ENTRIES_ID),
    MemoryId::new(ROOT_COMPONENT_PRINCIPAL_INDEX_ID),
    MemoryId::new(ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY_ID),
    MemoryId::new(ROOT_COMPONENT_DRAINING_ID),
];
const ROOT_CANISTER_POOL_IDS: &[MemoryId] = &[
    MemoryId::new(ROOT_CANISTER_INVENTORY_ASSETS_ID),
    MemoryId::new(ROOT_CANISTER_POOL_STATE_ID),
    MemoryId::new(ROOT_CANISTER_POOL_HANDOFF_RECEIPTS_ID),
];
const ROOT_COMPONENT_PROVISIONING_IDS: &[MemoryId] = &[
    MemoryId::new(ROOT_COMPONENT_PROVISIONING_OPERATIONS_ID),
    MemoryId::new(ROOT_COMPONENT_PROVISIONING_PLACEMENTS_ID),
    MemoryId::new(ROOT_COMPONENT_PROVISIONING_STATE_ID),
];

const CORE_RUNTIME_CHILDREN_IDS: &[MemoryId] = &[MemoryId::new(RUNTIME_CANISTER_CHILDREN_ID)];
const CORE_RUNTIME_BINDINGS_IDS: &[MemoryId] = &[MemoryId::new(RUNTIME_BINDINGS_ID)];
const CORE_FLEET_STATE_IDS: &[MemoryId] = &[MemoryId::new(FLEET_STATE_ID)];
const CORE_FLEET_ACTIVATION_IDS: &[MemoryId] = &[MemoryId::new(FLEET_ACTIVATION_ID)];
const CORE_AUTH_STATE_IDS: &[MemoryId] = &[MemoryId::new(AUTH_STATE_ID)];
const CORE_REPLAY_RECEIPTS_IDS: &[MemoryId] = &[MemoryId::new(REPLAY_RECEIPTS_ID)];
const CORE_CYCLES_IDS: &[MemoryId] = &[
    MemoryId::new(CYCLES_TRACKER_ID),
    MemoryId::new(CYCLES_TOPUP_EVENTS_ID),
    MemoryId::new(CYCLES_FUNDING_LEDGER_ID),
];
const CORE_CYCLES_ICP_REFILL_RECORDS_IDS: &[MemoryId] =
    &[MemoryId::new(CYCLES_ICP_REFILL_RECORDS_ID)];
const CORE_RUNTIME_LOG_IDS: &[MemoryId] = &[MemoryId::new(LOG_ENTRIES_ID)];
const CORE_INTENT_IDS: &[MemoryId] = &[
    MemoryId::new(INTENT_META_ID),
    MemoryId::new(INTENT_RECORDS_ID),
    MemoryId::new(INTENT_TOTALS_ID),
    MemoryId::new(INTENT_PENDING_ID),
    MemoryId::new(INTENT_RECEIPT_BACKED_RECORDS_ID),
    MemoryId::new(INTENT_EXPIRY_INDEX_ID),
];
const CORE_APPLICATION_RECEIPT_IDS: &[MemoryId] = &[
    MemoryId::new(APPLICATION_RECEIPT_REPLAY_ID),
    MemoryId::new(APPLICATION_RECEIPT_ELIGIBILITY_ID),
];
const CORE_PLACEMENT_ACKNOWLEDGEMENT_IDS: &[MemoryId] =
    &[MemoryId::new(PLACEMENT_ACKNOWLEDGEMENT_INDEX_ID)];
const CORE_AUTHORITY_RESTORE_FENCE_IDS: &[MemoryId] = &[MemoryId::new(AUTHORITY_RESTORE_FENCE_ID)];
const CORE_ASYNC_JOB_RECOVERY_IDS: &[MemoryId] = &[MemoryId::new(ASYNC_JOB_RECOVERY_ID)];
const CORE_RUNTIME_WHITELIST_IDS: &[MemoryId] = &[MemoryId::new(RUNTIME_WHITELIST_ID)];
const PLACEMENT_SCALING_REGISTRY_IDS: &[MemoryId] = &[MemoryId::new(PLACEMENT_SCALING_REGISTRY_ID)];
const PLACEMENT_INDEX_REGISTRY_IDS: &[MemoryId] = &[MemoryId::new(PLACEMENT_INDEX_REGISTRY_ID)];
const SHARDING_REGISTRY_IDS: &[MemoryId] = &[MemoryId::new(SHARDING_REGISTRY_ID)];
const SHARDING_ASSIGNMENTS_IDS: &[MemoryId] = &[MemoryId::new(SHARDING_ASSIGNMENTS_ID)];
const SHARDING_ACTIVE_SET_IDS: &[MemoryId] = &[MemoryId::new(SHARDING_ACTIVE_SET_ID)];
const BLOB_STORAGE_ROOTS_IDS: &[MemoryId] = &[MemoryId::new(BLOB_STORAGE_ROOTS_ID)];
const BLOB_STORAGE_PENDING_DELETIONS_IDS: &[MemoryId] =
    &[MemoryId::new(BLOB_STORAGE_PENDING_DELETIONS_ID)];
const BLOB_STORAGE_GATEWAY_PRINCIPALS_IDS: &[MemoryId] =
    &[MemoryId::new(BLOB_STORAGE_GATEWAY_PRINCIPALS_ID)];
const BLOB_STORAGE_BILLING_IDS: &[MemoryId] = &[MemoryId::new(BLOB_STORAGE_BILLING_ID)];

const ALLOCATION_DEFINITIONS: &[AllocationDefinition] = &[
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
        StateAllocationKey::WasmStoreGcState,
        AllocationOwner::CanicControlPlane,
        WASM_STORE_GC_STATE_IDS,
    ),
    definition(
        StateAllocationKey::FleetCoordinatorFunding,
        AllocationOwner::CanicControlPlane,
        FLEET_COORDINATOR_FUNDING_IDS,
    ),
    definition(
        StateAllocationKey::FleetCoordinatorRegistry,
        AllocationOwner::CanicControlPlane,
        FLEET_COORDINATOR_REGISTRY_IDS,
    ),
    definition(
        StateAllocationKey::RootWasmStoreState,
        AllocationOwner::CanicControlPlane,
        ROOT_WASM_STORE_STATE_IDS,
    ),
    definition(
        StateAllocationKey::RootFleetRegistryMirror,
        AllocationOwner::CanicControlPlane,
        ROOT_FLEET_REGISTRY_MIRROR_IDS,
    ),
    definition(
        StateAllocationKey::RootComponentRegistry,
        AllocationOwner::CanicControlPlane,
        ROOT_COMPONENT_REGISTRY_IDS,
    ),
    definition(
        StateAllocationKey::RootCanisterPool,
        AllocationOwner::CanicControlPlane,
        ROOT_CANISTER_POOL_IDS,
    ),
    definition(
        StateAllocationKey::RootComponentProvisioning,
        AllocationOwner::CanicControlPlane,
        ROOT_COMPONENT_PROVISIONING_IDS,
    ),
    definition(
        StateAllocationKey::CoreRuntimeChildren,
        AllocationOwner::CanicCore,
        CORE_RUNTIME_CHILDREN_IDS,
    ),
    definition(
        StateAllocationKey::CoreRuntimeBindings,
        AllocationOwner::CanicCore,
        CORE_RUNTIME_BINDINGS_IDS,
    ),
    definition(
        StateAllocationKey::CoreFleetState,
        AllocationOwner::CanicCore,
        CORE_FLEET_STATE_IDS,
    ),
    definition(
        StateAllocationKey::CoreFleetActivation,
        AllocationOwner::CanicCore,
        CORE_FLEET_ACTIVATION_IDS,
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
        StateAllocationKey::CoreCycles,
        AllocationOwner::CanicCore,
        CORE_CYCLES_IDS,
    ),
    definition(
        StateAllocationKey::CoreCyclesIcpRefillRecords,
        AllocationOwner::CanicCore,
        CORE_CYCLES_ICP_REFILL_RECORDS_IDS,
    ),
    definition(
        StateAllocationKey::CoreRuntimeLog,
        AllocationOwner::CanicCore,
        CORE_RUNTIME_LOG_IDS,
    ),
    definition(
        StateAllocationKey::CoreIntent,
        AllocationOwner::CanicCore,
        CORE_INTENT_IDS,
    ),
    definition(
        StateAllocationKey::CoreApplicationReceipts,
        AllocationOwner::CanicCore,
        CORE_APPLICATION_RECEIPT_IDS,
    ),
    definition(
        StateAllocationKey::CorePlacementAcknowledgement,
        AllocationOwner::CanicCore,
        CORE_PLACEMENT_ACKNOWLEDGEMENT_IDS,
    ),
    definition(
        StateAllocationKey::PlacementScalingRegistry,
        AllocationOwner::CanicCore,
        PLACEMENT_SCALING_REGISTRY_IDS,
    ),
    definition(
        StateAllocationKey::PlacementIndexRegistry,
        AllocationOwner::CanicCore,
        PLACEMENT_INDEX_REGISTRY_IDS,
    ),
    definition(
        StateAllocationKey::ShardingRegistry,
        AllocationOwner::CanicCore,
        SHARDING_REGISTRY_IDS,
    ),
    definition(
        StateAllocationKey::ShardingAssignments,
        AllocationOwner::CanicCore,
        SHARDING_ASSIGNMENTS_IDS,
    ),
    definition(
        StateAllocationKey::ShardingActiveSet,
        AllocationOwner::CanicCore,
        SHARDING_ACTIVE_SET_IDS,
    ),
    definition(
        StateAllocationKey::BlobStorageRoots,
        AllocationOwner::CanicCore,
        BLOB_STORAGE_ROOTS_IDS,
    ),
    definition(
        StateAllocationKey::BlobStoragePendingDeletions,
        AllocationOwner::CanicCore,
        BLOB_STORAGE_PENDING_DELETIONS_IDS,
    ),
    definition(
        StateAllocationKey::BlobStorageGatewayPrincipals,
        AllocationOwner::CanicCore,
        BLOB_STORAGE_GATEWAY_PRINCIPALS_IDS,
    ),
    definition(
        StateAllocationKey::BlobStorageBilling,
        AllocationOwner::CanicCore,
        BLOB_STORAGE_BILLING_IDS,
    ),
    definition(
        StateAllocationKey::CoreAuthorityRestoreFence,
        AllocationOwner::CanicCore,
        CORE_AUTHORITY_RESTORE_FENCE_IDS,
    ),
    definition(
        StateAllocationKey::CoreAsyncJobRecovery,
        AllocationOwner::CanicCore,
        CORE_ASYNC_JOB_RECOVERY_IDS,
    ),
    definition(
        StateAllocationKey::CoreRuntimeWhitelist,
        AllocationOwner::CanicCore,
        CORE_RUNTIME_WHITELIST_IDS,
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
            let (owner_min_id, owner_max_id) = match definition.owner {
                AllocationOwner::CanicCore => (CANIC_CORE_MIN_ID, CANIC_CORE_MAX_ID),
                AllocationOwner::CanicControlPlane => {
                    (CANIC_CONTROL_PLANE_MIN_ID, CANIC_CONTROL_PLANE_MAX_ID)
                }
            };
            let id = memory_id.get();
            let owner_matches = match definition.owner {
                AllocationOwner::CanicControlPlane => {
                    (owner_min_id..=owner_max_id).contains(&id)
                        || id == FLEET_COORDINATOR_FUNDING_ID
                }
                AllocationOwner::CanicCore => {
                    (owner_min_id..=owner_max_id).contains(&id)
                        && id != FLEET_COORDINATOR_FUNDING_ID
                }
            };
            if !owner_matches {
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
