use super::super::*;

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
    std::assert_matches!(err, LifecycleAuthorityReportError::CountMismatch);

    let mut report = lifecycle_authority_report_from_check("lifecycle-authority-1", &check);
    report.authorities[0]
        .warnings
        .push("stale warning".to_string());
    let err = validate_lifecycle_authority_report(&report).expect_err("digest drift should fail");
    std::assert_matches!(
        err,
        LifecycleAuthorityReportError::DigestMismatch {
            field: "report_digest"
        }
    );
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
    std::assert_matches!(
        err,
        ExternalLifecyclePlanError::DigestMismatch {
            field: "lifecycle_plan_digest"
        }
    );

    plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    plan.status = ExternalLifecyclePlanStatusV1::Blocked;
    plan.lifecycle_plan_digest = sample_sha256("9");
    let err = validate_external_lifecycle_plan(&plan).expect_err("status drift should fail");
    std::assert_matches!(err, ExternalLifecyclePlanError::DigestMismatch { .. });
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
    std::assert_matches!(
        err,
        ExternalLifecyclePlanError::SourceMismatch {
            field: "deployment_check"
        }
    );
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
