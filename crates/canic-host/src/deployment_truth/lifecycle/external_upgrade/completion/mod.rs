use super::super::super::*;
use super::super::digest::*;
use super::super::error::ExternalUpgradeCompletionReportError;
use super::consent::validate_external_upgrade_consent_evidence;
use super::proposal::validate_external_upgrade_proposal;
use super::validation::{
    ensure_completion_sources_match_proposal, ensure_external_completion_report_field,
};
use super::verification::validate_external_upgrade_verification_check;

/// Build a passive completion report for an external lifecycle proposal.
///
/// This report only combines structural evidence. It does not deliver consent,
/// execute upgrades, query live inventory, or mutate deployment state.
pub fn external_upgrade_completion_report_from_evidence(
    report_id: impl Into<String>,
    proposal: &ExternalUpgradeProposalV1,
    consent_evidence: &ExternalUpgradeConsentEvidenceV1,
    verification_check: &ExternalUpgradeVerificationCheckV1,
) -> Result<ExternalUpgradeCompletionReportV1, ExternalUpgradeCompletionReportError> {
    validate_external_upgrade_proposal(proposal)?;
    validate_external_upgrade_consent_evidence(consent_evidence)?;
    validate_external_upgrade_verification_check(verification_check)?;
    ensure_completion_sources_match_proposal(proposal, consent_evidence, verification_check)?;

    let completion_status = external_upgrade_completion_status(
        consent_evidence.consent_state,
        verification_check.verification_result,
        verification_check.observation.source,
    );
    let blockers = external_upgrade_completion_blockers(completion_status);
    let next_actions = external_upgrade_completion_next_actions(completion_status);
    let mut report = ExternalUpgradeCompletionReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        report_digest: String::new(),
        proposal_id: proposal.proposal_id.clone(),
        proposal_digest: proposal.proposal_digest.clone(),
        consent_evidence_id: consent_evidence.evidence_id.clone(),
        consent_evidence_digest: consent_evidence.evidence_digest.clone(),
        verification_check_id: verification_check.check_id.clone(),
        verification_check_digest: verification_check.check_digest.clone(),
        subject: proposal.subject.clone(),
        canister_id: proposal.canister_id.clone(),
        role: proposal.role.clone(),
        consent_state: consent_evidence.consent_state,
        verification_result: verification_check.verification_result,
        verification_observation_source: verification_check.observation.source,
        completion_status,
        blockers,
        next_actions,
        status_summary: external_upgrade_completion_summary(completion_status).to_string(),
    };
    report.report_digest = external_upgrade_completion_report_digest(&report);
    Ok(report)
}

/// Validate archived completion report consistency and digest.
pub fn validate_external_upgrade_completion_report(
    report: &ExternalUpgradeCompletionReportV1,
) -> Result<(), ExternalUpgradeCompletionReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ExternalUpgradeCompletionReportError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                actual: report.schema_version,
            },
        );
    }
    ensure_external_completion_report_field("report_id", report.report_id.as_str())?;
    ensure_external_completion_report_field("report_digest", report.report_digest.as_str())?;
    ensure_external_completion_report_field("proposal_id", report.proposal_id.as_str())?;
    ensure_external_completion_report_field("proposal_digest", report.proposal_digest.as_str())?;
    ensure_external_completion_report_field(
        "consent_evidence_id",
        report.consent_evidence_id.as_str(),
    )?;
    ensure_external_completion_report_field(
        "consent_evidence_digest",
        report.consent_evidence_digest.as_str(),
    )?;
    ensure_external_completion_report_field(
        "verification_check_id",
        report.verification_check_id.as_str(),
    )?;
    ensure_external_completion_report_field(
        "verification_check_digest",
        report.verification_check_digest.as_str(),
    )?;
    ensure_external_completion_report_field("subject", report.subject.as_str())?;
    ensure_external_completion_report_field("status_summary", report.status_summary.as_str())?;
    if report.completion_status
        != external_upgrade_completion_status(
            report.consent_state,
            report.verification_result,
            report.verification_observation_source,
        )
    {
        return Err(ExternalUpgradeCompletionReportError::SourceMismatch {
            field: "completion_status",
        });
    }
    if report.status_summary != external_upgrade_completion_summary(report.completion_status) {
        return Err(ExternalUpgradeCompletionReportError::SourceMismatch {
            field: "status_summary",
        });
    }
    if report.blockers != external_upgrade_completion_blockers(report.completion_status) {
        return Err(ExternalUpgradeCompletionReportError::SourceMismatch { field: "blockers" });
    }
    if report.next_actions != external_upgrade_completion_next_actions(report.completion_status) {
        return Err(ExternalUpgradeCompletionReportError::SourceMismatch {
            field: "next_actions",
        });
    }
    if report.report_digest != external_upgrade_completion_report_digest(report) {
        return Err(ExternalUpgradeCompletionReportError::DigestMismatch {
            field: "report_digest",
        });
    }
    Ok(())
}

