use super::*;

/// Render a cross-deployment comparison report as passive operator text.
#[must_use]
pub fn deployment_comparison_report_text(report: &DeploymentComparisonReportV1) -> String {
    let mut lines = vec![
        "Deployment comparison report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("status: {}", safety_status_label(report.status)),
        format!("report_id: {}", report.report_id),
        format!("report_digest: {}", report.report_digest),
        format!("compared_at: {}", report.compared_at),
        format!(
            "left: {} check={} plan={} inventory={}",
            report.left.label, report.left.check_id, report.left.plan_id, report.left.inventory_id
        ),
        format!(
            "right: {} check={} plan={} inventory={}",
            report.right.label,
            report.right.check_id,
            report.right.plan_id,
            report.right.inventory_id
        ),
        String::new(),
        "counts:".to_string(),
        format!("  identity: {}", report.identity_diff.len()),
        format!("  artifact: {}", report.artifact_diff.len()),
        format!("  module_hash: {}", report.module_hash_diff.len()),
        format!("  embedded_config: {}", report.embedded_config_diff.len()),
        format!("  authority: {}", report.authority_diff.len()),
        format!("  pool: {}", report.pool_diff.len()),
        format!(
            "  verifier_readiness: {}",
            report.verifier_readiness_diff.len()
        ),
        format!(
            "  external_lifecycle: {}",
            report.external_lifecycle_diff.len()
        ),
        format!("  hard_failures: {}", report.hard_failures.len()),
        format!("  warnings: {}", report.warnings.len()),
    ];

    append_comparison_diff_items(&mut lines, "identity_diff", &report.identity_diff);
    append_comparison_diff_items(&mut lines, "artifact_diff", &report.artifact_diff);
    append_comparison_diff_items(&mut lines, "module_hash_diff", &report.module_hash_diff);
    append_comparison_diff_items(
        &mut lines,
        "embedded_config_diff",
        &report.embedded_config_diff,
    );
    append_comparison_diff_items(&mut lines, "authority_diff", &report.authority_diff);
    append_comparison_diff_items(&mut lines, "pool_diff", &report.pool_diff);
    append_comparison_diff_items(
        &mut lines,
        "verifier_readiness_diff",
        &report.verifier_readiness_diff,
    );
    append_comparison_diff_items(
        &mut lines,
        "external_lifecycle_diff",
        &report.external_lifecycle_diff,
    );
    append_hard_failure_items(&mut lines, "hard_failures", &report.hard_failures);
    append_warning_items(&mut lines, "warnings", &report.warnings);
    append_string_items(&mut lines, "next_actions", &report.next_actions);
    lines.join("\n")
}

/// Render a deployment-root verification report as passive operator text.
#[must_use]
pub fn deployment_root_verification_report_text(
    report: &DeploymentRootVerificationReportV1,
) -> String {
    let mut lines = vec![
        "Deployment root verification report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        "local_state_write: none".to_string(),
        format!("evidence_status: {:?}", report.evidence_status),
        format!("state_transition: {:?}", report.state_transition),
        format!("report_id: {}", report.report_id),
        format!("report_digest: {}", report.report_digest),
        format!("requested_at: {}", report.requested_at),
        format!("deployment: {}", report.deployment_name),
        format!("network: {}", report.network),
        format!("fleet_template: {}", report.expected_fleet_template),
        format!("root_principal: {}", report.expected_root_principal),
        format!(
            "observed_root_canister_id: {}",
            report
                .observed_root_canister_id
                .as_deref()
                .unwrap_or("missing")
        ),
        format!(
            "observed_root_observation_source: {}",
            report
                .observed_root_observation_source
                .map_or_else(|| "missing".to_string(), |source| format!("{source:?}"))
        ),
        format!("source_check_id: {}", report.source_check_id),
        format!("source_check_digest: {}", report.source_check_digest),
        format!("source_inventory_id: {}", report.source_inventory_id),
        format!(
            "source_inventory_digest: {}",
            report.source_inventory_digest
        ),
        String::new(),
        "counts:".to_string(),
        format!("  identity_checks: {}", report.identity_checks.len()),
        format!("  evidence_checks: {}", report.evidence_checks.len()),
        format!("  blockers: {}", report.blockers.len()),
        format!("  warnings: {}", report.warnings.len()),
    ];

    append_root_verification_check_items(&mut lines, "identity_checks", &report.identity_checks);
    append_root_verification_check_items(&mut lines, "evidence_checks", &report.evidence_checks);
    append_hard_failure_items(&mut lines, "blockers", &report.blockers);
    append_warning_items(&mut lines, "warnings", &report.warnings);
    append_string_items(
        &mut lines,
        "recommended_next_actions",
        &report.recommended_next_actions,
    );
    lines.join("\n")
}

/// Render a deployment-root verification receipt as operator text.
#[must_use]
pub fn deployment_root_verification_receipt_text(
    receipt: &DeploymentRootVerificationReceiptV1,
) -> String {
    let mut lines = vec![
        "Deployment root verification receipt".to_string(),
        "mode: local-state-write".to_string(),
        "canister_execution: none".to_string(),
        "local_state_write: recorded".to_string(),
        format!("state_transition: {:?}", receipt.state_transition),
        format!("receipt_id: {}", receipt.receipt_id),
        format!("receipt_digest: {}", receipt.receipt_digest),
        format!("deployment: {}", receipt.deployment_name),
        format!("network: {}", receipt.network),
        format!("fleet_template: {}", receipt.fleet_template),
        format!("root_principal: {}", receipt.root_principal),
        format!(
            "previous_root_verification: {:?}",
            receipt.previous_root_verification
        ),
        format!("new_root_verification: {:?}", receipt.new_root_verification),
        format!("source_report_id: {}", receipt.source_report_id),
        format!("source_report_digest: {}", receipt.source_report_digest),
        format!(
            "source_report_requested_at: {}",
            receipt.source_report_requested_at
        ),
        format!("source_report_source: {:?}", receipt.source_report_source),
        format!(
            "source_report_evidence_status: {:?}",
            receipt.source_report_evidence_status
        ),
        format!(
            "source_report_current_root_verification: {:?}",
            receipt.source_report_current_root_verification
        ),
        format!(
            "source_report_state_transition: {:?}",
            receipt.source_report_state_transition
        ),
        format!(
            "source_root_observation_source: {:?}",
            receipt.source_root_observation_source
        ),
        format!(
            "source_observed_root_canister_id: {}",
            receipt.source_observed_root_canister_id
        ),
        format!("source_check_id: {}", receipt.source_check_id),
        format!("source_check_digest: {}", receipt.source_check_digest),
        format!("source_inventory_id: {}", receipt.source_inventory_id),
        format!(
            "source_inventory_digest: {}",
            receipt.source_inventory_digest
        ),
        format!("verified_at_unix_secs: {}", receipt.verified_at_unix_secs),
        format!("local_state_path: {}", receipt.local_state_path),
        format!(
            "local_state_digest_before: {}",
            receipt.local_state_digest_before
        ),
        format!(
            "local_state_digest_after: {}",
            receipt.local_state_digest_after
        ),
    ];

    append_warning_items(&mut lines, "warnings", &receipt.warnings);
    lines.join("\n")
}

