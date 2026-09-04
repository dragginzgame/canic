//! Focused stable-journal replay proof for one Root and one managed participant.

use super::*;
use crate::storage::stable::root_admission::{
    RootAdmissionData, RootAdmissionStateRecord, RootAdmissionStore,
};
use crate::test_support::{fleet_admission_policy, fleet_subnet_root_funding_authority};
use canic_core::{
    cdk::structures::storable::Storable,
    cdk::types::Cycles,
    ids::{
        AppId, CanisterRole, CanonicalNetworkId, ComponentBinding, ComponentInstanceId,
        ComponentSpecId, ComponentTopologyDigest, CyclesFundingBudget, FleetBinding,
        FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority,
        FleetSubnetCanisterPoolConfig, FleetSubnetRootLimits, ManagedCanisterBinding, SubnetId,
    },
    shared_support::fleet_admission_policy::{
        compile_installed_fleet_admission_policy, expected_fleet_admission_target_receipt,
        materialize_fleet_admission_projection,
    },
};

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one stable journal replay across every phase"
)]
fn stable_root_journal_replays_every_phase_without_a_second_target_effect() {
    RootAdmissionStore::import(RootAdmissionData::default());
    let target = managed_component_binding();
    let root = root_for(&target);
    let active = fleet_admission_policy(root.authority.binding.fleet.clone());
    let successor = compile_installed_fleet_admission_policy(
        active.fleet.clone(),
        2,
        active.fleet_principals.clone(),
        Vec::new(),
    )
    .expect("successor");
    let projection = materialize_fleet_admission_projection(
        &successor,
        target,
        successor.fleet_principals.clone(),
    )
    .expect("projection");
    let prepare = FleetAdmissionPrepareRootRequest {
        authority: root.authority.binding.clone(),
        operation_id: [41; 32],
        expected_generation: active.generation,
        expected_policy_digest: active.policy_digest,
        successor: successor.clone(),
        stage: FleetAdmissionPrepareRootStage::Reserve,
    };

    assert_eq!(
        RootAdmissionOps::prepare(&root, active, prepare.clone(), vec![projection])
            .expect("reserve")
            .expect("reservation receipt")
            .phase,
        FleetAdmissionRootTransitionPhase::Preparing
    );
    assert!(RootAdmissionOps::require_catalog_mutation_allowed(&root).is_err());
    let mut fence = prepare.clone();
    fence.stage = FleetAdmissionPrepareRootStage::Fence;
    assert!(
        RootAdmissionOps::prepare(&root, successor.clone(), fence.clone(), Vec::new())
            .expect("start fencing")
            .is_none()
    );
    reload_root_admission();
    let (expected, RootAdmissionStep::Prepare { projection }) =
        RootAdmissionOps::next_step(&root).expect("prepare step")
    else {
        panic!("expected prepare step");
    };
    let receipt = expected_fleet_admission_target_receipt(
        prepare.operation_id,
        FleetAdmissionTargetTransitionPhase::Prepare,
        prepare.expected_generation,
        prepare.expected_policy_digest,
        projection.clone(),
    )
    .expect("prepare receipt");
    RootAdmissionOps::record_target_receipt(
        &root,
        &expected,
        projection,
        FleetAdmissionTargetTransitionPhase::Prepare,
        receipt,
    )
    .expect("record prepare");
    reload_root_admission();
    assert_eq!(
        RootAdmissionOps::prepare(&root, successor.clone(), fence, Vec::new())
            .expect("fence replay")
            .expect("perimeter receipt")
            .phase,
        FleetAdmissionRootTransitionPhase::PerimeterFenced
    );

    let activate = FleetAdmissionActivateRootRequest {
        authority: prepare.authority.clone(),
        operation_id: prepare.operation_id,
        expected_generation: prepare.expected_generation,
        expected_policy_digest: prepare.expected_policy_digest,
        successor_generation: successor.generation,
        successor_policy_digest: successor.policy_digest,
    };
    assert!(
        RootAdmissionOps::activate(&root, activate.clone())
            .expect("activate")
            .is_none()
    );
    reload_root_admission();
    let (expected, RootAdmissionStep::Activate { projection }) =
        RootAdmissionOps::next_step(&root).expect("activate step")
    else {
        panic!("expected activate step");
    };
    let receipt = expected_fleet_admission_target_receipt(
        prepare.operation_id,
        FleetAdmissionTargetTransitionPhase::Activate,
        prepare.expected_generation,
        prepare.expected_policy_digest,
        projection.clone(),
    )
    .expect("activate receipt");
    RootAdmissionOps::record_target_receipt(
        &root,
        &expected,
        projection,
        FleetAdmissionTargetTransitionPhase::Activate,
        receipt,
    )
    .expect("record activate");
    reload_root_admission();
    assert_eq!(
        RootAdmissionOps::activate(&root, activate)
            .expect("activate replay")
            .expect("opening receipt")
            .phase,
        FleetAdmissionRootTransitionPhase::Opening
    );

    let open = FleetAdmissionOpenRootRequest {
        authority: prepare.authority,
        operation_id: prepare.operation_id,
        generation: successor.generation,
        policy_digest: successor.policy_digest,
    };
    assert!(
        RootAdmissionOps::open(&root, open.clone())
            .expect("open")
            .is_none()
    );
    reload_root_admission();
    let (expected, RootAdmissionStep::Open { projection }) =
        RootAdmissionOps::next_step(&root).expect("open step")
    else {
        panic!("expected open step");
    };
    let receipt = expected_fleet_admission_target_receipt(
        prepare.operation_id,
        FleetAdmissionTargetTransitionPhase::Open,
        successor.generation,
        successor.policy_digest,
        projection.clone(),
    )
    .expect("open receipt");
    RootAdmissionOps::record_target_receipt(
        &root,
        &expected,
        projection,
        FleetAdmissionTargetTransitionPhase::Open,
        receipt,
    )
    .expect("record open");
    reload_root_admission();
    let (expected, RootAdmissionStep::Complete) =
        RootAdmissionOps::next_step(&root).expect("complete step")
    else {
        panic!("expected complete step");
    };
    let completed = RootAdmissionOps::complete(&root, &expected).expect("complete");
    reload_root_admission();
    assert!(RootAdmissionOps::require_catalog_mutation_allowed(&root).is_ok());
    assert_eq!(
        completed.phase,
        FleetAdmissionRootTransitionPhase::Converged
    );
    assert_eq!(
        RootAdmissionOps::open(&root, open).expect("terminal replay"),
        Some(completed)
    );
}

