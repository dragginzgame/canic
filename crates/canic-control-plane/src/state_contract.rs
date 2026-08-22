//! Module: state_contract
//!
//! Responsibility: declare control-plane stable state metadata keyed by the
//! canonical allocation registry.
//! Does not own: role applicability, CLI rendering, migration execution, or
//! stable-memory access.
//! Boundary: descriptors are static metadata supplied to host-side materialization.

#[cfg(feature = "root-control-plane")]
use crate::storage::stable::canister_pool::{
    CanisterPoolAssetRecord, CanisterPoolData, CanisterPoolHandoffReceiptData,
    CanisterPoolHandoffReceiptRecord, CanisterPoolStateRecord,
};
#[cfg(feature = "root-control-plane")]
use crate::storage::stable::component_provisioning::{
    RootComponentOperationRecord, RootComponentProvisioningData,
    RootComponentProvisioningPlacementRecord, RootComponentProvisioningStateRecord,
};
#[cfg(feature = "root-control-plane")]
use crate::storage::stable::root_funding::{RootFundingData, RootFundingRecord};
use crate::storage::stable::{
    component_registry::{
        ComponentRegistryEntryRecord, ComponentRegistryPrincipalIndexRecord,
        RootComponentAllocationRecord, RootComponentDrainingRecord, RootComponentRegistryData,
        RootComponentRegistryStateRecord, RootComponentSubtreeRemovalCompletedLeafRecord,
    },
    fleet_coordinator::{
        FleetCoordinatorFundingData, FleetCoordinatorFundingRecord, FleetCoordinatorRegistryData,
        FleetCoordinatorRegistryRecord,
    },
    fleet_registry_mirror::{RootFleetRegistryMirrorData, RootFleetRegistryMirrorStateRecord},
    state::root_wasm_store::{RootWasmStoreStateData, RootWasmStoreStateRecord},
    template::{
        TemplateChunkSetRecord, TemplateChunkSetsData, TemplateManifestRecord,
        TemplateManifestsData, WasmStoreGcStateData, WasmStoreGcStateRecord,
        chunked::{
            TemplateChunkPayloadRecord, TemplateChunkPayloadsData, TemplateChunkRefRecord,
            TemplateChunkRefsData,
        },
    },
};
#[cfg(feature = "root-control-plane")]
use canic_core::role_contract::allocation::memory::control_plane::{
    ROOT_CANISTER_INVENTORY_ASSETS_ID, ROOT_CANISTER_POOL_HANDOFF_RECEIPTS_ID,
    ROOT_CANISTER_POOL_STATE_ID, ROOT_COMPONENT_PROVISIONING_OPERATIONS_ID,
    ROOT_COMPONENT_PROVISIONING_PLACEMENTS_ID, ROOT_COMPONENT_PROVISIONING_STATE_ID,
    ROOT_FUNDING_ID,
};
use canic_core::{
    role_contract::{
        AllocationOwner, StateAllocationKey,
        allocation::memory::control_plane::{
            FLEET_COORDINATOR_FUNDING_ID, FLEET_COORDINATOR_REGISTRY_ID,
            ROOT_COMPONENT_ALLOCATIONS_ID, ROOT_COMPONENT_DRAINING_ID,
            ROOT_COMPONENT_PRINCIPAL_INDEX_ID, ROOT_COMPONENT_REGISTRY_ENTRIES_ID,
            ROOT_COMPONENT_REGISTRY_STATE_ID, ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY_ID,
            ROOT_FLEET_REGISTRY_MIRROR_ID, ROOT_WASM_STORE_STATE_ID, TEMPLATE_CHUNK_PAYLOADS_ID,
            TEMPLATE_CHUNK_REFS_ID, TEMPLATE_CHUNK_SETS_ID, TEMPLATE_MANIFESTS_ID,
            WASM_STORE_GC_STATE_ID,
        },
    },
    state_contract::{
        MigrationPolicy, StateAllocationDescriptor, StateDomainManifest, StateStorage,
    },
};

