use super::super::super::*;
use super::super::append_string_items;
use super::shared::{append_external_lifecycle_role_items, external_lifecycle_plan_status_label};

/// Render an external lifecycle plan as passive operator text.
#[must_use]
pub fn external_lifecycle_plan_text(plan: &ExternalLifecyclePlanV1) -> String {
    let mut lines = vec![
        "External lifecycle plan".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!(
            "status: {}",
            external_lifecycle_plan_status_label(plan.status)
        ),
        format!("lifecycle_plan_id: {}", plan.lifecycle_plan_id),
        format!("lifecycle_plan_digest: {}", plan.lifecycle_plan_digest),
        format!("deployment_plan_id: {}", plan.deployment_plan_id),
        format!("deployment_plan_digest: {}", plan.deployment_plan_digest),
        format!("inventory_id: {}", plan.inventory_id),
        String::new(),
        "counts:".to_string(),
        format!(
            "  directly_executable: {}",
            plan.directly_executable_role_upgrades.len()
        ),
        format!(
            "  proposed_external: {}",
            plan.proposed_external_role_upgrades.len()
        ),
        format!("  blocked: {}", plan.blocked_role_upgrades.len()),
        format!("  residual_exposure: {}", plan.residual_exposure.len()),
    ];

    append_external_lifecycle_role_items(
        &mut lines,
        "directly_executable_role_upgrades",
        &plan.directly_executable_role_upgrades,
    );
    append_external_lifecycle_role_items(
        &mut lines,
        "proposed_external_role_upgrades",
        &plan.proposed_external_role_upgrades,
    );
    append_external_lifecycle_role_items(
        &mut lines,
        "blocked_role_upgrades",
        &plan.blocked_role_upgrades,
    );
    append_string_items(&mut lines, "residual_exposure", &plan.residual_exposure);
    lines.join("\n")
}
