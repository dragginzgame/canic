use super::super::super::*;
use super::super::{optional_bool_label, optional_text};

pub(super) fn append_external_lifecycle_role_items(
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
            "  - subject={} role={} canister_id={} control_class={} lifecycle_mode={} required_external_action={}",
            row.subject,
            optional_text(row.role.as_deref()),
            optional_text(row.canister_id.as_deref()),
            row.control_class.label(),
            row.lifecycle_mode.label(),
            optional_text(row.required_external_action.as_deref())
        ));
    }
}

pub(super) fn append_lifecycle_authority_items(
    lines: &mut Vec<String>,
    rows: &[LifecycleAuthorityV1],
) {
    if rows.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("authorities:".to_string());
    for row in rows {
        lines.push(format!(
            "  - subject={} role={} canister_id={} control_class={} lifecycle_mode={} external_action_required={} blocked={}",
            row.subject,
            optional_text(row.role.as_deref()),
            optional_text(row.canister_id.as_deref()),
            row.control_class.label(),
            row.lifecycle_mode.label(),
            row.external_action_required,
            row.blocked
        ));
    }
}

pub(super) fn append_external_upgrade_proposal_items(
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
            proposal.lifecycle_mode.label(),
            proposal.required_external_action,
            proposal.consent_requirements.len(),
            proposal.proposal_digest
        ));
    }
}

pub(super) fn append_external_lifecycle_pending_action_items(
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
            action.lifecycle_mode.label(),
            action.required_external_action,
            action.consent_requirements.len(),
            action.proposal_digest
        ));
    }
}

pub(super) fn append_external_lifecycle_handoff_action_items(
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
            action.lifecycle_mode.label(),
            action.required_external_action,
            action.consent_channel_kind.label(),
            action.consent_subject_kind.label(),
            action.verification_requirements.len(),
            action.proposal_digest
        ));
    }
}

pub(super) fn append_verification_policy_requirement_items(
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
            requirement.requirement.label(),
            requirement.status.label(),
            optional_text(requirement.expected_value.as_deref())
        ));
    }
}

pub(super) fn append_verification_check_requirement_items(
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
            requirement.requirement.label(),
            requirement.status.label(),
            optional_text(requirement.expected_value.as_deref()),
            optional_text(requirement.observed_value.as_deref()),
            optional_bool_label(requirement.satisfied)
        ));
    }
}
