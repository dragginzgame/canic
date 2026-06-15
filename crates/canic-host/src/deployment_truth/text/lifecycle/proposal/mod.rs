use super::super::super::*;
use super::super::append_string_items;
use super::shared::append_external_upgrade_proposal_items;

/// Render an external-upgrade proposal report as passive operator text.
#[must_use]
pub fn external_upgrade_proposal_report_text(report: &ExternalUpgradeProposalReportV1) -> String {
    let mut lines = vec![
        "External upgrade proposal report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("report_id: {}", report.report_id),
        format!("report_digest: {}", report.report_digest),
        format!("lifecycle_plan_id: {}", report.lifecycle_plan_id),
        format!("lifecycle_plan_digest: {}", report.lifecycle_plan_digest),
        format!("deployment_plan_id: {}", report.deployment_plan_id),
        format!("deployment_plan_digest: {}", report.deployment_plan_digest),
        format!("inventory_id: {}", report.inventory_id),
        String::new(),
        "counts:".to_string(),
        format!("  proposals: {}", report.proposals.len()),
        format!("  blocked_subjects: {}", report.blocked_subjects.len()),
    ];

    append_external_upgrade_proposal_items(&mut lines, &report.proposals);
    append_string_items(&mut lines, "blocked_subjects", &report.blocked_subjects);
    lines.join("\n")
}
