//! Focused proofs for durable root-batch acceptance and protected member derivation.

use super::*;
use crate::storage::stable::component_provisioning::{
    RootComponentProvisioningData, RootComponentProvisioningStore,
};
use crate::view::component_registry::RootComponentAllocationProgressView;
use candid::Principal;
use canic_core::{
    bootstrap::parse_config_model,
    cdk::types::Cycles,
    control_plane_support::config::ComponentTopology,
    dto::{
        component_provisioning::{
            ComponentGroupPlacementPlan, ComponentGroupPlanEntry, FleetSubnetRootProvisioningBatch,
        },
        fleet_registry::FleetRegistryVersion,
    },
    ids::{
        AppId, CanisterRole, CanonicalNetworkId, ComponentGroupMemberPath, ComponentInstanceId,
        ComponentSpecAdmission, CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding,
        FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetCanisterPoolConfig,
        FleetSubnetRootBinding, FleetSubnetRootLimits, FleetSubnetRootReleaseSet, ReleaseBuildId,
        ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    },
};
use std::collections::{BTreeMap, BTreeSet};

const CONFIG: &str = r#"
[app]
name = "root_batch_test"

[roles.root]
kind = "root"
package = "root"

[roles.alpha]
kind = "canister"
package = "alpha"

[component_specs.alpha]
component_role = "alpha"
maximum_instances = 8

[component_groups.cell.components.alpha]
component_spec = "alpha"
labels = { tier = "api" }

[component_group_deployments.cells]
component_group = "cell"
initial_placements = 1
maximum_placements = 4
placement.maximum_per_root = 2
placement.minimum_distinct_roots = 1
"#;

struct Fixture {
    request: RootComponentProvisioningAcceptanceRequest,
    validation: RootComponentProvisioningBatchValidation,
}

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(principal(byte))
}

fn authority() -> FleetRegistryAuthority {
    FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([3; 32]),
                },
                app: AppId::from("root_batch_test"),
            },
            coordinator_subnet: subnet(4),
            coordinator: principal(5),
        },
        epoch: 1,
    }
}

fn root_binding(
    topology: &ComponentTopology,
    component_spec: &canic_core::ids::ComponentSpecId,
) -> FleetSubnetRootBinding {
    let spec = topology.get(component_spec).expect("Component Spec");
    let component_admissions = vec![ComponentSpecAdmission {
        component_spec: component_spec.clone(),
        spec_hash: spec.spec_hash,
        maximum_root_instances: 8,
    }];
    let projection = topology
        .project_for_admissions(&component_admissions)
        .expect("root topology projection");
    FleetSubnetRootBinding {
        authority: authority(),
        placement_subnet: subnet(6),
        fleet_subnet_root: principal(7),
        component_admissions,
        component_topology_digest: projection.digest().expect("topology digest"),
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 8,
            maximum_registry_bytes: 16_777_216,
            maximum_wasm_store_bytes: 40_000_000,
            canister_pool: FleetSubnetCanisterPoolConfig {
                minimum_size: 1,
                maximum_size: 8,
                canister_cycles: Cycles::new(5_000_000_000_000),
            },
            cycles_funding: CyclesFundingBudget {
                window_secs: 3_600,
                maximum_cycles: Cycles::new(10_000_000_000_000),
            },
            maximum_group_placements: 4,
        },
    }
}

