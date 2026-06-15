use super::super::super::*;
use super::{
    error::AuthorityEvidenceError,
    observations::authority_controller_observation_from_action,
    shared::{
        ensure_authority_report_matches_plan, ensure_authority_schema_version,
        ensure_required_authority_field, ensure_required_optional_authority_field,
        ensure_timestamp_order,
    },
};

/// Build a read-only authority dry-run receipt from a deployment truth check.
///
/// This is the receipt-only counterpart to
/// `authority_dry_run_evidence_from_check(...)`: it preserves the same plan,
/// report, and check provenance without constructing a full evidence bundle.
pub fn authority_dry_run_receipt_from_check(
    check: &DeploymentCheckV1,
    report_id: impl Into<String>,
    receipt_id: impl Into<String>,
    started_at: impl Into<String>,
    finished_at: Option<String>,
) -> Result<AuthorityReceiptV1, AuthorityEvidenceError> {
    let reconciliation = build_authority_reconciliation_plan(check);
    let report = authority_report_from_plan_with_check_id(
        report_id,
        Some(check.check_id.clone()),
        &reconciliation,
    );
    authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id.clone()),
        receipt_id,
        started_at,
        finished_at,
    )
}

/// Build a read-only authority dry-run receipt using the standard local
/// deployment-truth artifact identifier.
pub fn authority_dry_run_receipt_from_check_with_local_id(
    check: &DeploymentCheckV1,
    generated_at: impl Into<String>,
) -> Result<AuthorityReceiptV1, AuthorityEvidenceError> {
    let generated_at = generated_at.into();
    authority_dry_run_receipt_from_check(
        check,
        local_authority_artifact_id(check, "authority-report"),
        local_authority_artifact_id(check, "authority-dry-run-receipt"),
        generated_at.clone(),
        Some(generated_at),
    )
}

/// Build an evidence-only receipt for a completed dry-run authority
/// reconciliation.
///
/// The receipt records that no controller mutations were attempted. The
/// original plan/report remain the authority for whether later apply work is
/// safe.
pub fn authority_dry_run_receipt_from_plan(
    plan: &AuthorityReconciliationPlanV1,
    report: &AuthorityReportV1,
    check_id: Option<String>,
    operation_id: impl Into<String>,
    started_at: impl Into<String>,
    finished_at: Option<String>,
) -> Result<AuthorityReceiptV1, AuthorityEvidenceError> {
    let operation_id = operation_id.into();
    let started_at = started_at.into();
    ensure_authority_receipt_source_inputs(
        plan,
        report,
        &operation_id,
        &started_at,
        finished_at.as_deref(),
    )?;
    ensure_authority_report_matches_plan(plan, report)?;
    if let (Some(receipt_check_id), Some(report_check_id)) = (&check_id, &report.check_id)
        && receipt_check_id != report_check_id
    {
        return Err(AuthorityEvidenceError::CheckIdMismatch {
            receipt_value: receipt_check_id.clone(),
            report_value: report_check_id.clone(),
        });
    }
    let receipt_check_id = check_id.or_else(|| report.check_id.clone());

    Ok(AuthorityReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        operation_id,
        check_id: receipt_check_id,
        reconciliation_plan_id: plan.plan_id.clone(),
        authority_report_id: report.report_id.clone(),
        inventory_id: plan.inventory_id.clone(),
        authority_profile_hash: plan.authority_profile_hash.clone(),
        operation_status: DeploymentExecutionStatusV1::Complete,
        started_at,
        finished_at,
        attempted_actions: Vec::new(),
        verified_controller_observations: plan
            .canister_actions
            .iter()
            .map(authority_controller_observation_from_action)
            .collect(),
        hard_failures: report.hard_failures.clone(),
        unresolved_observation_gaps: report.observation_gaps.clone(),
        unresolved_external_actions: report.external_actions_required.clone(),
        command_result: DeploymentCommandResultV1::Succeeded,
    })
}

fn ensure_authority_receipt_source_inputs(
    plan: &AuthorityReconciliationPlanV1,
    report: &AuthorityReportV1,
    operation_id: &str,
    started_at: &str,
    finished_at: Option<&str>,
) -> Result<(), AuthorityEvidenceError> {
    ensure_authority_schema_version("plan", plan.schema_version)?;
    ensure_authority_schema_version("report", report.schema_version)?;
    ensure_required_authority_field("plan.plan_id", &plan.plan_id)?;
    ensure_required_authority_field("plan.inventory_id", &plan.inventory_id)?;
    ensure_required_authority_field("report.report_id", &report.report_id)?;
    ensure_required_optional_authority_field("report.check_id", report.check_id.as_deref())?;
    ensure_required_authority_field("receipt.operation_id", operation_id)?;
    ensure_required_authority_field("receipt.started_at", started_at)?;
    ensure_required_optional_authority_field("receipt.finished_at", finished_at)?;
    let Some(finished_at) = finished_at else {
        return Err(AuthorityEvidenceError::MissingRequiredField {
            field: "receipt.finished_at",
        });
    };
    ensure_timestamp_order(
        "receipt.started_at",
        started_at,
        "receipt.finished_at",
        finished_at,
    )
}
