use super::*;

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

#[test]
fn external_lifecycle_check_summarizes_pending_report() {
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
    let pending_report = external_lifecycle_pending_report_from_plan(
        "external-lifecycle-pending-1",
        &lifecycle_plan,
        &proposal_report,
    );

    let check_report = external_lifecycle_check_from_reports(
        "external-lifecycle-check-1",
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    );

    assert_eq!(check_report.check_id, "external-lifecycle-check-1");
    assert_eq!(
        check_report.status,
        ExternalLifecyclePlanStatusV1::PendingExternalAction
    );
    assert_eq!(check_report.direct_upgrade_count, 0);
    assert_eq!(check_report.pending_external_count, 1);
    assert_eq!(check_report.blocked_count, 0);
    assert_eq!(
        check_report.pending_report_digest,
        pending_report.report_digest
    );
    assert!(check_report.summary.contains("pending external action"));
    assert_eq!(check_report.next_actions.len(), 1);
    assert_eq!(check_report.check_digest.len(), 64);
    validate_external_lifecycle_check(&check_report).expect("check should validate");
    validate_external_lifecycle_check_for_reports(
        &check_report,
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    )
    .expect("check should validate against source reports");
    assert_json_round_trip(&check_report);
}

#[test]
fn external_lifecycle_check_validation_rejects_stale_source() {
    let (lifecycle_plan, mut pending_report) = sample_external_lifecycle_pending_artifacts();
    let proposal_report = ExternalUpgradeProposalReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: pending_report.proposal_report_id.clone(),
        report_digest: pending_report.proposal_report_digest.clone(),
        lifecycle_plan_id: lifecycle_plan.lifecycle_plan_id.clone(),
        lifecycle_plan_digest: lifecycle_plan.lifecycle_plan_digest.clone(),
        deployment_plan_id: lifecycle_plan.deployment_plan_id.clone(),
        deployment_plan_digest: lifecycle_plan.deployment_plan_digest.clone(),
        inventory_id: lifecycle_plan.inventory_id.clone(),
        proposals: Vec::new(),
        blocked_subjects: Vec::new(),
    };
    let check_report = external_lifecycle_check_from_reports(
        "external-lifecycle-check-1",
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    );
    pending_report.pending_external_count = 0;

    let err = validate_external_lifecycle_check_for_reports(
        &check_report,
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    )
    .expect_err("stale check count should fail");

    assert_eq!(err, ExternalLifecycleCheckError::CountMismatch);
}

#[test]
fn external_lifecycle_check_text_reports_passive_boundary() {
    let (lifecycle_plan, pending_report) = sample_external_lifecycle_pending_artifacts();
    let proposal_report = ExternalUpgradeProposalReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: pending_report.proposal_report_id.clone(),
        report_digest: pending_report.proposal_report_digest.clone(),
        lifecycle_plan_id: lifecycle_plan.lifecycle_plan_id.clone(),
        lifecycle_plan_digest: lifecycle_plan.lifecycle_plan_digest.clone(),
        deployment_plan_id: lifecycle_plan.deployment_plan_id.clone(),
        deployment_plan_digest: lifecycle_plan.deployment_plan_digest.clone(),
        inventory_id: lifecycle_plan.inventory_id.clone(),
        proposals: Vec::new(),
        blocked_subjects: Vec::new(),
    };
    let check_report = external_lifecycle_check_from_reports(
        "external-lifecycle-check-1",
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    );

    let text = external_lifecycle_check_text(&check_report);

    assert!(text.contains("External lifecycle check"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("pending_external: 1"));
    assert!(text.contains("next_actions"));
}

#[test]
fn external_lifecycle_handoff_packages_pending_proposals() {
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
    let pending_report = external_lifecycle_pending_report_from_plan(
        "external-lifecycle-pending-1",
        &lifecycle_plan,
        &proposal_report,
    );
    let lifecycle_check = external_lifecycle_check_from_reports(
        "external-lifecycle-check-1",
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    );

    let handoff = external_lifecycle_handoff_from_reports(
        "external-lifecycle-handoff-1",
        &lifecycle_check,
        &proposal_report,
        &pending_report,
    );

    assert_eq!(handoff.handoff_id, "external-lifecycle-handoff-1");
    assert_eq!(handoff.lifecycle_check_id, lifecycle_check.check_id);
    assert_eq!(handoff.handoff_actions.len(), 1);
    assert_eq!(
        handoff.handoff_actions[0].proposal_id,
        proposal_report.proposals[0].proposal_id
    );
    assert!(
        handoff.handoff_actions[0]
            .operator_instructions
            .iter()
            .any(|instruction| instruction.contains("verify live inventory"))
    );
    assert_eq!(handoff.handoff_digest.len(), 64);
    validate_external_lifecycle_handoff(&handoff).expect("handoff should validate");
    validate_external_lifecycle_handoff_for_reports(
        &handoff,
        &lifecycle_check,
        &proposal_report,
        &pending_report,
    )
    .expect("handoff should validate against source reports");
    assert_json_round_trip(&handoff);
}

