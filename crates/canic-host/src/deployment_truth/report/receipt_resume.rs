use super::super::*;
use super::{
    compare_plan_to_inventory, duplicate_evidence_groups, finding, resume_safety_reasons,
    safety_status,
};
use std::collections::BTreeSet;

/// Compare intended state, observed inventory, and a prior receipt into a
/// resume-aware deployment diff.
#[must_use]
pub fn compare_plan_inventory_and_receipt(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    receipt: &DeploymentReceiptV1,
) -> DeploymentDiffV1 {
    let mut diff = compare_plan_to_inventory(plan, inventory);
    apply_receipt_resume_safety(plan, receipt, &mut diff);
    diff
}

fn apply_receipt_resume_safety(
    plan: &DeploymentPlanV1,
    receipt: &DeploymentReceiptV1,
    diff: &mut DeploymentDiffV1,
) {
    validate_receipt_identity(plan, receipt, &mut diff.hard_failures);
    validate_receipt_command_result(receipt, &mut diff.hard_failures);
    validate_receipt_execution_status(receipt, &mut diff.hard_failures);
    let phase_conflicts =
        validate_receipt_phase_duplicates(receipt, &mut diff.hard_failures, &mut diff.warnings);
    let role_phase_conflicts = validate_receipt_role_phase_duplicates(
        receipt,
        &mut diff.hard_failures,
        &mut diff.warnings,
    );
    if !diff.hard_failures.is_empty() {
        diff.resume_safety.status = safety_status(&diff.hard_failures, &diff.warnings);
        diff.resume_safety.reasons = resume_safety_reasons(&diff.hard_failures, &diff.warnings);
        return;
    }
    let phase_failures = receipt_phase_failures(receipt);
    for receipt in &receipt.phase_receipts {
        if phase_conflicts.contains(&receipt.phase) {
            continue;
        }
        if receipt.verified_postcondition.status != ObservationStatusV1::Observed {
            diff.hard_failures.push(finding(
                "receipt_postcondition_unverified",
                format!(
                    "receipt phase {} has no observed postcondition",
                    receipt.phase
                ),
                SafetySeverityV1::HardFailure,
                Some(receipt.phase.clone()),
            ));
            continue;
        }
        if phase_failures.contains(receipt.phase.as_str()) {
            continue;
        }
        if role_phase_conflicts.contains(receipt.phase.as_str()) {
            continue;
        }
        diff.resumable_phases.push(receipt.phase.clone());
    }
    diff.resumable_phases.sort();
    diff.resumable_phases.dedup();
    diff.resume_safety.status = safety_status(&diff.hard_failures, &diff.warnings);
    diff.resume_safety.reasons = resume_safety_reasons(&diff.hard_failures, &diff.warnings);
}

