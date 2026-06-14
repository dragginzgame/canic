use super::super::*;
use super::{
    append_hard_failure_items, append_string_items, append_warning_items, safety_status_label,
};

/// Render a cross-deployment comparison report as passive operator text.
#[must_use]
pub fn deployment_comparison_report_text(report: &DeploymentComparisonReportV1) -> String {
    let mut lines = vec![
        "Deployment comparison report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("status: {}", safety_status_label(report.status)),
        format!("report_id: {}", report.report_id),
        format!("report_digest: {}", report.report_digest),
        format!("compared_at: {}", report.compared_at),
        format!(
            "left: {} check={} plan={} inventory={}",
            report.left.label, report.left.check_id, report.left.plan_id, report.left.inventory_id
        ),
        format!(
            "right: {} check={} plan={} inventory={}",
            report.right.label,
            report.right.check_id,
            report.right.plan_id,
            report.right.inventory_id
        ),
        String::new(),
        "counts:".to_string(),
        format!("  identity: {}", report.identity_diff.len()),
        format!("  artifact: {}", report.artifact_diff.len()),
        format!("  module_hash: {}", report.module_hash_diff.len()),
        format!("  embedded_config: {}", report.embedded_config_diff.len()),
        format!("  authority: {}", report.authority_diff.len()),
        format!("  pool: {}", report.pool_diff.len()),
        format!(
            "  verifier_readiness: {}",
            report.verifier_readiness_diff.len()
        ),
        format!(
            "  external_lifecycle: {}",
            report.external_lifecycle_diff.len()
        ),
        format!("  hard_failures: {}", report.hard_failures.len()),
        format!("  warnings: {}", report.warnings.len()),
    ];

    append_comparison_diff_items(&mut lines, "identity_diff", &report.identity_diff);
    append_comparison_diff_items(&mut lines, "artifact_diff", &report.artifact_diff);
    append_comparison_diff_items(&mut lines, "module_hash_diff", &report.module_hash_diff);
    append_comparison_diff_items(
        &mut lines,
        "embedded_config_diff",
        &report.embedded_config_diff,
    );
    append_comparison_diff_items(&mut lines, "authority_diff", &report.authority_diff);
    append_comparison_diff_items(&mut lines, "pool_diff", &report.pool_diff);
    append_comparison_diff_items(
        &mut lines,
        "verifier_readiness_diff",
        &report.verifier_readiness_diff,
    );
    append_comparison_diff_items(
        &mut lines,
        "external_lifecycle_diff",
        &report.external_lifecycle_diff,
    );
    append_hard_failure_items(&mut lines, "hard_failures", &report.hard_failures);
    append_warning_items(&mut lines, "warnings", &report.warnings);
    append_string_items(&mut lines, "next_actions", &report.next_actions);
    lines.join("\n")
}

fn append_comparison_diff_items(
    lines: &mut Vec<String>,
    label: &str,
    items: &[DeploymentComparisonDiffV1],
) {
    if items.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for item in items {
        lines.push(format!(
            "  - {:?}: {} left={} right={} severity={:?}",
            item.category,
            item.subject,
            item.left.as_deref().unwrap_or("missing"),
            item.right.as_deref().unwrap_or("missing"),
            item.severity
        ));
        lines.push(format!("    {}", item.message));
    }
}
