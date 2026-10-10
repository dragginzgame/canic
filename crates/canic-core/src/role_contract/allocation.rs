//! Module: role_contract::allocation
//!
//! Responsibility: own canonical Canic stable-memory keys and allocation definitions.
//! Does not own: stable records, descriptor metadata, or role selection.
//! Boundary: runtime storage imports keys; pure role policy selects allocation keys.

use crate::role_contract::model::{
    AllocationDefinition, AllocationOwner, RoleContractFinding, StateAllocationKey,
};
use memory::{
    application_receipt::APPLICATION_RECEIPT_ELIGIBILITY_KEY,
    async_job_recovery::ASYNC_JOB_RECOVERY_KEY,
    auth::{
        DELEGATED_TOKEN_ISSUER_STATE_KEY, LOCAL_APPLICATION_AUTHORIZATION_STATE_KEY,
        ROOT_DELEGATION_STATE_KEY,
    },
    authority_restore::AUTHORITY_RESTORE_FENCE_KEY,
    control_plane::{
        FIXTURE_STORE_KEY, FLEET_COORDINATOR_ADMISSION_KEY, FLEET_COORDINATOR_FUNDING_KEY,
        FLEET_COORDINATOR_REGISTRY_KEY, ROOT_ADMISSION_KEY, ROOT_CANISTER_INVENTORY_ASSETS_KEY,
        ROOT_CANISTER_POOL_HANDOFF_RECEIPTS_KEY, ROOT_CANISTER_POOL_STATE_KEY,
        ROOT_COMPONENT_ALLOCATIONS_KEY, ROOT_COMPONENT_DRAINING_KEY,
        ROOT_COMPONENT_PRINCIPAL_INDEX_KEY, ROOT_COMPONENT_PROVISIONING_OPERATIONS_KEY,
        ROOT_COMPONENT_PROVISIONING_PLACEMENTS_KEY, ROOT_COMPONENT_PROVISIONING_STATE_KEY,
        ROOT_COMPONENT_REGISTRY_ENTRIES_KEY, ROOT_COMPONENT_REGISTRY_STATE_KEY,
        ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY_KEY, ROOT_FLEET_REGISTRY_MIRROR_KEY,
        ROOT_FUNDING_KEY, ROOT_WASM_STORE_STATE_KEY, TEMPLATE_CHUNK_PAYLOADS_KEY,
        TEMPLATE_CHUNK_REFS_KEY, TEMPLATE_CHUNK_SETS_KEY, TEMPLATE_MANIFESTS_KEY,
        WASM_STORE_GC_STATE_KEY,
    },
    cycles::{
        CYCLES_FUNDING_LEDGER_KEY, CYCLES_ICP_REFILL_RECORDS_KEY, CYCLES_TOPUP_EVENTS_KEY,
        CYCLES_TRACKER_KEY,
    },
    fleet::{FLEET_ACTIVATION_KEY, FLEET_STATE_KEY},
    fleet_admission_projection::FLEET_ADMISSION_PROJECTION_KEY,
    intent::{
        INTENT_EXPIRY_INDEX_KEY, INTENT_META_KEY, INTENT_PENDING_KEY,
        INTENT_RECEIPT_BACKED_RECORDS_KEY, INTENT_RECORDS_KEY, INTENT_TOTALS_KEY,
    },
    log::LOG_ENTRIES_KEY,
    placement::{
        PLACEMENT_ACKNOWLEDGEMENT_INDEX_KEY, PLACEMENT_INDEX_REGISTRY_KEY,
        PLACEMENT_SCALING_REGISTRY_KEY,
    },
    replay::REPLAY_RECEIPTS_KEY,
    runtime::{RUNTIME_BINDINGS_KEY, RUNTIME_CANISTER_CHILDREN_KEY},
    sharding::{SHARDING_ASSIGNMENTS_KEY, SHARDING_REGISTRY_KEY},
};
use std::collections::{BTreeMap, BTreeSet};

/// Canonical stable-memory keys grouped by record owner.
pub mod memory {
    pub mod control_plane {
        // Shared template state.
        pub const TEMPLATE_MANIFESTS_KEY: &str = "canic.control_plane.template.manifests.v1";
        pub const TEMPLATE_CHUNK_SETS_KEY: &str = "canic.control_plane.template.chunk_sets.v1";
        pub const TEMPLATE_CHUNK_REFS_KEY: &str = "canic.control_plane.template.chunk_refs.v1";
        pub const TEMPLATE_CHUNK_PAYLOADS_KEY: &str =
            "canic.control_plane.template.chunk_payloads.v1";

