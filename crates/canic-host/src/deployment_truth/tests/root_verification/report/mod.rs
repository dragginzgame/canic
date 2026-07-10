use super::super::*;
use crate::deployment_truth::report::{ARTIFACT_MISSING_CODE, UNVERIFIED_DEPLOYMENT_ROOT_CODE};
use crate::deployment_truth::root::{
    ROOT_VERIFICATION_CHECK_FAILED_CODE, ROOT_VERIFICATION_SOURCE_CHECK_DIFF_STALE_CODE,
    ROOT_VERIFICATION_SOURCE_CHECK_REPORT_STALE_CODE,
    ROOT_VERIFICATION_SOURCE_CHECK_SCHEMA_MISMATCH_CODE,
};

#[test]
fn root_verification_report_accepts_bound_root_evidence() {
    let check = sample_root_verification_check();
    let report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));

    assert_eq!(
        report.evidence_status,
        DeploymentRootVerificationEvidenceStatusV1::EvidenceSatisfied,
        "{:?}",
        report.blockers
    );
    assert_eq!(
        report.state_transition,
        DeploymentRootVerificationStateTransitionV1::WouldPromoteNotVerifiedToVerified
    );
    assert_eq!(report.deployment_name, "demo");
    assert_eq!(report.expected_fleet_template, "root");
    assert_eq!(report.expected_root_principal, "aaaaa-aa");
    assert_eq!(report.observed_root_principal.as_deref(), Some("aaaaa-aa"));
    assert_eq!(
        report.observed_root_canister_id.as_deref(),
        Some("aaaaa-aa")
    );
    assert_eq!(
        report.observed_root_observation_source,
        Some(DeploymentRootObservationSourceV1::IcpCanisterStatus)
    );
    assert!(report.blockers.is_empty());
    assert_eq!(report.source_deployment_plan_id, "plan-local-root");
    assert_eq!(report.source_inventory_id, "inventory-1");
    assert!(validate_deployment_root_verification_report(&report).is_ok());
}

#[test]
fn root_verification_report_json_shape_includes_observed_root_source() {
    let check = sample_root_verification_check();
    let report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));
    let value = serde_json::to_value(&report).expect("encode root verification report");

    assert_eq!(
        value["observed_root_observation_source"],
        "IcpCanisterStatus"
    );
    assert_eq!(value["observed_root_canister_id"], "aaaaa-aa");
}

#[test]
fn root_verification_report_text_renders_observed_root_source() {
    let check = sample_root_verification_check();
    let report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));
    let text = deployment_root_verification_report_text(&report);

    assert!(text.contains("mode: passive"));
    assert!(text.contains("local_state_write: none"));
    assert!(text.contains("observed_root_canister_id: aaaaa-aa"));
    assert!(text.contains("observed_root_observation_source: IcpCanisterStatus"));
}

#[test]
fn root_verification_report_accepts_exact_unverified_root_blocker() {
    let mut plan = sample_root_verification_plan();
    plan.unresolved_assumptions.push(DeploymentAssumptionV1 {
        key: "local_state.unverified_root_canister_id".to_string(),
        description: "registered root is not verified".to_string(),
    });
    let check = sample_check(plan, sample_root_verification_inventory());
    let report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));

    assert_eq!(
        report.evidence_status,
        DeploymentRootVerificationEvidenceStatusV1::EvidenceSatisfied,
        "{:?}",
        report.blockers
    );
    assert!(report.blockers.is_empty());
    assert!(validate_deployment_root_verification_report(&report).is_ok());
}

