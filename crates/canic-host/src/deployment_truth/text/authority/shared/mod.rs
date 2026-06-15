use super::super::super::*;
use super::super::append_hard_failure_items;

pub(super) fn authority_plan_state_counts(
    plan: &AuthorityReconciliationPlanV1,
) -> AuthorityPlanStateCounts {
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

pub(super) fn append_plan_canister_actions(
    lines: &mut Vec<String>,
    plan: &AuthorityReconciliationPlanV1,
) {
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

pub(super) fn append_plan_action_preview(
    lines: &mut Vec<String>,
    plan: &AuthorityReconciliationPlanV1,
) {
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
    append_external_action_items(
        lines,
        "external_actions_required",
        &plan.external_actions_required,
    );
    append_hard_failure_items(lines, "hard_failures", &plan.hard_failures);
}

///
/// AuthorityPlanStateCounts
///
#[derive(Default)]
pub(super) struct AuthorityPlanStateCounts {
    pub(super) already_correct: usize,
    pub(super) can_apply_automatically: usize,
    pub(super) requires_external_action: usize,
    pub(super) unsafe_blocked: usize,
    pub(super) unknown: usize,
}

pub(super) fn append_blockers(lines: &mut Vec<String>, report: &AuthorityReportV1) {
    if report.apply_readiness.blockers.is_empty() {
        lines.push("  blockers: none".to_string());
        return;
    }
    lines.push("  blockers:".to_string());
    for blocker in &report.apply_readiness.blockers {
        lines.push(format!("    - {}", authority_apply_blocker_label(*blocker)));
    }
}

pub(super) fn append_next_actions(lines: &mut Vec<String>, report: &AuthorityReportV1) {
    if report.next_actions.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("next_actions:".to_string());
    for action in &report.next_actions {
        lines.push(format!("  - {action}"));
    }
}

pub(super) fn append_observation_gap_items(
    lines: &mut Vec<String>,
    label: &str,
    gaps: &[DeploymentObservationGapV1],
) {
    if gaps.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for gap in gaps {
        lines.push(format!("  - {}: {}", gap.key, gap.description));
    }
}

pub(super) fn append_external_action_items(
    lines: &mut Vec<String>,
    label: &str,
    actions: &[AuthorityExternalActionV1],
) {
    if actions.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for action in actions {
        lines.push(authority_action_line_with_delta(
            &action.subject,
            action.action,
            &action.reason,
            &action.controller_delta,
        ));
    }
}

pub(super) fn append_controller_observation_items(
    lines: &mut Vec<String>,
    label: &str,
    observations: &[AuthorityControllerObservationV1],
) {
    if observations.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for observation in observations {
        lines.push(format!(
            "  - {} {:?}/{:?}: observed=[{}] desired=[{}]{}",
            observation.subject,
            observation.state,
            observation.action,
            authority_delta_list(&observation.observed_controllers),
            authority_delta_list(&observation.desired_controllers),
            authority_delta_suffix(&observation.controller_delta)
        ));
    }
}

pub(super) fn append_authority_action_summary(lines: &mut Vec<String>, report: &AuthorityReportV1) {
    if !report.automatic_actions.is_empty() {
        lines.push(String::new());
        lines.push("automatic_actions:".to_string());
        for action in &report.automatic_actions {
            lines.push(authority_action_line_with_delta(
                &action.subject,
                action.action,
                &action.reason,
                &action.controller_delta,
            ));
        }
    }
    append_external_action_items(
        lines,
        "external_actions_required",
        &report.external_actions_required,
    );
}

fn authority_action_line(subject: &str, action: AuthorityActionV1, reason: &str) -> String {
    format!("  - {subject} {action:?}: {reason}")
}

fn authority_action_line_with_delta(
    subject: &str,
    action: AuthorityActionV1,
    reason: &str,
    delta: &AuthorityControllerDeltaV1,
) -> String {
    format!(
        "{}{}",
        authority_action_line(subject, action, reason),
        authority_delta_suffix(delta)
    )
}

fn authority_delta_suffix(delta: &AuthorityControllerDeltaV1) -> String {
    let add = authority_delta_list(&delta.add_controllers);
    let remove = authority_delta_list(&delta.remove_controllers);
    format!(" [add={add}; remove={remove}]")
}

fn authority_delta_list(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(",")
    }
}

const fn authority_apply_blocker_label(blocker: AuthorityApplyBlockerV1) -> &'static str {
    match blocker {
        AuthorityApplyBlockerV1::UnsafeBlocked => "unsafe_blocked",
        AuthorityApplyBlockerV1::HardFailures => "hard_failures",
        AuthorityApplyBlockerV1::ObservationGaps => "observation_gaps",
        AuthorityApplyBlockerV1::ExternalActions => "external_actions",
    }
}

pub(super) const fn deployment_execution_status_label(
    status: DeploymentExecutionStatusV1,
) -> &'static str {
    match status {
        DeploymentExecutionStatusV1::NotStarted => "not_started",
        DeploymentExecutionStatusV1::InProgress => "in_progress",
        DeploymentExecutionStatusV1::FailedBeforeMutation => "failed_before_mutation",
        DeploymentExecutionStatusV1::PartiallyApplied => "partially_applied",
        DeploymentExecutionStatusV1::FailedAfterMutation => "failed_after_mutation",
        DeploymentExecutionStatusV1::Complete => "complete",
    }
}

pub(super) fn deployment_command_result_label(result: &DeploymentCommandResultV1) -> String {
    match result {
        DeploymentCommandResultV1::NotFinished => "not_finished".to_string(),
        DeploymentCommandResultV1::Succeeded => "succeeded".to_string(),
        DeploymentCommandResultV1::Failed { code, message } => {
            format!("failed[{code}]: {message}")
        }
    }
}

pub(super) const fn authority_receipt_mutation_label(receipt: &AuthorityReceiptV1) -> &'static str {
    if receipt.attempted_actions.is_empty() {
        "none_attempted"
    } else {
        "attempted_actions_present"
    }
}
