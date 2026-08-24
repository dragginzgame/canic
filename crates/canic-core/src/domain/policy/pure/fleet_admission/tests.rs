//! Module: domain::policy::pure::fleet_admission::tests
//!
//! Responsibility: qualify deterministic intersection for exact managed targets.
//! Does not own: policy compilation, validation, persistence, or caller acquisition.

use super::*;
use crate::ids::{
    AppId, CanonicalNetworkId, ComponentInstanceId, ComponentSpecId, FleetAdmissionRule,
    FleetAdmissionSelector, FleetBinding, FleetId, FleetKey, SubnetId,
};
use crate::model::fleet_admission_authority::FleetAdmissionMutationActionModel;
use crate::model::fleet_admission_authority::{
    FLEET_ADMISSION_AUTHORITY_SCHEMA_VERSION, FleetAdmissionAuthorityState,
    FleetAdmissionCoordinatorRootPhaseModel, FleetAdmissionCoordinatorRootProgressModel,
    FleetAdmissionMutationOutcomeModel, FleetAdmissionMutationRequestModel,
};

fn principal(index: u8) -> Principal {
    Principal::from_slice(&[index; 29])
}

#[test]
fn every_matching_scope_intersects_without_widening() {
    let component_spec = "core"
        .parse::<ComponentSpecId>()
        .expect("Component Spec ID");
    let component_instance = ComponentInstanceId::from_generated_bytes([7; 32]);
    let root = SubnetId::from_principal(principal(9));
    let policy = FleetAdmissionPolicy {
        schema_version: 1,
        fleet: FleetBinding {
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: FleetId::from_generated_bytes([8; 32]),
            },
            app: AppId::from("demo"),
        },
        generation: 1,
        fleet_principals: vec![principal(1), principal(2), principal(3), principal(4)],
        rules: vec![
            FleetAdmissionRule {
                selector: FleetAdmissionSelector::ComponentSpec(component_spec.clone()),
                principals: vec![principal(1), principal(2), principal(3)],
            },
            FleetAdmissionRule {
                selector: FleetAdmissionSelector::ComponentInstance(component_instance),
                principals: vec![principal(2), principal(3)],
            },
            FleetAdmissionRule {
                selector: FleetAdmissionSelector::FleetSubnetRoot(root),
                principals: vec![principal(2), principal(4)],
            },
        ],
        policy_digest: [0; 32],
    };
    let target = FleetAdmissionTarget {
        component_spec,
        component_instance: Some(component_instance),
        fleet_subnet_root: root,
    };

    assert_eq!(
        effective_fleet_admission_principals(&policy, &target),
        vec![principal(2)]
    );
}

#[test]
fn fleet_removal_is_global_and_cannot_leave_a_widening_reference() {
    let mut policy = policy_with_fleet_principals(vec![principal(1), principal(2)]);
    policy.rules = vec![FleetAdmissionRule {
        selector: FleetAdmissionSelector::ComponentSpec(component_spec()),
        principals: vec![principal(1), principal(2)],
    }];

    let mutation = mutate_fleet_admission_membership(
        &policy,
        FleetAdmissionMutationActionModel::Remove,
        &FleetAdmissionSelector::Fleet,
        principal(2),
    )
    .expect("global removal");

    assert_eq!(mutation.fleet_principals, vec![principal(1)]);
    assert_eq!(mutation.rules[0].principals, vec![principal(1)]);
}

#[test]
fn narrower_remove_creates_a_restriction_and_add_collapses_redundancy() {
    let policy = policy_with_fleet_principals(vec![principal(1), principal(2)]);
    let selector = FleetAdmissionSelector::ComponentSpec(component_spec());
    let removed = mutate_fleet_admission_membership(
        &policy,
        FleetAdmissionMutationActionModel::Remove,
        &selector,
        principal(2),
    )
    .expect("narrower removal");
    assert_eq!(removed.rules.len(), 1);
    assert_eq!(removed.rules[0].principals, vec![principal(1)]);

    let mut restricted = policy;
    restricted.rules = removed.rules;
    let restored = mutate_fleet_admission_membership(
        &restricted,
        FleetAdmissionMutationActionModel::Add,
        &selector,
        principal(2),
    )
    .expect("restore inherited Fleet membership");
    assert!(restored.rules.is_empty());
}

