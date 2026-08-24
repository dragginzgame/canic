//! Focused parser and deterministic-plan identity tests for the admission CLI.

use super::*;
use canic_core::{
    dto::{
        fleet_admission::{FleetAdmissionOperationStatusResponse, FleetAdmissionPolicyStatus},
        page::Page,
    },
    ids::{
        AppId, CanisterRole, CanonicalNetworkId, ComponentBinding, ComponentInstanceId,
        ComponentSpecId, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
        FleetRegistryAuthority, ManagedCanisterBinding,
    },
};

#[test]
fn plan_parser_requires_one_action_and_one_selector() {
    let principal = Principal::self_authenticating([7; 32]).to_text();
    let parsed = AdmissionPlanOptions::parse(
        [
            "demo",
            "--add",
            principal.as_str(),
            "--fleet",
            "--out",
            "admission.json",
        ]
        .into_iter()
        .map(OsString::from)
        .collect(),
    )
    .expect("parse exact Fleet addition");
    assert_eq!(parsed.action, FleetAdmissionMutationAction::Add);
    assert_eq!(parsed.selector, FleetAdmissionSelector::Fleet);
    assert_eq!(parsed.out, PathBuf::from("admission.json"));

    assert!(
        AdmissionPlanOptions::parse(
            ["demo", "--add", principal.as_str(), "--out", "plan.json"]
                .into_iter()
                .map(OsString::from)
                .collect(),
        )
        .is_err()
    );
}

#[test]
fn admission_subcommands_and_help_are_ascii_ordered() {
    let command = admission_command();
    let names = command
        .get_subcommands()
        .map(clap::Command::get_name)
        .collect::<Vec<_>>();
    assert_eq!(names, ["apply", "plan", "status"]);
    assert!(admission_usage().contains("canic admission plan"));
    assert!(plan_usage().contains("--component-instance"));
}

#[test]
fn plan_file_rejects_tampered_operation_and_successor_identity() {
    let plan = plan_fixture();
    let options = apply_options();
    assert!(validate_plan_file(&options, &plan).is_ok());

    let mut tampered_operation = plan.clone();
    tampered_operation.request.operation_id = [0x71; 32];
    assert!(validate_plan_file(&options, &tampered_operation).is_err());

    let mut tampered_successor = plan.clone();
    tampered_successor.successor_policy.policy_digest = [0x72; 32];
    assert!(validate_plan_file(&options, &tampered_successor).is_err());

    let mut tampered_catalog = plan;
    tampered_catalog.request.participant_count = 2;
    assert!(validate_plan_file(&options, &tampered_catalog).is_err());
}

#[test]
fn exact_active_operation_replays_while_registry_still_matches_predecessor() {
    let plan = plan_fixture();
    let mut admission = admission_status(&plan);
    admission.current_operation = Some(operation_status(&plan));

    assert!(validate_live_plan_state(&admission, &plan, true).is_ok());

    admission
        .current_operation
        .as_mut()
        .expect("active operation")
        .principal = Principal::self_authenticating([0x73; 32]);
    assert!(validate_live_plan_state(&admission, &plan, true).is_err());
}

#[test]
fn canonical_empty_root_catalogs_retain_nonzero_identity_and_zero_count() {
    let catalogs = vec![AdmissionParticipantCatalog {
        fleet_subnet_root: Principal::self_authenticating([0x74; 32]),
        participant_catalog_digest: [0x75; 32],
        participants: Vec::new(),
    }];

    let authorities =
        participant_catalog_authorities(&catalogs).expect("canonical empty Root catalog");
    assert_eq!(authorities.len(), 1);
    assert_eq!(authorities[0].participant_count, 0);
    let (digest, count) =
        aggregate_participant_catalog_authority(&authorities).expect("empty Fleet catalog");
    assert_ne!(digest, [0; 32]);
    assert_eq!(count, 0);
}