#[test]
fn restored_root_journal_rejects_substituted_hashes_and_policy() {
    let root = seed_preparing_journal();
    let mut corrupted = RootAdmissionStore::export();
    corrupted
        .current
        .as_mut()
        .expect("Root journal")
        .current_transition
        .as_mut()
        .expect("current transition")
        .request
        .request_hash = [0x91; 32];
    RootAdmissionStore::import(corrupted);
    assert!(RootAdmissionOps::next_step(&root).is_err());

    let root = seed_preparing_journal();
    let mut corrupted = RootAdmissionStore::export();
    corrupted
        .current
        .as_mut()
        .expect("Root journal")
        .current_transition
        .as_mut()
        .expect("current transition")
        .participant_catalog_digest = [0x92; 32];
    RootAdmissionStore::import(corrupted);
    assert!(RootAdmissionOps::next_step(&root).is_err());

    let root = seed_preparing_journal();
    let mut corrupted = RootAdmissionStore::export();
    corrupted
        .current
        .as_mut()
        .expect("Root journal")
        .current_transition
        .as_mut()
        .expect("current transition")
        .request
        .successor
        .policy_digest = [0x93; 32];
    RootAdmissionStore::import(corrupted);
    assert!(RootAdmissionOps::next_step(&root).is_err());
}

#[test]
fn active_transition_status_reads_the_retained_journal_without_live_catalog_access() {
    let root = seed_preparing_journal();
    let active = fleet_admission_policy(root.authority.binding.fleet.clone());
    assert!(
        !RootAdmissionOps::status_requires_live_catalog(&root, active.clone())
            .expect("inspect status source")
    );
    let status = RootAdmissionOps::status(
        &root,
        active,
        Vec::new(),
        PageRequest {
            offset: 0,
            limit: 1,
        },
    )
    .expect("read retained transition status");
    assert_eq!(status.operation_id, Some([0x90; 32]));
    assert_eq!(
        status.phase,
        Some(FleetAdmissionRootTransitionPhase::Preparing)
    );
    assert_eq!(status.participants.total, 1);
}

