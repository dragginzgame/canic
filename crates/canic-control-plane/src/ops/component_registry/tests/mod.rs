//! Unit tests for root-local Component Registry state ownership.
//!
//! Production operations remain in the parent and focused responsibility modules.

use super::*;
use crate::{
    dto::template::WasmStoreStatusResponse,
    storage::stable::component_registry::RootComponentRegistryData,
    view::component_registry::{
        RootFleetSubnetDeletionPreparationAuthority,
        RootFleetSubnetStoreBindingFinalizationEvidence,
        RootFleetSubnetStoreCycleReclamationEvidence, RootFleetSubnetStoreDeletionAuthority,
        RootFleetSubnetStoreDeletionEvidence, RootFleetSubnetStoreReclamationEvidence,
    },
};
use canic_core::{
    cdk::types::Cycles,
    control_plane_support::{
        config::schema::ComponentChildKind,
        policy::{
            component_allocation::TopLevelComponentAllocationDecision,
            component_child_allocation::ComponentChildAllocationDecision,
        },
    },
    dto::{
        component_registry::{ComponentProvisioningOrigin, ComponentRuntimeActivationEvidence},
        fleet_registry::{
            FleetDirectoryProvenance, FleetDirectorySnapshot, FleetRegistryVersion,
            FleetSubnetRootDirectoryEntry, FleetSubnetRootDrainingReservationRequest,
            FleetSubnetRootDrainingReservationResponse, FleetSubnetRootEntry,
            FleetSubnetRootRemovalPublicationResponse, FleetSubnetRootStatus,
        },
        root_store::{RootStoreBootstrapRequest, RootStoreBootstrapResponse},
    },
    ids::{
        AppId, CanisterRole, CanonicalNetworkId, ComponentGroupDeploymentId,
        ComponentGroupMemberPath, ComponentGroupPlacementId, ComponentInstanceId,
        ComponentSpecAdmission, ComponentTopologyDigest, CyclesFundingBudget, FleetBinding,
        FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetRootLimits,
        ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    },
};

#[test]
fn deletion_retained_target_has_no_absolute_cycle_cap() {
    let target = deletion_retained_cycles_target(2_000_000_000_000, 86_400)
        .expect("derived deletion target");
    assert_eq!(
        target,
        2_000_000_000_000 + FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES
    );
    assert!(target > 1_500_000_000_000);
}

fn restart_component_registry() -> RootComponentRegistryData {
    let snapshot = RootComponentRegistryStore::export();
    RootComponentRegistryStore::import(snapshot.clone());
    assert_eq!(RootComponentRegistryStore::export(), snapshot);
    snapshot
}

fn root_draining_reservation(
    root: &FleetSubnetRootBinding,
    release_set: FleetSubnetRootReleaseSet,
    registry: &FleetRegistryVersion,
    operation_id: [u8; 32],
    prepared_at_ns: u64,
) -> FleetSubnetRootDrainingReservationResponse {
    let mut response = FleetSubnetRootDrainingReservationResponse {
        request: FleetSubnetRootDrainingReservationRequest {
            operation_id,
            expected_registry: registry.clone(),
            expected_root: FleetSubnetRootEntry {
                placement_subnet: root.placement_subnet,
                fleet_subnet_root: root.fleet_subnet_root,
                component_admissions: root.component_admissions.clone(),
                component_topology_digest: root.component_topology_digest,
                active_release_set: release_set,
                limits: root.limits.clone(),
                funding: root.funding.clone(),
                status: FleetSubnetRootStatus::Active,
            },
        },
        coordinator: root.authority.binding.coordinator,
        prepared_at_ns,
        reservation_hash: [0; 32],
    };
    response.reservation_hash = FleetSubnetRootDrainingReservationOps::content_hash(&response)
        .expect("hash root-draining reservation");
    response
}

fn exact_registry_entry_bytes(data: &RootComponentRegistryData) -> u64 {
    data.allocations
        .iter()
        .map(RootComponentRegistryStore::allocation_entry_bytes)
        .chain(
            data.partitions
                .iter()
                .map(RootComponentRegistryStore::partition_entry_bytes),
        )
        .chain(data.partitions.iter().map(|partition| {
            RootComponentRegistryStore::principal_index_entry_bytes(
                partition.binding.canister_id,
                partition.binding.component,
            )
        }))
        .chain(
            data.children
                .iter()
                .map(RootComponentRegistryStore::child_entry_bytes),
        )
        .chain(data.children.iter().map(|child| {
            RootComponentRegistryStore::principal_index_entry_bytes(
                child.canister_id,
                child.component,
            )
        }))
        .chain(
            data.child_traversals
                .iter()
                .map(RootComponentRegistryStore::child_traversal_entry_bytes),
        )
        .chain(
            data.child_allocations
                .iter()
                .map(RootComponentRegistryStore::child_allocation_entry_bytes),
        )
        .chain(
            data.subtree_removals
                .iter()
                .map(RootComponentRegistryStore::subtree_removal_entry_bytes),
        )
        .chain(
            data.subtree_removal_history
                .iter()
                .map(RootComponentRegistryStore::subtree_removal_completed_leaf_entry_bytes),
        )
        .chain(
            data.component_drainings
                .iter()
                .map(charged_component_draining_entry_bytes),
        )
        .chain(
            data.parent_role_counts
                .iter()
                .map(RootComponentRegistryStore::parent_role_count_entry_bytes),
        )
        .sum()
}

fn exact_component_registry_entry_bytes(
    data: &RootComponentRegistryData,
    component: ComponentInstanceId,
) -> u64 {
    data.allocations
        .iter()
        .filter(|allocation| allocation.component == component)
        .map(RootComponentRegistryStore::allocation_entry_bytes)
        .chain(
            data.partitions
                .iter()
                .filter(|partition| partition.binding.component == component)
                .map(RootComponentRegistryStore::partition_entry_bytes),
        )
        .chain(
            data.partitions
                .iter()
                .filter(|partition| partition.binding.component == component)
                .map(|partition| {
                    RootComponentRegistryStore::principal_index_entry_bytes(
                        partition.binding.canister_id,
                        component,
                    )
                }),
        )
        .chain(
            data.children
                .iter()
                .filter(|child| child.component == component)
                .map(RootComponentRegistryStore::child_entry_bytes),
        )
        .chain(
            data.children
                .iter()
                .filter(|child| child.component == component)
                .map(|child| {
                    RootComponentRegistryStore::principal_index_entry_bytes(
                        child.canister_id,
                        component,
                    )
                }),
        )
        .chain(
            data.child_traversals
                .iter()
                .filter(|traversal| traversal.component == component)
                .map(RootComponentRegistryStore::child_traversal_entry_bytes),
        )
        .chain(
            data.child_allocations
                .iter()
                .filter(|allocation| allocation.component == component)
                .map(charged_child_allocation_entry_bytes),
        )
        .chain(
            data.subtree_removals
                .iter()
                .filter(|removal| removal.component == component)
                .map(RootComponentRegistryStore::subtree_removal_entry_bytes),
        )
        .chain(
            data.subtree_removal_history
                .iter()
                .filter(|history| history.component == component)
                .map(RootComponentRegistryStore::subtree_removal_completed_leaf_entry_bytes),
        )
        .chain(
            data.component_drainings
                .iter()
                .filter(|draining| draining.component == component)
                .map(charged_component_draining_entry_bytes),
        )
        .chain(
            data.parent_role_counts
                .iter()
                .filter(|count| count.component == component)
                .map(RootComponentRegistryStore::parent_role_count_entry_bytes),
        )
        .sum()
}

fn charged_child_allocation_entry_bytes(record: &RootComponentChildAllocationRecord) -> u64 {
    match &record.progress {
        RootComponentChildAllocationProgressRecord::Reserved => {
            RootComponentRegistryStore::child_allocation_entry_bytes(record)
        }
        RootComponentChildAllocationProgressRecord::CreationIntent(creation)
        | RootComponentChildAllocationProgressRecord::Created {
            effect: creation, ..
        } => creation.charged_entry_bytes,
        RootComponentChildAllocationProgressRecord::InstallIntent { installation, .. }
        | RootComponentChildAllocationProgressRecord::Installed { installation, .. }
        | RootComponentChildAllocationProgressRecord::Verified { installation, .. }
        | RootComponentChildAllocationProgressRecord::Committed { installation, .. } => {
            installation.charged_entry_bytes
        }
    }
}

#[test]
fn preparation_is_exact_idempotent_and_conflict_closed() {
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
    let root = root_binding();
    let version = FleetRegistryVersion {
        authority: root.authority.clone(),
        revision: 4,
        content_hash: [5; 32],
    };
    let release_set = FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([8; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
    };
    let store_bootstrap = RootStoreBootstrapRequest {
        operation_id: [8; 32],
        manifest_payload_size_bytes: 128,
    };

    let prepared = ComponentRegistryOps::prepare(
        root.clone(),
        version.clone(),
        release_set,
        store_bootstrap.clone(),
    )
    .expect("prepare");
    let repeated =
        ComponentRegistryOps::prepare(root.clone(), version, release_set, store_bootstrap)
            .expect("exact retry");

    assert_eq!(prepared, repeated);
    assert_eq!(prepared.next_allocation_sequence, 1);
    assert_eq!(prepared.reserved_component_instances, 0);
    assert_eq!(prepared.committed_component_instances, 0);
    assert_eq!(prepared.managed_descendants, 0);
    assert_eq!(prepared.known_created_component_canisters, 0);
    assert_eq!(prepared.encoded_bytes, 0);

    let mut conflicting = root;
    conflicting.limits.maximum_component_instances += 1;
    assert!(
        ComponentRegistryOps::prepare(
            conflicting,
            repeated.prepared_against_registry,
            release_set,
            repeated.store_bootstrap,
        )
        .is_err()
    );
    let empty_inventory = ComponentRegistryOps::seal_initial_inventory([10; 32], 11)
        .expect("seal empty initial inventory");
    assert_eq!(empty_inventory.receipt.component_count, 0);
    assert_ne!(empty_inventory.receipt.inventory_hash, [0; 32]);
    assert!(empty_inventory.operation_ids.is_empty());
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

#[test]
fn allocation_reservation_is_exact_idempotent_and_charges_registry_capacity() {
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
    let root = root_binding();
    let release_set = FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([8; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
    };
    let version = FleetRegistryVersion {
        authority: root.authority.clone(),
        revision: 4,
        content_hash: [5; 32],
    };
    ComponentRegistryOps::prepare(
        root,
        version,
        release_set,
        RootStoreBootstrapRequest {
            operation_id: [8; 32],
            manifest_payload_size_bytes: 128,
        },
    )
    .expect("prepare");
    let decision = TopLevelComponentAllocationDecision {
        allocation_sequence: 1,
        component: ComponentInstanceId::from_generated_bytes([10; 32]),
        component_spec: "projects".parse().expect("Component Spec"),
        spec_hash: [6; 32],
        role: CanisterRole::new("project_hub"),
    };
    let origin = ComponentProvisioningOrigin::FleetAdministrator {
        caller: candid::Principal::from_slice(&[11; 29]),
    };

    let reserved =
        ComponentRegistryOps::reserve_allocation(decision.clone(), [12; 32], origin.clone(), false)
            .expect("reserve");
    let interrupted_snapshot = RootComponentRegistryStore::export();
    RootComponentRegistryStore::import(interrupted_snapshot);
    let repeated = ComponentRegistryOps::reserve_allocation(decision, [12; 32], origin, false)
        .expect("exact retry");

    assert_eq!(reserved, repeated);
    assert_eq!(reserved.allocation_sequence, 1);
    let status = ComponentRegistryOps::current().expect("Registry status");
    assert_eq!(status.next_allocation_sequence, 2);
    assert_eq!(status.reserved_component_instances, 1);
    assert_eq!(status.committed_component_instances, 0);
    assert_eq!(status.known_created_component_canisters, 0);
    assert!(status.encoded_bytes > 0);
    assert_eq!(
        ComponentRegistryOps::component_spec_counts(&reserved.component_spec).expect("Spec counts"),
        ComponentSpecInstanceCounts {
            reserved: 1,
            committed: 0,
        }
    );
    assert!(
        ComponentRegistryOps::seal_initial_inventory([20; 32], 21).is_err(),
        "a nonterminal allocation must prevent initial inventory sealing"
    );
    assert!(
        ComponentRegistryOps::current()
            .expect("Registry status")
            .initial_inventory
            .is_none()
    );

    let conflicting = TopLevelComponentAllocationDecision {
        allocation_sequence: 2,
        component: ComponentInstanceId::from_generated_bytes([13; 32]),
        component_spec: "projects".parse().expect("Component Spec"),
        spec_hash: [6; 32],
        role: CanisterRole::new("project_hub"),
    };
    assert!(
        ComponentRegistryOps::reserve_allocation(
            conflicting,
            [12; 32],
            ComponentProvisioningOrigin::FleetAdministrator {
                caller: candid::Principal::from_slice(&[11; 29]),
            },
            false,
        )
        .is_err()
    );
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

#[test]
fn peer_allocation_counts_are_scoped_to_exact_requester_and_target() {
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
    let root = root_binding();
    let release_set = FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([8; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
    };
    ComponentRegistryOps::prepare(
        root.clone(),
        FleetRegistryVersion {
            authority: root.authority.clone(),
            revision: 4,
            content_hash: [5; 32],
        },
        release_set,
        RootStoreBootstrapRequest {
            operation_id: [8; 32],
            manifest_payload_size_bytes: 128,
        },
    )
    .expect("prepare");
    let requester_spec: ComponentSpecId = "projects".parse().expect("requester Component Spec");
    let target_spec: ComponentSpecId = "users".parse().expect("target Component Spec");
    let requester = ComponentBinding {
        authority: root.authority,
        component: ComponentInstanceId::from_generated_bytes([20; 32]),
        component_spec: requester_spec.clone(),
        spec_hash: [6; 32],
        role: CanisterRole::new("project_hub"),
        placement_subnet: root.placement_subnet,
        fleet_subnet_root: root.fleet_subnet_root,
        canister_id: candid::Principal::from_slice(&[21; 29]),
    };
    let origin = ComponentProvisioningOrigin::Component {
        requester: Box::new(requester.clone()),
        grant: Box::new(
            canic_core::control_plane_support::config::ComponentProvisioningGrant {
                requester_component_spec: requester_spec,
                target_component_spec: target_spec.clone(),
                maximum_instances_per_requester_per_root: 2,
            },
        ),
    };
    for (sequence, operation_seed) in [(1, 22), (2, 23)] {
        ComponentRegistryOps::reserve_allocation(
            TopLevelComponentAllocationDecision {
                allocation_sequence: sequence,
                component: ComponentInstanceId::from_generated_bytes([operation_seed; 32]),
                component_spec: target_spec.clone(),
                spec_hash: [24; 32],
                role: CanisterRole::new("user_hub"),
            },
            [operation_seed; 32],
            origin.clone(),
            false,
        )
        .expect("reserve peer allocation");
    }

    assert_eq!(
        ComponentRegistryOps::peer_component_counts(&requester, &target_spec).expect("peer counts"),
        PeerComponentInstanceCounts {
            reserved: 2,
            committed: 0,
        }
    );
    let other_requester = ComponentBinding {
        canister_id: candid::Principal::from_slice(&[25; 29]),
        ..requester
    };
    assert_eq!(
        ComponentRegistryOps::peer_component_counts(&other_requester, &target_spec)
            .expect("other requester counts"),
        PeerComponentInstanceCounts::default()
    );
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

#[test]
fn root_draining_is_durable_and_fences_only_new_top_level_allocations() {
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
    let root = root_binding();
    let version = FleetRegistryVersion {
        authority: root.authority.clone(),
        revision: 4,
        content_hash: [5; 32],
    };
    let release_set = FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([8; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
    };
    ComponentRegistryOps::prepare(
        root.clone(),
        version.clone(),
        release_set,
        RootStoreBootstrapRequest {
            operation_id: [8; 32],
            manifest_payload_size_bytes: 128,
        },
    )
    .expect("prepare");
    let conflicting_version = FleetRegistryVersion {
        authority: version.authority.clone(),
        revision: version.revision,
        content_hash: [15; 32],
    };
    let conflicting_reservation =
        root_draining_reservation(&root, release_set, &conflicting_version, [13; 32], 12);
    ComponentRegistryOps::begin_root_draining(
        [13; 32],
        &conflicting_version,
        &conflicting_reservation,
        14,
    )
    .expect_err("equal Registry revision with a different hash must fail closed");
    let existing_decision = TopLevelComponentAllocationDecision {
        allocation_sequence: 1,
        component: ComponentInstanceId::from_generated_bytes([10; 32]),
        component_spec: "projects".parse().expect("Component Spec"),
        spec_hash: [6; 32],
        role: CanisterRole::new("project_hub"),
    };
    let origin = ComponentProvisioningOrigin::FleetAdministrator {
        caller: candid::Principal::from_slice(&[11; 29]),
    };
    let existing = ComponentRegistryOps::reserve_allocation(
        existing_decision.clone(),
        [12; 32],
        origin.clone(),
        false,
    )
    .expect("reserve before draining");

    let current_version = FleetRegistryVersion {
        authority: version.authority.clone(),
        revision: version.revision + 2,
        content_hash: [15; 32],
    };
    let reservation = root_draining_reservation(&root, release_set, &current_version, [13; 32], 12);
    let draining =
        ComponentRegistryOps::begin_root_draining([13; 32], &current_version, &reservation, 14)
            .expect("begin after later mirror synchronization");
    assert_eq!(draining.active_registry, current_version);
    assert_eq!(draining.next_allocation_sequence, 2);
    assert_eq!(draining.reserved_component_instances, 1);
    assert_eq!(draining.committed_component_instances, 0);
    assert_eq!(draining.managed_descendants, 0);
    assert_eq!(draining.known_created_component_canisters, 0);
    assert_eq!(draining.active_release_set, release_set);
    assert_eq!(draining.final_inventory, None);
    assert_root_final_inventory_is_fenced(&current_version);
    assert_root_draining_is_durable(
        &root,
        release_set,
        &current_version,
        &reservation,
        &draining,
    );
    assert_root_draining_allocation_fence(existing_decision, origin, existing);
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

fn assert_root_draining_is_durable(
    root: &FleetSubnetRootBinding,
    release_set: FleetSubnetRootReleaseSet,
    current_version: &FleetRegistryVersion,
    reservation: &FleetSubnetRootDrainingReservationResponse,
    draining: &RootFleetSubnetDrainingView,
) {
    restart_component_registry();
    assert_eq!(
        &ComponentRegistryOps::root_draining([13; 32]).expect("status after restart"),
        draining
    );
    assert_eq!(
        &ComponentRegistryOps::begin_root_draining([13; 32], current_version, reservation, 99,)
            .expect("exact retry"),
        draining
    );
    let durable = RootComponentRegistryStore::export();
    let mut corrupted = durable.clone();
    corrupted
        .current
        .as_mut()
        .expect("root Component Registry")
        .root_draining
        .as_mut()
        .expect("root draining record")
        .reservation
        .reservation_hash[0] ^= 1;
    RootComponentRegistryStore::import(corrupted);
    let invalid = ComponentRegistryOps::root_draining([13; 32])
        .expect_err("corrupt retained reservation must fail closed");
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID
    );
    RootComponentRegistryStore::import(durable);
    ComponentRegistryOps::root_draining([15; 32])
        .expect_err("status must bind the exact operation");
    let conflict = root_draining_reservation(root, release_set, current_version, [15; 32], 14);
    ComponentRegistryOps::begin_root_draining([15; 32], current_version, &conflict, 16)
        .expect_err("different draining intent must conflict");
}

fn assert_root_draining_allocation_fence(
    existing_decision: TopLevelComponentAllocationDecision,
    origin: ComponentProvisioningOrigin,
    existing: RootComponentAllocationView,
) {
    let repeated = ComponentRegistryOps::reserve_allocation(
        existing_decision,
        [12; 32],
        origin.clone(),
        false,
    )
    .expect("an existing reservation remains response-idempotent");
    assert_eq!(repeated, existing);

    let before_rejection = RootComponentRegistryStore::export();
    let new_decision = TopLevelComponentAllocationDecision {
        allocation_sequence: 2,
        component: ComponentInstanceId::from_generated_bytes([16; 32]),
        component_spec: "projects".parse().expect("Component Spec"),
        spec_hash: [6; 32],
        role: CanisterRole::new("project_hub"),
    };
    let error = ComponentRegistryOps::reserve_allocation(new_decision, [17; 32], origin, false)
        .expect_err("draining root must reject a new top-level allocation");
    assert_eq!(
        error.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    assert_eq!(RootComponentRegistryStore::export(), before_rejection);
}

fn assert_root_final_inventory_is_fenced(current_version: &FleetRegistryVersion) {
    ComponentRegistryOps::prepare_root_final_inventory([13; 32], current_version)
        .expect_err("reserved Component history must prevent root final inventory");
    ComponentRegistryOps::require_root_store_admin_open()
        .expect_err("root draining must fence Store administration");
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one end-to-end test owns durable finalization and exact replay"
)]
fn empty_root_final_inventory_is_exact_durable_and_response_idempotent() {
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
    let root = root_binding();
    let prepared_registry = FleetRegistryVersion {
        authority: root.authority.clone(),
        revision: 4,
        content_hash: [5; 32],
    };
    let release_set = FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([8; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
    };
    ComponentRegistryOps::prepare(
        root.clone(),
        prepared_registry.clone(),
        release_set,
        RootStoreBootstrapRequest {
            operation_id: [8; 32],
            manifest_payload_size_bytes: 128,
        },
    )
    .expect("prepare");
    ComponentRegistryOps::require_root_store_admin_open().expect("active root Store admin");
    let reservation =
        root_draining_reservation(&root, release_set, &prepared_registry, [10; 32], 9);
    ComponentRegistryOps::begin_root_draining([10; 32], &prepared_registry, &reservation, 11)
        .expect("begin root draining");
    let published_registry = FleetRegistryVersion {
        authority: prepared_registry.authority,
        revision: prepared_registry.revision + 1,
        content_hash: [12; 32],
    };
    assert!(
        ComponentRegistryOps::root_funding_eligible(FleetSubnetRootStatus::Draining)
            .expect("pre-fence draining funding eligibility")
    );
    ComponentRegistryOps::prepare_root_final_inventory([10; 32], &published_registry)
        .expect_err("terminal inventory must wait for the durable funding fence");
    let fenced = ComponentRegistryOps::record_root_funding_fence([10; 32], 12)
        .expect("record root funding fence");
    assert_eq!(fenced.funding_fenced_at_ns, Some(12));
    assert!(
        !ComponentRegistryOps::root_funding_eligible(FleetSubnetRootStatus::Draining)
            .expect("post-fence draining funding eligibility")
    );
    let replayed_fence = ComponentRegistryOps::record_root_funding_fence([10; 32], 13)
        .expect("replay root funding fence");
    assert_eq!(replayed_fence.funding_fenced_at_ns, Some(12));
    let plan = ComponentRegistryOps::prepare_root_final_inventory([10; 32], &published_registry)
        .expect("empty root is terminal");
    assert_eq!(plan.removed_component_instances, 0);
    assert_eq!(plan.root_registry_encoded_bytes, 0);
    assert_ne!(plan.terminal_component_history_hash, [0; 32]);
    assert_durable_root_final_inventory_intent(&plan, &published_registry);

    let store_pid = candid::Principal::from_slice(&[13; 29]);
    let store = RootStoreBootstrapResponse {
        fleet_subnet_root: root.fleet_subnet_root,
        wasm_store: store_pid,
        release_set,
        catalog: vec![canic_core::dto::root_store::RootStoreCatalogEntry {
            role: CanisterRole::new("example_component"),
            raw_module_hash: [14; 32],
            candid_sha256: [15; 32],
            protocol_profile_digest: ProtocolProfileDigest::from_bytes([16; 32]),
            payload_hash: [17; 32],
            payload_size_bytes: 16_000,
        }],
    };
    let status = prepared_store_status();
    let mut writable = status.clone();
    writable.gc.mode = WasmStoreGcMode::Normal;
    writable.gc.changed_at = 0;
    writable.gc.prepared_at = None;
    ComponentRegistryOps::finalize_root_inventory(
        [10; 32],
        &published_registry,
        &store,
        &writable,
        19,
    )
    .expect_err("writable Store must not authorize root final inventory");
    let inventory = ComponentRegistryOps::finalize_root_inventory(
        [10; 32],
        &published_registry,
        &store,
        &status,
        20,
    )
    .expect("finalize root inventory");
    assert_eq!(inventory.registry, published_registry);
    assert_eq!(inventory.removed_component_instances, 0);
    assert_eq!(inventory.wasm_store, store_pid);
    assert_eq!(inventory.wasm_store_catalog_entries, 1);
    assert_eq!(inventory.wasm_store_gc_prepared_at_secs, 18);
    assert_ne!(inventory.wasm_store_catalog_hash, [0; 32]);
    assert_ne!(inventory.inventory_hash, [0; 32]);

    restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::root_final_inventory([10; 32])
            .expect("final inventory after restart"),
        inventory
    );
    assert_eq!(
        ComponentRegistryOps::finalize_root_inventory(
            [10; 32],
            &published_registry,
            &store,
            &status,
            99,
        )
        .expect("exact response-idempotent retry"),
        inventory
    );

    assert_root_removal_publication_is_exact(&store, status, &inventory, &published_registry);
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

fn assert_durable_root_final_inventory_intent(
    plan: &RootFleetSubnetFinalInventoryPlan,
    published_registry: &FleetRegistryVersion,
) {
    assert_eq!(
        ComponentRegistryOps::begin_root_final_inventory([10; 32], published_registry, 12,)
            .expect("prepare durable root final inventory intent"),
        plan.clone()
    );
    restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::root_final_inventory_intent_registry([10; 32])
            .expect("final inventory intent after restart"),
        Some(published_registry.clone())
    );
}

fn prepared_store_status() -> WasmStoreStatusResponse {
    WasmStoreStatusResponse {
        gc: crate::dto::template::WasmStoreGcStatusResponse {
            mode: WasmStoreGcMode::Prepared,
            changed_at: 18,
            prepared_at: Some(18),
            started_at: None,
            completed_at: None,
            runs_completed: 0,
        },
        occupied_store_bytes: 16_000,
        occupied_store_size: "15.6 KiB".to_string(),
        max_store_bytes: 32_000,
        max_store_size: "31.2 KiB".to_string(),
        remaining_store_bytes: 16_000,
        remaining_store_size: "15.6 KiB".to_string(),
        headroom_bytes: None,
        headroom_size: None,
        within_headroom: true,
        template_count: 2,
        max_templates: Some(10),
        release_count: 2,
        max_template_versions_per_template: Some(2),
        templates: vec![
            crate::dto::template::WasmStoreTemplateStatusResponse {
                template_id: crate::ids::TemplateId::new("component:example_component"),
                versions: 1,
            },
            crate::dto::template::WasmStoreTemplateStatusResponse {
                template_id: crate::ids::TemplateId::new("root-release-set:digest"),
                versions: 1,
            },
        ],
    }
}

fn assert_root_removal_publication_is_exact(
    store: &RootStoreBootstrapResponse,
    status: WasmStoreStatusResponse,
    inventory: &RootFleetSubnetFinalInventoryView,
    published_registry: &FleetRegistryVersion,
) {
    ComponentRegistryOps::begin_root_store_reclamation([10; 32], inventory.inventory_hash, 21)
        .expect_err("Store reclamation must require logical root removal");
    assert_eq!(
        ComponentRegistryOps::verify_root_final_inventory_store([10; 32], store, &status)
            .expect("revalidate retained Store"),
        inventory.clone()
    );
    let mut changed_status = status.clone();
    changed_status.occupied_store_bytes += 1;
    ComponentRegistryOps::verify_root_final_inventory_store([10; 32], store, &changed_status)
        .expect_err("changed Store inventory must fail closed");

    let response = FleetSubnetRootRemovalPublicationResponse {
        final_inventory: final_inventory_dto(inventory),
        previous_version: published_registry.clone(),
        version: FleetRegistryVersion {
            authority: published_registry.authority.clone(),
            revision: published_registry.revision + 1,
            content_hash: [21; 32],
        },
    };
    let publication =
        ComponentRegistryOps::record_root_removal_publication([10; 32], &response, 22)
            .expect("record root removal publication");
    assert_eq!(publication.final_inventory_hash, inventory.inventory_hash);
    restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::record_root_removal_publication([10; 32], &response, 99)
            .expect("exact root removal publication retry"),
        publication
    );
    let mut conflicting = response;
    conflicting.version.content_hash[0] ^= 1;
    ComponentRegistryOps::record_root_removal_publication([10; 32], &conflicting, 23)
        .expect_err("conflicting removal publication must fail closed");

    ComponentRegistryOps::begin_root_store_reclamation([10; 32], [24; 32], 24)
        .expect_err("Store reclamation must bind the exact final inventory");
    let intent =
        ComponentRegistryOps::begin_root_store_reclamation([10; 32], inventory.inventory_hash, 24)
            .expect("prepare Store reclamation intent");
    assert_eq!(intent.wasm_store, inventory.wasm_store);
    assert_eq!(intent.final_inventory_hash, inventory.inventory_hash);
    restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::begin_root_store_reclamation([10; 32], inventory.inventory_hash, 99,)
            .expect("exact Store reclamation intent retry"),
        intent
    );

    let evidence = RootFleetSubnetStoreReclamationEvidence {
        wasm_store: inventory.wasm_store,
        occupied_store_bytes: 0,
        catalog_entries: 0,
        template_count: 0,
        release_count: 0,
        gc_prepared_at_secs: status.gc.prepared_at.expect("prepared time"),
        gc_started_at_secs: 25,
        gc_completed_at_secs: 26,
        gc_runs_completed: 1,
    };
    let reclamation = ComponentRegistryOps::record_root_store_reclamation([10; 32], evidence, 27)
        .expect("record terminal Store reclamation");
    assert_eq!(
        reclamation.reclaimed_store_bytes,
        inventory.wasm_store_occupied_bytes
    );
    assert_eq!(
        reclamation.reclaimed_catalog_entries,
        inventory.wasm_store_catalog_entries
    );
    assert_eq!(reclamation.gc_runs_completed, 1);
    assert_ne!(reclamation.reclamation_hash, [0; 32]);
    restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::record_root_store_reclamation([10; 32], evidence, 999)
            .expect("exact terminal Store reclamation retry"),
        reclamation
    );
    assert_eq!(
        ComponentRegistryOps::root_store_reclamation_if_present([10; 32])
            .expect("Store reclamation status after restart"),
        Some(reclamation)
    );

    assert_root_store_binding_finalization_is_exact(inventory, reclamation);
}

fn assert_root_store_binding_finalization_is_exact(
    inventory: &RootFleetSubnetFinalInventoryView,
    reclamation: RootFleetSubnetStoreReclamationView,
) {
    let binding = WasmStoreBinding::owned(inventory.wasm_store.to_text());
    ComponentRegistryOps::begin_root_store_binding_finalization(
        [10; 32],
        [28; 32],
        binding.clone(),
        4,
        28,
    )
    .expect_err("Store binding finalization must bind the reclamation receipt");
    let finalization_intent = ComponentRegistryOps::begin_root_store_binding_finalization(
        [10; 32],
        reclamation.reclamation_hash,
        binding.clone(),
        4,
        28,
    )
    .expect("prepare Store binding finalization intent");
    restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::begin_root_store_binding_finalization(
            [10; 32],
            reclamation.reclamation_hash,
            binding.clone(),
            4,
            999,
        )
        .expect("exact Store binding finalization intent retry"),
        finalization_intent
    );
    let finalization_evidence = RootFleetSubnetStoreBindingFinalizationEvidence {
        wasm_store: inventory.wasm_store,
        binding,
        source_generation: 4,
        finalized_generation: 7,
        finalized_at_secs: 29,
    };
    let finalization = ComponentRegistryOps::record_root_store_binding_finalization(
        [10; 32],
        finalization_evidence.clone(),
        30,
    )
    .expect("record terminal Store binding finalization");
    assert_eq!(finalization.reclamation_hash, reclamation.reclamation_hash);
    assert_eq!(finalization.source_generation, 4);
    assert_eq!(finalization.finalized_generation, 7);
    assert_ne!(finalization.finalization_hash, [0; 32]);
    restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::record_root_store_binding_finalization(
            [10; 32],
            finalization_evidence,
            999,
        )
        .expect("exact terminal Store binding finalization retry"),
        finalization
    );
    assert_root_store_deletion_is_exact(inventory, finalization);
}

fn assert_root_store_deletion_is_exact(
    inventory: &RootFleetSubnetFinalInventoryView,
    finalization: RootFleetSubnetStoreBindingFinalizationView,
) {
    let binding = WasmStoreBinding::owned(inventory.wasm_store.to_text());
    let deletion_authority = || RootFleetSubnetStoreDeletionAuthority {
        wasm_store: inventory.wasm_store,
        binding: binding.clone(),
        observed_module_hash: [32; 32],
        observed_controllers: vec![inventory.fleet_subnet_root],
        observed_cycles_before_reclamation: 500,
        retained_cycles_target: 100,
    };
    ComponentRegistryOps::begin_root_store_deletion([10; 32], [31; 32], deletion_authority(), 31)
        .expect_err("Store deletion must bind the finalization receipt");
    let intent = ComponentRegistryOps::begin_root_store_deletion(
        [10; 32],
        finalization.finalization_hash,
        deletion_authority(),
        31,
    )
    .expect("prepare Store deletion intent");
    assert_eq!(intent.wasm_store, inventory.wasm_store);
    restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::begin_root_store_deletion(
            [10; 32],
            finalization.finalization_hash,
            deletion_authority(),
            999,
        )
        .expect("exact Store deletion intent retry"),
        intent
    );

    let intent = ComponentRegistryOps::record_root_store_cycle_reclamation(
        [10; 32],
        RootFleetSubnetStoreCycleReclamationEvidence {
            observed_cycles_after_reclamation: 90,
            cycles_reclaimed_at_ns: 32,
        },
    )
    .expect("record Store cycle reclamation");
    assert_eq!(intent.observed_cycles_after_reclamation, Some(90));
    restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::record_root_store_cycle_reclamation(
            [10; 32],
            RootFleetSubnetStoreCycleReclamationEvidence {
                observed_cycles_after_reclamation: 90,
                cycles_reclaimed_at_ns: 32,
            },
        )
        .expect("exact Store cycle-reclamation retry"),
        intent
    );

    let evidence = RootFleetSubnetStoreDeletionEvidence {
        wasm_store: inventory.wasm_store,
        binding,
        observed_module_hash: [32; 32],
        observed_controllers: vec![inventory.fleet_subnet_root],
        observed_cycles_before_reclamation: 500,
        retained_cycles_target: 100,
        observed_cycles_after_reclamation: 90,
        cycles_reclaimed_at_ns: 32,
        observed_absent_at_ns: 33,
    };
    let deletion = ComponentRegistryOps::record_root_store_deletion([10; 32], evidence.clone(), 34)
        .expect("record terminal Store deletion");
    assert_eq!(
        deletion.binding_finalization_hash,
        finalization.finalization_hash
    );
    assert_eq!(deletion.observed_module_hash, [32; 32]);
    assert_ne!(deletion.deletion_hash, [0; 32]);
    restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::record_root_store_deletion([10; 32], evidence, 999)
            .expect("exact terminal Store deletion retry"),
        deletion
    );
    assert_root_deletion_preparation_is_exact(inventory, &deletion);
    assert_eq!(
        ComponentRegistryOps::root_store_deletion_if_present([10; 32])
            .expect("Store deletion status after restart"),
        Some(deletion)
    );
}

