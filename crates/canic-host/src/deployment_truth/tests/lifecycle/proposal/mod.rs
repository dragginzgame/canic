use super::super::*;

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
    std::assert_matches!(
        err,
        ExternalUpgradeProposalReportError::DigestMismatch {
            field: "report_digest"
        }
    );
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
    std::assert_matches!(
        err,
        ExternalUpgradeProposalReportError::SourceMismatch {
            field: "deployment_check"
        }
    );
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
    std::assert_matches!(
        err,
        ExternalUpgradeProposalReportError::DigestMismatch {
            field: "proposal_digest"
        }
    );
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