        // Wasm Store state.
        pub const WASM_STORE_GC_STATE_KEY: &str = "canic.control_plane.wasm_store.gc_state.v1";
        pub const FIXTURE_STORE_KEY: &str = "canic.control_plane.fixture_store.v1";

        // Fleet Coordinator state.
        pub const FLEET_COORDINATOR_REGISTRY_KEY: &str =
            "canic.control_plane.fleet_coordinator.registry.v1";
        pub const FLEET_COORDINATOR_FUNDING_KEY: &str =
            "canic.control_plane.fleet_coordinator.funding.v1";
        pub const ROOT_FUNDING_KEY: &str = "canic.control_plane.root.funding.v1";
        pub const FLEET_COORDINATOR_ADMISSION_KEY: &str = "canic.control_plane.fleet_admission.v1";
        pub const ROOT_ADMISSION_KEY: &str = "canic.control_plane.root.admission.v1";

        // Fleet Subnet Root state.
        pub const ROOT_WASM_STORE_STATE_KEY: &str = "canic.control_plane.root.wasm_store.state.v1";
        pub const ROOT_FLEET_REGISTRY_MIRROR_KEY: &str =
            "canic.control_plane.root.fleet_registry_mirror.v1";
        pub const ROOT_COMPONENT_REGISTRY_STATE_KEY: &str =
            "canic.control_plane.root.component.registry_state.v1";
        pub const ROOT_COMPONENT_ALLOCATIONS_KEY: &str =
            "canic.control_plane.root.component.allocations.v1";
        pub const ROOT_COMPONENT_REGISTRY_ENTRIES_KEY: &str =
            "canic.control_plane.root.component.registry_entries.v1";
        pub const ROOT_COMPONENT_PRINCIPAL_INDEX_KEY: &str =
            "canic.control_plane.root.component.principal_index.v1";
        pub const ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY_KEY: &str =
            "canic.control_plane.root.component.subtree_removal_history.v1";
        pub const ROOT_COMPONENT_DRAINING_KEY: &str =
            "canic.control_plane.root.component.draining.v1";

        // Fleet Subnet Root prepaid empty-Canister inventory.
        pub const ROOT_CANISTER_INVENTORY_ASSETS_KEY: &str =
            "canic.control_plane.root.canister_inventory.assets.v1";
        pub const ROOT_CANISTER_POOL_STATE_KEY: &str =
            "canic.control_plane.root.canister_pool.state.v1";
        pub const ROOT_CANISTER_POOL_HANDOFF_RECEIPTS_KEY: &str =
            "canic.control_plane.root.canister_pool.handoff_receipts.v1";

        // Fleet Subnet Root aggregate Component Group provisioning authority.
        pub const ROOT_COMPONENT_PROVISIONING_OPERATIONS_KEY: &str =
            "canic.control_plane.root.component_provisioning.operations.v1";
        pub const ROOT_COMPONENT_PROVISIONING_PLACEMENTS_KEY: &str =
            "canic.control_plane.root.component_provisioning.placements.v1";
        pub const ROOT_COMPONENT_PROVISIONING_STATE_KEY: &str =
            "canic.control_plane.root.component_provisioning.state.v1";
    }

    pub mod runtime {
        pub const RUNTIME_CANISTER_CHILDREN_KEY: &str = "canic.core.runtime.canister_children.v1";
        pub const RUNTIME_BINDINGS_KEY: &str = "canic.core.runtime.bindings.v1";
    }

    pub mod fleet {
        pub const FLEET_STATE_KEY: &str = "canic.core.fleet.state.v1";
        pub const FLEET_ACTIVATION_KEY: &str = "canic.core.fleet.activation.v1";
    }

    pub mod auth {
        pub const LOCAL_APPLICATION_AUTHORIZATION_STATE_KEY: &str =
            "canic.core.auth.local_application_authorization.state.v1";
        pub const DELEGATED_TOKEN_ISSUER_STATE_KEY: &str =
            "canic.core.auth.delegated_token_issuer.state.v1";
        pub const ROOT_DELEGATION_STATE_KEY: &str = "canic.core.auth.root_delegation.state.v1";
    }

    pub mod replay {
        pub const REPLAY_RECEIPTS_KEY: &str = "canic.core.replay.receipts.v1";
    }

