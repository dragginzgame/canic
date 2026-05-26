use super::*;
use crate::deployment_truth::observe::{
    apply_canister_control_to_observed_pool, apply_live_status_to_registry_observation,
    observed_root_from_status, registry_entries_to_observed_canisters,
    registry_entries_to_observed_pool,
};
use crate::icp::{IcpCanisterStatusReport, IcpCanisterStatusSettings};
use crate::install_root::InstallState;
use crate::registry::RegistryEntry;
use crate::release_set::{ConfiguredPoolExpectation, ROOT_RELEASE_SET_MANIFEST_FILE};
use crate::test_support::temp_dir;
use serde::Serialize;
use std::{fs, path::Path};

struct LimitedExecutor {
    context: DeploymentExecutionContextV1,
}

impl DeploymentExecutor for LimitedExecutor {
    fn execution_context(&self) -> DeploymentExecutionContextV1 {
        self.context.clone()
    }
}

#[test]
fn plan_round_trips_through_json() {
    let plan = DeploymentPlanV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_id: "plan-local-root".to_string(),
        deployment_identity: sample_identity(),
        trust_domain: TrustDomainV1 {
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            migration_from: None,
        },
        fleet_template: "root".to_string(),
        runtime_variant: "local".to_string(),
        authority_profile: AuthorityProfileV1 {
            profile_id: "local-default".to_string(),
            expected_controllers: vec!["aaaaa-aa".to_string()],
            staging_controllers: Vec::new(),
            emergency_controllers: Vec::new(),
        },
        role_artifacts: vec![sample_role_artifact()],
        expected_canisters: vec![ExpectedCanisterV1 {
            role: "root".to_string(),
            canister_id: Some("aaaaa-aa".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
        }],
        expected_pool: Vec::new(),
        expected_verifier_readiness: VerifierReadinessExpectationV1 {
            required: true,
            expected_role_epochs: vec![RoleEpochExpectationV1 {
                role: "root".to_string(),
                minimum_epoch: 1,
            }],
        },
        unresolved_assumptions: Vec::new(),
    };

    let encoded = serde_json::to_string(&plan).expect("plan should encode");
    let decoded = serde_json::from_str::<DeploymentPlanV1>(&encoded).expect("plan should decode");

    assert_eq!(decoded, plan);
}

#[test]
fn lifecycle_authority_report_projects_deployment_controlled_roles() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let report = lifecycle_authority_report_from_check("lifecycle-authority-1", &check);

    assert_eq!(report.schema_version, DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(report.report_id, "lifecycle-authority-1");
    assert_eq!(report.report_digest.len(), 64);
    assert_eq!(report.check_id, "check-1");
    assert_eq!(report.authorities.len(), 1);
    assert_eq!(report.external_action_required_count, 0);
    assert_eq!(report.blocked_count, 0);
    let authority = &report.authorities[0];
    assert_eq!(authority.role.as_deref(), Some("root"));
    assert_eq!(
        authority.control_class,
        CanisterControlClassV1::DeploymentControlled
    );
    assert_eq!(
        authority.lifecycle_mode,
        LifecycleModeV1::DirectDeploymentAuthority
    );
    assert_eq!(authority.required_controllers, vec!["aaaaa-aa"]);
    assert_eq!(authority.expected_deployment_controllers, vec!["aaaaa-aa"]);
    assert!(authority.external_controllers.is_empty());
    assert!(authority.consent_requirements.is_empty());
    assert!(
        authority
            .allowed_upgrade_modes
            .contains(&LifecycleUpgradeModeV1::DirectByDeploymentAuthority)
    );
    assert!(
        authority
            .verification_requirements
            .contains(&LifecycleVerificationRequirementV1::ProtectedCallReadiness)
    );
    assert_json_round_trip(&report);
    validate_lifecycle_authority_report(&report).expect("lifecycle report should validate");
}

#[test]
fn lifecycle_authority_report_projects_user_controlled_roles_as_external() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);

    let report = lifecycle_authority_report_from_check("lifecycle-authority-1", &check);

    assert_eq!(report.external_action_required_count, 1);
    assert_eq!(report.blocked_count, 0);
    let authority = &report.authorities[0];
    assert_eq!(
        authority.control_class,
        CanisterControlClassV1::UserControlled
    );
    assert_eq!(
        authority.lifecycle_mode,
        LifecycleModeV1::ExternalCompletionOnly
    );
    assert_eq!(authority.external_controllers, vec!["user-principal"]);
    assert_eq!(
        authority.consent_requirements[0].required_principals,
        vec!["user-principal"]
    );
    assert!(authority.external_action_required);
    assert!(!authority.blocked);
    assert!(
        !authority
            .allowed_upgrade_modes
            .contains(&LifecycleUpgradeModeV1::DirectByDeploymentAuthority)
    );
    assert!(
        authority
            .allowed_upgrade_modes
            .contains(&LifecycleUpgradeModeV1::ExternalProposal)
    );
    assert!(
        authority
            .allowed_upgrade_modes
            .contains(&LifecycleUpgradeModeV1::VerifyExternalCompletion)
    );
    validate_lifecycle_authority_report(&report).expect("lifecycle report should validate");
}

#[test]
fn lifecycle_authority_report_blocks_unknown_unsafe_roles() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UnknownUnsafe;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UnknownUnsafe;
    let check = sample_check(plan, inventory);

    let report = lifecycle_authority_report_from_check("lifecycle-authority-1", &check);

    assert_eq!(report.external_action_required_count, 0);
    assert_eq!(report.blocked_count, 1);
    let authority = &report.authorities[0];
    assert_eq!(
        authority.control_class,
        CanisterControlClassV1::UnknownUnsafe
    );
    assert_eq!(
        authority.allowed_upgrade_modes,
        vec![LifecycleUpgradeModeV1::Blocked]
    );
    assert!(authority.blocked);
    assert_eq!(
        authority.lifecycle_mode,
        LifecycleModeV1::UnknownUnsafeBlocked
    );
    assert!(!authority.blockers.is_empty());
}

#[test]
fn lifecycle_authority_report_validation_rejects_count_and_digest_drift() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let mut report = lifecycle_authority_report_from_check("lifecycle-authority-1", &check);
    report.external_action_required_count = 4;

    let err = validate_lifecycle_authority_report(&report).expect_err("count drift should fail");
    assert!(matches!(err, LifecycleAuthorityReportError::CountMismatch));

    let mut report = lifecycle_authority_report_from_check("lifecycle-authority-1", &check);
    report.authorities[0]
        .warnings
        .push("stale warning".to_string());
    let err = validate_lifecycle_authority_report(&report).expect_err("digest drift should fail");
    assert!(matches!(
        err,
        LifecycleAuthorityReportError::DigestMismatch {
            field: "report_digest"
        }
    ));
}

#[test]
fn lifecycle_authority_report_text_reports_passive_boundary() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let report = lifecycle_authority_report_from_check("lifecycle-authority-1", &check);

    let text = lifecycle_authority_report_text(&report);

    assert!(text.contains("Lifecycle authority report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("report_digest:"));
    assert!(text.contains("external_action_required: 1"));
    assert!(text.contains("lifecycle_mode=external_completion_only"));
}

#[test]
fn external_lifecycle_plan_partitions_roles_before_proposals() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);

    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );

    assert_eq!(
        lifecycle_plan.schema_version,
        DEPLOYMENT_TRUTH_SCHEMA_VERSION
    );
    assert_eq!(
        lifecycle_plan.lifecycle_plan_id,
        "external-lifecycle-plan-1"
    );
    assert_eq!(
        lifecycle_plan.lifecycle_authority_report_id,
        "lifecycle-authority-1"
    );
    assert_eq!(
        lifecycle_plan.status,
        ExternalLifecyclePlanStatusV1::PendingExternalAction
    );
    assert_eq!(lifecycle_plan.lifecycle_plan_digest.len(), 64);
    assert!(lifecycle_plan.directly_executable_role_upgrades.is_empty());
    assert_eq!(lifecycle_plan.proposed_external_role_upgrades.len(), 1);
    assert!(lifecycle_plan.blocked_role_upgrades.is_empty());
    assert!(!lifecycle_plan.residual_exposure.is_empty());
    validate_external_lifecycle_plan(&lifecycle_plan).expect("lifecycle plan should validate");
    validate_external_lifecycle_plan_for_check(&lifecycle_plan, &check)
        .expect("lifecycle plan should match source check");
    assert_json_round_trip(&lifecycle_plan);
}

#[test]
fn external_lifecycle_plan_validation_rejects_digest_and_status_drift() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let mut plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    plan.lifecycle_plan_digest = sample_sha256("9");

    let err = validate_external_lifecycle_plan(&plan).expect_err("stale digest should fail");
    assert!(matches!(
        err,
        ExternalLifecyclePlanError::DigestMismatch {
            field: "lifecycle_plan_digest"
        }
    ));

    plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    plan.status = ExternalLifecyclePlanStatusV1::Blocked;
    plan.lifecycle_plan_digest = sample_sha256("9");
    let err = validate_external_lifecycle_plan(&plan).expect_err("status drift should fail");
    assert!(matches!(
        err,
        ExternalLifecyclePlanError::DigestMismatch { .. }
    ));
}

#[test]
fn external_lifecycle_plan_validation_rejects_source_check_drift() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let mut drifted_check = check;
    drifted_check.inventory.observed_canisters[0]
        .controllers
        .push("other-controller".to_string());

    let err = validate_external_lifecycle_plan_for_check(&lifecycle_plan, &drifted_check)
        .expect_err("source check drift should fail");
    assert!(matches!(
        err,
        ExternalLifecyclePlanError::SourceMismatch {
            field: "deployment_check"
        }
    ));
}

#[test]
fn external_lifecycle_plan_text_reports_passive_boundary() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );

    let text = external_lifecycle_plan_text(&lifecycle_plan);

    assert!(text.contains("External lifecycle plan"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("status: pending_external_action"));
    assert!(text.contains("proposed_external: 1"));
    assert!(text.contains("proposed_external_role_upgrades"));
    assert!(text.contains("required_external_action=external_controller_execution"));
}

#[test]
fn lifecycle_authority_report_json_shape_is_stable() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let report = lifecycle_authority_report_from_check("lifecycle-authority-1", &check);
    let encoded = serde_json::to_value(&report).expect("report should encode");

    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "report_id",
            "report_digest",
            "check_id",
            "plan_id",
            "inventory_id",
            "authorities",
            "external_action_required_count",
            "blocked_count",
        ],
    );
    assert_object_keys(
        &encoded["authorities"][0],
        &[
            "subject",
            "canister_id",
            "role",
            "control_class",
            "lifecycle_mode",
            "observed_controllers",
            "expected_deployment_controllers",
            "external_controllers",
            "required_controllers",
            "consent_requirements",
            "allowed_upgrade_modes",
            "verification_requirements",
            "external_action_required",
            "blocked",
            "blockers",
            "warnings",
            "reason",
        ],
    );
}

#[test]
fn external_upgrade_proposal_report_binds_user_controlled_target_identity() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);

    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );

    assert_eq!(report.schema_version, DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(report.report_id, "external-upgrade-proposals-1");
    assert_eq!(report.report_digest.len(), 64);
    assert_eq!(report.lifecycle_plan_id, "external-lifecycle-plan-1");
    assert_eq!(report.lifecycle_plan_digest.len(), 64);
    assert!(report.blocked_subjects.is_empty());
    assert_eq!(report.proposals.len(), 1);
    let proposal = &report.proposals[0];
    assert_eq!(proposal.deployment_plan_id, "plan-local-root");
    assert_eq!(proposal.deployment_plan_digest.len(), 64);
    assert_eq!(proposal.lifecycle_plan_id, lifecycle_plan.lifecycle_plan_id);
    assert_eq!(
        proposal.lifecycle_plan_digest,
        lifecycle_plan.lifecycle_plan_digest
    );
    assert_eq!(proposal.proposal_digest.len(), 64);
    assert_eq!(proposal.subject, "root:aaaaa-aa");
    assert_eq!(proposal.role.as_deref(), Some("root"));
    assert_eq!(
        proposal.control_class,
        CanisterControlClassV1::UserControlled
    );
    assert_eq!(proposal.current_module_hash.as_deref(), Some("module"));
    assert_eq!(
        proposal.current_canonical_embedded_config_sha256.as_deref(),
        Some("canonical")
    );
    assert_eq!(proposal.target_wasm_sha256.as_deref(), Some("wasm"));
    assert_eq!(proposal.target_wasm_gz_sha256.as_deref(), Some("gzip"));
    assert_eq!(
        proposal.target_canonical_embedded_config_sha256.as_deref(),
        Some("canonical")
    );
    assert_eq!(proposal.observed_before_digest.len(), 64);
    assert!(proposal.target_role_artifact_identity.is_some());
    assert_eq!(
        proposal.consent_requirements[0].required_principals,
        vec!["user-principal"]
    );
    assert_eq!(
        proposal.required_external_action,
        "external_controller_execution"
    );
    assert!(
        proposal
            .allowed_authorization_modes
            .contains(&ExternalUpgradeAuthorizationModeV1::ExternalControllerExecution)
    );
    assert!(
        proposal
            .verification_requirements
            .contains(&LifecycleVerificationRequirementV1::ModuleHash)
    );
    validate_external_upgrade_proposal_report(&report).expect("proposal report should validate");
    validate_external_upgrade_proposal_report_for_lifecycle_plan(&report, &lifecycle_plan, &check)
        .expect("proposal report should match source plan and check");
    assert_json_round_trip(&report);
}

#[test]
fn external_upgrade_proposal_report_validation_rejects_stale_report_digest() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let mut report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    report.blocked_subjects.push("stale:blocker".to_string());

    let err = validate_external_upgrade_proposal_report(&report)
        .expect_err("stale report digest should fail");
    assert!(matches!(
        err,
        ExternalUpgradeProposalReportError::DigestMismatch {
            field: "report_digest"
        }
    ));
}

#[test]
fn external_upgrade_proposal_report_validation_rejects_source_check_drift() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let mut drifted_check = check;
    drifted_check.plan.role_artifacts[0].installed_module_hash =
        Some("other-installed-module".to_string());

    let err = validate_external_upgrade_proposal_report_for_lifecycle_plan(
        &report,
        &lifecycle_plan,
        &drifted_check,
    )
    .expect_err("source check drift should fail");
    assert!(matches!(
        err,
        ExternalUpgradeProposalReportError::SourceMismatch {
            field: "deployment_check"
        }
    ));
}

#[test]
fn external_upgrade_proposal_report_validation_rejects_stale_proposal_digest() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let mut report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    report.proposals[0].proposal_digest = sample_sha256("9");

    let err = validate_external_upgrade_proposal_report(&report)
        .expect_err("stale proposal digest should fail");
    assert!(matches!(
        err,
        ExternalUpgradeProposalReportError::DigestMismatch {
            field: "proposal_digest"
        }
    ));
}

#[test]
fn external_upgrade_proposal_report_text_reports_passive_boundary() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );

    let text = external_upgrade_proposal_report_text(&report);

    assert!(text.contains("External upgrade proposal report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("report_digest:"));
    assert!(text.contains("proposals: 1"));
    assert!(text.contains("required_external_action=external_controller_execution"));
}

#[test]
fn external_upgrade_proposal_report_skips_direct_roles_and_blocks_unsafe_rows() {
    let direct = sample_check(sample_plan(), sample_matching_inventory());
    let direct_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &direct,
    );
    let direct_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &direct_plan,
        &direct,
    );
    assert!(direct_report.proposals.is_empty());
    assert!(direct_report.blocked_subjects.is_empty());

    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UnknownUnsafe;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UnknownUnsafe;
    let unsafe_check = sample_check(plan, inventory);
    let unsafe_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &unsafe_check,
    );
    let unsafe_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &unsafe_plan,
        &unsafe_check,
    );

    assert!(unsafe_report.proposals.is_empty());
    assert_eq!(unsafe_report.blocked_subjects, vec!["root:aaaaa-aa"]);
}

#[test]
fn external_upgrade_proposal_report_json_shape_is_stable() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let encoded = serde_json::to_value(&report).expect("report should encode");

    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "report_id",
            "report_digest",
            "lifecycle_plan_id",
            "lifecycle_plan_digest",
            "deployment_plan_id",
            "deployment_plan_digest",
            "inventory_id",
            "proposals",
            "blocked_subjects",
        ],
    );
    assert_object_keys(
        &encoded["proposals"][0],
        &[
            "proposal_id",
            "proposal_digest",
            "deployment_plan_id",
            "deployment_plan_digest",
            "lifecycle_plan_id",
            "lifecycle_plan_digest",
            "promotion_plan_id",
            "promotion_plan_digest",
            "promotion_provenance_id",
            "promotion_provenance_digest",
            "subject",
            "canister_id",
            "role",
            "control_class",
            "lifecycle_mode",
            "observed_before_digest",
            "current_module_hash",
            "current_canonical_embedded_config_sha256",
            "target_wasm_sha256",
            "target_wasm_gz_sha256",
            "target_installed_module_hash",
            "target_role_artifact_identity",
            "target_canonical_embedded_config_sha256",
            "root_trust_anchor",
            "authority_profile_hash",
            "required_external_action",
            "consent_requirements",
            "allowed_authorization_modes",
            "verification_requirements",
            "expires_at",
            "supersedes_proposal_id",
        ],
    );
}

#[test]
fn external_lifecycle_pending_report_summarizes_external_work() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );

    let report = external_lifecycle_pending_report_from_plan(
        "external-lifecycle-pending-1",
        &lifecycle_plan,
        &proposal_report,
    );

    assert_eq!(
        report.status,
        ExternalLifecyclePlanStatusV1::PendingExternalAction
    );
    assert_eq!(report.direct_upgrade_count, 0);
    assert_eq!(report.pending_external_count, 1);
    assert_eq!(report.blocked_count, 0);
    assert_eq!(
        report.pending_external_actions[0].proposal_id,
        proposal_report.proposals[0].proposal_id
    );
    assert_eq!(
        report.pending_external_actions[0].proposal_digest,
        proposal_report.proposals[0].proposal_digest
    );
    assert_eq!(report.report_digest.len(), 64);
    validate_external_lifecycle_pending_report(&report).expect("pending report should validate");
    validate_external_lifecycle_pending_report_for_plan(&report, &lifecycle_plan, &proposal_report)
        .expect("pending report should validate against source artifacts");
    assert_json_round_trip(&report);
}

#[test]
fn external_lifecycle_pending_report_validation_rejects_stale_artifacts() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let mut proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let mut report = external_lifecycle_pending_report_from_plan(
        "external-lifecycle-pending-1",
        &lifecycle_plan,
        &proposal_report,
    );
    report.pending_external_count = 0;

    let err = validate_external_lifecycle_pending_report(&report)
        .expect_err("stale pending report count should fail");
    assert_eq!(err, ExternalLifecyclePendingReportError::CountMismatch);

    let report = external_lifecycle_pending_report_from_plan(
        "external-lifecycle-pending-1",
        &lifecycle_plan,
        &proposal_report,
    );
    proposal_report.report_id = "other-proposal-report".to_string();
    let err = validate_external_lifecycle_pending_report_for_plan(
        &report,
        &lifecycle_plan,
        &proposal_report,
    )
    .expect_err("source drift should fail");
    assert_eq!(
        err,
        ExternalLifecyclePendingReportError::SourceMismatch {
            field: "proposal_report_id"
        }
    );
}

#[test]
fn external_lifecycle_pending_report_text_reports_passive_boundary() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let report = external_lifecycle_pending_report_from_plan(
        "external-lifecycle-pending-1",
        &lifecycle_plan,
        &proposal_report,
    );

    let text = external_lifecycle_pending_report_text(&report);

    assert!(text.contains("External lifecycle pending report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("pending_external: 1"));
    assert!(text.contains("pending_external_actions"));
}

fn sample_external_upgrade_proposal_and_receipt()
-> (ExternalUpgradeProposalV1, ExternalUpgradeReceiptV1) {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let proposal = proposal_report.proposals[0].clone();
    let receipt = external_upgrade_receipt_from_observation(
        "external-upgrade-receipt-1",
        &proposal,
        ExternalUpgradeConsentStateV1::ExecutedExternally,
        Some("user-principal".to_string()),
        Some(&check.inventory.observed_canisters[0]),
    );
    (proposal, receipt)
}

#[test]
fn external_upgrade_receipt_verifies_matching_external_completion() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let proposal = &proposal_report.proposals[0];

    let receipt = external_upgrade_receipt_from_observation(
        "external-upgrade-receipt-1",
        proposal,
        ExternalUpgradeConsentStateV1::ExecutedExternally,
        Some("user-principal".to_string()),
        Some(&check.inventory.observed_canisters[0]),
    );

    assert_eq!(receipt.schema_version, DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(receipt.proposal_id, proposal.proposal_id);
    assert_eq!(receipt.proposal_digest, proposal.proposal_digest);
    assert_eq!(receipt.subject, proposal.subject);
    assert_eq!(
        receipt.verification_result,
        ExternalUpgradeVerificationResultV1::Verified
    );
    assert!(receipt.verification_notes.is_empty());
    validate_external_upgrade_receipt(&receipt).expect("receipt should validate");
    validate_external_upgrade_receipt_for_proposal(&receipt, proposal)
        .expect("receipt should validate against proposal");
    assert_eq!(receipt.receipt_digest.len(), 64);
    assert_json_round_trip(&receipt);
}

#[test]
fn external_upgrade_receipt_reports_mismatched_external_completion() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let mut observed_after = check.inventory.observed_canisters[0].clone();
    observed_after.module_hash = Some("different-module".to_string());

    let receipt = external_upgrade_receipt_from_observation(
        "external-upgrade-receipt-1",
        &proposal_report.proposals[0],
        ExternalUpgradeConsentStateV1::ExecutedExternally,
        Some("user-principal".to_string()),
        Some(&observed_after),
    );

    assert_eq!(
        receipt.verification_result,
        ExternalUpgradeVerificationResultV1::Mismatch
    );
    assert!(
        receipt
            .verification_notes
            .iter()
            .any(|note| note.contains("module hash"))
    );
    validate_external_upgrade_receipt(&receipt).expect("mismatch receipt is still valid evidence");
    validate_external_upgrade_receipt_for_proposal(&receipt, &proposal_report.proposals[0])
        .expect("mismatch receipt should still validate against the proposal");
}

#[test]
fn external_upgrade_receipt_validation_rejects_stale_digest() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let mut receipt = external_upgrade_receipt_from_observation(
        "external-upgrade-receipt-1",
        &proposal_report.proposals[0],
        ExternalUpgradeConsentStateV1::ExecutedExternally,
        Some("user-principal".to_string()),
        Some(&check.inventory.observed_canisters[0]),
    );
    receipt.receipt_digest = sample_sha256("9");

    let err =
        validate_external_upgrade_receipt(&receipt).expect_err("stale receipt digest should fail");
    assert!(matches!(
        err,
        ExternalUpgradeReceiptError::DigestMismatch {
            field: "receipt_digest"
        }
    ));
}

#[test]
fn external_upgrade_receipt_validation_rejects_mismatched_proposal_source() {
    let (mut mismatched, receipt) = sample_external_upgrade_proposal_and_receipt();
    mismatched.proposal_id = "other-proposal".to_string();

    let err = validate_external_upgrade_receipt_for_proposal(&receipt, &mismatched)
        .expect_err("receipt cannot validate against another proposal");

    assert!(matches!(
        err,
        ExternalUpgradeReceiptError::SourceMismatch {
            field: "proposal_id"
        }
    ));
}

#[test]
fn external_upgrade_receipt_validation_rejects_stale_proposal_target() {
    let (mut proposal, receipt) = sample_external_upgrade_proposal_and_receipt();
    proposal.target_installed_module_hash = Some("different-target-module".to_string());

    let err = validate_external_upgrade_receipt_for_proposal(&receipt, &proposal)
        .expect_err("receipt cannot verify against changed target facts");

    assert_eq!(err, ExternalUpgradeReceiptError::VerificationMismatch);
}

#[test]
fn external_upgrade_receipt_text_reports_structural_boundary() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let receipt = external_upgrade_receipt_from_observation(
        "external-upgrade-receipt-1",
        &proposal_report.proposals[0],
        ExternalUpgradeConsentStateV1::ExecutedExternally,
        Some("user-principal".to_string()),
        Some(&check.inventory.observed_canisters[0]),
    );

    let text = external_upgrade_receipt_text(&receipt);

    assert!(text.contains("External upgrade receipt"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("consent_state: executed_externally"));
    assert!(text.contains("verification_result: verified"));
}

#[test]
fn external_upgrade_receipt_validation_rejects_contradictory_refusal() {
    let mut receipt = ExternalUpgradeReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        receipt_id: "external-upgrade-receipt-1".to_string(),
        proposal_id: "proposal-1".to_string(),
        proposal_digest: sample_sha256("1"),
        subject: "root:aaaaa-aa".to_string(),
        canister_id: Some("aaaaa-aa".to_string()),
        role: Some("root".to_string()),
        consent_state: ExternalUpgradeConsentStateV1::Refused,
        reported_by: None,
        observed_before_module_hash: Some("old".to_string()),
        observed_after_module_hash: Some("new".to_string()),
        observed_after_canonical_embedded_config_sha256: Some("config".to_string()),
        verification_result: ExternalUpgradeVerificationResultV1::Verified,
        verification_notes: Vec::new(),
        receipt_digest: sample_sha256("2"),
    };

    let err = validate_external_upgrade_receipt(&receipt)
        .expect_err("refused consent cannot verify completion");
    assert!(matches!(
        err,
        ExternalUpgradeReceiptError::RefusedConsentVerified
    ));

    receipt.verification_result = ExternalUpgradeVerificationResultV1::Pending;
    receipt.receipt_id.clear();
    let err =
        validate_external_upgrade_receipt(&receipt).expect_err("blank receipt id should fail");
    assert!(matches!(
        err,
        ExternalUpgradeReceiptError::MissingRequiredField {
            field: "receipt_id"
        }
    ));
}

#[test]
fn external_upgrade_receipt_json_shape_is_stable() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let receipt = external_upgrade_receipt_from_observation(
        "external-upgrade-receipt-1",
        &proposal_report.proposals[0],
        ExternalUpgradeConsentStateV1::ExecutedExternally,
        Some("user-principal".to_string()),
        Some(&check.inventory.observed_canisters[0]),
    );
    let encoded = serde_json::to_value(&receipt).expect("receipt should encode");

    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "receipt_id",
            "proposal_id",
            "proposal_digest",
            "subject",
            "canister_id",
            "role",
            "consent_state",
            "reported_by",
            "observed_before_module_hash",
            "observed_after_module_hash",
            "observed_after_canonical_embedded_config_sha256",
            "verification_result",
            "verification_notes",
            "receipt_digest",
        ],
    );
}

#[test]
fn role_artifact_source_round_trips_through_json() {
    let source = sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasmGz);

    assert_json_round_trip(&source);
    let encoded = serde_json::to_value(&source).expect("source should encode");
    assert_object_keys(
        &encoded,
        &[
            "role",
            "kind",
            "locator",
            "previous_receipt_kind",
            "previous_receipt_lineage_digest",
            "expected_wasm_sha256",
            "expected_wasm_gz_sha256",
            "expected_candid_sha256",
            "expected_canonical_embedded_config_sha256",
        ],
    );
}

#[test]
fn role_promotion_input_round_trips_through_json() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);

    assert_json_round_trip(&input);
    let encoded = serde_json::to_value(&input).expect("input should encode");
    assert_object_keys(
        &encoded,
        &[
            "role",
            "promotion_level",
            "source",
            "require_byte_identical_wasm",
            "require_target_embedded_config",
            "target_store_has_artifact",
        ],
    );
}

#[test]
fn role_promotion_policy_round_trips_through_json() {
    let policy = sample_role_promotion_policy();

    validate_role_promotion_policy(&policy).expect("policy should validate");
    assert_json_round_trip(&policy);
    let encoded = serde_json::to_value(&policy).expect("policy should encode");
    assert_object_keys(
        &encoded,
        &["role", "allowed_promotion_levels", "requirements"],
    );
}

#[test]
fn role_promotion_policy_validation_rejects_sealed_only_policy_with_source_build_allowed() {
    let mut policy = sample_role_promotion_policy();
    policy
        .allowed_promotion_levels
        .push(PromotionArtifactLevelV1::SourceBuild);

    let err = validate_role_promotion_policy(&policy)
        .expect_err("sealed-only policy cannot allow source build");

    assert!(matches!(
        err,
        PromotionPolicyCheckError::DecisionMismatch {
            role,
            field: "sealed_bytes"
        } if role == "root"
    ));
}

#[test]
fn promotion_policy_check_accepts_sealed_wasm_policy() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let policy = sample_role_promotion_policy();

    let check = check_promotion_policy(PromotionPolicyCheckRequest {
        check_id: "promotion-policy-1".to_string(),
        inputs: vec![input],
        policies: vec![policy],
    })
    .expect("policy check should validate");

    assert_eq!(check.status, PromotionReadinessStatusV1::Ready);
    assert_eq!(check.roles.len(), 1);
    assert!(check.roles[0].policy_satisfied);
    assert!(
        check.roles[0]
            .requirements
            .contains(&PromotionPolicyRequirementV1::SealedBytes)
    );
    assert!(
        check.roles[0]
            .requirements
            .contains(&PromotionPolicyRequirementV1::ByteIdenticalWasm)
    );
    assert!(
        check.roles[0]
            .requirements
            .contains(&PromotionPolicyRequirementV1::TargetConfigDigest)
    );
    assert_json_round_trip(&check);
    let encoded = serde_json::to_value(&check).expect("policy check should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "check_id",
            "promotion_policy_check_digest",
            "status",
            "roles",
            "blockers",
        ],
    );
    assert!(
        encoded["promotion_policy_check_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    let role = &encoded["roles"][0];
    assert_object_keys(
        role,
        &[
            "role",
            "requested_promotion_level",
            "allowed_promotion_levels",
            "requirements",
            "claims",
            "level_allowed",
            "policy_satisfied",
        ],
    );
}

#[test]
fn promotion_policy_check_blocks_source_build_when_sealed_bytes_are_required() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    let policy = sample_role_promotion_policy();

    let check = promotion_policy_check_from_inputs("promotion-policy-1", &[input], &[policy]);

    assert_eq!(check.status, PromotionReadinessStatusV1::Blocked);
    assert!(!check.roles[0].policy_satisfied);
    assert!(check.blockers.iter().any(|finding| {
        finding.code == "promotion_policy_level_not_allowed"
            || finding.code == "promotion_policy_must_use_sealed_bytes"
    }));
    validate_promotion_policy_check(&check).expect("blocked policy check should validate");
}

#[test]
fn promotion_policy_check_distinguishes_byte_identity_from_sealed_bytes() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.require_byte_identical_wasm = true;
    let mut policy = sample_role_promotion_policy();
    policy.allowed_promotion_levels = vec![PromotionArtifactLevelV1::SourceBuild];
    policy.requirements = vec![PromotionPolicyRequirementV1::ByteIdenticalWasm];

    let check = promotion_policy_check_from_inputs("promotion-policy-1", &[input], &[policy]);

    assert_eq!(check.status, PromotionReadinessStatusV1::Ready);
    assert!(check.roles[0].policy_satisfied);
    assert!(
        !check.roles[0]
            .requirements
            .contains(&PromotionPolicyRequirementV1::SealedBytes)
    );
    assert!(
        check.roles[0]
            .requirements
            .contains(&PromotionPolicyRequirementV1::ByteIdenticalWasm)
    );
}

#[test]
fn promotion_policy_check_blocks_missing_byte_identity_claim() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.require_byte_identical_wasm = false;
    let mut policy = sample_role_promotion_policy();
    policy.allowed_promotion_levels = vec![PromotionArtifactLevelV1::SourceBuild];
    policy.requirements = vec![PromotionPolicyRequirementV1::ByteIdenticalWasm];

    let check = promotion_policy_check_from_inputs("promotion-policy-1", &[input], &[policy]);

    assert_eq!(check.status, PromotionReadinessStatusV1::Blocked);
    assert!(
        check
            .blockers
            .iter()
            .any(|finding| { finding.code == "promotion_policy_byte_identity_required" })
    );
}