/// Render lifecycle authority projection as passive operator text.
#[must_use]
pub fn lifecycle_authority_report_text(report: &LifecycleAuthorityReportV1) -> String {
    let mut lines = vec![
        "Lifecycle authority report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("report_id: {}", report.report_id),
        format!("report_digest: {}", report.report_digest),
        format!("check_id: {}", report.check_id),
        format!("plan_id: {}", report.plan_id),
        format!("inventory_id: {}", report.inventory_id),
        String::new(),
        "counts:".to_string(),
        format!("  authorities: {}", report.authorities.len()),
        format!(
            "  external_action_required: {}",
            report.external_action_required_count
        ),
        format!("  blocked: {}", report.blocked_count),
    ];

    append_lifecycle_authority_items(&mut lines, &report.authorities);
    lines.join("\n")
}

fn append_root_verification_check_items(
    lines: &mut Vec<String>,
    label: &str,
    items: &[DeploymentRootVerificationCheckV1],
) {
    if items.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for item in items {
        lines.push(format!(
            "  - {} expected={} observed={} satisfied={}",
            item.name,
            item.expected.as_deref().unwrap_or("missing"),
            item.observed.as_deref().unwrap_or("missing"),
            item.satisfied
        ));
    }
}

fn append_comparison_diff_items(
    lines: &mut Vec<String>,
    label: &str,
    items: &[DeploymentComparisonDiffV1],
) {
    if items.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for item in items {
        lines.push(format!(
            "  - {:?}: {} left={} right={} severity={:?}",
            item.category,
            item.subject,
            item.left.as_deref().unwrap_or("missing"),
            item.right.as_deref().unwrap_or("missing"),
            item.severity
        ));
        lines.push(format!("    {}", item.message));
    }
}

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

/// Render an external-upgrade proposal report as passive operator text.
#[must_use]
pub fn external_upgrade_proposal_report_text(report: &ExternalUpgradeProposalReportV1) -> String {
    let mut lines = vec![
        "External upgrade proposal report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("report_id: {}", report.report_id),
        format!("report_digest: {}", report.report_digest),
        format!("lifecycle_plan_id: {}", report.lifecycle_plan_id),
        format!("lifecycle_plan_digest: {}", report.lifecycle_plan_digest),
        format!("deployment_plan_id: {}", report.deployment_plan_id),
        format!("deployment_plan_digest: {}", report.deployment_plan_digest),
        format!("inventory_id: {}", report.inventory_id),
        String::new(),
        "counts:".to_string(),
        format!("  proposals: {}", report.proposals.len()),
        format!("  blocked_subjects: {}", report.blocked_subjects.len()),
    ];

    append_external_upgrade_proposal_items(&mut lines, &report.proposals);
    append_string_items(&mut lines, "blocked_subjects", &report.blocked_subjects);
    lines.join("\n")
}

