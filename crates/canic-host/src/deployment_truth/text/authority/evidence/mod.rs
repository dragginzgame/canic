use super::super::super::*;
use super::super::{append_hard_failure_items, safety_status_label};
use super::shared::{
    append_controller_observation_items, append_external_action_items, append_next_actions,
    append_observation_gap_items, authority_receipt_mutation_label,
    deployment_command_result_label, deployment_execution_status_label,
};

/// Render a complete authority evidence bundle as read-only operator text.
#[must_use]
pub fn authority_evidence_text(evidence: &AuthorityDryRunEvidenceV1) -> String {
    let mut lines = vec![
        "Authority dry-run evidence".to_string(),
        "mode: dry_run".to_string(),
        format!("evidence_id: {}", evidence.evidence_id),
        format!("check_id: {}", evidence.check_id),
        format!("generated_at: {}", evidence.generated_at),
        format!("plan_id: {}", evidence.reconciliation_plan.plan_id),
        format!("report_id: {}", evidence.authority_report.report_id),
        format!("receipt_id: {}", evidence.authority_receipt.operation_id),
        format!(
            "inventory_id: {}",
            evidence.reconciliation_plan.inventory_id
        ),
        format!(
            "authority_profile_hash: {}",
            evidence
                .reconciliation_plan
                .authority_profile_hash
                .as_deref()
                .unwrap_or("not recorded")
        ),
        String::new(),
        "report:".to_string(),
        format!(
            "  status: {}",
            safety_status_label(evidence.authority_report.status)
        ),
        format!("  summary: {}", evidence.authority_report.summary),
        format!(
            "  hard_failures: {}",
            evidence.authority_report.hard_failures.len()
        ),
        format!(
            "  external_actions_required: {}",
            evidence.authority_report.external_actions_required.len()
        ),
        format!(
            "  observation_gaps: {}",
            evidence.authority_report.observation_gaps.len()
        ),
        String::new(),
        "receipt:".to_string(),
        format!(
            "  status: {}",
            deployment_execution_status_label(evidence.authority_receipt.operation_status)
        ),
        format!(
            "  command_result: {}",
            deployment_command_result_label(&evidence.authority_receipt.command_result)
        ),
        format!(
            "  controller_mutation: {}",
            authority_receipt_mutation_label(&evidence.authority_receipt)
        ),
        format!(
            "  attempted_actions: {}",
            evidence.authority_receipt.attempted_actions.len()
        ),
        format!(
            "  verified_controller_observations: {}",
            evidence
                .authority_receipt
                .verified_controller_observations
                .len()
        ),
    ];

    append_controller_observation_items(
        &mut lines,
        "verified_controller_observations",
        &evidence.authority_receipt.verified_controller_observations,
    );
    append_next_actions(&mut lines, &evidence.authority_report);
    append_hard_failure_items(
        &mut lines,
        "hard_failures",
        &evidence.authority_report.hard_failures,
    );
    append_observation_gap_items(
        &mut lines,
        "observation_gaps",
        &evidence.authority_report.observation_gaps,
    );
    append_external_action_items(
        &mut lines,
        "external_actions_required",
        &evidence.authority_report.external_actions_required,
    );
    lines.join("\n")
}
