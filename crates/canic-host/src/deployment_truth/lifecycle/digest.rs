use super::super::{
    CanisterControlClassV1, ConsentRequirementV1, CriticalExternalFixReportV1,
    ExternalLifecycleCheckV1, ExternalLifecycleHandoffActionV1, ExternalLifecycleHandoffV1,
    ExternalLifecyclePendingActionV1, ExternalLifecyclePendingReportV1,
    ExternalLifecyclePlanStatusV1, ExternalLifecyclePlanV1, ExternalLifecycleRoleUpgradeV1,
    ExternalUpgradeAuthorizationModeV1, ExternalUpgradeCompletionReportV1,
    ExternalUpgradeCompletionStatusV1, ExternalUpgradeConsentEvidenceV1,
    ExternalUpgradeConsentStateV1, ExternalUpgradeProposalReportV1, ExternalUpgradeProposalV1,
    ExternalUpgradeReceiptV1, ExternalUpgradeVerificationCheckRequirementV1,
    ExternalUpgradeVerificationCheckV1, ExternalUpgradeVerificationObservationV1,
    ExternalUpgradeVerificationPolicyRequirementV1, ExternalUpgradeVerificationPolicyV1,
    ExternalUpgradeVerificationReportV1, ExternalUpgradeVerificationResultV1,
    ExternalVerificationObservationSourceV1, LifecycleAuthorityReportV1, LifecycleAuthorityV1,
    LifecycleModeV1, LifecycleVerificationRequirementV1, stable_json_sha256_hex,
};
use serde::Serialize;

#[derive(Serialize)]
struct LifecycleAuthorityReportDigestInput<'a> {
    report_id: &'a str,
    check_id: &'a str,
    plan_id: &'a str,
    inventory_id: &'a str,
    authorities: &'a [LifecycleAuthorityV1],
    external_action_required_count: usize,
    blocked_count: usize,
}

#[derive(Serialize)]
struct ExternalLifecyclePlanDigestInput<'a> {
    lifecycle_authority_report_id: &'a str,
    deployment_plan_id: &'a str,
    deployment_plan_digest: &'a str,
    inventory_id: &'a str,
    lifecycle_authority_rows: &'a [LifecycleAuthorityV1],
    directly_executable_role_upgrades: &'a [ExternalLifecycleRoleUpgradeV1],
    proposed_external_role_upgrades: &'a [ExternalLifecycleRoleUpgradeV1],
    blocked_role_upgrades: &'a [ExternalLifecycleRoleUpgradeV1],
    dependency_blockers: &'a [String],
    protected_call_implications: &'a [String],
    residual_exposure: &'a [String],
    status: ExternalLifecyclePlanStatusV1,
}

#[derive(Serialize)]
struct ExternalUpgradeProposalReportDigestInput<'a> {
    report_id: &'a str,
    lifecycle_plan_id: &'a str,
    lifecycle_plan_digest: &'a str,
    deployment_plan_id: &'a str,
    deployment_plan_digest: &'a str,
    inventory_id: &'a str,
    proposals: &'a [ExternalUpgradeProposalV1],
    blocked_subjects: &'a [String],
}

#[derive(Serialize)]
struct ExternalLifecyclePendingReportDigestInput<'a> {
    report_id: &'a str,
    lifecycle_plan_id: &'a str,
    lifecycle_plan_digest: &'a str,
    proposal_report_id: &'a str,
    proposal_report_digest: &'a str,
    deployment_plan_id: &'a str,
    deployment_plan_digest: &'a str,
    inventory_id: &'a str,
    direct_upgrade_count: usize,
    pending_external_count: usize,
    blocked_count: usize,
    pending_external_actions: &'a [ExternalLifecyclePendingActionV1],
    blocked_subjects: &'a [String],
    residual_exposure: &'a [String],
    status: ExternalLifecyclePlanStatusV1,
}

#[derive(Serialize)]
struct ExternalLifecycleCheckDigestInput<'a> {
    check_id: &'a str,
    lifecycle_plan_id: &'a str,
    lifecycle_plan_digest: &'a str,
    proposal_report_id: &'a str,
    proposal_report_digest: &'a str,
    pending_report_id: &'a str,
    pending_report_digest: &'a str,
    deployment_plan_id: &'a str,
    deployment_plan_digest: &'a str,
    inventory_id: &'a str,
    status: ExternalLifecyclePlanStatusV1,
    direct_upgrade_count: usize,
    pending_external_count: usize,
    blocked_count: usize,
    residual_exposure_count: usize,
    summary: &'a str,
    next_actions: &'a [String],
}

