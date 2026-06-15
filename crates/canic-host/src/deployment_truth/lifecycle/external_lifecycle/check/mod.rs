use super::super::super::*;
use super::super::digest::*;
use super::super::error::ExternalLifecycleCheckError;
use super::validation::ensure_external_lifecycle_check_field;

/// Build a passive operator check over external lifecycle work.
#[must_use]
pub fn external_lifecycle_check_from_reports(
    check_id: impl Into<String>,
    lifecycle_plan: &ExternalLifecyclePlanV1,
    proposal_report: &ExternalUpgradeProposalReportV1,
    pending_report: &ExternalLifecyclePendingReportV1,
) -> ExternalLifecycleCheckV1 {
    let check_id = check_id.into();
    let status = pending_report.status;
    let mut check = ExternalLifecycleCheckV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        check_id,
        check_digest: String::new(),
        lifecycle_plan_id: lifecycle_plan.lifecycle_plan_id.clone(),
        lifecycle_plan_digest: lifecycle_plan.lifecycle_plan_digest.clone(),
        proposal_report_id: proposal_report.report_id.clone(),
        proposal_report_digest: proposal_report.report_digest.clone(),
        pending_report_id: pending_report.report_id.clone(),
        pending_report_digest: pending_report.report_digest.clone(),
        deployment_plan_id: lifecycle_plan.deployment_plan_id.clone(),
        deployment_plan_digest: lifecycle_plan.deployment_plan_digest.clone(),
        inventory_id: lifecycle_plan.inventory_id.clone(),
        status,
        direct_upgrade_count: pending_report.direct_upgrade_count,
        pending_external_count: pending_report.pending_external_count,
        blocked_count: pending_report.blocked_count,
        residual_exposure_count: pending_report.residual_exposure.len(),
        summary: external_lifecycle_check_summary(status, pending_report),
        next_actions: external_lifecycle_check_next_actions(status, pending_report),
    };
    check.check_digest = external_lifecycle_check_digest(&check);
    check
}

/// Validate archived external lifecycle check consistency and digest.
pub fn validate_external_lifecycle_check(
    check: &ExternalLifecycleCheckV1,
) -> Result<(), ExternalLifecycleCheckError> {
    if check.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(ExternalLifecycleCheckError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            actual: check.schema_version,
        });
    }
    ensure_external_lifecycle_check_field("check_id", check.check_id.as_str())?;
    ensure_external_lifecycle_check_field("check_digest", check.check_digest.as_str())?;
    ensure_external_lifecycle_check_field("lifecycle_plan_id", check.lifecycle_plan_id.as_str())?;
    ensure_external_lifecycle_check_field(
        "lifecycle_plan_digest",
        check.lifecycle_plan_digest.as_str(),
    )?;
    ensure_external_lifecycle_check_field("proposal_report_id", check.proposal_report_id.as_str())?;
    ensure_external_lifecycle_check_field(
        "proposal_report_digest",
        check.proposal_report_digest.as_str(),
    )?;
    ensure_external_lifecycle_check_field("pending_report_id", check.pending_report_id.as_str())?;
    ensure_external_lifecycle_check_field(
        "pending_report_digest",
        check.pending_report_digest.as_str(),
    )?;
    ensure_external_lifecycle_check_field("deployment_plan_id", check.deployment_plan_id.as_str())?;
    ensure_external_lifecycle_check_field(
        "deployment_plan_digest",
        check.deployment_plan_digest.as_str(),
    )?;
    ensure_external_lifecycle_check_field("inventory_id", check.inventory_id.as_str())?;
    ensure_external_lifecycle_check_field("summary", check.summary.as_str())?;
    if check.check_digest != external_lifecycle_check_digest(check) {
        return Err(ExternalLifecycleCheckError::DigestMismatch {
            field: "check_digest",
        });
    }
    Ok(())
}