fn assert_root_deletion_preparation_is_exact(
    inventory: &RootFleetSubnetFinalInventoryView,
    store_deletion: &RootFleetSubnetStoreDeletionView,
) {
    let coordinator = inventory.registry.authority.binding.coordinator;
    let retained_cycles_target =
        deletion_retained_cycles_target(86_400, 1).expect("bounded root deletion maximum cycles");
    let intent = ComponentRegistryOps::begin_root_deletion_preparation(
        [10; 32],
        RootFleetSubnetDeletionPreparationAuthority {
            store_deletion_hash: store_deletion.deletion_hash,
            coordinator,
            observed_cycles_before_reclamation: 500_000_000_000,
            retained_cycles_target,
            observed_reserved_cycles: 0,
            observed_idle_cycles_burned_per_day: 86_400,
            observed_freezing_threshold_seconds: 1,
        },
        35,
    )
    .expect("prepare root deletion intent");
    assert_eq!(intent.store_deletion_hash, store_deletion.deletion_hash);
    restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::begin_root_deletion_preparation(
            [10; 32],
            RootFleetSubnetDeletionPreparationAuthority {
                store_deletion_hash: store_deletion.deletion_hash,
                coordinator,
                observed_cycles_before_reclamation: 500_000_000_000,
                retained_cycles_target,
                observed_reserved_cycles: 0,
                observed_idle_cycles_burned_per_day: 86_400,
                observed_freezing_threshold_seconds: 1,
            },
            999,
        )
        .expect("exact root deletion intent retry"),
        intent
    );

    let intent = ComponentRegistryOps::record_root_deletion_cycle_reclamation(
        [10; 32],
        [36; 32],
        90_000_000_000,
        36,
    )
    .expect("record root deletion cycle reclamation");
    assert_eq!(intent.coordinator_intent_hash, Some([36; 32]));
    restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::record_root_deletion_cycle_reclamation(
            [10; 32],
            [36; 32],
            90_000_000_000,
            36,
        )
        .expect("exact root cycle-reclamation retry"),
        intent
    );

    let preparation =
        ComponentRegistryOps::record_root_deletion_preparation([10; 32], [37; 32], 38)
            .expect("record root deletion readiness");
    assert_eq!(preparation.coordinator_readiness_hash, [37; 32]);
    restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::record_root_deletion_preparation([10; 32], [37; 32], 999)
            .expect("exact root deletion readiness retry"),
        preparation
    );
    assert_eq!(
        ComponentRegistryOps::root_deletion_preparation_if_present([10; 32])
            .expect("root deletion readiness status"),
        Some(preparation)
    );
}

fn final_inventory_dto(
    inventory: &RootFleetSubnetFinalInventoryView,
) -> canic_core::dto::fleet_subnet_root::FleetSubnetRootFinalInventoryResponse {
    canic_core::dto::fleet_subnet_root::FleetSubnetRootFinalInventoryResponse {
        operation_id: inventory.operation_id,
        fleet_subnet_root: inventory.fleet_subnet_root,
        placement_subnet: inventory.placement_subnet,
        registry: inventory.registry.clone(),
        component_topology_digest: inventory.component_topology_digest,
        active_release_set: inventory.active_release_set,
        next_allocation_sequence: inventory.next_allocation_sequence,
        removed_component_instances: inventory.removed_component_instances,
        terminal_component_history_hash: inventory.terminal_component_history_hash,
        root_registry_encoded_bytes: inventory.root_registry_encoded_bytes,
        wasm_store: inventory.wasm_store,
        wasm_store_catalog_hash: inventory.wasm_store_catalog_hash,
        wasm_store_catalog_entries: inventory.wasm_store_catalog_entries,
        wasm_store_occupied_bytes: inventory.wasm_store_occupied_bytes,
        wasm_store_template_count: inventory.wasm_store_template_count,
        wasm_store_release_count: inventory.wasm_store_release_count,
        wasm_store_gc_prepared_at_secs: inventory.wasm_store_gc_prepared_at_secs,
        finalized_at_ns: inventory.finalized_at_ns,
        inventory_hash: inventory.inventory_hash,
    }
}