#[test]
fn external_lifecycle_handoff_validation_rejects_stale_source() {
    let (lifecycle_plan, pending_report) = sample_external_lifecycle_pending_artifacts();
    let proposal_report = ExternalUpgradeProposalReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: pending_report.proposal_report_id.clone(),
        report_digest: pending_report.proposal_report_digest.clone(),
        lifecycle_plan_id: lifecycle_plan.lifecycle_plan_id.clone(),
        lifecycle_plan_digest: lifecycle_plan.lifecycle_plan_digest.clone(),
        deployment_plan_id: lifecycle_plan.deployment_plan_id.clone(),
        deployment_plan_digest: lifecycle_plan.deployment_plan_digest.clone(),
        inventory_id: lifecycle_plan.inventory_id.clone(),
        proposals: Vec::new(),
        blocked_subjects: Vec::new(),
    };
    let mut lifecycle_check = external_lifecycle_check_from_reports(
        "external-lifecycle-check-1",
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    );
    let handoff = external_lifecycle_handoff_from_reports(
        "external-lifecycle-handoff-1",
        &lifecycle_check,
        &proposal_report,
        &pending_report,
    );
    lifecycle_check.check_id = "other-check".to_string();

    let err = validate_external_lifecycle_handoff_for_reports(
        &handoff,
        &lifecycle_check,
        &proposal_report,
        &pending_report,
    )
    .expect_err("stale handoff source should fail");

    assert_eq!(
        err,
        ExternalLifecycleHandoffError::SourceMismatch {
            field: "lifecycle_check_id"
        }
    );
}

#[test]
fn external_lifecycle_handoff_text_reports_passive_boundary() {
    let (lifecycle_plan, pending_report) = sample_external_lifecycle_pending_artifacts();
    let proposal_report = ExternalUpgradeProposalReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: pending_report.proposal_report_id.clone(),
        report_digest: pending_report.proposal_report_digest.clone(),
        lifecycle_plan_id: lifecycle_plan.lifecycle_plan_id.clone(),
        lifecycle_plan_digest: lifecycle_plan.lifecycle_plan_digest.clone(),
        deployment_plan_id: lifecycle_plan.deployment_plan_id.clone(),
        deployment_plan_digest: lifecycle_plan.deployment_plan_digest.clone(),
        inventory_id: lifecycle_plan.inventory_id.clone(),
        proposals: Vec::new(),
        blocked_subjects: Vec::new(),
    };
    let lifecycle_check = external_lifecycle_check_from_reports(
        "external-lifecycle-check-1",
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    );
    let handoff = external_lifecycle_handoff_from_reports(
        "external-lifecycle-handoff-1",
        &lifecycle_check,
        &proposal_report,
        &pending_report,
    );

    let text = external_lifecycle_handoff_text(&handoff);

    assert!(text.contains("External lifecycle handoff"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("handoff_actions"));
}

#[test]
fn critical_external_fix_report_summarizes_residual_exposure() {
    let (lifecycle_plan, pending_report) = sample_external_lifecycle_pending_artifacts();

    let report = critical_external_fix_report_from_pending(
        "critical-fix-1",
        "fix-2026-05-critical",
        "critical",
        &lifecycle_plan,
        &pending_report,
    );

    assert_eq!(report.fix_id, "fix-2026-05-critical");
    assert_eq!(report.severity, "critical");
    assert_eq!(report.lifecycle_plan_id, lifecycle_plan.lifecycle_plan_id);
    assert_eq!(report.pending_report_id, pending_report.report_id);
    assert_eq!(report.affected_roles, vec!["root"]);
    assert_eq!(report.affected_canisters, vec!["aaaaa-aa"]);
    assert!(report.directly_patchable_roles.is_empty());
    assert_eq!(report.externally_blocked_roles, vec!["root"]);
    assert!(report.dependency_blocked_roles.is_empty());
    assert_eq!(report.required_external_actions.len(), 1);
    assert!(!report.protected_call_implications.is_empty());
    assert!(!report.residual_exposure.is_empty());
    assert!(
        report
            .operator_next_steps
            .iter()
            .any(|step| step.contains("external consent"))
    );
    validate_critical_external_fix_report(&report).expect("critical fix report should validate");
    validate_critical_external_fix_report_for_pending(&report, &lifecycle_plan, &pending_report)
        .expect("critical fix report should validate against source artifacts");
    assert_json_round_trip(&report);
}

#[test]
fn critical_external_fix_report_validation_rejects_stale_source() {
    let (lifecycle_plan, mut pending_report) = sample_external_lifecycle_pending_artifacts();
    let report = critical_external_fix_report_from_pending(
        "critical-fix-1",
        "fix-2026-05-critical",
        "critical",
        &lifecycle_plan,
        &pending_report,
    );

    pending_report.report_id = "other-pending-report".to_string();
    let err = validate_critical_external_fix_report_for_pending(
        &report,
        &lifecycle_plan,
        &pending_report,
    )
    .expect_err("stale source should fail");

    assert_eq!(
        err,
        CriticalExternalFixReportError::SourceMismatch {
            field: "pending_report_id"
        }
    );
}