#[derive(Serialize)]
struct ExternalLifecycleHandoffDigestInput<'a> {
    handoff_id: &'a str,
    lifecycle_check_id: &'a str,
    lifecycle_check_digest: &'a str,
    pending_report_id: &'a str,
    pending_report_digest: &'a str,
    proposal_report_id: &'a str,
    proposal_report_digest: &'a str,
    deployment_plan_id: &'a str,
    deployment_plan_digest: &'a str,
    inventory_id: &'a str,
    status: ExternalLifecyclePlanStatusV1,
    handoff_actions: &'a [ExternalLifecycleHandoffActionV1],
    blocked_subjects: &'a [String],
    residual_exposure: &'a [String],
    operator_summary: &'a str,
}

#[derive(Serialize)]
struct CriticalExternalFixReportDigestInput<'a> {
    report_id: &'a str,
    fix_id: &'a str,
    severity: &'a str,
    lifecycle_plan_id: &'a str,
    lifecycle_plan_digest: &'a str,
    pending_report_id: &'a str,
    pending_report_digest: &'a str,
    deployment_plan_id: &'a str,
    deployment_plan_digest: &'a str,
    inventory_id: &'a str,
    affected_roles: &'a [String],
    affected_canisters: &'a [String],
    directly_patchable_roles: &'a [String],
    externally_blocked_roles: &'a [String],
    dependency_blocked_roles: &'a [String],
    required_external_actions: &'a [String],
    protected_call_implications: &'a [String],
    residual_exposure: &'a [String],
    operator_next_steps: &'a [String],
}

#[derive(Serialize)]
struct ExternalUpgradeProposalDigestInput<'a> {
    deployment_plan_id: &'a str,
    deployment_plan_digest: &'a str,
    lifecycle_plan_id: &'a str,
    lifecycle_plan_digest: &'a str,
    promotion_plan_id: &'a Option<String>,
    promotion_plan_digest: &'a Option<String>,
    promotion_provenance_id: &'a Option<String>,
    promotion_provenance_digest: &'a Option<String>,
    subject: &'a str,
    canister_id: &'a Option<String>,
    role: &'a Option<String>,
    control_class: CanisterControlClassV1,
    lifecycle_mode: LifecycleModeV1,
    observed_before_digest: &'a str,
    current_module_hash: &'a Option<String>,
    current_canonical_embedded_config_sha256: &'a Option<String>,
    target_wasm_sha256: &'a Option<String>,
    target_wasm_gz_sha256: &'a Option<String>,
    target_installed_module_hash: &'a Option<String>,
    target_role_artifact_identity: &'a Option<String>,
    target_canonical_embedded_config_sha256: &'a Option<String>,
    root_trust_anchor: &'a Option<String>,
    authority_profile_hash: &'a Option<String>,
    required_external_action: &'a str,
    consent_requirements: &'a [ConsentRequirementV1],
    allowed_authorization_modes: &'a [ExternalUpgradeAuthorizationModeV1],
    verification_requirements: &'a [LifecycleVerificationRequirementV1],
    expires_at: &'a Option<String>,
    supersedes_proposal_id: &'a Option<String>,
}

#[derive(Serialize)]
struct ExternalUpgradeReceiptDigestInput<'a> {
    proposal_id: &'a str,
    proposal_digest: &'a str,
    subject: &'a str,
    canister_id: &'a Option<String>,
    role: &'a Option<String>,
    consent_state: ExternalUpgradeConsentStateV1,
    reported_by: &'a Option<String>,
    observed_before_module_hash: &'a Option<String>,
    observed_after_module_hash: &'a Option<String>,
    observed_after_canonical_embedded_config_sha256: &'a Option<String>,
    verification_result: ExternalUpgradeVerificationResultV1,
    verification_notes: &'a [String],
}

