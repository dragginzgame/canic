use super::super::*;
use super::digest::*;
use super::error::{
    CriticalExternalFixReportError, ExternalLifecycleCheckError, ExternalLifecycleHandoffError,
    ExternalLifecyclePendingReportError,
};
use std::collections::{BTreeMap, BTreeSet};

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

/// Build a passive handoff packet for external lifecycle operators.
#[must_use]
pub fn external_lifecycle_handoff_from_reports(
    handoff_id: impl Into<String>,
    lifecycle_check: &ExternalLifecycleCheckV1,
    proposal_report: &ExternalUpgradeProposalReportV1,
    pending_report: &ExternalLifecyclePendingReportV1,
) -> ExternalLifecycleHandoffV1 {
    let proposal_by_id = proposal_report
        .proposals
        .iter()
        .map(|proposal| (proposal.proposal_id.as_str(), proposal))
        .collect::<BTreeMap<_, _>>();
    let handoff_actions = pending_report
        .pending_external_actions
        .iter()
        .filter_map(|action| proposal_by_id.get(action.proposal_id.as_str()))
        .map(|proposal| external_lifecycle_handoff_action(proposal))
        .collect::<Vec<_>>();
    let mut handoff = ExternalLifecycleHandoffV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        handoff_id: handoff_id.into(),
        handoff_digest: String::new(),
        lifecycle_check_id: lifecycle_check.check_id.clone(),
        lifecycle_check_digest: lifecycle_check.check_digest.clone(),
        pending_report_id: pending_report.report_id.clone(),
        pending_report_digest: pending_report.report_digest.clone(),
        proposal_report_id: proposal_report.report_id.clone(),
        proposal_report_digest: proposal_report.report_digest.clone(),
        deployment_plan_id: pending_report.deployment_plan_id.clone(),
        deployment_plan_digest: pending_report.deployment_plan_digest.clone(),
        inventory_id: pending_report.inventory_id.clone(),
        status: pending_report.status,
        handoff_actions,
        blocked_subjects: pending_report.blocked_subjects.clone(),
        residual_exposure: pending_report.residual_exposure.clone(),
        operator_summary: external_lifecycle_handoff_summary(pending_report),
    };
    handoff.handoff_digest = external_lifecycle_handoff_digest(&handoff);
    handoff
}

/// Validate archived external lifecycle handoff consistency and digest.
pub fn validate_external_lifecycle_handoff(
    handoff: &ExternalLifecycleHandoffV1,
) -> Result<(), ExternalLifecycleHandoffError> {
    if handoff.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(ExternalLifecycleHandoffError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            actual: handoff.schema_version,
        });
    }
    ensure_external_lifecycle_handoff_field("handoff_id", handoff.handoff_id.as_str())?;
    ensure_external_lifecycle_handoff_field("handoff_digest", handoff.handoff_digest.as_str())?;
    ensure_external_lifecycle_handoff_field(
        "lifecycle_check_id",
        handoff.lifecycle_check_id.as_str(),
    )?;
    ensure_external_lifecycle_handoff_field(
        "lifecycle_check_digest",
        handoff.lifecycle_check_digest.as_str(),
    )?;
    ensure_external_lifecycle_handoff_field(
        "pending_report_id",
        handoff.pending_report_id.as_str(),
    )?;
    ensure_external_lifecycle_handoff_field(
        "pending_report_digest",
        handoff.pending_report_digest.as_str(),
    )?;
    ensure_external_lifecycle_handoff_field(
        "proposal_report_id",
        handoff.proposal_report_id.as_str(),
    )?;
    ensure_external_lifecycle_handoff_field(
        "proposal_report_digest",
        handoff.proposal_report_digest.as_str(),
    )?;
    ensure_external_lifecycle_handoff_field(
        "deployment_plan_id",
        handoff.deployment_plan_id.as_str(),
    )?;
    ensure_external_lifecycle_handoff_field(
        "deployment_plan_digest",
        handoff.deployment_plan_digest.as_str(),
    )?;
    ensure_external_lifecycle_handoff_field("inventory_id", handoff.inventory_id.as_str())?;
    ensure_external_lifecycle_handoff_field("operator_summary", handoff.operator_summary.as_str())?;
    let mut subjects = BTreeSet::new();
    for action in &handoff.handoff_actions {
        ensure_external_lifecycle_handoff_field("handoff_action.subject", action.subject.as_str())?;
        ensure_external_lifecycle_handoff_field(
            "handoff_action.proposal_id",
            action.proposal_id.as_str(),
        )?;
        ensure_external_lifecycle_handoff_field(
            "handoff_action.proposal_digest",
            action.proposal_digest.as_str(),
        )?;
        ensure_external_lifecycle_handoff_field(
            "handoff_action.required_external_action",
            action.required_external_action.as_str(),
        )?;
        if !subjects.insert(action.subject.clone()) {
            return Err(ExternalLifecycleHandoffError::DuplicateSubject {
                subject: action.subject.clone(),
            });
        }
    }
    if handoff.handoff_digest != external_lifecycle_handoff_digest(handoff) {
        return Err(ExternalLifecycleHandoffError::DigestMismatch {
            field: "handoff_digest",
        });
    }
    Ok(())
}