#[test]
fn component_final_inventory_is_exact_durable_and_response_idempotent() {
    let (partition, draining, fleet) = import_empty_quiescent_component();
    let RootComponentDrainingAdvanceView::DescendantsEmpty {
        registry,
        descendant_content_hash,
    } = ComponentRegistryOps::advance_component_draining(
        partition.binding.component,
        draining.operation_id,
    )
    .expect("observe exact empty draining inventory")
    else {
        panic!("empty Component must have no draining subtree");
    };
    assert_eq!(registry, draining.registry);
    assert_eq!(
        descendant_content_hash,
        empty_component_descendant_content_hash(partition.binding.component)
    );

    let before_finalization = RootComponentRegistryStore::export();
    let mut conflicting_registry = registry.clone();
    conflicting_registry.revision += 1;
    ComponentRegistryOps::finalize_component_inventory(
        partition.binding.component,
        draining.operation_id,
        conflicting_registry.clone(),
        fleet.clone(),
        112,
    )
    .expect_err("final inventory must bind the exact current Registry head");
    ComponentRegistryOps::finalize_component_inventory(
        partition.binding.component,
        draining.operation_id,
        registry.clone(),
        fleet.clone(),
        110,
    )
    .expect_err("final inventory time cannot precede terminal quiescence");
    assert_eq!(RootComponentRegistryStore::export(), before_finalization);

    let inventory = ComponentRegistryOps::finalize_component_inventory(
        partition.binding.component,
        draining.operation_id,
        registry.clone(),
        fleet.clone(),
        112,
    )
    .expect("freeze exact final Component inventory");
    assert_final_inventory_receipt(&partition, &fleet, &inventory);

    let durable = restart_component_registry();
    let restarted = ComponentRegistryOps::component_draining(partition.binding.component)
        .expect("valid final inventory")
        .expect("durable draining authority");
    assert_eq!(restarted.final_inventory, Some(inventory.clone()));
    assert_eq!(
        ComponentRegistryOps::finalize_component_inventory(
            partition.binding.component,
            draining.operation_id,
            registry,
            fleet,
            999,
        )
        .expect("exact final-inventory retry returns the original receipt"),
        inventory
    );
    assert_eq!(RootComponentRegistryStore::export(), durable);
    ComponentRegistryOps::finalize_component_inventory(
        partition.binding.component,
        draining.operation_id,
        conflicting_registry,
        fleet_directory(&root_binding()),
        999,
    )
    .expect_err("final inventory rejects a different Registry head after commit");
    assert_eq!(RootComponentRegistryStore::export(), durable);
    assert_eq!(
        ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes,
        exact_registry_entry_bytes(&durable)
    );
    assert_eq!(
        partition.encoded_bytes,
        exact_component_registry_entry_bytes(&durable, partition.binding.component)
    );

    let mut corrupted = durable;
    corrupted.component_drainings[0]
        .final_inventory
        .as_mut()
        .expect("final inventory receipt")
        .inventory_hash = [0; 32];
    RootComponentRegistryStore::import(corrupted);
    ComponentRegistryOps::component_draining(partition.binding.component)
        .expect_err("final inventory hash must remain canonical");
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one test proves every durable top-level deletion transition and ledger invariant"
)]
fn component_deletion_is_prepared_durable_and_absence_idempotent() {
    let (partition, draining, fleet) = import_empty_quiescent_component();
    let inventory = ComponentRegistryOps::finalize_component_inventory(
        partition.binding.component,
        draining.operation_id,
        draining.registry,
        fleet,
        112,
    )
    .expect("freeze exact final Component inventory");
    let before_preparation = RootComponentRegistryStore::export();
    ComponentRegistryOps::prepare_component_deletion(
        partition.binding.component,
        draining.operation_id,
        [0; 32],
        113,
    )
    .expect_err("deletion must bind the exact final inventory hash");
    ComponentRegistryOps::prepare_component_deletion(
        partition.binding.component,
        draining.operation_id,
        inventory.inventory_hash,
        111,
    )
    .expect_err("deletion preparation cannot precede final inventory");
    assert_eq!(RootComponentRegistryStore::export(), before_preparation);

    let prepared = ComponentRegistryOps::prepare_component_deletion(
        partition.binding.component,
        draining.operation_id,
        inventory.inventory_hash,
        113,
    )
    .expect("prepare top-level Component deletion");
    let Some(RootComponentDeletionProgressView::DeleteIntent(intent)) = &prepared.deletion else {
        panic!("durable top-level deletion intent");
    };
    assert_eq!(intent.final_inventory, inventory);
    assert_eq!(
        intent.quiescence.stop.canister_id,
        partition.binding.canister_id
    );
    assert_eq!(
        intent.quiescence.stop.controller,
        partition.binding.fleet_subnet_root
    );
    assert_eq!(
        intent.quiescence.observed_module_hash,
        intent.quiescence.stop.expected_module_hash
    );
    assert_eq!(intent.prepared_at_ns, 113);

    let prepared_snapshot = restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::prepare_component_deletion(
            partition.binding.component,
            draining.operation_id,
            inventory.inventory_hash,
            999,
        )
        .expect("exact deletion preparation retry"),
        prepared
    );
    assert_eq!(RootComponentRegistryStore::export(), prepared_snapshot);
    ComponentRegistryOps::mark_component_deleted(
        partition.binding.component,
        draining.operation_id,
        [0; 32],
        114,
    )
    .expect_err("absence receipt must bind the prepared inventory");
    ComponentRegistryOps::mark_component_deleted(
        partition.binding.component,
        draining.operation_id,
        inventory.inventory_hash,
        112,
    )
    .expect_err("absence observation cannot precede deletion intent");
    assert_eq!(RootComponentRegistryStore::export(), prepared_snapshot);

    let deleted = ComponentRegistryOps::mark_component_deleted(
        partition.binding.component,
        draining.operation_id,
        inventory.inventory_hash,
        114,
    )
    .expect("commit independently observed top-level absence");
    let Some(RootComponentDeletionProgressView::Deleted(receipt)) = &deleted.deletion else {
        panic!("durable top-level deleted receipt");
    };
    assert_eq!(receipt.deletion, *intent);
    assert_eq!(receipt.deleted_at_ns, 114);

    let deleted_snapshot = restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::mark_component_deleted(
            partition.binding.component,
            draining.operation_id,
            inventory.inventory_hash,
            999,
        )
        .expect("exact deleted receipt retry"),
        deleted
    );
    assert_eq!(RootComponentRegistryStore::export(), deleted_snapshot);
    assert_eq!(
        ComponentRegistryOps::partition(partition.binding.component)
            .expect("valid retained partition")
            .expect("retained partition"),
        partition
    );
    assert_eq!(
        ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes,
        exact_registry_entry_bytes(&deleted_snapshot)
    );
    assert_eq!(
        partition.encoded_bytes,
        exact_component_registry_entry_bytes(&deleted_snapshot, partition.binding.component)
    );

    let mut corrupted = deleted_snapshot;
    let RootComponentDeletionProgressRecord::Deleted(receipt) = corrupted
        .component_drainings
        .first_mut()
        .and_then(|draining| draining.deletion.as_mut())
        .expect("deleted receipt")
    else {
        panic!("deleted progress");
    };
    receipt.deleted_at_ns = receipt.deletion.prepared_at_ns - 1;
    RootComponentRegistryStore::import(corrupted);
    ComponentRegistryOps::component_draining(partition.binding.component)
        .expect_err("deleted receipt time must remain monotonic");
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one test proves the atomic top-level membership and accounting settlement"
)]
fn component_membership_removal_is_atomic_settled_and_response_idempotent() {
    let (partition, draining, fleet) = import_empty_quiescent_component();
    let inventory = ComponentRegistryOps::finalize_component_inventory(
        partition.binding.component,
        draining.operation_id,
        draining.registry,
        fleet,
        112,
    )
    .expect("freeze exact final Component inventory");
    ComponentRegistryOps::prepare_component_deletion(
        partition.binding.component,
        draining.operation_id,
        inventory.inventory_hash,
        113,
    )
    .expect("prepare top-level deletion");
    ComponentRegistryOps::mark_component_deleted(
        partition.binding.component,
        draining.operation_id,
        inventory.inventory_hash,
        114,
    )
    .expect("observe top-level absence");
    let deleted = RootComponentRegistryStore::export();

    ComponentRegistryOps::remove_component_membership(
        partition.binding.component,
        [1; 32],
        inventory.inventory_hash,
        115,
    )
    .expect_err("membership removal must bind the draining operation");
    ComponentRegistryOps::remove_component_membership(
        partition.binding.component,
        draining.operation_id,
        [0; 32],
        115,
    )
    .expect_err("membership removal must bind the frozen inventory");
    ComponentRegistryOps::remove_component_membership(
        partition.binding.component,
        draining.operation_id,
        inventory.inventory_hash,
        113,
    )
    .expect_err("membership removal cannot precede observed deletion");
    assert_eq!(RootComponentRegistryStore::export(), deleted);

    let removed = ComponentRegistryOps::remove_component_membership(
        partition.binding.component,
        draining.operation_id,
        inventory.inventory_hash,
        115,
    )
    .expect("atomically remove top-level Component membership");
    let Some(RootComponentDeletionProgressView::MembershipRemoved(receipt)) = &removed.deletion
    else {
        panic!("terminal top-level membership-removal receipt");
    };
    assert_eq!(receipt.deleted.deleted_at_ns, 114);
    assert_eq!(receipt.allocation_operation_id, [12; 32]);
    assert_eq!(receipt.remaining_spec_committed_instances, 0);
    assert_eq!(receipt.root_committed_component_instances, 0);
    assert_eq!(receipt.root_known_created_component_canisters, 0);
    assert_eq!(receipt.removed_at_ns, 115);
    assert_ne!(receipt.removal_hash, [0; 32]);
    assert_eq!(
        ComponentRegistryOps::partition(partition.binding.component)
            .expect("removed partition lookup"),
        None
    );
    assert_eq!(
        RootComponentRegistryStore::component_for_principal(partition.binding.canister_id),
        None
    );
    assert!(
        RootComponentRegistryStore::component_principal_inventory_is_empty(
            partition.binding.component
        )
    );
    assert!(matches!(
        ComponentRegistryOps::allocation([12; 32])
            .expect("retained allocation history")
            .progress,
        RootComponentAllocationProgressView::Removed { .. }
    ));
    assert_eq!(
        ComponentRegistryOps::component_spec_counts(&partition.binding.component_spec)
            .expect("settled Spec counts"),
        ComponentSpecInstanceCounts {
            reserved: 0,
            committed: 0,
        }
    );

    let terminal = restart_component_registry();
    let current = ComponentRegistryOps::current().expect("settled Registry status");
    assert_eq!(current.committed_component_instances, 0);
    assert_eq!(current.known_created_component_canisters, 0);
    assert_eq!(current.encoded_bytes, receipt.root_registry_encoded_bytes);
    assert_eq!(current.encoded_bytes, exact_registry_entry_bytes(&terminal));
    assert_eq!(terminal.partitions, Vec::new());
    assert_eq!(
        ComponentRegistryOps::component_draining(partition.binding.component)
            .expect("valid terminal authority")
            .expect("retained terminal authority"),
        removed
    );
    ComponentRegistryOps::reserve_allocation(
        TopLevelComponentAllocationDecision {
            allocation_sequence: 2,
            component: ComponentInstanceId::from_generated_bytes([122; 32]),
            component_spec: partition.binding.component_spec.clone(),
            spec_hash: partition.binding.spec_hash,
            role: partition.binding.role.clone(),
        },
        [123; 32],
        partition.provisioning_origin.clone(),
        false,
    )
    .expect("unrelated later allocation");
    let evolved = RootComponentRegistryStore::export();
    assert_ne!(
        ComponentRegistryOps::current()
            .expect("evolved Registry status")
            .encoded_bytes,
        receipt.root_registry_encoded_bytes
    );
    assert_eq!(
        ComponentRegistryOps::remove_component_membership(
            partition.binding.component,
            draining.operation_id,
            inventory.inventory_hash,
            999,
        )
        .expect("exact membership-removal retry"),
        removed
    );
    assert_eq!(RootComponentRegistryStore::export(), evolved);

    let mut corrupted = evolved;
    let RootComponentDeletionProgressRecord::MembershipRemoved(receipt) = corrupted
        .component_drainings
        .first_mut()
        .and_then(|draining| draining.deletion.as_mut())
        .expect("membership-removal receipt")
    else {
        panic!("terminal membership-removal progress");
    };
    receipt.root_committed_component_instances = 1;
    RootComponentRegistryStore::import(corrupted);
    ComponentRegistryOps::component_draining(partition.binding.component)
        .expect_err("terminal receipt must remain bound to settled root accounting");
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one test proves the durable fence across a multi-level Component tree"
)]
fn subtree_removal_fence_is_durable_scoped_and_capacity_bounded() {
    let fixture = import_active_component_tree();
    let initial = RootComponentRegistryStore::export();
    let registry = component_registry_head(&fixture.partition);

    ComponentRegistryOps::reserve_child_allocation(
        child_allocation_decision_for_parent(
            &fixture.partition,
            fixture.descendant.canister_id,
            &fixture.descendant.role,
            "project_machine",
        ),
        [69; 32],
        None,
        registry.clone(),
    )
    .expect("reserve in-flight descendant");
    let before_inflight_rejection = RootComponentRegistryStore::export();
    ComponentRegistryOps::begin_subtree_removal(
        fixture.component,
        [70; 32],
        fixture.target.canister_id,
        registry.clone(),
        16_777_216,
    )
    .expect_err("in-flight descendant lifecycle must prevent fencing");
    assert_eq!(
        RootComponentRegistryStore::export(),
        before_inflight_rejection
    );
    RootComponentRegistryStore::import(initial.clone());

    ComponentRegistryOps::begin_subtree_removal(
        fixture.component,
        [70; 32],
        fixture.target.canister_id,
        registry.clone(),
        fixture.partition.encoded_bytes,
    )
    .expect_err("subtree fence must fit before mutation");
    assert_eq!(RootComponentRegistryStore::export(), initial);

    let fenced = ComponentRegistryOps::begin_subtree_removal(
        fixture.component,
        [70; 32],
        fixture.target.canister_id,
        registry.clone(),
        16_777_216,
    )
    .expect("durably fence target subtree");
    assert_eq!(fenced.target_canister_id, fixture.target.canister_id);
    assert_eq!(
        fenced.target_parent_canister_id,
        fixture.target.parent_canister_id
    );
    assert_eq!(
        fenced.progress,
        RootComponentSubtreeRemovalProgressView::Fenced
    );
    let durable_fence = restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::subtree_removal(fixture.component, [70; 32])
            .expect("valid subtree removal")
            .expect("durable subtree removal"),
        fenced
    );
    assert_eq!(
        ComponentRegistryOps::begin_subtree_removal(
            fixture.component,
            [70; 32],
            fixture.target.canister_id,
            registry.clone(),
            16_777_216,
        )
        .expect("exact fence retry"),
        fenced
    );
    let current = ComponentRegistryOps::current().expect("Registry status");
    let partition = ComponentRegistryOps::partition(fixture.component)
        .expect("partition read")
        .expect("active partition");
    assert_eq!(
        partition.encoded_bytes,
        exact_component_registry_entry_bytes(&durable_fence, fixture.component)
    );
    assert_eq!(
        current.encoded_bytes,
        exact_registry_entry_bytes(&durable_fence)
    );

    for (operation_id, parent) in [([71; 32], &fixture.target), ([72; 32], &fixture.descendant)] {
        let before = RootComponentRegistryStore::export();
        ComponentRegistryOps::reserve_child_allocation(
            child_allocation_decision_for_parent(
                &fixture.partition,
                parent.canister_id,
                &parent.role,
                "project_machine",
            ),
            operation_id,
            None,
            registry.clone(),
        )
        .expect_err("fenced subtree member cannot reserve a new child");
        assert_eq!(RootComponentRegistryStore::export(), before);
    }

    let before_second_fence = RootComponentRegistryStore::export();
    ComponentRegistryOps::begin_subtree_removal(
        fixture.component,
        [73; 32],
        fixture.unrelated.canister_id,
        registry.clone(),
        16_777_216,
    )
    .expect_err("one Component admits only one in-progress subtree removal");
    assert_eq!(RootComponentRegistryStore::export(), before_second_fence);

    ComponentRegistryOps::reserve_child_allocation(
        child_allocation_decision_for_parent(
            &fixture.partition,
            fixture.unrelated.canister_id,
            &fixture.unrelated.role,
            "project_machine",
        ),
        [74; 32],
        None,
        registry.clone(),
    )
    .expect("unrelated branch remains mutable");
    assert_eq!(
        ComponentRegistryOps::begin_subtree_removal(
            fixture.component,
            [70; 32],
            fixture.target.canister_id,
            registry,
            16_777_216,
        )
        .expect("fence retry survives unrelated progress"),
        fenced
    );

    let before_premature_stop = RootComponentRegistryStore::export();
    ComponentRegistryOps::prepare_subtree_leaf_stop(
        fixture.component,
        [70; 32],
        0,
        fixture.target.canister_id,
        fixture.target.parent_canister_id,
        16_777_216,
    )
    .expect_err("stop preparation requires one selected childless leaf");
    assert_eq!(RootComponentRegistryStore::export(), before_premature_stop);

    let before_traversal = RootComponentRegistryStore::export();
    let before_traversal_partition = ComponentRegistryOps::partition(fixture.component)
        .expect("partition read")
        .expect("active partition");
    ComponentRegistryOps::advance_subtree_removal(
        fixture.component,
        [70; 32],
        0,
        before_traversal_partition.encoded_bytes,
    )
    .expect_err("larger traversal receipt must fit before mutation");
    assert_eq!(RootComponentRegistryStore::export(), before_traversal);

    let selected =
        ComponentRegistryOps::advance_subtree_removal(fixture.component, [70; 32], 0, 16_777_216)
            .expect("select first post-order leaf");
    assert_eq!(selected.traversal_steps, 2);
    assert!(matches!(
        &selected.progress,
        RootComponentSubtreeRemovalProgressView::LeafSelected { leaf }
            if leaf.canister_id == fixture.descendant.canister_id
                && leaf.parent_canister_id == fixture.target.canister_id
    ));
    let selected_state = restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::advance_subtree_removal(fixture.component, [70; 32], 0, 16_777_216,)
            .expect("stale traversal retry returns current progress"),
        selected
    );
    assert_eq!(RootComponentRegistryStore::export(), selected_state);

    let selected_partition = ComponentRegistryOps::partition(fixture.component)
        .expect("partition read")
        .expect("active partition");
    ComponentRegistryOps::prepare_subtree_leaf_stop(
        fixture.component,
        [70; 32],
        selected.traversal_steps,
        fixture.descendant.canister_id,
        fixture.target.canister_id,
        selected_partition.encoded_bytes,
    )
    .expect_err("larger stop intent must fit before mutation");
    assert_eq!(RootComponentRegistryStore::export(), selected_state);

    for (steps, leaf, parent) in [
        (
            selected.traversal_steps + 1,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
        ),
        (
            selected.traversal_steps,
            fixture.target.canister_id,
            fixture.target.parent_canister_id,
        ),
        (
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.parent_canister_id,
        ),
    ] {
        ComponentRegistryOps::prepare_subtree_leaf_stop(
            fixture.component,
            [70; 32],
            steps,
            leaf,
            parent,
            16_777_216,
        )
        .expect_err("stop preparation must bind the exact selected leaf observation");
        assert_eq!(RootComponentRegistryStore::export(), selected_state);
    }

    let prepared = ComponentRegistryOps::prepare_subtree_leaf_stop(
        fixture.component,
        [70; 32],
        selected.traversal_steps,
        fixture.descendant.canister_id,
        fixture.target.canister_id,
        16_777_216,
    )
    .expect("freeze exact leaf stop intent");
    assert!(matches!(
        &prepared.progress,
        RootComponentSubtreeRemovalProgressView::StopIntent(effect)
            if effect.leaf.canister_id == fixture.descendant.canister_id
                && effect.leaf.parent_canister_id == fixture.target.canister_id
                && effect.leaf.installed_artifact_hash
                    == fixture.descendant.installed_artifact_hash
                && effect.controller == fixture.partition.binding.fleet_subnet_root
    ));
    let prepared_state = restart_component_registry();
    let mut corrupted_controller = prepared_state.clone();
    let RootComponentSubtreeRemovalProgressRecord::StopIntent(effect) =
        &mut corrupted_controller.subtree_removals[0].progress
    else {
        panic!("durable stop intent");
    };
    effect.controller = Principal::from_slice(&[99; 29]);
    RootComponentRegistryStore::import(corrupted_controller);
    ComponentRegistryOps::subtree_removal(fixture.component, [70; 32])
        .expect_err("stop intent must retain the exact protected root controller");
    RootComponentRegistryStore::import(prepared_state.clone());
    ComponentRegistryOps::prepare_subtree_leaf_stop(
        fixture.component,
        [70; 32],
        selected.traversal_steps,
        fixture.descendant.canister_id,
        fixture.target.parent_canister_id,
        16_777_216,
    )
    .expect_err("durable stop intent rejects conflicting parent observation");
    assert_eq!(RootComponentRegistryStore::export(), prepared_state);
    assert_eq!(
        ComponentRegistryOps::prepare_subtree_leaf_stop(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            16_777_216,
        )
        .expect("exact stop preparation retry"),
        prepared
    );
    assert_eq!(RootComponentRegistryStore::export(), prepared_state);
    assert_eq!(
        ComponentRegistryOps::advance_subtree_removal(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            16_777_216,
        )
        .expect("traversal retry converges on stop intent"),
        prepared
    );
    assert_eq!(RootComponentRegistryStore::export(), prepared_state);

    let prepared_partition = ComponentRegistryOps::partition(fixture.component)
        .expect("partition read")
        .expect("active partition");
    ComponentRegistryOps::mark_subtree_leaf_stopped(
        fixture.component,
        [70; 32],
        selected.traversal_steps,
        fixture.descendant.canister_id,
        fixture.target.canister_id,
        [55; 32],
        prepared_partition.encoded_bytes,
    )
    .expect_err("larger stopped receipt must fit before mutation");
    assert_eq!(RootComponentRegistryStore::export(), prepared_state);

    let stopped = ComponentRegistryOps::mark_subtree_leaf_stopped(
        fixture.component,
        [70; 32],
        selected.traversal_steps,
        fixture.descendant.canister_id,
        fixture.target.canister_id,
        [55; 32],
        16_777_216,
    )
    .expect("commit independently observed stopped receipt");
    assert!(matches!(
        &stopped.progress,
        RootComponentSubtreeRemovalProgressView::Stopped(receipt)
            if receipt.stop.leaf.canister_id == fixture.descendant.canister_id
                && receipt.stop.leaf.parent_canister_id == fixture.target.canister_id
                && receipt.stop.controller == fixture.partition.binding.fleet_subnet_root
                && receipt.observed_module_hash == [55; 32]
    ));
    let stopped_state = restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::mark_subtree_leaf_stopped(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            [55; 32],
            16_777_216,
        )
        .expect("exact stopped receipt retry"),
        stopped
    );
    assert_eq!(RootComponentRegistryStore::export(), stopped_state);
    ComponentRegistryOps::mark_subtree_leaf_stopped(
        fixture.component,
        [70; 32],
        selected.traversal_steps,
        fixture.descendant.canister_id,
        fixture.target.canister_id,
        [56; 32],
        16_777_216,
    )
    .expect_err("stopped receipt rejects conflicting module observation");
    assert_eq!(RootComponentRegistryStore::export(), stopped_state);
    assert_eq!(
        ComponentRegistryOps::prepare_subtree_leaf_stop(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            16_777_216,
        )
        .expect("stop preparation converges on stopped receipt"),
        stopped
    );
    assert_eq!(RootComponentRegistryStore::export(), stopped_state);

    ComponentRegistryOps::prepare_subtree_leaf_delete(
        fixture.component,
        [70; 32],
        selected.traversal_steps,
        fixture.descendant.canister_id,
        fixture.target.canister_id,
        1,
    )
    .expect_err("deletion intent must fit before mutation");
    assert_eq!(RootComponentRegistryStore::export(), stopped_state);
    ComponentRegistryOps::prepare_subtree_leaf_delete(
        fixture.component,
        [70; 32],
        selected.traversal_steps,
        fixture.descendant.canister_id,
        fixture.target.parent_canister_id,
        16_777_216,
    )
    .expect_err("deletion intent rejects a conflicting parent observation");
    assert_eq!(RootComponentRegistryStore::export(), stopped_state);

    let deletion = ComponentRegistryOps::prepare_subtree_leaf_delete(
        fixture.component,
        [70; 32],
        selected.traversal_steps,
        fixture.descendant.canister_id,
        fixture.target.canister_id,
        16_777_216,
    )
    .expect("freeze exact stopped receipt as deletion intent");
    assert!(matches!(
        &deletion.progress,
        RootComponentSubtreeRemovalProgressView::DeleteIntent(intent)
            if intent.stopped.stop.leaf.canister_id == fixture.descendant.canister_id
                && intent.stopped.stop.leaf.parent_canister_id
                    == fixture.target.canister_id
                && intent.stopped.stop.controller
                    == fixture.partition.binding.fleet_subnet_root
                && intent.stopped.observed_module_hash == [55; 32]
    ));
    let deletion_state = restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::prepare_subtree_leaf_delete(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            16_777_216,
        )
        .expect("exact deletion preparation retry"),
        deletion
    );
    assert_eq!(RootComponentRegistryStore::export(), deletion_state);

    ComponentRegistryOps::mark_subtree_leaf_deleted(
        fixture.component,
        [70; 32],
        selected.traversal_steps,
        fixture.descendant.canister_id,
        fixture.target.canister_id,
        1,
    )
    .expect_err("deleted receipt must fit before mutation");
    assert_eq!(RootComponentRegistryStore::export(), deletion_state);

    let deleted = ComponentRegistryOps::mark_subtree_leaf_deleted(
        fixture.component,
        [70; 32],
        selected.traversal_steps,
        fixture.descendant.canister_id,
        fixture.target.canister_id,
        16_777_216,
    )
    .expect("commit independently observed deleted receipt");
    assert!(matches!(
        &deleted.progress,
        RootComponentSubtreeRemovalProgressView::Deleted(receipt)
            if receipt.deletion.stopped.stop.leaf.canister_id
                == fixture.descendant.canister_id
                && receipt.deletion.stopped.observed_module_hash == [55; 32]
    ));
    let deleted_state = restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::mark_subtree_leaf_deleted(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            16_777_216,
        )
        .expect("exact deleted receipt retry"),
        deleted
    );
    assert_eq!(RootComponentRegistryStore::export(), deleted_state);
    assert_eq!(
        ComponentRegistryOps::mark_subtree_leaf_stopped(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            [55; 32],
            16_777_216,
        )
        .expect("stale stopped retry converges on deleted receipt"),
        deleted
    );
    assert_eq!(RootComponentRegistryStore::export(), deleted_state);
    assert_eq!(
        ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes,
        exact_registry_entry_bytes(&deleted_state)
    );
    assert_eq!(
        ComponentRegistryOps::partition(fixture.component)
            .expect("partition read")
            .expect("active partition")
            .encoded_bytes,
        exact_component_registry_entry_bytes(&deleted_state, fixture.component)
    );

    let deleted_partition = ComponentRegistryOps::partition(fixture.component)
        .expect("partition read")
        .expect("active partition");
    let active_fleet_directory = fleet_directory(&root_binding());
    ComponentRegistryOps::remove_subtree_leaf_membership(
        fixture.component,
        [70; 32],
        selected.traversal_steps,
        fixture.descendant.canister_id,
        fixture.target.parent_canister_id,
        deleted_partition.directory_synchronized_at_ns + 1,
        16_777_216,
        active_fleet_directory.clone(),
    )
    .expect_err("membership removal rejects a conflicting parent observation");
    assert_eq!(RootComponentRegistryStore::export(), deleted_state);
    ComponentRegistryOps::remove_subtree_leaf_membership(
        fixture.component,
        [70; 32],
        selected.traversal_steps,
        fixture.descendant.canister_id,
        fixture.target.canister_id,
        deleted_partition.directory_synchronized_at_ns + 1,
        1,
        active_fleet_directory.clone(),
    )
    .expect_err("membership-removal receipt must fit before mutation");
    assert_eq!(RootComponentRegistryStore::export(), deleted_state);

    let membership_removed = ComponentRegistryOps::remove_subtree_leaf_membership(
        fixture.component,
        [70; 32],
        selected.traversal_steps,
        fixture.descendant.canister_id,
        fixture.target.canister_id,
        deleted_partition.directory_synchronized_at_ns + 1,
        16_777_216,
        active_fleet_directory.clone(),
    )
    .expect("atomically remove deleted leaf membership");
    let RootComponentSubtreeRemovalProgressView::MembershipRemoved(receipt) =
        &membership_removed.progress
    else {
        panic!("durable membership-removal receipt");
    };
    assert_eq!(
        receipt.deleted.deletion.stopped.observed_module_hash,
        [55; 32]
    );
    assert_eq!(
        receipt.removed_from_registry,
        ComponentRegistryHead {
            component: fixture.component,
            revision: deleted_partition.revision,
            content_hash: deleted_partition.content_hash,
        }
    );
    assert_eq!(receipt.previous_committed_descendants, 4);
    assert_eq!(receipt.committed_descendants, 3);
    assert_eq!(receipt.reserved_descendants, 1);
    assert_eq!(receipt.parent_role_instances, 0);
    assert_eq!(receipt.root_managed_descendants, 4);
    assert_eq!(receipt.root_known_created_component_canisters, 4);
    assert_eq!(
        receipt.directory_authority_hash,
        component_directory_authority_hash(
            &deleted_partition.binding,
            receipt.registry.revision,
            receipt.registry.content_hash,
            receipt.directory_synchronized_at_ns,
            receipt.committed_descendants,
            &active_fleet_directory,
        )
        .expect("membership-removal Directory authority hash")
    );
    assert_eq!(
        RootComponentRegistryStore::child(fixture.component, fixture.descendant.canister_id),
        None
    );
    assert_eq!(
        RootComponentRegistryStore::child_traversal(
            fixture.component,
            fixture.target.canister_id,
            &fixture.descendant.role,
            fixture.descendant.canister_id,
        ),
        None
    );
    assert_eq!(
        RootComponentRegistryStore::component_for_principal(fixture.descendant.canister_id),
        None
    );
    assert_eq!(
        ComponentRegistryOps::parent_role_instances(
            fixture.component,
            fixture.target.canister_id,
            &fixture.descendant.role,
        )
        .expect("parent-role count"),
        0
    );
    let membership_removed_state = restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::subtree_removal(fixture.component, [70; 32])
            .expect("valid membership-removal receipt")
            .expect("durable membership-removal receipt"),
        membership_removed
    );
    assert_eq!(
        ComponentRegistryOps::remove_subtree_leaf_membership(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            deleted_partition.directory_synchronized_at_ns + 1,
            16_777_216,
            active_fleet_directory.clone(),
        )
        .expect("exact membership-removal retry"),
        membership_removed
    );
    assert_eq!(
        RootComponentRegistryStore::export(),
        membership_removed_state
    );
    assert_eq!(
        ComponentRegistryOps::mark_subtree_leaf_deleted(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            16_777_216,
        )
        .expect("stale deleted retry converges on membership-removal receipt"),
        membership_removed
    );
    let current = ComponentRegistryOps::current().expect("Registry status");
    let partition = ComponentRegistryOps::partition(fixture.component)
        .expect("partition read")
        .expect("active partition");
    assert_eq!(current.managed_descendants, 4);
    assert_eq!(current.known_created_component_canisters, 4);
    assert_eq!(partition.committed_descendants, 3);
    assert_eq!(partition.encoded_bytes, receipt.registry_encoded_bytes);
    assert_eq!(
        current.encoded_bytes,
        exact_registry_entry_bytes(&membership_removed_state)
    );
    assert_eq!(
        partition.encoded_bytes,
        exact_component_registry_entry_bytes(&membership_removed_state, fixture.component)
    );

    let directory_authority = ComponentRuntimeDirectoryAuthority {
        fleet: active_fleet_directory,
        component: ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: partition.binding.clone(),
                source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                component_registry_revision: partition.revision,
                component_registry_content_hash: partition.content_hash,
                synchronized_at_ns: partition.directory_synchronized_at_ns,
            },
            descendant_count: partition.committed_descendants,
        },
        component_group: None,
    };
    let directory_authority_hash =
        ComponentRuntimeOps::directory_authority_hash(&directory_authority)
            .expect("current Directory authority hash");
    let owning_component = ComponentRuntimeDirectoryConvergenceEvidence {
        operation_id: [81; 32],
        binding: ManagedCanisterBinding::Component(partition.binding.clone()),
        covered_authority: directory_authority.clone(),
        covered_authority_hash: directory_authority_hash,
        activation: ComponentRuntimeActivationEvidence {
            directory_authority_hash: [82; 32],
            activated_at_ns: 83,
        },
    };
    let parent_binding =
        ComponentRegistryOps::registered_parent(fixture.component, fixture.target.canister_id)
            .expect("registered parent read")
            .expect("registered parent")
            .0;
    let parent = ComponentRuntimeDirectoryConvergenceEvidence {
        operation_id: [84; 32],
        binding: parent_binding,
        covered_authority: directory_authority.clone(),
        covered_authority_hash: directory_authority_hash,
        activation: ComponentRuntimeActivationEvidence {
            directory_authority_hash: [85; 32],
            activated_at_ns: 86,
        },
    };
    ComponentRegistryOps::mark_subtree_leaf_directory_synchronized(
        fixture.component,
        [70; 32],
        selected.traversal_steps,
        fixture.descendant.canister_id,
        fixture.target.canister_id,
        directory_authority.clone(),
        directory_authority_hash,
        Some(owning_component.clone()),
        Some(parent.clone()),
        1,
    )
    .expect_err("Directory convergence receipt must fit before mutation");
    assert_eq!(
        RootComponentRegistryStore::export(),
        membership_removed_state
    );

    let directory_synchronized = ComponentRegistryOps::mark_subtree_leaf_directory_synchronized(
        fixture.component,
        [70; 32],
        selected.traversal_steps,
        fixture.descendant.canister_id,
        fixture.target.canister_id,
        directory_authority.clone(),
        directory_authority_hash,
        Some(owning_component.clone()),
        Some(parent.clone()),
        16_777_216,
    )
    .expect("retain surviving-member Directory convergence");
    assert!(matches!(
        &directory_synchronized.progress,
        RootComponentSubtreeRemovalProgressView::DirectorySynchronized(receipt)
            if receipt.membership_removed.registry == receipt.covered_component_registry
                && receipt.owning_component.as_ref().map(|evidence| evidence.canister_id)
                    == Some(partition.binding.canister_id)
                && receipt.parent.as_ref().map(|evidence| evidence.canister_id)
                    == Some(fixture.target.canister_id)
                && receipt.covered_authority_hash == directory_authority_hash
    ));
    let directory_synchronized_state = restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::mark_subtree_leaf_directory_synchronized(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            directory_authority.clone(),
            directory_authority_hash,
            Some(owning_component.clone()),
            Some(parent.clone()),
            16_777_216,
        )
        .expect("exact Directory synchronization retry"),
        directory_synchronized
    );
    assert_eq!(
        RootComponentRegistryStore::export(),
        directory_synchronized_state
    );
    assert_eq!(
        ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes,
        exact_registry_entry_bytes(&directory_synchronized_state)
    );
    assert_eq!(
        ComponentRegistryOps::partition(fixture.component)
            .expect("partition read")
            .expect("active partition")
            .encoded_bytes,
        exact_component_registry_entry_bytes(&directory_synchronized_state, fixture.component)
    );

    let resumed = ComponentRegistryOps::finalize_subtree_leaf(
        fixture.component,
        [70; 32],
        selected.traversal_steps,
        fixture.descendant.canister_id,
        fixture.target.canister_id,
        directory_synchronized_state.partitions[0].encoded_bytes,
    )
    .expect("compact completed leaf and resume within the existing byte ceiling");
    assert_eq!(resumed.completed_leaves, 1);
    assert_eq!(resumed.maximum_completed_leaves, 4);
    assert!(matches!(
        &resumed.progress,
        RootComponentSubtreeRemovalProgressView::Traversing { cursor }
            if cursor.canister_id == fixture.target.canister_id
    ));
    let resumed_state = restart_component_registry();
    assert_eq!(resumed_state.subtree_removal_history.len(), 1);
    assert!(
        resumed_state.partitions[0].encoded_bytes
            <= directory_synchronized_state.partitions[0].encoded_bytes,
        "normalization must not require more Component Registry capacity"
    );
    let RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(expected_history_receipt) =
        &directory_synchronized_state.subtree_removals[0].progress
    else {
        panic!("Directory-synchronized history source");
    };
    let history = &resumed_state.subtree_removal_history[0];
    assert_eq!(history.leaf_canister_id, fixture.descendant.canister_id);
    assert_eq!(history.leaf_parent_canister_id, fixture.target.canister_id);
    assert_eq!(history.observed_module_hash, [55; 32]);
    assert_eq!(
        history.receipt_hash,
        subtree_directory_synchronized_receipt_hash(expected_history_receipt)
            .expect("canonical completed-leaf receipt hash")
    );
    assert!(
        RootComponentRegistryStore::subtree_removal_completed_leaf_entry_bytes(history)
            < RootComponentRegistryStore::subtree_removal_entry_bytes(
                &directory_synchronized_state.subtree_removals[0]
            ),
        "normalized leaf history must remain smaller than its live source receipt"
    );
    assert_eq!(
        ComponentRegistryOps::finalize_subtree_leaf(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            16_777_216,
        )
        .expect("exact leaf-finalization retry"),
        resumed
    );
    assert_eq!(RootComponentRegistryStore::export(), resumed_state);
    assert_eq!(
        ComponentRegistryOps::prepare_subtree_leaf_stop(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            16_777_216,
        )
        .expect("stale stop preparation resolves through completed history"),
        resumed
    );
    assert_eq!(
        ComponentRegistryOps::mark_subtree_leaf_stopped(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            [55; 32],
            16_777_216,
        )
        .expect("stale stopped receipt resolves through completed history"),
        resumed
    );
    assert_eq!(
        ComponentRegistryOps::mark_subtree_leaf_directory_synchronized(
            fixture.component,
            [70; 32],
            selected.traversal_steps,
            fixture.descendant.canister_id,
            fixture.target.canister_id,
            directory_authority,
            directory_authority_hash,
            Some(owning_component),
            Some(parent),
            16_777_216,
        )
        .expect("stale Directory synchronization resolves through completed history"),
        resumed
    );
    assert_eq!(RootComponentRegistryStore::export(), resumed_state);
    assert_eq!(
        ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes,
        exact_registry_entry_bytes(&resumed_state)
    );
    assert_eq!(
        ComponentRegistryOps::partition(fixture.component)
            .expect("partition read")
            .expect("active partition")
            .encoded_bytes,
        exact_component_registry_entry_bytes(&resumed_state, fixture.component)
    );

    let next_selected = ComponentRegistryOps::advance_subtree_removal(
        fixture.component,
        [70; 32],
        resumed.traversal_steps,
        16_777_216,
    )
    .expect("select retained parent after completed child");
    assert!(next_selected.traversal_steps > resumed.traversal_steps);
    assert!(matches!(
        &next_selected.progress,
        RootComponentSubtreeRemovalProgressView::LeafSelected { leaf }
            if leaf.canister_id != fixture.target.canister_id
                && leaf.parent_canister_id == fixture.target.canister_id
    ));
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

