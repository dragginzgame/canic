use super::super::super::*;
use super::super::{append_string_items, optional_text};
use super::shared::{
    external_upgrade_consent_state_label, external_upgrade_verification_result_label,
    external_verification_observation_source_label,
};

/// Render an external-upgrade completion report as passive operator text.
#[must_use]
pub fn external_upgrade_completion_report_text(
    report: &ExternalUpgradeCompletionReportV1,
) -> String {
    let mut lines = vec![
        "External upgrade completion report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        "live_lookup: none".to_string(),
        format!("report_id: {}", report.report_id),
        format!("report_digest: {}", report.report_digest),
        format!("proposal_id: {}", report.proposal_id),
        format!("proposal_digest: {}", report.proposal_digest),
        format!("consent_evidence_id: {}", report.consent_evidence_id),
        format!("verification_check_id: {}", report.verification_check_id),
        format!("subject: {}", report.subject),
        format!("role: {}", optional_text(report.role.as_deref())),
        format!(
            "canister_id: {}",
            optional_text(report.canister_id.as_deref())
        ),
        format!(
            "consent_state: {}",
            external_upgrade_consent_state_label(report.consent_state)
        ),
        format!(
            "verification_result: {}",
            external_upgrade_verification_result_label(report.verification_result)
        ),
        format!(
            "verification_observation_source: {}",
            external_verification_observation_source_label(report.verification_observation_source)
        ),
        format!("completion_status: {}", report.completion_status.label()),
        format!("summary: {}", report.status_summary),
    ];
    append_string_items(&mut lines, "blockers", &report.blockers);
    append_string_items(&mut lines, "next_actions", &report.next_actions);
    lines.join("\n")
}