/// Render an external lifecycle pending report as passive operator text.
#[must_use]
pub fn external_lifecycle_pending_report_text(report: &ExternalLifecyclePendingReportV1) -> String {
    let mut lines = vec![
        "External lifecycle pending report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!(
            "status: {}",
            external_lifecycle_plan_status_label(report.status)
        ),
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
        format!(
            "status: {}",
            external_lifecycle_plan_status_label(check.status)
        ),
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
        format!(
            "status: {}",
            external_lifecycle_plan_status_label(handoff.status)
        ),
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

/// Render an external-upgrade receipt as passive operator text.
#[must_use]
pub fn external_upgrade_receipt_text(receipt: &ExternalUpgradeReceiptV1) -> String {
    [
        "External upgrade receipt".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("receipt_id: {}", receipt.receipt_id),
        format!("receipt_digest: {}", receipt.receipt_digest),
        format!("proposal_id: {}", receipt.proposal_id),
        format!("proposal_digest: {}", receipt.proposal_digest),
        format!("subject: {}", receipt.subject),
        format!("role: {}", optional_text(receipt.role.as_deref())),
        format!(
            "canister_id: {}",
            optional_text(receipt.canister_id.as_deref())
        ),
        format!(
            "consent_state: {}",
            external_upgrade_consent_state_label(receipt.consent_state)
        ),
        format!(
            "verification_result: {}",
            external_upgrade_verification_result_label(receipt.verification_result)
        ),
        format!(
            "reported_by: {}",
            optional_text(receipt.reported_by.as_deref())
        ),
        format!(
            "observed_before_module_hash: {}",
            optional_text(receipt.observed_before_module_hash.as_deref())
        ),
        format!(
            "observed_after_module_hash: {}",
            optional_text(receipt.observed_after_module_hash.as_deref())
        ),
        format!("verification_notes: {}", receipt.verification_notes.len()),
    ]
    .join("\n")
}

/// Render external-upgrade consent evidence as passive operator text.
#[must_use]
pub fn external_upgrade_consent_evidence_text(
    evidence: &ExternalUpgradeConsentEvidenceV1,
) -> String {
    [
        "External upgrade consent evidence".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("evidence_id: {}", evidence.evidence_id),
        format!("evidence_digest: {}", evidence.evidence_digest),
        format!("proposal_id: {}", evidence.proposal_id),
        format!("proposal_digest: {}", evidence.proposal_digest),
        format!("receipt_id: {}", evidence.receipt_id),
        format!("receipt_digest: {}", evidence.receipt_digest),
        format!("subject: {}", evidence.subject),
        format!("role: {}", optional_text(evidence.role.as_deref())),
        format!(
            "canister_id: {}",
            optional_text(evidence.canister_id.as_deref())
        ),
        format!(
            "consent_state: {}",
            external_upgrade_consent_state_label(evidence.consent_state)
        ),
        format!(
            "reported_by: {}",
            optional_text(evidence.reported_by.as_deref())
        ),
        format!("status_summary: {}", evidence.status_summary),
        format!(
            "consent_requirements: {}",
            evidence.consent_requirements.len()
        ),
        format!(
            "allowed_authorization_modes: {}",
            evidence.allowed_authorization_modes.len()
        ),
    ]
    .join("\n")
}

/// Render an external-upgrade verification report as passive operator text.
#[must_use]
pub fn external_upgrade_verification_report_text(
    report: &ExternalUpgradeVerificationReportV1,
) -> String {
    let mut lines = vec![
        "External upgrade verification report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("report_id: {}", report.report_id),
        format!("report_digest: {}", report.report_digest),
        format!("proposal_id: {}", report.proposal_id),
        format!("proposal_digest: {}", report.proposal_digest),
        format!("receipt_id: {}", report.receipt_id),
        format!("receipt_digest: {}", report.receipt_digest),
        format!("subject: {}", report.subject),
        format!("role: {}", optional_text(report.role.as_deref())),
        format!(
            "canister_id: {}",
            optional_text(report.canister_id.as_deref())
        ),
        format!(
            "verification_result: {}",
            external_upgrade_verification_result_label(report.verification_result)
        ),
        format!(
            "live_inventory_required: {}",
            report.live_inventory_required
        ),
        format!("status_summary: {}", report.status_summary),
        format!("verification_notes: {}", report.verification_notes.len()),
    ];
    append_string_items(&mut lines, "verification_notes", &report.verification_notes);
    lines.join("\n")
}

/// Render an external-upgrade verification policy as passive operator text.
#[must_use]
pub fn external_upgrade_verification_policy_text(
    policy: &ExternalUpgradeVerificationPolicyV1,
) -> String {
    let mut lines = vec![
        "External upgrade verification policy".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("policy_id: {}", policy.policy_id),
        format!("policy_digest: {}", policy.policy_digest),
        format!("proposal_id: {}", policy.proposal_id),
        format!("proposal_digest: {}", policy.proposal_digest),
        format!("subject: {}", policy.subject),
        format!("role: {}", optional_text(policy.role.as_deref())),
        format!(
            "canister_id: {}",
            optional_text(policy.canister_id.as_deref())
        ),
        format!("summary: {}", policy.status_summary),
        String::new(),
        format!(
            "max_observation_age_seconds: {}",
            policy
                .max_observation_age_seconds
                .map_or_else(|| "none".to_string(), |value| value.to_string())
        ),
    ];
    append_verification_policy_requirement_items(&mut lines, &policy.verification_requirements);
    lines.join("\n")
}

/// Render an external-upgrade verification check as passive operator text.
#[must_use]
pub fn external_upgrade_verification_check_text(
    check: &ExternalUpgradeVerificationCheckV1,
) -> String {
    let mut lines = vec![
        "External upgrade verification check".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        "live_lookup: none".to_string(),
        format!("check_id: {}", check.check_id),
        format!("check_digest: {}", check.check_digest),
        format!("policy_id: {}", check.policy_id),
        format!("policy_digest: {}", check.policy_digest),
        format!("proposal_id: {}", check.proposal_id),
        format!("proposal_digest: {}", check.proposal_digest),
        format!("subject: {}", check.subject),
        format!("role: {}", optional_text(check.role.as_deref())),
        format!(
            "canister_id: {}",
            optional_text(check.canister_id.as_deref())
        ),
        format!(
            "verification_result: {}",
            external_upgrade_verification_result_label(check.verification_result)
        ),
        format!("summary: {}", check.status_summary),
        String::new(),
        format!(
            "observation.source: {}",
            external_verification_observation_source_label(check.observation.source)
        ),
        format!(
            "observation.deployment_check_id: {}",
            optional_text(check.observation.deployment_check_id.as_deref())
        ),
        format!(
            "observation.deployment_check_digest: {}",
            optional_text(check.observation.deployment_check_digest.as_deref())
        ),
        format!(
            "observation.inventory_id: {}",
            optional_text(check.observation.inventory_id.as_deref())
        ),
        format!(
            "observation.observed_at: {}",
            optional_text(check.observation.observed_at.as_deref())
        ),
        format!(
            "observation.observed_control_class: {}",
            check
                .observation
                .observed_control_class
                .map_or_else(|| "none".to_string(), |value| format!("{value:?}"))
        ),
    ];
    append_verification_check_requirement_items(&mut lines, &check.requirement_results);
    lines.join("\n")
}

/// Render an external-upgrade completion report as passive operator text.
#[must_use]
pub fn external_upgrade_completion_report_text(
    report: &ExternalUpgradeCompletionReportV1,
) -> String {
    let mut lines = vec![
        "External upgrade completion report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        "live_lookup: none".to_string(),
        format!("report_id: {}", report.report_id),
        format!("report_digest: {}", report.report_digest),
        format!("proposal_id: {}", report.proposal_id),
        format!("proposal_digest: {}", report.proposal_digest),
        format!("consent_evidence_id: {}", report.consent_evidence_id),
        format!("verification_check_id: {}", report.verification_check_id),
        format!("subject: {}", report.subject),
        format!("role: {}", optional_text(report.role.as_deref())),
        format!(
            "canister_id: {}",
            optional_text(report.canister_id.as_deref())
        ),
        format!(
            "consent_state: {}",
            external_upgrade_consent_state_label(report.consent_state)
        ),
        format!(
            "verification_result: {}",
            external_upgrade_verification_result_label(report.verification_result)
        ),
        format!(
            "verification_observation_source: {}",
            external_verification_observation_source_label(report.verification_observation_source)
        ),
        format!(
            "completion_status: {}",
            external_upgrade_completion_status_label(report.completion_status)
        ),
        format!("summary: {}", report.status_summary),
    ];
    append_string_items(&mut lines, "blockers", &report.blockers);
    append_string_items(&mut lines, "next_actions", &report.next_actions);
    lines.join("\n")
}

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

/// Render source/build materialization evidence as passive operator text.
#[must_use]
pub fn build_materialization_evidence_text(evidence: &BuildMaterializationEvidenceV1) -> String {
    [
        "Build materialization evidence".to_string(),
        "mode: passive".to_string(),
        format!("evidence_id: {}", evidence.evidence_id),
        format!(
            "materialization_evidence_digest: {}",
            evidence.materialization_evidence_digest
        ),
        format!("recipe_id: {}", evidence.recipe.recipe_id),
        format!(
            "materialization_input_id: {}",
            evidence.materialization_input.materialization_input_id
        ),
        format!(
            "materialization_result_id: {}",
            evidence.materialization_result.materialization_result_id
        ),
        format!(
            "computed_materialization_input_digest: {}",
            evidence.computed_materialization_input_digest
        ),
        format!(
            "recipe_id_matches_input: {}",
            evidence.recipe_id_matches_input
        ),
        format!(
            "recipe_id_matches_result: {}",
            evidence.recipe_id_matches_result
        ),
        format!(
            "materialization_input_digest_matches_result: {}",
            evidence.materialization_input_digest_matches_result
        ),
        "execution: none".to_string(),
    ]
    .join("\n")
}

/// Render source/build materialization identity as passive operator text.
#[must_use]
pub fn promotion_materialization_identity_report_text(
    report: &PromotionMaterializationIdentityReportV1,
) -> String {
    let mut lines = vec![
        "Promotion materialization identity report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!(
            "status: {}",
            promotion_readiness_status_label(report.status)
        ),
        format!("report_id: {}", report.report_id),
        format!(
            "materialization_identity_report_digest: {}",
            report.materialization_identity_report_digest
        ),
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", report.roles.len()),
        format!("  output_groups: {}", report.output_groups.len()),
        format!("  blockers: {}", report.blockers.len()),
    ];

    append_hard_failure_items(&mut lines, "blockers", &report.blockers);
    if !report.output_groups.is_empty() {
        lines.push(String::new());
        lines.push("output groups:".to_string());
        for group in &report.output_groups {
            lines.push(format!(
                "  {} roles={} wasm={} installed={}",
                group.output_identity_key,
                group.roles.join(","),
                group.wasm_sha256,
                group.installed_module_hash
            ));
        }
    }
    if !report.roles.is_empty() {
        lines.push(String::new());
        lines.push("roles:".to_string());
        for role in &report.roles {
            lines.push(format!(
                "  {} evidence={} recipe={} input={} result={} network={} runtime={}",
                role.role,
                role.evidence_id,
                role.recipe_id,
                role.materialization_input_id,
                role.materialization_result_id,
                role.network,
                role.runtime_variant
            ));
        }
    }
    lines.join("\n")
}

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