fn validate_receipt_phase_duplicates(
    receipt: &DeploymentReceiptV1,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) -> BTreeSet<String> {
    let mut conflicting_phases = BTreeSet::new();
    for group in duplicate_evidence_groups(
        &receipt.phase_receipts,
        |phase_receipt| phase_receipt.phase.as_str().to_string(),
        receipt_phase_evidence_label,
        " | ",
    ) {
        if group.is_conflict {
            conflicting_phases.insert(group.subject.clone());
            hard_failures.push(finding(
                "receipt_phase_conflict",
                format!(
                    "receipt phase {} has conflicting evidence: {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            warnings.push(finding(
                "duplicate_receipt_phase",
                format!(
                    "receipt phase {} was reported {} times with identical evidence",
                    group.subject, group.count
                ),
                SafetySeverityV1::Warning,
                Some(group.subject),
            ));
        }
    }
    conflicting_phases
}

fn receipt_phase_evidence_label(receipt: &PhaseReceiptV1) -> String {
    format!(
        "status={:?};evidence={}",
        receipt.verified_postcondition.status,
        receipt.verified_postcondition.evidence.join(",")
    )
}

fn validate_receipt_role_phase_duplicates(
    receipt: &DeploymentReceiptV1,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) -> BTreeSet<String> {
    let mut conflicting_phases = BTreeSet::new();
    for group in duplicate_evidence_groups(
        &receipt.role_phase_receipts,
        role_phase_subject,
        role_phase_evidence_label,
        " | ",
    ) {
        if group.is_conflict {
            if let Some(phase) = group
                .subject
                .rsplit_once(':')
                .map(|(_, phase)| phase.to_string())
            {
                conflicting_phases.insert(phase);
            }
            hard_failures.push(finding(
                "receipt_role_phase_conflict",
                format!(
                    "receipt role phase {} has conflicting evidence: {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            warnings.push(finding(
                "duplicate_receipt_role_phase",
                format!(
                    "receipt role phase {} was reported {} times with identical evidence",
                    group.subject, group.count
                ),
                SafetySeverityV1::Warning,
                Some(group.subject),
            ));
        }
    }
    conflicting_phases
}

fn role_phase_subject(receipt: &RolePhaseReceiptV1) -> String {
    format!("{}:{}", receipt.role, receipt.phase)
}

fn role_phase_evidence_label(receipt: &RolePhaseReceiptV1) -> String {
    format!(
        "result={:?};previous={};target={};observed={};artifact={};config={};error={}",
        receipt.result,
        receipt.previous_module_hash.as_deref().unwrap_or("<none>"),
        receipt.target_module_hash.as_deref().unwrap_or("<none>"),
        receipt
            .observed_module_hash_after
            .as_deref()
            .unwrap_or("<none>"),
        receipt.artifact_digest.as_deref().unwrap_or("<none>"),
        receipt
            .canonical_embedded_config_sha256
            .as_deref()
            .unwrap_or("<none>"),
        receipt.error.as_deref().unwrap_or("<none>")
    )
}

fn validate_receipt_identity(
    plan: &DeploymentPlanV1,
    receipt: &DeploymentReceiptV1,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    if receipt.plan_id != plan.plan_id {
        hard_failures.push(finding(
            "receipt_plan_mismatch",
            format!(
                "receipt plan {} does not match current plan {}",
                receipt.plan_id, plan.plan_id
            ),
            SafetySeverityV1::HardFailure,
            Some("receipt.plan_id".to_string()),
        ));
    }
    if let (Some(expected), Some(observed)) = (
        plan.deployment_identity.root_principal.as_ref(),
        receipt.root_principal.as_ref(),
    ) && expected != observed
    {
        hard_failures.push(finding(
            "receipt_root_mismatch",
            format!("receipt root {observed} does not match current plan root {expected}"),
            SafetySeverityV1::HardFailure,
            Some("receipt.root_principal".to_string()),
        ));
    }
}

fn validate_receipt_command_result(
    receipt: &DeploymentReceiptV1,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    if let DeploymentCommandResultV1::Failed { code, message } = &receipt.command_result {
        hard_failures.push(finding(
            "receipt_failed_command",
            format!("receipt command failed with {code}: {message}"),
            SafetySeverityV1::HardFailure,
            Some("receipt.command_result".to_string()),
        ));
    }
}

fn validate_receipt_execution_status(
    receipt: &DeploymentReceiptV1,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    let derived_status = deployment_execution_status_for_receipt_parts(
        &receipt.command_result,
        &receipt.role_phase_receipts,
    );
    let status_is_consistent = match receipt.operation_status {
        DeploymentExecutionStatusV1::FailedAfterMutation
            if matches!(
                derived_status,
                DeploymentExecutionStatusV1::FailedBeforeMutation
            ) =>
        {
            receipt.role_phase_receipts.is_empty()
        }
        _ => receipt.operation_status == derived_status,
    };

    if !status_is_consistent {
        hard_failures.push(finding(
            "receipt_execution_status_mismatch",
            format!(
                "receipt operation status {:?} does not match command result and role-phase evidence {:?}",
                receipt.operation_status, derived_status
            ),
            SafetySeverityV1::HardFailure,
            Some("receipt.operation_status".to_string()),
        ));
    }
}

fn receipt_phase_failures(receipt: &DeploymentReceiptV1) -> BTreeSet<&str> {
    let mut failures = BTreeSet::new();
    for role_receipt in &receipt.role_phase_receipts {
        if matches!(role_receipt.result, RolePhaseResultV1::Failed) {
            failures.insert(role_receipt.phase.as_str());
        }
    }
    failures
}