#[derive(Serialize)]
struct ExternalUpgradeConsentEvidenceDigestInput<'a> {
    evidence_id: &'a str,
    proposal_id: &'a str,
    proposal_digest: &'a str,
    receipt_id: &'a str,
    receipt_digest: &'a str,
    subject: &'a str,
    canister_id: &'a Option<String>,
    role: &'a Option<String>,
    consent_state: ExternalUpgradeConsentStateV1,
    reported_by: &'a Option<String>,
    consent_requirements: &'a [ConsentRequirementV1],
    allowed_authorization_modes: &'a [ExternalUpgradeAuthorizationModeV1],
    status_summary: &'a str,
}

#[derive(Serialize)]
struct ExternalUpgradeVerificationReportDigestInput<'a> {
    report_id: &'a str,
    proposal_id: &'a str,
    proposal_digest: &'a str,
    receipt_id: &'a str,
    receipt_digest: &'a str,
    subject: &'a str,
    canister_id: &'a Option<String>,
    role: &'a Option<String>,
    verification_result: ExternalUpgradeVerificationResultV1,
    verification_notes: &'a [String],
    live_inventory_required: bool,
    status_summary: &'a str,
}

#[derive(Serialize)]
struct ExternalUpgradeVerificationPolicyDigestInput<'a> {
    policy_id: &'a str,
    proposal_id: &'a str,
    proposal_digest: &'a str,
    deployment_plan_id: &'a str,
    deployment_plan_digest: &'a str,
    subject: &'a str,
    canister_id: &'a Option<String>,
    role: &'a Option<String>,
    required_verification: &'a [LifecycleVerificationRequirementV1],
    verification_requirements: &'a [ExternalUpgradeVerificationPolicyRequirementV1],
    max_observation_age_seconds: Option<u64>,
    status_summary: &'a str,
}

#[derive(Serialize)]
struct ExternalUpgradeVerificationCheckDigestInput<'a> {
    check_id: &'a str,
    policy_id: &'a str,
    policy_digest: &'a str,
    proposal_id: &'a str,
    proposal_digest: &'a str,
    subject: &'a str,
    canister_id: &'a Option<String>,
    role: &'a Option<String>,
    observation: &'a ExternalUpgradeVerificationObservationV1,
    requirement_results: &'a [ExternalUpgradeVerificationCheckRequirementV1],
    verification_result: ExternalUpgradeVerificationResultV1,
    status_summary: &'a str,
}

#[derive(Serialize)]
struct ExternalUpgradeCompletionReportDigestInput<'a> {
    report_id: &'a str,
    proposal_id: &'a str,
    proposal_digest: &'a str,
    consent_evidence_id: &'a str,
    consent_evidence_digest: &'a str,
    verification_check_id: &'a str,
    verification_check_digest: &'a str,
    subject: &'a str,
    canister_id: &'a Option<String>,
    role: &'a Option<String>,
    consent_state: ExternalUpgradeConsentStateV1,
    verification_result: ExternalUpgradeVerificationResultV1,
    verification_observation_source: ExternalVerificationObservationSourceV1,
    completion_status: ExternalUpgradeCompletionStatusV1,
    blockers: &'a [String],
    next_actions: &'a [String],
    status_summary: &'a str,
}

#[derive(Serialize)]
struct ObservedBeforeDigestInput<'a> {
    subject: &'a str,
    canister_id: &'a Option<String>,
    role: &'a Option<String>,
    observed_controllers: &'a [String],
    current_module_hash: Option<&'a String>,
    current_canonical_embedded_config_sha256: Option<&'a String>,
}

pub(super) fn external_lifecycle_plan_digest(plan: &ExternalLifecyclePlanV1) -> String {
    stable_json_sha256_hex(&ExternalLifecyclePlanDigestInput {
        lifecycle_authority_report_id: &plan.lifecycle_authority_report_id,
        deployment_plan_id: &plan.deployment_plan_id,
        deployment_plan_digest: &plan.deployment_plan_digest,
        inventory_id: &plan.inventory_id,
        lifecycle_authority_rows: &plan.lifecycle_authority_rows,
        directly_executable_role_upgrades: &plan.directly_executable_role_upgrades,
        proposed_external_role_upgrades: &plan.proposed_external_role_upgrades,
        blocked_role_upgrades: &plan.blocked_role_upgrades,
        dependency_blockers: &plan.dependency_blockers,
        protected_call_implications: &plan.protected_call_implications,
        residual_exposure: &plan.residual_exposure,
        status: plan.status,
    })
}

