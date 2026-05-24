use super::*;

/// Render an execution preflight as operator text.
#[must_use]
pub fn deployment_execution_preflight_text(preflight: &DeploymentExecutionPreflightV1) -> String {
    let mut lines = vec![
        "Deployment execution preflight".to_string(),
        "mode: passive".to_string(),
        format!(
            "status: {}",
            deployment_execution_preflight_status_label(preflight.status)
        ),
        format!("plan_id: {}", preflight.plan_id),
        format!("safety_report_id: {}", preflight.safety_report_id),
        format!("authority_plan_id: {}", preflight.authority_plan_id),
        format!("backend: {:?}", preflight.backend),
        String::new(),
        "counts:".to_string(),
        format!("  planned_phases: {}", preflight.planned_phases.len()),
        format!(
            "  required_capabilities: {}",
            preflight.required_capabilities.len()
        ),
        format!(
            "  missing_capabilities: {}",
            preflight.missing_capabilities.len()
        ),
        format!("  blockers: {}", preflight.blockers.len()),
    ];

    append_string_items(&mut lines, "planned_phases", &preflight.planned_phases);
    append_capability_items(
        &mut lines,
        "required_capabilities",
        &preflight.required_capabilities,
    );
    append_capability_items(
        &mut lines,
        "missing_capabilities",
        &preflight.missing_capabilities,
    );
    append_hard_failure_items(&mut lines, "blockers", &preflight.blockers);
    lines.join("\n")
}

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

/// Render an authority report as read-only operator text.
#[must_use]
pub fn authority_report_text(report: &AuthorityReportV1) -> String {
    let mut lines = vec![
        "Authority reconciliation report".to_string(),
        "mode: dry_run".to_string(),
        format!("status: {}", safety_status_label(report.status)),
        format!("summary: {}", report.summary),
        format!("report_id: {}", report.report_id),
        format!(
            "check_id: {}",
            report.check_id.as_deref().unwrap_or("not recorded")
        ),
        format!("plan_id: {}", report.reconciliation_plan_id),
        format!("inventory_id: {}", report.inventory_id),
        format!(
            "authority_profile_hash: {}",
            report
                .authority_profile_hash
                .as_deref()
                .unwrap_or("not recorded")
        ),
        String::new(),
        "counts:".to_string(),
        format!("  already_correct: {}", report.counts.already_correct),
        format!(
            "  can_apply_automatically: {}",
            report.counts.can_apply_automatically
        ),
        format!(
            "  requires_external_action: {}",
            report.counts.requires_external_action
        ),
        format!("  unsafe_blocked: {}", report.counts.unsafe_blocked),
        format!("  unknown: {}", report.counts.unknown),
        format!("  hard_failures: {}", report.counts.hard_failures),
        String::new(),
        "apply_readiness:".to_string(),
        format!(
            "  can_apply_automatically: {}",
            report.apply_readiness.can_apply_automatically
        ),
        format!(
            "  automatic_action_count: {}",
            report.apply_readiness.automatic_action_count
        ),
    ];

    append_blockers(&mut lines, report);
    append_next_actions(&mut lines, report);
    append_hard_failure_items(&mut lines, "hard_failures", &report.hard_failures);
    append_observation_gap_items(&mut lines, "observation_gaps", &report.observation_gaps);
    append_authority_action_summary(&mut lines, report);
    lines.join("\n")
}

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
struct AuthorityPlanStateCounts {
    already_correct: usize,
    can_apply_automatically: usize,
    requires_external_action: usize,
    unsafe_blocked: usize,
    unknown: usize,
}

fn append_blockers(lines: &mut Vec<String>, report: &AuthorityReportV1) {
    if report.apply_readiness.blockers.is_empty() {
        lines.push("  blockers: none".to_string());
        return;
    }
    lines.push("  blockers:".to_string());
    for blocker in &report.apply_readiness.blockers {
        lines.push(format!("    - {}", authority_apply_blocker_label(*blocker)));
    }
}

fn append_next_actions(lines: &mut Vec<String>, report: &AuthorityReportV1) {
    if report.next_actions.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("next_actions:".to_string());
    for action in &report.next_actions {
        lines.push(format!("  - {action}"));
    }
}

fn append_observation_gap_items(
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

fn append_hard_failure_items(lines: &mut Vec<String>, label: &str, failures: &[SafetyFindingV1]) {
    if failures.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for failure in failures {
        let subject = failure.subject.as_deref().unwrap_or("unknown subject");
        lines.push(format!(
            "  - [{}] {}: {}",
            failure.code, subject, failure.message
        ));
    }
}

fn append_string_items(lines: &mut Vec<String>, label: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for value in values {
        lines.push(format!("  - {value}"));
    }
}

fn append_capability_items(
    lines: &mut Vec<String>,
    label: &str,
    capabilities: &[DeploymentExecutorCapabilityV1],
) {
    if capabilities.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for capability in capabilities {
        lines.push(format!("  - {capability:?}"));
    }
}

fn append_external_action_items(
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

fn append_controller_observation_items(
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

fn append_authority_action_summary(lines: &mut Vec<String>, report: &AuthorityReportV1) {
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

const fn safety_status_label(status: SafetyStatusV1) -> &'static str {
    match status {
        SafetyStatusV1::NotEvaluated => "not_evaluated",
        SafetyStatusV1::Safe => "safe",
        SafetyStatusV1::Warning => "warning",
        SafetyStatusV1::Blocked => "blocked",
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

const fn deployment_execution_status_label(status: DeploymentExecutionStatusV1) -> &'static str {
    match status {
        DeploymentExecutionStatusV1::NotStarted => "not_started",
        DeploymentExecutionStatusV1::InProgress => "in_progress",
        DeploymentExecutionStatusV1::FailedBeforeMutation => "failed_before_mutation",
        DeploymentExecutionStatusV1::PartiallyApplied => "partially_applied",
        DeploymentExecutionStatusV1::FailedAfterMutation => "failed_after_mutation",
        DeploymentExecutionStatusV1::Complete => "complete",
    }
}

const fn deployment_execution_preflight_status_label(
    status: DeploymentExecutionPreflightStatusV1,
) -> &'static str {
    match status {
        DeploymentExecutionPreflightStatusV1::Ready => "ready",
        DeploymentExecutionPreflightStatusV1::Blocked => "blocked",
    }
}

fn deployment_command_result_label(result: &DeploymentCommandResultV1) -> String {
    match result {
        DeploymentCommandResultV1::NotFinished => "not_finished".to_string(),
        DeploymentCommandResultV1::Succeeded => "succeeded".to_string(),
        DeploymentCommandResultV1::Failed { code, message } => {
            format!("failed[{code}]: {message}")
        }
    }
}

const fn authority_receipt_mutation_label(receipt: &AuthorityReceiptV1) -> &'static str {
    if receipt.attempted_actions.is_empty() {
        "none_attempted"
    } else {
        "attempted_actions_present"
    }
}
