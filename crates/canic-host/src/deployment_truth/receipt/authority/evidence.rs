use super::super::super::*;
use super::{
    dry_run_receipt::authority_dry_run_receipt_from_plan,
    error::AuthorityEvidenceError,
    observations::authority_controller_observation_from_action,
    shared::{
        ensure_authority_report_matches_plan, ensure_authority_schema_version,
        ensure_matching_authority_evidence_content, ensure_matching_authority_evidence_field,
        ensure_required_authority_field, ensure_required_optional_authority_field,
        ensure_timestamp_order, optional_authority_value,
    },
};

/// Validate that a dry-run authority evidence bundle is internally coherent.
///
/// This is a consistency guard for archived/read-only evidence. It does not
/// make the evidence authoritative over live controller state.
pub fn validate_authority_dry_run_evidence(
    evidence: &AuthorityDryRunEvidenceV1,
) -> Result<(), AuthorityEvidenceError> {
    ensure_authority_evidence_schema_versions(evidence)?;
    ensure_authority_evidence_required_fields(evidence)?;
    ensure_authority_report_matches_plan(
        &evidence.reconciliation_plan,
        &evidence.authority_report,
    )?;
    ensure_authority_evidence_provenance(evidence)?;
    ensure_authority_receipt_is_completed_dry_run(&evidence.authority_receipt)?;
    ensure_evidence_generated_at_matches_finished_at(
        &evidence.generated_at,
        evidence.authority_receipt.finished_at.as_deref(),
    )?;
    ensure_authority_receipt_timestamp_order(&evidence.authority_receipt)?;
    ensure_authority_receipt_matches_evidence(evidence)
}

/// Build a complete read-only authority dry-run evidence bundle from a
/// deployment truth check.
///
/// The returned bundle is validated before it leaves this function. It remains
/// archive/report evidence only; live controller inventory is still the
/// authority for future reconciliation.
pub fn authority_dry_run_evidence_from_check(
    check: &DeploymentCheckV1,
    evidence_id: impl Into<String>,
    report_id: impl Into<String>,
    receipt_id: impl Into<String>,
    generated_at: impl Into<String>,
) -> Result<AuthorityDryRunEvidenceV1, AuthorityEvidenceError> {
    let generated_at = generated_at.into();
    let reconciliation = build_authority_reconciliation_plan(check);
    let report = authority_report_from_plan_with_check_id(
        report_id,
        Some(check.check_id.clone()),
        &reconciliation,
    );
    let receipt = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id.clone()),
        receipt_id,
        generated_at.clone(),
        Some(generated_at.clone()),
    )?;
    let evidence = AuthorityDryRunEvidenceV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        evidence_id: evidence_id.into(),
        check_id: check.check_id.clone(),
        generated_at,
        reconciliation_plan: reconciliation,
        authority_report: report,
        authority_receipt: receipt,
    };
    validate_authority_dry_run_evidence(&evidence)?;
    Ok(evidence)
}

/// Build a complete read-only authority dry-run evidence bundle using the
/// standard local deployment-truth artifact identifiers.
pub fn authority_dry_run_evidence_from_check_with_local_ids(
    check: &DeploymentCheckV1,
    generated_at: impl Into<String>,
) -> Result<AuthorityDryRunEvidenceV1, AuthorityEvidenceError> {
    authority_dry_run_evidence_from_check(
        check,
        local_authority_artifact_id(check, "authority-evidence"),
        local_authority_artifact_id(check, "authority-report"),
        local_authority_artifact_id(check, "authority-dry-run-receipt"),
        generated_at,
    )
}

fn ensure_authority_evidence_schema_versions(
    evidence: &AuthorityDryRunEvidenceV1,
) -> Result<(), AuthorityEvidenceError> {
    ensure_authority_schema_version("evidence", evidence.schema_version)?;
    ensure_authority_schema_version("plan", evidence.reconciliation_plan.schema_version)?;
    ensure_authority_schema_version("report", evidence.authority_report.schema_version)?;
    ensure_authority_schema_version("receipt", evidence.authority_receipt.schema_version)
}

fn ensure_authority_evidence_required_fields(
    evidence: &AuthorityDryRunEvidenceV1,
) -> Result<(), AuthorityEvidenceError> {
    ensure_required_authority_field("evidence.evidence_id", &evidence.evidence_id)?;
    ensure_required_authority_field("evidence.check_id", &evidence.check_id)?;
    ensure_required_authority_field("evidence.generated_at", &evidence.generated_at)?;
    ensure_required_authority_field("plan.plan_id", &evidence.reconciliation_plan.plan_id)?;
    ensure_required_authority_field(
        "plan.inventory_id",
        &evidence.reconciliation_plan.inventory_id,
    )?;
    ensure_required_authority_field("report.report_id", &evidence.authority_report.report_id)?;
    ensure_required_authority_field(
        "receipt.operation_id",
        &evidence.authority_receipt.operation_id,
    )?;
    ensure_required_authority_field("receipt.started_at", &evidence.authority_receipt.started_at)?;
    ensure_required_optional_authority_field(
        "report.check_id",
        evidence.authority_report.check_id.as_deref(),
    )?;
    ensure_required_optional_authority_field(
        "receipt.check_id",
        evidence.authority_receipt.check_id.as_deref(),
    )
}

