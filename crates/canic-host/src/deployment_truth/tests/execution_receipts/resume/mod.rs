use super::super::*;
use crate::deployment_truth::report::{
    ARTIFACT_MISSING_CODE, DUPLICATE_RECEIPT_PHASE_CODE, DUPLICATE_RECEIPT_ROLE_PHASE_CODE,
    RECEIPT_EXECUTION_STATUS_MISMATCH_CODE, RECEIPT_PHASE_CONFLICT_CODE,
    RECEIPT_PLAN_MISMATCH_CODE, RECEIPT_POSTCONDITION_UNVERIFIED_CODE,
    RECEIPT_ROLE_PHASE_CONFLICT_CODE,
};

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
            .any(|finding| finding.code == RECEIPT_PLAN_MISMATCH_CODE)
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
            .any(|finding| finding.code == RECEIPT_POSTCONDITION_UNVERIFIED_CODE)
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
        finding.code == RECEIPT_EXECUTION_STATUS_MISMATCH_CODE
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
            .any(|finding| finding.code == RECEIPT_PHASE_CONFLICT_CODE
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
            .any(|finding| finding.code == DUPLICATE_RECEIPT_PHASE_CODE
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
    conflicting.error = Some(ARTIFACT_MISSING_CODE.to_string());
    receipt.role_phase_receipts.push(conflicting);

    let diff = compare_plan_inventory_and_receipt(&plan, &inventory, &receipt);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(diff.resumable_phases.is_empty());
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == RECEIPT_ROLE_PHASE_CONFLICT_CODE
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
            .any(|finding| finding.code == DUPLICATE_RECEIPT_ROLE_PHASE_CODE
                && finding.subject.as_deref() == Some("root:materialize_artifacts"))
    );
}