#[test]
fn promotion_policy_check_blocks_duplicate_policy_roles_without_matching_input() {
    let mut duplicate_policy = sample_role_promotion_policy();
    duplicate_policy.role = "wasm_store".to_string();

    let check = promotion_policy_check_from_inputs(
        "promotion-policy-1",
        &[sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
        &[
            sample_role_promotion_policy(),
            duplicate_policy.clone(),
            duplicate_policy,
        ],
    );

    assert_eq!(check.status, PromotionReadinessStatusV1::Blocked);
    assert!(check.blockers.iter().any(|finding| {
        finding.code == "promotion_policy_duplicate"
            && finding.subject.as_deref() == Some("wasm_store")
    }));
    validate_promotion_policy_check(&check).expect("duplicate-policy blocker should validate");
}

#[test]
fn promotion_policy_check_text_reports_passive_summary() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let policy = sample_role_promotion_policy();
    let check = promotion_policy_check_from_inputs("promotion-policy-1", &[input], &[policy]);

    let text = promotion_policy_check_text(&check);

    assert!(text.contains("Promotion policy check"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("status: ready"));
    assert!(text.contains("check_id: promotion-policy-1"));
    assert!(text.contains("promotion_policy_check_digest:"));
    assert!(text.contains("policy_satisfied: 1"));
    assert!(text.contains("root SealedWasm: policy_satisfied=true"));
}

#[test]
fn promotion_policy_check_round_trips_through_json() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let policy = sample_role_promotion_policy();
    let check = promotion_policy_check_from_inputs("promotion-policy-1", &[input], &[policy]);

    assert_json_round_trip(&check);
    let encoded = serde_json::to_value(&check).expect("promotion policy check should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "check_id",
            "promotion_policy_check_digest",
            "status",
            "roles",
            "blockers",
        ],
    );
    assert_eq!(encoded["check_id"], "promotion-policy-1");
    assert!(
        encoded["promotion_policy_check_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
}

#[test]
fn promotion_policy_check_validation_rejects_stale_decision() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let policy = sample_role_promotion_policy();
    let mut check = promotion_policy_check_from_inputs("promotion-policy-1", &[input], &[policy]);
    check.roles[0].policy_satisfied = false;

    let err = validate_promotion_policy_check(&check).expect_err("stale decision should fail");

    assert!(matches!(
        err,
        PromotionPolicyCheckError::DecisionMismatch {
            role,
            field: "policy_satisfied"
        } if role == "root"
    ));
}

#[test]
fn promotion_policy_check_validation_rejects_stale_digest() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let policy = sample_role_promotion_policy();
    let mut check = promotion_policy_check_from_inputs("promotion-policy-1", &[input], &[policy]);
    check.promotion_policy_check_digest = sample_sha256("9");

    let err = validate_promotion_policy_check(&check)
        .expect_err("stale promotion policy check digest should fail");

    assert!(matches!(
        err,
        PromotionPolicyCheckError::LinkageMismatch {
            field: "promotion_policy_check_digest"
        }
    ));
}

#[test]
fn promotion_artifact_identity_report_round_trips_through_json() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let report = promotion_artifact_identity_report("promotion-identity-1", &[input]);

    assert_json_round_trip(&report);
    let encoded = serde_json::to_value(&report).expect("identity report should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "report_id",
            "artifact_identity_report_digest",
            "status",
            "summary",
            "roles",
            "identity_groups",
            "blockers",
        ],
    );
    assert_object_keys(
        &encoded["summary"],
        &[
            "role_count",
            "identity_group_count",
            "shared_identity_group_count",
            "digest_pinned_role_count",
            "source_build_role_count",
            "deferred_identity_role_count",
        ],
    );
    assert!(
        encoded["artifact_identity_report_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    let role = &encoded["roles"][0];
    assert_object_keys(
        role,
        &[
            "role",
            "promotion_level",
            "source_kind",
            "source_locator",
            "identity_kind",
            "digest_pinned",
            "wasm_sha256",
            "wasm_gz_sha256",
            "candid_sha256",
            "canonical_embedded_config_sha256",
        ],
    );
    let group = &encoded["identity_groups"][0];
    assert_object_keys(
        group,
        &[
            "identity_key",
            "identity_kind",
            "roles",
            "source_kinds",
            "source_locators",
            "digest_pinned",
            "wasm_sha256",
            "wasm_gz_sha256",
            "candid_sha256",
            "canonical_embedded_config_sha256",
        ],
    );
}

#[test]
fn promotion_artifact_identity_report_distinguishes_source_kind_from_identity_kind() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.kind = RoleArtifactSourceKindV1::LocalWasm;
    input.source.expected_wasm_gz_sha256 = None;

    let report =
        promotion_artifact_identity_report_from_inputs(PromotionArtifactIdentityReportRequest {
            report_id: "promotion-identity-1".to_string(),
            inputs: vec![input],
        })
        .expect("identity report should be produced");

    assert_eq!(report.status, PromotionReadinessStatusV1::Ready);
    assert_eq!(report.roles.len(), 1);
    assert_eq!(
        report.roles[0].source_kind,
        RoleArtifactSourceKindV1::LocalWasm
    );
    assert_eq!(
        report.roles[0].identity_kind,
        PromotionArtifactIdentityKindV1::SealedWasm
    );
    assert!(report.roles[0].digest_pinned);
}

#[test]
fn promotion_artifact_identity_report_groups_roles_by_identity_key() {
    let mut root = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    root.source.kind = RoleArtifactSourceKindV1::LocalWasm;
    root.source.locator = Some("artifacts/root.wasm".to_string());
    root.source.expected_wasm_gz_sha256 = None;

    let mut worker = root.clone();
    worker.role = "worker".to_string();
    worker.source.role = "worker".to_string();
    worker.source.kind = RoleArtifactSourceKindV1::PreviousReceiptArtifact;
    worker.source.locator = Some("receipts/worker.json".to_string());
    worker.source.previous_receipt_kind = Some(PreviousArtifactReceiptKindV1::DeploymentReceipt);

    let report = promotion_artifact_identity_report("promotion-identity-1", &[root, worker]);

    assert_eq!(report.roles.len(), 2);
    assert_eq!(report.identity_groups.len(), 1);
    assert_eq!(report.summary.role_count, 2);
    assert_eq!(report.summary.identity_group_count, 1);
    assert_eq!(report.summary.shared_identity_group_count, 1);
    assert_eq!(report.summary.digest_pinned_role_count, 2);
    let group = &report.identity_groups[0];
    assert_eq!(
        group.identity_kind,
        PromotionArtifactIdentityKindV1::SealedWasm
    );
    assert_eq!(group.roles, vec!["root".to_string(), "worker".to_string()]);
    assert_eq!(
        group.source_kinds,
        vec![
            RoleArtifactSourceKindV1::LocalWasm,
            RoleArtifactSourceKindV1::PreviousReceiptArtifact
        ]
    );
    assert_eq!(
        group.source_locators,
        vec![
            "artifacts/root.wasm".to_string(),
            "receipts/worker.json".to_string()
        ]
    );
    assert_eq!(group.wasm_sha256, Some(sample_sha256("d")));
    assert!(group.identity_key.starts_with("sealed:wasm="));
    validate_promotion_artifact_identity_report(&report)
        .expect("grouped identity report should validate");
}

#[test]
fn promotion_artifact_identity_report_marks_source_build_identity() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);

    let report = promotion_artifact_identity_report("promotion-identity-1", &[input]);

    assert_eq!(report.status, PromotionReadinessStatusV1::Ready);
    assert_eq!(
        report.roles[0].identity_kind,
        PromotionArtifactIdentityKindV1::SourceBuild
    );
    assert_eq!(report.summary.source_build_role_count, 1);
}

#[test]
fn promotion_artifact_identity_report_records_invalid_source_as_blocker() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.expected_wasm_sha256 = None;
    input.source.expected_wasm_gz_sha256 = None;

    let report = promotion_artifact_identity_report("promotion-identity-1", &[input]);

    assert_eq!(report.status, PromotionReadinessStatusV1::Blocked);
    assert_eq!(report.blockers.len(), 1);
    assert_eq!(report.blockers[0].code, "promotion_artifact_source_invalid");
    assert_eq!(
        report.roles[0].identity_kind,
        PromotionArtifactIdentityKindV1::Deferred
    );
    assert_eq!(report.summary.deferred_identity_role_count, 1);
    validate_promotion_artifact_identity_report(&report)
        .expect("blocked report should still validate");
}

#[test]
fn promotion_artifact_identity_report_text_reports_passive_summary() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.kind = RoleArtifactSourceKindV1::LocalWasm;
    input.source.expected_wasm_gz_sha256 = None;
    let report = promotion_artifact_identity_report("promotion-identity-1", &[input]);

    let text = promotion_artifact_identity_report_text(&report);

    assert!(text.contains("Promotion artifact identity report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("status: ready"));
    assert!(text.contains("report_id: promotion-identity-1"));
    assert!(text.contains("artifact_identity_report_digest:"));
    assert!(text.contains("identity_groups: 1"));
    assert!(text.contains("shared_identity_groups: 0"));
    assert!(text.contains("digest_pinned_roles: 1"));
    assert!(text.contains("source_build_roles: 0"));
    assert!(text.contains("deferred_identity_roles: 0"));
    assert!(text.contains("identity groups:"));
    assert!(
        text.contains("root SealedWasm/LocalWasm: identity_kind=SealedWasm digest_pinned=true")
    );
    assert!(text.contains("source_locator: artifacts/root.wasm.gz"));
    assert!(text.contains("wasm_gz_sha256: not recorded"));
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_stale_summary() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.summary.identity_group_count = 2;

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("stale summary should fail");

    assert!(matches!(
        err,
        PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "identity_group_count"
        }
    ));
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_stale_digest() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.artifact_identity_report_digest = sample_sha256("9");

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("stale identity report digest should fail");

    assert!(matches!(
        err,
        PromotionArtifactIdentityReportError::LinkageMismatch {
            field: "artifact_identity_report_digest"
        }
    ));
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_duplicate_roles() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.roles.push(report.roles[0].clone());

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("duplicate role should fail");
    assert!(matches!(
        err,
        PromotionArtifactIdentityReportError::DuplicateRole { role } if role == "root"
    ));
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_status_blocker_mismatch() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.status = PromotionReadinessStatusV1::Blocked;

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("status blocker mismatch should fail");
    assert!(matches!(
        err,
        PromotionArtifactIdentityReportError::StatusBlockerMismatch { .. }
    ));
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_bad_digest_shape() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.roles[0].wasm_gz_sha256 = Some("NOT-A-DIGEST".to_string());

    let err =
        validate_promotion_artifact_identity_report(&report).expect_err("bad digest should fail");
    assert!(matches!(
        err,
        PromotionArtifactIdentityReportError::InvalidSha256Digest {
            field: "wasm_gz_sha256"
        }
    ));
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_stale_group_key() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.identity_groups[0].identity_key = "sealed:stale".to_string();

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("stale group key should fail");
    assert!(matches!(
        err,
        PromotionArtifactIdentityReportError::IdentityGroupKeyMismatch { .. }
    ));
}

#[test]
fn promotion_artifact_identity_report_validation_rejects_ungrouped_role() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut report = promotion_artifact_identity_report("promotion-identity-1", &[input]);
    report.identity_groups.clear();

    let err = validate_promotion_artifact_identity_report(&report)
        .expect_err("ungrouped role should fail");
    assert!(matches!(
        err,
        PromotionArtifactIdentityReportError::MissingGroupedRole { role } if role == "root"
    ));
}

#[test]
fn build_recipe_identity_round_trips_through_json() {
    let recipe = sample_build_recipe_identity();

    validate_build_recipe_identity(&recipe).expect("recipe identity should validate");
    assert_json_round_trip(&recipe);
    let encoded = serde_json::to_value(&recipe).expect("recipe identity should encode");
    assert_object_keys(
        &encoded,
        &[
            "recipe_id",
            "source_kind",
            "source_revision",
            "source_tree_clean",
            "package_or_role_selector",
            "cargo_profile",
            "cargo_features_digest",
            "cargo_lock_digest",
            "rust_toolchain",
            "builder_version",
            "target_triple",
            "linker_identity",
            "deterministic_build_mode",
            "wasm_opt_version",
            "compression_identity",
        ],
    );
}

#[test]
fn build_recipe_identity_validation_rejects_dirty_ambiguous_revision() {
    let mut recipe = sample_build_recipe_identity();
    recipe.source_revision = " ".to_string();

    let err = validate_build_recipe_identity(&recipe).expect_err("blank revision should fail");

    assert!(matches!(
        err,
        PromotionMaterializationIdentityError::MissingRequiredField {
            field: "source_revision"
        }
    ));
}

#[test]
fn build_materialization_input_round_trips_through_json() {
    let input = sample_build_materialization_input();

    validate_build_materialization_input(&input).expect("materialization input should validate");
    assert_json_round_trip(&input);
    let encoded = serde_json::to_value(&input).expect("materialization input should encode");
    assert_object_keys(
        &encoded,
        &[
            "materialization_input_id",
            "build_recipe_id",
            "canonical_embedded_config_sha256",
            "network",
            "root_trust_anchor",
            "runtime_variant",
        ],
    );
}

#[test]
fn build_materialization_input_validation_rejects_bad_config_digest() {
    let mut input = sample_build_materialization_input();
    input.canonical_embedded_config_sha256 = "bad-digest".to_string();

    let err =
        validate_build_materialization_input(&input).expect_err("bad config digest should fail");

    assert!(matches!(
        err,
        PromotionMaterializationIdentityError::InvalidSha256Digest {
            field: "canonical_embedded_config_sha256"
        }
    ));
}

#[test]
fn build_materialization_result_round_trips_through_json() {
    let result = sample_build_materialization_result();

    validate_build_materialization_result(&result).expect("materialization result should validate");
    assert_json_round_trip(&result);
    let encoded = serde_json::to_value(&result).expect("materialization result should encode");
    assert_object_keys(
        &encoded,
        &[
            "materialization_result_id",
            "build_recipe_id",
            "materialization_input_digest",
            "wasm_sha256",
            "wasm_gz_sha256",
            "installed_module_hash",
            "candid_sha256",
        ],
    );
}

#[test]
fn build_materialization_result_validation_rejects_bad_output_digest() {
    let mut result = sample_build_materialization_result();
    result.wasm_sha256 = "BAD".to_string();

    let err =
        validate_build_materialization_result(&result).expect_err("bad output digest should fail");

    assert!(matches!(
        err,
        PromotionMaterializationIdentityError::InvalidSha256Digest {
            field: "wasm_sha256"
        }
    ));
}

#[test]
fn build_materialization_evidence_round_trips_through_json() {
    let input = sample_build_materialization_input();
    let mut result = sample_build_materialization_result();
    result.materialization_input_digest = build_materialization_input_digest(&input);

    let evidence = build_materialization_evidence(BuildMaterializationEvidenceRequest {
        evidence_id: "materialization-evidence-1".to_string(),
        recipe: sample_build_recipe_identity(),
        materialization_input: input,
        materialization_result: result,
    })
    .expect("materialization evidence should validate");

    assert!(evidence.recipe_id_matches_input);
    assert!(evidence.recipe_id_matches_result);
    assert!(evidence.materialization_input_digest_matches_result);
    assert_json_round_trip(&evidence);
    let encoded = serde_json::to_value(&evidence).expect("materialization evidence should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "evidence_id",
            "materialization_evidence_digest",
            "recipe",
            "materialization_input",
            "materialization_result",
            "computed_materialization_input_digest",
            "recipe_id_matches_input",
            "recipe_id_matches_result",
            "materialization_input_digest_matches_result",
        ],
    );
    assert!(
        encoded["materialization_evidence_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
}

#[test]
fn build_materialization_evidence_text_reports_passive_boundary() {
    let input = sample_build_materialization_input();
    let mut result = sample_build_materialization_result();
    result.materialization_input_digest = build_materialization_input_digest(&input);
    let evidence = build_materialization_evidence(BuildMaterializationEvidenceRequest {
        evidence_id: "materialization-evidence-1".to_string(),
        recipe: sample_build_recipe_identity(),
        materialization_input: input,
        materialization_result: result,
    })
    .expect("materialization evidence should validate");

    let text = build_materialization_evidence_text(&evidence);

    assert!(text.contains("Build materialization evidence"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("evidence_id: materialization-evidence-1"));
    assert!(text.contains("materialization_evidence_digest:"));
    assert!(text.contains("recipe_id_matches_input: true"));
    assert!(text.contains("recipe_id_matches_result: true"));
    assert!(text.contains("materialization_input_digest_matches_result: true"));
    assert!(text.contains("execution: none"));
}

#[test]
fn build_materialization_evidence_validation_rejects_stale_computed_digest() {
    let input = sample_build_materialization_input();
    let mut result = sample_build_materialization_result();
    result.materialization_input_digest = build_materialization_input_digest(&input);
    let mut evidence = build_materialization_evidence(BuildMaterializationEvidenceRequest {
        evidence_id: "materialization-evidence-1".to_string(),
        recipe: sample_build_recipe_identity(),
        materialization_input: input,
        materialization_result: result,
    })
    .expect("materialization evidence should validate");
    evidence.computed_materialization_input_digest = sample_sha256("9");

    let err = validate_build_materialization_evidence(&evidence)
        .expect_err("stale computed digest should fail");

    assert!(matches!(
        err,
        PromotionMaterializationIdentityError::DigestMismatch {
            field: "computed_materialization_input_digest",
            ..
        }
    ));
}

#[test]
fn build_materialization_evidence_validation_rejects_stale_link_flag() {
    let input = sample_build_materialization_input();
    let mut result = sample_build_materialization_result();
    result.materialization_input_digest = build_materialization_input_digest(&input);
    let mut evidence = build_materialization_evidence(BuildMaterializationEvidenceRequest {
        evidence_id: "materialization-evidence-1".to_string(),
        recipe: sample_build_recipe_identity(),
        materialization_input: input,
        materialization_result: result,
    })
    .expect("materialization evidence should validate");
    evidence.recipe_id_matches_input = false;

    let err =
        validate_build_materialization_evidence(&evidence).expect_err("stale flag should fail");

    assert!(matches!(
        err,
        PromotionMaterializationIdentityError::LinkageMismatch {
            field: "recipe_id_matches_input"
        }
    ));
}

#[test]
fn build_materialization_evidence_validation_rejects_stale_digest() {
    let input = sample_build_materialization_input();
    let mut result = sample_build_materialization_result();
    result.materialization_input_digest = build_materialization_input_digest(&input);
    let mut evidence = build_materialization_evidence(BuildMaterializationEvidenceRequest {
        evidence_id: "materialization-evidence-1".to_string(),
        recipe: sample_build_recipe_identity(),
        materialization_input: input,
        materialization_result: result,
    })
    .expect("materialization evidence should validate");
    evidence.materialization_evidence_digest = sample_sha256("9");

    let err =
        validate_build_materialization_evidence(&evidence).expect_err("stale digest should fail");

    assert!(matches!(
        err,
        PromotionMaterializationIdentityError::LinkageMismatch {
            field: "materialization_evidence_digest"
        }
    ));
}

#[test]
fn build_materialization_evidence_rejects_mismatched_result_input_digest() {
    let input = sample_build_materialization_input();
    let result = sample_build_materialization_result();

    let err = build_materialization_evidence(BuildMaterializationEvidenceRequest {
        evidence_id: "materialization-evidence-1".to_string(),
        recipe: sample_build_recipe_identity(),
        materialization_input: input,
        materialization_result: result,
    })
    .expect_err("mismatched result input digest should fail");

    assert!(matches!(
        err,
        PromotionMaterializationIdentityError::LinkageMismatch {
            field: "materialization_input_digest_matches_result"
        }
    ));
}

#[test]
fn promotion_materialization_identity_report_round_trips_through_json() {
    let report = promotion_materialization_identity_report_from_evidence(
        PromotionMaterializationIdentityReportRequest {
            report_id: "materialization-report-1".to_string(),
            evidence: vec![sample_build_materialization_evidence()],
        },
    )
    .expect("materialization report should validate");

    assert_json_round_trip(&report);
    let encoded = serde_json::to_value(&report).expect("materialization report should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "report_id",
            "materialization_identity_report_digest",
            "status",
            "roles",
            "output_groups",
            "blockers",
        ],
    );
    assert_eq!(encoded["report_id"], "materialization-report-1");
    assert!(
        encoded["materialization_identity_report_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(encoded["status"], "Ready");
    assert_eq!(encoded["roles"][0]["role"], "root");
    assert!(
        encoded["roles"][0]["materialization_evidence_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(encoded["output_groups"][0]["roles"][0], "root");
}

#[test]
fn promotion_materialization_identity_report_groups_roles_by_output_identity() {
    let mut recipe = sample_build_recipe_identity();
    recipe.package_or_role_selector = "user_hub".to_string();
    let second = build_materialization_evidence(BuildMaterializationEvidenceRequest {
        evidence_id: "materialization-evidence-2".to_string(),
        recipe,
        materialization_input: sample_build_materialization_input(),
        materialization_result: {
            let input = sample_build_materialization_input();
            let mut result = sample_build_materialization_result();
            result.materialization_input_digest = build_materialization_input_digest(&input);
            result
        },
    })
    .expect("second materialization evidence should validate");
    let report = promotion_materialization_identity_report_from_evidence(
        PromotionMaterializationIdentityReportRequest {
            report_id: "materialization-report-1".to_string(),
            evidence: vec![sample_build_materialization_evidence(), second],
        },
    )
    .expect("materialization report should validate");

    assert_eq!(report.roles.len(), 2);
    assert_eq!(report.output_groups.len(), 1);
    assert_eq!(
        report.output_groups[0].roles,
        vec!["root".to_string(), "user_hub".to_string()]
    );
}

#[test]
fn promotion_materialization_identity_report_validation_rejects_stale_output_group() {
    let mut report = promotion_materialization_identity_report_from_evidence(
        PromotionMaterializationIdentityReportRequest {
            report_id: "materialization-report-1".to_string(),
            evidence: vec![sample_build_materialization_evidence()],
        },
    )
    .expect("materialization report should validate");
    report.output_groups[0].output_identity_key = "stale".to_string();

    let err = validate_promotion_materialization_identity_report(&report)
        .expect_err("stale output group should fail");

    assert!(matches!(
        err,
        PromotionMaterializationIdentityReportError::OutputGroupKeyMismatch { .. }
            | PromotionMaterializationIdentityReportError::OutputGroupRoleMismatch { .. }
    ));
}

#[test]
fn promotion_materialization_identity_report_validation_rejects_stale_digest() {
    let mut report = promotion_materialization_identity_report_from_evidence(
        PromotionMaterializationIdentityReportRequest {
            report_id: "materialization-report-1".to_string(),
            evidence: vec![sample_build_materialization_evidence()],
        },
    )
    .expect("materialization report should validate");
    report.materialization_identity_report_digest = sample_sha256("9");

    let err = validate_promotion_materialization_identity_report(&report)
        .expect_err("stale materialization report digest should fail");

    assert!(matches!(
        err,
        PromotionMaterializationIdentityReportError::LinkageMismatch {
            field: "materialization_identity_report_digest"
        }
    ));
}

#[test]
fn promotion_materialization_identity_report_validation_rejects_duplicate_evidence() {
    let mut report = promotion_materialization_identity_report_from_evidence(
        PromotionMaterializationIdentityReportRequest {
            report_id: "materialization-report-1".to_string(),
            evidence: vec![sample_build_materialization_evidence()],
        },
    )
    .expect("materialization report should validate");
    let mut duplicate = report.roles[0].clone();
    duplicate.role = "user_hub".to_string();
    report.roles.push(duplicate);
    report.output_groups[0].roles.push("user_hub".to_string());

    let err = validate_promotion_materialization_identity_report(&report)
        .expect_err("duplicate evidence ids should fail");

    assert!(matches!(
        err,
        PromotionMaterializationIdentityReportError::DuplicateEvidence { .. }
    ));
}

#[test]
fn promotion_materialization_identity_report_text_reports_passive_summary() {
    let report = promotion_materialization_identity_report_from_evidence(
        PromotionMaterializationIdentityReportRequest {
            report_id: "materialization-report-1".to_string(),
            evidence: vec![sample_build_materialization_evidence()],
        },
    )
    .expect("materialization report should validate");

    let text = promotion_materialization_identity_report_text(&report);

    assert!(text.contains("Promotion materialization identity report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("report_id: materialization-report-1"));
    assert!(text.contains("materialization_identity_report_digest:"));
    assert!(text.contains("output_groups: 1"));
    assert!(text.contains("root evidence=materialization-evidence-1 recipe=recipe:root:debug"));
}

#[test]
fn promotion_readiness_round_trips_through_json() {
    let plan = sample_promotion_target_plan();
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_json_round_trip(&readiness);
    let encoded = serde_json::to_value(&readiness).expect("readiness should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "readiness_id",
            "promotion_readiness_digest",
            "target_plan_id",
            "status",
            "roles",
            "blockers",
            "warnings",
        ],
    );
    assert!(
        encoded["promotion_readiness_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    let role = &encoded["roles"][0];
    assert_object_keys(
        role,
        &[
            "role",
            "promotion_level",
            "source_kind",
            "source_locator",
            "source_wasm_sha256",
            "source_wasm_gz_sha256",
            "target_wasm_sha256",
            "target_wasm_gz_sha256",
            "source_canonical_embedded_config_sha256",
            "target_canonical_embedded_config_sha256",
            "byte_identical_wasm",
            "embedded_config_identical",
            "target_store_has_artifact",
            "restage_required",
        ],
    );
}

#[test]
fn promotion_plan_transform_round_trips_through_json() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");

    assert_json_round_trip(&transform);
    let encoded = serde_json::to_value(&transform).expect("transform should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "transform_id",
            "target_plan_id",
            "promoted_plan_id",
            "promotion_plan_lineage_digest",
            "promoted_plan",
            "roles",
        ],
    );
    let role = &encoded["roles"][0];
    assert_object_keys(
        role,
        &[
            "role",
            "promotion_level",
            "source_kind",
            "source_locator",
            "artifact_source_before",
            "artifact_source_after",
            "wasm_sha256_before",
            "wasm_sha256_after",
            "wasm_gz_sha256_before",
            "wasm_gz_sha256_after",
            "candid_sha256_before",
            "candid_sha256_after",
            "canonical_embedded_config_sha256_before",
            "canonical_embedded_config_sha256_after",
            "artifact_identity_changed",
            "embedded_config_changed",
            "target_materialization_preserved",
            "source_build_materialization",
        ],
    );
}

#[test]
fn promotion_plan_transform_evidence_round_trips_through_json() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    let evidence = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: "promotion-evidence-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect("evidence should be produced");

    assert_json_round_trip(&evidence);
    let encoded = serde_json::to_value(&evidence).expect("evidence should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "evidence_id",
            "promotion_plan_transform_evidence_digest",
            "generated_at",
            "transform",
        ],
    );
    assert!(
        encoded["promotion_plan_transform_evidence_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
}

#[test]
fn promotion_plan_transform_evidence_validation_accepts_generated_evidence() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    let evidence = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: "promotion-evidence-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect("evidence should be produced");

    validate_promotion_plan_transform_evidence(&evidence)
        .expect("generated evidence should validate");
}

#[test]
fn promotion_plan_transform_evidence_text_reports_passive_boundary() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("f"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.require_byte_identical_wasm = false;
    let transform =
        promoted_deployment_plan_transform_from_inputs(&PromotionPlanTransformRequest {
            promoted_plan_id: "promoted-plan-1".to_string(),
            target_plan,
            inputs: vec![input],
        })
        .expect("transform should be produced");
    let evidence = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: "promotion-evidence-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect("evidence should be produced");

    let text = promotion_plan_transform_evidence_text(&evidence);

    assert!(text.contains("Promotion plan transform evidence"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("evidence_id: promotion-evidence-1"));
    assert!(text.contains("promotion_plan_transform_evidence_digest:"));
    assert!(text.contains("generated_at: 2026-05-25T00:00:00Z"));
    assert!(text.contains("transform_id: promotion-transform:promoted-plan-1"));
    assert!(text.contains("  Promotion plan transform"));
    assert!(text.contains("  mode: passive"));
    assert!(text.contains("  artifact_identity_changed: 1"));
}

#[test]
fn promotion_plan_transform_evidence_validation_rejects_blank_evidence_id() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    let err = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: " ".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect_err("blank evidence id should fail");

    assert!(matches!(
        err,
        PromotionPlanTransformEvidenceError::MissingRequiredField {
            field: "evidence_id"
        }
    ));
}

#[test]
fn promotion_plan_transform_evidence_validation_rejects_schema_drift() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    let mut evidence = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: "promotion-evidence-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect("evidence should be produced");
    evidence.schema_version += 1;

    let err = validate_promotion_plan_transform_evidence(&evidence)
        .expect_err("schema drift should fail");
    assert!(matches!(
        err,
        PromotionPlanTransformEvidenceError::SchemaVersionMismatch { .. }
    ));
}

#[test]
fn promotion_plan_transform_evidence_validation_rejects_stale_digest() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    let mut evidence = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: "promotion-evidence-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect("evidence should be produced");
    evidence.promotion_plan_transform_evidence_digest = sample_sha256("9");

    let err = validate_promotion_plan_transform_evidence(&evidence)
        .expect_err("stale evidence digest should fail");
    assert!(matches!(
        err,
        PromotionPlanTransformEvidenceError::LinkageMismatch {
            field: "promotion_plan_transform_evidence_digest"
        }
    ));
}

#[test]
fn promotion_plan_transform_evidence_validation_rejects_stale_transform() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    let mut evidence = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: "promotion-evidence-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect("evidence should be produced");
    evidence.transform.roles[0].artifact_identity_changed = false;

    let err = validate_promotion_plan_transform_evidence(&evidence)
        .expect_err("stale transform should fail");
    assert!(matches!(
        err,
        PromotionPlanTransformEvidenceError::Transform(
            PromotionPlanTransformError::RoleStateMismatch {
                role,
                field: "artifact_identity_changed"
            }
        ) if role == "root"
    ));
}

#[test]
fn promotion_target_execution_lineage_round_trips_through_json() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("promoted-plan-1");

    let lineage = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect("target execution lineage should be produced");

    assert_json_round_trip(&lineage);
    let encoded = serde_json::to_value(&lineage).expect("lineage should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "lineage_id",
            "generated_at",
            "target_execution_lineage_digest",
            "transform",
            "execution_preflight",
            "execution_attempted",
        ],
    );
    assert_eq!(encoded["lineage_id"], "target-execution-lineage-1");
    assert_eq!(encoded["execution_attempted"], false);
}

#[test]
fn promotion_target_execution_lineage_validation_accepts_generated_lineage() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("promoted-plan-1");

    let lineage = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect("target execution lineage should be produced");

    validate_promotion_target_execution_lineage(&lineage)
        .expect("generated lineage should validate");
}

#[test]
fn promotion_target_execution_lineage_rejects_preflight_for_other_plan() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("other-promoted-plan");

    let err = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect_err("preflight for another plan should fail");

    assert!(matches!(
        err,
        PromotionTargetExecutionLineageError::LinkageMismatch {
            field: "execution_preflight.plan_id"
        }
    ));
}

#[test]
fn promotion_target_execution_lineage_rejects_execution_claim() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("promoted-plan-1");
    let mut lineage = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect("target execution lineage should be produced");
    lineage.execution_attempted = true;

    let err = validate_promotion_target_execution_lineage(&lineage)
        .expect_err("execution claim should fail");

    assert!(matches!(
        err,
        PromotionTargetExecutionLineageError::ExecutionAttempted
    ));
}

#[test]
fn promotion_target_execution_lineage_rejects_stale_digest() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("promoted-plan-1");
    let mut lineage = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect("target execution lineage should be produced");
    lineage.target_execution_lineage_digest = sample_sha256("9");

    let err = validate_promotion_target_execution_lineage(&lineage)
        .expect_err("stale lineage digest should fail");

    assert!(matches!(
        err,
        PromotionTargetExecutionLineageError::LinkageMismatch {
            field: "target_execution_lineage_digest"
        }
    ));
}

#[test]
fn promotion_target_execution_lineage_text_reports_passive_boundary() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("promoted-plan-1");
    let lineage = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect("target execution lineage should be produced");

    let text = promotion_target_execution_lineage_text(&lineage);

    assert!(text.contains("Promotion target execution lineage"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("execution_attempted: false"));
    assert!(text.contains("lineage_id: target-execution-lineage-1"));
    assert!(text.contains("target_execution_lineage_digest: "));
    assert!(text.contains("transform_id: promotion-transform:promoted-plan-1"));
    assert!(text.contains("preflight_plan_id: promoted-plan-1"));
    assert!(text.contains("  Deployment execution preflight"));
}