/// Render a promotion artifact identity report as passive operator text.
#[must_use]
pub fn promotion_artifact_identity_report_text(
    report: &PromotionArtifactIdentityReportV1,
) -> String {
    let mut lines = vec![
        "Promotion artifact identity report".to_string(),
        "mode: passive".to_string(),
        format!(
            "status: {}",
            promotion_readiness_status_label(report.status)
        ),
        format!("report_id: {}", report.report_id),
        format!(
            "artifact_identity_report_digest: {}",
            report.artifact_identity_report_digest
        ),
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", report.summary.role_count),
        format!("  identity_groups: {}", report.summary.identity_group_count),
        format!(
            "  shared_identity_groups: {}",
            report.summary.shared_identity_group_count
        ),
        format!(
            "  digest_pinned_roles: {}",
            report.summary.digest_pinned_role_count
        ),
        format!(
            "  source_build_roles: {}",
            report.summary.source_build_role_count
        ),
        format!(
            "  deferred_identity_roles: {}",
            report.summary.deferred_identity_role_count
        ),
        format!("  blockers: {}", report.blockers.len()),
    ];

    append_promotion_artifact_identity_group_items(&mut lines, &report.identity_groups);
    append_promotion_artifact_identity_role_items(&mut lines, &report.roles);
    append_hard_failure_items(&mut lines, "blockers", &report.blockers);
    lines.join("\n")
}

/// Render a wasm-store identity report as passive operator text.
#[must_use]
pub fn promotion_wasm_store_identity_report_text(
    report: &PromotionWasmStoreIdentityReportV1,
) -> String {
    let mut lines = vec![
        "Promotion wasm-store identity report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!(
            "status: {}",
            promotion_readiness_status_label(report.status)
        ),
        format!("report_id: {}", report.report_id),
        format!(
            "wasm_store_identity_report_digest: {}",
            report.wasm_store_identity_report_digest
        ),
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", report.roles.len()),
        format!("  blockers: {}", report.blockers.len()),
    ];

    append_hard_failure_items(&mut lines, "blockers", &report.blockers);
    if !report.roles.is_empty() {
        lines.push(String::new());
        lines.push("roles:".to_string());
        for role in &report.roles {
            lines.push(format!(
                "  {} artifact={} locator={} chunks={}/{} postcondition={:?}",
                role.role,
                role.artifact_identity,
                role.wasm_store_locator.as_deref().unwrap_or("none"),
                role.published_chunk_count,
                role.prepared_chunk_hashes.len(),
                role.verified_postcondition.status
            ));
        }
    }
    lines.join("\n")
}

/// Render a wasm-store catalog verification report as passive operator text.
#[must_use]
pub fn promotion_wasm_store_catalog_verification_text(
    verification: &PromotionWasmStoreCatalogVerificationV1,
) -> String {
    let matching_roles = verification
        .roles
        .iter()
        .filter(|role| role.catalog_matches)
        .count();
    let missing_roles = verification
        .roles
        .iter()
        .filter(|role| !role.catalog_entry_present)
        .count();
    let mut lines = vec![
        "Promotion wasm-store catalog verification".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!(
            "status: {}",
            promotion_readiness_status_label(verification.status)
        ),
        format!("verification_id: {}", verification.verification_id),
        format!(
            "wasm_store_catalog_verification_digest: {}",
            verification.wasm_store_catalog_verification_digest
        ),
        format!(
            "wasm_store_identity_report_id: {}",
            verification.wasm_store_identity_report_id
        ),
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", verification.roles.len()),
        format!("  matching_roles: {matching_roles}"),
        format!("  missing_catalog_entries: {missing_roles}"),
        format!("  blockers: {}", verification.blockers.len()),
    ];

    append_hard_failure_items(&mut lines, "blockers", &verification.blockers);
    if !verification.roles.is_empty() {
        lines.push(String::new());
        lines.push("roles:".to_string());
        for role in &verification.roles {
            lines.push(format!(
                "  {} locator={} match={} digest={} expected_artifact={} observed_artifact={}",
                role.role,
                role.wasm_store_locator,
                role.catalog_matches,
                role.catalog_observation_digest,
                role.expected_artifact_identity,
                role.observed_artifact_identity.as_deref().unwrap_or("none")
            ));
        }
    }
    lines.join("\n")
}

/// Render a promotion plan transform as passive operator text.
#[must_use]
pub fn promotion_plan_transform_text(transform: &PromotionPlanTransformV1) -> String {
    let changed_artifacts = transform
        .roles
        .iter()
        .filter(|role| role.artifact_identity_changed)
        .count();
    let changed_configs = transform
        .roles
        .iter()
        .filter(|role| role.embedded_config_changed)
        .count();
    let preserved_materializations = transform
        .roles
        .iter()
        .filter(|role| role.target_materialization_preserved)
        .count();
    let mut lines = vec![
        "Promotion plan transform".to_string(),
        "mode: passive".to_string(),
        format!("transform_id: {}", transform.transform_id),
        format!("target_plan_id: {}", transform.target_plan_id),
        format!("promoted_plan_id: {}", transform.promoted_plan_id),
        format!(
            "promotion_plan_lineage_digest: {}",
            transform.promotion_plan_lineage_digest
        ),
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", transform.roles.len()),
        format!("  artifact_identity_changed: {changed_artifacts}"),
        format!("  embedded_config_changed: {changed_configs}"),
        format!("  target_materialization_preserved: {preserved_materializations}"),
    ];

    append_promotion_transform_role_items(&mut lines, &transform.roles);
    lines.join("\n")
}

