//! Focused ordered-replay tests for the Root-owned admission journal.

use super::*;
use crate::{
    cdk::types::Cycles,
    ids::{
        ComponentTopologyDigest, CyclesFundingBudget, FleetSubnetCanisterPoolConfig,
        FleetSubnetRootBinding, FleetSubnetRootLimits, ManagedCanisterBinding,
    },
    ops::fleet_admission_policy::compile_installed_fleet_admission_policy,
    test::support::{
        fleet_admission_policy, fleet_subnet_root_funding_authority, managed_component_binding,
    },
};

#[test]
fn prepare_activate_open_is_monotonic_and_exactly_replayable() {
    let (target, request, state, participant) = transition_fixture();

    let reserved =
        prepare_fleet_admission_root(&state, request.clone(), [4; 32], [6; 32], vec![participant])
            .expect("reserve");
    assert!(!reserved.replayed);
    assert_eq!(reserved.receipt_hash, [6; 32]);
    let (fencing, receipt) =
        fence_fleet_admission_root(&reserved.state, request.operation_id, [13; 32], [6; 32])
            .expect("fence");
    assert!(receipt.is_none());
    let prepared = record_fleet_admission_root_participant(
        &fencing,
        request.operation_id,
        &target,
        FleetAdmissionRootParticipantPhaseModel::Prepared,
        [5; 32],
        [6; 32],
    )
    .expect("prepared receipt");
    assert_eq!(
        prepared
            .current_transition
            .as_ref()
            .map(|current| current.phase),
        Some(FleetAdmissionRootPhaseModel::PerimeterFenced)
    );
    assert_eq!(
        record_fleet_admission_root_participant(
            &prepared,
            request.operation_id,
            &target,
            FleetAdmissionRootParticipantPhaseModel::Prepared,
            [5; 32],
            [6; 32],
        )
        .expect("exact prepared replay"),
        prepared
    );

    let (activating, receipt) =
        activate_fleet_admission_root(&prepared, request.operation_id, [7; 32], [9; 32])
            .expect("activate");
    assert!(receipt.is_none());
    let activated = record_fleet_admission_root_participant(
        &activating,
        request.operation_id,
        &target,
        FleetAdmissionRootParticipantPhaseModel::Activated,
        [8; 32],
        [9; 32],
    )
    .expect("activated receipt");
    assert_eq!(
        activate_fleet_admission_root(&activated, request.operation_id, [7; 32], [9; 32])
            .expect("activate response-loss replay")
            .1,
        Some([9; 32])
    );

    let opening = open_fleet_admission_root(&activated, request.operation_id, [10; 32])
        .expect("open")
        .expect("current operation");
    let opened = record_fleet_admission_root_participant(
        &opening,
        request.operation_id,
        &target,
        FleetAdmissionRootParticipantPhaseModel::Open,
        [11; 32],
        [12; 32],
    )
    .expect("open receipt");
    let completed = complete_fleet_admission_root(&opened, [12; 32]).expect("complete");
    assert!(completed.current_transition.is_none());
    assert_eq!(completed.active_policy, request.successor);
    assert_eq!(
        open_fleet_admission_root(&completed, request.operation_id, [10; 32])
            .expect("terminal open replay"),
        None
    );
    let replay = prepare_fleet_admission_root(&completed, request, [4; 32], [6; 32], Vec::new())
        .expect("terminal prepare replay");
    assert!(replay.replayed);
    assert_eq!(replay.receipt_hash, [6; 32]);
}

#[test]
fn root_without_enrolled_targets_converges_through_the_same_durable_journal() {
    let (_target, request, state, _participant) = transition_fixture();
    let reserved =
        prepare_fleet_admission_root(&state, request.clone(), [4; 32], [6; 32], Vec::new())
            .expect("empty Root reservation");
    assert_eq!(reserved.receipt_hash, [6; 32]);
    assert_eq!(
        reserved
            .state
            .current_transition
            .as_ref()
            .map(|current| current.phase),
        Some(FleetAdmissionRootPhaseModel::Preparing)
    );
    let (prepared, receipt) =
        fence_fleet_admission_root(&reserved.state, request.operation_id, [13; 32], [6; 32])
            .expect("empty Root fence");
    assert_eq!(receipt, Some([6; 32]));
    assert_eq!(
        prepared
            .current_transition
            .as_ref()
            .map(|current| current.phase),
        Some(FleetAdmissionRootPhaseModel::PerimeterFenced)
    );

    let (activated, receipt) =
        activate_fleet_admission_root(&prepared, request.operation_id, [7; 32], [9; 32])
            .expect("empty Root activate");
    assert_eq!(receipt, Some([9; 32]));
    assert_eq!(
        activated
            .current_transition
            .as_ref()
            .map(|current| current.phase),
        Some(FleetAdmissionRootPhaseModel::Opening)
    );

    let opening = open_fleet_admission_root(&activated, request.operation_id, [10; 32])
        .expect("empty Root open")
        .expect("current operation");
    let completed =
        complete_fleet_admission_root(&opening, [12; 32]).expect("empty Root completion");
    assert!(completed.current_transition.is_none());
    assert_eq!(completed.active_policy, request.successor);
    assert!(
        completed
            .last_result
            .as_ref()
            .expect("retained empty Root result")
            .participants
            .is_empty()
    );
}

