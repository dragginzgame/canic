use super::super::super::*;
use super::super::{optional_bool_label, optional_text};

pub(super) const fn external_lifecycle_plan_status_label(
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

pub(super) const fn external_upgrade_consent_state_label(
    state: ExternalUpgradeConsentStateV1,
) -> &'static str {
    match state {
        ExternalUpgradeConsentStateV1::Pending => "pending",
        ExternalUpgradeConsentStateV1::Refused => "refused",
        ExternalUpgradeConsentStateV1::Delegated => "delegated",
        ExternalUpgradeConsentStateV1::ExecutedExternally => "executed_externally",
    }
}

pub(super) const fn external_upgrade_verification_result_label(
    result: ExternalUpgradeVerificationResultV1,
) -> &'static str {
    match result {
        ExternalUpgradeVerificationResultV1::Pending => "pending",
        ExternalUpgradeVerificationResultV1::Refused => "refused",
        ExternalUpgradeVerificationResultV1::Verified => "verified",
        ExternalUpgradeVerificationResultV1::Mismatch => "mismatch",
    }
}

pub(super) const fn external_upgrade_completion_status_label(
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

pub(super) const fn external_verification_observation_source_label(
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
            lifecycle_mode_label(proposal.lifecycle_mode),
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
            lifecycle_mode_label(action.lifecycle_mode),
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
            lifecycle_mode_label(action.lifecycle_mode),
            action.required_external_action,
            consent_channel_label(action.consent_channel_kind),
            consent_subject_label(action.consent_subject_kind),
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
            verification_requirement_label(requirement.requirement),
            verification_requirement_status_label(requirement.status),
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