#[test]
fn stale_catalog_reservation_releases_idempotently_before_fencing() {
    RootAdmissionStore::import(RootAdmissionData::default());
    let target = managed_component_binding();
    let root = root_for(&target);
    let active = fleet_admission_policy(root.authority.binding.fleet.clone());
    let successor = compile_installed_fleet_admission_policy(
        active.fleet.clone(),
        active.generation + 1,
        active.fleet_principals.clone(),
        Vec::new(),
    )
    .expect("successor");
    let projection = materialize_fleet_admission_projection(
        &successor,
        target,
        successor.fleet_principals.clone(),
    )
    .expect("projection");
    let mut request = FleetAdmissionPrepareRootRequest {
        authority: root.authority.binding.clone(),
        operation_id: [0x94; 32],
        expected_generation: active.generation,
        expected_policy_digest: active.policy_digest,
        successor,
        stage: FleetAdmissionPrepareRootStage::Reserve,
    };
    assert_eq!(
        RootAdmissionOps::prepare(&root, active.clone(), request.clone(), vec![projection])
            .expect("reserve")
            .expect("reservation receipt")
            .phase,
        FleetAdmissionRootTransitionPhase::Preparing
    );
    assert!(RootAdmissionOps::require_catalog_mutation_allowed(&root).is_err());

    request.stage = FleetAdmissionPrepareRootStage::Release;
    let released = RootAdmissionOps::prepare(&root, active.clone(), request.clone(), Vec::new())
        .expect("release")
        .expect("release receipt");
    assert_eq!(released.phase, FleetAdmissionRootTransitionPhase::Released);
    assert!(RootAdmissionOps::require_catalog_mutation_allowed(&root).is_ok());
    reload_root_admission();
    assert_eq!(
        RootAdmissionOps::prepare(&root, active, request, Vec::new())
            .expect("release replay")
            .expect("retained release receipt"),
        released
    );
}

fn seed_preparing_journal() -> FleetSubnetRootBinding {
    RootAdmissionStore::import(RootAdmissionData::default());
    let target = managed_component_binding();
    let root = root_for(&target);
    let active = fleet_admission_policy(root.authority.binding.fleet.clone());
    let successor = compile_installed_fleet_admission_policy(
        active.fleet.clone(),
        active.generation + 1,
        active.fleet_principals.clone(),
        Vec::new(),
    )
    .expect("successor");
    let projection = materialize_fleet_admission_projection(
        &successor,
        target,
        successor.fleet_principals.clone(),
    )
    .expect("projection");
    let prepare = FleetAdmissionPrepareRootRequest {
        authority: root.authority.binding.clone(),
        operation_id: [0x90; 32],
        expected_generation: active.generation,
        expected_policy_digest: active.policy_digest,
        successor,
        stage: FleetAdmissionPrepareRootStage::Reserve,
    };
    assert_eq!(
        RootAdmissionOps::prepare(&root, active, prepare, vec![projection])
            .expect("seed Root transition")
            .expect("reservation receipt")
            .phase,
        FleetAdmissionRootTransitionPhase::Preparing
    );
    reload_root_admission();
    root
}

fn reload_root_admission() {
    let state = RootAdmissionStateRecord {
        current: RootAdmissionStore::export().current,
    };
    let restored = RootAdmissionStateRecord::from_bytes(state.to_bytes());
    RootAdmissionStore::import(RootAdmissionData {
        current: restored.current,
    });
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
        component_topology_digest: ComponentTopologyDigest::from_bytes([42; 32]),
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 8,
            maximum_registry_bytes: 1_000_000,
            maximum_wasm_store_bytes: 1_000_000,
            canister_pool: FleetSubnetCanisterPoolConfig {
                minimum_size: 1,
                maximum_size: 8,
                canister_cycles: Cycles::new(1_000_000_000_000),
                creation_execution_margin: Cycles::new(1_000_000_000_000),
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

fn managed_component_binding() -> ManagedCanisterBinding {
    let fleet = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([43; 32]),
        },
        app: AppId::from("root-admission-test"),
    };
    let placement_subnet = SubnetId::from_principal(candid::Principal::from_slice(&[44; 29]));
    ManagedCanisterBinding::Component(ComponentBinding {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet,
                coordinator_subnet: SubnetId::from_principal(candid::Principal::from_slice(
                    &[45; 29],
                )),
                coordinator: candid::Principal::from_slice(&[46; 29]),
            },
            epoch: 1,
        },
        component: ComponentInstanceId::from_generated_bytes([47; 32]),
        component_spec: ComponentSpecId::try_from(String::from("app")).expect("Component Spec ID"),
        spec_hash: [48; 32],
        role: CanisterRole::from("app"),
        placement_subnet,
        fleet_subnet_root: candid::Principal::from_slice(&[49; 29]),
        canister_id: candid::Principal::from_slice(&[50; 29]),
    })
}
