use super::super::*;
use super::{append_string_items, optional_bool_label, optional_text};

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
