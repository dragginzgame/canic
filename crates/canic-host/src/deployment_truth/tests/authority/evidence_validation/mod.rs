use super::super::*;

#[test]
fn authority_dry_run_evidence_rejects_mismatched_nested_check_id() {
    let mut evidence = sample_authority_evidence();
    evidence.check_id = "other-check".to_string();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mismatched nested check id should fail evidence validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::EvidenceCheckIdMismatch {
            component: "report",
            ..
        }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_unsupported_schema_version() {
    let mut evidence = sample_authority_evidence();
    evidence.schema_version = DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("unsupported evidence schema should fail validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::SchemaVersionMismatch {
            component: "evidence",
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found
        } if found == DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1
    );
}

#[test]
fn authority_dry_run_evidence_rejects_nested_schema_version_drift() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_report.schema_version = DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("nested schema drift should fail evidence validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::SchemaVersionMismatch {
            component: "report",
            ..
        }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_blank_required_identity() {
    let mut evidence = sample_authority_evidence();
    evidence.evidence_id = "  ".to_string();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("blank evidence identity should fail validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "evidence.evidence_id"
        }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_missing_nested_check_provenance() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_report.check_id = None;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("full evidence should carry nested report check provenance");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "report.check_id"
        }
    );
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

    std::assert_matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "receipt.hard_failures",
        }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_mutated_report_counts() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_report.counts.already_correct = 0;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mutated report counts should fail evidence validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "report.counts",
        }
    );
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

    std::assert_matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "report.apply_readiness",
        }
    );
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

    std::assert_matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "report.apply_readiness",
        }
    );
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

    std::assert_matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptAttemptedActions { count: 1 }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_non_complete_receipt_status() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_receipt.operation_status = DeploymentExecutionStatusV1::FailedBeforeMutation;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("non-complete dry-run receipts should fail evidence validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptStatus {
            status: DeploymentExecutionStatusV1::FailedBeforeMutation
        }
    );
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

    std::assert_matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptCommandResult {
            result: DeploymentCommandResultV1::Failed { .. }
        }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_complete_receipt_without_finished_at() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_receipt.finished_at = None;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("complete dry-run receipts should record finished_at");

    std::assert_matches!(err, AuthorityEvidenceError::DryRunReceiptMissingFinishedAt);
}

#[test]
fn authority_dry_run_evidence_rejects_generated_at_mismatch() {
    let mut evidence = sample_authority_evidence();
    evidence.generated_at = "2026-05-23T00:00:02Z".to_string();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("evidence generated_at should match receipt completion time");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::EvidenceGeneratedAtMismatch {
            evidence_value,
            receipt_value,
        } if evidence_value == "2026-05-23T00:00:02Z"
            && receipt_value == "2026-05-23T00:00:01Z"
    );
}

#[test]
fn authority_dry_run_evidence_rejects_receipt_finished_before_started() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_receipt.started_at = "2026-05-23T00:00:02Z".to_string();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("dry-run receipt finish time should not precede start time");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptTimestampOrder {
            field: "receipt.started_at",
            other_field: "receipt.finished_at",
            ..
        }
    );
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

    std::assert_matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "receipt.verified_controller_observations",
        }
    );
}
