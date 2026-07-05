use super::super::append_hard_failure_items;
use super::shared::{append_promotion_policy_decision_items, promotion_readiness_status_label};
use crate::deployment_truth::PromotionPolicyCheckV1;

/// Render a promotion policy check as passive operator text.
#[must_use]
pub fn promotion_policy_check_text(check: &PromotionPolicyCheckV1) -> String {
    let satisfied = check
        .roles
        .iter()
        .filter(|role| role.policy_satisfied)
        .count();
    let mut lines = vec![
        "Promotion policy check".to_string(),
        "mode: passive".to_string(),
        format!("status: {}", promotion_readiness_status_label(check.status)),
        format!("check_id: {}", check.check_id),
        format!(
            "promotion_policy_check_digest: {}",
            check.promotion_policy_check_digest
        ),
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", check.roles.len()),
        format!("  policy_satisfied: {satisfied}"),
        format!("  blockers: {}", check.blockers.len()),
    ];

    append_promotion_policy_decision_items(&mut lines, &check.roles);
    append_hard_failure_items(&mut lines, "blockers", &check.blockers);
    lines.join("\n")
}