/// Render promotion transform evidence as passive operator text.
#[must_use]
pub fn promotion_plan_transform_evidence_text(
    evidence: &PromotionPlanTransformEvidenceV1,
) -> String {
    let mut lines = vec![
        "Promotion plan transform evidence".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("evidence_id: {}", evidence.evidence_id),
        format!(
            "promotion_plan_transform_evidence_digest: {}",
            evidence.promotion_plan_transform_evidence_digest
        ),
        format!("generated_at: {}", evidence.generated_at),
        format!("transform_id: {}", evidence.transform.transform_id),
        format!("target_plan_id: {}", evidence.transform.target_plan_id),
        format!("promoted_plan_id: {}", evidence.transform.promoted_plan_id),
        String::new(),
        "transform:".to_string(),
    ];

    lines.extend(
        promotion_plan_transform_text(&evidence.transform)
            .lines()
            .map(|line| format!("  {line}")),
    );
    lines.join("\n")
}

/// Render target execution lineage as passive operator text.
#[must_use]
pub fn promotion_target_execution_lineage_text(
    lineage: &PromotionTargetExecutionLineageV1,
) -> String {
    let mut lines = vec![
        "Promotion target execution lineage".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("lineage_id: {}", lineage.lineage_id),
        format!("generated_at: {}", lineage.generated_at),
        format!(
            "target_execution_lineage_digest: {}",
            lineage.target_execution_lineage_digest
        ),
        format!("transform_id: {}", lineage.transform.transform_id),
        format!("target_plan_id: {}", lineage.transform.target_plan_id),
        format!("promoted_plan_id: {}", lineage.transform.promoted_plan_id),
        format!("preflight_plan_id: {}", lineage.execution_preflight.plan_id),
        format!(
            "preflight_safety_report_id: {}",
            lineage.execution_preflight.safety_report_id
        ),
        format!(
            "preflight_authority_plan_id: {}",
            lineage.execution_preflight.authority_plan_id
        ),
        format!("backend: {:?}", lineage.execution_preflight.backend),
        format!("preflight_status: {:?}", lineage.execution_preflight.status),
        format!("execution_attempted: {}", lineage.execution_attempted),
    ];

    lines.push(String::new());
    lines.push("promotion_plan:".to_string());
    lines.extend(
        promotion_plan_transform_text(&lineage.transform)
            .lines()
            .map(|line| format!("  {line}")),
    );
    lines.push(String::new());
    lines.push("execution_preflight:".to_string());
    lines.extend(
        deployment_execution_preflight_text(&lineage.execution_preflight)
            .lines()
            .map(|line| format!("  {line}")),
    );
    lines.join("\n")
}

/// Render an artifact promotion plan as passive operator text.
#[must_use]
pub fn artifact_promotion_plan_text(plan: &ArtifactPromotionPlanV1) -> String {
    let mut lines = vec![
        "Artifact promotion plan".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("plan_id: {}", plan.plan_id),
        format!(
            "artifact_promotion_plan_digest: {}",
            plan.artifact_promotion_plan_digest
        ),
        format!("generated_at: {}", plan.generated_at),
        format!("status: {:?}", plan.status),
        format!("target_plan_id: {}", plan.target_plan_id),
        format!("promoted_plan_id: {}", plan.promoted_plan_id),
        format!(
            "promotion_plan_lineage_digest: {}",
            plan.promotion_plan_lineage_digest
        ),
        format!(
            "target_execution_lineage: {}",
            plan.target_execution_lineage
                .as_ref()
                .map_or("none", |lineage| lineage.lineage_id.as_str())
        ),
        String::new(),
        "counts:".to_string(),
        format!("  readiness_roles: {}", plan.readiness.roles.len()),
        format!(
            "  artifact_identity_roles: {}",
            plan.artifact_identity_report.roles.len()
        ),
        format!("  transform_roles: {}", plan.transform.roles.len()),
        format!("  blockers: {}", plan.blockers.len()),
    ];

    append_hard_failure_items(&mut lines, "blockers", &plan.blockers);
    lines.push(String::new());
    lines.push("readiness:".to_string());
    lines.extend(
        promotion_readiness_text(&plan.readiness)
            .lines()
            .map(|line| format!("  {line}")),
    );
    lines.push(String::new());
    lines.push("artifact_identity:".to_string());
    lines.extend(
        promotion_artifact_identity_report_text(&plan.artifact_identity_report)
            .lines()
            .map(|line| format!("  {line}")),
    );
    lines.push(String::new());
    lines.push("transform:".to_string());
    lines.extend(
        promotion_plan_transform_text(&plan.transform)
            .lines()
            .map(|line| format!("  {line}")),
    );
    if let Some(lineage) = &plan.target_execution_lineage {
        lines.push(String::new());
        lines.push("target_execution_lineage:".to_string());
        lines.extend(
            promotion_target_execution_lineage_text(lineage)
                .lines()
                .map(|line| format!("  {line}")),
        );
    }
    lines.join("\n")
}

/// Render artifact promotion provenance as passive operator text.
#[must_use]
pub fn artifact_promotion_provenance_report_text(
    report: &ArtifactPromotionProvenanceReportV1,
) -> String {
    let mut lines = vec![
        "Artifact promotion provenance report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!(
            "status: {}",
            promotion_readiness_status_label(report.status)
        ),
        format!("report_id: {}", report.report_id),
        format!(
            "artifact_promotion_plan_id: {}",
            report.artifact_promotion_plan_id
        ),
        format!(
            "artifact_promotion_plan_digest: {}",
            report.artifact_promotion_plan_digest
        ),
        format!("promoted_plan_id: {}", report.promoted_plan_id),
        format!(
            "promotion_plan_lineage_digest: {}",
            report.promotion_plan_lineage_digest
        ),
        format!(
            "provenance_report_digest: {}",
            report.provenance_report_digest
        ),
    ];
    append_artifact_promotion_provenance_linked_reports(&mut lines, report);
    lines.extend([
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", report.roles.len()),
        format!("  blockers: {}", report.blockers.len()),
    ]);

    append_hard_failure_items(&mut lines, "blockers", &report.blockers);
    if !report.roles.is_empty() {
        lines.push(String::new());
        lines.push("roles:".to_string());
        for role in &report.roles {
            lines.push(format!(
                "  {} {:?}/{:?}: materialization={} materialization_digest={} wasm_store={} catalog_digest={}",
                role.role,
                role.promotion_level,
                role.source_kind,
                role.materialization_evidence_id
                    .as_deref()
                    .unwrap_or("none"),
                role.materialization_evidence_digest
                    .as_deref()
                    .unwrap_or("none"),
                role.wasm_store_locator.as_deref().unwrap_or("none"),
                role.wasm_store_catalog_observation_digest
                    .as_deref()
                    .unwrap_or("none")
            ));
        }
    }
    lines.join("\n")
}