#[must_use]
pub fn canic_control_plane_state_descriptors() -> Vec<StateAllocationDescriptor> {
    vec![
        descriptor(
            StateAllocationKey::FleetCoordinatorRegistry,
            "fleet_coordinator_registry",
            FLEET_COORDINATOR_REGISTRY_ID,
            FleetCoordinatorRegistryRecord::STATE_CONTRACT_NAME,
            FleetCoordinatorRegistryData::STATE_CONTRACT_NAME,
            190,
            "fleet_coordinator_registry_restores_exact_authority_and_canonical_head",
        ),
        descriptor(
            StateAllocationKey::FleetCoordinatorFunding,
            "fleet_coordinator_funding",
            FLEET_COORDINATOR_FUNDING_ID,
            FleetCoordinatorFundingRecord::STATE_CONTRACT_NAME,
            FleetCoordinatorFundingData::STATE_CONTRACT_NAME,
            191,
            "fleet_coordinator_funding_restores_exact_reservations_and_terminal_results",
        ),
        #[cfg(feature = "root-control-plane")]
        descriptor(
            StateAllocationKey::RootFunding,
            "root_funding",
            ROOT_FUNDING_ID,
            RootFundingRecord::STATE_CONTRACT_NAME,
            RootFundingData::STATE_CONTRACT_NAME,
            192,
            "root_funding_restores_exact_current_request_and_terminal_result",
        ),
        descriptor(
            StateAllocationKey::RootFleetRegistryMirror,
            "root_fleet_registry_mirror",
            ROOT_FLEET_REGISTRY_MIRROR_ID,
            RootFleetRegistryMirrorStateRecord::STATE_CONTRACT_NAME,
            RootFleetRegistryMirrorData::STATE_CONTRACT_NAME,
            195,
            "root_fleet_registry_mirror_restores_exclusive_candidate_or_active_directory",
        ),
        root_component_registry_descriptor(),
        #[cfg(feature = "root-control-plane")]
        root_canister_pool_descriptor(),
        #[cfg(feature = "root-control-plane")]
        root_component_provisioning_descriptor(),
        descriptor(
            StateAllocationKey::TemplateManifests,
            "template_manifests",
            TEMPLATE_MANIFESTS_ID,
            TemplateManifestRecord::STATE_CONTRACT_NAME,
            TemplateManifestsData::STATE_CONTRACT_NAME,
            202,
            "template_manifests_restore_release_index",
        ),
        descriptor(
            StateAllocationKey::TemplateChunkSets,
            "template_chunk_sets",
            TEMPLATE_CHUNK_SETS_ID,
            TemplateChunkSetRecord::STATE_CONTRACT_NAME,
            TemplateChunkSetsData::STATE_CONTRACT_NAME,
            210,
            "template_chunk_sets_restore_release_metadata",
        ),
        descriptor(
            StateAllocationKey::TemplateChunkRefs,
            "template_chunk_refs",
            TEMPLATE_CHUNK_REFS_ID,
            TemplateChunkRefRecord::STATE_CONTRACT_NAME,
            TemplateChunkRefsData::STATE_CONTRACT_NAME,
            220,
            "template_chunk_refs_restore_chunk_slots",
        ),
        descriptor(
            StateAllocationKey::TemplateChunkPayloads,
            "template_chunk_payloads",
            TEMPLATE_CHUNK_PAYLOADS_ID,
            TemplateChunkPayloadRecord::STATE_CONTRACT_NAME,
            TemplateChunkPayloadsData::STATE_CONTRACT_NAME,
            230,
            "template_chunk_payloads_restore_chunk_bytes",
        ),
        descriptor(
            StateAllocationKey::RootWasmStoreState,
            "root_wasm_store_state",
            ROOT_WASM_STORE_STATE_ID,
            RootWasmStoreStateRecord::STATE_CONTRACT_NAME,
            RootWasmStoreStateData::STATE_CONTRACT_NAME,
            240,
            "root_wasm_store_state_restores_publication_inventory_and_creation_authority",
        ),
        descriptor(
            StateAllocationKey::WasmStoreGcState,
            "wasm_store_gc_state",
            WASM_STORE_GC_STATE_ID,
            WasmStoreGcStateRecord::STATE_CONTRACT_NAME,
            WasmStoreGcStateData::STATE_CONTRACT_NAME,
            240,
            "wasm_store_gc_state_restores_local_gc_mode",
        ),
    ]
}

