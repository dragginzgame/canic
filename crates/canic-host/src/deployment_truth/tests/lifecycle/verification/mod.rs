use super::super::*;

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
