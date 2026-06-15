use super::super::super::*;
use super::super::digest::*;
use super::super::error::ExternalLifecyclePendingReportError;
use super::validation::ensure_external_pending_report_field;
use std::collections::BTreeSet;

/// Build a passive summary of external lifecycle work still pending after a
/// plan/proposal pass.
#[must_use]
pub fn external_lifecycle_pending_report_from_plan(
    report_id: impl Into<String>,
    lifecycle_plan: &ExternalLifecyclePlanV1,
    proposal_report: &ExternalUpgradeProposalReportV1,
) -> ExternalLifecyclePendingReportV1 {
    let report_id = report_id.into();
    let pending_external_actions = proposal_report
        .proposals
        .iter()
        .map(external_lifecycle_pending_action)
        .collect::<Vec<_>>();
    let blocked_subjects = lifecycle_plan
        .blocked_role_upgrades
        .iter()
        .map(|upgrade| upgrade.subject.clone())
        .collect::<Vec<_>>();
    let mut report = ExternalLifecyclePendingReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id,
        report_digest: String::new(),
        lifecycle_plan_id: lifecycle_plan.lifecycle_plan_id.clone(),
        lifecycle_plan_digest: lifecycle_plan.lifecycle_plan_digest.clone(),
        proposal_report_id: proposal_report.report_id.clone(),
        proposal_report_digest: proposal_report.report_digest.clone(),
        deployment_plan_id: lifecycle_plan.deployment_plan_id.clone(),
        deployment_plan_digest: lifecycle_plan.deployment_plan_digest.clone(),
        inventory_id: lifecycle_plan.inventory_id.clone(),
        direct_upgrade_count: lifecycle_plan.directly_executable_role_upgrades.len(),
        pending_external_count: pending_external_actions.len(),
        blocked_count: blocked_subjects.len(),
        pending_external_actions,
        blocked_subjects,
        residual_exposure: lifecycle_plan.residual_exposure.clone(),
        status: lifecycle_plan.status,
    };
    report.report_digest = external_lifecycle_pending_report_digest(&report);
    report
}

/// Validate archived external lifecycle pending report consistency and digest.
pub fn validate_external_lifecycle_pending_report(
    report: &ExternalLifecyclePendingReportV1,
) -> Result<(), ExternalLifecyclePendingReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(ExternalLifecyclePendingReportError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            actual: report.schema_version,
        });
    }
    ensure_external_pending_report_field("report_id", report.report_id.as_str())?;
    ensure_external_pending_report_field("report_digest", report.report_digest.as_str())?;
    ensure_external_pending_report_field("lifecycle_plan_id", report.lifecycle_plan_id.as_str())?;
    ensure_external_pending_report_field(
        "lifecycle_plan_digest",
        report.lifecycle_plan_digest.as_str(),
    )?;
    ensure_external_pending_report_field("proposal_report_id", report.proposal_report_id.as_str())?;
    ensure_external_pending_report_field(
        "proposal_report_digest",
        report.proposal_report_digest.as_str(),
    )?;
    ensure_external_pending_report_field("deployment_plan_id", report.deployment_plan_id.as_str())?;
    ensure_external_pending_report_field(
        "deployment_plan_digest",
        report.deployment_plan_digest.as_str(),
    )?;
    ensure_external_pending_report_field("inventory_id", report.inventory_id.as_str())?;
    if report.pending_external_count != report.pending_external_actions.len()
        || report.blocked_count != report.blocked_subjects.len()
    {
        return Err(ExternalLifecyclePendingReportError::CountMismatch);
    }
    let mut subjects = BTreeSet::new();
    for action in &report.pending_external_actions {
        ensure_external_pending_report_field("pending_action.subject", action.subject.as_str())?;
        ensure_external_pending_report_field(
            "pending_action.proposal_id",
            action.proposal_id.as_str(),
        )?;
        ensure_external_pending_report_field(
            "pending_action.proposal_digest",
            action.proposal_digest.as_str(),
        )?;
        ensure_external_pending_report_field(
            "pending_action.required_external_action",
            action.required_external_action.as_str(),
        )?;
        if !subjects.insert(action.subject.clone()) {
            return Err(ExternalLifecyclePendingReportError::DuplicateSubject {
                subject: action.subject.clone(),
            });
        }
    }
    if report.report_digest != external_lifecycle_pending_report_digest(report) {
        return Err(ExternalLifecyclePendingReportError::DigestMismatch {
            field: "report_digest",
        });
    }
    Ok(())
}

/// Validate that an archived external lifecycle pending report still matches
/// the lifecycle and proposal artifacts it claims to derive from.
pub fn validate_external_lifecycle_pending_report_for_plan(
    report: &ExternalLifecyclePendingReportV1,
    lifecycle_plan: &ExternalLifecyclePlanV1,
    proposal_report: &ExternalUpgradeProposalReportV1,
) -> Result<(), ExternalLifecyclePendingReportError> {
    validate_external_lifecycle_pending_report(report)?;
    if report.lifecycle_plan_id != lifecycle_plan.lifecycle_plan_id {
        return Err(ExternalLifecyclePendingReportError::SourceMismatch {
            field: "lifecycle_plan_id",
        });
    }
    if report.lifecycle_plan_digest != lifecycle_plan.lifecycle_plan_digest {
        return Err(ExternalLifecyclePendingReportError::SourceMismatch {
            field: "lifecycle_plan_digest",
        });
    }
    if report.proposal_report_id != proposal_report.report_id {
        return Err(ExternalLifecyclePendingReportError::SourceMismatch {
            field: "proposal_report_id",
        });
    }
    if report.proposal_report_digest != proposal_report.report_digest {
        return Err(ExternalLifecyclePendingReportError::SourceMismatch {
            field: "proposal_report_digest",
        });
    }
    let expected = external_lifecycle_pending_report_from_plan(
        report.report_id.clone(),
        lifecycle_plan,
        proposal_report,
    );
    if report != &expected {
        return Err(ExternalLifecyclePendingReportError::SourceMismatch {
            field: "lifecycle_plan",
        });
    }
    Ok(())
}

fn external_lifecycle_pending_action(
    proposal: &ExternalUpgradeProposalV1,
) -> ExternalLifecyclePendingActionV1 {
    ExternalLifecyclePendingActionV1 {
        subject: proposal.subject.clone(),
        proposal_id: proposal.proposal_id.clone(),
        proposal_digest: proposal.proposal_digest.clone(),
        canister_id: proposal.canister_id.clone(),
        role: proposal.role.clone(),
        control_class: proposal.control_class,
        lifecycle_mode: proposal.lifecycle_mode,
        required_external_action: proposal.required_external_action.clone(),
        consent_requirements: proposal.consent_requirements.clone(),
        verification_requirements: proposal.verification_requirements.clone(),
    }
}