fn root_component_registry_descriptor() -> StateAllocationDescriptor {
    StateAllocationDescriptor {
        allocation: StateAllocationKey::RootComponentRegistry,
        owner: AllocationOwner::CanicControlPlane,
        state: vec![
            StateDomainManifest {
                domain: "root_component_registry_state".to_string(),
                version: 1,
                storage: StateStorage::StableMemory,
                memory_id: Some(ROOT_COMPONENT_REGISTRY_STATE_ID),
                owner: AllocationOwner::CanicControlPlane.as_str().to_string(),
                record: RootComponentRegistryStateRecord::STATE_CONTRACT_NAME.to_string(),
                snapshot: RootComponentRegistryData::STATE_CONTRACT_NAME.to_string(),
                min_supported_version: 1,
                migration_policy: MigrationPolicy::NewDomain,
                restore_order: Some(196),
                post_upgrade_invariant: Some(
                    "root_component_registry_restores_exact_preparation_authority_and_allocation_sequence"
                        .to_string(),
                ),
                migrations: Vec::new(),
            },
            StateDomainManifest {
                domain: "root_component_allocations".to_string(),
                version: 1,
                storage: StateStorage::StableMemory,
                memory_id: Some(ROOT_COMPONENT_ALLOCATIONS_ID),
                owner: AllocationOwner::CanicControlPlane.as_str().to_string(),
                record: RootComponentAllocationRecord::STATE_CONTRACT_NAME.to_string(),
                snapshot: RootComponentRegistryData::STATE_CONTRACT_NAME.to_string(),
                min_supported_version: 1,
                migration_policy: MigrationPolicy::NewDomain,
                restore_order: Some(197),
                post_upgrade_invariant: Some(
                    "root_component_allocations_restore_exact_operation_identity_and_capacity"
                        .to_string(),
                ),
                migrations: Vec::new(),
            },
            StateDomainManifest {
                domain: "component_registry_entries".to_string(),
                version: 1,
                storage: StateStorage::StableMemory,
                memory_id: Some(ROOT_COMPONENT_REGISTRY_ENTRIES_ID),
                owner: AllocationOwner::CanicControlPlane.as_str().to_string(),
                record: ComponentRegistryEntryRecord::STATE_CONTRACT_NAME.to_string(),
                snapshot: RootComponentRegistryData::STATE_CONTRACT_NAME.to_string(),
                min_supported_version: 1,
                migration_policy: MigrationPolicy::NewDomain,
                restore_order: Some(198),
                post_upgrade_invariant: Some(
                    "component_registry_entries_restore_exact_heads_reservations_and_counts"
                        .to_string(),
                ),
                migrations: Vec::new(),
            },
            StateDomainManifest {
                domain: "component_registry_principal_index".to_string(),
                version: 1,
                storage: StateStorage::StableMemory,
                memory_id: Some(ROOT_COMPONENT_PRINCIPAL_INDEX_ID),
                owner: AllocationOwner::CanicControlPlane.as_str().to_string(),
                record: ComponentRegistryPrincipalIndexRecord::STATE_CONTRACT_NAME.to_string(),
                snapshot: RootComponentRegistryData::STATE_CONTRACT_NAME.to_string(),
                min_supported_version: 1,
                migration_policy: MigrationPolicy::NewDomain,
                restore_order: Some(199),
                post_upgrade_invariant: Some(
                    "component_registry_principal_index_restores_unique_committed_bindings"
                        .to_string(),
                ),
                migrations: Vec::new(),
            },
            StateDomainManifest {
                domain: "root_component_subtree_removal_history".to_string(),
                version: 1,
                storage: StateStorage::StableMemory,
                memory_id: Some(ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY_ID),
                owner: AllocationOwner::CanicControlPlane.as_str().to_string(),
                record: RootComponentSubtreeRemovalCompletedLeafRecord::STATE_CONTRACT_NAME
                    .to_string(),
                snapshot: RootComponentRegistryData::STATE_CONTRACT_NAME.to_string(),
                min_supported_version: 1,
                migration_policy: MigrationPolicy::NewDomain,
                restore_order: Some(200),
                post_upgrade_invariant: Some(
                    "root_component_subtree_removal_history_restores_exact_operation_step_receipts"
                        .to_string(),
                ),
                migrations: Vec::new(),
            },
            root_component_draining_domain(),
        ],
        reserved_memory: Vec::new(),
    }
}

fn root_component_draining_domain() -> StateDomainManifest {
    StateDomainManifest {
        domain: "root_component_draining".to_string(),
        version: 1,
        storage: StateStorage::StableMemory,
        memory_id: Some(ROOT_COMPONENT_DRAINING_ID),
        owner: AllocationOwner::CanicControlPlane.as_str().to_string(),
        record: RootComponentDrainingRecord::STATE_CONTRACT_NAME.to_string(),
        snapshot: RootComponentRegistryData::STATE_CONTRACT_NAME.to_string(),
        min_supported_version: 1,
        migration_policy: MigrationPolicy::NewDomain,
        restore_order: Some(201),
        post_upgrade_invariant: Some(
            "root_component_draining_restores_exact_funding_fence_quiescence_cursor_final_inventory_deletion_and_membership_removal"
                .to_string(),
        ),
        migrations: Vec::new(),
    }
}