#[test]
fn artifact_promotion_plan_round_trips_through_json() {
    let plan = sample_artifact_promotion_plan();

    assert_json_round_trip(&plan);
    let encoded = serde_json::to_value(&plan).expect("promotion plan should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "plan_id",
            "artifact_promotion_plan_digest",
            "generated_at",
            "status",
            "target_plan_id",
            "promoted_plan_id",
            "promotion_plan_lineage_digest",
            "readiness",
            "artifact_identity_report",
            "transform",
            "target_execution_lineage",
            "blockers",
        ],
    );
    assert_eq!(encoded["plan_id"], "artifact-promotion-plan-1");
    assert_eq!(encoded["status"], "Ready");
    assert!(
        encoded["artifact_promotion_plan_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
}

#[test]
fn artifact_promotion_plan_validation_accepts_generated_plan() {
    let plan = sample_artifact_promotion_plan();

    validate_artifact_promotion_plan(&plan).expect("generated promotion plan should validate");
}

#[test]
fn artifact_promotion_plan_validation_rejects_status_blocker_mismatch() {
    let mut plan = sample_artifact_promotion_plan();
    plan.blockers.push(SafetyFindingV1 {
        code: "promotion_blocker".to_string(),
        message: "blocked".to_string(),
        severity: SafetySeverityV1::HardFailure,
        subject: Some("root".to_string()),
    });

    let err =
        validate_artifact_promotion_plan(&plan).expect_err("ready plan with blockers should fail");

    assert!(matches!(
        err,
        ArtifactPromotionPlanError::StatusBlockerMismatch { .. }
    ));
}

#[test]
fn artifact_promotion_plan_validation_rejects_stale_lineage_copy() {
    let mut plan = sample_artifact_promotion_plan();
    plan.promotion_plan_lineage_digest = sample_sha256("9");

    let err =
        validate_artifact_promotion_plan(&plan).expect_err("stale plan lineage copy should fail");

    assert!(matches!(
        err,
        ArtifactPromotionPlanError::LinkageMismatch {
            field: "promotion_plan_lineage_digest"
        }
    ));
}

#[test]
fn artifact_promotion_plan_validation_rejects_stale_digest() {
    let mut plan = sample_artifact_promotion_plan();
    plan.artifact_promotion_plan_digest = sample_sha256("9");

    let err = validate_artifact_promotion_plan(&plan).expect_err("stale plan digest should fail");

    assert!(matches!(
        err,
        ArtifactPromotionPlanError::LinkageMismatch {
            field: "artifact_promotion_plan_digest"
        }
    ));
}

#[test]
fn artifact_promotion_plan_validation_rejects_mismatched_target_lineage() {
    let mut plan = sample_artifact_promotion_plan();
    let mut lineage = plan
        .target_execution_lineage
        .clone()
        .expect("sample plan should carry target lineage");
    lineage.transform.transform_id = "different-transform".to_string();
    plan.target_execution_lineage = Some(lineage);

    let err = validate_artifact_promotion_plan(&plan)
        .expect_err("target lineage with different transform should fail");

    assert!(matches!(
        err,
        ArtifactPromotionPlanError::LinkageMismatch {
            field: "target_execution_lineage.transform"
        }
    ));
}

#[test]
fn artifact_promotion_plan_for_check_accepts_matching_promoted_plan_check() {
    let plan = sample_artifact_promotion_plan();
    let check = sample_check(
        plan.transform.promoted_plan.clone(),
        sample_matching_inventory(),
    );

    validate_artifact_promotion_plan_for_check(&plan, &check)
        .expect("promotion plan should validate against target check");
}

#[test]
fn artifact_promotion_plan_for_check_rejects_other_target_plan() {
    let plan = sample_artifact_promotion_plan();
    let mut other_plan = sample_promotion_target_plan();
    other_plan.plan_id = "other-target-plan".to_string();
    let check = sample_check(other_plan, sample_matching_inventory());

    let err = validate_artifact_promotion_plan_for_check(&plan, &check)
        .expect_err("target check for another plan should fail");

    assert!(matches!(
        err,
        ArtifactPromotionPlanError::LinkageMismatch {
            field: "target_check.plan"
        }
    ));
}

#[test]
fn artifact_promotion_plan_for_check_rejects_missing_target_execution_lineage() {
    let sample = sample_artifact_promotion_plan();
    let promoted_plan = sample.transform.promoted_plan.clone();
    let plan = artifact_promotion_plan(ArtifactPromotionPlanRequest {
        plan_id: sample.plan_id,
        generated_at: sample.generated_at,
        readiness: sample.readiness,
        artifact_identity_report: sample.artifact_identity_report,
        transform: sample.transform,
        target_execution_lineage: None,
    })
    .expect("sample plan without lineage should still validate");
    let check = sample_check(promoted_plan, sample_matching_inventory());

    let err = validate_artifact_promotion_plan_for_check(&plan, &check)
        .expect_err("target check validation should require execution lineage");

    assert!(matches!(
        err,
        ArtifactPromotionPlanError::MissingTargetExecutionLineage
    ));
}

#[test]
fn artifact_promotion_plan_for_check_rejects_preflight_check_mismatch() {
    let plan = sample_artifact_promotion_plan();
    let mut check = sample_check(
        plan.transform.promoted_plan.clone(),
        sample_matching_inventory(),
    );
    check.report.report_id = "other-report".to_string();

    let err = validate_artifact_promotion_plan_for_check(&plan, &check)
        .expect_err("preflight mismatch should fail");

    assert!(matches!(
        err,
        ArtifactPromotionPlanError::TargetCheck(
            DeploymentExecutionPreflightError::SourceCheckMismatch {
                field: "safety_report_id",
                ..
            }
        )
    ));
}

#[test]
fn artifact_promotion_plan_text_reports_passive_summary() {
    let plan = sample_artifact_promotion_plan();

    let text = artifact_promotion_plan_text(&plan);

    assert!(text.contains("Artifact promotion plan"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("plan_id: artifact-promotion-plan-1"));
    assert!(text.contains("artifact_promotion_plan_digest:"));
    assert!(text.contains("target_execution_lineage: target-execution-lineage-1"));
    assert!(text.contains("readiness_roles: 1"));
    assert!(text.contains("artifact_identity_roles: 1"));
    assert!(text.contains("transform_roles: 1"));
    assert!(text.contains("  Promotion readiness report"));
    assert!(text.contains("  Promotion artifact identity report"));
    assert!(text.contains("  Promotion plan transform"));
}

#[test]
fn artifact_promotion_provenance_report_round_trips_through_json() {
    let report = artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
        report_id: "promotion-provenance-1".to_string(),
        artifact_promotion_plan: sample_artifact_promotion_plan(),
        wasm_store_identity_report: Some(sample_wasm_store_identity_report()),
        wasm_store_catalog_verification: Some(sample_wasm_store_catalog_verification()),
        materialization_identity_report: Some(sample_materialization_identity_report()),
    })
    .expect("promotion provenance should validate");

    assert_json_round_trip(&report);
    let encoded = serde_json::to_value(&report).expect("promotion provenance should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "report_id",
            "status",
            "artifact_promotion_plan_id",
            "artifact_promotion_plan_digest",
            "target_plan_id",
            "promoted_plan_id",
            "promotion_plan_lineage_digest",
            "provenance_report_digest",
            "readiness_id",
            "artifact_identity_report_id",
            "transform_id",
            "target_execution_lineage_id",
            "wasm_store_identity_report_id",
            "wasm_store_identity_report_digest",
            "wasm_store_catalog_verification_id",
            "wasm_store_catalog_verification_digest",
            "materialization_identity_report_id",
            "materialization_identity_report_digest",
            "execution_attempted",
            "roles",
            "blockers",
        ],
    );
    assert_eq!(encoded["report_id"], "promotion-provenance-1");
    assert_eq!(encoded["status"], "Ready");
    assert!(
        encoded["artifact_promotion_plan_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert!(
        encoded["provenance_report_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert!(
        encoded["wasm_store_identity_report_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert!(
        encoded["wasm_store_catalog_verification_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert!(
        encoded["materialization_identity_report_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(encoded["execution_attempted"], false);
    assert_eq!(encoded["roles"][0]["role"], "root");
    assert!(encoded["roles"][0]["materialization_evidence_digest"].is_string());
    assert!(encoded["roles"][0]["wasm_store_catalog_observation_digest"].is_string());
}

#[test]
fn artifact_promotion_provenance_report_links_optional_reports() {
    let report = artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
        report_id: "promotion-provenance-1".to_string(),
        artifact_promotion_plan: sample_artifact_promotion_plan(),
        wasm_store_identity_report: Some(sample_wasm_store_identity_report()),
        wasm_store_catalog_verification: Some(sample_wasm_store_catalog_verification()),
        materialization_identity_report: Some(sample_materialization_identity_report()),
    })
    .expect("promotion provenance should validate");

    assert_eq!(
        report.wasm_store_identity_report_id.as_deref(),
        Some("wasm-store-identity-1")
    );
    assert!(
        report
            .wasm_store_identity_report_digest
            .as_deref()
            .is_some_and(|digest| digest.len() == 64),
        "provenance should cite the wasm-store identity report digest"
    );
    assert_eq!(
        report.wasm_store_catalog_verification_id.as_deref(),
        Some("wasm-store-catalog-1")
    );
    assert!(
        report
            .wasm_store_catalog_verification_digest
            .as_deref()
            .is_some_and(|digest| digest.len() == 64),
        "provenance should cite the wasm-store catalog verification digest"
    );
    assert_eq!(
        report.materialization_identity_report_id.as_deref(),
        Some("materialization-report-1")
    );
    assert!(
        report
            .materialization_identity_report_digest
            .as_deref()
            .is_some_and(|digest| digest.len() == 64),
        "provenance should cite the materialization report digest"
    );
    assert_eq!(
        report.artifact_promotion_plan_digest.len(),
        64,
        "provenance should cite the plan digest"
    );
    assert_eq!(
        report.roles[0].wasm_store_locator.as_deref(),
        Some("root:aaaaa-aa:bootstrap")
    );
    assert!(
        report.roles[0]
            .wasm_store_catalog_observation_digest
            .as_deref()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(
        report.roles[0].materialization_evidence_id.as_deref(),
        Some("materialization-evidence-1")
    );
    assert!(
        report.roles[0]
            .materialization_evidence_digest
            .as_deref()
            .is_some_and(|digest| digest.len() == 64)
    );
}

#[test]
fn artifact_promotion_provenance_report_blocks_unknown_report_roles() {
    let mut receipt = sample_wasm_store_staging_receipt();
    receipt.role = "unknown".to_string();
    let wasm_store_report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![receipt],
        },
    )
    .expect("wasm-store identity report should validate");

    let report = artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
        report_id: "promotion-provenance-1".to_string(),
        artifact_promotion_plan: sample_artifact_promotion_plan(),
        wasm_store_identity_report: Some(wasm_store_report),
        wasm_store_catalog_verification: None,
        materialization_identity_report: None,
    })
    .expect("unknown optional report role should become provenance blocker");

    assert_eq!(report.status, PromotionReadinessStatusV1::Blocked);
    assert!(report.blockers.iter().any(|finding| {
        finding.code == "promotion_provenance_unknown_wasm_store_role"
            && finding.subject.as_deref() == Some("unknown")
    }));
}

#[test]
fn artifact_promotion_provenance_report_blocks_catalog_identity_mismatch() {
    let other_identity = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "other-wasm-store-report".to_string(),
            staging_receipts: vec![sample_wasm_store_staging_receipt()],
        },
    )
    .expect("alternate wasm-store identity report should validate");
    let catalog =
        promotion_wasm_store_catalog_verification(PromotionWasmStoreCatalogVerificationRequest {
            verification_id: "wasm-store-catalog-1".to_string(),
            wasm_store_identity_report: other_identity,
            catalog_entries: vec![sample_wasm_store_catalog_entry()],
        })
        .expect("alternate catalog verification should validate");

    let report = artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
        report_id: "promotion-provenance-1".to_string(),
        artifact_promotion_plan: sample_artifact_promotion_plan(),
        wasm_store_identity_report: Some(sample_wasm_store_identity_report()),
        wasm_store_catalog_verification: Some(catalog),
        materialization_identity_report: None,
    })
    .expect("mismatched catalog verification should become provenance blocker");

    assert_eq!(report.status, PromotionReadinessStatusV1::Blocked);
    assert!(report.blockers.iter().any(|finding| {
        finding.code == "promotion_provenance_wasm_store_catalog_identity_mismatch"
            && finding.subject.as_deref() == Some("wasm_store_catalog")
    }));
}

#[test]
fn artifact_promotion_provenance_report_blocks_catalog_locator_mismatch() {
    let mut receipt = sample_wasm_store_staging_receipt();
    receipt.wasm_store_locator = Some("root:aaaaa-aa:other".to_string());
    let other_identity = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![receipt],
        },
    )
    .expect("alternate wasm-store identity report should validate");
    let catalog =
        promotion_wasm_store_catalog_verification(PromotionWasmStoreCatalogVerificationRequest {
            verification_id: "wasm-store-catalog-1".to_string(),
            wasm_store_identity_report: other_identity,
            catalog_entries: vec![PromotionWasmStoreCatalogEntryV1 {
                locator: "root:aaaaa-aa:other".to_string(),
                artifact_identity: "embedded:root:0.44.0:abc123".to_string(),
                published_chunk_count: 2,
            }],
        })
        .expect("alternate catalog verification should validate");

    let report = artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
        report_id: "promotion-provenance-1".to_string(),
        artifact_promotion_plan: sample_artifact_promotion_plan(),
        wasm_store_identity_report: Some(sample_wasm_store_identity_report()),
        wasm_store_catalog_verification: Some(catalog),
        materialization_identity_report: None,
    })
    .expect("mismatched catalog locator should become provenance blocker");

    assert_eq!(report.status, PromotionReadinessStatusV1::Blocked);
    assert!(report.blockers.iter().any(|finding| {
        finding.code == "promotion_provenance_wasm_store_catalog_locator_mismatch"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn artifact_promotion_provenance_report_rejects_stale_digest() {
    let mut report = sample_artifact_promotion_provenance_report();
    report.provenance_report_digest = sample_sha256("9");

    let err = validate_artifact_promotion_provenance_report(&report)
        .expect_err("stale provenance digest should fail");

    assert!(matches!(
        err,
        ArtifactPromotionProvenanceReportError::LinkageMismatch {
            field: "provenance_report_digest"
        }
    ));
}

#[test]
fn artifact_promotion_provenance_report_rejects_stale_plan_digest_link() {
    let mut report = sample_artifact_promotion_provenance_report();
    report.artifact_promotion_plan_digest = sample_sha256("9");

    let err = validate_artifact_promotion_provenance_report(&report)
        .expect_err("stale cited promotion plan digest should fail");

    assert!(matches!(
        err,
        ArtifactPromotionProvenanceReportError::LinkageMismatch {
            field: "provenance_report_digest"
        }
    ));
}

#[test]
fn artifact_promotion_provenance_report_rejects_stale_wasm_store_digest_link() {
    let mut report = sample_artifact_promotion_provenance_report();
    report.wasm_store_identity_report_digest = Some(sample_sha256("9"));

    let err = validate_artifact_promotion_provenance_report(&report)
        .expect_err("stale cited wasm-store identity report digest should fail");

    assert!(matches!(
        err,
        ArtifactPromotionProvenanceReportError::LinkageMismatch {
            field: "provenance_report_digest"
        }
    ));
}

#[test]
fn artifact_promotion_provenance_report_rejects_stale_wasm_store_catalog_digest_link() {
    let mut report = sample_artifact_promotion_provenance_report();
    report.wasm_store_catalog_verification_digest = Some(sample_sha256("9"));

    let err = validate_artifact_promotion_provenance_report(&report)
        .expect_err("stale cited wasm-store catalog verification digest should fail");

    assert!(matches!(
        err,
        ArtifactPromotionProvenanceReportError::LinkageMismatch {
            field: "provenance_report_digest"
        }
    ));
}

#[test]
fn artifact_promotion_provenance_report_rejects_stale_materialization_digest_link() {
    let mut report = sample_artifact_promotion_provenance_report();
    report.materialization_identity_report_digest = Some(sample_sha256("9"));

    let err = validate_artifact_promotion_provenance_report(&report)
        .expect_err("stale cited materialization report digest should fail");

    assert!(matches!(
        err,
        ArtifactPromotionProvenanceReportError::LinkageMismatch {
            field: "provenance_report_digest"
        }
    ));
}

#[test]
fn artifact_promotion_provenance_report_rejects_stale_role_materialization_digest() {
    let mut report = sample_artifact_promotion_provenance_report();
    report.roles[0].materialization_evidence_digest = Some(sample_sha256("9"));

    let err = validate_artifact_promotion_provenance_report(&report)
        .expect_err("stale role materialization evidence digest should fail");

    assert!(matches!(
        err,
        ArtifactPromotionProvenanceReportError::LinkageMismatch {
            field: "provenance_report_digest"
        }
    ));
}

#[test]
fn artifact_promotion_provenance_report_text_reports_passive_summary() {
    let report = artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
        report_id: "promotion-provenance-1".to_string(),
        artifact_promotion_plan: sample_artifact_promotion_plan(),
        wasm_store_identity_report: Some(sample_wasm_store_identity_report()),
        wasm_store_catalog_verification: Some(sample_wasm_store_catalog_verification()),
        materialization_identity_report: Some(sample_materialization_identity_report()),
    })
    .expect("promotion provenance should validate");

    let text = artifact_promotion_provenance_report_text(&report);

    assert!(text.contains("Artifact promotion provenance report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("report_id: promotion-provenance-1"));
    assert!(text.contains("artifact_promotion_plan_id: artifact-promotion-plan-1"));
    assert!(text.contains("artifact_promotion_plan_digest:"));
    assert!(text.contains("provenance_report_digest:"));
    assert!(text.contains("wasm_store_identity: wasm-store-identity-1"));
    assert!(text.contains("wasm_store_identity_digest:"));
    assert!(text.contains("wasm_store_catalog: wasm-store-catalog-1"));
    assert!(text.contains("wasm_store_catalog_digest:"));
    assert!(text.contains("catalog_digest="));
    assert!(text.contains("materialization_identity: materialization-report-1"));
    assert!(text.contains("materialization_identity_digest:"));
    assert!(text.contains("materialization_digest="));
    assert!(text.contains("root SealedWasm/LocalWasmGz"));
}

#[test]
fn artifact_promotion_execution_receipt_round_trips_through_json() {
    let receipt = sample_artifact_promotion_execution_receipt();

    assert_json_round_trip(&receipt);
    let encoded = serde_json::to_value(&receipt).expect("execution receipt should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "receipt_id",
            "execution_receipt_digest",
            "artifact_promotion_plan_id",
            "artifact_promotion_plan_digest",
            "provenance_report_id",
            "provenance_report_digest",
            "provenance_status",
            "promoted_plan_id",
            "promotion_plan_lineage_digest",
            "operation_id",
            "operation_status",
            "command_result",
            "started_at",
            "finished_at",
            "deployment_receipt",
            "roles",
        ],
    );
    assert_eq!(encoded["receipt_id"], "promotion-execution-receipt-1");
    assert!(
        encoded["execution_receipt_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(
        encoded["artifact_promotion_plan_id"],
        "artifact-promotion-plan-1"
    );
    assert!(
        encoded["artifact_promotion_plan_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(encoded["provenance_report_id"], "promotion-provenance-1");
    assert!(
        encoded["provenance_report_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(encoded["provenance_status"], "Ready");
    assert_eq!(encoded["promoted_plan_id"], "promoted-plan-1");
    assert_eq!(encoded["operation_id"], "promoted-operation-1");
    assert_eq!(encoded["roles"][0]["role"], "root");
    assert!(encoded["roles"][0]["materialization_evidence_digest"].is_string());
    assert!(encoded["roles"][0]["wasm_store_catalog_observation_digest"].is_string());
}

#[test]
fn artifact_promotion_execution_receipt_links_deployment_receipt() {
    let receipt = sample_artifact_promotion_execution_receipt();

    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(receipt.command_result, DeploymentCommandResultV1::Succeeded);
    assert_eq!(receipt.deployment_receipt.plan_id, receipt.promoted_plan_id);
    assert_eq!(
        receipt.deployment_receipt.operation_id,
        receipt.operation_id
    );
    assert_eq!(receipt.artifact_promotion_plan_digest.len(), 64);
    assert_eq!(
        receipt.roles[0].role_phase_result,
        Some(RolePhaseResultV1::Applied)
    );
    assert_eq!(receipt.provenance_report_digest.len(), 64);
    assert_eq!(receipt.execution_receipt_digest.len(), 64);
    assert_eq!(
        receipt.roles[0].artifact_digest.as_deref(),
        Some(sample_sha256("5").as_str())
    );
    assert!(
        receipt.roles[0]
            .materialization_evidence_digest
            .as_deref()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(
        receipt.roles[0].observed_module_hash_after.as_deref(),
        Some(sample_sha256("7").as_str())
    );
    assert!(
        receipt.roles[0]
            .wasm_store_catalog_observation_digest
            .as_deref()
            .is_some_and(|digest| digest.len() == 64)
    );
}

#[test]
fn artifact_promotion_execution_receipt_validation_rejects_stale_digest() {
    let mut receipt = sample_artifact_promotion_execution_receipt();
    receipt.execution_receipt_digest = sample_sha256("9");

    let err = validate_artifact_promotion_execution_receipt(&receipt)
        .expect_err("stale execution receipt digest should fail");

    assert!(matches!(
        err,
        ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "execution_receipt_digest"
        }
    ));
}

#[test]
fn artifact_promotion_execution_receipt_validation_rejects_stale_materialization_digest() {
    let mut receipt = sample_artifact_promotion_execution_receipt();
    receipt.roles[0].materialization_evidence_digest = Some(sample_sha256("9"));

    let err = validate_artifact_promotion_execution_receipt(&receipt)
        .expect_err("stale role materialization digest should fail");

    assert!(matches!(
        err,
        ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "execution_receipt_digest"
        }
    ));
}

#[test]
fn artifact_promotion_execution_receipt_validation_rejects_stale_plan_digest_link() {
    let mut receipt = sample_artifact_promotion_execution_receipt();
    receipt.artifact_promotion_plan_digest = sample_sha256("9");

    let err = validate_artifact_promotion_execution_receipt(&receipt)
        .expect_err("stale cited plan digest should fail");

    assert!(matches!(
        err,
        ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "execution_receipt_digest"
        }
    ));
}

#[test]
fn artifact_promotion_execution_receipt_validation_rejects_nested_receipt_drift() {
    let mut receipt = sample_artifact_promotion_execution_receipt();
    receipt.deployment_receipt.phase_receipts[0]
        .verified_postcondition
        .evidence
        .push("stale:evidence".to_string());

    let err = validate_artifact_promotion_execution_receipt(&receipt)
        .expect_err("nested deployment receipt drift should fail");

    assert!(matches!(
        err,
        ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "execution_receipt_digest"
        }
    ));
}

#[test]
fn artifact_promotion_execution_receipt_rejects_other_promoted_plan() {
    let err = artifact_promotion_execution_receipt(ArtifactPromotionExecutionReceiptRequest {
        receipt_id: "promotion-execution-receipt-1".to_string(),
        provenance_report: sample_artifact_promotion_provenance_report(),
        deployment_receipt: sample_receipt_with_phase(
            "other-plan",
            Some("aaaaa-aa"),
            ObservationStatusV1::Observed,
            RolePhaseResultV1::Applied,
        ),
    })
    .expect_err("deployment receipt must match promoted plan");

    assert!(matches!(
        err,
        ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "deployment_receipt.plan_id"
        }
    ));
}

#[test]
fn artifact_promotion_execution_receipt_rejects_blocked_provenance() {
    let mut receipt = sample_wasm_store_staging_receipt();
    receipt.role = "unknown".to_string();
    let wasm_store_report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![receipt],
        },
    )
    .expect("wasm-store identity report should validate");
    let provenance_report =
        artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
            report_id: "promotion-provenance-1".to_string(),
            artifact_promotion_plan: sample_artifact_promotion_plan(),
            wasm_store_identity_report: Some(wasm_store_report),
            wasm_store_catalog_verification: None,
            materialization_identity_report: None,
        })
        .expect("blocked provenance report should still be reportable");

    let err = artifact_promotion_execution_receipt(ArtifactPromotionExecutionReceiptRequest {
        receipt_id: "promotion-execution-receipt-1".to_string(),
        provenance_report,
        deployment_receipt: sample_promoted_deployment_receipt(),
    })
    .expect_err("blocked provenance cannot become execution receipt");

    assert!(matches!(
        err,
        ArtifactPromotionExecutionReceiptError::ProvenanceNotReady {
            status: PromotionReadinessStatusV1::Blocked
        }
    ));
}

#[test]
fn artifact_promotion_execution_receipt_validation_rejects_stale_operation_status() {
    let mut receipt = sample_artifact_promotion_execution_receipt();
    receipt.operation_status = DeploymentExecutionStatusV1::FailedBeforeMutation;

    let err = validate_artifact_promotion_execution_receipt(&receipt)
        .expect_err("wrapper status must match nested deployment receipt");

    assert!(matches!(
        err,
        ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "operation_status"
        }
    ));
}

#[test]
fn artifact_promotion_execution_receipt_validation_rejects_stale_provenance_status() {
    let mut receipt = sample_artifact_promotion_execution_receipt();
    receipt.provenance_status = PromotionReadinessStatusV1::Blocked;

    let err = validate_artifact_promotion_execution_receipt(&receipt)
        .expect_err("archived execution receipt must preserve ready provenance");

    assert!(matches!(
        err,
        ArtifactPromotionExecutionReceiptError::ProvenanceNotReady {
            status: PromotionReadinessStatusV1::Blocked
        }
    ));
}

#[test]
fn artifact_promotion_execution_receipt_validation_rejects_missing_deployment_role() {
    let mut receipt = sample_artifact_promotion_execution_receipt();
    receipt.deployment_receipt.role_phase_receipts.clear();

    let err = validate_artifact_promotion_execution_receipt(&receipt)
        .expect_err("promotion execution receipt must cite deployment role evidence");

    assert!(matches!(
        err,
        ArtifactPromotionExecutionReceiptError::MissingDeploymentRole { role } if role == "root"
    ));
}

#[test]
fn artifact_promotion_execution_receipt_validation_rejects_unknown_deployment_role() {
    let mut receipt = sample_artifact_promotion_execution_receipt();
    let mut extra = receipt.deployment_receipt.role_phase_receipts[0].clone();
    extra.role = "worker".to_string();
    receipt.deployment_receipt.role_phase_receipts.push(extra);

    let err = validate_artifact_promotion_execution_receipt(&receipt)
        .expect_err("deployment receipt cannot add roles outside promotion provenance");

    assert!(matches!(
        err,
        ArtifactPromotionExecutionReceiptError::UnknownDeploymentRole { role } if role == "worker"
    ));
}

#[test]
fn artifact_promotion_execution_receipt_text_reports_execution_summary() {
    let receipt = sample_artifact_promotion_execution_receipt();

    let text = artifact_promotion_execution_receipt_text(&receipt);

    assert!(text.contains("Artifact promotion execution receipt"));
    assert!(text.contains("mode: execution_receipt"));
    assert!(text.contains("receipt_id: promotion-execution-receipt-1"));
    assert!(text.contains("execution_receipt_digest:"));
    assert!(text.contains("artifact_promotion_plan_id: artifact-promotion-plan-1"));
    assert!(text.contains("artifact_promotion_plan_digest:"));
    assert!(text.contains("provenance_report_id: promotion-provenance-1"));
    assert!(text.contains("provenance_report_digest:"));
    assert!(text.contains("promoted_plan_id: promoted-plan-1"));
    assert!(text.contains("operation_id: promoted-operation-1"));
    assert!(text.contains("provenance_status: ready"));
    assert!(text.contains("deployment_phase_receipts: 1"));
    assert!(text.contains("root SealedWasm: result=Applied"));
    assert!(text.contains("catalog_digest="));
}

#[test]
fn promotion_plan_transform_validation_accepts_generated_transform() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");

    validate_promotion_plan_transform(&transform).expect("generated transform should validate");
}

#[test]
fn promotion_plan_transform_validation_rejects_schema_drift() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.schema_version += 1;

    let err = validate_promotion_plan_transform(&transform).expect_err("schema drift should fail");
    assert!(matches!(
        err,
        PromotionPlanTransformError::SchemaVersionMismatch { .. }
    ));
}

#[test]
fn promotion_plan_transform_validation_rejects_plan_id_mismatch() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.promoted_plan.plan_id = "different-plan".to_string();

    let err =
        validate_promotion_plan_transform(&transform).expect_err("plan id mismatch should fail");
    assert!(matches!(
        err,
        PromotionPlanTransformError::PromotedPlanIdMismatch { .. }
    ));
}

#[test]
fn promotion_plan_transform_validation_rejects_duplicate_roles() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.roles.push(transform.roles[0].clone());

    let err =
        validate_promotion_plan_transform(&transform).expect_err("duplicate role should fail");
    assert!(matches!(
        err,
        PromotionPlanTransformError::DuplicateRole { role } if role == "root"
    ));
}

#[test]
fn promotion_plan_transform_validation_rejects_missing_promoted_role() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.promoted_plan.role_artifacts.clear();

    let err = validate_promotion_plan_transform(&transform)
        .expect_err("missing promoted role should fail");
    assert!(matches!(
        err,
        PromotionPlanTransformError::PromotedRoleMissing { role } if role == "root"
    ));
}

#[test]
fn promotion_plan_transform_validation_rejects_stale_lineage_digest() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.promotion_plan_lineage_digest = sample_sha256("9");

    let err = validate_promotion_plan_transform(&transform)
        .expect_err("stale lineage digest should fail");
    assert!(matches!(
        err,
        PromotionPlanTransformError::RoleStateMismatch {
            role,
            field: "promotion_plan_lineage_digest"
        } if role == "promotion_plan_lineage"
    ));
}

#[test]
fn promotion_plan_lineage_digest_changes_when_materialization_link_changes() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_sha256 = Some(sample_sha256("5"));
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("6"));
    target_plan.role_artifacts[0].installed_module_hash = Some(sample_sha256("7"));
    target_plan.role_artifacts[0].candid_sha256 = Some(sample_sha256("8"));
    let request = PromotionPlanTransformWithMaterializationRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SourceBuild,
        )],
        materialization_evidence: vec![sample_build_materialization_evidence()],
    };
    let transform = promoted_deployment_plan_transform_from_inputs_with_materialization(&request)
        .expect("source-build transform should link evidence");
    let mut changed_roles = transform.roles.clone();
    changed_roles[0]
        .source_build_materialization
        .as_mut()
        .expect("materialization link should exist")
        .evidence_id = "different-evidence".to_string();

    let changed_digest = promotion_plan_lineage_digest(
        &transform.target_plan_id,
        &transform.promoted_plan_id,
        &transform.promoted_plan,
        &changed_roles,
    );

    assert_ne!(changed_digest, transform.promotion_plan_lineage_digest);
}

#[test]
fn promotion_plan_transform_validation_rejects_stale_after_summary() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.roles[0].wasm_gz_sha256_after = Some(sample_sha256("f"));

    let err = validate_promotion_plan_transform(&transform).expect_err("stale summary should fail");
    assert!(matches!(
        err,
        PromotionPlanTransformError::RoleStateMismatch {
            role,
            field: "wasm_gz_sha256_after"
        } if role == "root"
    ));
}

#[test]
fn promotion_plan_transform_validation_rejects_stale_change_flag() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("f"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.require_byte_identical_wasm = false;
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![input],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.roles[0].artifact_identity_changed = false;

    let err = validate_promotion_plan_transform(&transform).expect_err("stale flag should fail");
    assert!(matches!(
        err,
        PromotionPlanTransformError::RoleStateMismatch {
            role,
            field: "artifact_identity_changed"
        } if role == "root"
    ));
}

#[test]
fn role_artifact_source_requires_digest_pins_for_executable_overrides() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasm);
    source.expected_wasm_sha256 = None;
    source.expected_wasm_gz_sha256 = None;

    let err = validate_role_artifact_source(&source).expect_err("digest pin should be required");
    assert!(matches!(
        err,
        PromotionArtifactSourceError::MissingDigestPin {
            kind: RoleArtifactSourceKindV1::LocalWasm
        }
    ));
}

#[test]
fn role_artifact_source_rejects_invalid_digest_shape() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasmGz);
    source.expected_wasm_gz_sha256 = Some("NOT-A-DIGEST".to_string());

    let err = validate_role_artifact_source(&source).expect_err("digest should be checked");
    assert!(matches!(
        err,
        PromotionArtifactSourceError::InvalidSha256Digest {
            field: "expected_wasm_gz_sha256"
        }
    ));
}

