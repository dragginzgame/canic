use super::super::*;
use super::{append_hard_failure_items, append_string_items, append_warning_items};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DeploymentComparisonTextLabel(&'static str);

impl DeploymentComparisonTextLabel {
    const ARTIFACT: Self = Self("artifact");
    const ARTIFACT_DIFF: Self = Self("artifact_diff");
    const AUTHORITY: Self = Self("authority");
    const AUTHORITY_DIFF: Self = Self("authority_diff");
    const CHECK: Self = Self("check");
    const COMPARED_AT: Self = Self("compared_at");
    const COUNTS: Self = Self("counts");
    const EMBEDDED_CONFIG: Self = Self("embedded_config");
    const EMBEDDED_CONFIG_DIFF: Self = Self("embedded_config_diff");
    const EXECUTION_NONE: Self = Self("execution: none");
    const EXTERNAL_LIFECYCLE: Self = Self("external_lifecycle");
    const EXTERNAL_LIFECYCLE_DIFF: Self = Self("external_lifecycle_diff");
    const HARD_FAILURES: Self = Self("hard_failures");
    const IDENTITY: Self = Self("identity");
    const IDENTITY_DIFF: Self = Self("identity_diff");
    const INVENTORY: Self = Self("inventory");
    const LEFT: Self = Self("left");
    const MISSING: Self = Self("missing");
    const MODE_PASSIVE: Self = Self("mode: passive");
    const MODULE_HASH: Self = Self("module_hash");
    const MODULE_HASH_DIFF: Self = Self("module_hash_diff");
    const NEXT_ACTIONS: Self = Self("next_actions");
    const PLAN: Self = Self("plan");
    const POOL: Self = Self("pool");
    const POOL_DIFF: Self = Self("pool_diff");
    const REPORT_DIGEST: Self = Self("report_digest");
    const REPORT_ID: Self = Self("report_id");
    const RIGHT: Self = Self("right");
    const STATUS: Self = Self("status");
    const TITLE: Self = Self("Deployment comparison report");
    const VERIFIER_READINESS: Self = Self("verifier_readiness");
    const VERIFIER_READINESS_DIFF: Self = Self("verifier_readiness_diff");
    const WARNINGS: Self = Self("warnings");

    #[must_use]
    const fn as_str(self) -> &'static str {
        self.0
    }
}

/// Render a cross-deployment comparison report as passive operator text.
#[must_use]
pub fn deployment_comparison_report_text(report: &DeploymentComparisonReportV1) -> String {
    let mut lines = comparison_header_lines(report);
    lines.push(String::new());
    lines.extend(comparison_count_lines(report));
    append_comparison_sections(&mut lines, report);
    lines.join("\n")
}

fn comparison_header_lines(report: &DeploymentComparisonReportV1) -> Vec<String> {
    vec![
        DeploymentComparisonTextLabel::TITLE.as_str().to_string(),
        DeploymentComparisonTextLabel::MODE_PASSIVE
            .as_str()
            .to_string(),
        DeploymentComparisonTextLabel::EXECUTION_NONE
            .as_str()
            .to_string(),
        format!(
            "{}: {}",
            DeploymentComparisonTextLabel::STATUS.as_str(),
            report.status.label()
        ),
        format!(
            "{}: {}",
            DeploymentComparisonTextLabel::REPORT_ID.as_str(),
            report.report_id
        ),
        format!(
            "{}: {}",
            DeploymentComparisonTextLabel::REPORT_DIGEST.as_str(),
            report.report_digest
        ),
        format!(
            "{}: {}",
            DeploymentComparisonTextLabel::COMPARED_AT.as_str(),
            report.compared_at
        ),
        format!(
            "{}: {} {}={} {}={} {}={}",
            DeploymentComparisonTextLabel::LEFT.as_str(),
            report.left.label,
            DeploymentComparisonTextLabel::CHECK.as_str(),
            report.left.check_id,
            DeploymentComparisonTextLabel::PLAN.as_str(),
            report.left.plan_id,
            DeploymentComparisonTextLabel::INVENTORY.as_str(),
            report.left.inventory_id
        ),
        format!(
            "{}: {} {}={} {}={} {}={}",
            DeploymentComparisonTextLabel::RIGHT.as_str(),
            report.right.label,
            DeploymentComparisonTextLabel::CHECK.as_str(),
            report.right.check_id,
            DeploymentComparisonTextLabel::PLAN.as_str(),
            report.right.plan_id,
            DeploymentComparisonTextLabel::INVENTORY.as_str(),
            report.right.inventory_id
        ),
    ]
}