#[cfg(feature = "root-control-plane")]
fn root_canister_pool_descriptor() -> StateAllocationDescriptor {
    StateAllocationDescriptor {
        allocation: StateAllocationKey::RootCanisterPool,
        owner: AllocationOwner::CanicControlPlane,
        state: vec![
            StateDomainManifest {
                domain: "root_canister_pool_assets".to_string(),
                version: 1,
                storage: StateStorage::StableMemory,
                memory_id: Some(ROOT_CANISTER_INVENTORY_ASSETS_ID),
                owner: AllocationOwner::CanicControlPlane.as_str().to_string(),
                record: CanisterPoolAssetRecord::STATE_CONTRACT_NAME.to_string(),
                snapshot: CanisterPoolData::STATE_CONTRACT_NAME.to_string(),
                min_supported_version: 1,
                migration_policy: MigrationPolicy::NewDomain,
                restore_order: Some(202),
                post_upgrade_invariant: Some(
                    "root_canister_pool_assets_restore_exact_lifecycle_and_component_claims"
                        .to_string(),
                ),
                migrations: Vec::new(),
            },
            StateDomainManifest {
                domain: "root_canister_pool_state".to_string(),
                version: 1,
                storage: StateStorage::StableMemory,
                memory_id: Some(ROOT_CANISTER_POOL_STATE_ID),
                owner: AllocationOwner::CanicControlPlane.as_str().to_string(),
                record: CanisterPoolStateRecord::STATE_CONTRACT_NAME.to_string(),
                snapshot: CanisterPoolData::STATE_CONTRACT_NAME.to_string(),
                min_supported_version: 1,
                migration_policy: MigrationPolicy::NewDomain,
                restore_order: Some(203),
                post_upgrade_invariant: Some(
                    "root_canister_pool_creation_restores_exact_paid_effect_authority".to_string(),
                ),
                migrations: Vec::new(),
            },
            StateDomainManifest {
                domain: "root_canister_pool_handoff_receipts".to_string(),
                version: 1,
                storage: StateStorage::StableMemory,
                memory_id: Some(ROOT_CANISTER_POOL_HANDOFF_RECEIPTS_ID),
                owner: AllocationOwner::CanicControlPlane.as_str().to_string(),
                record: CanisterPoolHandoffReceiptRecord::STATE_CONTRACT_NAME.to_string(),
                snapshot: CanisterPoolHandoffReceiptData::STATE_CONTRACT_NAME.to_string(),
                min_supported_version: 1,
                migration_policy: MigrationPolicy::NewDomain,
                restore_order: Some(204),
                post_upgrade_invariant: Some(
                    "root_canister_pool_handoff_receipts_restore_exact_terminal_replay".to_string(),
                ),
                migrations: Vec::new(),
            },
        ],
        reserved_memory: Vec::new(),
    }
}