#[test]
fn root_verification_report_rejects_unverified_root_plus_unrelated_blocker() {
    let mut plan = sample_root_verification_plan();
    plan.unresolved_assumptions.push(DeploymentAssumptionV1 {
        key: "local_state.unverified_root_canister_id".to_string(),
        description: "registered root is not verified".to_string(),
    });
    let mut check = sample_check(plan, sample_root_verification_inventory());
    check.report.status = SafetyStatusV1::Blocked;
    check.report.hard_failures.push(SafetyFindingV1 {
        code: ARTIFACT_MISSING_CODE.to_string(),
        message: "root artifact is missing".to_string(),
        severity: SafetySeverityV1::HardFailure,
        subject: Some("root".to_string()),
    });
    let report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));

    assert_eq!(
        report.evidence_status,
        DeploymentRootVerificationEvidenceStatusV1::VerificationFailed
    );
    assert!(
        report
            .blockers
            .iter()
            .any(|finding| finding.code == ARTIFACT_MISSING_CODE)
    );
    assert!(
        report
            .blockers
            .iter()
            .any(|finding| finding.code == UNVERIFIED_DEPLOYMENT_ROOT_CODE)
    );
}

#[test]
fn root_verification_report_rejects_local_state_only_root_evidence() {
    let mut inventory = sample_root_verification_inventory();
    let observed_root = inventory.observed_root.as_mut().expect("root evidence");
    observed_root.observation_source = DeploymentRootObservationSourceV1::LocalDeploymentState;
    observed_root.role_assignment_source = Some(
        RoleAssignmentSourceV1::LocalInstallState
            .label()
            .to_string(),
    );
    let check = sample_check(sample_root_verification_plan(), inventory);
    let report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));

    assert_eq!(
        report.evidence_status,
        DeploymentRootVerificationEvidenceStatusV1::VerificationFailed
    );
    assert_eq!(
        report.state_transition,
        DeploymentRootVerificationStateTransitionV1::Blocked
    );
    assert!(report.blockers.iter().any(|finding| finding.code
        == ROOT_VERIFICATION_CHECK_FAILED_CODE
        && finding.subject.as_deref() == Some("root_observation_source")));
}

#[test]
fn root_verification_report_rejects_missing_explicit_root_evidence() {
    let mut inventory = sample_root_verification_inventory();
    inventory.observed_root = None;
    let check = sample_check(sample_root_verification_plan(), inventory);
    let report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));

    assert_eq!(
        report.evidence_status,
        DeploymentRootVerificationEvidenceStatusV1::VerificationFailed
    );
    assert!(report.blockers.iter().any(|finding| finding.code
        == ROOT_VERIFICATION_CHECK_FAILED_CODE
        && finding.subject.as_deref() == Some("explicit_observed_root")));
}

#[test]
fn root_verification_report_rejects_unrelated_source_check_blocker() {
    let mut check = sample_root_verification_check();
    check.report.status = SafetyStatusV1::Blocked;
    check.report.hard_failures.push(SafetyFindingV1 {
        code: ARTIFACT_MISSING_CODE.to_string(),
        message: "root artifact is missing".to_string(),
        severity: SafetySeverityV1::HardFailure,
        subject: Some("root".to_string()),
    });
    let report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));

    assert_eq!(
        report.evidence_status,
        DeploymentRootVerificationEvidenceStatusV1::VerificationFailed
    );
    assert!(
        report
            .blockers
            .iter()
            .any(|finding| finding.code == ARTIFACT_MISSING_CODE)
    );
}

#[test]
fn root_verification_report_rejects_stale_source_check_diff() {
    let mut check = sample_root_verification_check();
    check.diff.warnings.push(SafetyFindingV1 {
        code: "tampered_diff".to_string(),
        message: "tampered diff".to_string(),
        severity: SafetySeverityV1::Warning,
        subject: Some("diff".to_string()),
    });
    let report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));

    assert_eq!(
        report.evidence_status,
        DeploymentRootVerificationEvidenceStatusV1::VerificationFailed
    );
    assert!(report.blockers.iter().any(|finding| {
        finding.code == ROOT_VERIFICATION_SOURCE_CHECK_DIFF_STALE_CODE
            && finding.subject.as_deref() == Some("check-1")
    }));
}

#[test]
fn root_verification_report_rejects_unsupported_source_check_schema() {
    let mut check = sample_root_verification_check();
    check.schema_version = DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1;
    let report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));

    assert_eq!(
        report.evidence_status,
        DeploymentRootVerificationEvidenceStatusV1::VerificationFailed
    );
    assert!(report.blockers.iter().any(|finding| {
        finding.code == ROOT_VERIFICATION_SOURCE_CHECK_SCHEMA_MISMATCH_CODE
            && finding.subject.as_deref() == Some("check-1")
    }));
}

