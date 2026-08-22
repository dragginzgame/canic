use super::*;
use canic_core::role_contract::{
    AllocationOwner, BuiltInRoleKind, ResolvedStateAllocation, SelectionProvenance,
};

#[test]
fn complete_registry_has_exactly_one_descriptor_for_every_allocation() {
    let registry = validate_state_descriptor_registry().expect("valid descriptor registry");
    assert_eq!(
        registry.descriptors().count(),
        allocation_definitions().len()
    );
}

#[test]
fn duplicate_descriptor_is_blocking() {
    let mut descriptors = canic_state_descriptors()
        .into_iter()
        .chain(canic_control_plane_state_descriptors())
        .collect::<Vec<_>>();
    descriptors.push(descriptors[0].clone());

    assert!(matches!(
        validate_descriptors(descriptors),
        Err(errors) if errors.iter().any(|finding| matches!(
            finding,
            RoleContractFinding::AllocationDescriptorDuplicate { .. }
        ))
    ));
}

#[test]
fn descriptor_id_drift_is_blocking() {
    let mut descriptors = canic_state_descriptors()
        .into_iter()
        .chain(canic_control_plane_state_descriptors())
        .collect::<Vec<_>>();
    let stored_blobs = descriptors
        .iter_mut()
        .find(|descriptor| descriptor.allocation == StateAllocationKey::BlobStorageRoots)
        .expect("stored blobs descriptor");
    stored_blobs.state[0].memory_id = Some(61);

    assert!(matches!(
        validate_descriptors(descriptors),
        Err(errors) if errors.iter().any(|finding| matches!(
            finding,
            RoleContractFinding::AllocationDescriptorIdMismatch {
                key: StateAllocationKey::BlobStorageRoots,
                ..
            }
        ))
    ));
}

#[test]
fn materialization_joins_only_selected_allocations() {
    let contract = ResolvedRoleContract {
        role: canic_core::ids::CanisterRole::owned("blobber".to_string()),
        built_in: None,
        capabilities: BTreeSet::new(),
        required_features: BTreeSet::new(),
        effective_features: BTreeSet::new(),
        allocations: vec![ResolvedStateAllocation {
            key: StateAllocationKey::BlobStorageRoots,
            owner: AllocationOwner::CanicCore,
            memory_ids: vec![MemoryId::new(55)],
            selected_by: BTreeSet::from([SelectionProvenance::EffectiveFeature(
                canic_core::role_contract::CanicFeatureKey::BlobStorage,
            )]),
        }],
    };

    let manifest = materialize_state_manifest(&[contract]).expect("manifest");
    let role = manifest.roles.first().expect("role");
    assert_eq!(role.canister_role, "blobber");
    assert_eq!(role.state.len(), 1);
    assert_eq!(role.state[0].domain, "blob_storage_roots");
}

#[test]
fn wasm_store_materializes_template_and_gc_state() {
    let keys = [
        StateAllocationKey::TemplateManifests,
        StateAllocationKey::TemplateChunkSets,
        StateAllocationKey::TemplateChunkRefs,
        StateAllocationKey::TemplateChunkPayloads,
        StateAllocationKey::WasmStoreGcState,
    ];
    let allocations = keys
        .into_iter()
        .map(|key| {
            let definition = allocation_definitions()
                .iter()
                .find(|definition| definition.key == key)
                .expect("definition");
            ResolvedStateAllocation {
                key,
                owner: definition.owner,
                memory_ids: definition.memory_ids.to_vec(),
                selected_by: BTreeSet::from([SelectionProvenance::BuiltInRole(
                    BuiltInRoleKind::WasmStore,
                )]),
            }
        })
        .collect();
    let contract = ResolvedRoleContract {
        role: canic_core::ids::CanisterRole::WASM_STORE,
        built_in: Some(BuiltInRoleKind::WasmStore),
        capabilities: BTreeSet::new(),
        required_features: BTreeSet::new(),
        effective_features: BTreeSet::new(),
        allocations,
    };

    let manifest = materialize_state_manifest(&[contract]).expect("manifest");
    let ids = manifest.roles[0]
        .state
        .iter()
        .filter_map(|domain| domain.memory_id)
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 5);
    for expected in [10, 11, 12, 13, 14] {
        assert!(ids.contains(&expected));
    }
}

#[test]
fn fleet_coordinator_materializes_its_registry_and_funding_state() {
    let keys = [
        StateAllocationKey::FleetCoordinatorFunding,
        StateAllocationKey::FleetCoordinatorRegistry,
    ];
    let contract = ResolvedRoleContract {
        role: canic_core::ids::CanisterRole::FLEET_COORDINATOR,
        built_in: Some(BuiltInRoleKind::FleetCoordinator),
        capabilities: BTreeSet::new(),
        required_features: BTreeSet::new(),
        effective_features: BTreeSet::new(),
        allocations: keys
            .into_iter()
            .map(|key| {
                let definition = allocation_definitions()
                    .iter()
                    .find(|definition| definition.key == key)
                    .expect("definition");
                ResolvedStateAllocation {
                    key,
                    owner: definition.owner,
                    memory_ids: definition.memory_ids.to_vec(),
                    selected_by: BTreeSet::from([SelectionProvenance::BuiltInRole(
                        BuiltInRoleKind::FleetCoordinator,
                    )]),
                }
            })
            .collect(),
    };

    let manifest = materialize_state_manifest(&[contract]).expect("manifest");
    assert_eq!(manifest.roles.len(), 1);
    assert_eq!(manifest.roles[0].state.len(), 2);
    assert_eq!(
        manifest.roles[0]
            .state
            .iter()
            .filter_map(|domain| domain.memory_id)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([15, 62])
    );
}

#[test]
fn root_materializes_its_independent_funding_journal() {
    let key = StateAllocationKey::RootFunding;
    let definition = allocation_definitions()
        .iter()
        .find(|definition| definition.key == key)
        .expect("Root funding definition");
    let contract = ResolvedRoleContract {
        role: canic_core::ids::CanisterRole::ROOT,
        built_in: None,
        capabilities: BTreeSet::new(),
        required_features: BTreeSet::new(),
        effective_features: BTreeSet::new(),
        allocations: vec![ResolvedStateAllocation {
            key,
            owner: definition.owner,
            memory_ids: definition.memory_ids.to_vec(),
            selected_by: BTreeSet::from([SelectionProvenance::Capability(
                canic_core::role_contract::RoleCapabilityKey::RootControlPlane,
            )]),
        }],
    };

    let manifest = materialize_state_manifest(&[contract]).expect("manifest");
    assert_eq!(manifest.roles.len(), 1);
    assert_eq!(manifest.roles[0].state.len(), 1);
    assert_eq!(manifest.roles[0].state[0].domain, "root_funding");
    assert_eq!(manifest.roles[0].state[0].memory_id, Some(63));
}
