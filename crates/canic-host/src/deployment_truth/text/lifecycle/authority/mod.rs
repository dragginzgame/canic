use super::super::super::*;
use super::shared::append_lifecycle_authority_items;

/// Render lifecycle authority projection as passive operator text.
#[must_use]
pub fn lifecycle_authority_report_text(report: &LifecycleAuthorityReportV1) -> String {
    let mut lines = vec![
        "Lifecycle authority report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("report_id: {}", report.report_id),
        format!("report_digest: {}", report.report_digest),
        format!("check_id: {}", report.check_id),
        format!("plan_id: {}", report.plan_id),
        format!("inventory_id: {}", report.inventory_id),
        String::new(),
        "counts:".to_string(),
        format!("  authorities: {}", report.authorities.len()),
        format!(
            "  external_action_required: {}",
            report.external_action_required_count
        ),
        format!("  blocked: {}", report.blocked_count),
    ];

    append_lifecycle_authority_items(&mut lines, &report.authorities);
    lines.join("\n")
}
