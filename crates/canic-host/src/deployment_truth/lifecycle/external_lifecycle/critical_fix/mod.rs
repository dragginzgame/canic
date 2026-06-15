use super::super::super::*;
use super::super::digest::*;
use super::super::error::CriticalExternalFixReportError;
use super::validation::ensure_critical_fix_report_field;
use std::collections::BTreeSet;

/// Build a passive critical-fix residual exposure report from lifecycle
/// evidence.
#[must_use]
pub fn critical_external_fix_report_from_pending(
    report_id: impl Into<String>,
    fix_id: impl Into<String>,
    severity: impl Into<String>,
    lifecycle_plan: &ExternalLifecyclePlanV1,
    pending_report: &ExternalLifecyclePendingReportV1,
) -> CriticalExternalFixReportV1 {
    let report_id = report_id.into();
    let fix_id = fix_id.into();
    let severity = severity.into();
    let affected_roles = lifecycle_roles(lifecycle_plan);
    let affected_canisters = lifecycle_canisters(lifecycle_plan);
    let directly_patchable_roles = role_names(&lifecycle_plan.directly_executable_role_upgrades);
    let externally_blocked_roles = pending_report
        .pending_external_actions
        .iter()
        .filter_map(|action| action.role.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let dependency_blocked_roles = role_names(&lifecycle_plan.blocked_role_upgrades);
    let required_external_actions = pending_report
        .pending_external_actions
        .iter()
        .map(|action| format!("{}: {}", action.subject, action.required_external_action))
        .collect::<Vec<_>>();
    let operator_next_steps = critical_fix_next_steps(
        pending_report.pending_external_count,
        pending_report.blocked_count,
        lifecycle_plan.protected_call_implications.as_slice(),
    );
    let mut report = CriticalExternalFixReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id,
        report_digest: String::new(),
        fix_id,
        severity,
        lifecycle_plan_id: lifecycle_plan.lifecycle_plan_id.clone(),
        lifecycle_plan_digest: lifecycle_plan.lifecycle_plan_digest.clone(),
        pending_report_id: pending_report.report_id.clone(),
        pending_report_digest: pending_report.report_digest.clone(),
        deployment_plan_id: lifecycle_plan.deployment_plan_id.clone(),
        deployment_plan_digest: lifecycle_plan.deployment_plan_digest.clone(),
        inventory_id: lifecycle_plan.inventory_id.clone(),
        affected_roles,
        affected_canisters,
        directly_patchable_roles,
        externally_blocked_roles,
        dependency_blocked_roles,
        required_external_actions,
        protected_call_implications: lifecycle_plan.protected_call_implications.clone(),
        residual_exposure: pending_report.residual_exposure.clone(),
        operator_next_steps,
    };
    report.report_digest = critical_external_fix_report_digest(&report);
    report
}

/// Validate archived critical external fix report consistency and digest.
pub fn validate_critical_external_fix_report(
    report: &CriticalExternalFixReportV1,
) -> Result<(), CriticalExternalFixReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(CriticalExternalFixReportError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            actual: report.schema_version,
        });
    }
    ensure_critical_fix_report_field("report_id", report.report_id.as_str())?;
    ensure_critical_fix_report_field("report_digest", report.report_digest.as_str())?;
    ensure_critical_fix_report_field("fix_id", report.fix_id.as_str())?;
    ensure_critical_fix_report_field("severity", report.severity.as_str())?;
    ensure_critical_fix_report_field("lifecycle_plan_id", report.lifecycle_plan_id.as_str())?;
    ensure_critical_fix_report_field(
        "lifecycle_plan_digest",
        report.lifecycle_plan_digest.as_str(),
    )?;
    ensure_critical_fix_report_field("pending_report_id", report.pending_report_id.as_str())?;
    ensure_critical_fix_report_field(
        "pending_report_digest",
        report.pending_report_digest.as_str(),
    )?;
    ensure_critical_fix_report_field("deployment_plan_id", report.deployment_plan_id.as_str())?;
    ensure_critical_fix_report_field(
        "deployment_plan_digest",
        report.deployment_plan_digest.as_str(),
    )?;
    ensure_critical_fix_report_field("inventory_id", report.inventory_id.as_str())?;
    if report.report_digest != critical_external_fix_report_digest(report) {
        return Err(CriticalExternalFixReportError::DigestMismatch {
            field: "report_digest",
        });
    }
    Ok(())
}

/// Validate that an archived critical external fix report still matches the
/// lifecycle artifacts it claims to summarize.
pub fn validate_critical_external_fix_report_for_pending(
    report: &CriticalExternalFixReportV1,
    lifecycle_plan: &ExternalLifecyclePlanV1,
    pending_report: &ExternalLifecyclePendingReportV1,
) -> Result<(), CriticalExternalFixReportError> {
    validate_critical_external_fix_report(report)?;
    if report.lifecycle_plan_id != lifecycle_plan.lifecycle_plan_id {
        return Err(CriticalExternalFixReportError::SourceMismatch {
            field: "lifecycle_plan_id",
        });
    }
    if report.lifecycle_plan_digest != lifecycle_plan.lifecycle_plan_digest {
        return Err(CriticalExternalFixReportError::SourceMismatch {
            field: "lifecycle_plan_digest",
        });
    }
    if report.pending_report_id != pending_report.report_id {
        return Err(CriticalExternalFixReportError::SourceMismatch {
            field: "pending_report_id",
        });
    }
    if report.pending_report_digest != pending_report.report_digest {
        return Err(CriticalExternalFixReportError::SourceMismatch {
            field: "pending_report_digest",
        });
    }
    let expected = critical_external_fix_report_from_pending(
        report.report_id.clone(),
        report.fix_id.clone(),
        report.severity.clone(),
        lifecycle_plan,
        pending_report,
    );
    if report != &expected {
        return Err(CriticalExternalFixReportError::SourceMismatch {
            field: "lifecycle_plan",
        });
    }
    Ok(())
}

fn lifecycle_roles(lifecycle_plan: &ExternalLifecyclePlanV1) -> Vec<String> {
    lifecycle_plan
        .lifecycle_authority_rows
        .iter()
        .filter_map(|authority| authority.role.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn lifecycle_canisters(lifecycle_plan: &ExternalLifecyclePlanV1) -> Vec<String> {
    lifecycle_plan
        .lifecycle_authority_rows
        .iter()
        .filter_map(|authority| authority.canister_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn role_names(upgrades: &[ExternalLifecycleRoleUpgradeV1]) -> Vec<String> {
    upgrades
        .iter()
        .filter_map(|upgrade| upgrade.role.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn critical_fix_next_steps(
    pending_external_count: usize,
    blocked_count: usize,
    protected_call_implications: &[String],
) -> Vec<String> {
    let mut steps = Vec::new();
    if pending_external_count > 0 {
        steps.push(
            "request external consent or completion for externally controlled roles".to_string(),
        );
    }
    if blocked_count > 0 {
        steps.push(
            "resolve blocked lifecycle rows before reporting the deployment fully patched"
                .to_string(),
        );
    }
    if !protected_call_implications.is_empty() {
        steps.push(
            "review protected-call readiness and role epoch implications before closure"
                .to_string(),
        );
    }
    if steps.is_empty() {
        steps.push("no external lifecycle work remains for this critical fix".to_string());
    }
    steps
}