#[test]
fn subtree_removal_traversal_is_bounded_deterministic_and_restart_safe() {
    let (fixture, expected_leaf) = import_deep_active_component_tree(65);
    let registry = component_registry_head(&fixture.partition);
    ComponentRegistryOps::begin_subtree_removal(
        fixture.component,
        [80; 32],
        fixture.target.canister_id,
        registry,
        16_777_216,
    )
    .expect("fence deep subtree");

    let traversing =
        ComponentRegistryOps::advance_subtree_removal(fixture.component, [80; 32], 0, 16_777_216)
            .expect("advance one bounded traversal batch");
    assert_eq!(
        traversing.traversal_steps,
        SUBTREE_REMOVAL_TRAVERSAL_BATCH_SIZE
    );
    assert!(matches!(
        &traversing.progress,
        RootComponentSubtreeRemovalProgressView::Traversing { cursor }
            if cursor.canister_id != expected_leaf
    ));

    let durable_midpoint = restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::advance_subtree_removal(fixture.component, [80; 32], 0, 16_777_216,)
            .expect("stale traversal retry"),
        traversing
    );
    assert_eq!(RootComponentRegistryStore::export(), durable_midpoint);

    let selected = ComponentRegistryOps::advance_subtree_removal(
        fixture.component,
        [80; 32],
        traversing.traversal_steps,
        16_777_216,
    )
    .expect("resume traversal and select deepest leaf");
    assert_eq!(selected.traversal_steps, 67);
    assert!(matches!(
        &selected.progress,
        RootComponentSubtreeRemovalProgressView::LeafSelected { leaf }
            if leaf.canister_id == expected_leaf
    ));
    let terminal = restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::subtree_removal(fixture.component, [80; 32])
            .expect("valid traversal status")
            .expect("durable traversal status"),
        selected
    );
    assert_eq!(
        ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes,
        exact_registry_entry_bytes(&terminal)
    );
    assert_eq!(
        ComponentRegistryOps::partition(fixture.component)
            .expect("partition read")
            .expect("active partition")
            .encoded_bytes,
        exact_component_registry_entry_bytes(&terminal, fixture.component)
    );
    let before_ahead_rejection = RootComponentRegistryStore::export();
    ComponentRegistryOps::advance_subtree_removal(
        fixture.component,
        [80; 32],
        selected.traversal_steps + 1,
        16_777_216,
    )
    .expect_err("future traversal expectation must fail");
    assert_eq!(RootComponentRegistryStore::export(), before_ahead_rejection);
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