#[test]
fn critical_external_fix_report_text_reports_passive_boundary() {
    let (lifecycle_plan, pending_report) = sample_external_lifecycle_pending_artifacts();
    let report = critical_external_fix_report_from_pending(
        "critical-fix-1",
        "fix-2026-05-critical",
        "critical",
        &lifecycle_plan,
        &pending_report,
    );

    let text = critical_external_fix_report_text(&report);

    assert!(text.contains("Critical external fix report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("externally_blocked_roles: 1"));
    assert!(text.contains("residual_exposure"));
    assert!(text.contains("operator_next_steps"));
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
    std::assert_matches!(
        err,
        ExternalUpgradeReceiptError::DigestMismatch {
            field: "receipt_digest"
        }
    );
}

#[test]
fn external_upgrade_receipt_validation_rejects_mismatched_proposal_source() {
    let (mut mismatched, receipt) = sample_external_upgrade_proposal_and_receipt();
    mismatched.proposal_id = "other-proposal".to_string();

    let err = validate_external_upgrade_receipt_for_proposal(&receipt, &mismatched)
        .expect_err("receipt cannot validate against another proposal");

    std::assert_matches!(
        err,
        ExternalUpgradeReceiptError::SourceMismatch {
            field: "proposal_id"
        }
    );
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
    std::assert_matches!(err, ExternalUpgradeReceiptError::RefusedConsentVerified);

    receipt.verification_result = ExternalUpgradeVerificationResultV1::Pending;
    receipt.receipt_id.clear();
    let err =
        validate_external_upgrade_receipt(&receipt).expect_err("blank receipt id should fail");
    std::assert_matches!(
        err,
        ExternalUpgradeReceiptError::MissingRequiredField {
            field: "receipt_id"
        }
    );
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
fn external_upgrade_consent_evidence_packages_receipt_without_verification_claim() {
    let (proposal, receipt) = sample_external_upgrade_proposal_and_receipt();

    let evidence = external_upgrade_consent_evidence_from_receipt(
        "external-upgrade-consent-1",
        &proposal,
        &receipt,
    )
    .expect("consent evidence should build");

    assert_eq!(evidence.evidence_id, "external-upgrade-consent-1");
    assert_eq!(evidence.proposal_id, proposal.proposal_id);
    assert_eq!(evidence.proposal_digest, proposal.proposal_digest);
    assert_eq!(evidence.receipt_id, receipt.receipt_id);
    assert_eq!(evidence.receipt_digest, receipt.receipt_digest);
    assert_eq!(evidence.consent_state, receipt.consent_state);
    assert_eq!(evidence.reported_by, receipt.reported_by);
    assert_eq!(evidence.consent_requirements, proposal.consent_requirements);
    assert_eq!(
        evidence.allowed_authorization_modes,
        proposal.allowed_authorization_modes
    );
    assert!(
        evidence
            .status_summary
            .contains("external controller execution")
    );
    assert_eq!(evidence.evidence_digest.len(), 64);
    validate_external_upgrade_consent_evidence(&evidence)
        .expect("consent evidence should validate");
    validate_external_upgrade_consent_evidence_for_receipt(&evidence, &proposal, &receipt)
        .expect("consent evidence should validate against source evidence");
    assert_json_round_trip(&evidence);
}

#[test]
fn external_upgrade_consent_evidence_json_shape_is_stable() {
    let (proposal, receipt) = sample_external_upgrade_proposal_and_receipt();
    let evidence = external_upgrade_consent_evidence_from_receipt(
        "external-upgrade-consent-1",
        &proposal,
        &receipt,
    )
    .expect("consent evidence should build");
    let encoded = serde_json::to_value(&evidence).expect("evidence should encode");

    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "evidence_id",
            "evidence_digest",
            "proposal_id",
            "proposal_digest",
            "receipt_id",
            "receipt_digest",
            "subject",
            "canister_id",
            "role",
            "consent_state",
            "reported_by",
            "consent_requirements",
            "allowed_authorization_modes",
            "status_summary",
        ],
    );
}

#[test]
fn external_upgrade_consent_evidence_request_json_shape_is_stable() {
    let (proposal, receipt) = sample_external_upgrade_proposal_and_receipt();
    let request = ExternalUpgradeConsentEvidenceRequest {
        evidence_id: "external-upgrade-consent-1".to_string(),
        proposal,
        receipt,
    };
    let encoded = serde_json::to_value(&request).expect("request should encode");

    assert_object_keys(&encoded, &["evidence_id", "proposal", "receipt"]);
    assert_json_round_trip(&request);
}

#[test]
fn external_upgrade_consent_evidence_validation_rejects_stale_source() {
    let (mut proposal, receipt) = sample_external_upgrade_proposal_and_receipt();
    let evidence = external_upgrade_consent_evidence_from_receipt(
        "external-upgrade-consent-1",
        &proposal,
        &receipt,
    )
    .expect("consent evidence should build");
    proposal.proposal_id = "other-proposal".to_string();

    let err =
        validate_external_upgrade_consent_evidence_for_receipt(&evidence, &proposal, &receipt)
            .expect_err("stale source should fail");

    std::assert_matches!(
        err,
        ExternalUpgradeConsentEvidenceError::Receipt(ExternalUpgradeReceiptError::SourceMismatch {
            field: "proposal_id"
        })
    );
}

#[test]
fn external_upgrade_consent_evidence_text_reports_passive_boundary() {
    let (proposal, receipt) = sample_external_upgrade_proposal_and_receipt();
    let evidence = external_upgrade_consent_evidence_from_receipt(
        "external-upgrade-consent-1",
        &proposal,
        &receipt,
    )
    .expect("consent evidence should build");

    let text = external_upgrade_consent_evidence_text(&evidence);

    assert!(text.contains("External upgrade consent evidence"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("consent_state: executed_externally"));
    assert!(text.contains("status_summary"));
}

#[test]
fn external_upgrade_verification_report_packages_verified_receipt() {
    let (proposal, receipt) = sample_external_upgrade_proposal_and_receipt();

    let report = external_upgrade_verification_report_from_receipt(
        "external-upgrade-verification-1",
        &proposal,
        &receipt,
    )
    .expect("verification report should build");

    assert_eq!(report.report_id, "external-upgrade-verification-1");
    assert_eq!(report.proposal_id, proposal.proposal_id);
    assert_eq!(report.proposal_digest, proposal.proposal_digest);
    assert_eq!(report.receipt_id, receipt.receipt_id);
    assert_eq!(report.receipt_digest, receipt.receipt_digest);
    assert_eq!(
        report.verification_result,
        ExternalUpgradeVerificationResultV1::Verified
    );
    assert!(report.live_inventory_required);
    assert!(report.status_summary.contains("matches proposal target"));
    assert_eq!(report.report_digest.len(), 64);
    validate_external_upgrade_verification_report(&report)
        .expect("verification report should validate");
    validate_external_upgrade_verification_report_for_receipt(&report, &proposal, &receipt)
        .expect("verification report should validate against source evidence");
    assert_json_round_trip(&report);
}

#[test]
fn external_upgrade_verification_report_request_json_shape_is_stable() {
    let (proposal, receipt) = sample_external_upgrade_proposal_and_receipt();
    let request = ExternalUpgradeVerificationReportRequest {
        report_id: "external-upgrade-verification-1".to_string(),
        proposal,
        receipt,
    };
    let encoded = serde_json::to_value(&request).expect("request should encode");

    assert_object_keys(&encoded, &["report_id", "proposal", "receipt"]);
    assert_json_round_trip(&request);
}

#[test]
fn external_upgrade_verification_report_json_shape_is_stable() {
    let (proposal, receipt) = sample_external_upgrade_proposal_and_receipt();
    let report = external_upgrade_verification_report_from_receipt(
        "external-upgrade-verification-1",
        &proposal,
        &receipt,
    )
    .expect("verification report should build");
    let encoded = serde_json::to_value(&report).expect("report should encode");

    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "report_id",
            "report_digest",
            "proposal_id",
            "proposal_digest",
            "receipt_id",
            "receipt_digest",
            "subject",
            "canister_id",
            "role",
            "verification_result",
            "verification_notes",
            "live_inventory_required",
            "status_summary",
        ],
    );
}