#[cfg(feature = "root-control-plane")]
fn root_component_provisioning_descriptor() -> StateAllocationDescriptor {
    StateAllocationDescriptor {
        allocation: StateAllocationKey::RootComponentProvisioning,
        owner: AllocationOwner::CanicControlPlane,
        state: vec![
            StateDomainManifest {
                domain: "root_component_provisioning_operations".to_string(),
                version: 1,
                storage: StateStorage::StableMemory,
                memory_id: Some(ROOT_COMPONENT_PROVISIONING_OPERATIONS_ID),
                owner: AllocationOwner::CanicControlPlane.as_str().to_string(),
                record: RootComponentOperationRecord::STATE_CONTRACT_NAME.to_string(),
                snapshot: RootComponentProvisioningData::STATE_CONTRACT_NAME.to_string(),
                min_supported_version: 1,
                migration_policy: MigrationPolicy::NewDomain,
                restore_order: Some(205),
                post_upgrade_invariant: Some(
                    "root_component_provisioning_operations_restore_exact_intent_and_receipts"
                        .to_string(),
                ),
                migrations: Vec::new(),
            },
            StateDomainManifest {
                domain: "root_component_provisioning_placements".to_string(),
                version: 1,
                storage: StateStorage::StableMemory,
                memory_id: Some(ROOT_COMPONENT_PROVISIONING_PLACEMENTS_ID),
                owner: AllocationOwner::CanicControlPlane.as_str().to_string(),
                record: RootComponentProvisioningPlacementRecord::STATE_CONTRACT_NAME.to_string(),
                snapshot: RootComponentProvisioningData::STATE_CONTRACT_NAME.to_string(),
                min_supported_version: 1,
                migration_policy: MigrationPolicy::NewDomain,
                restore_order: Some(206),
                post_upgrade_invariant: Some(
                    "root_component_provisioning_placements_restore_permanent_unique_reservations"
                        .to_string(),
                ),
                migrations: Vec::new(),
            },
            StateDomainManifest {
                domain: "root_component_provisioning_state".to_string(),
                version: 1,
                storage: StateStorage::StableMemory,
                memory_id: Some(ROOT_COMPONENT_PROVISIONING_STATE_ID),
                owner: AllocationOwner::CanicControlPlane.as_str().to_string(),
                record: RootComponentProvisioningStateRecord::STATE_CONTRACT_NAME.to_string(),
                snapshot: RootComponentProvisioningData::STATE_CONTRACT_NAME.to_string(),
                min_supported_version: 1,
                migration_policy: MigrationPolicy::NewDomain,
                restore_order: Some(207),
                post_upgrade_invariant: Some(
                    "root_component_provisioning_state_restores_capacity_and_active_operation_fence"
                        .to_string(),
                ),
                migrations: Vec::new(),
            },
        ],
        reserved_memory: Vec::new(),
    }
}