#[test]
fn previous_receipt_artifact_source_requires_eligible_receipt_kind() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::PreviousReceiptArtifact);
    source.previous_receipt_kind = None;

    let err = validate_role_artifact_source(&source).expect_err("receipt kind should be required");
    assert!(matches!(
        err,
        PromotionArtifactSourceError::MissingPreviousReceiptKind
    ));

    source.previous_receipt_kind = Some(PreviousArtifactReceiptKindV1::DeploymentReceipt);
    validate_role_artifact_source(&source).expect("deployment receipt artifact should be eligible");
    source.previous_receipt_kind = Some(PreviousArtifactReceiptKindV1::StagingReceipt);
    validate_role_artifact_source(&source).expect("staging receipt artifact should be eligible");
}

#[test]
fn previous_receipt_artifact_source_requires_lineage_digest() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::PreviousReceiptArtifact);
    source.previous_receipt_lineage_digest = None;

    let err = validate_role_artifact_source(&source)
        .expect_err("receipt lineage digest should be required");

    assert!(matches!(
        err,
        PromotionArtifactSourceError::MissingPreviousReceiptLineageDigest
    ));
}

#[test]
fn previous_receipt_artifact_source_rejects_invalid_lineage_digest() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::PreviousReceiptArtifact);
    source.previous_receipt_lineage_digest = Some("bad-digest".to_string());

    let err =
        validate_role_artifact_source(&source).expect_err("receipt lineage digest should validate");

    assert!(matches!(
        err,
        PromotionArtifactSourceError::InvalidSha256Digest {
            field: "previous_receipt_lineage_digest"
        }
    ));
}

#[test]
fn non_receipt_artifact_source_rejects_previous_receipt_kind() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasmGz);
    source.previous_receipt_kind = Some(PreviousArtifactReceiptKindV1::DeploymentReceipt);

    let err =
        validate_role_artifact_source(&source).expect_err("receipt kind should be source-specific");
    assert!(matches!(
        err,
        PromotionArtifactSourceError::UnexpectedPreviousReceiptKind {
            kind: RoleArtifactSourceKindV1::LocalWasmGz
        }
    ));
}

#[test]
fn non_receipt_artifact_source_rejects_previous_receipt_lineage_digest() {
    let mut source = sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasmGz);
    source.previous_receipt_lineage_digest = Some(sample_sha256("9"));

    let err = validate_role_artifact_source(&source)
        .expect_err("receipt lineage digest should be source-specific");

    assert!(matches!(
        err,
        PromotionArtifactSourceError::UnexpectedPreviousReceiptLineageDigest {
            kind: RoleArtifactSourceKindV1::LocalWasmGz
        }
    ));
}

#[test]
fn canonical_wasm_store_default_source_does_not_require_locator_or_digest_pin() {
    let source = RoleArtifactSourceV1 {
        role: "wasm_store".to_string(),
        kind: RoleArtifactSourceKindV1::CanonicalWasmStoreDefault,
        locator: None,
        previous_receipt_kind: None,
        previous_receipt_lineage_digest: None,
        expected_wasm_sha256: None,
        expected_wasm_gz_sha256: None,
        expected_candid_sha256: None,
        expected_canonical_embedded_config_sha256: None,
    };

    validate_role_artifact_source(&source).expect("canonical source should be deferred");
}

#[test]
fn promotion_readiness_reports_ready_role_and_restage_warning() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.target_store_has_artifact = Some(false);

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.schema_version, DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(readiness.target_plan_id, plan.plan_id);
    assert_eq!(readiness.status, PromotionReadinessStatusV1::Ready);
    assert!(readiness.blockers.is_empty());
    assert_eq!(readiness.warnings.len(), 1);
    assert_eq!(
        readiness.warnings[0].code,
        "promotion_target_store_restage_required"
    );
    assert_eq!(readiness.roles.len(), 1);
    assert_eq!(readiness.roles[0].byte_identical_wasm, Some(true));
    assert_eq!(readiness.roles[0].embedded_config_identical, Some(true));
    assert!(readiness.roles[0].restage_required);
    validate_promotion_readiness(&readiness).expect("readiness artifact should validate");
}

#[test]
fn promotion_readiness_blocks_sealed_wasm_embedded_config_mismatch() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("e"));

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_sealed_wasm_embedded_config_mismatch"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn source_build_promotion_allows_target_config_digest_change() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("e"));

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Ready);
    assert!(readiness.blockers.is_empty());
    assert_eq!(readiness.roles[0].embedded_config_identical, Some(false));
    validate_promotion_readiness(&readiness).expect("source-build readiness should validate");
}

#[test]
fn check_promotion_readiness_validates_and_returns_artifact() {
    let request = PromotionReadinessRequest {
        readiness_id: "promotion-ready-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };

    let readiness = check_promotion_readiness(&request).expect("readiness should be valid");

    assert_eq!(readiness.readiness_id, "promotion-ready-1");
    assert_eq!(readiness.status, PromotionReadinessStatusV1::Ready);
    assert_eq!(readiness.roles.len(), 1);
}

#[test]
fn promotion_readiness_with_policy_blocks_source_build_when_sealed_bytes_are_required() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    let policy = sample_role_promotion_policy();

    let readiness = promotion_readiness_from_inputs_with_policy(
        "promotion-ready-1",
        &sample_promotion_target_plan(),
        &[input],
        &[policy],
    );

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_policy_level_not_allowed"
            || finding.code == "promotion_policy_must_use_sealed_bytes"
    }));
    validate_promotion_readiness(&readiness).expect("policy-blocked readiness should validate");
}

#[test]
fn promotion_readiness_with_policy_accepts_byte_identical_source_build_policy() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.require_byte_identical_wasm = true;
    let mut policy = sample_role_promotion_policy();
    policy.allowed_promotion_levels = vec![PromotionArtifactLevelV1::SourceBuild];
    policy.requirements = vec![PromotionPolicyRequirementV1::ByteIdenticalWasm];

    let readiness = check_promotion_readiness_with_policy(&PromotionReadinessWithPolicyRequest {
        readiness_id: "promotion-ready-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![input],
        policies: vec![policy],
    })
    .expect("source-build policy readiness should validate");

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Ready);
    assert!(readiness.blockers.is_empty());
}

#[test]
fn promotion_readiness_with_policy_reports_missing_role_policy() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);

    let readiness = promotion_readiness_from_inputs_with_policy(
        "promotion-ready-1",
        &sample_promotion_target_plan(),
        &[input],
        &[],
    );

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_policy_missing" && finding.subject.as_deref() == Some("root")
    }));
    validate_promotion_readiness(&readiness).expect("missing-policy readiness should validate");
}

#[test]
fn check_promotion_readiness_rejects_blank_readiness_id() {
    let request = PromotionReadinessRequest {
        readiness_id: " ".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };

    let err = check_promotion_readiness(&request).expect_err("blank readiness id should fail");
    assert!(matches!(
        err,
        PromotionReadinessError::MissingRequiredField {
            field: "readiness_id"
        }
    ));
}

#[test]
fn promoted_deployment_plan_applies_sealed_wasm_role_identity() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.kind = RoleArtifactSourceKindV1::LocalWasmGz;
    input.source.locator = Some("promoted/root.wasm.gz".to_string());
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![input],
    };

    let promoted =
        promoted_deployment_plan_from_inputs(&request).expect("promoted plan should be produced");

    assert_eq!(promoted.plan_id, "promoted-plan-1");
    assert_eq!(
        promoted.authority_profile,
        request.target_plan.authority_profile
    );
    assert_eq!(promoted.trust_domain, request.target_plan.trust_domain);
    let artifact = promoted
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == "root")
        .expect("root artifact should remain");
    assert_eq!(artifact.source, ArtifactSourceV1::External);
    assert_eq!(
        artifact.wasm_gz_path.as_deref(),
        Some("promoted/root.wasm.gz")
    );
    assert_eq!(artifact.wasm_sha256, Some(sample_sha256("d")));
    assert_eq!(artifact.wasm_gz_sha256, Some(sample_sha256("a")));
    assert_eq!(
        artifact.canonical_embedded_config_sha256,
        Some(sample_sha256("c"))
    );
}

#[test]
fn promoted_deployment_plan_transform_summarizes_sealed_wasm_changes() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("f"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.require_byte_identical_wasm = false;
    input.source.kind = RoleArtifactSourceKindV1::LocalWasmGz;
    input.source.locator = Some("promoted/root.wasm.gz".to_string());
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![input],
    };

    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("sealed wasm transform should be produced");

    assert_eq!(
        transform.transform_id,
        "promotion-transform:promoted-plan-1"
    );
    assert_eq!(transform.target_plan_id, "plan-local-root");
    assert_eq!(transform.promoted_plan_id, "promoted-plan-1");
    assert_eq!(transform.roles.len(), 1);
    let role = &transform.roles[0];
    assert_eq!(role.role, "root");
    assert_eq!(role.promotion_level, PromotionArtifactLevelV1::SealedWasm);
    assert_eq!(role.source_kind, RoleArtifactSourceKindV1::LocalWasmGz);
    assert_eq!(
        role.source_locator.as_deref(),
        Some("promoted/root.wasm.gz")
    );
    assert_eq!(role.artifact_source_before, ArtifactSourceV1::LocalBuild);
    assert_eq!(role.artifact_source_after, ArtifactSourceV1::External);
    assert_eq!(role.wasm_gz_sha256_before, Some(sample_sha256("f")));
    assert_eq!(role.wasm_gz_sha256_after, Some(sample_sha256("a")));
    assert!(role.artifact_identity_changed);
    assert!(!role.embedded_config_changed);
    assert!(!role.target_materialization_preserved);
}

#[test]
fn promotion_plan_transform_text_reports_passive_summary() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("f"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.require_byte_identical_wasm = false;
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![input],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");

    let text = promotion_plan_transform_text(&transform);

    assert!(text.contains("Promotion plan transform"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("transform_id: promotion-transform:promoted-plan-1"));
    assert!(text.contains("target_plan_id: plan-local-root"));
    assert!(text.contains("promoted_plan_id: promoted-plan-1"));
    assert!(text.contains("promotion_plan_lineage_digest: "));
    assert!(text.contains("artifact_identity_changed: 1"));
    assert!(text.contains("embedded_config_changed: 0"));
    assert!(text.contains("target_materialization_preserved: 0"));
    assert!(
        text.contains("root SealedWasm/LocalWasmGz: artifact_identity_changed=true embedded_config_changed=false target_materialization_preserved=false")
    );
    assert!(text.contains("wasm_gz_sha256: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff -> aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
}

#[test]
fn promoted_deployment_plan_leaves_source_build_materialization_to_target_plan() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("f"));
    target_plan.role_artifacts[0].canonical_embedded_config_sha256 = Some(sample_sha256("1"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("c"));
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: target_plan.clone(),
        inputs: vec![input],
    };

    let promoted = promoted_deployment_plan_from_inputs(&request)
        .expect("source-build plan should be produced");

    assert_eq!(promoted.plan_id, "promoted-plan-1");
    assert_eq!(
        promoted.role_artifacts[0].wasm_gz_sha256,
        target_plan.role_artifacts[0].wasm_gz_sha256
    );
    assert_eq!(
        promoted.role_artifacts[0].canonical_embedded_config_sha256,
        target_plan.role_artifacts[0].canonical_embedded_config_sha256
    );
}

#[test]
fn promoted_deployment_plan_transform_marks_source_build_target_materialization_preserved() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("f"));
    target_plan.role_artifacts[0].canonical_embedded_config_sha256 = Some(sample_sha256("1"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("c"));
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![input],
    };

    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("source-build transform should be produced");

    let role = &transform.roles[0];
    assert_eq!(role.promotion_level, PromotionArtifactLevelV1::SourceBuild);
    assert_eq!(role.wasm_gz_sha256_before, Some(sample_sha256("f")));
    assert_eq!(role.wasm_gz_sha256_after, Some(sample_sha256("f")));
    assert_eq!(
        role.canonical_embedded_config_sha256_before,
        Some(sample_sha256("1"))
    );
    assert_eq!(
        role.canonical_embedded_config_sha256_after,
        Some(sample_sha256("1"))
    );
    assert!(!role.artifact_identity_changed);
    assert!(!role.embedded_config_changed);
    assert!(role.target_materialization_preserved);
}

#[test]
fn promoted_deployment_plan_transform_links_source_build_materialization_evidence() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_sha256 = Some(sample_sha256("5"));
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("6"));
    target_plan.role_artifacts[0].installed_module_hash = Some(sample_sha256("7"));
    target_plan.role_artifacts[0].candid_sha256 = Some(sample_sha256("8"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.expected_canonical_embedded_config_sha256 = target_plan.role_artifacts[0]
        .canonical_embedded_config_sha256
        .clone();
    let request = PromotionPlanTransformWithMaterializationRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![input],
        materialization_evidence: vec![sample_build_materialization_evidence()],
    };

    let transform = promoted_deployment_plan_transform_from_inputs_with_materialization(&request)
        .expect("source-build transform should link materialization evidence");

    let link = transform.roles[0]
        .source_build_materialization
        .as_ref()
        .expect("source-build role should carry materialization link");
    let expected_input_digest =
        build_materialization_input_digest(&sample_build_materialization_input());
    assert_eq!(link.role, "root");
    assert_eq!(link.evidence_id, "materialization-evidence-1");
    assert_eq!(link.materialization_evidence_digest.len(), 64);
    assert_eq!(link.materialization_input_digest, expected_input_digest);
    assert_eq!(link.wasm_gz_sha256, sample_sha256("6"));
    validate_promotion_plan_transform(&transform)
        .expect("materialization-linked transform should validate");
}

#[test]
fn promoted_deployment_plan_transform_requires_source_build_materialization_evidence() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    let request = PromotionPlanTransformWithMaterializationRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![input],
        materialization_evidence: Vec::new(),
    };

    let err = promoted_deployment_plan_transform_from_inputs_with_materialization(&request)
        .expect_err("source-build transform should require materialization evidence");

    assert!(matches!(
        err,
        PromotionPlanTransformError::MaterializationRoleMissing { role } if role == "root"
    ));
}

#[test]
fn promoted_deployment_plan_transform_rejects_duplicate_materialization_evidence() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_sha256 = Some(sample_sha256("5"));
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("6"));
    target_plan.role_artifacts[0].installed_module_hash = Some(sample_sha256("7"));
    target_plan.role_artifacts[0].candid_sha256 = Some(sample_sha256("8"));
    let request = PromotionPlanTransformWithMaterializationRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SourceBuild,
        )],
        materialization_evidence: vec![
            sample_build_materialization_evidence(),
            sample_build_materialization_evidence(),
        ],
    };

    let err = promoted_deployment_plan_transform_from_inputs_with_materialization(&request)
        .expect_err("duplicate materialization evidence should fail");

    assert!(matches!(
        err,
        PromotionPlanTransformError::DuplicateMaterializationRole { role } if role == "root"
    ));
}

#[test]
fn promotion_plan_transform_text_reports_source_build_materialization_link() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_sha256 = Some(sample_sha256("5"));
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("6"));
    target_plan.role_artifacts[0].installed_module_hash = Some(sample_sha256("7"));
    target_plan.role_artifacts[0].candid_sha256 = Some(sample_sha256("8"));
    let request = PromotionPlanTransformWithMaterializationRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SourceBuild,
        )],
        materialization_evidence: vec![sample_build_materialization_evidence()],
    };
    let transform = promoted_deployment_plan_transform_from_inputs_with_materialization(&request)
        .expect("source-build transform should link evidence");

    let text = promotion_plan_transform_text(&transform);
    let expected_input_digest =
        build_materialization_input_digest(&sample_build_materialization_input());

    assert!(text.contains("materialization_evidence_id: materialization-evidence-1"));
    assert!(text.contains(&format!(
        "materialization_input_digest: {expected_input_digest}"
    )));
    assert!(text.contains(
        "materialized_wasm_gz_sha256: 6666666666666666666666666666666666666666666666666666666666666666"
    ));
}

#[test]
fn promoted_deployment_plan_rejects_blocked_readiness() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("e"));
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![input],
    };

    let err =
        promoted_deployment_plan_from_inputs(&request).expect_err("blocked readiness should fail");
    assert!(matches!(
        err,
        PromotionPlanTransformError::ReadinessBlocked { blocker_count: 1 }
    ));
}

#[test]
fn promoted_deployment_plan_rejects_blank_plan_id() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: " ".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };

    let err =
        promoted_deployment_plan_from_inputs(&request).expect_err("blank plan id should fail");
    assert!(matches!(
        err,
        PromotionPlanTransformError::MissingRequiredField {
            field: "promoted_plan_id"
        }
    ));
}

#[test]
fn promotion_readiness_blocks_source_role_mismatch() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.role = "other".to_string();

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_source_role_mismatch"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn promotion_readiness_blocks_missing_target_role() {
    let plan = sample_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.role = "missing".to_string();
    input.source.role = "missing".to_string();

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.roles.is_empty());
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_target_role_missing"
            && finding.subject.as_deref() == Some("missing")
    }));
}

#[test]
fn promotion_readiness_blocks_invalid_artifact_source() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.expected_wasm_sha256 = None;
    input.source.expected_wasm_gz_sha256 = None;

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_artifact_source_invalid"
            && finding.subject.as_deref() == Some("root")
    }));
    validate_promotion_readiness(&readiness).expect("blocked readiness artifact should validate");
}

#[test]
fn promotion_readiness_validation_rejects_status_blocker_mismatch() {
    let plan = sample_promotion_target_plan();
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);
    readiness.status = PromotionReadinessStatusV1::Blocked;

    let err = validate_promotion_readiness(&readiness).expect_err("status should match blockers");
    assert!(matches!(
        err,
        PromotionReadinessError::StatusBlockerMismatch { .. }
    ));
}

#[test]
fn promotion_readiness_validation_rejects_stale_digest() {
    let plan = sample_promotion_target_plan();
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);
    readiness.promotion_readiness_digest = sample_sha256("9");

    let err =
        validate_promotion_readiness(&readiness).expect_err("stale readiness digest should fail");
    assert!(matches!(
        err,
        PromotionReadinessError::LinkageMismatch {
            field: "promotion_readiness_digest"
        }
    ));
}

#[test]
fn promotion_readiness_validation_rejects_duplicate_roles() {
    let plan = sample_promotion_target_plan();
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);
    readiness.roles.push(readiness.roles[0].clone());

    let err = validate_promotion_readiness(&readiness).expect_err("duplicate role should fail");
    assert!(matches!(
        err,
        PromotionReadinessError::DuplicateRole { role } if role == "root"
    ));
}

#[test]
fn promotion_readiness_validation_rejects_restage_state_mismatch() {
    let plan = sample_promotion_target_plan();
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);
    readiness.roles[0].target_store_has_artifact = Some(true);
    readiness.roles[0].restage_required = true;

    let err = validate_promotion_readiness(&readiness).expect_err("restage state should match");
    assert!(matches!(
        err,
        PromotionReadinessError::RestageStateMismatch { role } if role == "root"
    ));
}

#[test]
fn promotion_readiness_validation_rejects_bad_digest_shape() {
    let plan = sample_promotion_target_plan();
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let mut readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);
    readiness.roles[0].target_wasm_gz_sha256 = Some("NOT-A-DIGEST".to_string());

    let err = validate_promotion_readiness(&readiness).expect_err("digest should be checked");
    assert!(matches!(
        err,
        PromotionReadinessError::InvalidSha256Digest {
            field: "target_wasm_gz_sha256"
        }
    ));
}

#[test]
fn promotion_readiness_validation_rejects_warning_in_blockers() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("e"));
    let mut readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);
    readiness.blockers[0].severity = SafetySeverityV1::Warning;

    let err = validate_promotion_readiness(&readiness).expect_err("blockers must be hard failures");
    assert!(matches!(
        err,
        PromotionReadinessError::FindingSeverityMismatch {
            field: "blockers",
            severity: SafetySeverityV1::Warning
        }
    ));
}

#[test]
fn promotion_readiness_text_reports_passive_summary() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.target_store_has_artifact = Some(false);
    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    let text = promotion_readiness_text(&readiness);

    assert!(text.contains("Promotion readiness report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("status: ready"));
    assert!(text.contains("readiness_id: promotion-ready-1"));
    assert!(text.contains("promotion_readiness_digest:"));
    assert!(text.contains("target_plan_id: plan-local-root"));
    assert!(text.contains("restage_required: 1"));
    assert!(
        text.contains("root SealedWasm/LocalWasmGz: byte_identical_wasm=true embedded_config_identical=true restage_required=true")
    );
    assert!(text.contains(
        "source_wasm_gz_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    ));
    assert!(text.contains(
        "target_wasm_gz_sha256: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    ));
    assert!(text.contains("[promotion_target_store_restage_required] root"));
}

#[test]
fn promotion_readiness_text_reports_blockers() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("e"));
    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    let text = promotion_readiness_text(&readiness);

    assert!(text.contains("status: blocked"));
    assert!(text.contains("blockers: 1"));
    assert!(text.contains("[promotion_sealed_wasm_embedded_config_mismatch] root"));
    assert!(text.contains("embedded_config_identical=false"));
}

#[test]
fn promotion_readiness_text_reports_policy_blockers() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    let policy = sample_role_promotion_policy();
    let readiness = promotion_readiness_from_inputs_with_policy(
        "promotion-ready-1",
        &sample_promotion_target_plan(),
        &[input],
        &[policy],
    );

    let text = promotion_readiness_text(&readiness);

    assert!(text.contains("status: blocked"));
    assert!(text.contains("promotion_policy_level_not_allowed"));
    assert!(text.contains("promotion_policy_must_use_sealed_bytes"));
}

#[test]
fn inventory_round_trips_through_json() {
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: Some("raw".to_string()),
            canonical_embedded_config_sha256: Some("canonical".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string()],
            module_hash: Some("module".to_string()),
            status: Some("running".to_string()),
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: Some("canonical".to_string()),
            role_assignment_source: Some("registry".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: ".icp/local/canisters/root/root.wasm.gz".to_string(),
            file_sha256: Some("artifact-file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: Some("artifact".to_string()),
            payload_size_bytes: Some(42),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::Observed,
            role_epochs: vec![RoleEpochObservationV1 {
                role: "root".to_string(),
                observed_epoch: Some(1),
                status: ObservationStatusV1::Observed,
            }],
        },
        unresolved_observations: Vec::new(),
    };

    let encoded = serde_json::to_string_pretty(&inventory).expect("inventory should encode");
    let decoded =
        serde_json::from_str::<DeploymentInventoryV1>(&encoded).expect("inventory should decode");

    assert_eq!(decoded, inventory);
}

#[test]
fn receipt_diff_and_safety_report_support_not_evaluated_state() {
    let receipt = DeploymentReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        operation_id: "operation-1".to_string(),
        plan_id: "plan-local-root".to_string(),
        execution_context: None,
        operation_status: DeploymentExecutionStatusV1::InProgress,
        started_at: "2026-05-21T00:00:00Z".to_string(),
        finished_at: None,
        operator_principal: None,
        root_principal: Some("aaaaa-aa".to_string()),
        previous_observed_deployment_epoch: None,
        phase_receipts: vec![PhaseReceiptV1 {
            phase: "build_artifacts".to_string(),
            started_at: "2026-05-21T00:00:00Z".to_string(),
            finished_at: None,
            attempted_action: "build root artifact".to_string(),
            verified_postcondition: VerifiedPostconditionV1 {
                status: ObservationStatusV1::NotObserved,
                evidence: Vec::new(),
            },
        }],
        role_phase_receipts: vec![RolePhaseReceiptV1 {
            role: "root".to_string(),
            phase: "install_root".to_string(),
            result: RolePhaseResultV1::NotAttempted,
            previous_module_hash: None,
            target_module_hash: Some("module".to_string()),
            observed_module_hash_after: None,
            artifact_digest: Some("artifact".to_string()),
            canonical_embedded_config_sha256: None,
            error: None,
        }],
        final_inventory_id: None,
        command_result: DeploymentCommandResultV1::NotFinished,
    };
    let diff = DeploymentDiffV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_identity: sample_identity(),
        observed_identity: None,
        artifact_diff: Vec::new(),
        controller_diff: Vec::new(),
        pool_diff: Vec::new(),
        embedded_config_diff: Vec::new(),
        module_hash_diff: Vec::new(),
        verifier_readiness_diff: Vec::new(),
        resume_safety: ResumeSafetyV1 {
            status: SafetyStatusV1::NotEvaluated,
            reasons: vec!["inventory not collected".to_string()],
        },
        hard_failures: Vec::new(),
        warnings: Vec::new(),
        resumable_phases: Vec::new(),
    };
    let report = SafetyReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: "report-1".to_string(),
        diff_id: None,
        status: SafetyStatusV1::NotEvaluated,
        summary: "deployment safety has not been evaluated".to_string(),
        hard_failures: Vec::new(),
        warnings: Vec::new(),
        next_actions: vec!["collect deployment inventory".to_string()],
    };

    assert_json_round_trip(&receipt);
    assert_json_round_trip(&diff);
    assert_json_round_trip(&report);
}

#[test]
fn current_cli_execution_context_records_backend_roots_and_capabilities() {
    let context = current_cli_execution_context(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec![
            "/workspace/canic/.icp/local/canisters".to_string(),
            "/workspace/canic/target/wasm".to_string(),
        ],
    );

    assert_eq!(context.backend, DeploymentExecutorBackendV1::CurrentCli);
    assert!(has_executor_capabilities(
        &context.backend_capabilities,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    ));
    assert_json_round_trip(&context);
}

#[test]
fn current_cli_executor_returns_declared_execution_context() {
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );

    let context = executor.execution_context();

    assert_eq!(context.backend, DeploymentExecutorBackendV1::CurrentCli);
    assert_eq!(context.workspace_root.as_deref(), Some("/workspace/canic"));
    assert_eq!(context.icp_root.as_deref(), Some("/workspace/canic/.icp"));
    assert_eq!(
        context.artifact_roots,
        vec!["/workspace/canic/.icp/local/canisters".to_string()]
    );
    assert!(has_executor_capabilities(
        &context.backend_capabilities,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    ));
}

#[test]
fn testkit_preflight_context_has_no_local_workspace_roots() {
    let context = testkit_execution_context(vec!["memory://pocket-ic/artifacts".to_string()]);

    assert_eq!(context.backend, DeploymentExecutorBackendV1::PocketIc);
    assert_eq!(context.workspace_root, None);
    assert_eq!(context.icp_root, None);
    assert_eq!(
        context.artifact_roots,
        vec!["memory://pocket-ic/artifacts".to_string()]
    );
    assert_eq!(context.backend_capabilities, TESTKIT_PREFLIGHT_CAPABILITIES);
}

#[test]
fn missing_executor_capabilities_are_reported_in_required_order() {
    let available = [
        DeploymentExecutorCapabilityV1::CanisterStatus,
        DeploymentExecutorCapabilityV1::StageArtifact,
    ];
    let required = [
        DeploymentExecutorCapabilityV1::StageArtifact,
        DeploymentExecutorCapabilityV1::InstallCode,
        DeploymentExecutorCapabilityV1::CanisterStatus,
        DeploymentExecutorCapabilityV1::UpdateSettings,
    ];

    assert_eq!(
        missing_executor_capabilities(&available, &required),
        vec![
            DeploymentExecutorCapabilityV1::InstallCode,
            DeploymentExecutorCapabilityV1::UpdateSettings,
        ],
    );
}

#[test]
fn deployment_execution_preflight_accepts_safe_plan_and_capable_executor() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let authority = build_authority_reconciliation_plan(&check);
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );

    let preflight = deployment_execution_preflight(
        &check.plan,
        &check.report,
        &authority,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );

    assert_eq!(preflight.plan_id, check.plan.plan_id);
    assert_eq!(preflight.safety_report_id, check.report.report_id);
    assert_eq!(preflight.authority_plan_id, authority.plan_id);
    assert_eq!(
        preflight.status,
        DeploymentExecutionPreflightStatusV1::Ready
    );
    assert!(preflight.blockers.is_empty());
    assert!(preflight.missing_capabilities.is_empty());
    assert_eq!(
        preflight.planned_phases,
        vec![
            "resolve_root_canister",
            "build_artifacts",
            "materialize_artifacts",
            "execution_preflight",
            "emit_manifest",
            "install_root",
            "fund_root_pre_bootstrap",
            "stage_release_set",
            "resume_bootstrap",
            "wait_ready",
            "fund_root_post_ready",
            "write_install_state",
        ]
    );
    assert_json_round_trip(&preflight);
}

#[test]
fn deployment_execution_preflight_from_check_derives_authority_plan() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );

    let from_check = deployment_execution_preflight_from_check(
        &check,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );
    let authority = build_authority_reconciliation_plan(&check);
    let explicit = deployment_execution_preflight(
        &check.plan,
        &check.report,
        &authority,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );

    assert_eq!(from_check, explicit);
}

#[test]
fn testkit_preflight_validates_same_plan_shape_as_current_cli() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let current_cli = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );
    let pocket_ic = TestkitPreflightContext::new(vec!["memory://pocket-ic/artifacts".to_string()]);

    let current_cli_preflight = deployment_execution_preflight_from_check(
        &check,
        &current_cli,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );
    let pocket_ic_preflight = deployment_execution_preflight_from_check(
        &check,
        &pocket_ic,
        TESTKIT_PREFLIGHT_CAPABILITIES,
    );

    validate_deployment_execution_preflight_for_check(&check, &current_cli_preflight)
        .expect("current CLI preflight should validate against source check");
    validate_deployment_execution_preflight_for_check(&check, &pocket_ic_preflight)
        .expect("PocketIC preflight should validate against source check");
    assert_eq!(current_cli_preflight.plan_id, pocket_ic_preflight.plan_id);
    assert_eq!(
        current_cli_preflight.safety_report_id,
        pocket_ic_preflight.safety_report_id
    );
    assert_eq!(
        current_cli_preflight.authority_plan_id,
        pocket_ic_preflight.authority_plan_id
    );
    assert_eq!(
        current_cli_preflight.planned_phases,
        pocket_ic_preflight.planned_phases
    );
    assert_eq!(
        pocket_ic_preflight.backend,
        DeploymentExecutorBackendV1::PocketIc
    );
}

#[test]
fn deployment_execution_preflight_validation_accepts_check_derived_artifact() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );
    let preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );

    validate_deployment_execution_preflight(&preflight).expect("preflight should validate");
    validate_deployment_execution_preflight_for_check(&check, &preflight)
        .expect("preflight should match source check");
}

#[test]
fn deployment_execution_preflight_validation_rejects_mutated_status() {
    let check = sample_unknown_unsafe_check();
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );
    let mut preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );
    preflight.status = DeploymentExecutionPreflightStatusV1::Ready;

    let err = validate_deployment_execution_preflight(&preflight)
        .expect_err("ready status with blockers should fail");

    assert!(matches!(
        err,
        DeploymentExecutionPreflightError::StatusBlockerMismatch { .. }
    ));
}

#[test]
fn deployment_execution_preflight_validation_rejects_source_check_mismatch() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );
    let mut preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );
    preflight.plan_id = "other-plan".to_string();

    let err = validate_deployment_execution_preflight_for_check(&check, &preflight)
        .expect_err("preflight from another plan should fail");

    assert!(matches!(
        err,
        DeploymentExecutionPreflightError::SourceCheckMismatch {
            field: "plan_id",
            ..
        }
    ));
}

#[test]
fn deployment_execution_preflight_validation_rejects_capability_inconsistency() {
    let check = sample_unknown_unsafe_check();
    let executor = LimitedExecutor {
        context: DeploymentExecutionContextV1 {
            workspace_root: Some("/workspace/canic".to_string()),
            icp_root: Some("/workspace/canic/.icp".to_string()),
            artifact_roots: Vec::new(),
            backend: DeploymentExecutorBackendV1::Other {
                name: "limited-test-backend".to_string(),
            },
            backend_capabilities: vec![DeploymentExecutorCapabilityV1::CanisterStatus],
        },
    };
    let mut preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        &[DeploymentExecutorCapabilityV1::CanisterStatus],
    );
    preflight
        .missing_capabilities
        .push(DeploymentExecutorCapabilityV1::InstallCode);

    let err = validate_deployment_execution_preflight(&preflight)
        .expect_err("missing non-required capability should fail");

    assert!(matches!(
        err,
        DeploymentExecutionPreflightError::MissingCapabilityNotRequired {
            capability: DeploymentExecutorCapabilityV1::InstallCode
        }
    ));
}