#[test]
fn root_verification_report_rejects_stale_source_check_report() {
    let mut check = sample_root_verification_check();
    check
        .report
        .next_actions
        .push("tampered next action".to_string());
    let report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));

    assert_eq!(
        report.evidence_status,
        DeploymentRootVerificationEvidenceStatusV1::VerificationFailed
    );
    assert!(report.blockers.iter().any(|finding| {
        finding.code == ROOT_VERIFICATION_SOURCE_CHECK_REPORT_STALE_CODE
            && finding.subject.as_deref() == Some("check-1")
    }));
}

#[test]
fn root_verification_report_validation_rejects_digest_drift() {
    let check = sample_root_verification_check();
    let mut report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));
    report
        .recommended_next_actions
        .push("stale next action".to_string());

    let err = validate_deployment_root_verification_report(&report)
        .expect_err("digest drift should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReportError::DigestMismatch {
            field: "report_digest"
        }
    );
}

#[test]
fn root_verification_report_validation_rejects_bad_digest_shape() {
    let check = sample_root_verification_check();
    let mut report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));
    report.source_check_digest = "NOT-A-DIGEST".to_string();

    let err = validate_deployment_root_verification_report(&report)
        .expect_err("malformed source check digest should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReportError::InvalidSha256Digest {
            field: "source_check_digest"
        }
    );
}

#[test]
fn root_verification_report_validation_rejects_check_row_drift() {
    let check = sample_root_verification_check();
    let mut report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));
    report.identity_checks[0].observed = Some("other-deployment".to_string());
    report.identity_checks[0].satisfied = true;

    let err = validate_deployment_root_verification_report(&report)
        .expect_err("forged check row should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReportError::CheckMismatch {
            check: "deployment_name".to_string()
        }
    );
}

#[test]
fn root_verification_report_validation_rejects_observed_source_drift() {
    let check = sample_root_verification_check();
    let mut report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));
    report.observed_root_observation_source =
        Some(DeploymentRootObservationSourceV1::LocalDeploymentState);

    let err = validate_deployment_root_verification_report(&report)
        .expect_err("observed source drift should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReportError::CheckMismatch {
            check: "root_observation_source".to_string()
        }
    );
}

#[test]
fn root_verification_report_validation_rejects_observed_root_canister_id_drift() {
    let check = sample_root_verification_check();
    let mut report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));
    report.observed_root_canister_id = Some("other-root".to_string());

    let err = validate_deployment_root_verification_report(&report)
        .expect_err("observed root canister id drift should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReportError::CheckMismatch {
            check: "observed_root_canister_id".to_string()
        }
    );
}

#[test]
fn root_verification_report_validation_rejects_duplicate_check_row() {
    let check = sample_root_verification_check();
    let mut report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));
    report
        .identity_checks
        .push(report.identity_checks[0].clone());

    let err = validate_deployment_root_verification_report(&report)
        .expect_err("duplicate check row should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReportError::CheckMismatch {
            check: "deployment_name".to_string()
        }
    );
}

#[test]
fn root_verification_report_validation_rejects_unexpected_check_row() {
    let check = sample_root_verification_check();
    let mut report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));
    report.evidence_checks[0].name = "unexpected_check".to_string();

    let err = validate_deployment_root_verification_report(&report)
        .expect_err("unexpected check row should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReportError::CheckMismatch {
            check: "unexpected_check".to_string()
        }
    );
}

#[test]
fn root_verification_report_validation_rejects_displayed_field_drift() {
    let check = sample_root_verification_check();
    let mut report =
        deployment_root_verification_report_from_check(sample_root_verification_request(check));
    report.observed_root_principal = None;

    let err = validate_deployment_root_verification_report(&report)
        .expect_err("displayed root evidence drift should fail");

    assert_eq!(
        err,
        DeploymentRootVerificationReportError::CheckMismatch {
            check: "root_principal".to_string()
        }
    );
}
