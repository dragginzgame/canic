use super::super::super::*;
use serde::Serialize;

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

pub(in crate::deployment_truth::lifecycle) fn external_upgrade_proposal_digest(
    proposal: &ExternalUpgradeProposalV1,
) -> String {
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

pub(in crate::deployment_truth::lifecycle) fn external_upgrade_proposal_report_digest(
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

pub(in crate::deployment_truth::lifecycle) fn external_upgrade_receipt_digest(
    receipt: &ExternalUpgradeReceiptV1,
) -> String {
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

pub(in crate::deployment_truth::lifecycle) fn external_upgrade_consent_evidence_digest(
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

pub(in crate::deployment_truth::lifecycle) fn external_upgrade_verification_report_digest(
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

pub(in crate::deployment_truth::lifecycle) fn external_upgrade_verification_policy_digest(
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

pub(in crate::deployment_truth::lifecycle) fn external_upgrade_verification_check_digest(
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

pub(in crate::deployment_truth::lifecycle) fn external_upgrade_completion_report_digest(
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

pub(in crate::deployment_truth::lifecycle) fn observed_before_digest(
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
