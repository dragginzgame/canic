use super::super::super::*;
use super::shared::{
    append_plan_action_preview, append_plan_canister_actions, authority_plan_state_counts,
};

/// Render an authority reconciliation plan as read-only operator text.
#[must_use]
pub fn authority_plan_text(plan: &AuthorityReconciliationPlanV1) -> String {
    let state_counts = authority_plan_state_counts(plan);
    let mut lines = vec![
        "Authority reconciliation plan".to_string(),
        "mode: dry_run".to_string(),
        format!("plan_id: {}", plan.plan_id),
        format!("inventory_id: {}", plan.inventory_id),
        format!(
            "authority_profile_hash: {}",
            plan.authority_profile_hash
                .as_deref()
                .unwrap_or("not recorded")
        ),
        String::new(),
        "counts:".to_string(),
        format!("  canister_actions: {}", plan.canister_actions.len()),
        format!("  automatic_actions: {}", plan.automatic_actions.len()),
        format!(
            "  external_actions_required: {}",
            plan.external_actions_required.len()
        ),
        format!("  hard_failures: {}", plan.hard_failures.len()),
        String::new(),
        "states:".to_string(),
        format!("  already_correct: {}", state_counts.already_correct),
        format!(
            "  can_apply_automatically: {}",
            state_counts.can_apply_automatically
        ),
        format!(
            "  requires_external_action: {}",
            state_counts.requires_external_action
        ),
        format!("  unsafe_blocked: {}", state_counts.unsafe_blocked),
        format!("  unknown: {}", state_counts.unknown),
    ];

    append_plan_canister_actions(&mut lines, plan);
    append_plan_action_preview(&mut lines, plan);
    lines.join("\n")
}