/// Validate that an archived external lifecycle check still matches the
/// lifecycle/proposal/pending artifacts it claims to summarize.
pub fn validate_external_lifecycle_check_for_reports(
    check: &ExternalLifecycleCheckV1,
    lifecycle_plan: &ExternalLifecyclePlanV1,
    proposal_report: &ExternalUpgradeProposalReportV1,
    pending_report: &ExternalLifecyclePendingReportV1,
) -> Result<(), ExternalLifecycleCheckError> {
    validate_external_lifecycle_check(check)?;
    if check.lifecycle_plan_id != lifecycle_plan.lifecycle_plan_id {
        return Err(ExternalLifecycleCheckError::SourceMismatch {
            field: "lifecycle_plan_id",
        });
    }
    if check.lifecycle_plan_digest != lifecycle_plan.lifecycle_plan_digest {
        return Err(ExternalLifecycleCheckError::SourceMismatch {
            field: "lifecycle_plan_digest",
        });
    }
    if check.proposal_report_id != proposal_report.report_id {
        return Err(ExternalLifecycleCheckError::SourceMismatch {
            field: "proposal_report_id",
        });
    }
    if check.proposal_report_digest != proposal_report.report_digest {
        return Err(ExternalLifecycleCheckError::SourceMismatch {
            field: "proposal_report_digest",
        });
    }
    if check.pending_report_id != pending_report.report_id {
        return Err(ExternalLifecycleCheckError::SourceMismatch {
            field: "pending_report_id",
        });
    }
    if check.pending_report_digest != pending_report.report_digest {
        return Err(ExternalLifecycleCheckError::SourceMismatch {
            field: "pending_report_digest",
        });
    }
    if check.direct_upgrade_count != pending_report.direct_upgrade_count
        || check.pending_external_count != pending_report.pending_external_count
        || check.blocked_count != pending_report.blocked_count
        || check.residual_exposure_count != pending_report.residual_exposure.len()
    {
        return Err(ExternalLifecycleCheckError::CountMismatch);
    }
    let expected = external_lifecycle_check_from_reports(
        check.check_id.clone(),
        lifecycle_plan,
        proposal_report,
        pending_report,
    );
    if check != &expected {
        return Err(ExternalLifecycleCheckError::SourceMismatch {
            field: "pending_report",
        });
    }
    Ok(())
}

fn external_lifecycle_check_summary(
    status: ExternalLifecyclePlanStatusV1,
    pending_report: &ExternalLifecyclePendingReportV1,
) -> String {
    match status {
        ExternalLifecyclePlanStatusV1::Ready => {
            format!(
                "external lifecycle is ready: {} directly executable role(s), no pending external action",
                pending_report.direct_upgrade_count
            )
        }
        ExternalLifecyclePlanStatusV1::PendingExternalAction => {
            format!(
                "external lifecycle has {} pending external action(s) and {} directly executable role(s)",
                pending_report.pending_external_count, pending_report.direct_upgrade_count
            )
        }
        ExternalLifecyclePlanStatusV1::Blocked => {
            format!(
                "external lifecycle is blocked by {} role/canister subject(s)",
                pending_report.blocked_count
            )
        }
    }
}

fn external_lifecycle_check_next_actions(
    status: ExternalLifecyclePlanStatusV1,
    pending_report: &ExternalLifecyclePendingReportV1,
) -> Vec<String> {
    match status {
        ExternalLifecyclePlanStatusV1::Ready => {
            vec!["continue through the normal guarded deployment path".to_string()]
        }
        ExternalLifecyclePlanStatusV1::PendingExternalAction => pending_report
            .pending_external_actions
            .iter()
            .map(|action| {
                format!(
                    "request {} for {}",
                    action.required_external_action, action.subject
                )
            })
            .collect(),
        ExternalLifecyclePlanStatusV1::Blocked => {
            vec!["resolve blocked external lifecycle subjects before execution".to_string()]
        }
    }
}
