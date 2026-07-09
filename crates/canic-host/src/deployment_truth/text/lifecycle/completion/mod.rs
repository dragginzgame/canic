use super::super::super::*;
use super::super::{append_string_items, optional_text};

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
        format!("consent_state: {}", report.consent_state.label()),
        format!(
            "verification_result: {}",
            report.verification_result.label()
        ),
        format!(
            "verification_observation_source: {}",
            report.verification_observation_source.label()
        ),
        format!("completion_status: {}", report.completion_status.label()),
        format!("summary: {}", report.status_summary),
    ];
    append_string_items(&mut lines, "blockers", &report.blockers);
    append_string_items(&mut lines, "next_actions", &report.next_actions);
    lines.join("\n")
}