pub(super) fn lifecycle_authority_report_digest(report: &LifecycleAuthorityReportV1) -> String {
    stable_json_sha256_hex(&LifecycleAuthorityReportDigestInput {
        report_id: &report.report_id,
        check_id: &report.check_id,
        plan_id: &report.plan_id,
        inventory_id: &report.inventory_id,
        authorities: &report.authorities,
        external_action_required_count: report.external_action_required_count,
        blocked_count: report.blocked_count,
    })
}

pub(super) fn external_upgrade_proposal_digest(proposal: &ExternalUpgradeProposalV1) -> String {
    stable_json_sha256_hex(&ExternalUpgradeProposalDigestInput {
        deployment_plan_id: &proposal.deployment_plan_id,
        deployment_plan_digest: &proposal.deployment_plan_digest,
        lifecycle_plan_id: &proposal.lifecycle_plan_id,
        lifecycle_plan_digest: &proposal.lifecycle_plan_digest,
        promotion_plan_id: &proposal.promotion_plan_id,
        promotion_plan_digest: &proposal.promotion_plan_digest,
        promotion_provenance_id: &proposal.promotion_provenance_id,
        promotion_provenance_digest: &proposal.promotion_provenance_digest,
        subject: &proposal.subject,
        canister_id: &proposal.canister_id,
        role: &proposal.role,
        control_class: proposal.control_class,
        lifecycle_mode: proposal.lifecycle_mode,
        observed_before_digest: &proposal.observed_before_digest,
        current_module_hash: &proposal.current_module_hash,
        current_canonical_embedded_config_sha256: &proposal
            .current_canonical_embedded_config_sha256,
        target_wasm_sha256: &proposal.target_wasm_sha256,
        target_wasm_gz_sha256: &proposal.target_wasm_gz_sha256,
        target_installed_module_hash: &proposal.target_installed_module_hash,
        target_role_artifact_identity: &proposal.target_role_artifact_identity,
        target_canonical_embedded_config_sha256: &proposal.target_canonical_embedded_config_sha256,
        root_trust_anchor: &proposal.root_trust_anchor,
        authority_profile_hash: &proposal.authority_profile_hash,
        required_external_action: &proposal.required_external_action,
        consent_requirements: &proposal.consent_requirements,
        allowed_authorization_modes: &proposal.allowed_authorization_modes,
        verification_requirements: &proposal.verification_requirements,
        expires_at: &proposal.expires_at,
        supersedes_proposal_id: &proposal.supersedes_proposal_id,
    })
}

pub(super) fn external_upgrade_proposal_report_digest(
    report: &ExternalUpgradeProposalReportV1,
) -> String {
    stable_json_sha256_hex(&ExternalUpgradeProposalReportDigestInput {
        report_id: &report.report_id,
        lifecycle_plan_id: &report.lifecycle_plan_id,
        lifecycle_plan_digest: &report.lifecycle_plan_digest,
        deployment_plan_id: &report.deployment_plan_id,
        deployment_plan_digest: &report.deployment_plan_digest,
        inventory_id: &report.inventory_id,
        proposals: &report.proposals,
        blocked_subjects: &report.blocked_subjects,
    })
}

pub(super) fn external_lifecycle_pending_report_digest(
    report: &ExternalLifecyclePendingReportV1,
) -> String {
    stable_json_sha256_hex(&ExternalLifecyclePendingReportDigestInput {
        report_id: &report.report_id,
        lifecycle_plan_id: &report.lifecycle_plan_id,
        lifecycle_plan_digest: &report.lifecycle_plan_digest,
        proposal_report_id: &report.proposal_report_id,
        proposal_report_digest: &report.proposal_report_digest,
        deployment_plan_id: &report.deployment_plan_id,
        deployment_plan_digest: &report.deployment_plan_digest,
        inventory_id: &report.inventory_id,
        direct_upgrade_count: report.direct_upgrade_count,
        pending_external_count: report.pending_external_count,
        blocked_count: report.blocked_count,
        pending_external_actions: &report.pending_external_actions,
        blocked_subjects: &report.blocked_subjects,
        residual_exposure: &report.residual_exposure,
        status: report.status,
    })
}