fn fixture() -> Fixture {
    RootComponentProvisioningStore::import(RootComponentProvisioningData::default());
    let config = parse_config_model(CONFIG).expect("valid config");
    let topology = config
        .compile_component_topology()
        .expect("Component Topology");
    let deployments = config
        .compile_component_group_deployment_topology()
        .expect("deployment topology");
    let deployment = deployments
        .get(&"cells".parse().expect("deployment ID"))
        .expect("deployment");
    let member = deployment.members.first().expect("flattened member");
    let root = root_binding(&topology, &member.component_spec);
    let entry = ComponentGroupPlanEntry {
        member_path: member.member_path.clone(),
        component_spec: member.component_spec.clone(),
        spec_hash: member.component_spec_hash,
        purpose: member.purpose.clone(),
        labels: member.labels.clone(),
        limits: member.limits.clone(),
    };
    let batch = FleetSubnetRootProvisioningBatch {
        root: root.clone(),
        active_release_set: FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        },
        placements: vec![ComponentGroupPlacementPlan {
            group_placement: canic_core::ids::ComponentGroupPlacementId {
                deployment: deployment.deployment.clone(),
                ordinal: 0,
            },
            component_group: deployment.component_group.clone(),
            entries: vec![entry],
        }],
    };
    Fixture {
        request: RootComponentProvisioningAcceptanceRequest {
            fleet_registry: FleetRegistryVersion {
                authority: root.authority,
                revision: 3,
                content_hash: [10; 32],
            },
            configuration_digest: config
                .compile_component_deployment_configuration_digest()
                .expect("configuration digest"),
            operation_id: [11; 32],
            plan_hash: [12; 32],
            batch,
        },
        validation: RootComponentProvisioningBatchValidation {
            placement_count: 1,
            component_count: 1,
            component_spec_counts: BTreeMap::from([(member.component_spec.clone(), 1)]),
            component_roles: BTreeSet::from([topology
                .get(&member.component_spec)
                .expect("Component Spec")
                .component_role
                .clone()]),
        },
    }
}

#[test]
fn exact_acceptance_replays_across_restart_without_mutating_capacity() {
    let fixture = fixture();
    assert!(RootComponentProvisioningOps::require_ordinary_allocation_open().is_ok());
    assert!(RootComponentProvisioningOps::require_root_draining_open().is_ok());
    let accepted =
        RootComponentProvisioningOps::accept(fixture.request.clone(), &fixture.validation, 100)
            .expect("accept batch");
    let replay = RootComponentProvisioningOps::acceptance_replay(&fixture.request)
        .expect("replay lookup")
        .expect("accepted replay");
    assert_eq!(replay, accepted);
    assert_eq!(
        RootComponentProvisioningOps::tracked_group_placements().expect("placement count"),
        1
    );
    assert!(RootComponentProvisioningOps::require_ordinary_allocation_open().is_err());
    assert!(RootComponentProvisioningOps::require_root_draining_open().is_err());

    let snapshot = RootComponentProvisioningStore::export();
    RootComponentProvisioningStore::import(snapshot.clone());
    assert_eq!(RootComponentProvisioningStore::export(), snapshot);
    assert_eq!(
        RootComponentProvisioningOps::accept(fixture.request, &fixture.validation, 999,)
            .expect("exact acceptance retry"),
        accepted
    );
    assert_eq!(
        RootComponentProvisioningOps::tracked_group_placements().expect("placement count"),
        1
    );
}