#[test]
fn deployment_execution_preflight_v1_json_schema_shape_is_stable() {
    let check = sample_unknown_unsafe_check();
    let executor = LimitedExecutor {
        context: DeploymentExecutionContextV1 {
            workspace_root: Some("/workspace/canic".to_string()),
            icp_root: Some("/workspace/canic/.icp".to_string()),
            artifact_roots: Vec::new(),
            backend: DeploymentExecutorBackendV1::Other {
                name: "limited-test-backend".to_string(),
            },
            backend_capabilities: vec![DeploymentExecutorCapabilityV1::CanisterStatus],
        },
    };
    let preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        &[
            DeploymentExecutorCapabilityV1::CanisterStatus,
            DeploymentExecutorCapabilityV1::StageArtifact,
        ],
    );
    let value = serde_json::to_value(&preflight).expect("encode execution preflight");

    assert_object_keys(
        &value,
        &[
            "schema_version",
            "plan_id",
            "safety_report_id",
            "authority_plan_id",
            "backend",
            "status",
            "planned_phases",
            "required_capabilities",
            "missing_capabilities",
            "blockers",
        ],
    );
    assert_eq!(value["schema_version"], DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(value["plan_id"], "plan-local-root");
    assert_eq!(value["safety_report_id"], "report-1");
    assert_eq!(value["authority_plan_id"], "plan-local-root");
    assert_eq!(value["backend"]["Other"]["name"], "limited-test-backend");
    assert_eq!(value["status"], "Blocked");
    assert_eq!(value["required_capabilities"][0], "CanisterStatus");
    assert_eq!(value["required_capabilities"][1], "StageArtifact");
    assert_eq!(value["missing_capabilities"][0], "StageArtifact");
    assert_eq!(
        value["blockers"]
            .as_array()
            .expect("blockers should be array")
            .iter()
            .filter(|finding| finding["code"] == "executor_capability_missing")
            .count(),
        1
    );
}

#[test]
fn staging_receipt_v1_json_schema_shape_is_stable() {
    let receipt = StagingReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        role: "user_hub".to_string(),
        artifact_identity: "embedded:user_hub:0.43.4:abc123".to_string(),
        transport: ArtifactTransportV1::WasmStore,
        wasm_store_locator: Some("root:aaaaa-aa:bootstrap".to_string()),
        prepared_chunk_hashes: vec!["chunk-a".to_string(), "chunk-b".to_string()],
        published_chunk_count: 2,
        verified_postcondition: VerifiedPostconditionV1 {
            status: ObservationStatusV1::Observed,
            evidence: vec!["payload_sha256:abc123".to_string()],
        },
    };
    let value = serde_json::to_value(&receipt).expect("encode staging receipt");

    assert_object_keys(
        &value,
        &[
            "schema_version",
            "role",
            "artifact_identity",
            "transport",
            "wasm_store_locator",
            "prepared_chunk_hashes",
            "published_chunk_count",
            "verified_postcondition",
        ],
    );
    assert_eq!(value["schema_version"], DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(value["role"], "user_hub");
    assert_eq!(value["transport"], "WasmStore");
    assert_eq!(value["prepared_chunk_hashes"][1], "chunk-b");
    assert_eq!(value["published_chunk_count"], 2);
    assert_eq!(value["verified_postcondition"]["status"], "Observed");
}

#[test]
fn staging_receipt_evidence_preserves_transport_and_chunk_facts() {
    let receipts = vec![StagingReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        role: "scale_hub".to_string(),
        artifact_identity: "embedded:scale_hub:0.43.4:def456".to_string(),
        transport: ArtifactTransportV1::WasmStore,
        wasm_store_locator: Some("root:aaaaa-aa:bootstrap".to_string()),
        prepared_chunk_hashes: vec!["chunk-a".to_string()],
        published_chunk_count: 1,
        verified_postcondition: VerifiedPostconditionV1 {
            status: ObservationStatusV1::Observed,
            evidence: Vec::new(),
        },
    }];

    let evidence = staging_receipt_evidence(&receipts);

    assert!(evidence.contains(&"staging_receipts:1".to_string()));
    assert!(evidence.contains(&"staging_role:scale_hub".to_string()));
    assert!(evidence.contains(&"staging_transport:WasmStore".to_string()));
    assert!(evidence.contains(&"staging_chunks_prepared:1".to_string()));
    assert!(evidence.contains(&"staging_chunks_published:1".to_string()));
    assert!(evidence.contains(&"staging_postcondition:Observed".to_string()));
    assert!(evidence.contains(&"staging_wasm_store:root:aaaaa-aa:bootstrap".to_string()));
}

#[test]
fn promotion_wasm_store_identity_report_round_trips_through_json() {
    let report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![sample_wasm_store_staging_receipt()],
        },
    )
    .expect("wasm-store identity report should validate");

    assert_json_round_trip(&report);
    let encoded = serde_json::to_value(&report).expect("wasm-store report should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "report_id",
            "wasm_store_identity_report_digest",
            "status",
            "roles",
            "blockers",
        ],
    );
    assert_eq!(encoded["report_id"], "wasm-store-identity-1");
    assert!(
        encoded["wasm_store_identity_report_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(encoded["status"], "Ready");
    assert_eq!(encoded["roles"][0]["transport"], "WasmStore");
}

#[test]
fn promotion_wasm_store_identity_report_records_staging_locator() {
    let report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![sample_wasm_store_staging_receipt()],
        },
    )
    .expect("wasm-store identity report should validate");

    validate_promotion_wasm_store_identity_report(&report)
        .expect("generated report should validate");
    assert_eq!(report.roles.len(), 1);
    assert_eq!(report.roles[0].role, "root");
    assert_eq!(
        report.roles[0].wasm_store_locator.as_deref(),
        Some("root:aaaaa-aa:bootstrap")
    );
    assert_eq!(report.roles[0].published_chunk_count, 2);
    assert_eq!(report.status, PromotionReadinessStatusV1::Ready);
    assert_eq!(report.wasm_store_identity_report_digest.len(), 64);
}

#[test]
fn promotion_wasm_store_identity_report_blocks_non_wasm_store_transport() {
    let mut receipt = sample_wasm_store_staging_receipt();
    receipt.transport = ArtifactTransportV1::LocalCli;
    let report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![receipt],
        },
    )
    .expect("blocked wasm-store identity report should still validate");

    assert_eq!(report.status, PromotionReadinessStatusV1::Blocked);
    assert!(report.blockers.iter().any(|finding| {
        finding.code == "promotion_wasm_store_transport_mismatch"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn promotion_wasm_store_identity_report_blocks_missing_locator() {
    let mut receipt = sample_wasm_store_staging_receipt();
    receipt.wasm_store_locator = None;
    let report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![receipt],
        },
    )
    .expect("blocked wasm-store identity report should still validate");

    assert_eq!(report.status, PromotionReadinessStatusV1::Blocked);
    assert!(report.blockers.iter().any(|finding| {
        finding.code == "promotion_wasm_store_locator_missing"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn promotion_wasm_store_identity_report_blocks_unobserved_postcondition() {
    let mut receipt = sample_wasm_store_staging_receipt();
    receipt.verified_postcondition.status = ObservationStatusV1::Missing;
    let report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![receipt],
        },
    )
    .expect("blocked wasm-store identity report should still validate");

    assert_eq!(report.status, PromotionReadinessStatusV1::Blocked);
    assert!(report.blockers.iter().any(|finding| {
        finding.code == "promotion_wasm_store_postcondition_not_observed"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn promotion_wasm_store_identity_report_blocks_chunk_count_mismatch() {
    let mut receipt = sample_wasm_store_staging_receipt();
    receipt.published_chunk_count = 1;
    let report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![receipt],
        },
    )
    .expect("blocked wasm-store identity report should still validate");

    assert_eq!(report.status, PromotionReadinessStatusV1::Blocked);
    assert!(report.blockers.iter().any(|finding| {
        finding.code == "promotion_wasm_store_chunk_count_mismatch"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn promotion_wasm_store_identity_report_validation_rejects_stale_blockers() {
    let mut report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![sample_wasm_store_staging_receipt()],
        },
    )
    .expect("wasm-store identity report should validate");
    report.blockers.push(SafetyFindingV1 {
        code: "stale".to_string(),
        message: "stale".to_string(),
        severity: SafetySeverityV1::HardFailure,
        subject: Some("root".to_string()),
    });
    report.status = PromotionReadinessStatusV1::Blocked;

    let err = validate_promotion_wasm_store_identity_report(&report)
        .expect_err("stale blockers should fail");

    assert!(matches!(
        err,
        PromotionWasmStoreIdentityReportError::BlockerMismatch
    ));
}

#[test]
fn promotion_wasm_store_identity_report_validation_rejects_stale_digest() {
    let mut report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![sample_wasm_store_staging_receipt()],
        },
    )
    .expect("wasm-store identity report should validate");
    report.wasm_store_identity_report_digest = sample_sha256("9");

    let err = validate_promotion_wasm_store_identity_report(&report)
        .expect_err("stale wasm-store identity digest should fail");

    assert!(matches!(
        err,
        PromotionWasmStoreIdentityReportError::LinkageMismatch {
            field: "wasm_store_identity_report_digest"
        }
    ));
}

#[test]
fn promotion_wasm_store_identity_report_rejects_staging_schema_drift() {
    let mut receipt = sample_wasm_store_staging_receipt();
    receipt.schema_version += 1;

    let err = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![receipt],
        },
    )
    .expect_err("staging receipt schema drift should fail before projection");

    assert!(matches!(
        err,
        PromotionWasmStoreIdentityReportError::StagingReceiptSchemaVersionMismatch { .. }
    ));
}

#[test]
fn promotion_wasm_store_identity_report_text_reports_passive_summary() {
    let report = promotion_wasm_store_identity_report_from_staging(
        PromotionWasmStoreIdentityReportRequest {
            report_id: "wasm-store-identity-1".to_string(),
            staging_receipts: vec![sample_wasm_store_staging_receipt()],
        },
    )
    .expect("wasm-store identity report should validate");

    let text = promotion_wasm_store_identity_report_text(&report);

    assert!(text.contains("Promotion wasm-store identity report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("report_id: wasm-store-identity-1"));
    assert!(text.contains("wasm_store_identity_report_digest:"));
    assert!(text.contains("roles: 1"));
    assert!(text.contains(
        "root artifact=embedded:root:0.44.0:abc123 locator=root:aaaaa-aa:bootstrap chunks=2/2 postcondition=Observed"
    ));
}

#[test]
fn promotion_wasm_store_catalog_verification_round_trips_through_json() {
    let verification = sample_wasm_store_catalog_verification();

    assert_json_round_trip(&verification);
    let encoded = serde_json::to_value(&verification).expect("catalog verification should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "verification_id",
            "wasm_store_catalog_verification_digest",
            "wasm_store_identity_report_id",
            "status",
            "roles",
            "blockers",
        ],
    );
    assert_eq!(encoded["verification_id"], "wasm-store-catalog-1");
    assert_eq!(
        encoded["wasm_store_identity_report_id"],
        "wasm-store-identity-1"
    );
    assert!(
        encoded["wasm_store_catalog_verification_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    assert_eq!(encoded["status"], "Ready");
    assert_eq!(encoded["roles"][0]["catalog_matches"], true);
    assert_object_keys(
        &encoded["roles"][0],
        &[
            "role",
            "wasm_store_locator",
            "expected_artifact_identity",
            "observed_artifact_identity",
            "expected_published_chunk_count",
            "observed_published_chunk_count",
            "catalog_entry_present",
            "catalog_matches",
            "catalog_observation_digest",
        ],
    );
}

#[test]
fn promotion_wasm_store_catalog_verification_blocks_missing_entry() {
    let report = sample_wasm_store_identity_report();

    let verification =
        promotion_wasm_store_catalog_verification(PromotionWasmStoreCatalogVerificationRequest {
            verification_id: "wasm-store-catalog-1".to_string(),
            wasm_store_identity_report: report,
            catalog_entries: Vec::new(),
        })
        .expect("missing catalog entry should still produce blocked verification");

    assert_eq!(verification.status, PromotionReadinessStatusV1::Blocked);
    assert!(verification.blockers.iter().any(|finding| {
        finding.code == "promotion_wasm_store_catalog_entry_missing"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn promotion_wasm_store_catalog_verification_blocks_artifact_mismatch() {
    let report = sample_wasm_store_identity_report();
    let mut entry = sample_wasm_store_catalog_entry();
    entry.artifact_identity = "embedded:root:0.44.0:other".to_string();

    let verification =
        promotion_wasm_store_catalog_verification(PromotionWasmStoreCatalogVerificationRequest {
            verification_id: "wasm-store-catalog-1".to_string(),
            wasm_store_identity_report: report,
            catalog_entries: vec![entry],
        })
        .expect("catalog artifact mismatch should still produce blocked verification");

    assert_eq!(verification.status, PromotionReadinessStatusV1::Blocked);
    assert!(verification.blockers.iter().any(|finding| {
        finding.code == "promotion_wasm_store_catalog_artifact_mismatch"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn promotion_wasm_store_catalog_verification_blocks_chunk_count_mismatch() {
    let report = sample_wasm_store_identity_report();
    let mut entry = sample_wasm_store_catalog_entry();
    entry.published_chunk_count = 1;

    let verification =
        promotion_wasm_store_catalog_verification(PromotionWasmStoreCatalogVerificationRequest {
            verification_id: "wasm-store-catalog-1".to_string(),
            wasm_store_identity_report: report,
            catalog_entries: vec![entry],
        })
        .expect("catalog chunk-count mismatch should still produce blocked verification");

    assert_eq!(verification.status, PromotionReadinessStatusV1::Blocked);
    assert!(verification.blockers.iter().any(|finding| {
        finding.code == "promotion_wasm_store_catalog_chunk_count_mismatch"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn promotion_wasm_store_catalog_verification_rejects_duplicate_catalog_locator() {
    let report = sample_wasm_store_identity_report();
    let entry = sample_wasm_store_catalog_entry();

    let err =
        promotion_wasm_store_catalog_verification(PromotionWasmStoreCatalogVerificationRequest {
            verification_id: "wasm-store-catalog-1".to_string(),
            wasm_store_identity_report: report,
            catalog_entries: vec![entry.clone(), entry],
        })
        .expect_err("duplicate catalog locator should fail before verification");

    assert!(matches!(
        err,
        PromotionWasmStoreCatalogVerificationError::DuplicateLocator { locator }
            if locator == "root:aaaaa-aa:bootstrap"
    ));
}

#[test]
fn promotion_wasm_store_catalog_verification_validation_rejects_stale_blockers() {
    let mut verification = sample_wasm_store_catalog_verification();
    verification.blockers.push(SafetyFindingV1 {
        code: "stale".to_string(),
        message: "stale".to_string(),
        severity: SafetySeverityV1::HardFailure,
        subject: Some("root".to_string()),
    });
    verification.status = PromotionReadinessStatusV1::Blocked;

    let err = validate_promotion_wasm_store_catalog_verification(&verification)
        .expect_err("stale catalog blockers should fail");

    assert!(matches!(
        err,
        PromotionWasmStoreCatalogVerificationError::BlockerMismatch
    ));
}

#[test]
fn promotion_wasm_store_catalog_verification_validation_rejects_stale_observation_digest() {
    let mut verification = sample_wasm_store_catalog_verification();
    verification.roles[0].catalog_observation_digest = sample_sha256("9");

    let err = validate_promotion_wasm_store_catalog_verification(&verification)
        .expect_err("stale catalog observation digest should fail");

    assert!(matches!(
        err,
        PromotionWasmStoreCatalogVerificationError::RoleMismatch {
            role,
            field: "catalog_observation_digest"
        } if role == "root"
    ));
}

#[test]
fn promotion_wasm_store_catalog_verification_validation_rejects_stale_digest() {
    let mut verification = sample_wasm_store_catalog_verification();
    verification.wasm_store_catalog_verification_digest = sample_sha256("9");

    let err = validate_promotion_wasm_store_catalog_verification(&verification)
        .expect_err("stale catalog verification digest should fail");

    assert!(matches!(
        err,
        PromotionWasmStoreCatalogVerificationError::LinkageMismatch {
            field: "wasm_store_catalog_verification_digest"
        }
    ));
}

#[test]
fn promotion_wasm_store_catalog_verification_text_reports_passive_summary() {
    let verification = sample_wasm_store_catalog_verification();

    let text = promotion_wasm_store_catalog_verification_text(&verification);

    assert!(text.contains("Promotion wasm-store catalog verification"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("verification_id: wasm-store-catalog-1"));
    assert!(text.contains("wasm_store_catalog_verification_digest:"));
    assert!(text.contains("wasm_store_identity_report_id: wasm-store-identity-1"));
    assert!(text.contains("matching_roles: 1"));
    assert!(text.contains("missing_catalog_entries: 0"));
    assert!(text.contains("root locator=root:aaaaa-aa:bootstrap match=true"));
    assert!(text.contains("digest="));
}

#[test]
fn deployment_execution_preflight_text_reports_passive_readiness() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );
    let preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );

    let text = deployment_execution_preflight_text(&preflight);

    assert!(text.contains("Deployment execution preflight"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("status: ready"));
    assert!(text.contains("plan_id: plan-local-root"));
    assert!(text.contains("backend: CurrentCli"));
    assert!(text.contains("planned_phases:"));
    assert!(text.contains("  - install_root"));
    assert!(text.contains("required_capabilities:"));
    assert!(text.contains("  - StageArtifact"));
}

#[test]
fn deployment_execution_preflight_blocks_safety_authority_and_capability_gaps() {
    let mut check = sample_unknown_unsafe_check();
    check.report.status = SafetyStatusV1::Blocked;
    check.report.hard_failures.push(SafetyFindingV1 {
        code: "deployment_artifact_missing".to_string(),
        message: "planned artifact was not observed".to_string(),
        severity: SafetySeverityV1::HardFailure,
        subject: Some("root".to_string()),
    });
    let authority = build_authority_reconciliation_plan(&check);
    let executor = LimitedExecutor {
        context: DeploymentExecutionContextV1 {
            workspace_root: Some("/workspace/canic".to_string()),
            icp_root: Some("/workspace/canic/.icp".to_string()),
            artifact_roots: Vec::new(),
            backend: DeploymentExecutorBackendV1::Other {
                name: "limited-test-backend".to_string(),
            },
            backend_capabilities: vec![DeploymentExecutorCapabilityV1::CanisterStatus],
        },
    };

    let preflight = deployment_execution_preflight(
        &check.plan,
        &check.report,
        &authority,
        &executor,
        &[
            DeploymentExecutorCapabilityV1::CanisterStatus,
            DeploymentExecutorCapabilityV1::StageArtifact,
        ],
    );

    assert_eq!(
        preflight.status,
        DeploymentExecutionPreflightStatusV1::Blocked
    );
    assert_eq!(
        preflight.missing_capabilities,
        vec![DeploymentExecutorCapabilityV1::StageArtifact]
    );
    assert!(preflight.blockers.iter().any(|finding| {
        finding.code == "deployment_safety_blocked"
            && finding.subject.as_deref() == Some("report-1")
    }));
    assert!(
        preflight
            .blockers
            .iter()
            .any(|finding| finding.code == "deployment_artifact_missing")
    );
    assert!(
        preflight
            .blockers
            .iter()
            .any(|finding| finding.code == "authority_unsafe_blocked")
    );
    assert!(preflight.blockers.iter().any(|finding| {
        finding.code == "executor_capability_missing"
            && finding.subject.as_deref() == Some("StageArtifact")
    }));
}

#[test]
fn artifact_gate_receipt_records_materialized_artifact_evidence() {
    let mut plan = sample_plan();
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-22T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("canic.toml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string()],
            module_hash: None,
            status: None,
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: None,
            role_assignment_source: Some("local_install_state".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: Some("gzip".to_string()),
            payload_size_bytes: Some(10),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    };
    let diff = compare_plan_to_inventory(&plan, &inventory);
    let report = safety_report_from_diff("report-1", Some("diff-1".to_string()), &diff);
    let check = DeploymentCheckV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        check_id: "check-1".to_string(),
        plan,
        inventory,
        diff,
        report,
    };

    let phase = artifact_gate_phase_receipt(
        &check,
        "2026-05-22T00:00:00Z",
        Some("2026-05-22T00:00:01Z".to_string()),
    );
    let role_receipts = artifact_gate_role_phase_receipts(&check);
    let receipt = deployment_receipt_from_check(
        &check,
        "operation-1",
        "2026-05-22T00:00:00Z",
        Some("2026-05-22T00:00:01Z".to_string()),
        vec![phase.clone()],
        role_receipts.clone(),
        DeploymentCommandResultV1::Succeeded,
    );

    assert_eq!(phase.phase, "materialize_artifacts");
    assert_eq!(
        phase.verified_postcondition.status,
        ObservationStatusV1::Observed
    );
    assert_eq!(
        phase.verified_postcondition.evidence,
        vec!["artifact:root:sha256:file"]
    );
    assert_eq!(receipt.plan_id, "plan-local-root");
    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(receipt.final_inventory_id.as_deref(), Some("inventory-1"));
    assert_eq!(role_receipts.len(), 1);
    assert_eq!(role_receipts[0].role, "root");
    assert_eq!(
        role_receipts[0].result,
        RolePhaseResultV1::VerifiedAlreadyApplied
    );
    assert_eq!(role_receipts[0].artifact_digest.as_deref(), Some("file"));
    assert_eq!(receipt.role_phase_receipts, role_receipts);
    assert_eq!(receipt.phase_receipts, vec![phase]);
}

#[test]
fn execution_status_classifier_marks_failed_before_mutation_without_applied_roles() {
    let status = deployment_execution_status_for_receipt_parts(
        &DeploymentCommandResultV1::Failed {
            code: "preflight_blocked".to_string(),
            message: "blocked before mutation".to_string(),
        },
        &[sample_role_phase_receipt(RolePhaseResultV1::NotAttempted)],
    );

    assert_eq!(status, DeploymentExecutionStatusV1::FailedBeforeMutation);
}

#[test]
fn execution_status_classifier_marks_failed_after_mutation_with_applied_role() {
    let status = deployment_execution_status_for_receipt_parts(
        &DeploymentCommandResultV1::Failed {
            code: "post_install_failed".to_string(),
            message: "failed after one role applied".to_string(),
        },
        &[sample_role_phase_receipt(RolePhaseResultV1::Applied)],
    );

    assert_eq!(status, DeploymentExecutionStatusV1::FailedAfterMutation);
}

#[test]
fn execution_status_classifier_marks_partially_applied_with_applied_and_failed_roles() {
    let role_phase_receipts = vec![
        sample_role_phase_receipt(RolePhaseResultV1::Applied),
        RolePhaseReceiptV1 {
            role: "user_hub".to_string(),
            ..sample_role_phase_receipt(RolePhaseResultV1::Failed)
        },
    ];
    let status = deployment_execution_status_for_receipt_parts(
        &DeploymentCommandResultV1::Failed {
            code: "multi_role_install_failed".to_string(),
            message: "one role applied and another failed".to_string(),
        },
        &role_phase_receipts,
    );

    assert_eq!(status, DeploymentExecutionStatusV1::PartiallyApplied);
}

#[test]
fn deployment_receipt_from_check_derives_partial_status_from_role_receipts() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let receipt = deployment_receipt_from_check(
        &check,
        "operation-1",
        "2026-05-22T00:00:00Z",
        Some("2026-05-22T00:00:01Z".to_string()),
        Vec::new(),
        vec![
            sample_role_phase_receipt(RolePhaseResultV1::Applied),
            RolePhaseReceiptV1 {
                role: "user_hub".to_string(),
                ..sample_role_phase_receipt(RolePhaseResultV1::Failed)
            },
        ],
        DeploymentCommandResultV1::Failed {
            code: "partial".to_string(),
            message: "partial execution".to_string(),
        },
    );

    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::PartiallyApplied
    );
}

#[test]
fn artifact_gate_receipt_records_missing_artifact_postcondition() {
    let temp = TempWorkspace::new("canic-host-artifact-gate-receipt");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");

    let check = check_local_deployment(&LocalDeploymentCheckRequest {
        deployment_name: "demo".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        observed_at: "2026-05-22T00:00:00Z".to_string(),
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    })
    .expect("check local deployment");

    let phase = artifact_gate_phase_receipt(&check, "start", Some("finish".to_string()));
    let role_receipts = artifact_gate_role_phase_receipts(&check);

    assert_eq!(
        phase.verified_postcondition.status,
        ObservationStatusV1::Missing
    );
    assert!(
        phase
            .verified_postcondition
            .evidence
            .iter()
            .any(|evidence| evidence == "artifact:user_hub:missing")
    );
    assert!(role_receipts.iter().any(|receipt| {
        receipt.role == "user_hub"
            && receipt.result == RolePhaseResultV1::Failed
            && receipt
                .error
                .as_deref()
                .is_some_and(|error| error.contains("artifact_missing"))
    }));
}

#[test]
fn receipt_aware_diff_marks_verified_phase_resumable() {
    let plan = sample_plan();
    let inventory = sample_matching_inventory();
    let receipt = sample_receipt_with_phase(
        "plan-local-root",
        Some("aaaaa-aa"),
        ObservationStatusV1::Observed,
        RolePhaseResultV1::VerifiedAlreadyApplied,
    );

    let diff = compare_plan_inventory_and_receipt(&plan, &inventory, &receipt);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Safe);
    assert_eq!(
        diff.resumable_phases,
        vec!["materialize_artifacts".to_string()]
    );
    assert_eq!(
        diff.resume_safety.reasons,
        vec!["no blocking deployment truth differences were found".to_string()]
    );
}

#[test]
fn receipt_aware_diff_blocks_plan_mismatch_resume() {
    let plan = sample_plan();
    let inventory = sample_matching_inventory();
    let receipt = sample_receipt_with_phase(
        "old-plan",
        Some("aaaaa-aa"),
        ObservationStatusV1::Observed,
        RolePhaseResultV1::VerifiedAlreadyApplied,
    );

    let diff = compare_plan_inventory_and_receipt(&plan, &inventory, &receipt);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.resumable_phases.is_empty());
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "receipt_plan_mismatch")
    );
}

#[test]
fn receipt_aware_diff_does_not_resume_unverified_phase() {
    let plan = sample_plan();
    let inventory = sample_matching_inventory();
    let receipt = sample_receipt_with_phase(
        "plan-local-root",
        Some("aaaaa-aa"),
        ObservationStatusV1::Missing,
        RolePhaseResultV1::Failed,
    );

    let diff = compare_plan_inventory_and_receipt(&plan, &inventory, &receipt);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.resumable_phases.is_empty());
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "receipt_postcondition_unverified")
    );
}

#[test]
fn receipt_aware_diff_blocks_execution_status_mismatch() {
    let plan = sample_plan();
    let inventory = sample_matching_inventory();
    let mut receipt = sample_receipt_with_phase(
        "plan-local-root",
        Some("aaaaa-aa"),
        ObservationStatusV1::Observed,
        RolePhaseResultV1::VerifiedAlreadyApplied,
    );
    receipt.command_result = DeploymentCommandResultV1::Failed {
        code: "partial".to_string(),
        message: "role application failed".to_string(),
    };
    receipt.operation_status = DeploymentExecutionStatusV1::PartiallyApplied;
    receipt.role_phase_receipts = vec![
        sample_role_phase_receipt(RolePhaseResultV1::Applied),
        RolePhaseReceiptV1 {
            role: "user_hub".to_string(),
            ..sample_role_phase_receipt(RolePhaseResultV1::NotAttempted)
        },
    ];

    let diff = compare_plan_inventory_and_receipt(&plan, &inventory, &receipt);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.resumable_phases.is_empty());
    let has_status_mismatch = diff.hard_failures.iter().any(|finding| {
        finding.code == "receipt_execution_status_mismatch"
            && finding.subject.as_deref() == Some("receipt.operation_status")
    });
    assert!(has_status_mismatch);
}

#[test]
fn receipt_aware_diff_blocks_conflicting_duplicate_phase_receipt() {
    let plan = sample_plan();
    let inventory = sample_matching_inventory();
    let mut receipt = sample_receipt_with_phase(
        "plan-local-root",
        Some("aaaaa-aa"),
        ObservationStatusV1::Observed,
        RolePhaseResultV1::VerifiedAlreadyApplied,
    );
    let mut conflicting = receipt.phase_receipts[0].clone();
    conflicting.verified_postcondition.status = ObservationStatusV1::Missing;
    receipt.phase_receipts.push(conflicting);

    let diff = compare_plan_inventory_and_receipt(&plan, &inventory, &receipt);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.resumable_phases.is_empty());
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "receipt_phase_conflict"
                && finding.subject.as_deref() == Some("materialize_artifacts"))
    );
}

#[test]
fn receipt_aware_diff_warns_for_duplicate_identical_phase_receipt() {
    let plan = sample_plan();
    let inventory = sample_matching_inventory();
    let mut receipt = sample_receipt_with_phase(
        "plan-local-root",
        Some("aaaaa-aa"),
        ObservationStatusV1::Observed,
        RolePhaseResultV1::VerifiedAlreadyApplied,
    );
    receipt
        .phase_receipts
        .push(receipt.phase_receipts[0].clone());

    let diff = compare_plan_inventory_and_receipt(&plan, &inventory, &receipt);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert_eq!(
        diff.resumable_phases,
        vec!["materialize_artifacts".to_string()]
    );
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_receipt_phase"
                && finding.subject.as_deref() == Some("materialize_artifacts"))
    );
}

#[test]
fn receipt_aware_diff_blocks_conflicting_duplicate_role_phase_receipt() {
    let plan = sample_plan();
    let inventory = sample_matching_inventory();
    let mut receipt = sample_receipt_with_phase(
        "plan-local-root",
        Some("aaaaa-aa"),
        ObservationStatusV1::Observed,
        RolePhaseResultV1::VerifiedAlreadyApplied,
    );
    receipt.role_phase_receipts[0].phase = "materialize_artifacts".to_string();
    let mut conflicting = receipt.role_phase_receipts[0].clone();
    conflicting.result = RolePhaseResultV1::Failed;
    conflicting.error = Some("artifact_missing".to_string());
    receipt.role_phase_receipts.push(conflicting);

    let diff = compare_plan_inventory_and_receipt(&plan, &inventory, &receipt);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.resumable_phases.is_empty());
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "receipt_role_phase_conflict"
                && finding.subject.as_deref() == Some("root:materialize_artifacts"))
    );
}

#[test]
fn receipt_aware_diff_warns_for_duplicate_identical_role_phase_receipt() {
    let plan = sample_plan();
    let inventory = sample_matching_inventory();
    let mut receipt = sample_receipt_with_phase(
        "plan-local-root",
        Some("aaaaa-aa"),
        ObservationStatusV1::Observed,
        RolePhaseResultV1::VerifiedAlreadyApplied,
    );
    receipt
        .role_phase_receipts
        .push(receipt.role_phase_receipts[0].clone());

    let diff = compare_plan_inventory_and_receipt(&plan, &inventory, &receipt);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert_eq!(
        diff.resumable_phases,
        vec!["materialize_artifacts".to_string()]
    );
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_receipt_role_phase"
                && finding.subject.as_deref() == Some("root:materialize_artifacts"))
    );
}