/// Validate that an archived handoff still matches the check/proposal/pending
/// evidence it claims to package.
pub fn validate_external_lifecycle_handoff_for_reports(
    handoff: &ExternalLifecycleHandoffV1,
    lifecycle_check: &ExternalLifecycleCheckV1,
    proposal_report: &ExternalUpgradeProposalReportV1,
    pending_report: &ExternalLifecyclePendingReportV1,
) -> Result<(), ExternalLifecycleHandoffError> {
    validate_external_lifecycle_handoff(handoff)?;
    if handoff.lifecycle_check_id != lifecycle_check.check_id {
        return Err(ExternalLifecycleHandoffError::SourceMismatch {
            field: "lifecycle_check_id",
        });
    }
    if handoff.lifecycle_check_digest != lifecycle_check.check_digest {
        return Err(ExternalLifecycleHandoffError::SourceMismatch {
            field: "lifecycle_check_digest",
        });
    }
    if handoff.pending_report_id != pending_report.report_id {
        return Err(ExternalLifecycleHandoffError::SourceMismatch {
            field: "pending_report_id",
        });
    }
    if handoff.pending_report_digest != pending_report.report_digest {
        return Err(ExternalLifecycleHandoffError::SourceMismatch {
            field: "pending_report_digest",
        });
    }
    if handoff.proposal_report_id != proposal_report.report_id {
        return Err(ExternalLifecycleHandoffError::SourceMismatch {
            field: "proposal_report_id",
        });
    }
    if handoff.proposal_report_digest != proposal_report.report_digest {
        return Err(ExternalLifecycleHandoffError::SourceMismatch {
            field: "proposal_report_digest",
        });
    }
    let expected = external_lifecycle_handoff_from_reports(
        handoff.handoff_id.clone(),
        lifecycle_check,
        proposal_report,
        pending_report,
    );
    if handoff != &expected {
        return Err(ExternalLifecycleHandoffError::SourceMismatch {
            field: "pending_report",
        });
    }
    Ok(())
}

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

fn external_lifecycle_handoff_action(
    proposal: &ExternalUpgradeProposalV1,
) -> ExternalLifecycleHandoffActionV1 {
    let primary_requirement = proposal.consent_requirements.first();
    ExternalLifecycleHandoffActionV1 {
        subject: proposal.subject.clone(),
        proposal_id: proposal.proposal_id.clone(),
        proposal_digest: proposal.proposal_digest.clone(),
        canister_id: proposal.canister_id.clone(),
        role: proposal.role.clone(),
        control_class: proposal.control_class,
        lifecycle_mode: proposal.lifecycle_mode,
        required_external_action: proposal.required_external_action.clone(),
        consent_channel_kind: primary_requirement
            .map_or(ConsentChannelKindV1::OutOfBand, |requirement| {
                requirement.consent_channel_kind
            }),
        consent_subject_kind: primary_requirement.map_or(
            ConsentSubjectKindV1::UnknownExternalController,
            |requirement| requirement.consent_subject_kind,
        ),
        required_principals: primary_requirement.map_or_else(Vec::new, |requirement| {
            requirement.required_principals.clone()
        }),
        current_module_hash: proposal.current_module_hash.clone(),
        target_installed_module_hash: proposal.target_installed_module_hash.clone(),
        target_canonical_embedded_config_sha256: proposal
            .target_canonical_embedded_config_sha256
            .clone(),
        verification_requirements: proposal.verification_requirements.clone(),
        operator_instructions: external_lifecycle_handoff_instructions(proposal),
    }
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

fn external_lifecycle_handoff_summary(report: &ExternalLifecyclePendingReportV1) -> String {
    match report.status {
        ExternalLifecyclePlanStatusV1::Ready => {
            "no external lifecycle handoff is required".to_string()
        }
        ExternalLifecyclePlanStatusV1::PendingExternalAction => format!(
            "{} external lifecycle handoff action(s) require operator coordination",
            report.pending_external_count
        ),
        ExternalLifecyclePlanStatusV1::Blocked => format!(
            "external lifecycle handoff is blocked by {} subject(s)",
            report.blocked_count
        ),
    }
}

fn external_lifecycle_handoff_instructions(proposal: &ExternalUpgradeProposalV1) -> Vec<String> {
    let mut instructions = vec![
        format!(
            "present proposal {} for subject {}",
            proposal.proposal_id, proposal.subject
        ),
        "verify live inventory after any reported external action".to_string(),
    ];
    if let Some(expires_at) = proposal.expires_at.as_deref() {
        instructions.push(format!("do not use this proposal after {expires_at}"));
    }
    match proposal.lifecycle_mode {
        LifecycleModeV1::ProposalRequired => {
            instructions.push("collect explicit consent before direct install".to_string());
        }
        LifecycleModeV1::DelegatedInstallRequired => {
            instructions.push("use delegated install authority only if policy allows".to_string());
        }
        LifecycleModeV1::ExternalCompletionOnly | LifecycleModeV1::VerifyOnly => {
            instructions
                .push("wait for external completion evidence before verification".to_string());
        }
        LifecycleModeV1::MustNotTouch | LifecycleModeV1::UnknownUnsafeBlocked => {
            instructions.push("do not execute; report blocked lifecycle state".to_string());
        }
        LifecycleModeV1::DirectDeploymentAuthority => {
            instructions.push("no external handoff should be required".to_string());
        }
    }
    instructions
}

fn ensure_external_pending_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalLifecyclePendingReportError> {
    if value.trim().is_empty() {
        return Err(ExternalLifecyclePendingReportError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_external_lifecycle_check_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalLifecycleCheckError> {
    if value.trim().is_empty() {
        return Err(ExternalLifecycleCheckError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_external_lifecycle_handoff_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalLifecycleHandoffError> {
    if value.trim().is_empty() {
        return Err(ExternalLifecycleHandoffError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_critical_fix_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), CriticalExternalFixReportError> {
    if value.trim().is_empty() {
        return Err(CriticalExternalFixReportError::MissingRequiredField { field });
    }
    Ok(())
}
