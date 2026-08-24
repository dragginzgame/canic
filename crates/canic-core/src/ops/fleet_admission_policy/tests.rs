//! Module: ops::fleet_admission_policy::tests
//!
//! Responsibility: qualify canonical template and Fleet-bound policy identities.
//! Does not own: protected-input parsing, persistence, mutation, or endpoints.

use super::*;
use crate::ids::{
    AppId, CanonicalNetworkId, FleetAdmissionRule, FleetAdmissionSelector, FleetAdmissionTarget,
    FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority, SubnetId,
};
use crate::model::fleet_admission_authority::{
    FleetAdmissionMutationActionModel, FleetAdmissionMutationRequestModel,
    FleetAdmissionRootCatalogAuthorityModel,
};
use crate::model::fleet_admission_projection::{
    FleetAdmissionProjectionPhaseModel, FleetAdmissionProjectionState,
};

fn principal(index: u8) -> Principal {
    Principal::from_slice(&[index; 29])
}

fn fleet(byte: u8) -> FleetBinding {
    FleetBinding {
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([byte; 32]),
        },
        app: AppId::from("demo"),
    }
}

fn template() -> FleetAdmissionPolicyTemplate {
    compile_fleet_admission_policy_template(
        vec![principal(1), principal(2)],
        vec![FleetAdmissionRule {
            selector: FleetAdmissionSelector::ComponentSpec(
                "core".parse().expect("Component Spec ID"),
            ),
            principals: vec![principal(1)],
        }],
    )
    .expect("valid template")
}

#[test]
fn template_and_installed_policy_digests_bind_every_authority_field() {
    let template = template();
    validate_fleet_admission_policy_template(&template).expect("template validates");
    let policy = bind_initial_fleet_admission_policy(fleet(7), &template).expect("bind policy");
    validate_installed_fleet_admission_policy(&policy).expect("policy validates");

    let other_fleet =
        bind_initial_fleet_admission_policy(fleet(8), &template).expect("bind policy");
    assert_ne!(policy.policy_digest, other_fleet.policy_digest);

    let mut tampered = policy;
    tampered.fleet_principals.push(principal(3));
    assert_eq!(
        validate_installed_fleet_admission_policy(&tampered),
        Err(FleetAdmissionPolicyValidationError::DigestMismatch)
    );
}

#[test]
fn template_digest_changes_with_narrowing_authority() {
    let baseline = template();
    let changed = compile_fleet_admission_policy_template(
        vec![principal(1), principal(2)],
        vec![FleetAdmissionRule {
            selector: FleetAdmissionSelector::ComponentSpec(
                "core".parse().expect("Component Spec ID"),
            ),
            principals: vec![principal(2)],
        }],
    )
    .expect("changed template");

    assert_ne!(baseline.template_digest, changed.template_digest);
}

#[test]
fn empty_participant_catalogs_retain_nonzero_exact_identities() {
    let root_digest = fleet_admission_root_participant_catalog_digest(&[]);
    let fleet_digest =
        fleet_admission_participant_catalog_digest(&[FleetAdmissionRootCatalogAuthorityModel {
            fleet_subnet_root: principal(20),
            participant_catalog_digest: root_digest,
            participant_count: 0,
        }]);

    assert_ne!(root_digest, [0; 32]);
    assert_ne!(fleet_digest, [0; 32]);
    assert_ne!(root_digest, fleet_digest);
}

#[test]
fn template_projection_digest_binds_target_and_effective_members() {
    let template = template();
    let target = FleetAdmissionTarget {
        component_spec: "core".parse().expect("Component Spec ID"),
        component_instance: None,
        fleet_subnet_root: SubnetId::from_principal(principal(7)),
    };
    let baseline = fleet_admission_template_projection_digest(
        template.template_digest,
        &target,
        &[principal(1)],
    );
    let other_members = fleet_admission_template_projection_digest(
        template.template_digest,
        &target,
        &[principal(2)],
    );
    let other_target = fleet_admission_template_projection_digest(
        template.template_digest,
        &FleetAdmissionTarget {
            fleet_subnet_root: SubnetId::from_principal(principal(8)),
            ..target
        },
        &[principal(1)],
    );

    assert_ne!(baseline, other_members);
    assert_ne!(baseline, other_target);
}

