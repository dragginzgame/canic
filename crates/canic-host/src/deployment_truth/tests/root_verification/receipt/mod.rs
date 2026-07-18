use super::super::*;

#[test]
fn root_verification_receipt_validation_accepts_state_transition() {
    let receipt = sample_root_verification_receipt();

    assert!(validate_deployment_root_verification_receipt(&receipt).is_ok());
}

#[test]
fn root_verification_receipt_round_trips_through_json() {
    let receipt = sample_root_verification_receipt();

    assert_json_round_trip(&receipt);
}

#[test]
fn root_verification_receipt_json_shape_is_stable() {
    let receipt = sample_root_verification_receipt();
    let value = serde_json::to_value(&receipt).expect("encode root verification receipt");

    assert_object_keys(
        &value,
        &[
            "schema_version",
            "receipt_id",
            "receipt_digest",
            "deployment_name",
            "environment",
            "fleet_template",
            "root_principal",
            "previous_root_verification",
            "new_root_verification",
            "state_transition",
            "source_report_id",
            "source_report_digest",
            "source_report_requested_at",
            "source_report_source",
            "source_report_evidence_status",
            "source_report_current_root_verification",
            "source_report_state_transition",
            "source_root_observation_source",
            "source_observed_root_canister_id",
            "source_check_id",
            "source_check_digest",
            "source_deployment_plan_id",
            "source_deployment_plan_digest",
            "source_inventory_id",
            "source_inventory_digest",
            "verified_at_unix_secs",
            "local_state_path",
            "local_state_digest_before",
            "local_state_digest_after",
            "warnings",
        ],
    );
    assert_eq!(value["schema_version"], DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(value["deployment_name"], "demo");
    assert_eq!(value["environment"], "local");
    assert_eq!(value["fleet_template"], "root");
    assert_eq!(value["root_principal"], "aaaaa-aa");
    assert_eq!(value["previous_root_verification"], "NotVerified");
    assert_eq!(value["new_root_verification"], "Verified");
    assert_eq!(value["state_transition"], "PromotedNotVerifiedToVerified");
    assert_eq!(value["source_report_requested_at"], "2026-05-27T00:00:00Z");
    assert_eq!(value["source_report_source"], "DeploymentTruthCheck");
    assert_eq!(value["source_report_evidence_status"], "EvidenceSatisfied");
    assert_eq!(
        value["source_report_current_root_verification"],
        "NotVerified"
    );
    assert_eq!(
        value["source_report_state_transition"],
        "WouldPromoteNotVerifiedToVerified"
    );
    assert_eq!(value["source_root_observation_source"], "IcpCanisterStatus");
    assert_eq!(value["source_observed_root_canister_id"], "aaaaa-aa");
    assert_eq!(value["source_check_id"], "check-1");
    assert_eq!(value["source_deployment_plan_id"], "plan-local-root");
    assert_eq!(value["source_inventory_id"], "inventory-1");
    assert_eq!(value["receipt_digest"].as_str().expect("digest").len(), 64);
}

#[test]
fn root_verification_receipt_text_distinguishes_local_state_write_from_canister_execution() {
    let receipt = sample_root_verification_receipt();
    let text = deployment_root_verification_receipt_text(&receipt);

    assert!(text.contains("mode: local-state-write"));
    assert!(text.contains("canister_execution: none"));
    assert!(text.contains("local_state_write: recorded"));
    assert!(text.contains("source_report_requested_at: 2026-05-27T00:00:00Z"));
    assert!(text.contains("source_report_source: DeploymentTruthCheck"));
    assert!(text.contains("source_report_evidence_status: EvidenceSatisfied"));
    assert!(text.contains("source_report_current_root_verification: NotVerified"));
    assert!(text.contains("source_report_state_transition: WouldPromoteNotVerifiedToVerified"));
    assert!(text.contains("source_root_observation_source: IcpCanisterStatus"));
    assert!(text.contains("source_observed_root_canister_id: aaaaa-aa"));
    assert!(!text.lines().any(|line| line == "execution: none"));
}

#[test]
fn root_verification_receipt_validation_rejects_digest_drift() {
    let mut receipt = sample_root_verification_receipt();
    receipt.environment = "other-environment".to_string();

    let err = validate_deployment_root_verification_receipt(&receipt)
        .expect_err("receipt digest drift should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReceiptError::DigestMismatch {
            field: "receipt_digest"
        }
    );
}

#[test]
fn root_verification_receipt_validation_rejects_bad_digest_shape() {
    let mut receipt = sample_root_verification_receipt();
    receipt.source_check_digest = "NOT-A-DIGEST".to_string();

    let err = validate_deployment_root_verification_receipt(&receipt)
        .expect_err("malformed source check digest should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReceiptError::InvalidSha256Digest {
            field: "source_check_digest"
        }
    );
}

#[test]
fn root_verification_receipt_validation_rejects_unsatisfied_source_report_status() {
    let mut receipt = sample_root_verification_receipt();
    receipt.source_report_evidence_status =
        DeploymentRootVerificationEvidenceStatusV1::VerificationFailed;
    receipt.receipt_digest = deployment_root_verification_receipt_digest(&receipt);

    let err = validate_deployment_root_verification_receipt(&receipt)
        .expect_err("receipt source report status drift should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReceiptError::SourceEvidenceMismatch
    );
}