#[test]
fn external_upgrade_verification_report_validation_rejects_stale_source() {
    let (mut proposal, receipt) = sample_external_upgrade_proposal_and_receipt();
    let report = external_upgrade_verification_report_from_receipt(
        "external-upgrade-verification-1",
        &proposal,
        &receipt,
    )
    .expect("verification report should build");
    proposal.proposal_id = "other-proposal".to_string();

    let err =
        validate_external_upgrade_verification_report_for_receipt(&report, &proposal, &receipt)
            .expect_err("stale source should fail");

    std::assert_matches!(
        err,
        ExternalUpgradeVerificationReportError::Receipt(
            ExternalUpgradeReceiptError::SourceMismatch {
                field: "proposal_id"
            }
        )
    );
}

#[test]
fn external_upgrade_verification_report_text_reports_passive_boundary() {
    let (proposal, receipt) = sample_external_upgrade_proposal_and_receipt();
    let report = external_upgrade_verification_report_from_receipt(
        "external-upgrade-verification-1",
        &proposal,
        &receipt,
    )
    .expect("verification report should build");

    let text = external_upgrade_verification_report_text(&report);

    assert!(text.contains("External upgrade verification report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("verification_result: verified"));
    assert!(text.contains("live_inventory_required: true"));
}

#[test]
fn external_upgrade_verification_policy_packages_proposal_requirements() {
    let (proposal, _) = sample_external_upgrade_proposal_and_receipt();

    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );

    assert_eq!(policy.policy_id, "external-upgrade-verification-policy-1");
    assert_eq!(policy.proposal_id, proposal.proposal_id);
    assert_eq!(policy.proposal_digest, proposal.proposal_digest);
    assert_eq!(policy.subject, proposal.subject);
    assert_eq!(
        policy.required_verification,
        proposal.verification_requirements
    );
    assert_required_policy_requirement(
        &policy,
        LifecycleVerificationRequirementV1::LiveInventory,
        None,
    );
    assert_required_policy_requirement(
        &policy,
        LifecycleVerificationRequirementV1::ControllerObservation,
        Some("UserControlled"),
    );
    assert_required_policy_requirement(
        &policy,
        LifecycleVerificationRequirementV1::ModuleHash,
        proposal.target_installed_module_hash.as_deref(),
    );
    assert_required_policy_requirement(
        &policy,
        LifecycleVerificationRequirementV1::CanonicalEmbeddedConfig,
        proposal.target_canonical_embedded_config_sha256.as_deref(),
    );
    assert_eq!(policy.policy_digest.len(), 64);
    validate_external_upgrade_verification_policy(&policy).expect("policy should validate");
    validate_external_upgrade_verification_policy_for_proposal(&policy, &proposal)
        .expect("policy should validate against proposal");
    assert_json_round_trip(&policy);
}

#[test]
fn external_upgrade_verification_policy_json_shape_is_stable() {
    let (proposal, _) = sample_external_upgrade_proposal_and_receipt();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let encoded = serde_json::to_value(&policy).expect("policy should encode");

    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "policy_id",
            "policy_digest",
            "proposal_id",
            "proposal_digest",
            "deployment_plan_id",
            "deployment_plan_digest",
            "subject",
            "canister_id",
            "role",
            "required_verification",
            "verification_requirements",
            "max_observation_age_seconds",
            "status_summary",
        ],
    );
}