    pub mod cycles {
        pub const CYCLES_TRACKER_KEY: &str = "canic.core.cycles.tracker.v1";
        pub const CYCLES_TOPUP_EVENTS_KEY: &str = "canic.core.cycles.topup_events.v1";
        pub const CYCLES_FUNDING_LEDGER_KEY: &str = "canic.core.cycles.funding_ledger.v1";
        pub const CYCLES_ICP_REFILL_RECORDS_KEY: &str = "canic.core.cycles.icp_refill_records.v1";
    }

    pub mod log {
        pub const LOG_ENTRIES_KEY: &str = "canic.core.log.entries.v1";
    }

    pub mod intent {
        pub const INTENT_META_KEY: &str = "canic.core.intent.meta.v1";
        pub const INTENT_RECORDS_KEY: &str = "canic.core.intent.records.v1";
        pub const INTENT_TOTALS_KEY: &str = "canic.core.intent.totals.v1";
        pub const INTENT_PENDING_KEY: &str = "canic.core.intent.pending.v1";
        pub const INTENT_RECEIPT_BACKED_RECORDS_KEY: &str =
            "canic.core.intent.receipt_backed_records.v1";
        pub const INTENT_EXPIRY_INDEX_KEY: &str = "canic.core.intent.expiry_index.v1";
    }

    pub mod application_receipt {
        pub const APPLICATION_RECEIPT_ELIGIBILITY_KEY: &str =
            "canic.core.application_receipt.eligibility.v1";
    }

    pub mod caller_authority {
        pub const CALLER_AUTHORITY_HEADER_KEY: &str = "canic.core.caller_authority.header.v1";
        pub const CALLER_AUTHORITY_ROWS_KEY: &str = "canic.core.caller_authority.rows.v1";
    }

    pub mod placement {
        pub const PLACEMENT_ACKNOWLEDGEMENT_INDEX_KEY: &str =
            "canic.core.placement.acknowledgement_index.v1";
        pub const PLACEMENT_SCALING_REGISTRY_KEY: &str = "canic.core.placement.scaling_registry.v1";
        pub const PLACEMENT_INDEX_REGISTRY_KEY: &str = "canic.core.placement.index_registry.v1";
    }

    pub mod sharding {
        pub const SHARDING_REGISTRY_KEY: &str = "canic.core.sharding.registry.v1";
        pub const SHARDING_ASSIGNMENTS_KEY: &str = "canic.core.sharding.assignments.v1";
    }

    pub mod authority_restore {
        pub const AUTHORITY_RESTORE_FENCE_KEY: &str = "canic.core.authority_restore.fence.v1";
    }

    pub mod async_job_recovery {
        pub const ASYNC_JOB_RECOVERY_KEY: &str = "canic.core.async_job_recovery.v1";
    }

    pub mod fleet_admission_projection {
        pub const FLEET_ADMISSION_PROJECTION_KEY: &str = "canic.core.fleet_admission.projection.v1";
    }
}

const TEMPLATE_MANIFESTS_KEYS: &[&str] = &[TEMPLATE_MANIFESTS_KEY];
const TEMPLATE_CHUNK_SETS_KEYS: &[&str] = &[TEMPLATE_CHUNK_SETS_KEY];
const TEMPLATE_CHUNK_REFS_KEYS: &[&str] = &[TEMPLATE_CHUNK_REFS_KEY];
const TEMPLATE_CHUNK_PAYLOADS_KEYS: &[&str] = &[TEMPLATE_CHUNK_PAYLOADS_KEY];
const WASM_STORE_GC_STATE_KEYS: &[&str] = &[WASM_STORE_GC_STATE_KEY];
const FIXTURE_STORE_KEYS: &[&str] = &[FIXTURE_STORE_KEY];
const FLEET_COORDINATOR_REGISTRY_KEYS: &[&str] = &[FLEET_COORDINATOR_REGISTRY_KEY];
const FLEET_COORDINATOR_ADMISSION_KEYS: &[&str] = &[FLEET_COORDINATOR_ADMISSION_KEY];
const FLEET_COORDINATOR_FUNDING_KEYS: &[&str] = &[FLEET_COORDINATOR_FUNDING_KEY];
const ROOT_ADMISSION_KEYS: &[&str] = &[ROOT_ADMISSION_KEY];
const ROOT_FUNDING_KEYS: &[&str] = &[ROOT_FUNDING_KEY];
const ROOT_WASM_STORE_STATE_KEYS: &[&str] = &[ROOT_WASM_STORE_STATE_KEY];
const ROOT_FLEET_REGISTRY_MIRROR_KEYS: &[&str] = &[ROOT_FLEET_REGISTRY_MIRROR_KEY];
const ROOT_COMPONENT_REGISTRY_KEYS: &[&str] = &[
    ROOT_COMPONENT_REGISTRY_STATE_KEY,
    ROOT_COMPONENT_ALLOCATIONS_KEY,
    ROOT_COMPONENT_REGISTRY_ENTRIES_KEY,
    ROOT_COMPONENT_PRINCIPAL_INDEX_KEY,
    ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY_KEY,
    ROOT_COMPONENT_DRAINING_KEY,
];
const ROOT_CANISTER_POOL_KEYS: &[&str] = &[
    ROOT_CANISTER_INVENTORY_ASSETS_KEY,
    ROOT_CANISTER_POOL_STATE_KEY,
    ROOT_CANISTER_POOL_HANDOFF_RECEIPTS_KEY,
];
const ROOT_COMPONENT_PROVISIONING_KEYS: &[&str] = &[
    ROOT_COMPONENT_PROVISIONING_OPERATIONS_KEY,
    ROOT_COMPONENT_PROVISIONING_PLACEMENTS_KEY,
    ROOT_COMPONENT_PROVISIONING_STATE_KEY,
];