fn append_artifact_promotion_provenance_linked_reports(
    lines: &mut Vec<String>,
    report: &ArtifactPromotionProvenanceReportV1,
) {
    lines.extend([
        String::new(),
        "linked reports:".to_string(),
        format!("  readiness: {}", report.readiness_id),
        format!(
            "  artifact_identity: {}",
            report.artifact_identity_report_id
        ),
        format!("  transform: {}", report.transform_id),
        format!(
            "  target_execution_lineage: {}",
            report
                .target_execution_lineage_id
                .as_deref()
                .unwrap_or("none")
        ),
        format!(
            "  wasm_store_identity: {}",
            report
                .wasm_store_identity_report_id
                .as_deref()
                .unwrap_or("none")
        ),
        format!(
            "  wasm_store_identity_digest: {}",
            report
                .wasm_store_identity_report_digest
                .as_deref()
                .unwrap_or("none")
        ),
        format!(
            "  wasm_store_catalog: {}",
            report
                .wasm_store_catalog_verification_id
                .as_deref()
                .unwrap_or("none")
        ),
        format!(
            "  wasm_store_catalog_digest: {}",
            report
                .wasm_store_catalog_verification_digest
                .as_deref()
                .unwrap_or("none")
        ),
        format!(
            "  materialization_identity: {}",
            report
                .materialization_identity_report_id
                .as_deref()
                .unwrap_or("none")
        ),
        format!(
            "  materialization_identity_digest: {}",
            report
                .materialization_identity_report_digest
                .as_deref()
                .unwrap_or("none")
        ),
    ]);
}