#[test]
fn external_upgrade_verification_policy_request_json_shape_is_stable() {
    let (proposal, _) = sample_external_upgrade_proposal_and_receipt();
    let request = ExternalUpgradeVerificationPolicyRequest {
        policy_id: "external-upgrade-verification-policy-1".to_string(),
        proposal,
    };
    let encoded = serde_json::to_value(&request).expect("request should encode");

    assert_object_keys(&encoded, &["policy_id", "proposal"]);
    assert_json_round_trip(&request);
}

#[test]
fn external_upgrade_verification_policy_validation_rejects_stale_source() {
    let (mut proposal, _) = sample_external_upgrade_proposal_and_receipt();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    proposal.proposal_id = "other-proposal".to_string();

    let err = validate_external_upgrade_verification_policy_for_proposal(&policy, &proposal)
        .expect_err("stale source should fail");

    assert_eq!(
        err,
        ExternalUpgradeVerificationPolicyError::SourceMismatch { field: "proposal" }
    );
}

#[test]
fn external_upgrade_verification_policy_text_reports_passive_boundary() {
    let (proposal, _) = sample_external_upgrade_proposal_and_receipt();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );

    let text = external_upgrade_verification_policy_text(&policy);

    assert!(text.contains("External upgrade verification policy"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("verification_requirements"));
    assert!(text.contains("requirement=live_inventory status=required"));
}

#[test]
fn external_upgrade_verification_check_evaluates_supplied_observation() {
    let (proposal, _) = sample_external_upgrade_proposal_and_receipt();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );

    let check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        matching_external_verification_observation(&proposal),
    );

    assert_eq!(check.check_id, "external-upgrade-verification-check-1");
    assert_eq!(check.policy_id, policy.policy_id);
    assert_eq!(check.policy_digest, policy.policy_digest);
    assert_eq!(
        check.verification_result,
        ExternalUpgradeVerificationResultV1::Pending
    );
    assert_eq!(
        check.observation.source,
        ExternalVerificationObservationSourceV1::SuppliedObservation
    );
    assert!(check.status_summary.contains("deployment-truth inventory"));
    assert!(check.requirement_results.iter().all(|row| {
        row.status == ExternalUpgradeVerificationRequirementStatusV1::NotRequired
            || row.satisfied == Some(true)
    }));
    assert_eq!(check.check_digest.len(), 64);
    validate_external_upgrade_verification_check(&check).expect("check should validate");
    validate_external_upgrade_verification_check_for_policy(&check, &policy)
        .expect("check should validate against policy");
    assert_json_round_trip(&check);
}

#[test]
fn external_upgrade_verification_check_verifies_deployment_truth_inventory() {
    let (proposal, _, deployment_check) = sample_external_upgrade_proposal_receipt_and_check();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let observation =
        external_upgrade_verification_observation_from_check(&policy, &deployment_check)
            .expect("deployment check should produce verification observation");

    let check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        observation,
    );

    assert_eq!(
        check.observation.source,
        ExternalVerificationObservationSourceV1::DeploymentTruthInventory
    );
    assert_eq!(
        check.observation.deployment_check_id.as_deref(),
        Some(deployment_check.check_id.as_str())
    );
    assert_eq!(
        check.verification_result,
        ExternalUpgradeVerificationResultV1::Verified
    );
    validate_external_upgrade_verification_check_for_deployment_check(
        &check,
        &policy,
        &deployment_check,
    )
    .expect("inventory-backed verification check should validate against deployment check");
}