#[test]
fn root_verification_receipt_validation_rejects_missing_source_report_timestamp() {
    let mut receipt = sample_root_verification_receipt();
    receipt.source_report_requested_at.clear();
    receipt.receipt_digest = deployment_root_verification_receipt_digest(&receipt);

    let err = validate_deployment_root_verification_receipt(&receipt)
        .expect_err("missing source report timestamp should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReceiptError::MissingRequiredField {
            field: "source_report_requested_at"
        }
    );
}

#[test]
fn root_verification_receipt_validation_rejects_bad_source_report_timestamp() {
    let mut receipt = sample_root_verification_receipt();
    receipt.source_report_requested_at = "not-a-timestamp".to_string();
    receipt.receipt_digest = deployment_root_verification_receipt_digest(&receipt);

    let err = validate_deployment_root_verification_receipt(&receipt)
        .expect_err("bad source report timestamp should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReceiptError::InvalidTimestampLabel {
            field: "source_report_requested_at"
        }
    );
}

#[test]
fn root_verification_receipt_validation_accepts_unix_source_report_timestamp() {
    let mut receipt = sample_root_verification_receipt();
    receipt.source_report_requested_at = "unix:100".to_string();
    receipt.receipt_digest = deployment_root_verification_receipt_digest(&receipt);

    assert!(validate_deployment_root_verification_receipt(&receipt).is_ok());
}

#[test]
fn root_verification_receipt_validation_rejects_unix_source_timestamp_mismatch() {
    let mut receipt = sample_root_verification_receipt();
    receipt.source_report_requested_at = "unix:101".to_string();
    receipt.receipt_digest = deployment_root_verification_receipt_digest(&receipt);

    let err = validate_deployment_root_verification_receipt(&receipt)
        .expect_err("unix source report timestamp mismatch should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReceiptError::SourceEvidenceMismatch
    );
}

#[test]
fn root_verification_receipt_validation_rejects_wrong_source_report_transition() {
    let mut receipt = sample_root_verification_receipt();
    receipt.source_report_state_transition =
        DeploymentRootVerificationStateTransitionV1::NoStateChange;
    receipt.receipt_digest = deployment_root_verification_receipt_digest(&receipt);

    let err = validate_deployment_root_verification_receipt(&receipt)
        .expect_err("receipt source report transition mismatch should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReceiptError::SourceEvidenceMismatch
    );
}

#[test]
fn root_verification_receipt_validation_rejects_source_report_current_state_mismatch() {
    let mut receipt = sample_root_verification_receipt();
    receipt.source_report_current_root_verification = DeploymentRootVerificationStateV1::Verified;
    receipt.receipt_digest = deployment_root_verification_receipt_digest(&receipt);

    let err = validate_deployment_root_verification_receipt(&receipt)
        .expect_err("receipt source report current state mismatch should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReceiptError::SourceEvidenceMismatch
    );
}

#[test]
fn root_verification_receipt_validation_rejects_local_state_root_source() {
    let mut receipt = sample_root_verification_receipt();
    receipt.source_root_observation_source =
        DeploymentRootObservationSourceV1::LocalDeploymentState;
    receipt.receipt_digest = deployment_root_verification_receipt_digest(&receipt);

    let err = validate_deployment_root_verification_receipt(&receipt)
        .expect_err("receipt source root observation source drift should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReceiptError::SourceEvidenceMismatch
    );
}

#[test]
fn root_verification_receipt_validation_rejects_observed_root_canister_id_mismatch() {
    let mut receipt = sample_root_verification_receipt();
    receipt.source_observed_root_canister_id = "other-root".to_string();
    receipt.receipt_digest = deployment_root_verification_receipt_digest(&receipt);

    let err = validate_deployment_root_verification_receipt(&receipt)
        .expect_err("receipt source observed root canister id mismatch should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReceiptError::SourceEvidenceMismatch
    );
}

#[test]
fn root_verification_receipt_validation_rejects_bad_transition() {
    let mut receipt = sample_root_verification_receipt();
    receipt.state_transition = DeploymentRootVerificationStateTransitionV1::NoStateChange;
    receipt.receipt_digest = deployment_root_verification_receipt_digest(&receipt);

    let err = validate_deployment_root_verification_receipt(&receipt)
        .expect_err("invalid transition should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReceiptError::StateTransitionMismatch
    );
}

#[test]
fn root_verification_receipt_validation_rejects_noop_digest_change() {
    let mut receipt = sample_root_verification_receipt();
    receipt.previous_root_verification = DeploymentRootVerificationStateV1::Verified;
    receipt.state_transition = DeploymentRootVerificationStateTransitionV1::NoStateChange;
    receipt.source_report_current_root_verification = DeploymentRootVerificationStateV1::Verified;
    receipt.source_report_state_transition =
        DeploymentRootVerificationStateTransitionV1::NoStateChange;
    receipt.receipt_digest = deployment_root_verification_receipt_digest(&receipt);

    let err = validate_deployment_root_verification_receipt(&receipt)
        .expect_err("no-op receipt with changed state digest should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReceiptError::LocalStateDigestMismatch
    );
}

#[test]
fn root_verification_receipt_validation_rejects_promotion_without_digest_change() {
    let mut receipt = sample_root_verification_receipt();
    receipt.local_state_digest_after = receipt.local_state_digest_before.clone();
    receipt.receipt_digest = deployment_root_verification_receipt_digest(&receipt);

    let err = validate_deployment_root_verification_receipt(&receipt)
        .expect_err("promotion receipt without state digest change should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReceiptError::LocalStateDigestMismatch
    );
}