#[test]
fn installed_projection_binds_exact_target_and_effective_intersection() {
    let target = crate::test::support::managed_component_binding();
    let target_fleet = projection_target_authority(&target).fleet.clone();
    let template = compile_fleet_admission_policy_template(
        vec![principal(1), principal(2)],
        vec![FleetAdmissionRule {
            selector: FleetAdmissionSelector::ComponentSpec(
                "default".parse().expect("Component Spec ID"),
            ),
            principals: vec![principal(1)],
        }],
    )
    .expect("projection policy template");
    let policy =
        bind_initial_fleet_admission_policy(target_fleet, &template).expect("installed policy");
    let principals =
        crate::domain::policy::pure::fleet_admission::effective_fleet_admission_principals(
            &policy,
            &fleet_admission_target_for_binding(&target),
        );
    let projection = materialize_fleet_admission_projection(&policy, target.clone(), principals)
        .expect("exact projection");

    assert_eq!(projection.principals, vec![principal(1)]);
    validate_installed_fleet_admission_projection(&projection, &target)
        .expect("projection validates");

    let mut substituted = target;
    match &mut substituted {
        ManagedCanisterBinding::Component(binding) => binding.canister_id = principal(99),
        ManagedCanisterBinding::ComponentChild(_) => unreachable!(),
    }
    assert_eq!(
        validate_installed_fleet_admission_projection(&projection, &substituted),
        Err(FleetAdmissionProjectionValidationError::TargetMismatch)
    );

    let mut tampered = projection;
    tampered.principals.push(principal(2));
    assert_eq!(
        validate_installed_fleet_admission_projection(&tampered, &tampered.target),
        Err(FleetAdmissionProjectionValidationError::ProjectionDigestMismatch)
    );
}

#[test]
fn mutation_request_digest_binds_every_authority_field() {
    let mut request = FleetAdmissionMutationRequestModel {
        authority: FleetCoordinatorBinding {
            fleet: fleet(7),
            coordinator_subnet: SubnetId::from_principal(principal(8)),
            coordinator: principal(9),
        },
        expected_generation: 3,
        expected_policy_digest: [4; 32],
        action: FleetAdmissionMutationActionModel::Add,
        selector: FleetAdmissionSelector::Fleet,
        principal: principal(5),
        operation_id: [6; 32],
        successor_policy_digest: [7; 32],
        participant_catalog_digest: [18; 32],
        participant_count: 1,
    };
    let baseline = fleet_admission_mutation_request_digest(&request);

    let mut digests = Vec::new();
    request.authority.fleet = fleet(10);
    digests.push(fleet_admission_mutation_request_digest(&request));
    request.authority.fleet = fleet(7);
    request.authority.coordinator_subnet = SubnetId::from_principal(principal(11));
    digests.push(fleet_admission_mutation_request_digest(&request));
    request.authority.coordinator_subnet = SubnetId::from_principal(principal(8));
    request.authority.coordinator = principal(12);
    digests.push(fleet_admission_mutation_request_digest(&request));
    request.authority.coordinator = principal(9);
    request.expected_generation = 13;
    digests.push(fleet_admission_mutation_request_digest(&request));
    request.expected_generation = 3;
    request.expected_policy_digest = [14; 32];
    digests.push(fleet_admission_mutation_request_digest(&request));
    request.expected_policy_digest = [4; 32];
    request.action = FleetAdmissionMutationActionModel::Remove;
    digests.push(fleet_admission_mutation_request_digest(&request));
    request.action = FleetAdmissionMutationActionModel::Add;
    request.selector =
        FleetAdmissionSelector::ComponentSpec("core".parse().expect("Component Spec ID"));
    digests.push(fleet_admission_mutation_request_digest(&request));
    request.selector = FleetAdmissionSelector::Fleet;
    request.principal = principal(15);
    digests.push(fleet_admission_mutation_request_digest(&request));
    request.principal = principal(5);
    request.operation_id = [16; 32];
    digests.push(fleet_admission_mutation_request_digest(&request));
    request.operation_id = [6; 32];
    request.successor_policy_digest = [17; 32];
    digests.push(fleet_admission_mutation_request_digest(&request));
    request.successor_policy_digest = [7; 32];
    request.participant_count = 2;
    digests.push(fleet_admission_mutation_request_digest(&request));

    assert!(digests.into_iter().all(|digest| digest != baseline));
}