pub(super) fn external_lifecycle_check_digest(check: &ExternalLifecycleCheckV1) -> String {
    stable_json_sha256_hex(&ExternalLifecycleCheckDigestInput {
        check_id: &check.check_id,
        lifecycle_plan_id: &check.lifecycle_plan_id,
        lifecycle_plan_digest: &check.lifecycle_plan_digest,
        proposal_report_id: &check.proposal_report_id,
        proposal_report_digest: &check.proposal_report_digest,
        pending_report_id: &check.pending_report_id,
        pending_report_digest: &check.pending_report_digest,
        deployment_plan_id: &check.deployment_plan_id,
        deployment_plan_digest: &check.deployment_plan_digest,
        inventory_id: &check.inventory_id,
        status: check.status,
        direct_upgrade_count: check.direct_upgrade_count,
        pending_external_count: check.pending_external_count,
        blocked_count: check.blocked_count,
        residual_exposure_count: check.residual_exposure_count,
        summary: &check.summary,
        next_actions: &check.next_actions,
    })
}

pub(super) fn external_lifecycle_handoff_digest(handoff: &ExternalLifecycleHandoffV1) -> String {
    stable_json_sha256_hex(&ExternalLifecycleHandoffDigestInput {
        handoff_id: &handoff.handoff_id,
        lifecycle_check_id: &handoff.lifecycle_check_id,
        lifecycle_check_digest: &handoff.lifecycle_check_digest,
        pending_report_id: &handoff.pending_report_id,
        pending_report_digest: &handoff.pending_report_digest,
        proposal_report_id: &handoff.proposal_report_id,
        proposal_report_digest: &handoff.proposal_report_digest,
        deployment_plan_id: &handoff.deployment_plan_id,
        deployment_plan_digest: &handoff.deployment_plan_digest,
        inventory_id: &handoff.inventory_id,
        status: handoff.status,
        handoff_actions: &handoff.handoff_actions,
        blocked_subjects: &handoff.blocked_subjects,
        residual_exposure: &handoff.residual_exposure,
        operator_summary: &handoff.operator_summary,
    })
}

pub(super) fn critical_external_fix_report_digest(report: &CriticalExternalFixReportV1) -> String {
    stable_json_sha256_hex(&CriticalExternalFixReportDigestInput {
        report_id: &report.report_id,
        fix_id: &report.fix_id,
        severity: &report.severity,
        lifecycle_plan_id: &report.lifecycle_plan_id,
        lifecycle_plan_digest: &report.lifecycle_plan_digest,
        pending_report_id: &report.pending_report_id,
        pending_report_digest: &report.pending_report_digest,
        deployment_plan_id: &report.deployment_plan_id,
        deployment_plan_digest: &report.deployment_plan_digest,
        inventory_id: &report.inventory_id,
        affected_roles: &report.affected_roles,
        affected_canisters: &report.affected_canisters,
        directly_patchable_roles: &report.directly_patchable_roles,
        externally_blocked_roles: &report.externally_blocked_roles,
        dependency_blocked_roles: &report.dependency_blocked_roles,
        required_external_actions: &report.required_external_actions,
        protected_call_implications: &report.protected_call_implications,
        residual_exposure: &report.residual_exposure,
        operator_next_steps: &report.operator_next_steps,
    })
}

pub(super) fn external_upgrade_receipt_digest(receipt: &ExternalUpgradeReceiptV1) -> String {
    stable_json_sha256_hex(&ExternalUpgradeReceiptDigestInput {
        proposal_id: &receipt.proposal_id,
        proposal_digest: &receipt.proposal_digest,
        subject: &receipt.subject,
        canister_id: &receipt.canister_id,
        role: &receipt.role,
        consent_state: receipt.consent_state,
        reported_by: &receipt.reported_by,
        observed_before_module_hash: &receipt.observed_before_module_hash,
        observed_after_module_hash: &receipt.observed_after_module_hash,
        observed_after_canonical_embedded_config_sha256: &receipt
            .observed_after_canonical_embedded_config_sha256,
        verification_result: receipt.verification_result,
        verification_notes: &receipt.verification_notes,
    })
}