const CORE_RUNTIME_CHILDREN_KEYS: &[&str] = &[RUNTIME_CANISTER_CHILDREN_KEY];
const CORE_RUNTIME_BINDINGS_KEYS: &[&str] = &[RUNTIME_BINDINGS_KEY];
const CORE_FLEET_STATE_KEYS: &[&str] = &[FLEET_STATE_KEY];
const CORE_FLEET_ACTIVATION_KEYS: &[&str] = &[FLEET_ACTIVATION_KEY];
const CORE_CALLER_AUTHORITY_KEYS: &[&str] = &[
    memory::caller_authority::CALLER_AUTHORITY_HEADER_KEY,
    memory::caller_authority::CALLER_AUTHORITY_ROWS_KEY,
];
const CORE_DELEGATED_TOKEN_ISSUER_STATE_KEYS: &[&str] = &[DELEGATED_TOKEN_ISSUER_STATE_KEY];
const CORE_LOCAL_APPLICATION_AUTHORIZATION_STATE_KEYS: &[&str] =
    &[LOCAL_APPLICATION_AUTHORIZATION_STATE_KEY];
const CORE_ROOT_DELEGATION_STATE_KEYS: &[&str] = &[ROOT_DELEGATION_STATE_KEY];
const CORE_REPLAY_RECEIPTS_KEYS: &[&str] = &[REPLAY_RECEIPTS_KEY];
const CORE_CYCLES_KEYS: &[&str] = &[
    CYCLES_TRACKER_KEY,
    CYCLES_TOPUP_EVENTS_KEY,
    CYCLES_FUNDING_LEDGER_KEY,
];
const CORE_CYCLES_ICP_REFILL_RECORDS_KEYS: &[&str] = &[CYCLES_ICP_REFILL_RECORDS_KEY];
const CORE_RUNTIME_LOG_KEYS: &[&str] = &[LOG_ENTRIES_KEY];
const CORE_INTENT_KEYS: &[&str] = &[
    INTENT_META_KEY,
    INTENT_RECORDS_KEY,
    INTENT_TOTALS_KEY,
    INTENT_PENDING_KEY,
    INTENT_RECEIPT_BACKED_RECORDS_KEY,
    INTENT_EXPIRY_INDEX_KEY,
];
const CORE_APPLICATION_RECEIPT_KEYS: &[&str] = &[APPLICATION_RECEIPT_ELIGIBILITY_KEY];
const CORE_PLACEMENT_ACKNOWLEDGEMENT_KEYS: &[&str] = &[PLACEMENT_ACKNOWLEDGEMENT_INDEX_KEY];
const CORE_AUTHORITY_RESTORE_FENCE_KEYS: &[&str] = &[AUTHORITY_RESTORE_FENCE_KEY];
const CORE_ASYNC_JOB_RECOVERY_KEYS: &[&str] = &[ASYNC_JOB_RECOVERY_KEY];
const CORE_FLEET_ADMISSION_PROJECTION_KEYS: &[&str] = &[FLEET_ADMISSION_PROJECTION_KEY];
const PLACEMENT_SCALING_REGISTRY_KEYS: &[&str] = &[PLACEMENT_SCALING_REGISTRY_KEY];
const PLACEMENT_INDEX_REGISTRY_KEYS: &[&str] = &[PLACEMENT_INDEX_REGISTRY_KEY];
const SHARDING_REGISTRY_KEYS: &[&str] = &[SHARDING_REGISTRY_KEY];
const SHARDING_ASSIGNMENTS_KEYS: &[&str] = &[SHARDING_ASSIGNMENTS_KEY];