#[test]
fn reservation_advance_is_cursor_bound_restart_safe_and_terminal() {
    let fixture = fixture();
    let accepted =
        RootComponentProvisioningOps::accept(fixture.request.clone(), &fixture.validation, 100)
            .expect("accept batch");
    let request = RootComponentProvisioningAdvanceRequest {
        operation_id: fixture.request.operation_id,
        plan_hash: fixture.request.plan_hash,
        expected_reserved_component_count: 0,
    };
    assert_eq!(
        RootComponentProvisioningOps::reservation_disposition(request, &accepted)
            .expect("advance disposition"),
        RootComponentProvisioningReservationDisposition::Advance
    );
    let member = RootComponentProvisioningOps::next_member_reservation(&accepted)
        .expect("next member reservation");
    assert_eq!(
        member.member_operation_id,
        [
            171, 220, 109, 38, 164, 5, 152, 44, 236, 134, 217, 253, 36, 55, 213, 121, 56, 239, 238,
            30, 99, 196, 36, 234, 206, 2, 149, 236, 72, 124, 86, 131,
        ]
    );
    let allocation = RootComponentAllocationView {
        operation_id: member.member_operation_id,
        allocation_sequence: 1,
        component: ComponentInstanceId::from_generated_bytes([31; 32]),
        component_spec: member.component_spec.clone(),
        spec_hash: member.spec_hash,
        role: CanisterRole::new("alpha"),
        provisioning_origin: ComponentProvisioningOrigin::ComponentGroup {
            operation_id: fixture.request.operation_id,
            plan_hash: fixture.request.plan_hash,
            group_placement: member.group_placement,
            member_path: member.member_path,
        },
        release_set: fixture.request.batch.active_release_set,
        progress: RootComponentAllocationProgressView::Reserved,
    };
    let advanced = RootComponentProvisioningOps::mark_member_reserved(request, &allocation)
        .expect("commit member reservation");
    assert_eq!(advanced.reservation_cursor.reserved_component_count, 1);
    assert_eq!(advanced.reservation_cursor.placement_index, 1);
    assert_eq!(advanced.reservation_cursor.member_index, 0);
    assert_eq!(
        RootComponentProvisioningOps::reservation_disposition(request, &advanced)
            .expect("response-loss replay"),
        RootComponentProvisioningReservationDisposition::Replay
    );
    let complete = RootComponentProvisioningAdvanceRequest {
        expected_reserved_component_count: 1,
        ..request
    };
    assert_eq!(
        RootComponentProvisioningOps::reservation_disposition(complete, &advanced)
            .expect("terminal disposition"),
        RootComponentProvisioningReservationDisposition::Complete
    );

    let snapshot = RootComponentProvisioningStore::export();
    RootComponentProvisioningStore::import(snapshot);
    let restored = RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
    })
    .expect("restored reservation progress");
    assert_eq!(restored, advanced);
}

#[test]
fn reservation_cursor_crosses_canonical_placements_without_reusing_identity() {
    let mut fixture = fixture();
    let mut second_placement = fixture.request.batch.placements[0].clone();
    second_placement.group_placement.ordinal = 1;
    fixture.request.batch.placements.push(second_placement);
    fixture.validation.placement_count = 2;
    fixture.validation.component_count = 2;
    *fixture
        .validation
        .component_spec_counts
        .values_mut()
        .next()
        .expect("Component Spec count") = 2;

    let mut current =
        RootComponentProvisioningOps::accept(fixture.request.clone(), &fixture.validation, 100)
            .expect("accept two-placement batch");
    let mut operation_ids = BTreeSet::new();
    for expected in 0..2 {
        let request = RootComponentProvisioningAdvanceRequest {
            operation_id: fixture.request.operation_id,
            plan_hash: fixture.request.plan_hash,
            expected_reserved_component_count: expected,
        };
        let member = RootComponentProvisioningOps::next_member_reservation(&current)
            .expect("next placement member");
        assert_eq!(member.group_placement.ordinal, expected);
        assert!(operation_ids.insert(member.member_operation_id));
        let allocation = RootComponentAllocationView {
            operation_id: member.member_operation_id,
            allocation_sequence: u64::from(expected) + 1,
            component: ComponentInstanceId::from_generated_bytes(
                [u8::try_from(expected).expect("small ordinal") + 40; 32],
            ),
            component_spec: member.component_spec.clone(),
            spec_hash: member.spec_hash,
            role: CanisterRole::new("alpha"),
            provisioning_origin: ComponentProvisioningOrigin::ComponentGroup {
                operation_id: fixture.request.operation_id,
                plan_hash: fixture.request.plan_hash,
                group_placement: member.group_placement,
                member_path: member.member_path,
            },
            release_set: fixture.request.batch.active_release_set,
            progress: RootComponentAllocationProgressView::Reserved,
        };
        current = RootComponentProvisioningOps::mark_member_reserved(request, &allocation)
            .expect("advance placement member");
    }
    assert_eq!(current.reservation_cursor.reserved_component_count, 2);
    assert_eq!(current.reservation_cursor.placement_index, 2);
    assert_eq!(current.reservation_cursor.member_index, 0);
}