#[test]
fn mutation_bounds_fail_before_a_successor_can_be_retained() {
    let full =
        policy_with_fleet_principals((1..=u8::MAX).map(principal).chain([principal(0)]).collect());
    assert_eq!(full.fleet_principals.len(), 256);
    assert_eq!(
        mutate_fleet_admission_membership(
            &full,
            FleetAdmissionMutationActionModel::Add,
            &FleetAdmissionSelector::Fleet,
            Principal::from_slice(&[9; 28]),
        ),
        Err(FleetAdmissionMutationPolicyError::PrincipalCapacityExhausted)
    );

    let selector = FleetAdmissionSelector::ComponentSpec(component_spec());
    assert_eq!(
        mutate_fleet_admission_membership(
            &full,
            FleetAdmissionMutationActionModel::Remove,
            &selector,
            full.fleet_principals[0],
        ),
        Err(FleetAdmissionMutationPolicyError::RulePrincipalReferenceCapacityExhausted)
    );
}

#[test]
fn effective_mutation_retains_one_exact_planned_replay_and_blocks_overlap() {
    let active = policy_with_fleet_principals(vec![principal(1)]);
    let authority = authority(active.fleet.clone());
    let state = FleetAdmissionAuthorityState {
        schema_version: FLEET_ADMISSION_AUTHORITY_SCHEMA_VERSION,
        active_policy: active.clone(),
        current_transition: None,
        last_result: None,
    };
    let request = mutation_request(
        &authority,
        &active,
        FleetAdmissionMutationActionModel::Add,
        principal(2),
        [7; 32],
        [8; 32],
    );
    let mut successor = active.clone();
    successor.generation = 2;
    successor.fleet_principals.push(principal(2));
    successor.policy_digest = [8; 32];

    let planned = plan_fleet_admission_mutation(
        &state,
        &authority,
        request.clone(),
        [9; 32],
        successor.clone(),
        roots(),
    )
    .expect("plan effective mutation");
    assert_eq!(
        planned.response.outcome,
        FleetAdmissionMutationOutcomeModel::Planned
    );
    let replay = plan_fleet_admission_mutation(
        &planned.state,
        &authority,
        request.clone(),
        [9; 32],
        successor,
        roots(),
    )
    .expect("exact replay");
    assert!(replay.replayed);
    assert_eq!(replay.state, planned.state);

    assert_eq!(
        plan_fleet_admission_mutation(
            &planned.state,
            &authority,
            request,
            [10; 32],
            active.clone(),
            roots(),
        ),
        Err(FleetAdmissionAuthorityPolicyError::OperationConflict)
    );
    let overlapping = mutation_request(
        &authority,
        &active,
        FleetAdmissionMutationActionModel::Remove,
        principal(1),
        [11; 32],
        [12; 32],
    );
    assert_eq!(
        plan_fleet_admission_mutation(
            &planned.state,
            &authority,
            overlapping,
            [13; 32],
            active,
            roots(),
        ),
        Err(FleetAdmissionAuthorityPolicyError::OperationInProgress)
    );
}

#[test]
fn effective_mutation_accepts_an_exact_zero_participant_catalog() {
    let active = policy_with_fleet_principals(vec![principal(1)]);
    let authority = authority(active.fleet.clone());
    let state = FleetAdmissionAuthorityState {
        schema_version: FLEET_ADMISSION_AUTHORITY_SCHEMA_VERSION,
        active_policy: active.clone(),
        current_transition: None,
        last_result: None,
    };
    let mut request = mutation_request(
        &authority,
        &active,
        FleetAdmissionMutationActionModel::Add,
        principal(2),
        [7; 32],
        [8; 32],
    );
    request.participant_count = 0;
    request.participant_catalog_digest = participant_catalog_digest();
    let mut successor = active;
    successor.generation = 2;
    successor.fleet_principals.push(principal(2));
    successor.policy_digest = [8; 32];

    let planned =
        plan_fleet_admission_mutation(&state, &authority, request, [9; 32], successor, roots())
            .expect("plan mutation over an exact empty participant set");
    assert_eq!(
        planned.response.outcome,
        FleetAdmissionMutationOutcomeModel::Planned
    );
}