pub(super) fn external_upgrade_consent_evidence_digest(
    evidence: &ExternalUpgradeConsentEvidenceV1,
) -> String {
    stable_json_sha256_hex(&ExternalUpgradeConsentEvidenceDigestInput {
        evidence_id: &evidence.evidence_id,
        proposal_id: &evidence.proposal_id,
        proposal_digest: &evidence.proposal_digest,
        receipt_id: &evidence.receipt_id,
        receipt_digest: &evidence.receipt_digest,
        subject: &evidence.subject,
        canister_id: &evidence.canister_id,
        role: &evidence.role,
        consent_state: evidence.consent_state,
        reported_by: &evidence.reported_by,
        consent_requirements: &evidence.consent_requirements,
        allowed_authorization_modes: &evidence.allowed_authorization_modes,
        status_summary: &evidence.status_summary,
    })
}

pub(super) fn external_upgrade_verification_report_digest(
    report: &ExternalUpgradeVerificationReportV1,
) -> String {
    stable_json_sha256_hex(&ExternalUpgradeVerificationReportDigestInput {
        report_id: &report.report_id,
        proposal_id: &report.proposal_id,
        proposal_digest: &report.proposal_digest,
        receipt_id: &report.receipt_id,
        receipt_digest: &report.receipt_digest,
        subject: &report.subject,
        canister_id: &report.canister_id,
        role: &report.role,
        verification_result: report.verification_result,
        verification_notes: &report.verification_notes,
        live_inventory_required: report.live_inventory_required,
        status_summary: &report.status_summary,
    })
}

pub(super) fn external_upgrade_verification_policy_digest(
    policy: &ExternalUpgradeVerificationPolicyV1,
) -> String {
    stable_json_sha256_hex(&ExternalUpgradeVerificationPolicyDigestInput {
        policy_id: &policy.policy_id,
        proposal_id: &policy.proposal_id,
        proposal_digest: &policy.proposal_digest,
        deployment_plan_id: &policy.deployment_plan_id,
        deployment_plan_digest: &policy.deployment_plan_digest,
        subject: &policy.subject,
        canister_id: &policy.canister_id,
        role: &policy.role,
        required_verification: &policy.required_verification,
        verification_requirements: &policy.verification_requirements,
        max_observation_age_seconds: policy.max_observation_age_seconds,
        status_summary: &policy.status_summary,
    })
}

pub(super) fn external_upgrade_verification_check_digest(
    check: &ExternalUpgradeVerificationCheckV1,
) -> String {
    stable_json_sha256_hex(&ExternalUpgradeVerificationCheckDigestInput {
        check_id: &check.check_id,
        policy_id: &check.policy_id,
        policy_digest: &check.policy_digest,
        proposal_id: &check.proposal_id,
        proposal_digest: &check.proposal_digest,
        subject: &check.subject,
        canister_id: &check.canister_id,
        role: &check.role,
        observation: &check.observation,
        requirement_results: &check.requirement_results,
        verification_result: check.verification_result,
        status_summary: &check.status_summary,
    })
}

pub(super) fn external_upgrade_completion_report_digest(
    report: &ExternalUpgradeCompletionReportV1,
) -> String {
    stable_json_sha256_hex(&ExternalUpgradeCompletionReportDigestInput {
        report_id: &report.report_id,
        proposal_id: &report.proposal_id,
        proposal_digest: &report.proposal_digest,
        consent_evidence_id: &report.consent_evidence_id,
        consent_evidence_digest: &report.consent_evidence_digest,
        verification_check_id: &report.verification_check_id,
        verification_check_digest: &report.verification_check_digest,
        subject: &report.subject,
        canister_id: &report.canister_id,
        role: &report.role,
        consent_state: report.consent_state,
        verification_result: report.verification_result,
        verification_observation_source: report.verification_observation_source,
        completion_status: report.completion_status,
        blockers: &report.blockers,
        next_actions: &report.next_actions,
        status_summary: &report.status_summary,
    })
}

pub(super) fn observed_before_digest(
    authority: &LifecycleAuthorityV1,
    current_module_hash: Option<&String>,
    current_config_hash: Option<&String>,
) -> String {
    stable_json_sha256_hex(&ObservedBeforeDigestInput {
        subject: &authority.subject,
        canister_id: &authority.canister_id,
        role: &authority.role,
        observed_controllers: &authority.observed_controllers,
        current_module_hash,
        current_canonical_embedded_config_sha256: current_config_hash,
    })
}