fn comparison_count_lines(report: &DeploymentComparisonReportV1) -> Vec<String> {
    vec![
        format!("{}:", DeploymentComparisonTextLabel::COUNTS.as_str()),
        format!(
            "  {}: {}",
            DeploymentComparisonTextLabel::IDENTITY.as_str(),
            report.identity_diff.len()
        ),
        format!(
            "  {}: {}",
            DeploymentComparisonTextLabel::ARTIFACT.as_str(),
            report.artifact_diff.len()
        ),
        format!(
            "  {}: {}",
            DeploymentComparisonTextLabel::MODULE_HASH.as_str(),
            report.module_hash_diff.len()
        ),
        format!(
            "  {}: {}",
            DeploymentComparisonTextLabel::EMBEDDED_CONFIG.as_str(),
            report.embedded_config_diff.len()
        ),
        format!(
            "  {}: {}",
            DeploymentComparisonTextLabel::AUTHORITY.as_str(),
            report.authority_diff.len()
        ),
        format!(
            "  {}: {}",
            DeploymentComparisonTextLabel::POOL.as_str(),
            report.pool_diff.len()
        ),
        format!(
            "  {}: {}",
            DeploymentComparisonTextLabel::VERIFIER_READINESS.as_str(),
            report.verifier_readiness_diff.len()
        ),
        format!(
            "  {}: {}",
            DeploymentComparisonTextLabel::EXTERNAL_LIFECYCLE.as_str(),
            report.external_lifecycle_diff.len()
        ),
        format!(
            "  {}: {}",
            DeploymentComparisonTextLabel::HARD_FAILURES.as_str(),
            report.hard_failures.len()
        ),
        format!(
            "  {}: {}",
            DeploymentComparisonTextLabel::WARNINGS.as_str(),
            report.warnings.len()
        ),
    ]
}

fn append_comparison_sections(lines: &mut Vec<String>, report: &DeploymentComparisonReportV1) {
    append_comparison_diff_items(
        lines,
        DeploymentComparisonTextLabel::IDENTITY_DIFF,
        &report.identity_diff,
    );
    append_comparison_diff_items(
        lines,
        DeploymentComparisonTextLabel::ARTIFACT_DIFF,
        &report.artifact_diff,
    );
    append_comparison_diff_items(
        lines,
        DeploymentComparisonTextLabel::MODULE_HASH_DIFF,
        &report.module_hash_diff,
    );
    append_comparison_diff_items(
        lines,
        DeploymentComparisonTextLabel::EMBEDDED_CONFIG_DIFF,
        &report.embedded_config_diff,
    );
    append_comparison_diff_items(
        lines,
        DeploymentComparisonTextLabel::AUTHORITY_DIFF,
        &report.authority_diff,
    );
    append_comparison_diff_items(
        lines,
        DeploymentComparisonTextLabel::POOL_DIFF,
        &report.pool_diff,
    );
    append_comparison_diff_items(
        lines,
        DeploymentComparisonTextLabel::VERIFIER_READINESS_DIFF,
        &report.verifier_readiness_diff,
    );
    append_comparison_diff_items(
        lines,
        DeploymentComparisonTextLabel::EXTERNAL_LIFECYCLE_DIFF,
        &report.external_lifecycle_diff,
    );
    append_hard_failure_items(
        lines,
        DeploymentComparisonTextLabel::HARD_FAILURES.as_str(),
        &report.hard_failures,
    );
    append_warning_items(
        lines,
        DeploymentComparisonTextLabel::WARNINGS.as_str(),
        &report.warnings,
    );
    append_string_items(
        lines,
        DeploymentComparisonTextLabel::NEXT_ACTIONS.as_str(),
        &report.next_actions,
    );
}

fn append_comparison_diff_items(
    lines: &mut Vec<String>,
    label: DeploymentComparisonTextLabel,
    items: &[DeploymentComparisonDiffV1],
) {
    if items.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{}:", label.as_str()));
    for item in items {
        lines.push(format!(
            "  - {:?}: {} left={} right={} severity={:?}",
            item.category,
            item.subject,
            item.left
                .as_deref()
                .unwrap_or(DeploymentComparisonTextLabel::MISSING.as_str()),
            item.right
                .as_deref()
                .unwrap_or(DeploymentComparisonTextLabel::MISSING.as_str()),
            item.severity
        ));
        lines.push(format!("    {}", item.message));
    }
}
