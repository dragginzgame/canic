use super::super::super::*;
use super::super::{append_hard_failure_items, safety_status_label};
use super::shared::{
    append_authority_action_summary, append_blockers, append_next_actions,
    append_observation_gap_items,
};

/// Render an authority report as read-only operator text.
#[must_use]
pub fn authority_report_text(report: &AuthorityReportV1) -> String {
    let mut lines = vec![
        "Authority reconciliation report".to_string(),
        "mode: dry_run".to_string(),
        format!("status: {}", safety_status_label(report.status)),
        format!("summary: {}", report.summary),
        format!("report_id: {}", report.report_id),
        format!(
            "check_id: {}",
            report.check_id.as_deref().unwrap_or("not recorded")
        ),
        format!("plan_id: {}", report.reconciliation_plan_id),
        format!("inventory_id: {}", report.inventory_id),
        format!(
            "authority_profile_hash: {}",
            report
                .authority_profile_hash
                .as_deref()
                .unwrap_or("not recorded")
        ),
        String::new(),
        "counts:".to_string(),
        format!("  already_correct: {}", report.counts.already_correct),
        format!(
            "  can_apply_automatically: {}",
            report.counts.can_apply_automatically
        ),
        format!(
            "  requires_external_action: {}",
            report.counts.requires_external_action
        ),
        format!("  unsafe_blocked: {}", report.counts.unsafe_blocked),
        format!("  unknown: {}", report.counts.unknown),
        format!("  hard_failures: {}", report.counts.hard_failures),
        String::new(),
        "apply_readiness:".to_string(),
        format!(
            "  can_apply_automatically: {}",
            report.apply_readiness.can_apply_automatically
        ),
        format!(
            "  automatic_action_count: {}",
            report.apply_readiness.automatic_action_count
        ),
    ];

    append_blockers(&mut lines, report);
    append_next_actions(&mut lines, report);
    append_hard_failure_items(&mut lines, "hard_failures", &report.hard_failures);
    append_observation_gap_items(&mut lines, "observation_gaps", &report.observation_gaps);
    append_authority_action_summary(&mut lines, report);
    lines.join("\n")
}