#[test]
fn conflicting_operation_and_active_batch_reject_without_new_reservations() {
    let fixture = fixture();
    RootComponentProvisioningOps::accept(fixture.request.clone(), &fixture.validation, 100)
        .expect("accept batch");

    let mut conflicting = fixture.request.clone();
    conflicting.plan_hash = [13; 32];
    assert!(RootComponentProvisioningOps::acceptance_replay(&conflicting).is_err());

    let mut second = fixture.request;
    second.operation_id = [14; 32];
    second.plan_hash = [15; 32];
    second.batch.placements[0].group_placement.ordinal = 1;
    assert!(RootComponentProvisioningOps::require_acceptance_open(second.operation_id).is_err());
    assert!(RootComponentProvisioningOps::accept(second, &fixture.validation, 101).is_err());
    assert_eq!(
        RootComponentProvisioningOps::tracked_group_placements().expect("placement count"),
        1
    );
}

#[test]
fn corrupted_receipt_index_or_aggregate_state_fails_closed() {
    let fixture = fixture();
    RootComponentProvisioningOps::accept(fixture.request.clone(), &fixture.validation, 100)
        .expect("accept batch");
    let exact = RootComponentProvisioningStore::export();

    let mut corrupted = exact.clone();
    let state = &mut corrupted.operations[0].state;
    let RootComponentProvisioningStateRecordPhase::Accepted {
        receipt_content_hash,
        ..
    } = state;
    *receipt_content_hash = [99; 32];
    RootComponentProvisioningStore::import(corrupted);
    assert!(RootComponentProvisioningOps::acceptance_replay(&fixture.request).is_err());

    let mut corrupted = exact.clone();
    corrupted.placements[0].1.plan_hash = [98; 32];
    RootComponentProvisioningStore::import(corrupted);
    assert!(RootComponentProvisioningOps::acceptance_replay(&fixture.request).is_err());

    let mut corrupted = exact.clone();
    corrupted.state.tracked_group_placements = 2;
    RootComponentProvisioningStore::import(corrupted);
    assert!(RootComponentProvisioningOps::acceptance_replay(&fixture.request).is_err());

    let mut corrupted = exact;
    let RootComponentProvisioningStateRecordPhase::Accepted {
        reservation_cursor, ..
    } = &mut corrupted.operations[0].state;
    reservation_cursor.content_hash = [97; 32];
    RootComponentProvisioningStore::import(corrupted);
    assert!(RootComponentProvisioningOps::acceptance_replay(&fixture.request).is_err());
}

#[test]
fn member_origin_is_valid_only_under_exact_accepted_authority() {
    let fixture = fixture();
    RootComponentProvisioningOps::accept(fixture.request.clone(), &fixture.validation, 100)
        .expect("accept batch");
    let placement = &fixture.request.batch.placements[0];
    let entry = &placement.entries[0];
    let origin = ComponentProvisioningOrigin::ComponentGroup {
        operation_id: fixture.request.operation_id,
        plan_hash: fixture.request.plan_hash,
        group_placement: placement.group_placement.clone(),
        member_path: entry.member_path.clone(),
    };
    RootComponentProvisioningOps::validate_member_provisioning_origin(
        &origin,
        &entry.component_spec,
        entry.spec_hash,
    )
    .expect("retained origin");
    let mut wrong_origin = origin;
    let ComponentProvisioningOrigin::ComponentGroup { member_path, .. } = &mut wrong_origin else {
        unreachable!("constructed group origin")
    };
    *member_path = ComponentGroupMemberPath::try_from(vec!["other".parse().expect("member ID")])
        .expect("member path");
    assert!(
        RootComponentProvisioningOps::validate_member_provisioning_origin(
            &wrong_origin,
            &entry.component_spec,
            entry.spec_hash,
        )
        .is_err()
    );
}