#[test]
fn idempotent_mutation_is_terminal_and_exactly_replayable() {
    let active = policy_with_fleet_principals(vec![principal(1)]);
    let authority = authority(active.fleet.clone());
    let state = FleetAdmissionAuthorityState {
        schema_version: FLEET_ADMISSION_AUTHORITY_SCHEMA_VERSION,
        active_policy: active.clone(),
        current_transition: None,
        last_result: None,
    };
    let request = mutation_request(
        &authority,
        &active,
        FleetAdmissionMutationActionModel::Add,
        principal(1),
        [14; 32],
        active.policy_digest,
    );
    let accepted = plan_fleet_admission_mutation(
        &state,
        &authority,
        request.clone(),
        [15; 32],
        active.clone(),
        Vec::new(),
    )
    .expect("idempotent add");
    assert_eq!(
        accepted.response.outcome,
        FleetAdmissionMutationOutcomeModel::AlreadyPresent
    );
    assert!(accepted.state.current_transition.is_none());
    assert!(accepted.state.last_result.is_some());

    let replay = plan_fleet_admission_mutation(
        &accepted.state,
        &authority,
        request,
        [15; 32],
        active,
        Vec::new(),
    )
    .expect("terminal exact replay");
    assert!(replay.replayed);
    assert_eq!(replay.state, accepted.state);
}

fn policy_with_fleet_principals(mut fleet_principals: Vec<Principal>) -> FleetAdmissionPolicy {
    fleet_principals.sort();
    FleetAdmissionPolicy {
        schema_version: 1,
        fleet: FleetBinding {
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: FleetId::from_generated_bytes([8; 32]),
            },
            app: AppId::from("demo"),
        },
        generation: 1,
        fleet_principals,
        rules: Vec::new(),
        policy_digest: [0; 32],
    }
}

fn component_spec() -> ComponentSpecId {
    "core".parse().expect("Component Spec ID")
}

fn roots() -> Vec<FleetAdmissionCoordinatorRootProgressModel> {
    vec![FleetAdmissionCoordinatorRootProgressModel {
        fleet_subnet_root: principal(20),
        placement_subnet: SubnetId::from_principal(principal(21)),
        phase: FleetAdmissionCoordinatorRootPhaseModel::Pending,
        participant_catalog_digest: None,
        participant_count: None,
        last_receipt_hash: None,
    }]
}

fn authority(fleet: FleetBinding) -> crate::ids::FleetCoordinatorBinding {
    crate::ids::FleetCoordinatorBinding {
        fleet,
        coordinator_subnet: SubnetId::from_principal(principal(21)),
        coordinator: principal(22),
    }
}

fn mutation_request(
    authority: &crate::ids::FleetCoordinatorBinding,
    active: &FleetAdmissionPolicy,
    action: FleetAdmissionMutationActionModel,
    principal: Principal,
    operation_id: [u8; 32],
    successor_policy_digest: [u8; 32],
) -> FleetAdmissionMutationRequestModel {
    FleetAdmissionMutationRequestModel {
        authority: authority.clone(),
        expected_generation: active.generation,
        expected_policy_digest: active.policy_digest,
        action,
        selector: FleetAdmissionSelector::Fleet,
        principal,
        operation_id,
        successor_policy_digest,
        participant_catalog_digest: participant_catalog_digest(),
        participant_count: 1,
    }
}

fn participant_catalog_digest() -> [u8; 32] {
    [26; 32]
}
