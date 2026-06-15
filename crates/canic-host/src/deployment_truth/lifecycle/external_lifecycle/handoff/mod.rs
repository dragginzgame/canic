use super::super::super::*;
use super::super::digest::*;
use super::super::error::ExternalLifecycleHandoffError;
use super::validation::ensure_external_lifecycle_handoff_field;
use std::collections::{BTreeMap, BTreeSet};

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