/// Render artifact promotion execution receipt linkage as operator text.
#[must_use]
pub fn artifact_promotion_execution_receipt_text(
    receipt: &ArtifactPromotionExecutionReceiptV1,
) -> String {
    let mut lines = vec![
        "Artifact promotion execution receipt".to_string(),
        "mode: execution_receipt".to_string(),
        format!("receipt_id: {}", receipt.receipt_id),
        format!(
            "execution_receipt_digest: {}",
            receipt.execution_receipt_digest
        ),
        format!(
            "artifact_promotion_plan_id: {}",
            receipt.artifact_promotion_plan_id
        ),
        format!(
            "artifact_promotion_plan_digest: {}",
            receipt.artifact_promotion_plan_digest
        ),
        format!("provenance_report_id: {}", receipt.provenance_report_id),
        format!(
            "provenance_report_digest: {}",
            receipt.provenance_report_digest
        ),
        format!("promoted_plan_id: {}", receipt.promoted_plan_id),
        format!(
            "promotion_plan_lineage_digest: {}",
            receipt.promotion_plan_lineage_digest
        ),
        format!("operation_id: {}", receipt.operation_id),
        format!(
            "provenance_status: {}",
            promotion_readiness_status_label(receipt.provenance_status)
        ),
        format!("operation_status: {:?}", receipt.operation_status),
        format!("command_result: {:?}", receipt.command_result),
        format!("started_at: {}", receipt.started_at),
        format!(
            "finished_at: {}",
            receipt.finished_at.as_deref().unwrap_or("none")
        ),
        String::new(),
        "counts:".to_string(),
        format!("  roles: {}", receipt.roles.len()),
        format!(
            "  deployment_phase_receipts: {}",
            receipt.deployment_receipt.phase_receipts.len()
        ),
        format!(
            "  deployment_role_phase_receipts: {}",
            receipt.deployment_receipt.role_phase_receipts.len()
        ),
    ];

    if !receipt.roles.is_empty() {
        lines.push(String::new());
        lines.push("roles:".to_string());
        for role in &receipt.roles {
            lines.push(format!(
                "  {} {:?}: result={} artifact={} observed_module={} materialization_digest={} catalog_digest={}",
                role.role,
                role.promotion_level,
                role.role_phase_result
                    .map_or_else(|| "none".to_string(), |result| format!("{result:?}")),
                role.artifact_digest.as_deref().unwrap_or("none"),
                role.observed_module_hash_after.as_deref().unwrap_or("none"),
                role.materialization_evidence_digest
                    .as_deref()
                    .unwrap_or("none"),
                role.wasm_store_catalog_observation_digest
                    .as_deref()
                    .unwrap_or("none")
            ));
        }
    }
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

fn append_warning_items(lines: &mut Vec<String>, label: &str, warnings: &[SafetyFindingV1]) {
    if warnings.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for warning in warnings {
        let subject = warning.subject.as_deref().unwrap_or("unknown subject");
        lines.push(format!(
            "  - [{}] {}: {}",
            warning.code, subject, warning.message
        ));
    }
}

fn append_promotion_role_items(lines: &mut Vec<String>, roles: &[RolePromotionReadinessV1]) {
    if roles.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("roles:".to_string());
    for role in roles {
        lines.push(format!(
            "  - {} {:?}/{:?}: byte_identical_wasm={} embedded_config_identical={} restage_required={}",
            role.role,
            role.promotion_level,
            role.source_kind,
            optional_bool_label(role.byte_identical_wasm),
            optional_bool_label(role.embedded_config_identical),
            role.restage_required
        ));
        lines.push(format!(
            "    source_wasm_gz_sha256: {}",
            role.source_wasm_gz_sha256
                .as_deref()
                .unwrap_or("not recorded")
        ));
        lines.push(format!(
            "    target_wasm_gz_sha256: {}",
            role.target_wasm_gz_sha256
                .as_deref()
                .unwrap_or("not recorded")
        ));
        lines.push(format!(
            "    source_config_sha256: {}",
            role.source_canonical_embedded_config_sha256
                .as_deref()
                .unwrap_or("not recorded")
        ));
        lines.push(format!(
            "    target_config_sha256: {}",
            role.target_canonical_embedded_config_sha256
                .as_deref()
                .unwrap_or("not recorded")
        ));
    }
}

fn append_promotion_artifact_identity_role_items(
    lines: &mut Vec<String>,
    roles: &[RolePromotionArtifactIdentityV1],
) {
    if roles.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("roles:".to_string());
    for role in roles {
        lines.push(format!(
            "  - {} {:?}/{:?}: identity_kind={:?} digest_pinned={}",
            role.role,
            role.promotion_level,
            role.source_kind,
            role.identity_kind,
            role.digest_pinned
        ));
        lines.push(format!(
            "    source_locator: {}",
            role.source_locator.as_deref().unwrap_or("not recorded")
        ));
        lines.push(format!(
            "    wasm_gz_sha256: {}",
            role.wasm_gz_sha256.as_deref().unwrap_or("not recorded")
        ));
        lines.push(format!(
            "    config_sha256: {}",
            role.canonical_embedded_config_sha256
                .as_deref()
                .unwrap_or("not recorded")
        ));
    }
}

fn append_promotion_artifact_identity_group_items(
    lines: &mut Vec<String>,
    groups: &[PromotionArtifactIdentityGroupV1],
) {
    if groups.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("identity groups:".to_string());
    for group in groups {
        lines.push(format!(
            "  - {}: kind={:?} source_kinds={} roles={}",
            group.identity_key,
            group.identity_kind,
            group
                .source_kinds
                .iter()
                .map(|kind| format!("{kind:?}"))
                .collect::<Vec<_>>()
                .join(","),
            group.roles.join(",")
        ));
    }
}

fn append_promotion_policy_decision_items(
    lines: &mut Vec<String>,
    roles: &[RolePromotionPolicyDecisionV1],
) {
    if roles.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("roles:".to_string());
    for role in roles {
        lines.push(format!(
            "  - {} {:?}: policy_satisfied={} level_allowed={} requirements={} claims={}",
            role.role,
            role.requested_promotion_level,
            role.policy_satisfied,
            role.level_allowed,
            role.requirements
                .iter()
                .map(|requirement| format!("{requirement:?}"))
                .collect::<Vec<_>>()
                .join(","),
            role.claims
                .iter()
                .map(|claim| format!("{claim:?}"))
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
}

fn append_promotion_transform_role_items(
    lines: &mut Vec<String>,
    roles: &[RolePromotionPlanTransformV1],
) {
    if roles.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("roles:".to_string());
    for role in roles {
        lines.push(format!(
            "  - {} {:?}/{:?}: artifact_identity_changed={} embedded_config_changed={} target_materialization_preserved={}",
            role.role,
            role.promotion_level,
            role.source_kind,
            role.artifact_identity_changed,
            role.embedded_config_changed,
            role.target_materialization_preserved
        ));
        lines.push(format!(
            "    artifact_source: {:?} -> {:?}",
            role.artifact_source_before, role.artifact_source_after
        ));
        lines.push(format!(
            "    wasm_gz_sha256: {} -> {}",
            role.wasm_gz_sha256_before
                .as_deref()
                .unwrap_or("not recorded"),
            role.wasm_gz_sha256_after
                .as_deref()
                .unwrap_or("not recorded")
        ));
        lines.push(format!(
            "    config_sha256: {} -> {}",
            role.canonical_embedded_config_sha256_before
                .as_deref()
                .unwrap_or("not recorded"),
            role.canonical_embedded_config_sha256_after
                .as_deref()
                .unwrap_or("not recorded")
        ));
        if let Some(materialization) = &role.source_build_materialization {
            lines.push(format!(
                "    materialization_evidence_id: {}",
                materialization.evidence_id
            ));
            lines.push(format!(
                "    materialization_evidence_digest: {}",
                materialization.materialization_evidence_digest
            ));
            lines.push(format!(
                "    materialization_input_digest: {}",
                materialization.materialization_input_digest
            ));
            lines.push(format!(
                "    materialized_wasm_gz_sha256: {}",
                materialization.wasm_gz_sha256
            ));
        }
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

const fn promotion_readiness_status_label(status: PromotionReadinessStatusV1) -> &'static str {
    match status {
        PromotionReadinessStatusV1::Ready => "ready",
        PromotionReadinessStatusV1::Blocked => "blocked",
    }
}

const fn external_lifecycle_plan_status_label(
    status: ExternalLifecyclePlanStatusV1,
) -> &'static str {
    match status {
        ExternalLifecyclePlanStatusV1::Ready => "ready",
        ExternalLifecyclePlanStatusV1::PendingExternalAction => "pending_external_action",
        ExternalLifecyclePlanStatusV1::Blocked => "blocked",
    }
}

const fn lifecycle_mode_label(mode: LifecycleModeV1) -> &'static str {
    match mode {
        LifecycleModeV1::DirectDeploymentAuthority => "direct_deployment_authority",
        LifecycleModeV1::ProposalRequired => "proposal_required",
        LifecycleModeV1::DelegatedInstallRequired => "delegated_install_required",
        LifecycleModeV1::ExternalCompletionOnly => "external_completion_only",
        LifecycleModeV1::VerifyOnly => "verify_only",
        LifecycleModeV1::MustNotTouch => "must_not_touch",
        LifecycleModeV1::UnknownUnsafeBlocked => "unknown_unsafe_blocked",
    }
}

const fn external_upgrade_consent_state_label(
    state: ExternalUpgradeConsentStateV1,
) -> &'static str {
    match state {
        ExternalUpgradeConsentStateV1::Pending => "pending",
        ExternalUpgradeConsentStateV1::Refused => "refused",
        ExternalUpgradeConsentStateV1::Delegated => "delegated",
        ExternalUpgradeConsentStateV1::ExecutedExternally => "executed_externally",
    }
}

const fn external_upgrade_verification_result_label(
    result: ExternalUpgradeVerificationResultV1,
) -> &'static str {
    match result {
        ExternalUpgradeVerificationResultV1::Pending => "pending",
        ExternalUpgradeVerificationResultV1::Refused => "refused",
        ExternalUpgradeVerificationResultV1::Verified => "verified",
        ExternalUpgradeVerificationResultV1::Mismatch => "mismatch",
    }
}

const fn external_upgrade_completion_status_label(
    status: ExternalUpgradeCompletionStatusV1,
) -> &'static str {
    match status {
        ExternalUpgradeCompletionStatusV1::AwaitingConsent => "awaiting_consent",
        ExternalUpgradeCompletionStatusV1::ConsentRefused => "consent_refused",
        ExternalUpgradeCompletionStatusV1::SuppliedEvidenceConsistent => {
            "supplied_evidence_consistent"
        }
        ExternalUpgradeCompletionStatusV1::AwaitingVerification => "awaiting_verification",
        ExternalUpgradeCompletionStatusV1::VerifiedComplete => "verified_complete",
        ExternalUpgradeCompletionStatusV1::VerificationFailed => "verification_failed",
    }
}

const fn external_verification_observation_source_label(
    source: ExternalVerificationObservationSourceV1,
) -> &'static str {
    match source {
        ExternalVerificationObservationSourceV1::SuppliedObservation => "supplied_observation",
        ExternalVerificationObservationSourceV1::DeploymentTruthInventory => {
            "deployment_truth_inventory"
        }
    }
}