#[test]
fn external_upgrade_verification_check_rejects_stale_inventory_source() {
    let (proposal, _, mut deployment_check) = sample_external_upgrade_proposal_receipt_and_check();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let observation =
        external_upgrade_verification_observation_from_check(&policy, &deployment_check)
            .expect("deployment check should produce verification observation");
    let check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        observation,
    );
    deployment_check.inventory.observed_canisters[0].module_hash =
        Some("stale-module-hash".to_string());

    let err = validate_external_upgrade_verification_check_for_deployment_check(
        &check,
        &policy,
        &deployment_check,
    )
    .expect_err("stale inventory should fail closed");

    assert_eq!(
        err,
        ExternalUpgradeVerificationCheckError::SourceMismatch {
            field: "deployment_check"
        }
    );
}

#[test]
fn external_upgrade_verification_check_reports_inventory_fact_mismatches() {
    assert_inventory_verification_mismatch(
        |check| check.inventory.observed_canisters[0].module_hash = Some("wrong-module".into()),
        LifecycleVerificationRequirementV1::ModuleHash,
    );
    assert_inventory_verification_mismatch(
        |check| {
            check.inventory.observed_canisters[0].canonical_embedded_config_digest =
                Some("wrong-config".into());
        },
        LifecycleVerificationRequirementV1::CanonicalEmbeddedConfig,
    );
    assert_inventory_verification_mismatch(
        |check| {
            check.inventory.observed_canisters[0].control_class =
                CanisterControlClassV1::DeploymentControlled;
        },
        LifecycleVerificationRequirementV1::ControllerObservation,
    );
    assert_inventory_verification_mismatch(
        |check| {
            check.inventory.observed_verifier_readiness.status = ObservationStatusV1::NotObserved;
        },
        LifecycleVerificationRequirementV1::ProtectedCallReadiness,
    );
}

#[test]
fn external_upgrade_verification_check_rejects_stale_inventory_links() {
    let (proposal, _, deployment_check) = sample_external_upgrade_proposal_receipt_and_check();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let observation =
        external_upgrade_verification_observation_from_check(&policy, &deployment_check)
            .expect("deployment check should produce verification observation");
    let check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        observation,
    );

    let mut stale_check_id = check.clone();
    stale_check_id.observation.deployment_check_id = Some("other-check".to_string());
    assert!(
        validate_external_upgrade_verification_check_for_deployment_check(
            &stale_check_id,
            &policy,
            &deployment_check,
        )
        .is_err()
    );

    let mut stale_check_digest = check.clone();
    stale_check_digest.observation.deployment_check_digest = Some(sample_sha256("9"));
    assert!(
        validate_external_upgrade_verification_check_for_deployment_check(
            &stale_check_digest,
            &policy,
            &deployment_check,
        )
        .is_err()
    );

    let mut stale_inventory_id = check;
    stale_inventory_id.observation.inventory_id = Some("other-inventory".to_string());
    assert!(
        validate_external_upgrade_verification_check_for_deployment_check(
            &stale_inventory_id,
            &policy,
            &deployment_check,
        )
        .is_err()
    );
}

#[test]
fn external_upgrade_verification_observation_rejects_stale_deployment_plan() {
    let (proposal, _, mut deployment_check) = sample_external_upgrade_proposal_receipt_and_check();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    deployment_check.plan.plan_id = "other-plan".to_string();

    let err = external_upgrade_verification_observation_from_check(&policy, &deployment_check)
        .expect_err("deployment plan drift should fail closed");

    assert_eq!(
        err,
        ExternalUpgradeVerificationCheckError::SourceMismatch {
            field: "deployment_plan"
        }
    );
}

#[test]
fn external_upgrade_verification_check_reports_mismatch_for_bad_observation() {
    let (proposal, _) = sample_external_upgrade_proposal_and_receipt();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let mut observation = matching_external_verification_observation(&proposal);
    observation.observed_module_hash = Some("wrong-module-hash".to_string());

    let check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        observation,
    );

    assert_eq!(
        check.verification_result,
        ExternalUpgradeVerificationResultV1::Mismatch
    );
    assert!(check.requirement_results.iter().any(|row| {
        row.requirement == LifecycleVerificationRequirementV1::ModuleHash
            && row.satisfied == Some(false)
    }));
}

#[test]
fn external_upgrade_verification_check_json_shape_is_stable() {
    let (proposal, _) = sample_external_upgrade_proposal_and_receipt();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        matching_external_verification_observation(&proposal),
    );
    let encoded = serde_json::to_value(&check).expect("check should encode");

    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "check_id",
            "check_digest",
            "policy_id",
            "policy_digest",
            "proposal_id",
            "proposal_digest",
            "subject",
            "canister_id",
            "role",
            "observation",
            "requirement_results",
            "verification_result",
            "status_summary",
        ],
    );
}

#[test]
fn external_upgrade_verification_check_request_json_shape_is_stable() {
    let (proposal, _) = sample_external_upgrade_proposal_and_receipt();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let request = ExternalUpgradeVerificationCheckRequest {
        check_id: "external-upgrade-verification-check-1".to_string(),
        policy,
        observation: Some(matching_external_verification_observation(&proposal)),
        deployment_check: None,
    };
    let encoded = serde_json::to_value(&request).expect("request should encode");

    assert_object_keys(
        &encoded,
        &["check_id", "policy", "observation", "deployment_check"],
    );
    assert_json_round_trip(&request);
}