#[test]
fn mutation_operation_identity_binds_registry_and_exact_successor() {
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: fleet(7),
            coordinator_subnet: SubnetId::from_principal(principal(8)),
            coordinator: principal(9),
        },
        epoch: 10,
    };
    let registry = FleetRegistryVersion {
        authority,
        revision: 11,
        content_hash: [12; 32],
    };
    let catalogs = vec![FleetAdmissionRootCatalogAuthorityModel {
        fleet_subnet_root: principal(18),
        participant_catalog_digest: [19; 32],
        participant_count: 1,
    }];
    let catalog_digest = fleet_admission_participant_catalog_digest(&catalogs);
    let input = FleetAdmissionMutationOperationInput {
        expected_generation: 13,
        expected_policy_digest: [14; 32],
        action: FleetAdmissionMutationActionModel::Add,
        selector: FleetAdmissionSelector::Fleet,
        principal: principal(15),
        successor_policy_digest: [16; 32],
        participant_catalog_digest: catalog_digest,
        participant_count: 1,
    };
    let operation = fleet_admission_mutation_operation_id(&registry, &input);
    let mut changed = registry.clone();
    changed.revision += 1;
    assert_ne!(
        operation,
        fleet_admission_mutation_operation_id(&changed, &input)
    );
    let mut changed_input = input.clone();
    changed_input.action = FleetAdmissionMutationActionModel::Remove;
    assert_ne!(
        operation,
        fleet_admission_mutation_operation_id(&registry, &changed_input)
    );
    changed_input = input.clone();
    changed_input.successor_policy_digest = [17; 32];
    assert_ne!(
        operation,
        fleet_admission_mutation_operation_id(&registry, &changed_input)
    );
    let mut changed_catalogs = catalogs;
    changed_catalogs[0].participant_count = 2;
    changed_input = input;
    changed_input.participant_catalog_digest =
        fleet_admission_participant_catalog_digest(&changed_catalogs);
    changed_input.participant_count = 2;
    assert_ne!(
        operation,
        fleet_admission_mutation_operation_id(&registry, &changed_input)
    );
}

#[test]
fn activate_request_reconstructs_an_exact_response_loss_replay() {
    let target = crate::test::support::managed_component_binding();
    let active = crate::test::support::fleet_admission_projection(target.clone());
    let successor_policy = compile_installed_fleet_admission_policy(
        active.authority.fleet.clone(),
        active.generation + 1,
        vec![principal(1), principal(2)],
        Vec::new(),
    )
    .expect("successor policy");
    let successor =
        crate::workflow::fleet_admission_projection::compile_fleet_admission_projection(
            &successor_policy,
            target,
        )
        .expect("successor projection");
    let operation_id = [0x81; 32];
    let request = FleetAdmissionActivateTargetRequest {
        operation_id,
        expected_generation: active.generation,
        expected_policy_digest: active.policy_digest,
        successor_generation: successor.generation,
        successor_policy_digest: successor.policy_digest,
        successor_projection_digest: successor.projection_digest,
    };
    let prepared_state = FleetAdmissionProjectionState {
        schema_version: 1,
        active,
        prepared: Some(successor),
        phase: FleetAdmissionProjectionPhaseModel::Fenced,
        last_receipt: Some(
            crate::model::fleet_admission_projection::FleetAdmissionProjectionReceiptModel {
                operation_id,
                phase: FleetAdmissionTargetTransitionPhaseModel::Prepare,
                request_hash: [0x82; 32],
                receipt_hash: [0x83; 32],
            },
        ),
    };
    let compiled = fleet_admission_activate_target_request(&prepared_state, request.clone())
        .expect("compile first activation");
    let activated = crate::domain::policy::pure::fleet_admission_projection::transition_fleet_admission_projection(
        &prepared_state,
        compiled,
    )
    .expect("activate target");

    let replay = fleet_admission_activate_target_request(&activated.state, request)
        .expect("compile response-loss replay");
    assert!(
        crate::domain::policy::pure::fleet_admission_projection::transition_fleet_admission_projection(
            &activated.state,
            replay,
        )
        .expect("replay activation")
        .replayed
    );
}
