use super::super::super::*;
use super::super::{append_hard_failure_items, append_warning_items};
use super::shared::{append_promotion_role_items, promotion_readiness_status_label};

/// Render promotion readiness as passive operator text.
#[must_use]
pub fn promotion_readiness_text(readiness: &PromotionReadinessV1) -> String {
    let restage_required = readiness
        .roles
        .iter()
        .filter(|role| role.restage_required)
        .count();
    let mut lines = vec![
        "Promotion readiness report".to_string(),
        "mode: passive".to_string(),
        format!(
            "status: {}",
            promotion_readiness_status_label(readiness.status)
        ),
        format!("readiness_id: {}", readiness.readiness_id),
        format!(
            "promotion_readiness_digest: {}",
            readiness.promotion_readiness_digest
        ),
        format!("target_plan_id: {}", readiness.target_plan_id),
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", readiness.roles.len()),
        format!("  blockers: {}", readiness.blockers.len()),
        format!("  warnings: {}", readiness.warnings.len()),
        format!("  restage_required: {restage_required}"),
    ];

    append_promotion_role_items(&mut lines, &readiness.roles);
    append_hard_failure_items(&mut lines, "blockers", &readiness.blockers);
    append_warning_items(&mut lines, "warnings", &readiness.warnings);
    lines.join("\n")
}