#[test]
fn external_upgrade_verification_check_validation_rejects_stale_policy() {
    let (proposal, _) = sample_external_upgrade_proposal_and_receipt();
    let mut policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        matching_external_verification_observation(&proposal),
    );
    policy.policy_id = "other-policy".to_string();

    let err = validate_external_upgrade_verification_check_for_policy(&check, &policy)
        .expect_err("stale policy should fail");

    assert_eq!(
        err,
        ExternalUpgradeVerificationCheckError::SourceMismatch { field: "policy" }
    );
}

#[test]
fn external_upgrade_verification_check_validation_rejects_duplicate_requirement() {
    let (proposal, _) = sample_external_upgrade_proposal_and_receipt();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let mut check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        matching_external_verification_observation(&proposal),
    );
    let duplicate = check.requirement_results[0].clone();
    check.requirement_results.push(duplicate.clone());

    let err = validate_external_upgrade_verification_check(&check)
        .expect_err("duplicate requirement should fail");

    assert_eq!(
        err,
        ExternalUpgradeVerificationCheckError::DuplicateRequirement {
            requirement: duplicate.requirement
        }
    );
}

#[test]
fn external_upgrade_verification_check_validation_rejects_bad_requirement_status() {
    let (proposal, _) = sample_external_upgrade_proposal_and_receipt();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let mut check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        matching_external_verification_observation(&proposal),
    );
    let requirement = check.requirement_results[0].requirement;
    check.requirement_results[0].satisfied = None;

    let err = validate_external_upgrade_verification_check(&check)
        .expect_err("missing required satisfaction should fail");

    assert_eq!(
        err,
        ExternalUpgradeVerificationCheckError::RequirementStatusMismatch { requirement }
    );
}

#[test]
fn external_upgrade_verification_check_text_reports_passive_boundary() {
    let (proposal, _) = sample_external_upgrade_proposal_and_receipt();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        matching_external_verification_observation(&proposal),
    );

    let text = external_upgrade_verification_check_text(&check);

    assert!(text.contains("External upgrade verification check"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("live_lookup: none"));
    assert!(text.contains("requirement_results"));
}

#[test]
fn external_upgrade_completion_report_marks_verified_completion() {
    let (proposal, consent_evidence, verification_check) = sample_external_completion_sources();

    let report = external_upgrade_completion_report_from_evidence(
        "external-upgrade-completion-1",
        &proposal,
        &consent_evidence,
        &verification_check,
    )
    .expect("completion report should build");

    assert_eq!(report.report_id, "external-upgrade-completion-1");
    assert_eq!(report.proposal_id, proposal.proposal_id);
    assert_eq!(report.consent_evidence_id, consent_evidence.evidence_id);
    assert_eq!(report.verification_check_id, verification_check.check_id);
    assert_eq!(
        report.completion_status,
        ExternalUpgradeCompletionStatusV1::VerifiedComplete
    );
    assert!(report.blockers.is_empty());
    assert_eq!(report.report_digest.len(), 64);
    validate_external_upgrade_completion_report(&report).expect("report should validate");
    validate_external_upgrade_completion_report_for_evidence(
        &report,
        &proposal,
        &consent_evidence,
        &verification_check,
    )
    .expect("report should validate against evidence");
    assert_json_round_trip(&report);
}

#[test]
fn external_upgrade_completion_report_does_not_complete_from_supplied_observation() {
    let (proposal, consent_evidence, _) = sample_external_completion_sources();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let verification_check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        matching_external_verification_observation(&proposal),
    );

    let report = external_upgrade_completion_report_from_evidence(
        "external-upgrade-completion-1",
        &proposal,
        &consent_evidence,
        &verification_check,
    )
    .expect("completion report should build");

    assert_eq!(
        report.verification_observation_source,
        ExternalVerificationObservationSourceV1::SuppliedObservation
    );
    assert_eq!(
        report.completion_status,
        ExternalUpgradeCompletionStatusV1::AwaitingVerification
    );
    assert_ne!(
        report.completion_status,
        ExternalUpgradeCompletionStatusV1::VerifiedComplete
    );
    assert!(!report.blockers.is_empty());
}

#[test]
fn external_upgrade_completion_report_marks_verification_failed() {
    let (proposal, consent_evidence, _) = sample_external_completion_sources();
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let mut observation = matching_external_verification_observation(&proposal);
    observation.observed_module_hash = Some("wrong-module-hash".to_string());
    let verification_check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        observation,
    );

    let report = external_upgrade_completion_report_from_evidence(
        "external-upgrade-completion-1",
        &proposal,
        &consent_evidence,
        &verification_check,
    )
    .expect("completion report should build");

    assert_eq!(
        report.completion_status,
        ExternalUpgradeCompletionStatusV1::VerificationFailed
    );
    assert_eq!(report.blockers.len(), 1);
}

