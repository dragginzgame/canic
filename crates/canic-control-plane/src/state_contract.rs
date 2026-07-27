//! Module: state_contract
//!
//! Responsibility: declare control-plane stable state metadata keyed by the
//! canonical allocation registry.
//! Does not own: role applicability, CLI rendering, migration execution, or
//! stable-memory access.
//! Boundary: descriptors are static metadata supplied to host-side materialization.

use crate::storage::stable::{
    component_registry::{
        RootComponentAllocationRecord, RootComponentRegistryData, RootComponentRegistryStateRecord,
    },
    fleet_coordinator::{FleetCoordinatorRegistryData, FleetCoordinatorRegistryRecord},
    fleet_registry_mirror::{RootFleetRegistryMirrorData, RootFleetRegistryMirrorStateRecord},
    state::subnet::{ControlPlaneSubnetStateData, SubnetStateRecord},
    template::{
        TemplateChunkSetRecord, TemplateChunkSetsData, TemplateManifestRecord,
        TemplateManifestsData, WasmStoreGcStateData, WasmStoreGcStateRecord,
        chunked::{
            TemplateChunkPayloadRecord, TemplateChunkPayloadsData, TemplateChunkRefRecord,
            TemplateChunkRefsData,
        },
    },
};
use canic_core::{
    role_contract::{
        AllocationOwner, StateAllocationKey,
        allocation::memory::template::{
            CONTROL_PLANE_SUBNET_STATE_ID, FLEET_COORDINATOR_REGISTRY_ID,
            ROOT_COMPONENT_ALLOCATIONS_ID, ROOT_COMPONENT_REGISTRY_META_ID,
            ROOT_FLEET_REGISTRY_MIRROR_ID, TEMPLATE_CHUNK_PAYLOADS_ID, TEMPLATE_CHUNK_REFS_ID,
            TEMPLATE_CHUNK_SETS_ID, TEMPLATE_MANIFESTS_ID, WASM_STORE_GC_STATE_ID,
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
            StateAllocationKey::RootFleetRegistryMirror,
            "root_fleet_registry_mirror",
            ROOT_FLEET_REGISTRY_MIRROR_ID,
            RootFleetRegistryMirrorStateRecord::STATE_CONTRACT_NAME,
            RootFleetRegistryMirrorData::STATE_CONTRACT_NAME,
            195,
            "root_fleet_registry_mirror_restores_exclusive_candidate_or_active_directory",
        ),
        root_component_registry_descriptor(),
        descriptor(
            StateAllocationKey::TemplateManifests,
            "template_manifests",
            TEMPLATE_MANIFESTS_ID,
            TemplateManifestRecord::STATE_CONTRACT_NAME,
            TemplateManifestsData::STATE_CONTRACT_NAME,
            200,
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
            StateAllocationKey::ControlPlaneSubnetState,
            "control_plane_subnet_state",
            CONTROL_PLANE_SUBNET_STATE_ID,
            SubnetStateRecord::STATE_CONTRACT_NAME,
            ControlPlaneSubnetStateData::STATE_CONTRACT_NAME,
            240,
            "control_plane_subnet_state_restores_publication_bindings",
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
                domain: "root_component_registry".to_string(),
                version: 1,
                storage: StateStorage::StableMemory,
                memory_id: Some(ROOT_COMPONENT_REGISTRY_META_ID),
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
            StateAllocationKey::FleetCoordinatorRegistry,
            StateAllocationKey::RootComponentRegistry,
            StateAllocationKey::RootFleetRegistryMirror,
            StateAllocationKey::TemplateManifests,
            StateAllocationKey::TemplateChunkSets,
            StateAllocationKey::TemplateChunkRefs,
            StateAllocationKey::TemplateChunkPayloads,
            StateAllocationKey::ControlPlaneSubnetState,
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
                StateAllocationKey::ControlPlaneSubnetState,
                SubnetStateRecord::STATE_CONTRACT_NAME,
                ControlPlaneSubnetStateData::STATE_CONTRACT_NAME,
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
    fn root_component_registry_declares_meta_and_allocation_domains() {
        let descriptors = canic_control_plane_state_descriptors();
        let descriptor = descriptors
            .iter()
            .find(|descriptor| descriptor.allocation == StateAllocationKey::RootComponentRegistry)
            .expect("root Component Registry descriptor");

        assert_eq!(descriptor.state.len(), 2);
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
                    "root_component_registry",
                    Some(ROOT_COMPONENT_REGISTRY_META_ID),
                    RootComponentRegistryStateRecord::STATE_CONTRACT_NAME,
                    Some(196),
                ),
                (
                    "root_component_allocations",
                    Some(ROOT_COMPONENT_ALLOCATIONS_ID),
                    RootComponentAllocationRecord::STATE_CONTRACT_NAME,
                    Some(197),
                ),
            ]
        );
    }
}
