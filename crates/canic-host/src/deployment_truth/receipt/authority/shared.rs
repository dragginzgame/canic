use super::super::super::*;
use super::error::AuthorityEvidenceError;

pub(super) const fn ensure_authority_schema_version(
    component: &'static str,
    found: u32,
) -> Result<(), AuthorityEvidenceError> {
    if found == DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Ok(());
    }

    Err(AuthorityEvidenceError::SchemaVersionMismatch {
        component,
        expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        found,
    })
}

pub(super) fn ensure_required_authority_field(
    field: &'static str,
    value: &str,
) -> Result<(), AuthorityEvidenceError> {
    if !value.trim().is_empty() {
        return Ok(());
    }

    Err(AuthorityEvidenceError::MissingRequiredField { field })
}

pub(super) fn ensure_required_optional_authority_field(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), AuthorityEvidenceError> {
    let Some(value) = value else {
        return Err(AuthorityEvidenceError::MissingRequiredField { field });
    };
    ensure_required_authority_field(field, value)
}
pub(super) fn ensure_timestamp_order(
    field: &'static str,
    left: &str,
    other_field: &'static str,
    right: &str,
) -> Result<(), AuthorityEvidenceError> {
    if left <= right {
        return Ok(());
    }

    Err(AuthorityEvidenceError::DryRunReceiptTimestampOrder {
        field,
        left: left.to_string(),
        other_field,
        right: right.to_string(),
    })
}
pub(super) fn ensure_authority_report_matches_plan(
    plan: &AuthorityReconciliationPlanV1,
    report: &AuthorityReportV1,
) -> Result<(), AuthorityEvidenceError> {
    ensure_matching_authority_evidence_field(
        "reconciliation_plan_id",
        &plan.plan_id,
        &report.reconciliation_plan_id,
    )?;
    ensure_matching_authority_evidence_field(
        "inventory_id",
        &plan.inventory_id,
        &report.inventory_id,
    )?;
    ensure_matching_authority_evidence_field(
        "authority_profile_hash",
        &optional_authority_value(plan.authority_profile_hash.as_ref()),
        &optional_authority_value(report.authority_profile_hash.as_ref()),
    )?;
    ensure_matching_authority_evidence_content(
        "automatic_actions",
        &plan.automatic_actions,
        &report.automatic_actions,
    )?;
    ensure_matching_authority_evidence_content(
        "hard_failures",
        &plan.hard_failures,
        &report.hard_failures,
    )?;
    ensure_matching_authority_evidence_content(
        "external_actions_required",
        &plan.external_actions_required,
        &report.external_actions_required,
    )?;
    ensure_authority_report_is_derived_from_plan(plan, report)
}

fn ensure_authority_report_is_derived_from_plan(
    plan: &AuthorityReconciliationPlanV1,
    report: &AuthorityReportV1,
) -> Result<(), AuthorityEvidenceError> {
    let expected_report = authority_report_from_plan_with_check_id(
        report.report_id.clone(),
        report.check_id.clone(),
        plan,
    );
    ensure_matching_authority_evidence_content(
        "report.status",
        &expected_report.status,
        &report.status,
    )?;
    ensure_matching_authority_evidence_content(
        "report.summary",
        &expected_report.summary,
        &report.summary,
    )?;
    ensure_matching_authority_evidence_content(
        "report.counts",
        &expected_report.counts,
        &report.counts,
    )?;
    ensure_matching_authority_evidence_content(
        "report.apply_readiness",
        &expected_report.apply_readiness,
        &report.apply_readiness,
    )?;
    ensure_matching_authority_evidence_content(
        "report.action_counts",
        &expected_report.action_counts,
        &report.action_counts,
    )?;
    ensure_matching_authority_evidence_content(
        "report.control_class_counts",
        &expected_report.control_class_counts,
        &report.control_class_counts,
    )?;
    ensure_matching_authority_evidence_content(
        "report.observation_gaps",
        &expected_report.observation_gaps,
        &report.observation_gaps,
    )?;
    ensure_matching_authority_evidence_content(
        "report.next_actions",
        &expected_report.next_actions,
        &report.next_actions,
    )
}

pub(super) fn ensure_matching_authority_evidence_field(
    field: &'static str,
    plan_value: &str,
    report_value: &str,
) -> Result<(), AuthorityEvidenceError> {
    if plan_value == report_value {
        return Ok(());
    }

    Err(AuthorityEvidenceError::PlanReportMismatch {
        field,
        plan_value: plan_value.to_string(),
        report_value: report_value.to_string(),
    })
}
pub(super) fn optional_authority_value(value: Option<&String>) -> String {
    value.map_or_else(|| "<none>".to_string(), ToString::to_string)
}

pub(super) fn ensure_matching_authority_evidence_content<T: Eq>(
    field: &'static str,
    plan_value: &T,
    report_value: &T,
) -> Result<(), AuthorityEvidenceError> {
    if plan_value == report_value {
        return Ok(());
    }

    Err(AuthorityEvidenceError::PlanReportContentMismatch { field })
}
