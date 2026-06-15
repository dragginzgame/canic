use super::super::super::*;
use super::super::append_hard_failure_items;
use super::shared::{
    append_controller_observation_items, append_external_action_items,
    append_observation_gap_items, authority_receipt_mutation_label,
    deployment_command_result_label, deployment_execution_status_label,
};

/// Render an authority dry-run receipt as read-only operator text.
#[must_use]
pub fn authority_receipt_text(receipt: &AuthorityReceiptV1) -> String {
    let mut lines = vec![
        "Authority dry-run receipt".to_string(),
        "mode: dry_run".to_string(),
        format!("operation_id: {}", receipt.operation_id),
        format!(
            "status: {}",
            deployment_execution_status_label(receipt.operation_status)
        ),
        format!(
            "command_result: {}",
            deployment_command_result_label(&receipt.command_result)
        ),
        format!(
            "check_id: {}",
            receipt.check_id.as_deref().unwrap_or("not recorded")
        ),
        format!("plan_id: {}", receipt.reconciliation_plan_id),
        format!("report_id: {}", receipt.authority_report_id),
        format!("inventory_id: {}", receipt.inventory_id),
        format!(
            "authority_profile_hash: {}",
            receipt
                .authority_profile_hash
                .as_deref()
                .unwrap_or("not recorded")
        ),
        format!("started_at: {}", receipt.started_at),
        format!(
            "finished_at: {}",
            receipt.finished_at.as_deref().unwrap_or("not recorded")
        ),
        String::new(),
        "dry_run_evidence:".to_string(),
        format!(
            "  controller_mutation: {}",
            authority_receipt_mutation_label(receipt)
        ),
        format!("  attempted_actions: {}", receipt.attempted_actions.len()),
        format!(
            "  verified_controller_observations: {}",
            receipt.verified_controller_observations.len()
        ),
        format!("  hard_failures: {}", receipt.hard_failures.len()),
        format!(
            "  unresolved_observation_gaps: {}",
            receipt.unresolved_observation_gaps.len()
        ),
        format!(
            "  unresolved_external_actions: {}",
            receipt.unresolved_external_actions.len()
        ),
    ];

    append_controller_observation_items(
        &mut lines,
        "verified_controller_observations",
        &receipt.verified_controller_observations,
    );
    append_hard_failure_items(&mut lines, "hard_failures", &receipt.hard_failures);
    append_observation_gap_items(
        &mut lines,
        "unresolved_observation_gaps",
        &receipt.unresolved_observation_gaps,
    );
    append_external_action_items(
        &mut lines,
        "unresolved_external_actions",
        &receipt.unresolved_external_actions,
    );
    lines.join("\n")
}