fn ensure_authority_evidence_provenance(
    evidence: &AuthorityDryRunEvidenceV1,
) -> Result<(), AuthorityEvidenceError> {
    ensure_evidence_check_id_matches(
        &evidence.check_id,
        "report",
        evidence.authority_report.check_id.as_deref(),
    )?;
    ensure_evidence_check_id_matches(
        &evidence.check_id,
        "receipt",
        evidence.authority_receipt.check_id.as_deref(),
    )?;
    ensure_matching_authority_evidence_field(
        "receipt.reconciliation_plan_id",
        &evidence.reconciliation_plan.plan_id,
        &evidence.authority_receipt.reconciliation_plan_id,
    )?;
    ensure_matching_authority_evidence_field(
        "receipt.authority_report_id",
        &evidence.authority_report.report_id,
        &evidence.authority_receipt.authority_report_id,
    )?;
    ensure_matching_authority_evidence_field(
        "receipt.inventory_id",
        &evidence.reconciliation_plan.inventory_id,
        &evidence.authority_receipt.inventory_id,
    )?;
    ensure_matching_authority_evidence_field(
        "receipt.authority_profile_hash",
        &optional_authority_value(evidence.reconciliation_plan.authority_profile_hash.as_ref()),
        &optional_authority_value(evidence.authority_receipt.authority_profile_hash.as_ref()),
    )
}

fn ensure_authority_receipt_is_completed_dry_run(
    receipt: &AuthorityReceiptV1,
) -> Result<(), AuthorityEvidenceError> {
    if !receipt.attempted_actions.is_empty() {
        return Err(AuthorityEvidenceError::DryRunReceiptAttemptedActions {
            count: receipt.attempted_actions.len(),
        });
    }
    if receipt.operation_status != DeploymentExecutionStatusV1::Complete {
        return Err(AuthorityEvidenceError::DryRunReceiptStatus {
            status: receipt.operation_status,
        });
    }
    if receipt.command_result != DeploymentCommandResultV1::Succeeded {
        return Err(AuthorityEvidenceError::DryRunReceiptCommandResult {
            result: receipt.command_result.clone(),
        });
    }
    Ok(())
}

fn ensure_authority_receipt_matches_evidence(
    evidence: &AuthorityDryRunEvidenceV1,
) -> Result<(), AuthorityEvidenceError> {
    let expected_observations = evidence
        .reconciliation_plan
        .canister_actions
        .iter()
        .map(authority_controller_observation_from_action)
        .collect::<Vec<_>>();
    ensure_matching_authority_evidence_content(
        "receipt.verified_controller_observations",
        &expected_observations,
        &evidence.authority_receipt.verified_controller_observations,
    )?;
    ensure_matching_authority_evidence_content(
        "receipt.hard_failures",
        &evidence.authority_report.hard_failures,
        &evidence.authority_receipt.hard_failures,
    )?;
    ensure_matching_authority_evidence_content(
        "receipt.unresolved_observation_gaps",
        &evidence.authority_report.observation_gaps,
        &evidence.authority_receipt.unresolved_observation_gaps,
    )?;
    ensure_matching_authority_evidence_content(
        "receipt.unresolved_external_actions",
        &evidence.authority_report.external_actions_required,
        &evidence.authority_receipt.unresolved_external_actions,
    )
}
fn ensure_evidence_generated_at_matches_finished_at(
    evidence_generated_at: &str,
    receipt_finished_at: Option<&str>,
) -> Result<(), AuthorityEvidenceError> {
    let Some(receipt_finished_at) = receipt_finished_at else {
        return Err(AuthorityEvidenceError::DryRunReceiptMissingFinishedAt);
    };
    ensure_required_authority_field("receipt.finished_at", receipt_finished_at)?;
    if evidence_generated_at == receipt_finished_at {
        return Ok(());
    }

    Err(AuthorityEvidenceError::EvidenceGeneratedAtMismatch {
        evidence_value: evidence_generated_at.to_string(),
        receipt_value: receipt_finished_at.to_string(),
    })
}

fn ensure_authority_receipt_timestamp_order(
    receipt: &AuthorityReceiptV1,
) -> Result<(), AuthorityEvidenceError> {
    let Some(finished_at) = receipt.finished_at.as_deref() else {
        return Err(AuthorityEvidenceError::DryRunReceiptMissingFinishedAt);
    };
    ensure_timestamp_order(
        "receipt.started_at",
        &receipt.started_at,
        "receipt.finished_at",
        finished_at,
    )
}
fn ensure_evidence_check_id_matches(
    evidence_check_id: &str,
    component: &'static str,
    nested_check_id: Option<&str>,
) -> Result<(), AuthorityEvidenceError> {
    let Some(nested_check_id) = nested_check_id else {
        return Ok(());
    };
    if evidence_check_id == nested_check_id {
        return Ok(());
    }

    Err(AuthorityEvidenceError::EvidenceCheckIdMismatch {
        component,
        evidence_value: evidence_check_id.to_string(),
        nested_value: nested_check_id.to_string(),
    })
}