#[test]
fn first_excess_root_participant_rejects_before_reservation() {
    let (_target, request, state, participant) = transition_fixture();
    let participants = vec![participant; MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS + 1];
    assert_eq!(
        prepare_fleet_admission_root(&state, request, [4; 32], [6; 32], participants),
        Err(FleetAdmissionRootTransitionError::ParticipantCapacity)
    );
    assert!(state.current_transition.is_none());
}

#[test]
fn released_reservation_restore_rejects_predecessor_or_capacity_corruption() {
    let (_target, request, state, participant) = transition_fixture();
    let reserved =
        prepare_fleet_admission_root(&state, request.clone(), [4; 32], [6; 32], vec![participant])
            .expect("reserve");
    let (released, _) =
        release_fleet_admission_root(&reserved.state, request.operation_id, [7; 32], [8; 32])
            .expect("release reservation");
    validate_fleet_admission_root_state(&released).expect("valid released reservation");

    let mut wrong_predecessor = released.clone();
    wrong_predecessor
        .last_release
        .as_mut()
        .expect("released reservation")
        .request
        .expected_generation += 1;
    assert_eq!(
        validate_fleet_admission_root_state(&wrong_predecessor),
        Err(FleetAdmissionRootTransitionError::InvalidState)
    );

    let mut excessive_catalog = released;
    excessive_catalog
        .last_release
        .as_mut()
        .expect("released reservation")
        .participant_count =
        u32::try_from(MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS + 1).expect("first excess fits u32");
    assert_eq!(
        validate_fleet_admission_root_state(&excessive_catalog),
        Err(FleetAdmissionRootTransitionError::InvalidState)
    );
}

fn transition_fixture() -> (
    ManagedCanisterBinding,
    FleetAdmissionRootPrepareRequestModel,
    FleetAdmissionRootState,
    FleetAdmissionRootParticipantModel,
) {
    let target = managed_component_binding();
    let root = root_for(&target);
    let active = fleet_admission_policy(root.authority.binding.fleet.clone());
    let successor = compile_installed_fleet_admission_policy(
        active.fleet.clone(),
        2,
        vec![active.fleet_principals[0]],
        Vec::new(),
    )
    .expect("successor policy");
    let request = FleetAdmissionRootPrepareRequestModel {
        authority: root.authority.binding.clone(),
        root,
        operation_id: [1; 32],
        expected_generation: active.generation,
        expected_policy_digest: active.policy_digest,
        successor,
        request_hash: [2; 32],
    };
    let state = FleetAdmissionRootState {
        schema_version: FLEET_ADMISSION_ROOT_SCHEMA_VERSION,
        active_policy: active,
        current_transition: None,
        last_result: None,
        last_release: None,
    };
    let participant = FleetAdmissionRootParticipantModel {
        target: target.clone(),
        projection_digest: [3; 32],
        phase: FleetAdmissionRootParticipantPhaseModel::Pending,
        last_receipt_hash: None,
    };
    (target, request, state, participant)
}

fn root_for(target: &ManagedCanisterBinding) -> FleetSubnetRootBinding {
    let ManagedCanisterBinding::Component(component) = target else {
        panic!("fixture target is top-level");
    };
    FleetSubnetRootBinding {
        authority: component.authority.clone(),
        placement_subnet: component.placement_subnet,
        fleet_subnet_root: component.fleet_subnet_root,
        component_admissions: Vec::new(),
        component_topology_digest: ComponentTopologyDigest::from_bytes([13; 32]),
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 8,
            maximum_registry_bytes: 1_000_000,
            maximum_wasm_store_bytes: 1_000_000,
            canister_pool: FleetSubnetCanisterPoolConfig {
                minimum_size: 1,
                maximum_size: 8,
                canister_cycles: Cycles::new(1_000_000_000_000),
            },
            cycles_funding: CyclesFundingBudget {
                window_secs: 3_600,
                maximum_cycles: Cycles::new(1_000_000_000_000),
            },
            maximum_group_placements: 8,
        },
        funding: fleet_subnet_root_funding_authority(),
    }
}