const ALLOCATION_DEFINITIONS: &[AllocationDefinition] = &[
    definition(
        StateAllocationKey::FixtureStore,
        AllocationOwner::CanicControlPlane,
        FIXTURE_STORE_KEYS,
    ),
    definition(
        StateAllocationKey::TemplateManifests,
        AllocationOwner::CanicControlPlane,
        TEMPLATE_MANIFESTS_KEYS,
    ),
    definition(
        StateAllocationKey::TemplateChunkSets,
        AllocationOwner::CanicControlPlane,
        TEMPLATE_CHUNK_SETS_KEYS,
    ),
    definition(
        StateAllocationKey::TemplateChunkRefs,
        AllocationOwner::CanicControlPlane,
        TEMPLATE_CHUNK_REFS_KEYS,
    ),
    definition(
        StateAllocationKey::TemplateChunkPayloads,
        AllocationOwner::CanicControlPlane,
        TEMPLATE_CHUNK_PAYLOADS_KEYS,
    ),
    definition(
        StateAllocationKey::WasmStoreGcState,
        AllocationOwner::CanicControlPlane,
        WASM_STORE_GC_STATE_KEYS,
    ),
    definition(
        StateAllocationKey::FleetCoordinatorAdmission,
        AllocationOwner::CanicControlPlane,
        FLEET_COORDINATOR_ADMISSION_KEYS,
    ),
    definition(
        StateAllocationKey::FleetCoordinatorFunding,
        AllocationOwner::CanicControlPlane,
        FLEET_COORDINATOR_FUNDING_KEYS,
    ),
    definition(
        StateAllocationKey::FleetCoordinatorRegistry,
        AllocationOwner::CanicControlPlane,
        FLEET_COORDINATOR_REGISTRY_KEYS,
    ),
    definition(
        StateAllocationKey::RootAdmission,
        AllocationOwner::CanicControlPlane,
        ROOT_ADMISSION_KEYS,
    ),
    definition(
        StateAllocationKey::RootFunding,
        AllocationOwner::CanicControlPlane,
        ROOT_FUNDING_KEYS,
    ),
    definition(
        StateAllocationKey::RootWasmStoreState,
        AllocationOwner::CanicControlPlane,
        ROOT_WASM_STORE_STATE_KEYS,
    ),
    definition(
        StateAllocationKey::RootFleetRegistryMirror,
        AllocationOwner::CanicControlPlane,
        ROOT_FLEET_REGISTRY_MIRROR_KEYS,
    ),
    definition(
        StateAllocationKey::RootComponentRegistry,
        AllocationOwner::CanicControlPlane,
        ROOT_COMPONENT_REGISTRY_KEYS,
    ),
    definition(
        StateAllocationKey::RootCanisterPool,
        AllocationOwner::CanicControlPlane,
        ROOT_CANISTER_POOL_KEYS,
    ),
    definition(
        StateAllocationKey::RootComponentProvisioning,
        AllocationOwner::CanicControlPlane,
        ROOT_COMPONENT_PROVISIONING_KEYS,
    ),
    definition(
        StateAllocationKey::CoreRuntimeChildren,
        AllocationOwner::CanicCore,
        CORE_RUNTIME_CHILDREN_KEYS,
    ),
    definition(
        StateAllocationKey::CoreRuntimeBindings,
        AllocationOwner::CanicCore,
        CORE_RUNTIME_BINDINGS_KEYS,
    ),
    definition(
        StateAllocationKey::CoreFleetState,
        AllocationOwner::CanicCore,
        CORE_FLEET_STATE_KEYS,
    ),
    definition(
        StateAllocationKey::CoreFleetActivation,
        AllocationOwner::CanicCore,
        CORE_FLEET_ACTIVATION_KEYS,
    ),
    definition(
        StateAllocationKey::CoreCallerAuthority,
        AllocationOwner::CanicCore,
        CORE_CALLER_AUTHORITY_KEYS,
    ),
    definition(
        StateAllocationKey::CoreLocalApplicationAuthorizationState,
        AllocationOwner::CanicCore,
        CORE_LOCAL_APPLICATION_AUTHORIZATION_STATE_KEYS,
    ),
    definition(
        StateAllocationKey::CoreDelegatedTokenIssuerState,
        AllocationOwner::CanicCore,
        CORE_DELEGATED_TOKEN_ISSUER_STATE_KEYS,
    ),
    definition(
        StateAllocationKey::CoreRootDelegationState,
        AllocationOwner::CanicCore,
        CORE_ROOT_DELEGATION_STATE_KEYS,
    ),
    definition(
        StateAllocationKey::CoreReplayReceipts,
        AllocationOwner::CanicCore,
        CORE_REPLAY_RECEIPTS_KEYS,
    ),
    definition(
        StateAllocationKey::CoreCycles,
        AllocationOwner::CanicCore,
        CORE_CYCLES_KEYS,
    ),
    definition(
        StateAllocationKey::CoreCyclesIcpRefillRecords,
        AllocationOwner::CanicCore,
        CORE_CYCLES_ICP_REFILL_RECORDS_KEYS,
    ),
    definition(
        StateAllocationKey::CoreRuntimeLog,
        AllocationOwner::CanicCore,
        CORE_RUNTIME_LOG_KEYS,
    ),
    definition(
        StateAllocationKey::CoreIntent,
        AllocationOwner::CanicCore,
        CORE_INTENT_KEYS,
    ),
    definition(
        StateAllocationKey::CoreApplicationReceipts,
        AllocationOwner::CanicCore,
        CORE_APPLICATION_RECEIPT_KEYS,
    ),
    definition(
        StateAllocationKey::CorePlacementAcknowledgement,
        AllocationOwner::CanicCore,
        CORE_PLACEMENT_ACKNOWLEDGEMENT_KEYS,
    ),
    definition(
        StateAllocationKey::PlacementScalingRegistry,
        AllocationOwner::CanicCore,
        PLACEMENT_SCALING_REGISTRY_KEYS,
    ),
    definition(
        StateAllocationKey::PlacementIndexRegistry,
        AllocationOwner::CanicCore,
        PLACEMENT_INDEX_REGISTRY_KEYS,
    ),
    definition(
        StateAllocationKey::ShardingRegistry,
        AllocationOwner::CanicCore,
        SHARDING_REGISTRY_KEYS,
    ),
    definition(
        StateAllocationKey::ShardingAssignments,
        AllocationOwner::CanicCore,
        SHARDING_ASSIGNMENTS_KEYS,
    ),
    definition(
        StateAllocationKey::CoreAuthorityRestoreFence,
        AllocationOwner::CanicCore,
        CORE_AUTHORITY_RESTORE_FENCE_KEYS,
    ),
    definition(
        StateAllocationKey::CoreAsyncJobRecovery,
        AllocationOwner::CanicCore,
        CORE_ASYNC_JOB_RECOVERY_KEYS,
    ),
    definition(
        StateAllocationKey::CoreFleetAdmissionProjection,
        AllocationOwner::CanicCore,
        CORE_FLEET_ADMISSION_PROJECTION_KEYS,
    ),
];

const fn definition(
    key: StateAllocationKey,
    owner: AllocationOwner,
    memory_keys: &'static [&'static str],
) -> AllocationDefinition {
    AllocationDefinition {
        key,
        owner,
        memory_keys,
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
        if definition.memory_keys.is_empty() {
            return Err(RoleContractFinding::CatalogInvalid {
                reason: format!("allocation has no memory keys: {:?}", definition.key),
            });
        }

        let prefix = match definition.owner {
            AllocationOwner::CanicCore => "canic.core.",
            AllocationOwner::CanicControlPlane => "canic.control_plane.",
        };
        for stable_key in definition.memory_keys {
            if !stable_key.starts_with(prefix) || ic_memory::StableKey::parse(stable_key).is_err() {
                return Err(RoleContractFinding::CatalogInvalid {
                    reason: format!(
                        "allocation {:?} has an invalid owner key {stable_key}",
                        definition.key
                    ),
                });
            }
            if let Some(first) = memory_owners.insert(*stable_key, definition.key) {
                return Err(RoleContractFinding::MemoryKeyCollision {
                    stable_key: stable_key.to_string(),
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