#[test]
fn local_check_builds_plan_inventory_diff_and_report() {
    let temp = TempWorkspace::new("canic-host-local-check");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_release_set_manifest(&icp_root);

    let check = check_local_deployment(&LocalDeploymentCheckRequest {
        deployment_name: "demo".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    })
    .expect("check local deployment");

    assert_eq!(check.schema_version, DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(check.check_id, "local:local:demo:check");
    assert_eq!(check.plan.plan_id, "local:local:demo:plan");
    assert_eq!(check.inventory.inventory_id, "local:local:demo");
    assert_eq!(check.diff.resume_safety.status, check.report.status);
    assert!(
        check
            .diff
            .hard_failures
            .iter()
            .any(|finding| finding.code == "artifact_missing")
    );
    assert_eq!(check.report.status, SafetyStatusV1::Blocked);
}

#[test]
fn local_inventory_collects_configured_roles_and_artifacts_without_live_queries() {
    let temp = TempWorkspace::new("canic-host-local-inventory");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");

    let artifact_path = icp_root
        .join(".icp")
        .join("local")
        .join("canisters")
        .join("root")
        .join("root.wasm.gz");
    fs::create_dir_all(artifact_path.parent().expect("artifact parent"))
        .expect("create artifact dir");
    fs::write(&artifact_path, b"artifact").expect("write artifact");
    write_release_set_manifest(&icp_root);

    let inventory = collect_local_deployment_inventory(&LocalInventoryRequest {
        deployment_name: "demo".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        observed_at: "2026-05-21T00:00:00Z".to_string(),
    })
    .expect("collect inventory");

    assert_eq!(inventory.schema_version, DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(inventory.inventory_id, "local:local:demo");
    assert_sha256_len(inventory.local_config.raw_config_sha256.as_ref());
    assert_sha256_len(
        inventory
            .local_config
            .canonical_embedded_config_sha256
            .as_ref(),
    );
    let observed_identity = inventory.observed_identity.as_ref().expect("identity");
    assert_sha256_len(observed_identity.deployment_manifest_digest.as_ref());
    assert_sha256_len(observed_identity.canonical_runtime_config_digest.as_ref());
    assert_sha256_len(observed_identity.role_topology_hash.as_ref());
    assert_sha256_len(observed_identity.artifact_set_digest.as_ref());
    assert_sha256_len(observed_identity.pool_identity_set_digest.as_ref());
    assert_eq!(inventory.observed_artifacts.len(), 1);
    assert_eq!(inventory.observed_artifacts[0].role, "root");
    assert_eq!(inventory.observed_artifacts[0].payload_size_bytes, Some(8));
    assert_eq!(
        inventory.observed_artifacts[0].file_sha256_source,
        Some(ArtifactDigestSourceV1::ObservedFileDigest)
    );
    assert_sha256_len(inventory.observed_artifacts[0].file_sha256.as_ref());
    assert!(
        inventory
            .unresolved_observations
            .iter()
            .any(|gap| gap.key == "local_artifacts.user_hub")
    );
}

#[test]
fn live_root_status_observation_maps_status_controllers_and_module_hash() {
    let state = sample_install_state("aaaaa-aa");
    let report = IcpCanisterStatusReport {
        id: "aaaaa-aa".to_string(),
        name: Some("root".to_string()),
        status: "Running".to_string(),
        settings: Some(IcpCanisterStatusSettings {
            controllers: vec!["aaaaa-aa".to_string()],
            compute_allocation: Some("0".to_string()),
            memory_allocation: None,
            freezing_threshold: None,
            reserved_cycles_limit: None,
            wasm_memory_limit: None,
            wasm_memory_threshold: None,
            log_memory_limit: None,
        }),
        module_hash: Some("0xABCD".to_string()),
        memory_size: None,
        cycles: None,
        reserved_cycles: None,
        idle_cycles_burned_per_day: None,
    };

    let observed = observed_root_from_status(&state, &report);

    assert_eq!(observed.canister_id, "aaaaa-aa");
    assert_eq!(
        observed.control_class,
        CanisterControlClassV1::DeploymentControlled
    );
    assert_eq!(observed.controllers, vec!["aaaaa-aa"]);
    assert_eq!(observed.module_hash.as_deref(), Some("abcd"));
    assert_eq!(observed.status.as_deref(), Some("Running"));
    assert_eq!(
        observed.role_assignment_source.as_deref(),
        Some("icp_canister_status")
    );
}

#[test]
fn registry_entries_map_configured_pool_roles_to_observed_pool() {
    let mut gaps = Vec::new();
    let entries = vec![
        RegistryEntry {
            pid: "root-id".to_string(),
            role: Some("root".to_string()),
            kind: None,
            parent_pid: None,
            module_hash: None,
        },
        RegistryEntry {
            pid: "shard-id".to_string(),
            role: Some("user_shard".to_string()),
            kind: None,
            parent_pid: Some("user_hub-id".to_string()),
            module_hash: Some("module".to_string()),
        },
        RegistryEntry {
            pid: "user_hub-id".to_string(),
            role: Some("user_hub".to_string()),
            kind: None,
            parent_pid: Some("root-id".to_string()),
            module_hash: None,
        },
    ];
    let expectations = vec![ConfiguredPoolExpectation {
        pool: "user_shards".to_string(),
        canister_role: "user_shard".to_string(),
    }];

    let observed = registry_entries_to_observed_pool("root-id", &entries, &expectations, &mut gaps);

    assert_eq!(
        observed,
        vec![ObservedPoolCanisterV1 {
            pool: "user_shards".to_string(),
            canister_id: "shard-id".to_string(),
            role: Some("user_shard".to_string()),
            control_class: CanisterControlClassV1::CanicManagedPool,
        }]
    );
    assert!(gaps.is_empty());
}

#[test]
fn registry_entries_map_roles_to_observed_canisters_without_controller_authority() {
    let entries = vec![
        RegistryEntry {
            pid: "root-id".to_string(),
            role: Some("root".to_string()),
            kind: None,
            parent_pid: None,
            module_hash: None,
        },
        RegistryEntry {
            pid: "user_hub-id".to_string(),
            role: Some("user_hub".to_string()),
            kind: None,
            parent_pid: Some("root-id".to_string()),
            module_hash: Some("0xABCDEF".to_string()),
        },
    ];

    let observed = registry_entries_to_observed_canisters("root-id", &entries);

    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].canister_id, "user_hub-id");
    assert_eq!(observed[0].role.as_deref(), Some("user_hub"));
    assert_eq!(
        observed[0].control_class,
        CanisterControlClassV1::CanicManagedPool
    );
    assert!(observed[0].controllers.is_empty());
    assert_eq!(observed[0].module_hash.as_deref(), Some("abcdef"));
    assert_eq!(
        observed[0].role_assignment_source.as_deref(),
        Some("subnet_registry")
    );
}

#[test]
fn registry_observation_can_be_enriched_with_live_status() {
    let mut observed = registry_entries_to_observed_canisters(
        "root-id",
        &[RegistryEntry {
            pid: "user_hub-id".to_string(),
            role: Some("user_hub".to_string()),
            kind: None,
            parent_pid: Some("root-id".to_string()),
            module_hash: Some("stale".to_string()),
        }],
    )
    .pop()
    .expect("registry observation");
    let report = IcpCanisterStatusReport {
        id: "user_hub-id".to_string(),
        name: Some("user_hub".to_string()),
        status: "Running".to_string(),
        settings: Some(IcpCanisterStatusSettings {
            controllers: vec!["root-id".to_string()],
            compute_allocation: Some("0".to_string()),
            memory_allocation: None,
            freezing_threshold: None,
            reserved_cycles_limit: None,
            wasm_memory_limit: None,
            wasm_memory_threshold: None,
            log_memory_limit: None,
        }),
        module_hash: Some("0xCAFE".to_string()),
        memory_size: None,
        cycles: None,
        reserved_cycles: None,
        idle_cycles_burned_per_day: None,
    };

    apply_live_status_to_registry_observation(&mut observed, &report);

    assert_eq!(
        observed.control_class,
        CanisterControlClassV1::CanicManagedPool
    );
    assert_eq!(observed.controllers, vec!["root-id"]);
    assert_eq!(observed.module_hash.as_deref(), Some("cafe"));
    assert_eq!(observed.status.as_deref(), Some("Running"));
    assert_eq!(
        observed.role_assignment_source.as_deref(),
        Some("subnet_registry+icp_canister_status")
    );
}

#[test]
fn observed_pool_control_uses_enriched_canister_status() {
    let mut observed_pool = vec![ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "shard-id".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    }];
    let observed_canisters = vec![ObservedCanisterV1 {
        canister_id: "shard-id".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::UnknownUnsafe,
        controllers: vec!["external-controller".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("root-id".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    }];

    apply_canister_control_to_observed_pool(&mut observed_pool, &observed_canisters);

    assert_eq!(
        observed_pool[0].control_class,
        CanisterControlClassV1::UnknownUnsafe
    );
}

#[test]
fn registry_entries_report_ambiguous_pool_role_mapping() {
    let mut gaps = Vec::new();
    let entries = vec![RegistryEntry {
        pid: "worker-id".to_string(),
        role: Some("worker".to_string()),
        kind: None,
        parent_pid: Some("root-id".to_string()),
        module_hash: None,
    }];
    let expectations = vec![
        ConfiguredPoolExpectation {
            pool: "workers_a".to_string(),
            canister_role: "worker".to_string(),
        },
        ConfiguredPoolExpectation {
            pool: "workers_b".to_string(),
            canister_role: "worker".to_string(),
        },
    ];

    let observed = registry_entries_to_observed_pool("root-id", &entries, &expectations, &mut gaps);

    assert!(observed.is_empty());
    assert!(
        gaps.iter()
            .any(|gap| gap.key == "live_subnet_registry.pool.worker")
    );
}

#[test]
fn local_inventory_reports_missing_config_as_observation_gap() {
    let temp = TempWorkspace::new("canic-host-local-inventory-missing-config");

    let inventory = collect_local_deployment_inventory(&LocalInventoryRequest {
        deployment_name: "demo".to_string(),
        network: "local".to_string(),
        workspace_root: temp.path().join("workspace"),
        icp_root: temp.path().join("icp"),
        config_path: None,
        observed_at: "2026-05-21T00:00:00Z".to_string(),
    })
    .expect("collect inventory");

    assert_eq!(inventory.inventory_id, "local:local:demo");
    assert!(
        inventory
            .unresolved_observations
            .iter()
            .any(|gap| gap.key == "local_config.fleet_name")
    );
    assert!(
        inventory
            .unresolved_observations
            .iter()
            .any(|gap| gap.key == "local_config.roles")
    );
}

#[test]
fn local_artifact_manifest_collects_roles_and_release_set_hashes() {
    let temp = TempWorkspace::new("canic-host-local-artifact-manifest");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "wasm_store", b"wasm-store-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_release_set_manifest(&icp_root);

    let manifest = collect_local_role_artifact_manifest(&LocalArtifactManifestRequest {
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
    });

    assert_eq!(manifest.manifest_id, "local:local:demo:artifacts");
    assert_eq!(manifest.role_artifacts.len(), 3);
    let wasm_store = manifest
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == "wasm_store")
        .expect("wasm_store artifact");
    assert_eq!(wasm_store.source, ArtifactSourceV1::WasmStore);
    assert_eq!(
        wasm_store.observed_wasm_gz_file_sha256_source,
        Some(ArtifactDigestSourceV1::ObservedFileDigest)
    );
    let user_hub = manifest
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == "user_hub")
        .expect("user_hub artifact");
    assert_eq!(user_hub.wasm_gz_sha256.as_deref(), Some("user-hub-hash"));
    assert_eq!(
        user_hub.wasm_gz_sha256_source,
        Some(ArtifactDigestSourceV1::ReleaseSetManifest)
    );
    assert_eq!(user_hub.wasm_gz_size_bytes, Some(17));
    assert_eq!(
        user_hub.observed_wasm_gz_file_sha256_source,
        Some(ArtifactDigestSourceV1::ObservedFileDigest)
    );
    assert_eq!(
        user_hub
            .observed_wasm_gz_file_sha256
            .as_ref()
            .map(String::len),
        Some(64)
    );
    let root = manifest
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == "root")
        .expect("root artifact");
    assert_eq!(root.wasm_gz_sha256, None);
    assert_eq!(root.wasm_gz_sha256_source, None);
    assert!(manifest.unresolved_artifacts.is_empty());
}

#[test]
fn local_artifact_manifest_reports_network_artifact_fallback() {
    let temp = TempWorkspace::new("canic-host-local-artifact-manifest-fallback");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");

    let manifest = collect_local_role_artifact_manifest(&LocalArtifactManifestRequest {
        network: "ic".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
    });

    assert!(
        manifest
            .unresolved_artifacts
            .iter()
            .any(|gap| gap.key == "local_artifacts.network_fallback")
    );
}

#[test]
fn local_artifact_manifest_records_missing_artifacts_as_gaps() {
    let temp = TempWorkspace::new("canic-host-local-artifact-manifest-missing");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");

    let manifest = collect_local_role_artifact_manifest(&LocalArtifactManifestRequest {
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
    });

    assert!(
        manifest
            .unresolved_artifacts
            .iter()
            .any(|gap| gap.key == "local_artifacts.release_set_manifest")
    );
    assert!(
        manifest
            .unresolved_artifacts
            .iter()
            .any(|gap| gap.key == "local_artifacts.user_hub")
    );
    assert!(
        manifest
            .unresolved_artifacts
            .iter()
            .any(|gap| gap.key == "local_artifacts.wasm_store")
    );
}

#[test]
fn local_plan_uses_configured_roles_and_local_artifact_manifest() {
    let temp = TempWorkspace::new("canic-host-local-plan");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "wasm_store", b"wasm-store-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_release_set_manifest(&icp_root);

    let plan = build_local_deployment_plan(&LocalDeploymentPlanRequest {
        deployment_name: "demo-local".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    });

    assert_eq!(plan.plan_id, "local:local:demo-local:plan");
    assert_eq!(plan.deployment_identity.deployment_name, "demo-local");
    assert_eq!(
        plan.deployment_identity
            .deployment_manifest_digest
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.deployment_identity
            .canonical_runtime_config_digest
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.deployment_identity
            .authority_profile_hash
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.deployment_identity
            .role_topology_hash
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.deployment_identity
            .artifact_set_digest
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.deployment_identity
            .pool_identity_set_digest
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(
        plan.role_artifacts[0]
            .raw_config_sha256
            .as_ref()
            .map(String::len),
        Some(64)
    );
    assert_eq!(plan.fleet_template, "demo");
    assert_eq!(plan.runtime_variant, "local");
    assert_eq!(plan.role_artifacts.len(), 3);
    assert!(
        plan.role_artifacts
            .iter()
            .all(|artifact| artifact.build_profile == "fast")
    );
    assert_plan_has_implicit_wasm_store_artifact(&plan);
    assert_plan_has_user_hub_release_artifact(&plan);
    assert_eq!(
        plan.expected_canisters
            .iter()
            .map(|canister| canister.role.as_str())
            .collect::<Vec<_>>(),
        vec!["root", "wasm_store", "user_hub"]
    );
    assert!(
        plan.unresolved_assumptions
            .iter()
            .any(|assumption| assumption.key == "local_state.root_canister_id")
    );
}

fn assert_plan_has_implicit_wasm_store_artifact(plan: &DeploymentPlanV1) {
    assert!(
        plan.role_artifacts
            .iter()
            .any(|artifact| artifact.role == "wasm_store"
                && artifact.source == ArtifactSourceV1::WasmStore
                && artifact.observed_wasm_gz_file_sha256_source
                    == Some(ArtifactDigestSourceV1::ObservedFileDigest))
    );
}

fn assert_plan_has_user_hub_release_artifact(plan: &DeploymentPlanV1) {
    assert!(
        plan.role_artifacts
            .iter()
            .any(|artifact| artifact.role == "user_hub"
                && artifact.wasm_gz_sha256.as_deref() == Some("user-hub-hash")
                && artifact.wasm_gz_sha256_source
                    == Some(ArtifactDigestSourceV1::ReleaseSetManifest)
                && artifact.observed_wasm_gz_file_sha256_source
                    == Some(ArtifactDigestSourceV1::ObservedFileDigest))
    );
}

#[test]
fn local_plan_uses_configured_controllers_as_expected_authority() {
    let temp = TempWorkspace::new("canic-host-local-plan-controllers");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(
        config_dir.join("canic.toml"),
        r#"
controllers = [
  "zbf4m-zw3nk-6owqc-qmluz-xhwxt-2pkky-xhjy2-kqxor-qzxsn-6d2bz-nae",
  "aaaaa-aa",
]
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");

    let plan = build_local_deployment_plan(&LocalDeploymentPlanRequest {
        deployment_name: "demo-local".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    });

    assert_eq!(
        plan.authority_profile.expected_controllers,
        vec![
            "aaaaa-aa".to_string(),
            "zbf4m-zw3nk-6owqc-qmluz-xhwxt-2pkky-xhjy2-kqxor-qzxsn-6d2bz-nae".to_string(),
        ]
    );
    assert!(plan.authority_profile.staging_controllers.is_empty());
    assert!(plan.authority_profile.emergency_controllers.is_empty());
    assert!(
        plan.unresolved_assumptions
            .iter()
            .any(|assumption| assumption.key == "local_state.root_canister_id")
    );
}

#[test]
fn local_plan_uses_install_state_root_as_expected_canister() {
    let temp = TempWorkspace::new("canic-host-local-plan-root-state");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(config_dir.join("canic.toml"), SAMPLE_CONFIG).expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "wasm_store", b"wasm-store-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_release_set_manifest(&icp_root);
    let state_path = icp_root.join(".canic/local/fleets/demo.json");
    fs::create_dir_all(state_path.parent().expect("state parent")).expect("create state dir");
    fs::write(
        state_path,
        serde_json::to_vec_pretty(&sample_install_state("aaaaa-aa")).expect("encode state"),
    )
    .expect("write install state");

    let plan = build_local_deployment_plan(&LocalDeploymentPlanRequest {
        deployment_name: "demo-local".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    });

    assert_eq!(
        plan.deployment_identity.root_principal.as_deref(),
        Some("aaaaa-aa")
    );
    assert_eq!(
        plan.trust_domain.root_trust_anchor.as_deref(),
        Some("aaaaa-aa")
    );
    assert!(
        plan.expected_canisters
            .iter()
            .any(|canister| canister.role == "root"
                && canister.canister_id.as_deref() == Some("aaaaa-aa"))
    );
    assert!(plan.unresolved_assumptions.is_empty());
}

#[test]
fn local_plan_uses_configured_pools_as_expected_pool_identities() {
    let temp = TempWorkspace::new("canic-host-local-plan-pools");
    let workspace_root = temp.path().join("workspace");
    let icp_root = temp.path().join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(
        config_dir.join("canic.toml"),
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "singleton"

[subnets.prime.canisters.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.max_shards = 4

[subnets.prime.canisters.user_shard]
kind = "shard"
"#,
    )
    .expect("write config");
    write_artifact(&icp_root, "root", b"root-artifact");
    write_artifact(&icp_root, "user_hub", b"user-hub-artifact");
    write_artifact(&icp_root, "user_shard", b"user-shard-artifact");

    let plan = build_local_deployment_plan(&LocalDeploymentPlanRequest {
        deployment_name: "demo-local".to_string(),
        network: "local".to_string(),
        workspace_root,
        icp_root,
        config_path: None,
        runtime_variant: "local".to_string(),
        build_profile: "fast".to_string(),
    });

    assert_eq!(
        plan.expected_pool,
        vec![ExpectedPoolCanisterV1 {
            pool: "user_shards".to_string(),
            canister_id: None,
            role: Some("user_shard".to_string()),
        }]
    );
    let inventory = sample_matching_inventory();
    let diff = compare_plan_to_inventory(&plan, &inventory);
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "pool_canister_unobserved"
                && finding.subject.as_deref() == Some("user_shards:user_shard"))
    );
}

#[test]
fn deployment_diff_blocks_deployment_manifest_mismatch() {
    let mut plan = sample_plan();
    plan.expected_canisters.clear();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.expected_verifier_readiness.required = false;
    let mut observed_identity = sample_identity();
    observed_identity.deployment_manifest_digest = Some("different-manifest".to_string());
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(observed_identity),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: Some("different-manifest".to_string()),
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: Vec::new(),
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
            payload_size_bytes: Some(10),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    };

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "deployment_manifest_mismatch")
    );
}

#[test]
fn deployment_diff_blocks_raw_config_digest_mismatch_without_claiming_manifest_identity() {
    let mut plan = sample_plan();
    plan.deployment_identity.deployment_manifest_digest = None;
    plan.expected_canisters.clear();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.role_artifacts[0].raw_config_sha256 = Some("planned-raw-config".to_string());
    plan.expected_verifier_readiness.required = false;
    let mut observed_identity = sample_identity();
    observed_identity.deployment_manifest_digest = None;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(observed_identity),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: Some("observed-raw-config".to_string()),
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: Vec::new(),
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
            payload_size_bytes: Some(10),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    };

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "raw_config_digest_mismatch")
    );
    assert!(diff.embedded_config_diff.iter().any(|item| {
        item.category == "raw_config_sha256"
            && item.expected.as_deref() == Some("planned-raw-config")
            && item.observed.as_deref() == Some("observed-raw-config")
    }));
}

#[test]
fn deployment_diff_blocks_installed_module_hash_mismatch() {
    let mut plan = sample_plan();
    plan.expected_canisters.clear();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string()],
            module_hash: Some("different-module".to_string()),
            status: Some("Running".to_string()),
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: None,
            role_assignment_source: Some("icp_canister_status".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
            payload_size_bytes: Some(10),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    };

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "installed_module_hash_mismatch")
    );
    assert!(diff.module_hash_diff.iter().any(|item| {
        item.category == "installed_module_hash"
            && item.expected.as_deref() == Some("module")
            && item.observed.as_deref() == Some("different-module")
    }));
}

#[test]
fn deployment_diff_uses_concrete_expected_id_for_installed_module_hash() {
    let mut plan = sample_plan();
    plan.expected_verifier_readiness.required = false;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "duplicate-root-id".to_string(),
        role: Some("root".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("different-module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(
        diff.hard_failures
            .iter()
            .all(|finding| finding.code != "installed_module_hash_mismatch")
    );
    assert!(
        diff.module_hash_diff
            .iter()
            .all(|item| item.category != "installed_module_hash")
    );
}

#[test]
fn deployment_diff_blocks_ambiguous_installed_module_hash_target() {
    let mut plan = sample_plan();
    plan.expected_canisters.clear();
    plan.expected_verifier_readiness.required = false;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "duplicate-root-id".to_string(),
        role: Some("root".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "installed_module_hash_ambiguous"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.module_hash_diff.iter().any(|item| {
        item.category == "installed_module_hash_ambiguous"
            && item.subject == "root"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("aaaaa-aa") && observed.contains("duplicate-root-id")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_blocks_missing_expected_controller() {
    let mut plan = sample_plan();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.role_artifacts[0].installed_module_hash = None;
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["external-controller".to_string()],
            module_hash: None,
            status: Some("Running".to_string()),
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: None,
            role_assignment_source: Some("icp_canister_status".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
            payload_size_bytes: Some(10),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    };

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "expected_controller_missing")
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "controller_missing"
            && item.expected.as_deref() == Some("aaaaa-aa")
            && item.observed.as_deref() == Some("external-controller")
    }));
}

#[test]
fn deployment_diff_warns_for_extra_declared_emergency_controller() {
    let mut plan = sample_plan();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.role_artifacts[0].installed_module_hash = None;
    plan.authority_profile
        .emergency_controllers
        .push("emergency-controller".to_string());
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string(), "emergency-controller".to_string()],
            module_hash: None,
            status: Some("Running".to_string()),
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: None,
            role_assignment_source: Some("icp_canister_status".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
            payload_size_bytes: Some(10),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    };

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Safe);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .all(|finding| finding.code != "extra_controller_observed")
    );
}

#[test]
fn deployment_diff_blocks_authority_profile_controller_overlap() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_canisters.clear();
    plan.expected_verifier_readiness.required = false;
    plan.authority_profile
        .staging_controllers
        .push("aaaaa-aa".to_string());
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: Vec::new(),
        observed_pool: Vec::new(),
        observed_artifacts: Vec::new(),
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    };

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "controller_authority_overlap")
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "controller_authority_overlap"
            && item.expected.as_deref() == Some("expected-only")
            && item.observed.as_deref() == Some("aaaaa-aa")
    }));
}

#[test]
fn deployment_diff_warns_for_undeclared_extra_controller() {
    let mut plan = sample_plan();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.role_artifacts[0].installed_module_hash = None;
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string(), "surprise-controller".to_string()],
            module_hash: None,
            status: Some("Running".to_string()),
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: None,
            role_assignment_source: Some("icp_canister_status".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
            payload_size_bytes: Some(10),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    };

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "extra_controller_observed")
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "controller_extra"
            && item.expected.as_deref() == Some("aaaaa-aa")
            && item.observed.as_deref() == Some("surprise-controller")
    }));
}

#[test]
fn deployment_diff_blocks_artifact_file_digest_mismatch() {
    let mut plan = sample_plan();
    plan.expected_canisters.clear();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.role_artifacts[0].observed_wasm_gz_file_sha256 = Some("planned-file".to_string());
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: Vec::new(),
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("observed-file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
            payload_size_bytes: Some(10),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    };

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "artifact_file_digest_mismatch")
    );
    assert!(diff.artifact_diff.iter().any(|item| {
        item.category == "artifact_file_sha256"
            && item.expected.as_deref() == Some("planned-file")
            && item.observed.as_deref() == Some("observed-file")
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_artifact_observations_for_same_role() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_artifacts.push(ObservedArtifactV1 {
        role: "root".to_string(),
        artifact_path: "alternate-root.wasm.gz".to_string(),
        file_sha256: Some("different-file".to_string()),
        file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
        payload_sha256: Some("different-gzip".to_string()),
        payload_size_bytes: Some(99),
        source: ArtifactSourceV1::LocalBuild,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "artifact_role_conflict"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.artifact_diff.iter().any(|item| {
        item.category == "artifact_role_conflict"
            && item.subject == "root"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("root.wasm.gz") && observed.contains("alternate-root.wasm.gz")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_duplicate_identical_artifact_observation() {
    let mut inventory = sample_matching_inventory();
    inventory
        .observed_artifacts
        .push(inventory.observed_artifacts[0].clone());

    let diff = compare_plan_to_inventory(&sample_plan(), &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_artifact_observed"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.artifact_diff.iter().any(|item| {
        item.category == "artifact_duplicate"
            && item.subject == "root"
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_planned_artifacts_for_same_role() {
    let mut plan = sample_plan();
    let mut duplicate = sample_role_artifact();
    duplicate.wasm_gz_path = Some("alternate-root.wasm.gz".to_string());
    duplicate.wasm_gz_sha256 = Some("different-gzip".to_string());
    plan.role_artifacts.push(duplicate);

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "planned_artifact_role_conflict"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.artifact_diff.iter().any(|item| {
        item.category == "planned_artifact_role_conflict"
            && item.subject == "root"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("root.wasm.gz") && observed.contains("alternate-root.wasm.gz")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_duplicate_identical_planned_artifact_role() {
    let mut plan = sample_plan();
    plan.role_artifacts.push(sample_role_artifact());

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_planned_artifact_role"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.artifact_diff.iter().any(|item| {
        item.category == "planned_artifact_duplicate"
            && item.subject == "root"
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_missing_artifacts_and_unsafe_control_class() {
    let plan = sample_plan();
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::UnknownUnsafe,
            controllers: Vec::new(),
            module_hash: None,
            status: None,
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: None,
            role_assignment_source: Some("local_install_state".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: Vec::new(),
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::Observed,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    };

    let diff = compare_plan_to_inventory(&plan, &inventory);
    let report = safety_report_from_diff("report-1", Some("diff-1".to_string()), &diff);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|item| item.code == "artifact_missing")
    );
    assert!(
        diff.hard_failures
            .iter()
            .any(|item| item.code == "unsafe_control_class")
    );
    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert_eq!(
        report.next_actions,
        vec!["resolve blocking deployment truth differences before mutation".to_string()]
    );
}

#[test]
fn deployment_diff_warns_on_observation_gaps_without_blocking() {
    let mut plan = sample_plan();
    plan.expected_canisters.clear();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: Vec::new(),
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
            payload_size_bytes: Some(10),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: vec![DeploymentObservationGapV1 {
            key: "local_artifacts.user_hub".to_string(),
            description: "missing built artifact".to_string(),
        }],
    };

    let diff = compare_plan_to_inventory(&plan, &inventory);
    let report = safety_report_from_diff("report-1", None, &diff);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.artifact_diff
            .iter()
            .any(|item| item.category == "artifact_file_sha256"
                && item.severity == SafetySeverityV1::Info)
    );
    assert!(
        diff.warnings
            .iter()
            .any(|item| item.code == "observation_gap")
    );
    assert_eq!(report.status, SafetyStatusV1::Warning);
}

#[test]
fn deployment_diff_warns_on_plan_assumptions_without_blocking() {
    let mut plan = sample_plan();
    plan.expected_canisters.clear();
    plan.role_artifacts[0].wasm_gz_sha256 = None;
    plan.expected_verifier_readiness.required = false;
    plan.unresolved_assumptions.push(DeploymentAssumptionV1 {
        key: "local_state.root_canister_id".to_string(),
        description: "root identity is unknown until install".to_string(),
    });
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: Vec::new(),
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
            payload_size_bytes: Some(10),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    };

    let diff = compare_plan_to_inventory(&plan, &inventory);
    let report = safety_report_from_diff("report-1", None, &diff);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|item| item.code == "plan_assumption"
                && item.subject.as_deref() == Some("local_state.root_canister_id"))
    );
    assert_eq!(report.status, SafetyStatusV1::Warning);
}

#[test]
fn deployment_diff_warns_when_unspecified_canister_id_is_unobserved() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].canister_id = None;
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: Vec::new(),
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: Some("gzip".to_string()),
            payload_size_bytes: Some(10),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    };

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "canister_unobserved"
                && finding.subject.as_deref() == Some("root"))
    );
}

#[test]
fn deployment_diff_blocks_conflicting_planned_canisters_for_same_role() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_verifier_readiness.required = false;
    plan.expected_canisters.push(ExpectedCanisterV1 {
        role: "root".to_string(),
        canister_id: Some("duplicate-root-id".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
    });

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "planned_canister_role_conflict"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "planned_canister_role_conflict"
            && item.subject == "root"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("aaaaa-aa") && observed.contains("duplicate-root-id")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_planned_roles_for_same_canister_id() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_verifier_readiness.required = false;
    plan.expected_canisters.push(ExpectedCanisterV1 {
        role: "user_hub".to_string(),
        canister_id: Some("aaaaa-aa".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
    });

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "planned_canister_id_conflict"
                && finding.subject.as_deref() == Some("aaaaa-aa"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "planned_canister_id_conflict"
            && item.subject == "aaaaa-aa"
            && item
                .observed
                .as_deref()
                .is_some_and(|observed| observed.contains("root") && observed.contains("user_hub"))
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_duplicate_identical_planned_canister_role() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_verifier_readiness.required = false;
    plan.expected_canisters
        .push(plan.expected_canisters[0].clone());

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_planned_canister_role"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "planned_canister_duplicate"
            && item.subject == "root"
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_warns_for_extra_observed_canister_roles() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "user-hub-id".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "extra_canister_observed"
                && finding.subject.as_deref() == Some("user_hub"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "canister_extra"
            && item.subject == "user_hub"
            && item.observed.as_deref() == Some("user-hub-id")
    }));
}

#[test]
fn deployment_diff_warns_for_duplicate_observed_planned_role() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "duplicate-root-id".to_string(),
        role: Some("root".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "extra_canister_observed"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "canister_extra"
            && item.subject == "root"
            && item.observed.as_deref() == Some("duplicate-root-id")
    }));
}

#[test]
fn deployment_diff_blocks_ambiguous_expected_role_without_canister_id() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_verifier_readiness.required = false;
    plan.expected_canisters.push(ExpectedCanisterV1 {
        role: "user_hub".to_string(),
        canister_id: None,
        control_class: CanisterControlClassV1::DeploymentControlled,
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_artifacts.clear();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "user-hub-a".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "user-hub-b".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "canister_role_ambiguous"
                && finding.subject.as_deref() == Some("user_hub"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "canister_role_ambiguous"
            && item.subject == "user_hub"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("user-hub-a") && observed.contains("user-hub-b")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_blocks_expected_canister_role_mismatch() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_verifier_readiness.required = false;
    let mut inventory = sample_matching_inventory();
    inventory.observed_artifacts.clear();
    inventory.observed_canisters[0].role = Some("user_hub".to_string());

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "canister_role_mismatch"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "role_mismatch"
            && item.subject == "root"
            && item.expected.as_deref() == Some("root")
            && item.observed.as_deref() == Some("user_hub")
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_roles_for_same_canister_id() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_verifier_readiness.required = false;
    let mut inventory = sample_matching_inventory();
    inventory.observed_artifacts.clear();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "aaaaa-aa".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "canister_id_role_conflict"
                && finding.subject.as_deref() == Some("aaaaa-aa"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "canister_id_role_conflict"
            && item.subject == "aaaaa-aa"
            && item.observed.as_deref() == Some("root,user_hub")
    }));
}

#[test]
fn deployment_diff_warns_for_exact_duplicate_canister_observation() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_verifier_readiness.required = false;
    let mut inventory = sample_matching_inventory();
    inventory.observed_artifacts.clear();
    inventory
        .observed_canisters
        .push(inventory.observed_canisters[0].clone());

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_canister_observed"
                && finding.subject.as_deref() == Some("aaaaa-aa"))
    );
    assert!(diff.controller_diff.iter().any(|item| {
        item.category == "canister_duplicate"
            && item.subject == "aaaaa-aa"
            && item.expected.as_deref() == Some("root")
            && item.observed.as_deref() == Some("2")
    }));
}