fn descriptor(
    allocation: StateAllocationKey,
    domain: &str,
    memory_id: u8,
    record: &str,
    snapshot: &str,
    restore_order: u32,
    invariant: &str,
) -> StateAllocationDescriptor {
    StateAllocationDescriptor {
        allocation,
        owner: AllocationOwner::CanicControlPlane,
        state: vec![StateDomainManifest {
            domain: domain.to_string(),
            version: 1,
            storage: StateStorage::StableMemory,
            memory_id: Some(memory_id),
            owner: AllocationOwner::CanicControlPlane.as_str().to_string(),
            record: record.to_string(),
            snapshot: snapshot.to_string(),
            min_supported_version: 1,
            migration_policy: MigrationPolicy::NewDomain,
            restore_order: Some(restore_order),
            post_upgrade_invariant: Some(invariant.to_string()),
            migrations: Vec::new(),
        }],
        reserved_memory: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptors_declare_template_control_plane_and_gc_allocations() {
        let descriptors = canic_control_plane_state_descriptors();
        let keys = descriptors
            .iter()
            .map(|descriptor| descriptor.allocation)
            .collect::<Vec<_>>();

        for expected in [
            StateAllocationKey::FleetCoordinatorFunding,
            StateAllocationKey::FleetCoordinatorRegistry,
            StateAllocationKey::RootComponentRegistry,
            StateAllocationKey::RootFleetRegistryMirror,
            StateAllocationKey::RootCanisterPool,
            StateAllocationKey::RootComponentProvisioning,
            StateAllocationKey::TemplateManifests,
            StateAllocationKey::TemplateChunkSets,
            StateAllocationKey::TemplateChunkRefs,
            StateAllocationKey::TemplateChunkPayloads,
            StateAllocationKey::RootWasmStoreState,
            StateAllocationKey::WasmStoreGcState,
        ] {
            assert!(keys.contains(&expected));
        }
    }

    #[test]
    fn descriptors_reference_canonical_control_plane_data_types() {
        let descriptors = canic_control_plane_state_descriptors();

        for (allocation, record, snapshot) in [
            (
                StateAllocationKey::FleetCoordinatorFunding,
                FleetCoordinatorFundingRecord::STATE_CONTRACT_NAME,
                FleetCoordinatorFundingData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::FleetCoordinatorRegistry,
                FleetCoordinatorRegistryRecord::STATE_CONTRACT_NAME,
                FleetCoordinatorRegistryData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::RootComponentRegistry,
                RootComponentRegistryStateRecord::STATE_CONTRACT_NAME,
                RootComponentRegistryData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::RootFleetRegistryMirror,
                RootFleetRegistryMirrorStateRecord::STATE_CONTRACT_NAME,
                RootFleetRegistryMirrorData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::RootComponentProvisioning,
                RootComponentOperationRecord::STATE_CONTRACT_NAME,
                RootComponentProvisioningData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::TemplateManifests,
                TemplateManifestRecord::STATE_CONTRACT_NAME,
                TemplateManifestsData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::TemplateChunkSets,
                TemplateChunkSetRecord::STATE_CONTRACT_NAME,
                TemplateChunkSetsData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::TemplateChunkRefs,
                TemplateChunkRefRecord::STATE_CONTRACT_NAME,
                TemplateChunkRefsData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::TemplateChunkPayloads,
                TemplateChunkPayloadRecord::STATE_CONTRACT_NAME,
                TemplateChunkPayloadsData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::RootWasmStoreState,
                RootWasmStoreStateRecord::STATE_CONTRACT_NAME,
                RootWasmStoreStateData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::WasmStoreGcState,
                WasmStoreGcStateRecord::STATE_CONTRACT_NAME,
                WasmStoreGcStateData::STATE_CONTRACT_NAME,
            ),
        ] {
            let descriptor = descriptors
                .iter()
                .find(|descriptor| descriptor.allocation == allocation)
                .expect("control-plane state descriptor");
            let declaration = descriptor.state.first().expect("state declaration");

            assert_eq!(declaration.record, record);
            assert_eq!(declaration.snapshot, snapshot);
        }
    }

    #[test]
    fn root_component_registry_declares_every_normalized_domain() {
        let descriptors = canic_control_plane_state_descriptors();
        let descriptor = descriptors
            .iter()
            .find(|descriptor| descriptor.allocation == StateAllocationKey::RootComponentRegistry)
            .expect("root Component Registry descriptor");

        assert_eq!(descriptor.state.len(), 6);
        assert_eq!(
            descriptor
                .state
                .iter()
                .map(|domain| (
                    domain.domain.as_str(),
                    domain.memory_id,
                    domain.record.as_str(),
                    domain.restore_order,
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "root_component_registry_state",
                    Some(ROOT_COMPONENT_REGISTRY_STATE_ID),
                    RootComponentRegistryStateRecord::STATE_CONTRACT_NAME,
                    Some(196),
                ),
                (
                    "root_component_allocations",
                    Some(ROOT_COMPONENT_ALLOCATIONS_ID),
                    RootComponentAllocationRecord::STATE_CONTRACT_NAME,
                    Some(197),
                ),
                (
                    "component_registry_entries",
                    Some(ROOT_COMPONENT_REGISTRY_ENTRIES_ID),
                    ComponentRegistryEntryRecord::STATE_CONTRACT_NAME,
                    Some(198),
                ),
                (
                    "component_registry_principal_index",
                    Some(ROOT_COMPONENT_PRINCIPAL_INDEX_ID),
                    ComponentRegistryPrincipalIndexRecord::STATE_CONTRACT_NAME,
                    Some(199),
                ),
                (
                    "root_component_subtree_removal_history",
                    Some(ROOT_COMPONENT_SUBTREE_REMOVAL_HISTORY_ID),
                    RootComponentSubtreeRemovalCompletedLeafRecord::STATE_CONTRACT_NAME,
                    Some(200),
                ),
                (
                    "root_component_draining",
                    Some(ROOT_COMPONENT_DRAINING_ID),
                    RootComponentDrainingRecord::STATE_CONTRACT_NAME,
                    Some(201),
                ),
            ]
        );
    }

    #[test]
    fn root_component_provisioning_declares_exact_consecutive_domains() {
        let descriptors = canic_control_plane_state_descriptors();
        let descriptor = descriptors
            .iter()
            .find(|descriptor| {
                descriptor.allocation == StateAllocationKey::RootComponentProvisioning
            })
            .expect("root Component provisioning descriptor");

        assert_eq!(
            descriptor
                .state
                .iter()
                .map(|domain| (
                    domain.domain.as_str(),
                    domain.memory_id,
                    domain.record.as_str(),
                    domain.restore_order,
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "root_component_provisioning_operations",
                    Some(ROOT_COMPONENT_PROVISIONING_OPERATIONS_ID),
                    RootComponentOperationRecord::STATE_CONTRACT_NAME,
                    Some(205),
                ),
                (
                    "root_component_provisioning_placements",
                    Some(ROOT_COMPONENT_PROVISIONING_PLACEMENTS_ID),
                    RootComponentProvisioningPlacementRecord::STATE_CONTRACT_NAME,
                    Some(206),
                ),
                (
                    "root_component_provisioning_state",
                    Some(ROOT_COMPONENT_PROVISIONING_STATE_ID),
                    RootComponentProvisioningStateRecord::STATE_CONTRACT_NAME,
                    Some(207),
                ),
            ]
        );
    }
}
