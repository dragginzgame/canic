use super::super::*;

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