#[test]
fn enriched_registry_status_participates_in_controller_checks() {
    let mut plan = sample_plan();
    plan.role_artifacts.clear();
    plan.expected_verifier_readiness.required = false;
    plan.expected_canisters.push(ExpectedCanisterV1 {
        role: "user_hub".to_string(),
        canister_id: None,
        control_class: CanisterControlClassV1::DeploymentControlled,
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_artifacts.clear();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "user-hub-id".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: Vec::new(),
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "expected_controller_missing"
                && finding.subject.as_deref() == Some("user_hub"))
    );
    assert!(
        diff.warnings
            .iter()
            .all(|finding| finding.code != "controllers_unobserved")
    );
}

#[test]
fn deployment_diff_blocks_missing_expected_pool_canister() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let inventory = sample_matching_inventory();

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "pool_canister_missing")
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_canister"
            && item.subject == "user_shards:user_shard"
            && item.expected.as_deref() == Some("pool-canister")
            && item.observed.is_none()
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_planned_pool_subject() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-a".to_string()),
        role: Some("user_shard".to_string()),
    });
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-b".to_string()),
        role: Some("user_shard".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "planned_pool_conflict"
                && finding.subject.as_deref() == Some("user_shards:user_shard"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "planned_pool_conflict"
            && item.subject == "user_shards:user_shard"
            && item
                .observed
                .as_deref()
                .is_some_and(|observed| observed.contains("pool-a") && observed.contains("pool-b"))
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_planned_pool_id() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "directory".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("project_instance".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "planned_pool_id_conflict"
                && finding.subject.as_deref() == Some("pool-canister"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "planned_pool_id_conflict"
            && item.subject == "pool-canister"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("directory:project_instance")
                    && observed.contains("user_shards:user_shard")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_duplicate_identical_planned_pool() {
    let mut plan = sample_plan();
    let planned = ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    };
    plan.expected_pool.push(planned.clone());
    plan.expected_pool.push(planned);
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_planned_pool"
                && finding.subject.as_deref() == Some("user_shards:user_shard"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "planned_pool_duplicate"
            && item.subject == "user_shards:user_shard"
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_unsafe_pool_control_class() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::UserControlled,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "unsafe_pool_control_class")
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_control_class"
            && item.subject == "user_shards:user_shard"
            && item.expected.as_deref() == Some("CanicManagedPool")
            && item.observed.as_deref() == Some("UserControlled")
    }));
}

#[test]
fn deployment_diff_blocks_pool_canister_id_mismatch() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("planned-pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "observed-pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "pool_canister_id_mismatch")
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_canister_id"
            && item.subject == "user_shards:user_shard"
            && item.expected.as_deref() == Some("planned-pool-canister")
            && item.observed.as_deref() == Some("observed-pool-canister")
    }));
    assert!(
        diff.warnings
            .iter()
            .all(|finding| finding.code != "extra_pool_canister_observed")
    );
}

#[test]
fn deployment_diff_blocks_conflicting_pool_identities_for_same_canister_id() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "directory".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("project_instance".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "pool_canister_id_conflict"
                && finding.subject.as_deref() == Some("pool-canister"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_canister_id_conflict"
            && item.subject == "pool-canister"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("directory:project_instance")
                    && observed.contains("user_shards:user_shard")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_exact_duplicate_pool_observation() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    let observed = ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    };
    inventory.observed_pool.push(observed.clone());
    inventory.observed_pool.push(observed);

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_pool_canister_observed"
                && finding.subject.as_deref() == Some("pool-canister"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_canister_duplicate"
            && item.subject == "pool-canister"
            && item.expected.as_deref() == Some("user_shards:user_shard")
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_cross_surface_role_conflict_for_same_canister_id() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("shared-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "shared-canister".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "shared-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "canister_pool_role_conflict"
                && finding.subject.as_deref() == Some("shared-canister"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "canister_pool_role_conflict"
            && item.subject == "shared-canister"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("canister=user_hub")
                    && observed.contains("pool=user_shards:user_shard")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_extra_pool_canister() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "directory".to_string(),
        canister_id: "extra-pool-canister".to_string(),
        role: Some("project_instance".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "extra_pool_canister_observed")
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_extra"
            && item.subject == "directory:project_instance"
            && item.observed.as_deref() == Some("extra-pool-canister")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_stale_verifier_role_epoch() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_verifier_readiness.role_epochs = vec![RoleEpochObservationV1 {
        role: "root".to_string(),
        observed_epoch: Some(0),
        status: ObservationStatusV1::Observed,
    }];

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "verifier_role_epoch_stale")
    );
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == "verifier_role_epoch"
            && item.subject == "root"
            && item.expected.as_deref() == Some("1")
            && item.observed.as_deref() == Some("0")
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_when_required_verifier_role_epoch_is_unobserved() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_verifier_readiness.role_epochs.clear();

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "verifier_role_epoch_unobserved")
    );
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == "verifier_role_epoch"
            && item.subject == "root"
            && item.expected.as_deref() == Some("1")
            && item.observed.as_deref() == Some("not_observed")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_verifier_role_epoch_observations() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_verifier_readiness.role_epochs = vec![
        RoleEpochObservationV1 {
            role: "root".to_string(),
            observed_epoch: Some(1),
            status: ObservationStatusV1::Observed,
        },
        RoleEpochObservationV1 {
            role: "root".to_string(),
            observed_epoch: Some(0),
            status: ObservationStatusV1::Observed,
        },
    ];

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "verifier_role_epoch_conflict"
                && finding.subject.as_deref() == Some("root"))
    );
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == "verifier_role_epoch_conflict"
            && item.subject == "root"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("epoch=1") && observed.contains("epoch=0")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_duplicate_identical_verifier_role_epoch_observation() {
    let mut inventory = sample_matching_inventory();
    inventory
        .observed_verifier_readiness
        .role_epochs
        .push(inventory.observed_verifier_readiness.role_epochs[0].clone());

    let diff = compare_plan_to_inventory(&sample_plan(), &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(diff.warnings.iter().any(|finding| finding.code
        == "duplicate_verifier_role_epoch_observed"
        && finding.subject.as_deref() == Some("root")));
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == "verifier_role_epoch_duplicate"
            && item.subject == "root"
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_planned_verifier_role_epoch() {
    let mut plan = sample_plan();
    plan.expected_verifier_readiness
        .expected_role_epochs
        .push(RoleEpochExpectationV1 {
            role: "root".to_string(),
            minimum_epoch: 2,
        });

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.hard_failures.iter().any(|finding| finding.code
        == "planned_verifier_role_epoch_conflict"
        && finding.subject.as_deref() == Some("root")));
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == "planned_verifier_role_epoch_conflict"
            && item.subject == "root"
            && item
                .observed
                .as_deref()
                .is_some_and(|observed| observed.contains('1') && observed.contains('2'))
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_duplicate_identical_planned_verifier_role_epoch() {
    let mut plan = sample_plan();
    plan.expected_verifier_readiness
        .expected_role_epochs
        .push(plan.expected_verifier_readiness.expected_role_epochs[0].clone());

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(diff.warnings.iter().any(|finding| finding.code
        == "duplicate_planned_verifier_role_epoch"
        && finding.subject.as_deref() == Some("root")));
    assert!(diff.verifier_readiness_diff.iter().any(|item| {
        item.category == "planned_verifier_role_epoch_duplicate"
            && item.subject == "root"
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_is_safe_when_checked_facts_match() {
    let mut plan = sample_plan();
    plan.expected_verifier_readiness.required = false;
    let inventory = DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-21T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("icp.yml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string()],
            module_hash: Some("module".to_string()),
            status: None,
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: None,
            role_assignment_source: Some("local_install_state".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: Some("gzip".to_string()),
            payload_size_bytes: Some(10),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    };

    let diff = compare_plan_to_inventory(&plan, &inventory);
    let report = safety_report_from_diff("report-1", None, &diff);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Safe);
    assert!(
        diff.artifact_diff
            .iter()
            .any(|item| item.category == "artifact_file_sha256"
                && item.severity == SafetySeverityV1::Info)
    );
    assert!(diff.hard_failures.is_empty());
    assert!(diff.warnings.is_empty());
    assert_eq!(report.status, SafetyStatusV1::Safe);
    assert!(report.next_actions.is_empty());
}

#[test]
fn authority_reconciliation_reports_already_correct_controller_state() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let plan = build_authority_reconciliation_plan(&check);

    assert_eq!(plan.plan_id, "plan-local-root");
    assert_eq!(plan.inventory_id, "inventory-1");
    assert_eq!(plan.authority_profile_hash.as_deref(), Some("authority"));
    assert!(plan.hard_failures.is_empty());
    assert!(plan.external_actions_required.is_empty());
    assert_eq!(plan.canister_actions.len(), 1);
    assert_eq!(
        plan.canister_actions[0].state,
        AuthorityReconciliationStateV1::AlreadyCorrect
    );
    assert_eq!(plan.canister_actions[0].action, AuthorityActionV1::None);
    assert!(!plan.canister_actions[0].can_apply);
}

#[test]
fn authority_report_summarizes_safe_reconciliation_plan() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let plan = build_authority_reconciliation_plan(&check);

    let report = authority_report_from_plan("authority-report-1", &plan);

    assert_eq!(report.status, SafetyStatusV1::Safe);
    assert_eq!(report.reconciliation_plan_id, "plan-local-root");
    assert_eq!(report.check_id, None);
    assert_eq!(report.inventory_id, "inventory-1");
    assert_eq!(report.authority_profile_hash.as_deref(), Some("authority"));
    assert_eq!(report.counts.already_correct, 1);
    assert_eq!(report.counts.can_apply_automatically, 0);
    assert_eq!(report.counts.requires_external_action, 0);
    assert_eq!(report.counts.unsafe_blocked, 0);
    assert_eq!(report.counts.unknown, 0);
    assert_eq!(report.counts.hard_failures, 0);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: Vec::new(),
        }
    );
    assert_eq!(
        report.action_counts,
        vec![AuthorityActionCountV1 {
            action: AuthorityActionV1::None,
            count: 1,
        }]
    );
    assert_eq!(
        report.control_class_counts,
        vec![AuthorityControlClassCountV1 {
            control_class: CanisterControlClassV1::DeploymentControlled,
            count: 1,
        }]
    );
    assert!(report.observation_gaps.is_empty());
    assert!(report.automatic_actions.is_empty());
    assert!(report.external_actions_required.is_empty());
    assert!(report.next_actions.is_empty());
}

#[test]
fn authority_report_can_preserve_source_check_id() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let plan = build_authority_reconciliation_plan(&check);

    let report =
        authority_report_from_plan_with_check_id("authority-report-1", Some(check.check_id), &plan);

    assert_eq!(report.check_id.as_deref(), Some("check-1"));
    assert_eq!(report.reconciliation_plan_id, "plan-local-root");
    assert_eq!(report.inventory_id, "inventory-1");
}

#[test]
fn authority_report_from_check_preserves_source_provenance() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let report = authority_report_from_check("authority-report-1", &check);

    assert_eq!(report.check_id.as_deref(), Some("check-1"));
    assert_eq!(report.reconciliation_plan_id, "plan-local-root");
    assert_eq!(report.inventory_id, "inventory-1");
    assert_eq!(report.authority_profile_hash.as_deref(), Some("authority"));
    assert_eq!(report.counts.already_correct, 1);
}

#[test]
fn authority_report_from_check_with_local_id_uses_deployment_identity() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let report = authority_report_from_check_with_local_id(&check);

    assert_eq!(report.report_id, "local:local:local-root:authority-report");
    assert_eq!(report.check_id.as_deref(), Some("check-1"));
    assert_eq!(report.reconciliation_plan_id, "plan-local-root");
    assert_eq!(report.inventory_id, "inventory-1");
}

#[test]
fn authority_dry_run_evidence_from_check_with_local_ids_uses_deployment_identity() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let evidence =
        authority_dry_run_evidence_from_check_with_local_ids(&check, "2026-05-23T00:00:01Z")
            .expect("build authority evidence");

    assert_eq!(
        evidence.evidence_id,
        "local:local:local-root:authority-evidence"
    );
    assert_eq!(evidence.check_id, "check-1");
    assert_eq!(
        evidence.authority_report.report_id,
        "local:local:local-root:authority-report"
    );
    assert_eq!(
        evidence.authority_receipt.operation_id,
        "local:local:local-root:authority-dry-run-receipt"
    );
    assert_eq!(
        evidence.authority_receipt.authority_report_id,
        evidence.authority_report.report_id
    );
    assert_eq!(evidence.generated_at, "2026-05-23T00:00:01Z");
    assert_eq!(
        evidence.authority_receipt.finished_at.as_deref(),
        Some("2026-05-23T00:00:01Z")
    );
}

#[test]
fn authority_dry_run_receipt_from_check_with_local_id_uses_deployment_identity() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let receipt =
        authority_dry_run_receipt_from_check_with_local_id(&check, "2026-05-23T00:00:01Z")
            .expect("build authority receipt");

    assert_eq!(
        receipt.operation_id,
        "local:local:local-root:authority-dry-run-receipt"
    );
    assert_eq!(receipt.check_id.as_deref(), Some("check-1"));
    assert_eq!(receipt.reconciliation_plan_id, "plan-local-root");
    assert_eq!(
        receipt.authority_report_id,
        "local:local:local-root:authority-report"
    );
    assert_eq!(receipt.inventory_id, "inventory-1");
    assert_eq!(receipt.authority_profile_hash.as_deref(), Some("authority"));
    assert_eq!(receipt.finished_at.as_deref(), Some("2026-05-23T00:00:01Z"));
    assert!(receipt.attempted_actions.is_empty());
}

#[test]
fn authority_dry_run_receipt_from_check_preserves_explicit_report_id() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let receipt = authority_dry_run_receipt_from_check(
        &check,
        "authority-report-explicit",
        "authority-dry-run-explicit",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build authority receipt");

    assert_eq!(receipt.operation_id, "authority-dry-run-explicit");
    assert_eq!(receipt.authority_report_id, "authority-report-explicit");
    assert_eq!(receipt.check_id.as_deref(), Some("check-1"));
    assert_eq!(receipt.reconciliation_plan_id, "plan-local-root");
}

#[test]
fn authority_text_renders_plan_and_report_summaries() {
    let mut source_plan = sample_plan();
    source_plan.authority_profile.expected_controllers =
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()];
    let check = sample_check(source_plan, sample_matching_inventory());
    let plan = build_authority_reconciliation_plan(&check);
    let report =
        authority_report_from_plan_with_check_id("authority-report-1", Some(check.check_id), &plan);

    let plan_text = authority_plan_text(&plan);
    let report_text = authority_report_text(&report);

    assert!(plan_text.contains("Authority reconciliation plan"));
    assert!(plan_text.contains("mode: dry_run"));
    assert!(plan_text.contains("plan_id: plan-local-root"));
    assert!(plan_text.contains("root (aaaaa-aa) CanApplyAutomatically/AddControllers"));
    assert!(plan_text.contains("[add=ops-principal; remove=none]"));
    assert!(report_text.contains("Authority reconciliation report"));
    assert!(report_text.contains("mode: dry_run"));
    assert!(report_text.contains("check_id: check-1"));
    assert!(report_text.contains("status: safe"));
    assert!(report_text.contains("[add=ops-principal; remove=none]"));
}

#[test]
fn authority_text_renders_evidence_and_receipt_details() {
    let mut source_plan = sample_plan();
    source_plan.authority_profile.staging_controllers = vec!["aaaaa-aa".to_string()];
    let check = sample_check(source_plan, sample_matching_inventory());
    let plan = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &plan,
    );
    let receipt = authority_dry_run_receipt_from_plan(
        &plan,
        &report,
        Some(check.check_id.clone()),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build receipt");
    let evidence = AuthorityDryRunEvidenceV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        evidence_id: "authority-evidence-1".to_string(),
        check_id: check.check_id,
        generated_at: "2026-05-23T00:00:00Z".to_string(),
        reconciliation_plan: plan,
        authority_report: report,
        authority_receipt: receipt,
    };

    let evidence_text = authority_evidence_text(&evidence);
    let receipt_text = authority_receipt_text(&evidence.authority_receipt);

    assert!(evidence_text.contains("Authority dry-run evidence"));
    assert!(evidence_text.contains("mode: dry_run"));
    assert!(evidence_text.contains("evidence_id: authority-evidence-1"));
    assert!(evidence_text.contains("generated_at: 2026-05-23T00:00:00Z"));
    assert!(evidence_text.contains("controller_mutation: none_attempted"));
    assert!(evidence_text.contains("verified_controller_observations:"));
    assert!(
        evidence_text
            .contains("aaaaa-aa AlreadyCorrect/None: observed=[aaaaa-aa] desired=[aaaaa-aa]")
    );
    assert!(evidence_text.contains(
        "[authority_profile_overlap] aaaaa-aa: staging authority principal aaaaa-aa overlaps"
    ));
    assert!(receipt_text.contains("Authority dry-run receipt"));
    assert!(receipt_text.contains("mode: dry_run"));
    assert!(receipt_text.contains("operation_id: authority-dry-run-1"));
    assert!(receipt_text.contains("controller_mutation: none_attempted"));
    assert!(receipt_text.contains("verified_controller_observations:"));
    assert!(
        receipt_text
            .contains("aaaaa-aa AlreadyCorrect/None: observed=[aaaaa-aa] desired=[aaaaa-aa]")
    );
    assert!(receipt_text.contains(
        "[authority_profile_overlap] aaaaa-aa: staging authority principal aaaaa-aa overlaps"
    ));
}

#[test]
fn authority_receipt_rejects_mismatched_report_provenance() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let mut report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    report.inventory_id = "other-inventory".to_string();

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("mismatched report inventory should fail receipt construction");

    assert!(matches!(
        err,
        AuthorityEvidenceError::PlanReportMismatch {
            field: "inventory_id",
            ..
        }
    ));
}

#[test]
fn authority_receipt_rejects_mismatched_report_content() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers =
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()];
    let check = sample_check(plan, sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let mut report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    report.automatic_actions.clear();

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("mismatched report content should fail receipt construction");

    assert!(matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "automatic_actions",
        }
    ));
}

#[test]
fn authority_receipt_rejects_mismatched_check_id() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id),
        &reconciliation,
    );

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some("other-check".to_string()),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("mismatched check id should fail receipt construction");

    assert!(matches!(
        err,
        AuthorityEvidenceError::CheckIdMismatch { .. }
    ));
}

#[test]
fn authority_receipt_rejects_unsupported_source_schema_version() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let mut reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    reconciliation.schema_version = DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1;

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("unsupported plan schema should fail receipt construction");

    assert!(matches!(
        err,
        AuthorityEvidenceError::SchemaVersionMismatch {
            component: "plan",
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found
        } if found == DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1
    ));
}

#[test]
fn authority_receipt_rejects_blank_receipt_identity_inputs() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        " ",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("blank receipt operation id should fail receipt construction");

    assert!(matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "receipt.operation_id",
        }
    ));
}

#[test]
fn authority_receipt_rejects_missing_report_check_provenance() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let mut report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    report.check_id = None;

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("receipt construction should require report check provenance");

    assert!(matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "report.check_id",
        }
    ));
}

#[test]
fn authority_receipt_rejects_missing_finished_at() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        None,
    )
    .expect_err("completed dry-run receipt should require finished_at");

    assert!(matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "receipt.finished_at",
        }
    ));
}

#[test]
fn authority_receipt_rejects_finished_before_started() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:02Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("receipt construction should reject invalid timestamp order");

    assert!(matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptTimestampOrder {
            field: "receipt.started_at",
            other_field: "receipt.finished_at",
            ..
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_mismatched_nested_check_id() {
    let mut evidence = sample_authority_evidence();
    evidence.check_id = "other-check".to_string();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mismatched nested check id should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::EvidenceCheckIdMismatch {
            component: "report",
            ..
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_unsupported_schema_version() {
    let mut evidence = sample_authority_evidence();
    evidence.schema_version = DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("unsupported evidence schema should fail validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::SchemaVersionMismatch {
            component: "evidence",
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found
        } if found == DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_nested_schema_version_drift() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_report.schema_version = DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("nested schema drift should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::SchemaVersionMismatch {
            component: "report",
            ..
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_blank_required_identity() {
    let mut evidence = sample_authority_evidence();
    evidence.evidence_id = "  ".to_string();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("blank evidence identity should fail validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "evidence.evidence_id"
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_missing_nested_check_provenance() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_report.check_id = None;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("full evidence should carry nested report check provenance");

    assert!(matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "report.check_id"
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_mismatched_receipt_content() {
    let mut evidence = sample_authority_evidence();
    evidence
        .authority_receipt
        .hard_failures
        .push(SafetyFindingV1 {
            code: "extra".to_string(),
            message: "extra hard finding".to_string(),
            severity: SafetySeverityV1::HardFailure,
            subject: None,
        });

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mismatched receipt content should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "receipt.hard_failures",
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_mutated_report_counts() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_report.counts.already_correct = 0;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mutated report counts should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "report.counts",
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_mutated_report_readiness() {
    let mut evidence = sample_authority_evidence();
    evidence
        .authority_report
        .apply_readiness
        .can_apply_automatically = true;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mutated report readiness should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "report.apply_readiness",
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_mutated_unsafe_blocker_readiness() {
    let mut evidence = sample_authority_evidence_from_check(sample_unknown_unsafe_check());
    assert_eq!(
        evidence.authority_report.apply_readiness.blockers,
        vec![AuthorityApplyBlockerV1::UnsafeBlocked]
    );

    evidence.authority_report.apply_readiness.blockers =
        vec![AuthorityApplyBlockerV1::HardFailures];

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mutated unsafe blocker readiness should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "report.apply_readiness",
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_attempted_actions() {
    let mut evidence = sample_authority_evidence();
    evidence
        .authority_receipt
        .attempted_actions
        .push(AuthorityAttemptedActionV1 {
            subject: "aaaaa-aa".to_string(),
            canister_id: Some("aaaaa-aa".to_string()),
            role: Some("root".to_string()),
            action: AuthorityActionV1::AddControllers,
            result: RolePhaseResultV1::NotAttempted,
            error: None,
        });

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("attempted dry-run actions should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptAttemptedActions { count: 1 }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_non_complete_receipt_status() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_receipt.operation_status = DeploymentExecutionStatusV1::FailedBeforeMutation;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("non-complete dry-run receipts should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptStatus {
            status: DeploymentExecutionStatusV1::FailedBeforeMutation
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_failed_receipt_command_result() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_receipt.command_result = DeploymentCommandResultV1::Failed {
        code: "dry_run_failed".to_string(),
        message: "dry run failed".to_string(),
    };

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("failed dry-run command results should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptCommandResult {
            result: DeploymentCommandResultV1::Failed { .. }
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_complete_receipt_without_finished_at() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_receipt.finished_at = None;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("complete dry-run receipts should record finished_at");

    assert!(matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptMissingFinishedAt
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_generated_at_mismatch() {
    let mut evidence = sample_authority_evidence();
    evidence.generated_at = "2026-05-23T00:00:02Z".to_string();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("evidence generated_at should match receipt completion time");

    assert!(matches!(
        err,
        AuthorityEvidenceError::EvidenceGeneratedAtMismatch {
            evidence_value,
            receipt_value,
        } if evidence_value == "2026-05-23T00:00:02Z"
            && receipt_value == "2026-05-23T00:00:01Z"
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_receipt_finished_before_started() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_receipt.started_at = "2026-05-23T00:00:02Z".to_string();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("dry-run receipt finish time should not precede start time");

    assert!(matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptTimestampOrder {
            field: "receipt.started_at",
            other_field: "receipt.finished_at",
            ..
        }
    ));
}

#[test]
fn authority_dry_run_evidence_rejects_mismatched_controller_observations() {
    let mut evidence = sample_authority_evidence();
    evidence
        .authority_receipt
        .verified_controller_observations
        .clear();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mismatched controller observations should fail evidence validation");

    assert!(matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "receipt.verified_controller_observations",
        }
    ));
}

#[test]
fn authority_reconciliation_marks_deployment_controlled_delta_as_automatic_dry_run() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers =
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()];
    let check = sample_check(plan, sample_matching_inventory());

    let reconciliation = build_authority_reconciliation_plan(&check);

    assert!(reconciliation.hard_failures.is_empty());
    assert!(reconciliation.external_actions_required.is_empty());
    assert_eq!(
        reconciliation.canister_actions[0].state,
        AuthorityReconciliationStateV1::CanApplyAutomatically
    );
    assert_eq!(
        reconciliation.canister_actions[0].action,
        AuthorityActionV1::AddControllers
    );
    assert!(reconciliation.canister_actions[0].can_apply);
    assert!(
        reconciliation.canister_actions[0]
            .reason
            .contains("ops-principal")
    );
    assert_eq!(reconciliation.automatic_actions.len(), 1);
    assert_eq!(reconciliation.automatic_actions[0].subject, "aaaaa-aa");
    assert_eq!(reconciliation.automatic_actions[0].canister_id, "aaaaa-aa");
    assert_eq!(
        reconciliation.automatic_actions[0].action,
        AuthorityActionV1::AddControllers
    );
    assert_eq!(
        reconciliation.automatic_actions[0].observed_controllers,
        vec!["aaaaa-aa".to_string()]
    );
    assert_eq!(
        reconciliation.automatic_actions[0].desired_controllers,
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()]
    );
    assert_eq!(
        reconciliation.automatic_actions[0].controller_delta,
        AuthorityControllerDeltaV1 {
            add_controllers: vec!["ops-principal".to_string()],
            remove_controllers: Vec::new(),
        }
    );

    let report = authority_report_from_plan("authority-report-1", &reconciliation);
    assert_eq!(report.status, SafetyStatusV1::Safe);
    assert_eq!(report.counts.can_apply_automatically, 1);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: true,
            automatic_action_count: 1,
            blockers: Vec::new(),
        }
    );
    assert_eq!(
        report.action_counts,
        vec![AuthorityActionCountV1 {
            action: AuthorityActionV1::AddControllers,
            count: 1,
        }]
    );
    assert!(report.observation_gaps.is_empty());
    assert_eq!(report.automatic_actions, reconciliation.automatic_actions);
    assert_eq!(
        report.next_actions,
        vec![
            "review automatic authority dry-run actions before enabling an apply path".to_string()
        ]
    );
}

#[test]
fn authority_apply_readiness_blocks_automatic_candidates_when_external_actions_remain() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers =
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()];
    plan.expected_canisters.push(ExpectedCanisterV1 {
        role: "user_hub".to_string(),
        canister_id: Some("user-hub-canister".to_string()),
        control_class: CanisterControlClassV1::UserControlled,
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "user-hub-canister".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::UserControlled,
        controllers: vec!["user-controller".to_string()],
        module_hash: None,
        status: Some("running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    });
    let check = sample_check(plan, inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan("authority-report-1", &reconciliation);

    assert_eq!(report.counts.can_apply_automatically, 1);
    assert_eq!(report.counts.requires_external_action, 1);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 1,
            blockers: vec![AuthorityApplyBlockerV1::ExternalActions],
        }
    );
    assert_eq!(
        report.next_actions,
        vec![
            "review external authority actions before applying controller changes",
            "review automatic authority dry-run actions before enabling an apply path",
        ]
    );
}

#[test]
fn authority_reconciliation_blocks_staging_or_emergency_controller_overlap() {
    let mut plan = sample_plan();
    plan.authority_profile.staging_controllers = vec!["aaaaa-aa".to_string()];
    plan.authority_profile.emergency_controllers = vec!["aaaaa-aa".to_string()];
    let check = sample_check(plan, sample_matching_inventory());

    let reconciliation = build_authority_reconciliation_plan(&check);

    assert_eq!(reconciliation.hard_failures.len(), 2);
    assert!(
        reconciliation
            .hard_failures
            .iter()
            .all(|finding| finding.code == "authority_profile_overlap"
                && finding.severity == SafetySeverityV1::HardFailure
                && finding.subject.as_deref() == Some("aaaaa-aa"))
    );
    assert_eq!(
        reconciliation.canister_actions[0].state,
        AuthorityReconciliationStateV1::AlreadyCorrect
    );

    let report = authority_report_from_plan("authority-report-1", &reconciliation);
    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert_eq!(report.counts.already_correct, 1);
    assert_eq!(report.counts.unsafe_blocked, 0);
    assert_eq!(report.counts.hard_failures, 2);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: vec![AuthorityApplyBlockerV1::HardFailures],
        }
    );
    assert_eq!(report.hard_failures, reconciliation.hard_failures);
    assert_eq!(
        report.next_actions,
        vec!["resolve hard authority findings before applying controller changes"]
    );
}

#[test]
fn authority_reconciliation_requires_external_action_for_user_controlled_drift() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers = vec!["aaaaa-aa".to_string()];
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-controller".to_string()];
    let check = sample_check(plan, inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);

    assert!(reconciliation.hard_failures.is_empty());
    assert_eq!(reconciliation.external_actions_required.len(), 1);
    let external = &reconciliation.external_actions_required[0];
    assert_eq!(external.subject, "aaaaa-aa");
    assert_eq!(external.canister_id.as_deref(), Some("aaaaa-aa"));
    assert_eq!(external.role.as_deref(), Some("root"));
    assert_eq!(
        external.control_classification,
        CanisterControlClassV1::UserControlled
    );
    assert_eq!(
        external.state,
        AuthorityReconciliationStateV1::RequiresExternalAction
    );
    assert_eq!(
        external.action,
        AuthorityActionV1::RequiresExternalController
    );
    assert_eq!(
        external.observed_controllers,
        vec!["user-controller".to_string()]
    );
    assert_eq!(external.desired_controllers, vec!["aaaaa-aa".to_string()]);
    assert_eq!(
        external.controller_delta,
        AuthorityControllerDeltaV1 {
            add_controllers: vec!["aaaaa-aa".to_string()],
            remove_controllers: vec!["user-controller".to_string()],
        }
    );
    assert_eq!(
        reconciliation.canister_actions[0].state,
        AuthorityReconciliationStateV1::RequiresExternalAction
    );
    assert_eq!(
        reconciliation.canister_actions[0].action,
        AuthorityActionV1::RequiresExternalController
    );
    assert!(!reconciliation.canister_actions[0].can_apply);

    let report = authority_report_from_plan("authority-report-1", &reconciliation);
    assert_eq!(report.status, SafetyStatusV1::Warning);
    assert_eq!(report.counts.requires_external_action, 1);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: vec![AuthorityApplyBlockerV1::ExternalActions],
        }
    );
    assert_eq!(report.external_actions_required.len(), 1);
    assert_eq!(report.external_actions_required[0], *external);
    assert_eq!(
        report.next_actions,
        vec!["review external authority actions before applying controller changes"]
    );
}

#[test]
fn authority_dry_run_receipt_records_observations_without_attempts() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers = vec!["aaaaa-aa".to_string()];
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-controller".to_string()];
    let check = sample_check(plan, inventory);
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let receipt = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id.clone()),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build authority receipt");

    assert_eq!(receipt.operation_id, "authority-dry-run-1");
    assert_eq!(receipt.check_id.as_deref(), Some("check-1"));
    assert_eq!(receipt.reconciliation_plan_id, "plan-local-root");
    assert_eq!(receipt.authority_report_id, "authority-report-1");
    assert_eq!(receipt.inventory_id, "inventory-1");
    assert_eq!(receipt.authority_profile_hash.as_deref(), Some("authority"));
    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(receipt.command_result, DeploymentCommandResultV1::Succeeded);
    assert!(receipt.attempted_actions.is_empty());
    assert_eq!(receipt.verified_controller_observations.len(), 1);
    assert_eq!(
        receipt.verified_controller_observations[0],
        AuthorityControllerObservationV1 {
            subject: "aaaaa-aa".to_string(),
            canister_id: Some("aaaaa-aa".to_string()),
            role: Some("root".to_string()),
            state: AuthorityReconciliationStateV1::RequiresExternalAction,
            action: AuthorityActionV1::RequiresExternalController,
            observed_controllers: vec!["user-controller".to_string()],
            desired_controllers: vec!["aaaaa-aa".to_string()],
            controller_delta: AuthorityControllerDeltaV1 {
                add_controllers: vec!["aaaaa-aa".to_string()],
                remove_controllers: vec!["user-controller".to_string()],
            },
        }
    );
    assert_eq!(
        receipt.unresolved_external_actions,
        report.external_actions_required
    );
    assert_eq!(receipt.hard_failures, report.hard_failures);
    assert_eq!(receipt.unresolved_observation_gaps, report.observation_gaps);

    let evidence = AuthorityDryRunEvidenceV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        evidence_id: "authority-evidence-1".to_string(),
        check_id: check.check_id,
        generated_at: "2026-05-23T00:00:01Z".to_string(),
        reconciliation_plan: reconciliation,
        authority_report: report,
        authority_receipt: receipt,
    };

    assert_json_round_trip(&evidence);
}

#[test]
fn authority_v1_json_schema_shape_is_stable() {
    let evidence = sample_authority_evidence();
    let value = serde_json::to_value(&evidence).expect("encode authority evidence");

    assert_object_keys(
        &value,
        &[
            "schema_version",
            "evidence_id",
            "check_id",
            "generated_at",
            "reconciliation_plan",
            "authority_report",
            "authority_receipt",
        ],
    );

    assert_object_keys(
        &value["reconciliation_plan"],
        &[
            "schema_version",
            "plan_id",
            "inventory_id",
            "authority_profile_hash",
            "canister_actions",
            "automatic_actions",
            "hard_failures",
            "external_actions_required",
        ],
    );
    assert_object_keys(
        &value["authority_report"],
        &[
            "schema_version",
            "report_id",
            "check_id",
            "reconciliation_plan_id",
            "inventory_id",
            "authority_profile_hash",
            "status",
            "summary",
            "counts",
            "apply_readiness",
            "action_counts",
            "control_class_counts",
            "observation_gaps",
            "automatic_actions",
            "hard_failures",
            "external_actions_required",
            "next_actions",
        ],
    );
    assert_object_keys(
        &value["authority_receipt"],
        &[
            "schema_version",
            "operation_id",
            "check_id",
            "reconciliation_plan_id",
            "authority_report_id",
            "inventory_id",
            "authority_profile_hash",
            "operation_status",
            "started_at",
            "finished_at",
            "attempted_actions",
            "verified_controller_observations",
            "hard_failures",
            "unresolved_observation_gaps",
            "unresolved_external_actions",
            "command_result",
        ],
    );

    assert_eq!(value["authority_report"]["status"], "Safe");
    assert_eq!(
        value["reconciliation_plan"]["canister_actions"][0]["state"],
        "AlreadyCorrect"
    );
    assert_eq!(
        value["reconciliation_plan"]["canister_actions"][0]["action"],
        "None"
    );
    assert_eq!(
        value["reconciliation_plan"]["canister_actions"][0]["control_classification"],
        "DeploymentControlled"
    );
    assert_eq!(value["authority_receipt"]["operation_status"], "Complete");
    assert_eq!(value["authority_receipt"]["command_result"], "Succeeded");
}

#[test]
fn deployment_truth_authority_paths_have_no_controller_mutation_primitives() {
    for (path, source) in [
        ("authority.rs", include_str!("authority.rs")),
        ("lifecycle.rs", include_str!("lifecycle.rs")),
        ("receipt.rs", include_str!("receipt.rs")),
        ("text.rs", include_str!("text.rs")),
    ] {
        for forbidden in [
            "update_settings",
            "install_code",
            "create_canister",
            "delete_canister",
            "stop_canister",
            "uninstall_code",
            "provisional_create_canister",
            "dfx",
        ] {
            assert!(
                !source.contains(forbidden),
                "deployment truth authority path {path} must stay dry-run; found forbidden token {forbidden}"
            );
        }
    }
}

#[test]
fn authority_dry_run_receipt_preserves_hard_findings() {
    let mut plan = sample_plan();
    plan.authority_profile.staging_controllers = vec!["aaaaa-aa".to_string()];
    let check = sample_check(plan, sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let receipt = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build authority receipt");

    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert_eq!(report.hard_failures.len(), 1);
    assert_eq!(receipt.hard_failures, report.hard_failures);
    assert!(receipt.unresolved_observation_gaps.is_empty());
    assert!(receipt.attempted_actions.is_empty());
    assert_eq!(receipt.verified_controller_observations.len(), 1);
}

#[test]
fn authority_reconciliation_blocks_unknown_unsafe_canister() {
    let check = sample_unknown_unsafe_check();

    let reconciliation = build_authority_reconciliation_plan(&check);

    assert_eq!(reconciliation.hard_failures.len(), 1);
    assert_eq!(
        reconciliation.hard_failures[0].code,
        "authority_unsafe_blocked"
    );
    assert!(reconciliation.canister_actions.iter().any(|action| {
        action.canister_id.as_deref() == Some("unsafe-canister")
            && action.state == AuthorityReconciliationStateV1::UnsafeBlocked
            && action.action == AuthorityActionV1::BlockedByPolicy
    }));

    let report = authority_report_from_plan("authority-report-1", &reconciliation);
    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert_eq!(report.counts.unsafe_blocked, 1);
    assert_eq!(report.counts.hard_failures, 0);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: vec![AuthorityApplyBlockerV1::UnsafeBlocked],
        }
    );
    assert!(report.external_actions_required.is_empty());
    assert_eq!(
        report.control_class_counts,
        vec![
            AuthorityControlClassCountV1 {
                control_class: CanisterControlClassV1::DeploymentControlled,
                count: 1,
            },
            AuthorityControlClassCountV1 {
                control_class: CanisterControlClassV1::UnknownUnsafe,
                count: 1,
            },
        ]
    );
    assert_eq!(
        report.next_actions,
        vec!["resolve unsafe canister authority findings before applying controller changes"]
    );
    let report_text = authority_report_text(&report);
    assert!(report_text.contains("    - unsafe_blocked"));
    assert!(!report_text.contains("    - hard_failures"));
}

#[test]
fn unsafe_authority_receipt_preserves_finding_without_hard_readiness_double_count() {
    let check = sample_unknown_unsafe_check();
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let receipt = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build authority receipt");

    assert_eq!(report.counts.unsafe_blocked, 1);
    assert_eq!(report.counts.hard_failures, 0);
    assert_eq!(
        report.apply_readiness.blockers,
        vec![AuthorityApplyBlockerV1::UnsafeBlocked]
    );
    assert_eq!(receipt.hard_failures, report.hard_failures);
    assert_eq!(receipt.hard_failures.len(), 1);
    assert_eq!(receipt.hard_failures[0].code, "authority_unsafe_blocked");
}

#[test]
fn authority_report_distinguishes_unsafe_and_hard_authority_blockers() {
    let mut plan = sample_plan();
    plan.authority_profile.staging_controllers = vec!["aaaaa-aa".to_string()];
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "unsafe-canister".to_string(),
        role: Some("surprise".to_string()),
        control_class: CanisterControlClassV1::UnknownUnsafe,
        controllers: vec!["unknown-controller".to_string()],
        module_hash: None,
        status: None,
        root_trust_anchor: None,
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    });
    let check = sample_check(plan, inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan("authority-report-1", &reconciliation);

    assert_eq!(reconciliation.hard_failures.len(), 2);
    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert_eq!(report.counts.unsafe_blocked, 1);
    assert_eq!(report.counts.hard_failures, 1);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: vec![
                AuthorityApplyBlockerV1::UnsafeBlocked,
                AuthorityApplyBlockerV1::HardFailures,
            ],
        }
    );
    assert_eq!(
        report.next_actions,
        vec![
            "resolve unsafe canister authority findings before applying controller changes",
            "resolve hard authority findings before applying controller changes",
        ]
    );
}

