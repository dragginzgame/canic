use super::super::super::*;
use super::super::append_string_items;
use super::shared::{
    append_external_lifecycle_handoff_action_items, append_external_lifecycle_pending_action_items,
};

/// Render an external lifecycle pending report as passive operator text.
#[must_use]
pub fn external_lifecycle_pending_report_text(report: &ExternalLifecyclePendingReportV1) -> String {
    let mut lines = vec![
        "External lifecycle pending report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("status: {}", report.status.label()),
        format!("report_id: {}", report.report_id),
        format!("report_digest: {}", report.report_digest),
        format!("lifecycle_plan_id: {}", report.lifecycle_plan_id),
        format!("lifecycle_plan_digest: {}", report.lifecycle_plan_digest),
        format!("proposal_report_id: {}", report.proposal_report_id),
        format!("proposal_report_digest: {}", report.proposal_report_digest),
        format!("deployment_plan_id: {}", report.deployment_plan_id),
        format!("deployment_plan_digest: {}", report.deployment_plan_digest),
        format!("inventory_id: {}", report.inventory_id),
        String::new(),
        "counts:".to_string(),
        format!("  directly_executable: {}", report.direct_upgrade_count),
        format!("  pending_external: {}", report.pending_external_count),
        format!("  blocked: {}", report.blocked_count),
        format!("  residual_exposure: {}", report.residual_exposure.len()),
    ];

    append_external_lifecycle_pending_action_items(&mut lines, &report.pending_external_actions);
    append_string_items(&mut lines, "blocked_subjects", &report.blocked_subjects);
    append_string_items(&mut lines, "residual_exposure", &report.residual_exposure);
    lines.join("\n")
}

/// Render an external lifecycle check as passive operator text.
#[must_use]
pub fn external_lifecycle_check_text(check: &ExternalLifecycleCheckV1) -> String {
    let mut lines = vec![
        "External lifecycle check".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("status: {}", check.status.label()),
        format!("check_id: {}", check.check_id),
        format!("check_digest: {}", check.check_digest),
        format!("summary: {}", check.summary),
        format!("lifecycle_plan_id: {}", check.lifecycle_plan_id),
        format!("lifecycle_plan_digest: {}", check.lifecycle_plan_digest),
        format!("proposal_report_id: {}", check.proposal_report_id),
        format!("proposal_report_digest: {}", check.proposal_report_digest),
        format!("pending_report_id: {}", check.pending_report_id),
        format!("pending_report_digest: {}", check.pending_report_digest),
        format!("deployment_plan_id: {}", check.deployment_plan_id),
        format!("deployment_plan_digest: {}", check.deployment_plan_digest),
        format!("inventory_id: {}", check.inventory_id),
        String::new(),
        "counts:".to_string(),
        format!("  directly_executable: {}", check.direct_upgrade_count),
        format!("  pending_external: {}", check.pending_external_count),
        format!("  blocked: {}", check.blocked_count),
        format!("  residual_exposure: {}", check.residual_exposure_count),
    ];
    append_string_items(&mut lines, "next_actions", &check.next_actions);
    lines.join("\n")
}

/// Render an external lifecycle handoff as passive operator text.
#[must_use]
pub fn external_lifecycle_handoff_text(handoff: &ExternalLifecycleHandoffV1) -> String {
    let mut lines = vec![
        "External lifecycle handoff".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("status: {}", handoff.status.label()),
        format!("handoff_id: {}", handoff.handoff_id),
        format!("handoff_digest: {}", handoff.handoff_digest),
        format!("summary: {}", handoff.operator_summary),
        format!("lifecycle_check_id: {}", handoff.lifecycle_check_id),
        format!("lifecycle_check_digest: {}", handoff.lifecycle_check_digest),
        format!("pending_report_id: {}", handoff.pending_report_id),
        format!("pending_report_digest: {}", handoff.pending_report_digest),
        format!("proposal_report_id: {}", handoff.proposal_report_id),
        format!("proposal_report_digest: {}", handoff.proposal_report_digest),
        format!("deployment_plan_id: {}", handoff.deployment_plan_id),
        format!("deployment_plan_digest: {}", handoff.deployment_plan_digest),
        format!("inventory_id: {}", handoff.inventory_id),
        String::new(),
        "counts:".to_string(),
        format!("  handoff_actions: {}", handoff.handoff_actions.len()),
        format!("  blocked_subjects: {}", handoff.blocked_subjects.len()),
        format!("  residual_exposure: {}", handoff.residual_exposure.len()),
    ];
    append_external_lifecycle_handoff_action_items(&mut lines, &handoff.handoff_actions);
    append_string_items(&mut lines, "blocked_subjects", &handoff.blocked_subjects);
    append_string_items(&mut lines, "residual_exposure", &handoff.residual_exposure);
    lines.join("\n")
}

/// Render a critical external fix report as passive operator text.
#[must_use]
pub fn critical_external_fix_report_text(report: &CriticalExternalFixReportV1) -> String {
    let mut lines = vec![
        "Critical external fix report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("report_id: {}", report.report_id),
        format!("report_digest: {}", report.report_digest),
        format!("fix_id: {}", report.fix_id),
        format!("severity: {}", report.severity),
        format!("lifecycle_plan_id: {}", report.lifecycle_plan_id),
        format!("lifecycle_plan_digest: {}", report.lifecycle_plan_digest),
        format!("pending_report_id: {}", report.pending_report_id),
        format!("pending_report_digest: {}", report.pending_report_digest),
        format!("deployment_plan_id: {}", report.deployment_plan_id),
        format!("deployment_plan_digest: {}", report.deployment_plan_digest),
        format!("inventory_id: {}", report.inventory_id),
        String::new(),
        "counts:".to_string(),
        format!("  affected_roles: {}", report.affected_roles.len()),
        format!("  affected_canisters: {}", report.affected_canisters.len()),
        format!(
            "  directly_patchable_roles: {}",
            report.directly_patchable_roles.len()
        ),
        format!(
            "  externally_blocked_roles: {}",
            report.externally_blocked_roles.len()
        ),
        format!(
            "  dependency_blocked_roles: {}",
            report.dependency_blocked_roles.len()
        ),
        format!(
            "  required_external_actions: {}",
            report.required_external_actions.len()
        ),
        format!("  residual_exposure: {}", report.residual_exposure.len()),
    ];

    append_string_items(
        &mut lines,
        "directly_patchable_roles",
        &report.directly_patchable_roles,
    );
    append_string_items(
        &mut lines,
        "externally_blocked_roles",
        &report.externally_blocked_roles,
    );
    append_string_items(
        &mut lines,
        "dependency_blocked_roles",
        &report.dependency_blocked_roles,
    );
    append_string_items(
        &mut lines,
        "required_external_actions",
        &report.required_external_actions,
    );
    append_string_items(
        &mut lines,
        "protected_call_implications",
        &report.protected_call_implications,
    );
    append_string_items(&mut lines, "residual_exposure", &report.residual_exposure);
    append_string_items(
        &mut lines,
        "operator_next_steps",
        &report.operator_next_steps,
    );
    lines.join("\n")
}