/// Validate that an archived completion report still matches its source
/// proposal, consent evidence, and verification check.
pub fn validate_external_upgrade_completion_report_for_evidence(
    report: &ExternalUpgradeCompletionReportV1,
    proposal: &ExternalUpgradeProposalV1,
    consent_evidence: &ExternalUpgradeConsentEvidenceV1,
    verification_check: &ExternalUpgradeVerificationCheckV1,
) -> Result<(), ExternalUpgradeCompletionReportError> {
    validate_external_upgrade_completion_report(report)?;
    let expected = external_upgrade_completion_report_from_evidence(
        report.report_id.clone(),
        proposal,
        consent_evidence,
        verification_check,
    )?;
    if report != &expected {
        return Err(ExternalUpgradeCompletionReportError::SourceMismatch {
            field: "source_evidence",
        });
    }
    Ok(())
}

const fn external_upgrade_completion_status(
    consent_state: ExternalUpgradeConsentStateV1,
    verification_result: ExternalUpgradeVerificationResultV1,
    source: ExternalVerificationObservationSourceV1,
) -> ExternalUpgradeCompletionStatusV1 {
    match consent_state {
        ExternalUpgradeConsentStateV1::Pending => {
            ExternalUpgradeCompletionStatusV1::AwaitingConsent
        }
        ExternalUpgradeConsentStateV1::Refused => ExternalUpgradeCompletionStatusV1::ConsentRefused,
        ExternalUpgradeConsentStateV1::Delegated
        | ExternalUpgradeConsentStateV1::ExecutedExternally => match verification_result {
            ExternalUpgradeVerificationResultV1::Verified => match source {
                ExternalVerificationObservationSourceV1::DeploymentTruthInventory => {
                    ExternalUpgradeCompletionStatusV1::VerifiedComplete
                }
                ExternalVerificationObservationSourceV1::SuppliedObservation => {
                    ExternalUpgradeCompletionStatusV1::SuppliedEvidenceConsistent
                }
            },
            ExternalUpgradeVerificationResultV1::Mismatch => {
                ExternalUpgradeCompletionStatusV1::VerificationFailed
            }
            ExternalUpgradeVerificationResultV1::Pending
            | ExternalUpgradeVerificationResultV1::Refused => {
                ExternalUpgradeCompletionStatusV1::AwaitingVerification
            }
        },
    }
}

fn external_upgrade_completion_blockers(status: ExternalUpgradeCompletionStatusV1) -> Vec<String> {
    match status {
        ExternalUpgradeCompletionStatusV1::AwaitingConsent => {
            vec!["external consent or action has not been reported".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::ConsentRefused => {
            vec!["external consent was refused".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::SuppliedEvidenceConsistent => {
            vec!["supplied evidence is consistent but not live inventory proof".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::AwaitingVerification => {
            vec!["external action requires verification against live inventory".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::VerificationFailed => {
            vec!["supplied observation does not satisfy verification policy".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::VerifiedComplete => Vec::new(),
    }
}

fn external_upgrade_completion_next_actions(
    status: ExternalUpgradeCompletionStatusV1,
) -> Vec<String> {
    match status {
        ExternalUpgradeCompletionStatusV1::AwaitingConsent => {
            vec!["obtain external consent or reported external execution".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::ConsentRefused => {
            vec!["do not execute; supersede the proposal before retry".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::SuppliedEvidenceConsistent => {
            vec!["collect deployment-truth inventory and run verification check".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::AwaitingVerification => {
            vec!["collect fresh inventory observations and run verification check".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::VerificationFailed => {
            vec!["resolve observed module/config/readiness mismatch".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::VerifiedComplete => {
            vec!["record external lifecycle item as verified complete".to_string()]
        }
    }
}

const fn external_upgrade_completion_summary(
    status: ExternalUpgradeCompletionStatusV1,
) -> &'static str {
    match status {
        ExternalUpgradeCompletionStatusV1::AwaitingConsent => {
            "external lifecycle item is waiting for consent or external action"
        }
        ExternalUpgradeCompletionStatusV1::ConsentRefused => "external lifecycle item was refused",
        ExternalUpgradeCompletionStatusV1::SuppliedEvidenceConsistent => {
            "external lifecycle supplied evidence is consistent but awaits inventory verification"
        }
        ExternalUpgradeCompletionStatusV1::AwaitingVerification => {
            "external lifecycle item needs verification before completion"
        }
        ExternalUpgradeCompletionStatusV1::VerifiedComplete => {
            "external lifecycle item is structurally verified complete"
        }
        ExternalUpgradeCompletionStatusV1::VerificationFailed => {
            "external lifecycle item failed supplied verification"
        }
    }
}