#[test]
fn blocked_authority_report_keeps_external_and_gap_next_actions() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers =
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()];
    plan.authority_profile.staging_controllers = vec!["aaaaa-aa".to_string()];
    plan.expected_canisters.push(ExpectedCanisterV1 {
        role: "user_hub".to_string(),
        canister_id: Some("user-hub-canister".to_string()),
        control_class: CanisterControlClassV1::UserControlled,
    });
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user-shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "user-hub-canister".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::UserControlled,
        controllers: vec!["user-controller".to_string()],
        module_hash: None,
        status: Some("running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    });
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user-shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });
    let check = sample_check(plan, inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan("authority-report-1", &reconciliation);

    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert_eq!(
        report.summary,
        "authority reconciliation is blocked by 0 unsafe canister(s) and 1 hard authority finding(s); also requires 1 external action(s) and has 1 unknown observation(s)"
    );
    assert_eq!(report.counts.hard_failures, 1);
    assert_eq!(report.counts.requires_external_action, 1);
    assert_eq!(report.counts.unknown, 1);
    assert_eq!(
        report.next_actions,
        vec![
            "resolve hard authority findings before applying controller changes",
            "review external authority actions before applying controller changes",
            "collect missing controller observations before applying controller changes",
            "review automatic authority dry-run actions before enabling an apply path",
        ]
    );
}

#[test]
fn authority_reconciliation_reports_expected_pool_controller_observation_gap() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user-shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user-shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });
    let check = sample_check(plan, inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);

    let pool_action = reconciliation
        .canister_actions
        .iter()
        .find(|action| action.canister_id.as_deref() == Some("pool-canister"))
        .expect("pool action should be reported");
    assert_eq!(pool_action.state, AuthorityReconciliationStateV1::Unknown);
    assert_eq!(pool_action.action, AuthorityActionV1::UnknownObservation);
    assert_eq!(
        pool_action.reason,
        "pool canister controller set was not observed"
    );
    assert!(reconciliation.external_actions_required.is_empty());
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    assert_eq!(report.counts.unknown, 1);
    assert!(report.external_actions_required.is_empty());
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: vec![AuthorityApplyBlockerV1::ObservationGaps],
        }
    );
    assert_eq!(report.observation_gaps.len(), 1);
    assert_eq!(
        report.observation_gaps[0],
        DeploymentObservationGapV1 {
            key: "authority.controllers.pool-canister".to_string(),
            description: "pool canister controller set was not observed".to_string(),
        }
    );
    let receipt = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build authority receipt");
    assert_eq!(receipt.unresolved_observation_gaps, report.observation_gaps);
    assert!(receipt.unresolved_external_actions.is_empty());
    assert_eq!(
        report.action_counts,
        vec![
            AuthorityActionCountV1 {
                action: AuthorityActionV1::None,
                count: 1,
            },
            AuthorityActionCountV1 {
                action: AuthorityActionV1::UnknownObservation,
                count: 1,
            },
        ]
    );
    assert_eq!(
        report.control_class_counts,
        vec![
            AuthorityControlClassCountV1 {
                control_class: CanisterControlClassV1::DeploymentControlled,
                count: 1,
            },
            AuthorityControlClassCountV1 {
                control_class: CanisterControlClassV1::CanicManagedPool,
                count: 1,
            },
        ]
    );
    assert_eq!(
        report.next_actions,
        vec!["collect missing controller observations before applying controller changes"]
    );
}

#[test]
fn authority_reconciliation_reports_unplanned_pool_canister_for_external_action() {
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user-shards".to_string(),
        canister_id: "unplanned-pool".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });
    let check = sample_check(sample_plan(), inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);

    let pool_action = reconciliation
        .canister_actions
        .iter()
        .find(|action| action.canister_id.as_deref() == Some("unplanned-pool"))
        .expect("unplanned pool action should be reported");
    assert_eq!(
        pool_action.state,
        AuthorityReconciliationStateV1::RequiresExternalAction
    );
    assert_eq!(pool_action.action, AuthorityActionV1::AdoptPlanAvailable);
    assert!(
        reconciliation
            .external_actions_required
            .iter()
            .any(|external| {
                external.subject == "unplanned-pool"
                    && external.action == AuthorityActionV1::AdoptPlanAvailable
                    && external.reason
                        == "observed pool canister is not present in the expected pool plan"
            })
    );
}

const SAMPLE_CONFIG: &str = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "singleton"
"#;

fn assert_json_round_trip<T>(value: &T)
where
    T: Clone + std::fmt::Debug + Eq + serde::de::DeserializeOwned + Serialize,
{
    let encoded = serde_json::to_string(value).expect("value should encode");
    let decoded = serde_json::from_str::<T>(&encoded).expect("value should decode");
    assert_eq!(decoded, *value);
}

fn assert_object_keys(value: &serde_json::Value, expected: &[&str]) {
    let object = value.as_object().expect("value should be a JSON object");
    let mut actual = object.keys().map(String::as_str).collect::<Vec<_>>();
    actual.sort_unstable();
    let mut expected = expected.to_vec();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}

fn sample_identity() -> DeploymentIdentityV1 {
    DeploymentIdentityV1 {
        deployment_name: "local-root".to_string(),
        network: "local".to_string(),
        root_principal: Some("aaaaa-aa".to_string()),
        authority_profile_hash: Some("authority".to_string()),
        role_topology_hash: Some("topology".to_string()),
        deployment_manifest_digest: Some("manifest".to_string()),
        canonical_runtime_config_digest: Some("runtime".to_string()),
        role_embedded_config_set_digest: Some("embedded".to_string()),
        artifact_set_digest: Some("artifacts".to_string()),
        pool_identity_set_digest: None,
        canic_version: Some("0.41.0".to_string()),
        ic_memory_version: Some("0.6.1".to_string()),
    }
}

fn sample_role_artifact() -> RoleArtifactV1 {
    RoleArtifactV1 {
        role: "root".to_string(),
        source: ArtifactSourceV1::LocalBuild,
        build_profile: "fast".to_string(),
        wasm_path: Some("root.wasm".to_string()),
        wasm_gz_path: Some("root.wasm.gz".to_string()),
        wasm_gz_size_bytes: Some(42),
        wasm_sha256: Some("wasm".to_string()),
        wasm_gz_sha256: Some("gzip".to_string()),
        wasm_gz_sha256_source: Some(ArtifactDigestSourceV1::ReleaseSetManifest),
        observed_wasm_gz_file_sha256: Some("file".to_string()),
        observed_wasm_gz_file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
        installed_module_hash: Some("module".to_string()),
        candid_path: Some("root.did".to_string()),
        candid_sha256: Some("did".to_string()),
        raw_config_sha256: Some("raw".to_string()),
        canonical_embedded_config_sha256: Some("canonical".to_string()),
        embedded_topology_sha256: Some("topology".to_string()),
        builder_version: Some("0.41.0".to_string()),
        rust_toolchain: Some("stable".to_string()),
        package_version: Some("0.41.0".to_string()),
    }
}

fn sample_role_artifact_source(kind: RoleArtifactSourceKindV1) -> RoleArtifactSourceV1 {
    RoleArtifactSourceV1 {
        role: "root".to_string(),
        kind,
        locator: Some("artifacts/root.wasm.gz".to_string()),
        previous_receipt_kind: (kind == RoleArtifactSourceKindV1::PreviousReceiptArtifact)
            .then_some(PreviousArtifactReceiptKindV1::DeploymentReceipt),
        previous_receipt_lineage_digest: (kind
            == RoleArtifactSourceKindV1::PreviousReceiptArtifact)
            .then(|| sample_sha256("9")),
        expected_wasm_sha256: Some(sample_sha256("d")),
        expected_wasm_gz_sha256: Some(sample_sha256("a")),
        expected_candid_sha256: Some(sample_sha256("b")),
        expected_canonical_embedded_config_sha256: Some(sample_sha256("c")),
    }
}

fn sample_role_promotion_input(promotion_level: PromotionArtifactLevelV1) -> RolePromotionInputV1 {
    RolePromotionInputV1 {
        role: "root".to_string(),
        promotion_level,
        source: sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasmGz),
        require_byte_identical_wasm: promotion_level == PromotionArtifactLevelV1::SealedWasm,
        require_target_embedded_config: true,
        target_store_has_artifact: Some(true),
    }
}

fn sample_role_promotion_policy() -> RolePromotionPolicyV1 {
    RolePromotionPolicyV1 {
        role: "root".to_string(),
        allowed_promotion_levels: vec![PromotionArtifactLevelV1::SealedWasm],
        requirements: vec![
            PromotionPolicyRequirementV1::SameSourceRevision,
            PromotionPolicyRequirementV1::SameCargoFeatures,
            PromotionPolicyRequirementV1::TargetConfigDigest,
            PromotionPolicyRequirementV1::ByteIdenticalWasm,
            PromotionPolicyRequirementV1::SealedBytes,
        ],
    }
}

fn sample_build_recipe_identity() -> BuildRecipeIdentityV1 {
    BuildRecipeIdentityV1 {
        recipe_id: "recipe:root:debug".to_string(),
        source_kind: RoleArtifactSourceKindV1::WorkspacePackage,
        source_revision: "0123456789abcdef0123456789abcdef01234567".to_string(),
        source_tree_clean: true,
        package_or_role_selector: "root".to_string(),
        cargo_profile: "debug".to_string(),
        cargo_features_digest: sample_sha256("1"),
        cargo_lock_digest: sample_sha256("2"),
        rust_toolchain: "1.88.0".to_string(),
        builder_version: "canic-build-v1".to_string(),
        target_triple: "wasm32-unknown-unknown".to_string(),
        linker_identity: "rust-lld".to_string(),
        deterministic_build_mode: "locked".to_string(),
        wasm_opt_version: "not-used".to_string(),
        compression_identity: "gzip:default".to_string(),
    }
}

fn sample_build_materialization_input() -> BuildMaterializationInputV1 {
    BuildMaterializationInputV1 {
        materialization_input_id: "materialization-input:root:prod".to_string(),
        build_recipe_id: "recipe:root:debug".to_string(),
        canonical_embedded_config_sha256: sample_sha256("3"),
        network: "ic".to_string(),
        root_trust_anchor: "aaaaa-aa".to_string(),
        runtime_variant: "prod".to_string(),
    }
}

fn sample_build_materialization_result() -> BuildMaterializationResultV1 {
    BuildMaterializationResultV1 {
        materialization_result_id: "materialization-result:root:prod".to_string(),
        build_recipe_id: "recipe:root:debug".to_string(),
        materialization_input_digest: sample_sha256("4"),
        wasm_sha256: sample_sha256("5"),
        wasm_gz_sha256: sample_sha256("6"),
        installed_module_hash: sample_sha256("7"),
        candid_sha256: sample_sha256("8"),
    }
}

fn sample_build_materialization_evidence() -> BuildMaterializationEvidenceV1 {
    let input = sample_build_materialization_input();
    let mut result = sample_build_materialization_result();
    result.materialization_input_digest = build_materialization_input_digest(&input);
    build_materialization_evidence(BuildMaterializationEvidenceRequest {
        evidence_id: "materialization-evidence-1".to_string(),
        recipe: sample_build_recipe_identity(),
        materialization_input: input,
        materialization_result: result,
    })
    .expect("sample materialization evidence should validate")
}

fn sample_promotion_target_plan() -> DeploymentPlanV1 {
    let mut plan = sample_plan();
    plan.role_artifacts[0].wasm_sha256 = Some(sample_sha256("d"));
    plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("a"));
    plan.role_artifacts[0].canonical_embedded_config_sha256 = Some(sample_sha256("c"));
    plan
}

fn sample_promotion_transform() -> PromotionPlanTransformV1 {
    promoted_deployment_plan_transform_from_inputs(&PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    })
    .expect("sample promotion transform should validate")
}

fn sample_execution_preflight_for_plan(plan_id: &str) -> DeploymentExecutionPreflightV1 {
    DeploymentExecutionPreflightV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_id: plan_id.to_string(),
        safety_report_id: "report-1".to_string(),
        authority_plan_id: plan_id.to_string(),
        backend: DeploymentExecutorBackendV1::CurrentCli,
        status: DeploymentExecutionPreflightStatusV1::Ready,
        planned_phases: vec!["install_root".to_string(), "activate_root".to_string()],
        required_capabilities: vec![
            DeploymentExecutorCapabilityV1::StageArtifact,
            DeploymentExecutorCapabilityV1::InstallCode,
        ],
        missing_capabilities: Vec::new(),
        blockers: Vec::new(),
    }
}

fn sample_artifact_promotion_plan() -> ArtifactPromotionPlanV1 {
    let target_plan = sample_promotion_target_plan();
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let readiness = promotion_readiness_from_inputs(
        "promotion-ready-1",
        &target_plan,
        std::slice::from_ref(&input),
    );
    let artifact_identity_report =
        promotion_artifact_identity_report_from_inputs(PromotionArtifactIdentityReportRequest {
            report_id: "promotion-artifact-identity-1".to_string(),
            inputs: vec![input],
        })
        .expect("sample artifact identity report should validate");
    let transform = sample_promotion_transform();
    let target_execution_lineage =
        promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
            lineage_id: "target-execution-lineage-1".to_string(),
            generated_at: "2026-05-25T00:00:00Z".to_string(),
            transform: transform.clone(),
            execution_preflight: sample_execution_preflight_for_plan("promoted-plan-1"),
        })
        .expect("sample target execution lineage should validate");

    artifact_promotion_plan(ArtifactPromotionPlanRequest {
        plan_id: "artifact-promotion-plan-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        readiness,
        artifact_identity_report,
        transform,
        target_execution_lineage: Some(target_execution_lineage),
    })
    .expect("sample artifact promotion plan should validate")
}

fn sample_artifact_promotion_provenance_report() -> ArtifactPromotionProvenanceReportV1 {
    artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
        report_id: "promotion-provenance-1".to_string(),
        artifact_promotion_plan: sample_artifact_promotion_plan(),
        wasm_store_identity_report: Some(sample_wasm_store_identity_report()),
        wasm_store_catalog_verification: Some(sample_wasm_store_catalog_verification()),
        materialization_identity_report: Some(sample_materialization_identity_report()),
    })
    .expect("sample promotion provenance report should validate")
}

fn sample_artifact_promotion_execution_receipt() -> ArtifactPromotionExecutionReceiptV1 {
    artifact_promotion_execution_receipt(ArtifactPromotionExecutionReceiptRequest {
        receipt_id: "promotion-execution-receipt-1".to_string(),
        provenance_report: sample_artifact_promotion_provenance_report(),
        deployment_receipt: sample_promoted_deployment_receipt(),
    })
    .expect("sample promotion execution receipt should validate")
}

fn sample_promoted_deployment_receipt() -> DeploymentReceiptV1 {
    let mut receipt = sample_receipt_with_phase(
        "promoted-plan-1",
        Some("aaaaa-aa"),
        ObservationStatusV1::Observed,
        RolePhaseResultV1::Applied,
    );
    receipt.operation_id = "promoted-operation-1".to_string();
    receipt.phase_receipts[0].phase = "promote_artifacts".to_string();
    receipt.role_phase_receipts[0].phase = "install_root".to_string();
    receipt.role_phase_receipts[0].artifact_digest = Some(sample_sha256("5"));
    receipt.role_phase_receipts[0].observed_module_hash_after = Some(sample_sha256("7"));
    receipt.role_phase_receipts[0].canonical_embedded_config_sha256 = Some(sample_sha256("3"));
    receipt
}

fn sample_wasm_store_identity_report() -> PromotionWasmStoreIdentityReportV1 {
    promotion_wasm_store_identity_report_from_staging(PromotionWasmStoreIdentityReportRequest {
        report_id: "wasm-store-identity-1".to_string(),
        staging_receipts: vec![sample_wasm_store_staging_receipt()],
    })
    .expect("sample wasm-store identity report should validate")
}

fn sample_wasm_store_catalog_entry() -> PromotionWasmStoreCatalogEntryV1 {
    PromotionWasmStoreCatalogEntryV1 {
        locator: "root:aaaaa-aa:bootstrap".to_string(),
        artifact_identity: "embedded:root:0.44.0:abc123".to_string(),
        published_chunk_count: 2,
    }
}

fn sample_wasm_store_catalog_verification() -> PromotionWasmStoreCatalogVerificationV1 {
    promotion_wasm_store_catalog_verification(PromotionWasmStoreCatalogVerificationRequest {
        verification_id: "wasm-store-catalog-1".to_string(),
        wasm_store_identity_report: sample_wasm_store_identity_report(),
        catalog_entries: vec![sample_wasm_store_catalog_entry()],
    })
    .expect("sample wasm-store catalog verification should validate")
}

fn sample_materialization_identity_report() -> PromotionMaterializationIdentityReportV1 {
    promotion_materialization_identity_report_from_evidence(
        PromotionMaterializationIdentityReportRequest {
            report_id: "materialization-report-1".to_string(),
            evidence: vec![sample_build_materialization_evidence()],
        },
    )
    .expect("sample materialization identity report should validate")
}

fn sample_sha256(seed: &str) -> String {
    seed.repeat(64)
}

fn sample_plan() -> DeploymentPlanV1 {
    DeploymentPlanV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_id: "plan-local-root".to_string(),
        deployment_identity: sample_identity(),
        trust_domain: TrustDomainV1 {
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            migration_from: None,
        },
        fleet_template: "root".to_string(),
        runtime_variant: "local".to_string(),
        authority_profile: AuthorityProfileV1 {
            profile_id: "local-default".to_string(),
            expected_controllers: vec!["aaaaa-aa".to_string()],
            staging_controllers: Vec::new(),
            emergency_controllers: Vec::new(),
        },
        role_artifacts: vec![sample_role_artifact()],
        expected_canisters: vec![ExpectedCanisterV1 {
            role: "root".to_string(),
            canister_id: Some("aaaaa-aa".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
        }],
        expected_pool: Vec::new(),
        expected_verifier_readiness: VerifierReadinessExpectationV1 {
            required: true,
            expected_role_epochs: vec![RoleEpochExpectationV1 {
                role: "root".to_string(),
                minimum_epoch: 1,
            }],
        },
        unresolved_assumptions: Vec::new(),
    }
}

fn sample_matching_inventory() -> DeploymentInventoryV1 {
    DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-22T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("canic.toml".to_string()),
            raw_config_sha256: Some("raw".to_string()),
            canonical_embedded_config_sha256: Some("runtime".to_string()),
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string()],
            module_hash: Some("module".to_string()),
            status: Some("running".to_string()),
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: Some("canonical".to_string()),
            role_assignment_source: Some("icp_canister_status".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: Some("gzip".to_string()),
            payload_size_bytes: Some(42),
            source: ArtifactSourceV1::LocalBuild,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::Observed,
            role_epochs: vec![RoleEpochObservationV1 {
                role: "root".to_string(),
                observed_epoch: Some(1),
                status: ObservationStatusV1::Observed,
            }],
        },
        unresolved_observations: Vec::new(),
    }
}

fn sample_check(plan: DeploymentPlanV1, inventory: DeploymentInventoryV1) -> DeploymentCheckV1 {
    let diff = compare_plan_to_inventory(&plan, &inventory);
    let report = safety_report_from_diff("report-1", Some("diff-1".to_string()), &diff);
    DeploymentCheckV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        check_id: "check-1".to_string(),
        plan,
        inventory,
        diff,
        report,
    }
}

fn sample_authority_evidence() -> AuthorityDryRunEvidenceV1 {
    sample_authority_evidence_from_check(sample_check(sample_plan(), sample_matching_inventory()))
}

fn sample_authority_evidence_from_check(check: DeploymentCheckV1) -> AuthorityDryRunEvidenceV1 {
    authority_dry_run_evidence_from_check(
        &check,
        "authority-evidence-1",
        "authority-report-1",
        "authority-dry-run-1",
        "2026-05-23T00:00:01Z",
    )
    .expect("build authority evidence")
}

fn sample_unknown_unsafe_check() -> DeploymentCheckV1 {
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "unsafe-canister".to_string(),
        role: Some("surprise".to_string()),
        control_class: CanisterControlClassV1::UnknownUnsafe,
        controllers: vec!["unknown-controller".to_string()],
        module_hash: None,
        status: None,
        root_trust_anchor: None,
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    });

    sample_check(sample_plan(), inventory)
}

fn sample_receipt_with_phase(
    plan_id: &str,
    root_principal: Option<&str>,
    postcondition: ObservationStatusV1,
    role_result: RolePhaseResultV1,
) -> DeploymentReceiptV1 {
    DeploymentReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        operation_id: "operation-1".to_string(),
        plan_id: plan_id.to_string(),
        execution_context: None,
        operation_status: DeploymentExecutionStatusV1::Complete,
        started_at: "2026-05-22T00:00:00Z".to_string(),
        finished_at: Some("2026-05-22T00:00:01Z".to_string()),
        operator_principal: None,
        root_principal: root_principal.map(str::to_string),
        previous_observed_deployment_epoch: None,
        phase_receipts: vec![PhaseReceiptV1 {
            phase: "materialize_artifacts".to_string(),
            started_at: "2026-05-22T00:00:00Z".to_string(),
            finished_at: Some("2026-05-22T00:00:01Z".to_string()),
            attempted_action: "verify configured role artifacts are materialized".to_string(),
            verified_postcondition: VerifiedPostconditionV1 {
                status: postcondition,
                evidence: vec!["artifact:root:sha256:file".to_string()],
            },
        }],
        role_phase_receipts: vec![RolePhaseReceiptV1 {
            role: "root".to_string(),
            phase: "materialize_artifacts".to_string(),
            result: role_result,
            previous_module_hash: None,
            target_module_hash: Some("module".to_string()),
            observed_module_hash_after: None,
            artifact_digest: Some("file".to_string()),
            canonical_embedded_config_sha256: Some("canonical".to_string()),
            error: (role_result == RolePhaseResultV1::Failed)
                .then(|| "artifact_missing: missing observed artifact for role root".to_string()),
        }],
        final_inventory_id: Some("inventory-1".to_string()),
        command_result: DeploymentCommandResultV1::Succeeded,
    }
}

fn sample_wasm_store_staging_receipt() -> StagingReceiptV1 {
    StagingReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        role: "root".to_string(),
        artifact_identity: "embedded:root:0.44.0:abc123".to_string(),
        transport: ArtifactTransportV1::WasmStore,
        wasm_store_locator: Some("root:aaaaa-aa:bootstrap".to_string()),
        prepared_chunk_hashes: vec!["chunk-a".to_string(), "chunk-b".to_string()],
        published_chunk_count: 2,
        verified_postcondition: VerifiedPostconditionV1 {
            status: ObservationStatusV1::Observed,
            evidence: vec!["payload_sha256:abc123".to_string()],
        },
    }
}

fn sample_role_phase_receipt(result: RolePhaseResultV1) -> RolePhaseReceiptV1 {
    RolePhaseReceiptV1 {
        role: "root".to_string(),
        phase: "install_root".to_string(),
        result,
        previous_module_hash: None,
        target_module_hash: Some("module".to_string()),
        observed_module_hash_after: (result == RolePhaseResultV1::Applied)
            .then(|| "module".to_string()),
        artifact_digest: Some("file".to_string()),
        canonical_embedded_config_sha256: Some("canonical".to_string()),
        error: (result == RolePhaseResultV1::Failed).then(|| "install failed".to_string()),
    }
}

fn assert_sha256_len(value: Option<&String>) {
    assert_eq!(value.map(String::len), Some(64));
}

struct TempWorkspace {
    path: std::path::PathBuf,
}

impl TempWorkspace {
    fn new(name: &str) -> Self {
        let path = temp_dir(name);
        fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn write_artifact(icp_root: &Path, role: &str, bytes: &[u8]) {
    let path = icp_root
        .join(".icp")
        .join("local")
        .join("canisters")
        .join(role)
        .join(format!("{role}.wasm.gz"));
    fs::create_dir_all(path.parent().expect("artifact parent")).expect("create artifact dir");
    fs::write(path, bytes).expect("write artifact");
}

fn write_release_set_manifest(icp_root: &Path) {
    let path = icp_root
        .join(".icp")
        .join("local")
        .join("canisters")
        .join("root")
        .join(ROOT_RELEASE_SET_MANIFEST_FILE);
    let manifest = serde_json::json!({
        "release_version": "0.41.1",
        "entries": [{
            "role": "user_hub",
            "template_id": "embedded:user_hub",
            "artifact_relative_path": ".icp/local/canisters/user_hub/user_hub.wasm.gz",
            "payload_size_bytes": 17,
            "payload_sha256_hex": "user-hub-hash",
            "chunk_size_bytes": 1_048_576,
            "chunk_sha256_hex": ["user-hub-hash"]
        }]
    });
    fs::create_dir_all(path.parent().expect("manifest parent")).expect("create manifest dir");
    fs::write(
        path,
        serde_json::to_vec_pretty(&manifest).expect("encode manifest"),
    )
    .expect("write manifest");
}

fn sample_install_state(root_canister_id: &str) -> InstallState {
    InstallState {
        schema_version: 1,
        fleet: "demo".to_string(),
        installed_at_unix_secs: 1,
        network: "local".to_string(),
        root_target: "root".to_string(),
        root_canister_id: root_canister_id.to_string(),
        root_build_target: "root".to_string(),
        workspace_root: "/workspace".to_string(),
        icp_root: "/workspace".to_string(),
        config_path: "fleets/canic.toml".to_string(),
        release_set_manifest_path: ".icp/local/canisters/root/release-set.json".to_string(),
    }
}