#[test]
fn grouped_component_rejects_ordinary_draining_before_mutation() {
    let fixture = import_grouped_active_component_tree();
    let before = RootComponentRegistryStore::export();
    let error = ComponentRegistryOps::begin_component_draining(
        fixture.component,
        [78; 32],
        component_registry_head(&fixture.partition),
        100,
        16_777_216,
        fleet_directory(&before.current.as_ref().expect("Registry meta").root),
    )
    .expect_err("grouped Component must reject the ordinary draining lifecycle");

    assert_eq!(
        error.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    assert_eq!(RootComponentRegistryStore::export(), before);
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one test proves the complete durable Component draining fence"
)]
fn component_draining_fence_is_durable_capacity_bounded_and_stops_growth() {
    let fixture = import_active_component_tree();
    let initial = RootComponentRegistryStore::export();
    let previous_registry = component_registry_head(&fixture.partition);

    ComponentRegistryOps::reserve_child_allocation(
        child_allocation_decision_for_parent(
            &fixture.partition,
            fixture.unrelated.canister_id,
            &fixture.unrelated.role,
            "project_machine",
        ),
        [77; 32],
        None,
        previous_registry.clone(),
    )
    .expect("reserve in-flight descendant");
    let before_inflight_rejection = RootComponentRegistryStore::export();
    ComponentRegistryOps::begin_component_draining(
        fixture.component,
        [79; 32],
        previous_registry.clone(),
        100,
        16_777_216,
        fleet_directory(
            &before_inflight_rejection
                .current
                .as_ref()
                .expect("Registry meta")
                .root,
        ),
    )
    .expect_err("in-flight child lifecycle must prevent draining");
    assert_eq!(
        RootComponentRegistryStore::export(),
        before_inflight_rejection
    );
    RootComponentRegistryStore::import(initial.clone());

    ComponentRegistryOps::begin_subtree_removal(
        fixture.component,
        [78; 32],
        fixture.target.canister_id,
        previous_registry.clone(),
        16_777_216,
    )
    .expect("fence in-progress subtree removal");
    let before_removal_rejection = RootComponentRegistryStore::export();
    ComponentRegistryOps::begin_component_draining(
        fixture.component,
        [79; 32],
        previous_registry.clone(),
        100,
        16_777_216,
        fleet_directory(
            &before_removal_rejection
                .current
                .as_ref()
                .expect("Registry meta")
                .root,
        ),
    )
    .expect_err("in-progress subtree removal must prevent draining");
    assert_eq!(
        RootComponentRegistryStore::export(),
        before_removal_rejection
    );
    RootComponentRegistryStore::import(initial.clone());

    ComponentRegistryOps::begin_component_draining(
        fixture.component,
        [79; 32],
        previous_registry.clone(),
        100,
        fixture.partition.encoded_bytes,
        fleet_directory(&initial.current.as_ref().expect("Registry meta").root),
    )
    .expect_err("draining receipt must fit before mutation");
    assert_eq!(RootComponentRegistryStore::export(), initial);

    let draining = ComponentRegistryOps::begin_component_draining(
        fixture.component,
        [79; 32],
        previous_registry.clone(),
        100,
        16_777_216,
        fleet_directory(&initial.current.as_ref().expect("Registry meta").root),
    )
    .expect("durably fence Component growth");
    assert_eq!(draining.previous_registry, previous_registry);
    assert_eq!(draining.registry.revision, fixture.partition.revision + 1);
    assert_eq!(draining.descendant_count, 4);
    assert_eq!(
        draining.descendant_content_hash,
        fixture.partition.descendant_content_hash
    );
    assert_ne!(draining.directory_authority_hash, [0; 32]);

    let durable = restart_component_registry();
    let partition = ComponentRegistryOps::partition(fixture.component)
        .expect("valid draining partition")
        .expect("draining partition");
    assert_eq!(partition.status, ComponentLifecycleStatus::Draining);
    assert_eq!(partition.revision, draining.registry.revision);
    assert_eq!(partition.content_hash, draining.registry.content_hash);
    assert_eq!(
        ComponentRegistryOps::component_for_principal(fixture.partition.binding.canister_id),
        Some(fixture.component)
    );
    assert_eq!(
        ComponentRegistryOps::component_draining(fixture.component)
            .expect("valid draining receipt")
            .expect("durable draining receipt"),
        draining
    );
    let mut corrupted = durable.clone();
    corrupted.component_drainings[0].descendant_content_hash = [0; 32];
    RootComponentRegistryStore::import(corrupted);
    ComponentRegistryOps::component_draining(fixture.component)
        .expect_err("draining receipt must retain its canonical descendant digest");
    RootComponentRegistryStore::import(durable.clone());
    assert_eq!(
        ComponentRegistryOps::begin_component_draining(
            fixture.component,
            [79; 32],
            draining.previous_registry.clone(),
            101,
            16_777_216,
            fleet_directory(&initial.current.as_ref().expect("Registry meta").root),
        )
        .expect("exact retry"),
        draining
    );
    assert_eq!(
        partition.encoded_bytes,
        exact_component_registry_entry_bytes(&durable, fixture.component)
    );
    assert_eq!(
        ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes,
        exact_registry_entry_bytes(&durable)
    );

    let before_growth = RootComponentRegistryStore::export();
    ComponentRegistryOps::reserve_child_allocation(
        child_allocation_decision_for_parent(
            &fixture.partition,
            fixture.unrelated.canister_id,
            &fixture.unrelated.role,
            "project_machine",
        ),
        [80; 32],
        None,
        draining.registry.clone(),
    )
    .expect_err("Draining Component cannot reserve a new child");
    assert_eq!(RootComponentRegistryStore::export(), before_growth);

    ComponentRegistryOps::begin_subtree_removal(
        fixture.component,
        [81; 32],
        fixture.target.canister_id,
        draining.registry.clone(),
        16_777_216,
    )
    .expect_err("Draining Component must be quiescent before post-order removal");
    assert_eq!(RootComponentRegistryStore::export(), before_growth);

    let active_fleet_directory =
        fleet_directory(&initial.current.as_ref().expect("Registry meta").root);
    let directory_authority = ComponentRuntimeDirectoryAuthority {
        fleet: active_fleet_directory,
        component: ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: partition.binding.clone(),
                source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                component_registry_revision: partition.revision,
                component_registry_content_hash: partition.content_hash,
                synchronized_at_ns: partition.directory_synchronized_at_ns,
            },
            descendant_count: partition.committed_descendants,
        },
        component_group: None,
    };
    let authority_hash = ComponentRuntimeOps::directory_authority_hash(&directory_authority)
        .expect("draining Directory authority hash");
    let convergence = ComponentRuntimeDirectoryConvergenceEvidence {
        operation_id: [82; 32],
        binding: ManagedCanisterBinding::Component(partition.binding.clone()),
        covered_authority: directory_authority,
        covered_authority_hash: authority_hash,
        activation: ComponentRuntimeActivationEvidence {
            directory_authority_hash: [83; 32],
            activated_at_ns: 84,
        },
    };
    ComponentRegistryOps::prepare_component_quiescence(
        fixture.component,
        [79; 32],
        draining.registry.clone(),
        convergence.clone(),
        [85; 32],
        110,
        partition.encoded_bytes,
    )
    .expect_err("terminal quiescence receipt must fit before the stop intent is committed");
    assert_eq!(RootComponentRegistryStore::export(), before_growth);

    let prepared = ComponentRegistryOps::prepare_component_quiescence(
        fixture.component,
        [79; 32],
        draining.registry.clone(),
        convergence,
        [85; 32],
        110,
        16_777_216,
    )
    .expect("durably prepare qualified Component stop");
    assert!(matches!(
        &prepared.quiescence,
        Some(RootComponentQuiescenceProgressView::StopIntent(intent))
            if intent.registry == draining.registry
                && intent.canister_id == fixture.partition.binding.canister_id
                && intent.controller == fixture.partition.binding.fleet_subnet_root
                && intent.expected_module_hash == [85; 32]
                && intent.covered_authority_hash == authority_hash
    ));
    let prepared_state = restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::partition(fixture.component)
            .expect("partition read")
            .expect("draining partition")
            .encoded_bytes,
        exact_component_registry_entry_bytes(&prepared_state, fixture.component)
    );
    let mut corrupted_reservation = prepared_state.clone();
    let Some(RootComponentQuiescenceProgressRecord::StopIntent(intent)) =
        &mut corrupted_reservation.component_drainings[0].quiescence
    else {
        panic!("durable Component stop intent");
    };
    intent.charged_entry_bytes += 1;
    RootComponentRegistryStore::import(corrupted_reservation);
    ComponentRegistryOps::component_draining(fixture.component)
        .expect_err("quiescence terminal byte reservation must remain canonical");
    RootComponentRegistryStore::import(prepared_state);
    let before_observation_rejection = RootComponentRegistryStore::export();
    ComponentRegistryOps::mark_component_quiescent(fixture.component, [79; 32], [86; 32], 110)
        .expect_err("observed module must match the durable stop intent");
    assert_eq!(
        RootComponentRegistryStore::export(),
        before_observation_rejection
    );
    let quiescent =
        ComponentRegistryOps::mark_component_quiescent(fixture.component, [79; 32], [85; 32], 110)
            .expect("commit independently observed Component quiescence");
    assert!(matches!(
        &quiescent.quiescence,
        Some(RootComponentQuiescenceProgressView::Quiescent(receipt))
            if receipt.stop.canister_id == fixture.partition.binding.canister_id
                && receipt.observed_module_hash == [85; 32]
                && receipt.quiesced_at_ns == 110
    ));
    let quiescent_state = restart_component_registry();
    assert_eq!(
        ComponentRegistryOps::mark_component_quiescent(fixture.component, [79; 32], [85; 32], 111,)
            .expect("terminal quiescence retry"),
        quiescent
    );
    assert_eq!(RootComponentRegistryStore::export(), quiescent_state);

    ComponentRegistryOps::begin_subtree_removal(
        fixture.component,
        [81; 32],
        fixture.target.canister_id,
        draining.registry.clone(),
        16_777_216,
    )
    .expect_err("caller-selected subtree removal must remain Active-only");
    assert_eq!(RootComponentRegistryStore::export(), quiescent_state);

    let pending =
        ComponentRegistryOps::advance_component_draining(fixture.component, draining.operation_id)
            .expect("derive the first draining subtree");
    let pending_state = restart_component_registry();
    let restarted =
        ComponentRegistryOps::advance_component_draining(fixture.component, draining.operation_id)
            .expect("derive the same draining subtree after restart");
    assert_eq!(RootComponentRegistryStore::export(), pending_state);
    let (
        RootComponentDrainingAdvanceView::DescendantSubtreePending {
            operation_id,
            target_canister_id,
            reserved_against_registry,
        },
        RootComponentDrainingAdvanceView::DescendantSubtreePending {
            operation_id: restarted_operation_id,
            target_canister_id: restarted_target,
            reserved_against_registry: restarted_registry,
        },
    ) = (pending, restarted)
    else {
        panic!("draining driver must select an unfenced direct subtree");
    };
    assert_eq!(target_canister_id, fixture.target.canister_id);
    assert_eq!(restarted_operation_id, operation_id);
    assert_eq!(restarted_target, target_canister_id);
    assert_eq!(restarted_registry, reserved_against_registry);

    let fenced = ComponentRegistryOps::begin_draining_subtree_removal(
        fixture.component,
        draining.operation_id,
        16_777_216,
    )
    .expect("driver fences its deterministic direct subtree");
    assert_eq!(fenced.operation_id, operation_id);
    assert_eq!(fenced.target_canister_id, target_canister_id);
    assert!(matches!(
        ComponentRegistryOps::advance_component_draining(
            fixture.component,
            draining.operation_id,
        )
        .expect("read the durable draining cursor"),
        RootComponentDrainingAdvanceView::DescendantRemoval(removal)
            if *removal == fenced
    ));
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one terminal-path test retains the complete monotonic removal setup and assertions"
)]
fn subtree_removal_target_finalization_is_terminal_and_releases_the_live_fence() {
    let fixture = import_active_component_tree();
    let operation_id = [91; 32];
    let initial = RootComponentRegistryStore::export();
    let draining = ComponentRegistryOps::begin_component_draining(
        fixture.component,
        [90; 32],
        component_registry_head(&fixture.partition),
        100,
        16_777_216,
        fleet_directory(&initial.current.as_ref().expect("Registry meta").root),
    )
    .expect("drain Component before terminal target removal");
    let partition = ComponentRegistryOps::partition(fixture.component)
        .expect("partition read")
        .expect("draining partition");
    let directory_authority = ComponentRuntimeDirectoryAuthority {
        fleet: fleet_directory(&initial.current.as_ref().expect("Registry meta").root),
        component: ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: partition.binding.clone(),
                source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                component_registry_revision: partition.revision,
                component_registry_content_hash: partition.content_hash,
                synchronized_at_ns: partition.directory_synchronized_at_ns,
            },
            descendant_count: partition.committed_descendants,
        },
        component_group: None,
    };
    let directory_authority_hash =
        ComponentRuntimeOps::directory_authority_hash(&directory_authority)
            .expect("draining Directory authority");
    ComponentRegistryOps::prepare_component_quiescence(
        fixture.component,
        [90; 32],
        draining.registry.clone(),
        ComponentRuntimeDirectoryConvergenceEvidence {
            operation_id: [89; 32],
            binding: ManagedCanisterBinding::Component(partition.binding),
            covered_authority: directory_authority,
            covered_authority_hash: directory_authority_hash,
            activation: ComponentRuntimeActivationEvidence {
                directory_authority_hash: [88; 32],
                activated_at_ns: 87,
            },
        },
        [86; 32],
        101,
        16_777_216,
    )
    .expect("prepare Component quiescence");
    ComponentRegistryOps::mark_component_quiescent(fixture.component, [90; 32], [86; 32], 101)
        .expect("observe Component quiescent");
    let registry = draining.registry;
    ComponentRegistryOps::begin_subtree_removal_with_origin(
        fixture.component,
        operation_id,
        fixture.unrelated.canister_id,
        registry,
        16_777_216,
        SubtreeRemovalOrigin::DrainingDriver,
    )
    .expect("fence a direct leaf through the draining removal primitive");
    let selected = ComponentRegistryOps::advance_subtree_removal(
        fixture.component,
        operation_id,
        0,
        16_777_216,
    )
    .expect("select direct leaf target");
    assert!(matches!(
        &selected.progress,
        RootComponentSubtreeRemovalProgressView::LeafSelected { leaf }
            if leaf.canister_id == fixture.unrelated.canister_id
    ));

    ComponentRegistryOps::prepare_subtree_leaf_stop(
        fixture.component,
        operation_id,
        selected.traversal_steps,
        fixture.unrelated.canister_id,
        fixture.unrelated.parent_canister_id,
        16_777_216,
    )
    .expect("prepare target stop");
    ComponentRegistryOps::mark_subtree_leaf_stopped(
        fixture.component,
        operation_id,
        selected.traversal_steps,
        fixture.unrelated.canister_id,
        fixture.unrelated.parent_canister_id,
        [92; 32],
        16_777_216,
    )
    .expect("observe target stopped");
    ComponentRegistryOps::prepare_subtree_leaf_delete(
        fixture.component,
        operation_id,
        selected.traversal_steps,
        fixture.unrelated.canister_id,
        fixture.unrelated.parent_canister_id,
        16_777_216,
    )
    .expect("prepare target deletion");
    ComponentRegistryOps::mark_subtree_leaf_deleted(
        fixture.component,
        operation_id,
        selected.traversal_steps,
        fixture.unrelated.canister_id,
        fixture.unrelated.parent_canister_id,
        16_777_216,
    )
    .expect("observe target deleted");
    let before_membership = ComponentRegistryOps::partition(fixture.component)
        .expect("partition read")
        .expect("active partition");
    let active_fleet_directory = fleet_directory(&root_binding());
    ComponentRegistryOps::remove_subtree_leaf_membership(
        fixture.component,
        operation_id,
        selected.traversal_steps,
        fixture.unrelated.canister_id,
        fixture.unrelated.parent_canister_id,
        before_membership.directory_synchronized_at_ns + 1,
        16_777_216,
        active_fleet_directory.clone(),
    )
    .expect("remove target membership");
    assert!(matches!(
        ComponentRegistryOps::advance_component_draining(fixture.component, [90; 32])
            .expect("retain the removal cursor after target membership removal"),
        RootComponentDrainingAdvanceView::DescendantRemoval(removal)
            if removal.operation_id == operation_id
                && matches!(
                    &removal.progress,
                    RootComponentSubtreeRemovalProgressView::MembershipRemoved(_)
                )
    ));

    let partition = ComponentRegistryOps::partition(fixture.component)
        .expect("partition read")
        .expect("active partition");
    let directory_authority = ComponentRuntimeDirectoryAuthority {
        fleet: active_fleet_directory,
        component: ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: partition.binding.clone(),
                source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                component_registry_revision: partition.revision,
                component_registry_content_hash: partition.content_hash,
                synchronized_at_ns: partition.directory_synchronized_at_ns,
            },
            descendant_count: partition.committed_descendants,
        },
        component_group: None,
    };
    let directory_authority_hash =
        ComponentRuntimeOps::directory_authority_hash(&directory_authority)
            .expect("target Directory authority hash");
    let synchronized = ComponentRegistryOps::mark_subtree_leaf_directory_synchronized(
        fixture.component,
        operation_id,
        selected.traversal_steps,
        fixture.unrelated.canister_id,
        fixture.unrelated.parent_canister_id,
        directory_authority,
        directory_authority_hash,
        None,
        None,
        16_777_216,
    )
    .expect("retain local Directory authority without calling the quiescent owner");
    assert!(matches!(
        synchronized.progress,
        RootComponentSubtreeRemovalProgressView::DirectorySynchronized(receipt)
            if receipt.owning_component.is_none() && receipt.parent.is_none()
    ));

    let completed = ComponentRegistryOps::finalize_subtree_leaf(
        fixture.component,
        operation_id,
        selected.traversal_steps,
        fixture.unrelated.canister_id,
        fixture.unrelated.parent_canister_id,
        16_777_216,
    )
    .expect("finalize fenced target");
    assert_eq!(completed.completed_leaves, 1);
    assert!(matches!(
        &completed.progress,
        RootComponentSubtreeRemovalProgressView::Completed(receipt)
            if receipt.registry.component == fixture.component
                && receipt.directory_authority_hash == directory_authority_hash
    ));
    let completed_state = restart_component_registry();
    assert_eq!(completed_state.subtree_removal_history.len(), 1);
    assert_eq!(
        ComponentRegistryOps::finalize_subtree_leaf(
            fixture.component,
            operation_id,
            selected.traversal_steps,
            fixture.unrelated.canister_id,
            fixture.unrelated.parent_canister_id,
            16_777_216,
        )
        .expect("exact terminal finalization retry"),
        completed
    );
    assert_eq!(RootComponentRegistryStore::export(), completed_state);
    assert_eq!(
        ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes,
        exact_registry_entry_bytes(&completed_state)
    );

    let partition = ComponentRegistryOps::partition(fixture.component)
        .expect("partition read")
        .expect("active partition");
    let RootComponentDrainingAdvanceView::DescendantSubtreePending {
        operation_id: next_operation_id,
        target_canister_id,
        reserved_against_registry,
    } = ComponentRegistryOps::advance_component_draining(fixture.component, [90; 32])
        .expect("advance beyond the completed removal cursor")
    else {
        panic!("completed draining subtree must release the driver cursor");
    };
    assert_eq!(target_canister_id, fixture.target.canister_id);
    assert_eq!(
        reserved_against_registry,
        ComponentRegistryHead {
            component: fixture.component,
            revision: partition.revision,
            content_hash: partition.content_hash,
        }
    );
    assert_eq!(
        ComponentRegistryOps::begin_draining_subtree_removal(
            fixture.component,
            [90; 32],
            16_777_216,
        )
        .expect("fence the next deterministic draining subtree")
        .operation_id,
        next_operation_id
    );
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one test proves independent partition progress and exact shared root capacity"
)]
fn incomplete_component_operation_does_not_block_an_unrelated_partition() {
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
    let root = root_binding();
    let release_set = FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([8; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
    };
    let component_a = ComponentInstanceId::from_generated_bytes([10; 32]);
    let component_b = ComponentInstanceId::from_generated_bytes([11; 32]);
    let parent_a = candid::Principal::from_slice(&[18; 29]);
    let parent_b = candid::Principal::from_slice(&[19; 29]);
    let partition_a = active_component_partition(&root, release_set, component_a, parent_a);
    let partition_b = active_component_partition(&root, release_set, component_b, parent_b);
    let initial_encoded_bytes = partition_a
        .encoded_bytes
        .checked_add(partition_b.encoded_bytes)
        .expect("initial Registry bytes");
    RootComponentRegistryStore::import(RootComponentRegistryData {
        current: Some(RootComponentRegistryMetaRecord {
            root: root.clone(),
            prepared_against_registry: FleetRegistryVersion {
                authority: root.authority.clone(),
                revision: 4,
                content_hash: [5; 32],
            },
            release_set,
            store_bootstrap: RootStoreBootstrapRequest {
                operation_id: [8; 32],
                manifest_payload_size_bytes: 128,
            },
            next_allocation_sequence: 3,
            reserved_component_instances: 0,
            committed_component_instances: 2,
            managed_descendants: 0,
            known_created_component_canisters: 2,
            encoded_bytes: initial_encoded_bytes,
            initial_inventory: None,
            root_draining: None,
        }),
        partitions: vec![partition_a.clone(), partition_b.clone()],
        ..RootComponentRegistryData::default()
    });

    let operation_a = [44; 32];
    let decision_a = child_allocation_decision(&partition_a, "project_instance");
    let registry_a = component_registry_head(&partition_a);
    ComponentRegistryOps::reserve_child_allocation(
        decision_a.clone(),
        operation_a,
        None,
        registry_a.clone(),
    )
    .expect("reserve Component A child");
    let incomplete_a = ComponentRegistryOps::begin_child_creation(
        component_a,
        operation_a,
        child_creation_plan(&root, 50),
        ReplayCostGuardSettlement {
            quota_intent_id: IntentId(51),
            reservation_intent_id: IntentId(52),
        },
    )
    .expect("record Component A creation intent");
    let partition_a_after_intent = ComponentRegistryOps::partition(component_a)
        .expect("Component A partition read")
        .expect("Component A partition");
    let before_failed_a = RootComponentRegistryStore::export();
    ComponentRegistryOps::mark_child_created(component_a, operation_a, parent_a)
        .expect_err("Component A cannot create over its registered parent");
    assert_eq!(RootComponentRegistryStore::export(), before_failed_a);

    let operation_b = [54; 32];
    let decision_b = child_allocation_decision(&partition_b, "project_instance");
    let registry_b = component_registry_head(&partition_b);
    ComponentRegistryOps::reserve_child_allocation(decision_b, operation_b, None, registry_b)
        .expect("reserve unrelated Component B child");
    ComponentRegistryOps::begin_child_creation(
        component_b,
        operation_b,
        child_creation_plan(&root, 55),
        ReplayCostGuardSettlement {
            quota_intent_id: IntentId(56),
            reservation_intent_id: IntentId(57),
        },
    )
    .expect("record Component B creation intent");
    let child_b = candid::Principal::from_slice(&[58; 29]);
    let progressed_b = ComponentRegistryOps::mark_child_created(component_b, operation_b, child_b)
        .expect("Component B progresses independently");
    assert!(matches!(
        progressed_b.progress,
        RootComponentChildAllocationProgressView::Created { canister, .. }
            if canister == child_b
    ));

    let durable = restart_component_registry();
    let retried_a =
        ComponentRegistryOps::reserve_child_allocation(decision_a, operation_a, None, registry_a)
            .expect("retry preserves incomplete Component A intent");
    assert_eq!(retried_a, incomplete_a);
    assert_eq!(
        ComponentRegistryOps::partition(component_a)
            .expect("Component A partition read")
            .expect("Component A partition"),
        partition_a_after_intent
    );
    let current = ComponentRegistryOps::current().expect("Registry status");
    assert_eq!(current.managed_descendants, 2);
    assert_eq!(current.known_created_component_canisters, 3);
    assert!(
        exact_registry_entry_bytes(&durable) <= current.encoded_bytes,
        "persisted entries must fit inside their exact pre-effect charges"
    );
    assert_eq!(
        current.encoded_bytes,
        durable
            .partitions
            .iter()
            .map(|partition| partition.encoded_bytes)
            .sum::<u64>()
    );
    for component in [component_a, component_b] {
        let partition = ComponentRegistryOps::partition(component)
            .expect("partition read")
            .expect("active partition");
        assert_eq!(
            partition.encoded_bytes,
            exact_component_registry_entry_bytes(&durable, component)
        );
    }

    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one test follows the complete direct-child reserve-through-commit lifecycle"
)]
fn child_reservation_is_parent_indexed_idempotent_and_capacity_bounded() {
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
    let root = root_binding();
    let root_canister = root.fleet_subnet_root;
    let component = ComponentInstanceId::from_generated_bytes([10; 32]);
    let parent = candid::Principal::from_slice(&[18; 29]);
    let release_set = FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([8; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
    };
    let binding = ComponentBinding {
        authority: root.authority.clone(),
        component,
        component_spec: "projects".parse().expect("Component Spec"),
        spec_hash: [6; 32],
        role: CanisterRole::new("project_hub"),
        placement_subnet: root.placement_subnet,
        fleet_subnet_root: root.fleet_subnet_root,
        canister_id: parent,
    };
    let mut partition = ComponentRegistryPartitionRecord {
        binding: binding.clone(),
        protocol_profile_digest: ProtocolProfileDigest::from_bytes([12; 32]),
        provisioning_origin: ComponentProvisioningOrigin::FleetAdministrator {
            caller: candid::Principal::from_slice(&[11; 29]),
        },
        release_set,
        status: ComponentLifecycleStatus::Active,
        revision: 2,
        content_hash: component_partition_content_hash(
            &binding,
            ProtocolProfileDigest::from_bytes([12; 32]),
            &ComponentProvisioningOrigin::FleetAdministrator {
                caller: candid::Principal::from_slice(&[11; 29]),
            },
            release_set,
            ComponentLifecycleStatus::Active,
            2,
            empty_component_descendant_content_hash(component),
            0,
        )
        .expect("partition hash"),
        descendant_content_hash: empty_component_descendant_content_hash(component),
        directory_synchronized_at_ns: 33,
        reserved_descendants: 0,
        committed_descendants: 0,
        encoded_bytes: 0,
    };
    let component_principal_index_bytes =
        RootComponentRegistryStore::principal_index_entry_bytes(parent, component);
    for _ in 0..8 {
        let encoded_bytes = RootComponentRegistryStore::partition_entry_bytes(&partition)
            + component_principal_index_bytes;
        if partition.encoded_bytes == encoded_bytes {
            break;
        }
        partition.encoded_bytes = encoded_bytes;
    }
    assert_eq!(
        partition.encoded_bytes,
        RootComponentRegistryStore::partition_entry_bytes(&partition)
            + component_principal_index_bytes
    );
    let initial_encoded_bytes = partition.encoded_bytes;
    RootComponentRegistryStore::import(RootComponentRegistryData {
        current: Some(RootComponentRegistryMetaRecord {
            root: root.clone(),
            prepared_against_registry: FleetRegistryVersion {
                authority: root.authority.clone(),
                revision: 4,
                content_hash: [5; 32],
            },
            release_set,
            store_bootstrap: RootStoreBootstrapRequest {
                operation_id: [8; 32],
                manifest_payload_size_bytes: 128,
            },
            next_allocation_sequence: 2,
            reserved_component_instances: 0,
            committed_component_instances: 1,
            managed_descendants: 0,
            known_created_component_canisters: 1,
            encoded_bytes: partition.encoded_bytes,
            initial_inventory: None,
            root_draining: None,
        }),
        partitions: vec![partition.clone()],
        ..RootComponentRegistryData::default()
    });
    let decision = ComponentChildAllocationDecision {
        component,
        component_spec: binding.component_spec.clone(),
        spec_hash: binding.spec_hash,
        parent_canister_id: parent,
        parent_role: binding.role.clone(),
        child_role: CanisterRole::new("project_instance"),
        child_kind: ComponentChildKind::Instance,
        mode: canic_core::control_plane_support::policy::component_child_allocation::ComponentChildAllocationMode::Active,
        maximum_instances_per_parent: 10_000,
        maximum_descendants: 20_000,
        maximum_registry_bytes: 16_777_216,
    };
    let registry = ComponentRegistryHead {
        component,
        revision: partition.revision,
        content_hash: partition.content_hash,
    };

    let application_init_args = Some(vec![9, 8, 7]);
    let reserved = ComponentRegistryOps::reserve_child_allocation(
        decision.clone(),
        [44; 32],
        application_init_args.clone(),
        registry.clone(),
    )
    .expect("reserve child");
    let interrupted = RootComponentRegistryStore::export();
    RootComponentRegistryStore::import(interrupted);
    let repeated = ComponentRegistryOps::reserve_child_allocation(
        decision.clone(),
        [44; 32],
        application_init_args,
        registry.clone(),
    )
    .expect("retry child reservation");
    assert_eq!(
        RootComponentRegistryStore::registry_components(),
        vec![component],
        "one logical Component must be enumerated once regardless of its retained child rows"
    );
    assert_eq!(
        ComponentRegistryOps::child_allocation_by_operation([44; 32])
            .expect("lookup child by exact operation")
            .expect("retained child allocation"),
        repeated
    );
    let mut retained_reservation = RootComponentRegistryStore::export()
        .child_allocations
        .into_iter()
        .find(|allocation| allocation.operation_id == [44; 32])
        .expect("retained child reservation");
    assert_eq!(
        ComponentPartitionLifecycleAuthority::from_reservation(&retained_reservation).status,
        ComponentLifecycleStatus::Active
    );
    retained_reservation.initial_bootstrap = true;
    assert_eq!(
        ComponentPartitionLifecycleAuthority::from_reservation(&retained_reservation).status,
        ComponentLifecycleStatus::Prepared
    );

    assert_eq!(reserved, repeated);
    ComponentRegistryOps::reserve_child_allocation(
        decision.clone(),
        [44; 32],
        Some(vec![9, 8, 6]),
        registry,
    )
    .expect_err("retry cannot change application init arguments");
    assert_eq!(
        ComponentRegistryOps::registered_parent(component, parent)
            .expect("registered parent")
            .expect("top-level parent")
            .0,
        ManagedCanisterBinding::Component(binding.clone())
    );
    assert_eq!(
        ComponentRegistryOps::parent_role_instances(component, parent, &decision.child_role,)
            .expect("parent-role count"),
        1
    );
    let partition = ComponentRegistryOps::partition(component)
        .expect("partition read")
        .expect("partition");
    assert_eq!(partition.reserved_descendants, 1);
    assert_eq!(partition.committed_descendants, 0);
    let current = ComponentRegistryOps::current().expect("Registry status");
    assert_eq!(current.managed_descendants, 1);
    assert_eq!(current.encoded_bytes, partition.encoded_bytes);
    assert!(partition.encoded_bytes > initial_encoded_bytes);

    let mut exhausted = decision.clone();
    exhausted.maximum_instances_per_parent = 1;
    let before = RootComponentRegistryStore::export();
    let error = ComponentRegistryOps::reserve_child_allocation(
        exhausted,
        [45; 32],
        None,
        repeated.reserved_against_registry.clone(),
    )
    .expect_err("per-parent capacity must reject reservation");
    assert!(error.is_public_resource_exhausted());
    assert_eq!(RootComponentRegistryStore::export(), before);

    let mut conflicting = decision.clone();
    conflicting.maximum_descendants -= 1;
    assert!(
        ComponentRegistryOps::reserve_child_allocation(
            conflicting,
            [44; 32],
            Some(vec![9, 8, 7]),
            repeated.reserved_against_registry.clone(),
        )
        .is_err()
    );

    let plan = RootComponentCreationPlan {
        wasm_store: candid::Principal::from_slice(&[50; 29]),
        payload_hash: [51; 32],
        payload_size_bytes: 4_096,
        initial_cycles: Cycles::new(5_000_000_000_000),
        controller: root_canister,
    };
    let before_creation = RootComponentRegistryStore::export();
    let mut capacity_exhausted = before_creation.clone();
    let maximum_registry_bytes = capacity_exhausted
        .current
        .as_ref()
        .expect("Registry status")
        .encoded_bytes;
    capacity_exhausted
        .current
        .as_mut()
        .expect("Registry status")
        .root
        .limits
        .maximum_registry_bytes = maximum_registry_bytes;
    RootComponentRegistryStore::import(capacity_exhausted);
    let error = ComponentRegistryOps::validate_child_creation_capacity(component, [44; 32], &plan)
        .expect_err("creation must fit before the paid effect");
    assert!(error.is_public_resource_exhausted());
    assert!(matches!(
        ComponentRegistryOps::child_allocation(component, [44; 32])
            .expect("child allocation")
            .expect("reserved child")
            .progress,
        RootComponentChildAllocationProgressView::Reserved
    ));
    RootComponentRegistryStore::import(before_creation);

    ComponentRegistryOps::validate_child_creation_capacity(component, [44; 32], &plan)
        .expect("child creation capacity");
    let intent = ComponentRegistryOps::begin_child_creation(
        component,
        [44; 32],
        plan,
        ReplayCostGuardSettlement {
            quota_intent_id: IntentId(52),
            reservation_intent_id: IntentId(53),
        },
    )
    .expect("child creation intent");
    let intent_bytes = ComponentRegistryOps::current()
        .expect("Registry status")
        .encoded_bytes;
    assert!(intent_bytes > current.encoded_bytes);
    assert!(matches!(
        intent.progress,
        RootComponentChildAllocationProgressView::CreationIntent(_)
    ));

    restart_component_registry();
    let canister = candid::Principal::from_slice(&[54; 29]);
    let created = ComponentRegistryOps::mark_child_created(component, [44; 32], canister)
        .expect("record created child");
    restart_component_registry();
    let repeated_created = ComponentRegistryOps::mark_child_created(component, [44; 32], canister)
        .expect("exact created child retry");

    assert_eq!(created, repeated_created);
    assert!(matches!(
        created.progress,
        RootComponentChildAllocationProgressView::Created {
            canister: created_canister,
            ..
        } if created_canister == canister
    ));
    let created_status = ComponentRegistryOps::current().expect("Registry status");
    assert_eq!(created_status.known_created_component_canisters, 2);
    assert_eq!(created_status.managed_descendants, 1);
    assert_eq!(created_status.encoded_bytes, intent_bytes);
    assert_eq!(
        ComponentRegistryOps::partition(component)
            .expect("partition read")
            .expect("partition")
            .reserved_descendants,
        1
    );
    assert!(
        ComponentRegistryOps::mark_child_created(
            component,
            [44; 32],
            candid::Principal::from_slice(&[55; 29]),
        )
        .is_err()
    );
    let install_plan = RootComponentChildInstallPlan {
        raw_module_hash: [56; 32],
        protocol_profile_digest: ProtocolProfileDigest::from_bytes([59; 32]),
        chunk_hashes: vec![vec![57; 32], vec![58; 32]],
        binding: ComponentChildBinding {
            component: binding,
            parent_canister_id: parent,
            role: decision.child_role.clone(),
            canister_id: canister,
        },
        maximum_registry_bytes: decision.maximum_registry_bytes,
    };
    let before_install = RootComponentRegistryStore::export();
    let mut install_capacity_exhausted = before_install.clone();
    let maximum_registry_bytes = install_capacity_exhausted
        .current
        .as_ref()
        .expect("Registry status")
        .encoded_bytes;
    install_capacity_exhausted
        .current
        .as_mut()
        .expect("Registry status")
        .root
        .limits
        .maximum_registry_bytes = maximum_registry_bytes;
    RootComponentRegistryStore::import(install_capacity_exhausted);
    let error =
        ComponentRegistryOps::validate_child_install_capacity(component, [44; 32], &install_plan)
            .expect_err("installation must fit before the paid effect");
    assert!(error.is_public_resource_exhausted());
    assert!(matches!(
        ComponentRegistryOps::child_allocation(component, [44; 32])
            .expect("child allocation")
            .expect("created child")
            .progress,
        RootComponentChildAllocationProgressView::Created { .. }
    ));
    RootComponentRegistryStore::import(before_install);

    ComponentRegistryOps::validate_child_install_capacity(component, [44; 32], &install_plan)
        .expect("child install capacity");
    let install_intent = ComponentRegistryOps::begin_child_install(
        component,
        [44; 32],
        install_plan.clone(),
        ReplayCostGuardSettlement {
            quota_intent_id: IntentId(59),
            reservation_intent_id: IntentId(60),
        },
    )
    .expect("child install intent");
    let install_intent_bytes = ComponentRegistryOps::current()
        .expect("Registry status")
        .encoded_bytes;
    assert!(install_intent_bytes > intent_bytes);
    assert!(matches!(
        install_intent.progress,
        RootComponentChildAllocationProgressView::InstallIntent { .. }
    ));

    let mut conflicting_install = install_plan.clone();
    conflicting_install.raw_module_hash = [61; 32];
    assert!(
        ComponentRegistryOps::renew_child_install_intent(
            component,
            [44; 32],
            &conflicting_install,
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(62),
                reservation_intent_id: IntentId(63),
            },
        )
        .is_err()
    );
    let mut conflicting_profile = install_plan.clone();
    conflicting_profile.protocol_profile_digest = ProtocolProfileDigest::from_bytes([62; 32]);
    assert!(
        ComponentRegistryOps::renew_child_install_intent(
            component,
            [44; 32],
            &conflicting_profile,
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(62),
                reservation_intent_id: IntentId(63),
            },
        )
        .is_err()
    );
    restart_component_registry();
    let renewed = ComponentRegistryOps::renew_child_install_intent(
        component,
        [44; 32],
        &install_plan,
        ReplayCostGuardSettlement {
            quota_intent_id: IntentId(64),
            reservation_intent_id: IntentId(65),
        },
    )
    .expect("renew exact child install intent");
    let RootComponentChildAllocationProgressView::InstallIntent { installation, .. } =
        &renewed.progress
    else {
        panic!("renewed child install intent");
    };
    assert_eq!(installation.binding, install_plan.binding);
    assert_eq!(
        installation.protocol_profile_digest,
        install_plan.protocol_profile_digest
    );
    assert_eq!(
        installation.cost_guard_settlement.quota_intent_id,
        IntentId(64)
    );

    let installed = ComponentRegistryOps::mark_child_installed(component, [44; 32])
        .expect("mark child installed");
    restart_component_registry();
    let installed_retry = ComponentRegistryOps::mark_child_installed(component, [44; 32])
        .expect("installed child retry");
    assert_eq!(installed, installed_retry);
    assert!(matches!(
        installed.progress,
        RootComponentChildAllocationProgressView::Installed { .. }
    ));
    let verified = ComponentRegistryOps::mark_child_verified(component, [44; 32])
        .expect("mark child verified");
    restart_component_registry();
    let verified_retry = ComponentRegistryOps::mark_child_verified(component, [44; 32])
        .expect("verified child retry");
    assert_eq!(verified, verified_retry);
    assert!(matches!(
        verified.progress,
        RootComponentChildAllocationProgressView::Verified { .. }
    ));
    let verified_status = ComponentRegistryOps::current().expect("Registry status");
    assert_eq!(verified_status.known_created_component_canisters, 2);
    assert_eq!(verified_status.managed_descendants, 1);
    assert_eq!(verified_status.encoded_bytes, install_intent_bytes);
    let verified_partition = ComponentRegistryOps::partition(component)
        .expect("partition read")
        .expect("partition");
    assert_eq!(verified_partition.reserved_descendants, 1);
    assert_eq!(verified_partition.committed_descendants, 0);
    assert_eq!(verified_partition.encoded_bytes, install_intent_bytes);

    let committed = ComponentRegistryOps::commit_verified_child(
        component,
        [44; 32],
        66,
        fleet_directory(&root),
        None,
    )
    .expect("commit verified child");
    let committed_partition = ComponentRegistryOps::partition(component)
        .expect("partition read")
        .expect("partition");
    assert_eq!(committed_partition.revision, 3);
    assert_ne!(
        committed_partition.content_hash,
        verified_partition.content_hash
    );
    assert_eq!(committed_partition.directory_synchronized_at_ns, 66);
    assert_eq!(committed_partition.reserved_descendants, 0);
    assert_eq!(committed_partition.committed_descendants, 1);
    assert!(committed_partition.encoded_bytes <= install_intent_bytes);
    let RootComponentChildAllocationProgressView::Committed {
        commitment,
        installation,
        ..
    } = &committed.0.progress
    else {
        panic!("committed child progress");
    };
    assert_eq!(
        commitment.registry,
        ComponentRegistryHead {
            component,
            revision: committed_partition.revision,
            content_hash: committed_partition.content_hash,
        }
    );
    assert_eq!(
        commitment.registry_encoded_bytes,
        committed_partition.encoded_bytes
    );
    assert_eq!(commitment.reserved_descendants, 0);
    assert_eq!(commitment.committed_descendants, 1);
    assert_ne!(commitment.directory_authority_hash, [0; 32]);
    let child_directory_authority_hash = commitment.directory_authority_hash;
    assert_eq!(installation.binding, install_plan.binding);
    assert_eq!(committed.1, committed_partition);
    let committed_status = ComponentRegistryOps::current().expect("Registry status");
    assert_eq!(committed_status.managed_descendants, 1);
    assert_eq!(committed_status.known_created_component_canisters, 2);
    assert_eq!(
        committed_status.encoded_bytes,
        committed_partition.encoded_bytes
    );
    assert_eq!(
        ComponentRegistryOps::parent_role_instances(component, parent, &decision.child_role,)
            .expect("parent-role count"),
        1
    );
    let registered_child = ComponentRegistryOps::registered_parent(component, canister)
        .expect("registered child")
        .expect("normalized child");
    assert_eq!(
        registered_child,
        (
            ManagedCanisterBinding::ComponentChild(install_plan.binding),
            ComponentLifecycleStatus::Prepared,
        )
    );
    let durable = restart_component_registry();
    assert_eq!(durable.children.len(), 1);
    assert_eq!(durable.child_traversals.len(), 1);
    let progressed_partition = ComponentRegistryOps::partition(component)
        .expect("partition read")
        .expect("partition");
    let progressed_reservation = ComponentRegistryOps::reserve_child_allocation(
        decision.clone(),
        [68; 32],
        None,
        ComponentRegistryHead {
            component,
            revision: progressed_partition.revision,
            content_hash: progressed_partition.content_hash,
        },
    )
    .expect("later child reservation");
    assert!(matches!(
        progressed_reservation.progress,
        RootComponentChildAllocationProgressView::Reserved
    ));
    let committed_retry = ComponentRegistryOps::commit_verified_child(
        component,
        [44; 32],
        67,
        fleet_directory(&root),
        None,
    )
    .expect("exact child commit retry");
    assert_eq!(committed_retry, committed);

    let retried_reservation = ComponentRegistryOps::reserve_child_allocation(
        decision.clone(),
        [44; 32],
        Some(vec![9, 8, 7]),
        repeated.reserved_against_registry,
    )
    .expect("reservation retry preserves install progress");
    assert_eq!(retried_reservation, committed.0);
    let before_directory_receipt = ComponentRegistryOps::current().expect("Registry status");
    let before_directory_partition = ComponentRegistryOps::partition(component)
        .expect("partition read")
        .expect("partition");
    assert!(
        ComponentRegistryOps::mark_child_runtime_activated(
            component,
            [44; 32],
            child_directory_authority_hash,
        )
        .is_err()
    );
    let prepared = ComponentRegistryOps::mark_child_directory_prepared(
        component,
        [44; 32],
        child_directory_authority_hash,
    )
    .expect("mark child Directory prepared");
    restart_component_registry();
    let prepared_again = ComponentRegistryOps::mark_child_directory_prepared(
        component,
        [44; 32],
        child_directory_authority_hash,
    )
    .expect("repeat child Directory preparation receipt");
    assert_eq!(prepared_again, prepared);
    assert!(matches!(
        prepared.progress,
        RootComponentChildAllocationProgressView::Committed {
            commitment: RootComponentChildCommitmentView {
                directory_prepared: true,
                runtime_activated: false,
                membership: None,
                ..
            },
            ..
        }
    ));
    assert!(
        ComponentRegistryOps::activate_child_membership(
            component,
            [44; 32],
            69,
            fleet_directory(&root),
            None,
        )
        .is_err()
    );
    let activated = ComponentRegistryOps::mark_child_runtime_activated(
        component,
        [44; 32],
        child_directory_authority_hash,
    )
    .expect("mark child runtime activated");
    restart_component_registry();
    let activated_again = ComponentRegistryOps::mark_child_runtime_activated(
        component,
        [44; 32],
        child_directory_authority_hash,
    )
    .expect("repeat child runtime activation receipt");
    assert_eq!(activated_again, activated);
    assert!(matches!(
        activated.progress,
        RootComponentChildAllocationProgressView::Committed {
            commitment: RootComponentChildCommitmentView {
                directory_prepared: true,
                runtime_activated: true,
                membership: None,
                ..
            },
            ..
        }
    ));
    assert_eq!(
        ComponentRegistryOps::current().expect("Registry status"),
        before_directory_receipt
    );
    assert_eq!(
        ComponentRegistryOps::partition(component)
            .expect("partition read")
            .expect("partition"),
        before_directory_partition
    );
    let membership = ComponentRegistryOps::activate_child_membership(
        component,
        [44; 32],
        69,
        fleet_directory(&root),
        None,
    )
    .expect("activate child membership");
    restart_component_registry();
    let membership_again = ComponentRegistryOps::activate_child_membership(
        component,
        [44; 32],
        70,
        fleet_directory(&root),
        None,
    )
    .expect("repeat child membership activation");
    assert_eq!(membership_again, membership);
    assert_eq!(
        membership.1.revision,
        before_directory_partition.revision + 1
    );
    assert_eq!(membership.1.status, ComponentLifecycleStatus::Active);
    assert_eq!(
        membership.1.reserved_descendants,
        before_directory_partition.reserved_descendants
    );
    assert_eq!(
        membership.1.committed_descendants,
        before_directory_partition.committed_descendants
    );
    assert_ne!(
        membership.1.descendant_content_hash,
        before_directory_partition.descendant_content_hash
    );
    assert_eq!(membership.1.directory_synchronized_at_ns, 69);
    let RootComponentChildAllocationProgressView::Committed {
        commitment:
            RootComponentChildCommitmentView {
                membership: Some(active_membership),
                ..
            },
        ..
    } = &membership.0.progress
    else {
        panic!("active child membership receipt");
    };
    assert_eq!(
        active_membership.registry,
        ComponentRegistryHead {
            component,
            revision: membership.1.revision,
            content_hash: membership.1.content_hash,
        }
    );
    assert_eq!(
        active_membership.descendant_content_hash,
        membership.1.descendant_content_hash
    );
    assert_eq!(
        active_membership.registry_encoded_bytes,
        membership.1.encoded_bytes
    );
    assert!(!active_membership.directory_synchronized);
    assert_eq!(
        ComponentRegistryOps::registered_parent(component, canister)
            .expect("registered active child")
            .expect("active child row")
            .1,
        ComponentLifecycleStatus::Active
    );
    assert!(
        ComponentRegistryOps::mark_child_membership_synchronized(
            component,
            [44; 32],
            [u8::MAX; 32],
        )
        .is_err()
    );
    let terminal = ComponentRegistryOps::mark_child_membership_synchronized(
        component,
        [44; 32],
        active_membership.directory_authority_hash,
    )
    .expect("mark child membership synchronized");
    let terminal_snapshot = restart_component_registry();
    let terminal_again = ComponentRegistryOps::mark_child_membership_synchronized(
        component,
        [44; 32],
        active_membership.directory_authority_hash,
    )
    .expect("repeat child membership synchronization receipt");
    assert_eq!(terminal_again, terminal);
    assert!(matches!(
        terminal.progress,
        RootComponentChildAllocationProgressView::Committed {
            commitment: RootComponentChildCommitmentView {
                membership: Some(RootComponentChildMembershipView {
                    directory_synchronized: true,
                    ..
                }),
                ..
            },
            ..
        }
    ));
    let terminal_partition = ComponentRegistryOps::partition(component)
        .expect("partition read")
        .expect("terminal active partition");
    assert_eq!(terminal_partition, membership.1);
    let exact_terminal_bytes = exact_registry_entry_bytes(&terminal_snapshot);
    assert_eq!(terminal_partition.encoded_bytes, exact_terminal_bytes);
    assert_eq!(
        ComponentRegistryOps::current()
            .expect("terminal Registry status")
            .encoded_bytes,
        exact_terminal_bytes
    );
    let complete_directory = ComponentRegistryOps::directory_page(
        component,
        &ComponentDirectoryPageSelection {
            parent_canister_id: None,
            role: None,
            status: None,
            start_after: None,
        },
        100,
    )
    .expect("complete Component Directory page");
    assert_eq!(complete_directory.entries.len(), 1);
    assert_eq!(
        complete_directory.entries[0].binding.component,
        terminal_partition.binding
    );
    assert_eq!(
        complete_directory.entries[0].binding.parent_canister_id,
        parent
    );
    assert_eq!(
        complete_directory.entries[0].binding.role,
        decision.child_role
    );
    assert_eq!(complete_directory.entries[0].binding.canister_id, canister);
    assert_eq!(
        complete_directory.entries[0].status,
        ComponentLifecycleStatus::Active
    );
    assert_eq!(
        complete_directory.entries[0].protocol_profile_digest,
        install_plan.protocol_profile_digest
    );
    assert!(complete_directory.next_cursor.is_none());
    let direct_active_children = ComponentRegistryOps::directory_page(
        component,
        &ComponentDirectoryPageSelection {
            parent_canister_id: Some(parent),
            role: Some(decision.child_role.clone()),
            status: Some(ComponentLifecycleStatus::Active),
            start_after: None,
        },
        100,
    )
    .expect("filtered direct-child Directory page");
    assert_eq!(direct_active_children.entries, complete_directory.entries);
    let after_only_child = ComponentRegistryOps::directory_page(
        component,
        &ComponentDirectoryPageSelection {
            parent_canister_id: Some(parent),
            role: Some(decision.child_role.clone()),
            status: Some(ComponentLifecycleStatus::Active),
            start_after: Some(ComponentDirectoryCanonicalCursor {
                parent_canister_id: parent,
                role: decision.child_role.clone(),
                canister_id: canister,
            }),
        },
        100,
    )
    .expect("Directory page after only child");
    assert!(after_only_child.entries.is_empty());
    assert!(after_only_child.next_cursor.is_none());
    let prepared_children = ComponentRegistryOps::directory_page(
        component,
        &ComponentDirectoryPageSelection {
            parent_canister_id: Some(parent),
            role: Some(decision.child_role.clone()),
            status: Some(ComponentLifecycleStatus::Prepared),
            start_after: None,
        },
        100,
    )
    .expect("status-filtered Directory page");
    assert!(prepared_children.entries.is_empty());
    let later_reservation = ComponentRegistryOps::reserve_child_allocation(
        decision,
        [71; 32],
        None,
        ComponentRegistryHead {
            component,
            revision: terminal_partition.revision,
            content_hash: terminal_partition.content_hash,
        },
    )
    .expect("reserve later child after membership");
    assert!(matches!(
        later_reservation.progress,
        RootComponentChildAllocationProgressView::Reserved
    ));
    let membership_after_later_reservation = ComponentRegistryOps::activate_child_membership(
        component,
        [44; 32],
        72,
        fleet_directory(&root),
        None,
    )
    .expect("membership retry after later reservation");
    assert_eq!(membership_after_later_reservation.0, terminal);
    assert_eq!(membership_after_later_reservation.1, membership.1);
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one test follows the complete paid creation lifecycle and its exact retry invariants"
)]
fn creation_intent_reserves_terminal_bytes_and_created_retry_preserves_principal() {
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
    let root = root_binding();
    let version = FleetRegistryVersion {
        authority: root.authority.clone(),
        revision: 4,
        content_hash: [5; 32],
    };
    ComponentRegistryOps::prepare(
        root,
        version,
        FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        },
        RootStoreBootstrapRequest {
            operation_id: [8; 32],
            manifest_payload_size_bytes: 128,
        },
    )
    .expect("prepare");
    ComponentRegistryOps::reserve_allocation(
        TopLevelComponentAllocationDecision {
            allocation_sequence: 1,
            component: ComponentInstanceId::from_generated_bytes([10; 32]),
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [6; 32],
            role: CanisterRole::new("project_hub"),
        },
        [12; 32],
        ComponentProvisioningOrigin::FleetAdministrator {
            caller: candid::Principal::from_slice(&[11; 29]),
        },
        false,
    )
    .expect("reserve");
    let reserved_bytes = ComponentRegistryOps::current()
        .expect("Registry status")
        .encoded_bytes;
    let plan = RootComponentCreationPlan {
        wasm_store: candid::Principal::from_slice(&[13; 29]),
        payload_hash: [14; 32],
        payload_size_bytes: 4_096,
        initial_cycles: Cycles::new(5_000_000_000_000),
        controller: candid::Principal::from_slice(&[15; 29]),
    };

    assert_creation_capacity_is_reserved_before_effect(&plan, reserved_bytes);

    ComponentRegistryOps::validate_creation_capacity([12; 32], &plan).expect("creation capacity");
    let intent = ComponentRegistryOps::begin_creation(
        [12; 32],
        plan,
        ReplayCostGuardSettlement {
            quota_intent_id: IntentId(16),
            reservation_intent_id: IntentId(17),
        },
    )
    .expect("creation intent");
    let intent_bytes = ComponentRegistryOps::current()
        .expect("Registry status")
        .encoded_bytes;
    assert!(intent_bytes > reserved_bytes);
    assert!(matches!(
        intent.progress,
        RootComponentAllocationProgressView::CreationIntent(_)
    ));
    assert_eq!(
        ComponentRegistryOps::current()
            .expect("Registry status")
            .known_created_component_canisters,
        0,
        "creation intent without a known principal must not count"
    );

    let interrupted = RootComponentRegistryStore::export();
    RootComponentRegistryStore::import(interrupted);
    let canister = candid::Principal::from_slice(&[18; 29]);
    let created = ComponentRegistryOps::mark_created([12; 32], canister).expect("record created");
    let repeated =
        ComponentRegistryOps::mark_created([12; 32], canister).expect("exact created retry");

    assert_eq!(created, repeated);
    let status = ComponentRegistryOps::current().expect("Registry status");
    assert_eq!(
        status.encoded_bytes, intent_bytes,
        "the intent must reserve terminal record capacity before the effect"
    );
    assert_eq!(
        status.known_created_component_canisters, 1,
        "a known created principal must be counted exactly once"
    );
    assert!(matches!(
        created.progress,
        RootComponentAllocationProgressView::Created {
            canister: created_canister,
            ..
        } if created_canister == canister
    ));
    assert!(
        ComponentRegistryOps::mark_created([12; 32], candid::Principal::from_slice(&[19; 29]),)
            .is_err()
    );
    assert_eq!(
        ComponentRegistryOps::current()
            .expect("Registry status")
            .known_created_component_canisters,
        1,
        "a conflicting retry must not change the count"
    );
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