#[test]
fn external_upgrade_completion_report_validation_rejects_stale_evidence() {
    let (proposal, mut consent_evidence, verification_check) = sample_external_completion_sources();
    let report = external_upgrade_completion_report_from_evidence(
        "external-upgrade-completion-1",
        &proposal,
        &consent_evidence,
        &verification_check,
    )
    .expect("completion report should build");
    consent_evidence.evidence_id = "other-consent-evidence".to_string();

    let err = validate_external_upgrade_completion_report_for_evidence(
        &report,
        &proposal,
        &consent_evidence,
        &verification_check,
    )
    .expect_err("stale evidence should fail");

    assert_eq!(
        err,
        ExternalUpgradeCompletionReportError::ConsentEvidence(
            ExternalUpgradeConsentEvidenceError::DigestMismatch {
                field: "evidence_digest"
            }
        )
    );
}

#[test]
fn external_upgrade_completion_report_json_shape_is_stable() {
    let (proposal, consent_evidence, verification_check) = sample_external_completion_sources();
    let report = external_upgrade_completion_report_from_evidence(
        "external-upgrade-completion-1",
        &proposal,
        &consent_evidence,
        &verification_check,
    )
    .expect("completion report should build");
    let encoded = serde_json::to_value(&report).expect("report should encode");

    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "report_id",
            "report_digest",
            "proposal_id",
            "proposal_digest",
            "consent_evidence_id",
            "consent_evidence_digest",
            "verification_check_id",
            "verification_check_digest",
            "subject",
            "canister_id",
            "role",
            "consent_state",
            "verification_result",
            "verification_observation_source",
            "completion_status",
            "blockers",
            "next_actions",
            "status_summary",
        ],
    );
}

#[test]
fn external_upgrade_completion_report_request_json_shape_is_stable() {
    let (proposal, consent_evidence, verification_check) = sample_external_completion_sources();
    let request = ExternalUpgradeCompletionReportRequest {
        report_id: "external-upgrade-completion-1".to_string(),
        proposal,
        consent_evidence,
        verification_check,
    };
    let encoded = serde_json::to_value(&request).expect("request should encode");

    assert_object_keys(
        &encoded,
        &[
            "report_id",
            "proposal",
            "consent_evidence",
            "verification_check",
        ],
    );
    assert_json_round_trip(&request);
}

#[test]
fn external_upgrade_completion_report_text_reports_passive_boundary() {
    let (proposal, consent_evidence, verification_check) = sample_external_completion_sources();
    let report = external_upgrade_completion_report_from_evidence(
        "external-upgrade-completion-1",
        &proposal,
        &consent_evidence,
        &verification_check,
    )
    .expect("completion report should build");

    let text = external_upgrade_completion_report_text(&report);

    assert!(text.contains("External upgrade completion report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("live_lookup: none"));
    assert!(text.contains("completion_status: verified_complete"));
}

#[test]
fn external_lifecycle_uses_canonical_control_class_model() {
    let model = include_str!("../model/mod.rs");
    let lifecycle_sources = [
        include_str!("../lifecycle/mod.rs"),
        include_str!("../lifecycle/authority_plan/mod.rs"),
        include_str!("../lifecycle/authority_plan/authority/mod.rs"),
        include_str!("../lifecycle/authority_plan/plan/mod.rs"),
        include_str!("../lifecycle/authority_plan/policy/mod.rs"),
        include_str!("../lifecycle/authority_plan/validation/mod.rs"),
        include_str!("../lifecycle/external_lifecycle/mod.rs"),
        include_str!("../lifecycle/external_lifecycle/check/mod.rs"),
        include_str!("../lifecycle/external_lifecycle/critical_fix/mod.rs"),
        include_str!("../lifecycle/external_lifecycle/handoff/mod.rs"),
        include_str!("../lifecycle/external_lifecycle/pending/mod.rs"),
        include_str!("../lifecycle/external_lifecycle/validation/mod.rs"),
        include_str!("../lifecycle/external_upgrade/mod.rs"),
        include_str!("../lifecycle/external_upgrade/completion/mod.rs"),
        include_str!("../lifecycle/external_upgrade/consent/mod.rs"),
        include_str!("../lifecycle/external_upgrade/proposal/mod.rs"),
        include_str!("../lifecycle/external_upgrade/receipt/mod.rs"),
        include_str!("../lifecycle/external_upgrade/validation/mod.rs"),
        include_str!("../lifecycle/external_upgrade/verification/mod.rs"),
        include_str!("../lifecycle/external_upgrade/verification/check/mod.rs"),
        include_str!("../lifecycle/external_upgrade/verification/policy/mod.rs"),
        include_str!("../lifecycle/external_upgrade/verification/report/mod.rs"),
        include_str!("../lifecycle/external_upgrade/verification/shared/mod.rs"),
    ];

    assert_eq!(model.matches("pub enum CanisterControlClassV1").count(), 1);
    assert!(
        lifecycle_sources
            .iter()
            .any(|source| source.contains("CanisterControlClassV1"))
    );

    for forbidden in [
        "ExternalControlClass",
        "ExternalLifecycleControlClass",
        "LifecycleControlClass",
        "UserControlClass",
        "UserLifecycleControlClass",
    ] {
        assert!(
            lifecycle_sources
                .iter()
                .all(|source| !source.contains(forbidden)),
            "external lifecycle must project from CanisterControlClassV1; found {forbidden}"
        );
    }
}
