use super::super::super::*;
use super::super::append_hard_failure_items;
use super::shared::{
    append_promotion_artifact_identity_group_items, append_promotion_artifact_identity_role_items,
    promotion_readiness_status_label,
};

/// Render a promotion artifact identity report as passive operator text.
#[must_use]
pub fn promotion_artifact_identity_report_text(
    report: &PromotionArtifactIdentityReportV1,
) -> String {
    let mut lines = vec![
        "Promotion artifact identity report".to_string(),
        "mode: passive".to_string(),
        format!(
            "status: {}",
            promotion_readiness_status_label(report.status)
        ),
        format!("report_id: {}", report.report_id),
        format!(
            "artifact_identity_report_digest: {}",
            report.artifact_identity_report_digest
        ),
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", report.summary.role_count),
        format!("  identity_groups: {}", report.summary.identity_group_count),
        format!(
            "  shared_identity_groups: {}",
            report.summary.shared_identity_group_count
        ),
        format!(
            "  digest_pinned_roles: {}",
            report.summary.digest_pinned_role_count
        ),
        format!(
            "  source_build_roles: {}",
            report.summary.source_build_role_count
        ),
        format!(
            "  deferred_identity_roles: {}",
            report.summary.deferred_identity_role_count
        ),
        format!("  blockers: {}", report.blockers.len()),
    ];

    append_promotion_artifact_identity_group_items(&mut lines, &report.identity_groups);
    append_promotion_artifact_identity_role_items(&mut lines, &report.roles);
    append_hard_failure_items(&mut lines, "blockers", &report.blockers);
    lines.join("\n")
}