#[test]
fn install_intent_reserves_terminal_bytes_and_advances_idempotently() {
    let (root, created, canister) = prepared_created_allocation();
    let created_bytes = ComponentRegistryOps::current()
        .expect("Registry status")
        .encoded_bytes;
    let plan = RootComponentInstallPlan {
        raw_module_hash: [20; 32],
        protocol_profile_digest: ProtocolProfileDigest::from_bytes([23; 32]),
        chunk_hashes: vec![vec![21; 32], vec![22; 32]],
        binding: ComponentBinding {
            authority: root.authority.clone(),
            component: created.component,
            component_spec: created.component_spec.clone(),
            spec_hash: created.spec_hash,
            role: created.role,
            placement_subnet: root.placement_subnet,
            fleet_subnet_root: root.fleet_subnet_root,
            canister_id: canister,
        },
        maximum_registry_bytes: 16_777_216,
    };

    let mut component_exhausted = plan.clone();
    component_exhausted.maximum_registry_bytes = 1;
    let capacity_error =
        ComponentRegistryOps::validate_install_capacity([12; 32], &component_exhausted)
            .expect_err("terminal Component partition must fit before installation");
    assert!(capacity_error.is_public_resource_exhausted());
    assert!(matches!(
        ComponentRegistryOps::allocation([12; 32])
            .expect("created allocation")
            .progress,
        RootComponentAllocationProgressView::Created { .. }
    ));

    let intent_bytes = advance_install_to_verified(&plan, created_bytes);

    let directory = fleet_directory(&root);
    let (committed, partition) = ComponentRegistryOps::commit_verified(
        [12; 32],
        31,
        plan.maximum_registry_bytes,
        directory.clone(),
    )
    .expect("commit verified Component");
    let interrupted = RootComponentRegistryStore::export();
    RootComponentRegistryStore::import(interrupted);
    let repeated =
        ComponentRegistryOps::commit_verified([12; 32], 32, plan.maximum_registry_bytes, directory)
            .expect("exact commitment retry");
    assert_eq!(repeated, (committed.clone(), partition.clone()));
    assert!(matches!(
        committed.progress,
        RootComponentAllocationProgressView::Committed { .. }
    ));
    assert_eq!(partition.binding, plan.binding);
    assert_eq!(
        partition.protocol_profile_digest,
        plan.protocol_profile_digest
    );
    assert_eq!(partition.status, ComponentLifecycleStatus::Prepared);
    assert_eq!(partition.revision, 1);
    assert_ne!(partition.content_hash, [0; 32]);
    assert_eq!(partition.directory_synchronized_at_ns, 31);
    assert_eq!(
        ComponentRegistryOps::component_for_principal(canister),
        Some(committed.component)
    );
    assert_eq!(
        ComponentRegistryOps::partition(committed.component)
            .expect("valid partition")
            .expect("committed partition"),
        partition
    );
    let status = ComponentRegistryOps::current().expect("Registry status");
    assert_eq!(status.reserved_component_instances, 0);
    assert_eq!(status.committed_component_instances, 1);
    assert_eq!(status.managed_descendants, 0);
    assert_eq!(status.encoded_bytes, partition.encoded_bytes);
    assert!(status.encoded_bytes <= intent_bytes);
    assert_eq!(
        ComponentRegistryOps::component_spec_counts(&committed.component_spec)
            .expect("Spec counts"),
        ComponentSpecInstanceCounts {
            reserved: 0,
            committed: 1,
        }
    );
    assert_directory_preparation_receipt(
        &committed,
        &partition,
        fleet_directory(&root),
        plan.maximum_registry_bytes,
    );
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

fn assert_directory_preparation_receipt(
    committed: &RootComponentAllocationView,
    prepared_partition: &ComponentRegistryPartitionView,
    directory: FleetDirectorySnapshot,
    maximum_component_registry_bytes: u64,
) {
    let RootComponentAllocationProgressView::Committed { commitment, .. } = &committed.progress
    else {
        panic!("committed allocation progress");
    };
    assert_ne!(commitment.directory_authority_hash, [0; 32]);
    assert_eq!(
        commitment.prepared_registry_encoded_bytes,
        prepared_partition.encoded_bytes
    );
    assert!(!commitment.directory_prepared);
    assert!(!commitment.runtime_activated);
    assert!(
        ComponentRegistryOps::mark_runtime_activated(
            [12; 32],
            commitment.directory_authority_hash,
        )
        .is_err()
    );
    let prepared = ComponentRegistryOps::mark_directory_prepared(
        [12; 32],
        commitment.directory_authority_hash,
    )
    .expect("mark Directory prepared");
    let prepared_again = ComponentRegistryOps::mark_directory_prepared(
        [12; 32],
        commitment.directory_authority_hash,
    )
    .expect("retry Directory receipt");
    assert_eq!(prepared_again, prepared);
    assert!(matches!(
        &prepared.progress,
        RootComponentAllocationProgressView::Committed {
            commitment: RootComponentCommitmentView {
                directory_prepared: true,
                runtime_activated: false,
                ..
            },
            ..
        }
    ));
    let activated =
        ComponentRegistryOps::mark_runtime_activated([12; 32], commitment.directory_authority_hash)
            .expect("mark runtime activated");
    let activated_again =
        ComponentRegistryOps::mark_runtime_activated([12; 32], commitment.directory_authority_hash)
            .expect("retry runtime activation receipt");
    assert_eq!(activated_again, activated);
    assert!(matches!(
        &activated.progress,
        RootComponentAllocationProgressView::Committed {
            commitment: RootComponentCommitmentView {
                directory_prepared: true,
                runtime_activated: true,
                membership: None,
                ..
            },
            ..
        }
    ));
    assert_membership_receipt(
        &activated,
        prepared_partition,
        directory,
        maximum_component_registry_bytes,
    );
}

fn assert_membership_receipt(
    activated: &RootComponentAllocationView,
    prepared_partition: &ComponentRegistryPartitionView,
    directory: FleetDirectorySnapshot,
    maximum_component_registry_bytes: u64,
) {
    assert!(committed_membership(activated).is_none());
    let (membership_activated, active_partition) = ComponentRegistryOps::activate_membership(
        [12; 32],
        33,
        maximum_component_registry_bytes,
        directory.clone(),
    )
    .expect("activate Registry membership");
    let repeated_membership = ComponentRegistryOps::activate_membership(
        [12; 32],
        34,
        maximum_component_registry_bytes,
        directory.clone(),
    )
    .expect("repeat Registry membership activation");
    assert_eq!(
        repeated_membership,
        (membership_activated.clone(), active_partition.clone())
    );
    assert_eq!(active_partition.status, ComponentLifecycleStatus::Active);
    assert_eq!(active_partition.revision, 2);
    assert_eq!(active_partition.directory_synchronized_at_ns, 33);
    assert_eq!(
        ComponentRegistryOps::active_membership_partition([12; 32])
            .expect("reconstruct active membership partition"),
        active_partition
    );
    assert_ne!(
        active_partition.content_hash,
        prepared_partition.content_hash
    );
    assert_eq!(
        ComponentRegistryOps::prepared_partition([12; 32]).expect("reconstruct prepared partition"),
        *prepared_partition
    );
    assert_eq!(
        ComponentRegistryOps::commit_verified(
            [12; 32],
            35,
            maximum_component_registry_bytes,
            directory,
        )
        .expect("commit retry after membership activation")
        .1,
        *prepared_partition
    );

    let membership_synchronized = ComponentRegistryOps::mark_membership_synchronized(
        [12; 32],
        committed_membership(&membership_activated)
            .expect("membership receipt")
            .directory_authority_hash,
    )
    .expect("mark membership Directory synchronized");
    let synchronized_again = ComponentRegistryOps::mark_membership_synchronized(
        [12; 32],
        committed_membership(&membership_activated)
            .expect("membership receipt")
            .directory_authority_hash,
    )
    .expect("repeat membership Directory receipt");
    assert_eq!(synchronized_again, membership_synchronized);
    assert!(
        committed_membership(&membership_synchronized)
            .expect("membership receipt")
            .directory_synchronized
    );
    assert_initial_inventory_receipt();
}

fn assert_initial_inventory_receipt() {
    let sealed =
        ComponentRegistryOps::seal_initial_inventory([40; 32], 41).expect("seal initial inventory");
    assert_eq!(sealed.operation_ids, vec![[12; 32]]);
    assert_eq!(sealed.receipt.fleet_activation_operation_id, [40; 32]);
    assert_eq!(sealed.receipt.component_count, 1);
    assert_ne!(sealed.receipt.inventory_hash, [0; 32]);
    assert_eq!(sealed.receipt.sealed_at_ns, 41);
    assert!(!sealed.receipt.directories_converged);
    assert!(!sealed.receipt.root_runtime_activated);
    let repeated = ComponentRegistryOps::seal_initial_inventory([40; 32], 42)
        .expect("retry initial inventory seal");
    assert_eq!(repeated, sealed);
    assert!(
        ComponentRegistryOps::reserve_allocation(
            TopLevelComponentAllocationDecision {
                allocation_sequence: 2,
                component: ComponentInstanceId::from_generated_bytes([42; 32]),
                component_spec: "projects".parse().expect("Component Spec"),
                spec_hash: [43; 32],
                role: CanisterRole::new("project_hub"),
            },
            [44; 32],
            ComponentProvisioningOrigin::FleetAdministrator {
                caller: candid::Principal::from_slice(&[11; 29]),
            },
            false,
        )
        .is_err(),
        "a Prepared root cannot extend its sealed initial inventory"
    );

    let converged = ComponentRegistryOps::mark_initial_inventory_directories_converged(
        [40; 32],
        sealed.receipt.inventory_hash,
    )
    .expect("mark initial Directories converged");
    assert!(converged.directories_converged);
    assert!(!converged.root_runtime_activated);
    let terminal = ComponentRegistryOps::mark_initial_inventory_root_runtime_activated(
        [40; 32],
        sealed.receipt.inventory_hash,
    )
    .expect("mark root runtime activated");
    assert!(terminal.directories_converged);
    assert!(terminal.root_runtime_activated);
    assert_eq!(
        ComponentRegistryOps::initial_inventory([40; 32]).expect("terminal initial inventory"),
        terminal
    );
    assert_child_reservation_preserves_membership_receipt();
    ComponentRegistryOps::reserve_allocation(
        TopLevelComponentAllocationDecision {
            allocation_sequence: 2,
            component: ComponentInstanceId::from_generated_bytes([42; 32]),
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [43; 32],
            role: CanisterRole::new("project_hub"),
        },
        [44; 32],
        ComponentProvisioningOrigin::FleetAdministrator {
            caller: candid::Principal::from_slice(&[11; 29]),
        },
        true,
    )
    .expect("active root admits dynamic allocation after terminal initial receipt");
}

fn assert_child_reservation_preserves_membership_receipt() {
    let allocation =
        ComponentRegistryOps::allocation([12; 32]).expect("committed Component allocation");
    let membership = committed_membership(&allocation)
        .expect("active membership receipt")
        .clone();
    let partition = ComponentRegistryOps::partition(allocation.component)
        .expect("valid active partition")
        .expect("active partition");
    ComponentRegistryOps::reserve_child_allocation(
        ComponentChildAllocationDecision {
            component: allocation.component,
            component_spec: allocation.component_spec,
            spec_hash: allocation.spec_hash,
            parent_canister_id: partition.binding.canister_id,
            parent_role: partition.binding.role.clone(),
            child_role: CanisterRole::new("project_instance"),
            child_kind: ComponentChildKind::Instance,
            mode: canic_core::control_plane_support::policy::component_child_allocation::ComponentChildAllocationMode::Active,
            maximum_instances_per_parent: 10_000,
            maximum_descendants: 20_000,
            maximum_registry_bytes: 16_777_216,
        },
        [50; 32],
        None,
        ComponentRegistryHead {
            component: allocation.component,
            revision: partition.revision,
            content_hash: partition.content_hash,
        },
    )
    .expect("reserve active Component child");

    let retried = ComponentRegistryOps::mark_membership_synchronized(
        [12; 32],
        membership.directory_authority_hash,
    )
    .expect("immutable membership receipt remains valid after child reservation");
    assert_eq!(
        committed_membership(&retried).expect("membership receipt"),
        &membership
    );
    assert_eq!(
        ComponentRegistryOps::partition(allocation.component)
            .expect("valid active partition")
            .expect("active partition")
            .reserved_descendants,
        1
    );
}

fn committed_membership(
    allocation: &RootComponentAllocationView,
) -> Option<&RootComponentMembershipView> {
    let RootComponentAllocationProgressView::Committed { commitment, .. } = &allocation.progress
    else {
        panic!("committed allocation progress");
    };
    commitment.membership.as_ref()
}

fn fleet_directory(root: &FleetSubnetRootBinding) -> FleetDirectorySnapshot {
    FleetDirectorySnapshot {
        provenance: FleetDirectoryProvenance {
            registry: FleetRegistryVersion {
                authority: root.authority.clone(),
                revision: 4,
                content_hash: [5; 32],
            },
            source_fleet_subnet_root: root.fleet_subnet_root,
        },
        fleet_subnet_roots: vec![FleetSubnetRootDirectoryEntry {
            placement_subnet: root.placement_subnet,
            fleet_subnet_root: root.fleet_subnet_root,
            status: FleetSubnetRootStatus::Active,
        }],
        services: vec![],
    }
}

fn assert_final_inventory_receipt(
    partition: &ComponentRegistryPartitionView,
    fleet: &FleetDirectorySnapshot,
    inventory: &RootComponentFinalInventoryView,
) {
    assert_eq!(inventory.registry.component, partition.binding.component);
    assert_eq!(inventory.registry.revision, partition.revision);
    assert_eq!(inventory.registry.content_hash, partition.content_hash);
    assert_eq!(
        inventory.descendant_content_hash,
        empty_component_descendant_content_hash(partition.binding.component)
    );
    assert_eq!(inventory.registry_encoded_bytes, partition.encoded_bytes);
    assert_eq!(
        inventory.covered_fleet_registry_revision,
        fleet.provenance.registry.revision
    );
    assert_ne!(inventory.directory_authority_hash, [0; 32]);
    assert_ne!(inventory.inventory_hash, [0; 32]);
    assert_eq!(inventory.finalized_at_ns, 112);
}

#[test]
fn directory_refresh_is_exact_idempotent_and_conflict_closed() {
    let root = root_binding();
    let release_set = FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([8; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
    };
    let component = ComponentInstanceId::from_generated_bytes([10; 32]);
    let canister = candid::Principal::from_slice(&[18; 29]);
    RootComponentRegistryStore::import(active_single_component_registry_data(
        &root,
        release_set,
        component,
        canister,
    ));

    assert!(
        ComponentRegistryOps::directory_synchronization_targets(&[component, component]).is_err()
    );
    let before = RootComponentRegistryStore::export();
    let target = ComponentRegistryOps::directory_synchronization_targets(&[component])
        .expect("select active Directory target")
        .pop()
        .expect("one active Directory target");
    let fleet = fleet_directory(&root);
    let plan = ComponentRegistryOps::prepare_directory_refresh(&target, fleet.clone(), None, 40)
        .expect("prepare exact Directory refresh");
    assert_eq!(RootComponentRegistryStore::export(), before);
    assert_eq!(plan.previous_registry, target.source_registry);
    assert_eq!(plan.registry.revision, target.source_registry.revision + 1);
    assert_ne!(plan.directory_authority_hash, [0; 32]);
    assert!(
        ComponentRegistryOps::prepare_directory_refresh(&target, fleet.clone(), None, 33).is_err()
    );

    let intent = crate::view::component_directory_synchronization::RootComponentDirectorySynchronizationIntentView {
        component_index: 0,
        component,
        canister_id: canister,
        allocation_operation_id: target.allocation_operation_id,
        previous_registry: plan.previous_registry.clone(),
        registry: plan.registry.clone(),
        directory_synchronized_at_ns: plan.directory_synchronized_at_ns,
        directory_authority_hash: plan.directory_authority_hash,
        started_at_ns: 39,
    };
    assert_eq!(
        ComponentRegistryOps::directory_refresh_plan_for_intent(&intent, fleet.clone(), None,)
            .expect("reconstruct pre-commit Directory refresh"),
        plan
    );

    let committed =
        ComponentRegistryOps::commit_directory_refresh(&plan, root.limits.maximum_registry_bytes)
            .expect("commit Directory refresh");
    assert_eq!(committed.revision, plan.registry.revision);
    assert_eq!(committed.content_hash, plan.registry.content_hash);
    assert_eq!(
        committed.directory_synchronized_at_ns,
        plan.directory_synchronized_at_ns
    );
    let after = RootComponentRegistryStore::export();
    assert_eq!(
        ComponentRegistryOps::directory_refresh_plan_for_intent(&intent, fleet, None)
            .expect("reconstruct post-commit Directory refresh"),
        plan
    );
    assert_eq!(
        ComponentRegistryOps::commit_directory_refresh(&plan, root.limits.maximum_registry_bytes,)
            .expect("replay Directory refresh"),
        committed
    );
    assert_eq!(RootComponentRegistryStore::export(), after);

    let mut conflicting_intent = intent;
    conflicting_intent.directory_authority_hash = [u8::MAX; 32];
    assert!(
        ComponentRegistryOps::directory_refresh_plan_for_intent(
            &conflicting_intent,
            fleet_directory(&root),
            None,
        )
        .is_err()
    );
    assert_eq!(RootComponentRegistryStore::export(), after);
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

fn active_single_component_registry_data(
    root: &FleetSubnetRootBinding,
    release_set: FleetSubnetRootReleaseSet,
    component: ComponentInstanceId,
    canister: candid::Principal,
) -> RootComponentRegistryData {
    let mut partition = active_component_partition(root, release_set, component, canister);
    let allocation = active_component_allocation(&partition);
    let mut data = RootComponentRegistryData {
        current: Some(RootComponentRegistryMetaRecord {
            root: root.clone(),
            prepared_against_registry: FleetRegistryVersion {
                authority: root.authority.clone(),
                revision: 4,
                content_hash: [5; 32],
            },
            release_set,
            store_bootstrap: RootStoreBootstrapRequest {
                operation_id: [8; 32],
                manifest_payload_size_bytes: 128,
            },
            next_allocation_sequence: 2,
            reserved_component_instances: 0,
            committed_component_instances: 1,
            managed_descendants: 0,
            known_created_component_canisters: 1,
            encoded_bytes: 0,
            initial_inventory: None,
            root_draining: None,
        }),
        partitions: vec![partition.clone()],
        allocations: vec![allocation],
        ..RootComponentRegistryData::default()
    };
    partition.encoded_bytes = 0;
    for _ in 0..8 {
        data.partitions[0] = partition.clone();
        let encoded_bytes = exact_component_registry_entry_bytes(&data, component);
        if partition.encoded_bytes == encoded_bytes {
            break;
        }
        partition.encoded_bytes = encoded_bytes;
    }
    data.partitions[0] = partition;
    data.current.as_mut().expect("Registry meta").encoded_bytes = exact_registry_entry_bytes(&data);
    data
}

#[test]
fn prequiescence_draining_component_retains_committed_runtime_operation() {
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
    let root = root_binding();
    let release_set = FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([8; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
    };
    let component = ComponentInstanceId::from_generated_bytes([97; 32]);
    let canister = candid::Principal::from_slice(&[98; 29]);
    RootComponentRegistryStore::import(active_single_component_registry_data(
        &root,
        release_set,
        component,
        canister,
    ));
    let active = ComponentRegistryOps::partition(component)
        .expect("active partition read")
        .expect("active partition");
    let binding = ManagedCanisterBinding::Component(active.binding.clone());
    assert_eq!(
        ComponentRegistryOps::managed_runtime_operation_id(&binding)
            .expect("active runtime operation"),
        [12; 32]
    );

    let draining = ComponentRegistryOps::begin_component_draining(
        component,
        [99; 32],
        ComponentRegistryHead {
            component,
            revision: active.revision,
            content_hash: active.content_hash,
        },
        100,
        16_777_216,
        fleet_directory(&root),
    )
    .expect("begin Component draining");
    assert!(draining.quiescence.is_none());
    assert_eq!(
        ComponentRegistryOps::managed_runtime_operation_id(&binding)
            .expect("pre-quiescence draining runtime operation"),
        [12; 32]
    );

    let (quiescent, _, _) = import_empty_quiescent_component();
    let quiescent_binding = ManagedCanisterBinding::Component(quiescent.binding);
    assert!(ComponentRegistryOps::managed_runtime_operation_id(&quiescent_binding).is_err());
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

fn import_empty_quiescent_component() -> (
    ComponentRegistryPartitionView,
    RootComponentDrainingView,
    FleetDirectorySnapshot,
) {
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
    let root = root_binding();
    let release_set = FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([8; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
    };
    let component = ComponentInstanceId::from_generated_bytes([97; 32]);
    let canister = candid::Principal::from_slice(&[98; 29]);
    let data = active_single_component_registry_data(&root, release_set, component, canister);
    RootComponentRegistryStore::import(data);
    let fleet = fleet_directory(&root);
    let active_partition = ComponentRegistryOps::partition(component)
        .expect("active partition read")
        .expect("active partition");
    let draining = ComponentRegistryOps::begin_component_draining(
        component,
        [99; 32],
        ComponentRegistryHead {
            component,
            revision: active_partition.revision,
            content_hash: active_partition.content_hash,
        },
        100,
        16_777_216,
        fleet.clone(),
    )
    .expect("drain empty Component");
    let draining_partition = ComponentRegistryOps::partition(component)
        .expect("draining partition read")
        .expect("draining partition");
    let directory_authority = ComponentRuntimeDirectoryAuthority {
        fleet: fleet.clone(),
        component: ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: draining_partition.binding.clone(),
                source_fleet_subnet_root: draining_partition.binding.fleet_subnet_root,
                component_registry_revision: draining_partition.revision,
                component_registry_content_hash: draining_partition.content_hash,
                synchronized_at_ns: draining_partition.directory_synchronized_at_ns,
            },
            descendant_count: 0,
        },
        component_group: None,
    };
    let authority_hash = ComponentRuntimeOps::directory_authority_hash(&directory_authority)
        .expect("empty draining Directory authority hash");
    ComponentRegistryOps::prepare_component_quiescence(
        component,
        draining.operation_id,
        draining.registry.clone(),
        ComponentRuntimeDirectoryConvergenceEvidence {
            operation_id: [100; 32],
            binding: ManagedCanisterBinding::Component(draining_partition.binding),
            covered_authority: directory_authority,
            covered_authority_hash: authority_hash,
            activation: ComponentRuntimeActivationEvidence {
                directory_authority_hash: [101; 32],
                activated_at_ns: 102,
            },
        },
        [103; 32],
        110,
        16_777_216,
    )
    .expect("prepare empty Component quiescence");
    ComponentRegistryOps::mark_component_quiescent(
        component,
        draining.operation_id,
        [103; 32],
        111,
    )
    .expect("observe empty Component quiescence");
    let partition = ComponentRegistryOps::partition(component)
        .expect("quiescent partition read")
        .expect("quiescent partition");
    (partition, draining, fleet)
}

fn advance_install_to_verified(plan: &RootComponentInstallPlan, created_bytes: u64) -> u64 {
    ComponentRegistryOps::validate_install_capacity([12; 32], plan).expect("install capacity");
    let intent = ComponentRegistryOps::begin_install(
        [12; 32],
        plan.clone(),
        ReplayCostGuardSettlement {
            quota_intent_id: IntentId(23),
            reservation_intent_id: IntentId(24),
        },
    )
    .expect("install intent");
    let intent_bytes = ComponentRegistryOps::current()
        .expect("Registry status")
        .encoded_bytes;
    assert!(intent_bytes > created_bytes);
    assert!(matches!(
        intent.progress,
        RootComponentAllocationProgressView::InstallIntent { .. }
    ));

    let mut conflicting = plan.clone();
    conflicting.raw_module_hash = [25; 32];
    assert!(
        ComponentRegistryOps::renew_install_intent(
            [12; 32],
            &conflicting,
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(26),
                reservation_intent_id: IntentId(27),
            },
        )
        .is_err()
    );

    let interrupted = RootComponentRegistryStore::export();
    RootComponentRegistryStore::import(interrupted);
    let renewed = ComponentRegistryOps::renew_install_intent(
        [12; 32],
        plan,
        ReplayCostGuardSettlement {
            quota_intent_id: IntentId(28),
            reservation_intent_id: IntentId(29),
        },
    )
    .expect("renew exact install intent");
    let RootComponentAllocationProgressView::InstallIntent { installation, .. } = &renewed.progress
    else {
        panic!("renewed install intent");
    };
    assert_eq!(installation.raw_module_hash, plan.raw_module_hash);
    assert_eq!(installation.binding, plan.binding);
    assert_eq!(
        installation.cost_guard_settlement.quota_intent_id,
        IntentId(28)
    );
    assert_eq!(
        ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes,
        intent_bytes
    );

    let installed = ComponentRegistryOps::mark_installed([12; 32]).expect("mark installed");
    let installed_retry = ComponentRegistryOps::mark_installed([12; 32]).expect("installed retry");
    assert_eq!(installed, installed_retry);
    assert!(matches!(
        installed.progress,
        RootComponentAllocationProgressView::Installed { .. }
    ));

    let verified = ComponentRegistryOps::mark_verified([12; 32]).expect("mark verified");
    let verified_retry = ComponentRegistryOps::mark_verified([12; 32]).expect("verified retry");
    assert_eq!(verified, verified_retry);
    assert!(matches!(
        verified.progress,
        RootComponentAllocationProgressView::Verified { .. }
    ));
    assert_eq!(
        ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes,
        intent_bytes,
        "the install intent must reserve terminal record capacity before the effect"
    );
    intent_bytes
}

fn prepared_created_allocation() -> (
    FleetSubnetRootBinding,
    RootComponentAllocationView,
    candid::Principal,
) {
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
    let root = root_binding();
    ComponentRegistryOps::prepare(
        root.clone(),
        FleetRegistryVersion {
            authority: root.authority.clone(),
            revision: 4,
            content_hash: [5; 32],
        },
        FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        },
        RootStoreBootstrapRequest {
            operation_id: [8; 32],
            manifest_payload_size_bytes: 128,
        },
    )
    .expect("prepare");
    ComponentRegistryOps::reserve_allocation(
        TopLevelComponentAllocationDecision {
            allocation_sequence: 1,
            component: ComponentInstanceId::from_generated_bytes([10; 32]),
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [6; 32],
            role: CanisterRole::new("project_hub"),
        },
        [12; 32],
        ComponentProvisioningOrigin::FleetAdministrator {
            caller: candid::Principal::from_slice(&[11; 29]),
        },
        false,
    )
    .expect("reserve");
    ComponentRegistryOps::begin_creation(
        [12; 32],
        RootComponentCreationPlan {
            wasm_store: candid::Principal::from_slice(&[13; 29]),
            payload_hash: [14; 32],
            payload_size_bytes: 4_096,
            initial_cycles: Cycles::new(5_000_000_000_000),
            controller: root.fleet_subnet_root,
        },
        ReplayCostGuardSettlement {
            quota_intent_id: IntentId(16),
            reservation_intent_id: IntentId(17),
        },
    )
    .expect("creation intent");
    let canister = candid::Principal::from_slice(&[18; 29]);
    let created =
        ComponentRegistryOps::mark_created([12; 32], canister).expect("record created allocation");
    (root, created, canister)
}

fn assert_creation_capacity_is_reserved_before_effect(
    plan: &RootComponentCreationPlan,
    reserved_bytes: u64,
) {
    let before_creation = RootComponentRegistryStore::export();
    let mut exhausted = before_creation.clone();
    exhausted
        .current
        .as_mut()
        .expect("Registry meta")
        .root
        .limits
        .maximum_registry_bytes = reserved_bytes;
    RootComponentRegistryStore::import(exhausted);

    let capacity_error = ComponentRegistryOps::validate_creation_capacity([12; 32], plan)
        .expect_err("terminal creation evidence must fit before the paid effect");
    assert!(capacity_error.is_public_resource_exhausted());
    assert!(matches!(
        ComponentRegistryOps::allocation([12; 32])
            .expect("reserved allocation")
            .progress,
        RootComponentAllocationProgressView::Reserved
    ));
    assert_eq!(
        ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes,
        reserved_bytes
    );
    RootComponentRegistryStore::import(before_creation);
}

#[test]
fn allocation_reservation_fails_before_mutation_when_registry_capacity_is_exhausted() {
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
    let mut root = root_binding();
    root.limits.maximum_registry_bytes = 1;
    let version = FleetRegistryVersion {
        authority: root.authority.clone(),
        revision: 4,
        content_hash: [5; 32],
    };
    ComponentRegistryOps::prepare(
        root,
        version,
        FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        },
        RootStoreBootstrapRequest {
            operation_id: [8; 32],
            manifest_payload_size_bytes: 128,
        },
    )
    .expect("prepare");

    let error = ComponentRegistryOps::reserve_allocation(
        TopLevelComponentAllocationDecision {
            allocation_sequence: 1,
            component: ComponentInstanceId::from_generated_bytes([10; 32]),
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [6; 32],
            role: CanisterRole::new("project_hub"),
        },
        [12; 32],
        ComponentProvisioningOrigin::FleetAdministrator {
            caller: candid::Principal::from_slice(&[11; 29]),
        },
        false,
    )
    .expect_err("Registry byte capacity must reject reservation");
    assert!(error.is_public_resource_exhausted());
    assert!(ComponentRegistryOps::allocation([12; 32]).is_none());

    let status = ComponentRegistryOps::current().expect("Registry status");
    assert_eq!(status.next_allocation_sequence, 1);
    assert_eq!(status.reserved_component_instances, 0);
    assert_eq!(status.encoded_bytes, 0);
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

struct ActiveComponentTreeFixture {
    component: ComponentInstanceId,
    partition: ComponentRegistryPartitionRecord,
    target: ComponentRegistryChildRecord,
    descendant: ComponentRegistryChildRecord,
    unrelated: ComponentRegistryChildRecord,
}

fn import_active_component_tree() -> ActiveComponentTreeFixture {
    import_active_component_tree_with_origin(ComponentProvisioningOrigin::FleetAdministrator {
        caller: candid::Principal::from_slice(&[11; 29]),
    })
}

fn import_grouped_active_component_tree() -> ActiveComponentTreeFixture {
    import_active_component_tree_with_origin(ComponentProvisioningOrigin::ComponentGroup {
        operation_id: [41; 32],
        plan_hash: [42; 32],
        group_placement: ComponentGroupPlacementId {
            deployment: "project_cells"
                .parse::<ComponentGroupDeploymentId>()
                .expect("deployment ID"),
            ordinal: 0,
        },
        member_path: ComponentGroupMemberPath::try_from(vec![
            "project".parse().expect("member ID"),
        ])
        .expect("member path"),
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "the fixture assembles one exact normalized multi-level Component tree"
)]
fn import_active_component_tree_with_origin(
    provisioning_origin: ComponentProvisioningOrigin,
) -> ActiveComponentTreeFixture {
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
    let root = root_binding();
    let release_set = FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([8; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
    };
    let component = ComponentInstanceId::from_generated_bytes([10; 32]);
    let component_canister = candid::Principal::from_slice(&[18; 29]);
    let mut partition =
        active_component_partition(&root, release_set, component, component_canister);
    partition.provisioning_origin = provisioning_origin;
    let target = ComponentRegistryChildRecord {
        component,
        canister_id: candid::Principal::from_slice(&[21; 29]),
        parent_canister_id: component_canister,
        role: CanisterRole::new("project_instance"),
        kind: ComponentChildKind::Instance,
        installed_artifact_hash: [31; 32],
        protocol_profile_digest: ProtocolProfileDigest::from_bytes([41; 32]),
        status: ComponentLifecycleStatus::Active,
    };
    let descendant = ComponentRegistryChildRecord {
        component,
        canister_id: candid::Principal::from_slice(&[22; 29]),
        parent_canister_id: target.canister_id,
        role: CanisterRole::new("project_ledger"),
        kind: ComponentChildKind::Singleton,
        installed_artifact_hash: [32; 32],
        protocol_profile_digest: ProtocolProfileDigest::from_bytes([42; 32]),
        status: ComponentLifecycleStatus::Active,
    };
    let unrelated = ComponentRegistryChildRecord {
        component,
        canister_id: candid::Principal::from_slice(&[23; 29]),
        parent_canister_id: component_canister,
        role: CanisterRole::new("project_instance"),
        kind: ComponentChildKind::Instance,
        installed_artifact_hash: [33; 32],
        protocol_profile_digest: ProtocolProfileDigest::from_bytes([43; 32]),
        status: ComponentLifecycleStatus::Active,
    };
    let alternate_descendant = ComponentRegistryChildRecord {
        component,
        canister_id: candid::Principal::from_slice(&[24; 29]),
        parent_canister_id: target.canister_id,
        role: CanisterRole::new("project_machine"),
        kind: ComponentChildKind::Singleton,
        installed_artifact_hash: [34; 32],
        protocol_profile_digest: ProtocolProfileDigest::from_bytes([44; 32]),
        status: ComponentLifecycleStatus::Active,
    };
    let children = vec![
        target.clone(),
        descendant.clone(),
        unrelated.clone(),
        alternate_descendant,
    ];
    let child_traversals = children
        .iter()
        .map(|child| ComponentRegistryChildTraversalRecord {
            component,
            parent_canister_id: child.parent_canister_id,
            role: child.role.clone(),
            canister_id: child.canister_id,
        })
        .collect();
    let parent_role_counts = vec![
        ComponentRegistryParentRoleCountRecord {
            component,
            parent_canister_id: component_canister,
            child_role: CanisterRole::new("project_instance"),
            instances: 2,
        },
        ComponentRegistryParentRoleCountRecord {
            component,
            parent_canister_id: target.canister_id,
            child_role: CanisterRole::new("project_ledger"),
            instances: 1,
        },
        ComponentRegistryParentRoleCountRecord {
            component,
            parent_canister_id: target.canister_id,
            child_role: CanisterRole::new("project_machine"),
            instances: 1,
        },
    ];
    partition.committed_descendants = 4;
    partition.descendant_content_hash = [77; 32];
    partition.content_hash = component_partition_content_hash(
        &partition.binding,
        partition.protocol_profile_digest,
        &partition.provisioning_origin,
        partition.release_set,
        partition.status,
        partition.revision,
        partition.descendant_content_hash,
        partition.committed_descendants,
    )
    .expect("nonempty partition hash");
    partition.encoded_bytes = 0;
    let mut data = RootComponentRegistryData {
        partitions: vec![partition.clone()],
        children,
        child_traversals,
        parent_role_counts,
        ..RootComponentRegistryData::default()
    };
    for _ in 0..8 {
        data.partitions[0] = partition.clone();
        let encoded_bytes = exact_component_registry_entry_bytes(&data, component);
        if partition.encoded_bytes == encoded_bytes {
            break;
        }
        partition.encoded_bytes = encoded_bytes;
    }
    data.partitions[0] = partition.clone();
    assert_eq!(
        partition.encoded_bytes,
        exact_component_registry_entry_bytes(&data, component)
    );
    data.current = Some(RootComponentRegistryMetaRecord {
        root: root.clone(),
        prepared_against_registry: FleetRegistryVersion {
            authority: root.authority,
            revision: 4,
            content_hash: [5; 32],
        },
        release_set,
        store_bootstrap: RootStoreBootstrapRequest {
            operation_id: [8; 32],
            manifest_payload_size_bytes: 128,
        },
        next_allocation_sequence: 2,
        reserved_component_instances: 0,
        committed_component_instances: 1,
        managed_descendants: 4,
        known_created_component_canisters: 5,
        encoded_bytes: partition.encoded_bytes,
        initial_inventory: None,
        root_draining: None,
    });
    RootComponentRegistryStore::import(data);
    ActiveComponentTreeFixture {
        component,
        partition,
        target,
        descendant,
        unrelated,
    }
}

fn import_deep_active_component_tree(
    depth: usize,
) -> (ActiveComponentTreeFixture, candid::Principal) {
    assert!(depth > 0 && depth <= 100);
    let mut fixture = import_active_component_tree();
    let mut data = RootComponentRegistryStore::export();
    let mut parent_canister_id = fixture.descendant.canister_id;
    let role = CanisterRole::new("deep_node");

    for index in 0..depth {
        let principal_byte = 100_u8
            .checked_add(u8::try_from(index).expect("bounded deep-tree index"))
            .expect("bounded deep-tree principal");
        let canister_id = candid::Principal::from_slice(&[principal_byte; 29]);
        let child = ComponentRegistryChildRecord {
            component: fixture.component,
            canister_id,
            parent_canister_id,
            role: role.clone(),
            kind: ComponentChildKind::Instance,
            installed_artifact_hash: [principal_byte; 32],
            protocol_profile_digest: ProtocolProfileDigest::from_bytes([principal_byte; 32]),
            status: ComponentLifecycleStatus::Active,
        };
        data.children.push(child);
        data.child_traversals
            .push(ComponentRegistryChildTraversalRecord {
                component: fixture.component,
                parent_canister_id,
                role: role.clone(),
                canister_id,
            });
        data.parent_role_counts
            .push(ComponentRegistryParentRoleCountRecord {
                component: fixture.component,
                parent_canister_id,
                child_role: role.clone(),
                instances: 1,
            });
        parent_canister_id = canister_id;
    }

    let partition = data
        .partitions
        .iter_mut()
        .find(|partition| partition.binding.component == fixture.component)
        .expect("deep-tree partition");
    partition.committed_descendants = partition
        .committed_descendants
        .checked_add(u32::try_from(depth).expect("bounded deep-tree depth"))
        .expect("deep-tree descendant count");
    partition.descendant_content_hash = [78; 32];
    partition.content_hash = component_partition_content_hash(
        &partition.binding,
        partition.protocol_profile_digest,
        &partition.provisioning_origin,
        partition.release_set,
        partition.status,
        partition.revision,
        partition.descendant_content_hash,
        partition.committed_descendants,
    )
    .expect("deep-tree partition hash");
    partition.encoded_bytes = 0;
    for _ in 0..8 {
        let encoded_bytes = exact_component_registry_entry_bytes(&data, fixture.component);
        let partition = data
            .partitions
            .iter_mut()
            .find(|partition| partition.binding.component == fixture.component)
            .expect("deep-tree partition");
        if partition.encoded_bytes == encoded_bytes {
            break;
        }
        partition.encoded_bytes = encoded_bytes;
    }
    fixture.partition = data
        .partitions
        .iter()
        .find(|partition| partition.binding.component == fixture.component)
        .expect("deep-tree partition")
        .clone();
    assert_eq!(
        fixture.partition.encoded_bytes,
        exact_component_registry_entry_bytes(&data, fixture.component)
    );
    let depth = u32::try_from(depth).expect("bounded deep-tree depth");
    let encoded_bytes = exact_registry_entry_bytes(&data);
    let current = data.current.as_mut().expect("deep-tree Registry status");
    current.managed_descendants = current
        .managed_descendants
        .checked_add(depth)
        .expect("deep-tree managed count");
    current.known_created_component_canisters = current
        .known_created_component_canisters
        .checked_add(depth)
        .expect("deep-tree known-created count");
    current.encoded_bytes = encoded_bytes;
    RootComponentRegistryStore::import(data);
    (fixture, parent_canister_id)
}

fn active_component_partition(
    root: &FleetSubnetRootBinding,
    release_set: FleetSubnetRootReleaseSet,
    component: ComponentInstanceId,
    canister_id: candid::Principal,
) -> ComponentRegistryPartitionRecord {
    let binding = ComponentBinding {
        authority: root.authority.clone(),
        component,
        component_spec: "projects".parse().expect("Component Spec"),
        spec_hash: [6; 32],
        role: CanisterRole::new("project_hub"),
        placement_subnet: root.placement_subnet,
        fleet_subnet_root: root.fleet_subnet_root,
        canister_id,
    };
    let provisioning_origin = ComponentProvisioningOrigin::FleetAdministrator {
        caller: candid::Principal::from_slice(&[11; 29]),
    };
    let descendant_content_hash = empty_component_descendant_content_hash(component);
    let mut partition = ComponentRegistryPartitionRecord {
        content_hash: component_partition_content_hash(
            &binding,
            ProtocolProfileDigest::from_bytes([42; 32]),
            &provisioning_origin,
            release_set,
            ComponentLifecycleStatus::Active,
            2,
            descendant_content_hash,
            0,
        )
        .expect("partition hash"),
        binding,
        protocol_profile_digest: ProtocolProfileDigest::from_bytes([42; 32]),
        provisioning_origin,
        release_set,
        status: ComponentLifecycleStatus::Active,
        revision: 2,
        descendant_content_hash,
        directory_synchronized_at_ns: 33,
        reserved_descendants: 0,
        committed_descendants: 0,
        encoded_bytes: 0,
    };
    let principal_index_bytes =
        RootComponentRegistryStore::principal_index_entry_bytes(canister_id, component);
    for _ in 0..8 {
        let encoded_bytes =
            RootComponentRegistryStore::partition_entry_bytes(&partition) + principal_index_bytes;
        if partition.encoded_bytes == encoded_bytes {
            break;
        }
        partition.encoded_bytes = encoded_bytes;
    }
    assert_eq!(
        partition.encoded_bytes,
        RootComponentRegistryStore::partition_entry_bytes(&partition) + principal_index_bytes
    );
    partition
}

fn active_component_allocation(
    partition: &ComponentRegistryPartitionRecord,
) -> RootComponentAllocationRecord {
    let creation = RootComponentCreationEffectRecord {
        wasm_store: candid::Principal::from_slice(&[13; 29]),
        payload_hash: [14; 32],
        payload_size_bytes: 4_096,
        initial_cycles: Cycles::new(5_000_000_000_000),
        controller: partition.binding.fleet_subnet_root,
        cost_guard_settlement: ReplayCostGuardSettlement {
            quota_intent_id: IntentId(16),
            reservation_intent_id: IntentId(17),
        },
        charged_entry_bytes: 4_096,
    };
    let installation = RootComponentInstallEffectRecord {
        raw_module_hash: [19; 32],
        protocol_profile_digest: partition.protocol_profile_digest,
        chunk_hashes: vec![vec![20; 32]],
        binding: partition.binding.clone(),
        cost_guard_settlement: ReplayCostGuardSettlement {
            quota_intent_id: IntentId(21),
            reservation_intent_id: IntentId(22),
        },
        charged_entry_bytes: 16_777_216,
    };
    RootComponentAllocationRecord {
        operation_id: [12; 32],
        allocation_sequence: 1,
        component: partition.binding.component,
        component_spec: partition.binding.component_spec.clone(),
        spec_hash: partition.binding.spec_hash,
        role: partition.binding.role.clone(),
        provisioning_origin: partition.provisioning_origin.clone(),
        release_set: partition.release_set,
        progress: RootComponentAllocationProgressRecord::Committed {
            creation,
            canister: partition.binding.canister_id,
            installation,
            commitment: RootComponentCommitmentRecord {
                registry: component_partition_head(partition),
                prepared_registry_encoded_bytes: partition.encoded_bytes,
                directory_synchronized_at_ns: partition.directory_synchronized_at_ns,
                directory_authority_hash: [23; 32],
                directory_prepared: true,
                runtime_activated: true,
                membership: Some(RootComponentMembershipRecord {
                    registry: component_partition_head(partition),
                    descendant_content_hash: partition.descendant_content_hash,
                    reserved_descendants: partition.reserved_descendants,
                    committed_descendants: partition.committed_descendants,
                    registry_encoded_bytes: partition.encoded_bytes,
                    directory_synchronized_at_ns: partition.directory_synchronized_at_ns,
                    directory_authority_hash: [24; 32],
                    directory_synchronized: true,
                }),
            },
        },
    }
}

fn child_allocation_decision(
    partition: &ComponentRegistryPartitionRecord,
    child_role: &'static str,
) -> ComponentChildAllocationDecision {
    child_allocation_decision_for_parent(
        partition,
        partition.binding.canister_id,
        &partition.binding.role,
        child_role,
    )
}

fn child_allocation_decision_for_parent(
    partition: &ComponentRegistryPartitionRecord,
    parent_canister_id: candid::Principal,
    parent_role: &CanisterRole,
    child_role: &'static str,
) -> ComponentChildAllocationDecision {
    ComponentChildAllocationDecision {
        component: partition.binding.component,
        component_spec: partition.binding.component_spec.clone(),
        spec_hash: partition.binding.spec_hash,
        parent_canister_id,
        parent_role: parent_role.clone(),
        child_role: CanisterRole::new(child_role),
        child_kind: ComponentChildKind::Instance,
        mode: canic_core::control_plane_support::policy::component_child_allocation::ComponentChildAllocationMode::Active,
        maximum_instances_per_parent: 10_000,
        maximum_descendants: 20_000,
        maximum_registry_bytes: 16_777_216,
    }
}

fn component_registry_head(partition: &ComponentRegistryPartitionRecord) -> ComponentRegistryHead {
    ComponentRegistryHead {
        component: partition.binding.component,
        revision: partition.revision,
        content_hash: partition.content_hash,
    }
}

fn child_creation_plan(
    root: &FleetSubnetRootBinding,
    evidence_seed: u8,
) -> RootComponentCreationPlan {
    RootComponentCreationPlan {
        wasm_store: candid::Principal::from_slice(&[evidence_seed; 29]),
        payload_hash: [evidence_seed; 32],
        payload_size_bytes: 4_096,
        initial_cycles: Cycles::new(5_000_000_000_000),
        controller: root.fleet_subnet_root,
    }
}

fn root_binding() -> FleetSubnetRootBinding {
    let coordinator_subnet = SubnetId::from_principal(candid::Principal::from_slice(&[2; 29]));
    FleetSubnetRootBinding {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                        fleet_id: FleetId::from_generated_bytes([1; 32]),
                    },
                    app: AppId::from("toko"),
                },
                coordinator_subnet,
                coordinator: candid::Principal::from_slice(&[3; 29]),
            },
            epoch: 1,
        },
        placement_subnet: SubnetId::from_principal(candid::Principal::from_slice(&[4; 29])),
        fleet_subnet_root: candid::Principal::from_slice(&[5; 29]),
        component_admissions: vec![ComponentSpecAdmission {
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [6; 32],
            maximum_root_instances: 10,
        }],
        component_topology_digest: ComponentTopologyDigest::from_bytes([7; 32]),
        funding: crate::test_support::fleet_subnet_root_funding_authority(),
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 10,
            maximum_registry_bytes: 16_777_216,
            maximum_wasm_store_bytes: 268_435_456,
            maximum_group_placements: 16,
            canister_pool: canic_core::ids::FleetSubnetCanisterPoolConfig {
                minimum_size: 1,
                maximum_size: 10,
                canister_cycles: Cycles::new(5_000_000_000_000),
            },
            cycles_funding: CyclesFundingBudget {
                window_secs: 3_600,
                maximum_cycles: Cycles::new(1_000_000_000_000),
            },
        },
    }
}
