use super::super::super::*;
use super::super::append_hard_failure_items;

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

fn authority_plan_state_counts(plan: &AuthorityReconciliationPlanV1) -> AuthorityPlanStateCounts {
    let mut counts = AuthorityPlanStateCounts::default();
    for action in &plan.canister_actions {
        match action.state {
            AuthorityReconciliationStateV1::AlreadyCorrect => counts.already_correct += 1,
            AuthorityReconciliationStateV1::CanApplyAutomatically => {
                counts.can_apply_automatically += 1;
            }
            AuthorityReconciliationStateV1::RequiresExternalAction => {
                counts.requires_external_action += 1;
            }
            AuthorityReconciliationStateV1::UnsafeBlocked => counts.unsafe_blocked += 1,
            AuthorityReconciliationStateV1::Unknown => counts.unknown += 1,
        }
    }
    counts
}

fn append_plan_canister_actions(lines: &mut Vec<String>, plan: &AuthorityReconciliationPlanV1) {
    if plan.canister_actions.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("canister_actions:".to_string());
    for action in &plan.canister_actions {
        lines.push(format!(
            "  - {} {:?}/{:?}: {}",
            authority_canister_action_subject(action),
            action.state,
            action.action,
            action.reason
        ));
    }
}

fn authority_canister_action_subject(action: &CanisterAuthorityActionV1) -> String {
    if let Some(role) = &action.role
        && let Some(canister_id) = &action.canister_id
    {
        return format!("{role} ({canister_id})");
    }
    if let Some(role) = &action.role {
        return role.clone();
    }
    action
        .canister_id
        .clone()
        .unwrap_or_else(|| "unknown canister".to_string())
}

fn append_plan_action_preview(lines: &mut Vec<String>, plan: &AuthorityReconciliationPlanV1) {
    if !plan.automatic_actions.is_empty() {
        lines.push(String::new());
        lines.push("automatic_actions:".to_string());
        for action in &plan.automatic_actions {
            lines.push(authority_action_line_with_delta(
                &action.subject,
                action.action,
                &action.reason,
                &action.controller_delta,
            ));
        }
    }
    if !plan.external_actions_required.is_empty() {
        lines.push(String::new());
        lines.push("external_actions_required:".to_string());
        for action in &plan.external_actions_required {
            lines.push(authority_action_line_with_delta(
                &action.subject,
                action.action,
                &action.reason,
                &action.controller_delta,
            ));
        }
    }
    append_hard_failure_items(lines, "hard_failures", &plan.hard_failures);
}

fn authority_action_line_with_delta(
    subject: &str,
    action: AuthorityActionV1,
    reason: &str,
    delta: &AuthorityControllerDeltaV1,
) -> String {
    let add = authority_delta_list(&delta.add_controllers);
    let remove = authority_delta_list(&delta.remove_controllers);
    format!("  - {subject} {action:?}: {reason} [add={add}; remove={remove}]")
}

fn authority_delta_list(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(",")
    }
}

#[derive(Default)]
struct AuthorityPlanStateCounts {
    already_correct: usize,
    can_apply_automatically: usize,
    requires_external_action: usize,
    unsafe_blocked: usize,
    unknown: usize,
}
