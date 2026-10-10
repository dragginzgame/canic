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
    let registry = descriptors
        .iter_mut()
        .find(|descriptor| descriptor.allocation == StateAllocationKey::ShardingRegistry)
        .expect("sharding registry descriptor");
    registry.state[0].memory_key = Some("canic.core.fleet_admission.projection.v1".to_string());

    assert!(matches!(
        validate_descriptors(descriptors),
        Err(errors) if errors.iter().any(|finding| matches!(
            finding,
            RoleContractFinding::AllocationDescriptorKeyMismatch {
                key: StateAllocationKey::ShardingRegistry,
                ..
            }
        ))
    ));
}

#[test]
fn materialization_joins_only_selected_allocations() {
    let contract = ResolvedRoleContract {
        role: canic_contracts::ids::CanisterRole::owned("shard".to_string()),
        built_in: None,
        capabilities: BTreeSet::new(),
        required_features: BTreeSet::new(),
        effective_features: BTreeSet::new(),
        allocations: vec![ResolvedStateAllocation {
            key: StateAllocationKey::ShardingRegistry,
            owner: AllocationOwner::CanicCore,
            memory_keys: vec!["canic.core.sharding.registry.v1".to_string()],
            selected_by: BTreeSet::from([SelectionProvenance::EffectiveFeature(
                canic_core::role_contract::CanicFeatureKey::Sharding,
            )]),
        }],
    };

    let manifest = materialize_state_manifest(&[contract]).expect("manifest");
    let role = manifest.roles.first().expect("role");
    assert_eq!(role.canister_role, "shard");
    assert_eq!(role.state.len(), 1);
    assert_eq!(role.state[0].domain, "sharding_registry");
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
                memory_keys: definition
                    .memory_keys
                    .iter()
                    .map(|key| key.to_string())
                    .collect(),
                selected_by: BTreeSet::from([SelectionProvenance::BuiltInRole(
                    BuiltInRoleKind::WasmStore,
                )]),
            }
        })
        .collect();
    let contract = ResolvedRoleContract {
        role: canic_contracts::ids::CanisterRole::WASM_STORE,
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
        .filter_map(|domain| domain.memory_key.clone())
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 5);
    for expected in [
        "canic.control_plane.template.manifests.v1",
        "canic.control_plane.template.chunk_sets.v1",
        "canic.control_plane.template.chunk_refs.v1",
        "canic.control_plane.template.chunk_payloads.v1",
        "canic.control_plane.wasm_store.gc_state.v1",
    ] {
        assert!(ids.contains(&expected.to_string()));
    }
}

#[test]
fn fleet_coordinator_materializes_its_registry_and_funding_state() {
    let keys = [
        StateAllocationKey::FleetCoordinatorFunding,
        StateAllocationKey::FleetCoordinatorRegistry,
    ];
    let contract = ResolvedRoleContract {
        role: canic_contracts::ids::CanisterRole::FLEET_COORDINATOR,
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
                    memory_keys: definition
                        .memory_keys
                        .iter()
                        .map(|key| key.to_string())
                        .collect(),
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
            .filter_map(|domain| domain.memory_key.clone())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "canic.control_plane.fleet_coordinator.registry.v1".to_string(),
            "canic.control_plane.fleet_coordinator.funding.v1".to_string()
        ])
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
        role: canic_contracts::ids::CanisterRole::ROOT,
        built_in: None,
        capabilities: BTreeSet::new(),
        required_features: BTreeSet::new(),
        effective_features: BTreeSet::new(),
        allocations: vec![ResolvedStateAllocation {
            key,
            owner: definition.owner,
            memory_keys: definition
                .memory_keys
                .iter()
                .map(|key| key.to_string())
                .collect(),
            selected_by: BTreeSet::from([SelectionProvenance::Capability(
                canic_core::role_contract::RoleCapabilityKey::RootControlPlane,
            )]),
        }],
    };

    let manifest = materialize_state_manifest(&[contract]).expect("manifest");
    assert_eq!(manifest.roles.len(), 1);
    assert_eq!(manifest.roles[0].state.len(), 1);
    assert_eq!(manifest.roles[0].state[0].domain, "root_funding");
    assert_eq!(
        manifest.roles[0].state[0].memory_key,
        Some("canic.control_plane.root.funding.v1".to_string())
    );
}