fn plan_fixture() -> AdmissionPlanFile {
    let fleet = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([0x61; 32]),
        },
        app: AppId::from("admission-cli-test"),
    };
    let principal = Principal::self_authenticating([0x62; 32]);
    let predecessor_policy = compile_installed_fleet_admission_policy(
        fleet.clone(),
        1,
        vec![Principal::self_authenticating([0x63; 32])],
        Vec::new(),
    )
    .expect("predecessor policy");
    let mut successor_principals = predecessor_policy.fleet_principals.clone();
    successor_principals.push(principal);
    successor_principals.sort_unstable();
    let successor_policy = compile_installed_fleet_admission_policy(
        fleet.clone(),
        2,
        successor_principals,
        Vec::new(),
    )
    .expect("successor policy");
    let coordinator = Principal::self_authenticating([0x64; 32]);
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet,
            coordinator_subnet: SubnetId::from_principal(Principal::self_authenticating(
                [0x65; 32],
            )),
            coordinator,
        },
        epoch: 1,
    };
    let predecessor_registry = FleetRegistryVersion {
        authority: authority.clone(),
        revision: 7,
        content_hash: [0x66; 32],
    };
    let selector = FleetAdmissionSelector::Fleet;
    let fleet_subnet_root = Principal::self_authenticating([0x67; 32]);
    let participant_catalogs = vec![AdmissionParticipantCatalog {
        fleet_subnet_root,
        participant_catalog_digest: [0x68; 32],
        participants: vec![ManagedCanisterBinding::Component(ComponentBinding {
            authority: authority.clone(),
            component: ComponentInstanceId::from_generated_bytes([0x69; 32]),
            component_spec: ComponentSpecId::try_from(String::from("core"))
                .expect("Component Spec ID"),
            spec_hash: [0x6a; 32],
            role: CanisterRole::from("core"),
            placement_subnet: SubnetId::from_principal(Principal::self_authenticating([0x6b; 32])),
            fleet_subnet_root,
            canister_id: Principal::self_authenticating([0x6c; 32]),
        })],
    }];
    let catalog_authorities =
        participant_catalog_authorities(&participant_catalogs).expect("participant catalogs");
    let (participant_catalog_digest, participant_count) =
        aggregate_participant_catalog_authority(&catalog_authorities).expect("catalog authority");
    let operation_id = fleet_admission_mutation_operation_id(
        &predecessor_registry,
        &FleetAdmissionMutationOperationInput {
            expected_generation: predecessor_policy.generation,
            expected_policy_digest: predecessor_policy.policy_digest,
            action: FleetAdmissionMutationActionModel::Add,
            selector: selector.clone(),
            principal,
            successor_policy_digest: successor_policy.policy_digest,
            participant_catalog_digest,
            participant_count,
        },
    );
    AdmissionPlanFile {
        schema_version: ADMISSION_PLAN_SCHEMA_VERSION,
        fleet: "demo".to_string(),
        environment: "local".to_string(),
        coordinator,
        predecessor_registry,
        predecessor_policy: predecessor_policy.clone(),
        successor_policy: successor_policy.clone(),
        participant_catalogs,
        request: FleetAdmissionMutationRequest {
            authority: authority.binding,
            expected_generation: predecessor_policy.generation,
            expected_policy_digest: predecessor_policy.policy_digest,
            action: FleetAdmissionMutationAction::Add,
            selector,
            principal,
            operation_id,
            successor_policy_digest: successor_policy.policy_digest,
            participant_catalog_digest,
            participant_count,
        },
    }
}

fn apply_options() -> AdmissionApplyOptions {
    AdmissionApplyOptions {
        target: IcpTargetOptions {
            environment: "local".to_string(),
            icp: "icp".to_string(),
        },
        fleet: "demo".to_string(),
        plan_file: PathBuf::from("admission.json"),
    }
}

fn admission_status(plan: &AdmissionPlanFile) -> FleetAdmissionStatusResponse {
    FleetAdmissionStatusResponse {
        fleet: plan.predecessor_policy.fleet.clone(),
        active: FleetAdmissionPolicyStatus {
            generation: plan.predecessor_policy.generation,
            policy_digest: plan.predecessor_policy.policy_digest,
            fleet_principal_count: u16::try_from(plan.predecessor_policy.fleet_principals.len())
                .expect("bounded Principal count"),
            narrower_rule_count: 0,
            narrower_principal_reference_count: 0,
        },
        selector: FleetAdmissionSelector::Fleet,
        principals: Page {
            entries: plan.predecessor_policy.fleet_principals.clone(),
            total: plan.predecessor_policy.fleet_principals.len() as u64,
        },
        maximum_page_size: 128,
        current_operation: None,
        last_result: None,
    }
}

fn operation_status(plan: &AdmissionPlanFile) -> FleetAdmissionOperationStatusResponse {
    FleetAdmissionOperationStatusResponse {
        operation_id: plan.request.operation_id,
        action: plan.request.action,
        selector: plan.request.selector.clone(),
        principal: plan.request.principal,
        phase: FleetAdmissionOperationPhase::Planned {
            successor: FleetAdmissionPolicyStatus {
                generation: plan.successor_policy.generation,
                policy_digest: plan.successor_policy.policy_digest,
                fleet_principal_count: u16::try_from(plan.successor_policy.fleet_principals.len())
                    .expect("bounded Principal count"),
                narrower_rule_count: 0,
                narrower_principal_reference_count: 0,
            },
        },
    }
}