const fn consent_channel_label(kind: ConsentChannelKindV1) -> &'static str {
    match kind {
        ConsentChannelKindV1::OutOfBand => "out_of_band",
        ConsentChannelKindV1::GeneratedCommand => "generated_command",
        ConsentChannelKindV1::DelegatedInstall => "delegated_install",
        ConsentChannelKindV1::GovernanceProposal => "governance_proposal",
        ConsentChannelKindV1::ApplicationSpecific => "application_specific",
    }
}

const fn consent_subject_label(kind: ConsentSubjectKindV1) -> &'static str {
    match kind {
        ConsentSubjectKindV1::UserPrincipal => "user_principal",
        ConsentSubjectKindV1::ProjectHub => "project_hub",
        ConsentSubjectKindV1::GovernanceCanister => "governance_canister",
        ConsentSubjectKindV1::CustomerController => "customer_controller",
        ConsentSubjectKindV1::DelegatedInstallCanister => "delegated_install_canister",
        ConsentSubjectKindV1::MultisigAuthority => "multisig_authority",
        ConsentSubjectKindV1::UnknownExternalController => "unknown_external_controller",
    }
}

fn append_external_lifecycle_role_items(
    lines: &mut Vec<String>,
    label: &str,
    rows: &[ExternalLifecycleRoleUpgradeV1],
) {
    if rows.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for row in rows {
        lines.push(format!(
            "  - subject={} role={} canister_id={} control_class={:?} lifecycle_mode={} required_external_action={}",
            row.subject,
            optional_text(row.role.as_deref()),
            optional_text(row.canister_id.as_deref()),
            row.control_class,
            lifecycle_mode_label(row.lifecycle_mode),
            optional_text(row.required_external_action.as_deref())
        ));
    }
}

fn append_lifecycle_authority_items(lines: &mut Vec<String>, rows: &[LifecycleAuthorityV1]) {
    if rows.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("authorities:".to_string());
    for row in rows {
        lines.push(format!(
            "  - subject={} role={} canister_id={} control_class={:?} lifecycle_mode={} external_action_required={} blocked={}",
            row.subject,
            optional_text(row.role.as_deref()),
            optional_text(row.canister_id.as_deref()),
            row.control_class,
            lifecycle_mode_label(row.lifecycle_mode),
            row.external_action_required,
            row.blocked
        ));
    }
}

fn append_external_upgrade_proposal_items(
    lines: &mut Vec<String>,
    proposals: &[ExternalUpgradeProposalV1],
) {
    if proposals.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("proposals:".to_string());
    for proposal in proposals {
        lines.push(format!(
            "  - proposal_id={} subject={} role={} canister_id={} lifecycle_mode={} required_external_action={} consent_requirements={} proposal_digest={}",
            proposal.proposal_id,
            proposal.subject,
            optional_text(proposal.role.as_deref()),
            optional_text(proposal.canister_id.as_deref()),
            lifecycle_mode_label(proposal.lifecycle_mode),
            proposal.required_external_action,
            proposal.consent_requirements.len(),
            proposal.proposal_digest
        ));
    }
}

fn append_external_lifecycle_pending_action_items(
    lines: &mut Vec<String>,
    actions: &[ExternalLifecyclePendingActionV1],
) {
    if actions.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("pending_external_actions:".to_string());
    for action in actions {
        lines.push(format!(
            "  - proposal_id={} subject={} role={} canister_id={} lifecycle_mode={} required_external_action={} consent_requirements={} proposal_digest={}",
            action.proposal_id,
            action.subject,
            optional_text(action.role.as_deref()),
            optional_text(action.canister_id.as_deref()),
            lifecycle_mode_label(action.lifecycle_mode),
            action.required_external_action,
            action.consent_requirements.len(),
            action.proposal_digest
        ));
    }
}

fn append_external_lifecycle_handoff_action_items(
    lines: &mut Vec<String>,
    actions: &[ExternalLifecycleHandoffActionV1],
) {
    if actions.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("handoff_actions:".to_string());
    for action in actions {
        lines.push(format!(
            "  - proposal_id={} subject={} role={} canister_id={} lifecycle_mode={} required_external_action={} consent_channel={} consent_subject={} verification_requirements={} proposal_digest={}",
            action.proposal_id,
            action.subject,
            optional_text(action.role.as_deref()),
            optional_text(action.canister_id.as_deref()),
            lifecycle_mode_label(action.lifecycle_mode),
            action.required_external_action,
            consent_channel_label(action.consent_channel_kind),
            consent_subject_label(action.consent_subject_kind),
            action.verification_requirements.len(),
            action.proposal_digest
        ));
    }
}

fn append_verification_policy_requirement_items(
    lines: &mut Vec<String>,
    requirements: &[ExternalUpgradeVerificationPolicyRequirementV1],
) {
    if requirements.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("verification_requirements:".to_string());
    for requirement in requirements {
        lines.push(format!(
            "  - requirement={} status={} expected_value={}",
            verification_requirement_label(requirement.requirement),
            verification_requirement_status_label(requirement.status),
            optional_text(requirement.expected_value.as_deref())
        ));
    }
}

fn append_verification_check_requirement_items(
    lines: &mut Vec<String>,
    requirements: &[ExternalUpgradeVerificationCheckRequirementV1],
) {
    if requirements.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("requirement_results:".to_string());
    for requirement in requirements {
        lines.push(format!(
            "  - requirement={} status={} expected_value={} observed_value={} satisfied={}",
            verification_requirement_label(requirement.requirement),
            verification_requirement_status_label(requirement.status),
            optional_text(requirement.expected_value.as_deref()),
            optional_text(requirement.observed_value.as_deref()),
            optional_bool_label(requirement.satisfied)
        ));
    }
}

const fn verification_requirement_status_label(
    status: ExternalUpgradeVerificationRequirementStatusV1,
) -> &'static str {
    match status {
        ExternalUpgradeVerificationRequirementStatusV1::Required => "required",
        ExternalUpgradeVerificationRequirementStatusV1::NotRequired => "not_required",
    }
}

const fn verification_requirement_label(
    requirement: LifecycleVerificationRequirementV1,
) -> &'static str {
    match requirement {
        LifecycleVerificationRequirementV1::LiveInventory => "live_inventory",
        LifecycleVerificationRequirementV1::ControllerObservation => "controller_observation",
        LifecycleVerificationRequirementV1::ModuleHash => "module_hash",
        LifecycleVerificationRequirementV1::CanonicalEmbeddedConfig => "canonical_embedded_config",
        LifecycleVerificationRequirementV1::ProtectedCallReadiness => "protected_call_readiness",
    }
}

fn optional_text(value: Option<&str>) -> &str {
    value.unwrap_or("none")
}

const fn optional_bool_label(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "unknown",
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
