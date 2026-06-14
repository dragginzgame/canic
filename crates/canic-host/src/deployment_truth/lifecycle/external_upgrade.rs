use super::super::*;
use super::digest::*;
use super::error::{
    ExternalUpgradeCompletionReportError, ExternalUpgradeConsentEvidenceError,
    ExternalUpgradeProposalReportError, ExternalUpgradeReceiptError,
    ExternalUpgradeVerificationCheckError, ExternalUpgradeVerificationPolicyError,
    ExternalUpgradeVerificationReportError,
};
use std::collections::BTreeSet;

/// Build a passive external-upgrade receipt from post-action observation.
///
/// The receipt records what an external controller claims or completed. It does
/// not verify live state by itself and does not grant deployment authority.
#[must_use]
pub fn external_upgrade_receipt_from_observation(
    receipt_id: impl Into<String>,
    proposal: &ExternalUpgradeProposalV1,
    consent_state: ExternalUpgradeConsentStateV1,
    reported_by: Option<String>,
    observed_after: Option<&ObservedCanisterV1>,
) -> ExternalUpgradeReceiptV1 {
    let observed_after_module_hash =
        observed_after.and_then(|observed| observed.module_hash.clone());
    let observed_after_canonical_embedded_config_sha256 =
        observed_after.and_then(|observed| observed.canonical_embedded_config_digest.clone());
    let verification_result = external_upgrade_verification_result(
        consent_state,
        proposal,
        observed_after_module_hash.as_deref(),
        observed_after_canonical_embedded_config_sha256.as_deref(),
    );
    let verification_notes = external_upgrade_verification_notes(
        verification_result,
        proposal,
        observed_after_module_hash.as_deref(),
        observed_after_canonical_embedded_config_sha256.as_deref(),
    );

    let mut receipt = ExternalUpgradeReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        receipt_id: receipt_id.into(),
        proposal_id: proposal.proposal_id.clone(),
        proposal_digest: proposal.proposal_digest.clone(),
        subject: proposal.subject.clone(),
        canister_id: proposal.canister_id.clone(),
        role: proposal.role.clone(),
        consent_state,
        reported_by,
        observed_before_module_hash: proposal.current_module_hash.clone(),
        observed_after_module_hash,
        observed_after_canonical_embedded_config_sha256,
        verification_result,
        verification_notes,
        receipt_digest: String::new(),
    };
    receipt.receipt_digest = external_upgrade_receipt_digest(&receipt);
    receipt
}

/// Validate the internal consistency of an external-upgrade receipt.
///
/// This is structural validation only. Live inventory remains the source of
/// truth for whether the external upgrade actually completed.
pub fn validate_external_upgrade_receipt(
    receipt: &ExternalUpgradeReceiptV1,
) -> Result<(), ExternalUpgradeReceiptError> {
    if receipt.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(ExternalUpgradeReceiptError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            actual: receipt.schema_version,
        });
    }
    ensure_external_receipt_field("receipt_id", receipt.receipt_id.as_str())?;
    ensure_external_receipt_field("proposal_id", receipt.proposal_id.as_str())?;
    ensure_external_receipt_field("proposal_digest", receipt.proposal_digest.as_str())?;
    ensure_external_receipt_field("subject", receipt.subject.as_str())?;
    ensure_external_receipt_field("receipt_digest", receipt.receipt_digest.as_str())?;

    if receipt.consent_state == ExternalUpgradeConsentStateV1::Refused
        && receipt.verification_result == ExternalUpgradeVerificationResultV1::Verified
    {
        return Err(ExternalUpgradeReceiptError::RefusedConsentVerified);
    }
    let has_observation = receipt.observed_after_module_hash.is_some()
        || receipt
            .observed_after_canonical_embedded_config_sha256
            .is_some();
    if matches!(
        receipt.verification_result,
        ExternalUpgradeVerificationResultV1::Verified
            | ExternalUpgradeVerificationResultV1::Mismatch
    ) && !has_observation
    {
        return Err(ExternalUpgradeReceiptError::VerificationMismatch);
    }
    if receipt.receipt_digest != external_upgrade_receipt_digest(receipt) {
        return Err(ExternalUpgradeReceiptError::DigestMismatch {
            field: "receipt_digest",
        });
    }
    Ok(())
}

/// Validate an external-upgrade receipt against the proposal it claims to
/// satisfy.
///
/// This remains structural verification. It proves the receipt is linked to the
/// supplied proposal and that its verification result matches the proposal's
/// target facts, but live inventory remains the source of deployment truth.
pub fn validate_external_upgrade_receipt_for_proposal(
    receipt: &ExternalUpgradeReceiptV1,
    proposal: &ExternalUpgradeProposalV1,
) -> Result<(), ExternalUpgradeReceiptError> {
    validate_external_upgrade_receipt(receipt)?;
    ensure_external_receipt_matches_proposal(
        "proposal_id",
        receipt.proposal_id.as_str(),
        proposal.proposal_id.as_str(),
    )?;
    ensure_external_receipt_matches_proposal(
        "proposal_digest",
        receipt.proposal_digest.as_str(),
        proposal.proposal_digest.as_str(),
    )?;
    ensure_external_receipt_matches_proposal(
        "subject",
        receipt.subject.as_str(),
        proposal.subject.as_str(),
    )?;
    ensure_external_receipt_option_matches_proposal(
        "canister_id",
        receipt.canister_id.as_deref(),
        proposal.canister_id.as_deref(),
    )?;
    ensure_external_receipt_option_matches_proposal(
        "role",
        receipt.role.as_deref(),
        proposal.role.as_deref(),
    )?;
    ensure_external_receipt_option_matches_proposal(
        "observed_before_module_hash",
        receipt.observed_before_module_hash.as_deref(),
        proposal.current_module_hash.as_deref(),
    )?;

    let expected_result = external_upgrade_verification_result(
        receipt.consent_state,
        proposal,
        receipt.observed_after_module_hash.as_deref(),
        receipt
            .observed_after_canonical_embedded_config_sha256
            .as_deref(),
    );
    if receipt.verification_result != expected_result {
        return Err(ExternalUpgradeReceiptError::VerificationMismatch);
    }
    let expected_notes = external_upgrade_verification_notes(
        expected_result,
        proposal,
        receipt.observed_after_module_hash.as_deref(),
        receipt
            .observed_after_canonical_embedded_config_sha256
            .as_deref(),
    );
    if receipt.verification_notes != expected_notes {
        return Err(ExternalUpgradeReceiptError::SourceMismatch {
            field: "verification_notes",
        });
    }

    Ok(())
}

/// Build passive consent/action evidence from a proposal/receipt pair.
///
/// This records the reported consent or external action state only. It is not
/// completion proof; verification remains separate and live inventory remains
/// the source of deployment truth.
pub fn external_upgrade_consent_evidence_from_receipt(
    evidence_id: impl Into<String>,
    proposal: &ExternalUpgradeProposalV1,
    receipt: &ExternalUpgradeReceiptV1,
) -> Result<ExternalUpgradeConsentEvidenceV1, ExternalUpgradeReceiptError> {
    validate_external_upgrade_receipt_for_proposal(receipt, proposal)?;
    let consent_state = receipt.consent_state;
    let mut evidence = ExternalUpgradeConsentEvidenceV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        evidence_id: evidence_id.into(),
        evidence_digest: String::new(),
        proposal_id: proposal.proposal_id.clone(),
        proposal_digest: proposal.proposal_digest.clone(),
        receipt_id: receipt.receipt_id.clone(),
        receipt_digest: receipt.receipt_digest.clone(),
        subject: proposal.subject.clone(),
        canister_id: proposal.canister_id.clone(),
        role: proposal.role.clone(),
        consent_state,
        reported_by: receipt.reported_by.clone(),
        consent_requirements: proposal.consent_requirements.clone(),
        allowed_authorization_modes: proposal.allowed_authorization_modes.clone(),
        status_summary: external_upgrade_consent_summary(consent_state).to_string(),
    };
    evidence.evidence_digest = external_upgrade_consent_evidence_digest(&evidence);
    Ok(evidence)
}

/// Validate archived consent evidence consistency and digest.
pub fn validate_external_upgrade_consent_evidence(
    evidence: &ExternalUpgradeConsentEvidenceV1,
) -> Result<(), ExternalUpgradeConsentEvidenceError> {
    if evidence.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(ExternalUpgradeConsentEvidenceError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            actual: evidence.schema_version,
        });
    }
    ensure_external_consent_evidence_field("evidence_id", evidence.evidence_id.as_str())?;
    ensure_external_consent_evidence_field("evidence_digest", evidence.evidence_digest.as_str())?;
    ensure_external_consent_evidence_field("proposal_id", evidence.proposal_id.as_str())?;
    ensure_external_consent_evidence_field("proposal_digest", evidence.proposal_digest.as_str())?;
    ensure_external_consent_evidence_field("receipt_id", evidence.receipt_id.as_str())?;
    ensure_external_consent_evidence_field("receipt_digest", evidence.receipt_digest.as_str())?;
    ensure_external_consent_evidence_field("subject", evidence.subject.as_str())?;
    ensure_external_consent_evidence_field("status_summary", evidence.status_summary.as_str())?;
    if evidence.status_summary != external_upgrade_consent_summary(evidence.consent_state) {
        return Err(ExternalUpgradeConsentEvidenceError::SourceMismatch {
            field: "status_summary",
        });
    }
    if evidence.evidence_digest != external_upgrade_consent_evidence_digest(evidence) {
        return Err(ExternalUpgradeConsentEvidenceError::DigestMismatch {
            field: "evidence_digest",
        });
    }
    Ok(())
}

/// Validate that archived consent evidence still matches the proposal/receipt
/// pair it claims to summarize.
pub fn validate_external_upgrade_consent_evidence_for_receipt(
    evidence: &ExternalUpgradeConsentEvidenceV1,
    proposal: &ExternalUpgradeProposalV1,
    receipt: &ExternalUpgradeReceiptV1,
) -> Result<(), ExternalUpgradeConsentEvidenceError> {
    validate_external_upgrade_consent_evidence(evidence)?;
    let expected = external_upgrade_consent_evidence_from_receipt(
        evidence.evidence_id.clone(),
        proposal,
        receipt,
    )?;
    if evidence != &expected {
        return Err(ExternalUpgradeConsentEvidenceError::SourceMismatch { field: "receipt" });
    }
    Ok(())
}

/// Build a passive verification report for a proposal/receipt pair.
///
/// This packages structural verification evidence only. Live inventory remains
/// the source of truth for deployment state.
pub fn external_upgrade_verification_report_from_receipt(
    report_id: impl Into<String>,
    proposal: &ExternalUpgradeProposalV1,
    receipt: &ExternalUpgradeReceiptV1,
) -> Result<ExternalUpgradeVerificationReportV1, ExternalUpgradeReceiptError> {
    validate_external_upgrade_receipt_for_proposal(receipt, proposal)?;
    let verification_result = receipt.verification_result;
    let mut report = ExternalUpgradeVerificationReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        report_digest: String::new(),
        proposal_id: proposal.proposal_id.clone(),
        proposal_digest: proposal.proposal_digest.clone(),
        receipt_id: receipt.receipt_id.clone(),
        receipt_digest: receipt.receipt_digest.clone(),
        subject: proposal.subject.clone(),
        canister_id: proposal.canister_id.clone(),
        role: proposal.role.clone(),
        verification_result,
        verification_notes: receipt.verification_notes.clone(),
        live_inventory_required: verification_result
            != ExternalUpgradeVerificationResultV1::Pending
            && verification_result != ExternalUpgradeVerificationResultV1::Refused,
        status_summary: external_upgrade_verification_summary(verification_result).to_string(),
    };
    report.report_digest = external_upgrade_verification_report_digest(&report);
    Ok(report)
}

/// Validate archived external-upgrade verification report consistency and
/// digest.
pub fn validate_external_upgrade_verification_report(
    report: &ExternalUpgradeVerificationReportV1,
) -> Result<(), ExternalUpgradeVerificationReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ExternalUpgradeVerificationReportError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                actual: report.schema_version,
            },
        );
    }
    ensure_external_verification_report_field("report_id", report.report_id.as_str())?;
    ensure_external_verification_report_field("report_digest", report.report_digest.as_str())?;
    ensure_external_verification_report_field("proposal_id", report.proposal_id.as_str())?;
    ensure_external_verification_report_field("proposal_digest", report.proposal_digest.as_str())?;
    ensure_external_verification_report_field("receipt_id", report.receipt_id.as_str())?;
    ensure_external_verification_report_field("receipt_digest", report.receipt_digest.as_str())?;
    ensure_external_verification_report_field("subject", report.subject.as_str())?;
    ensure_external_verification_report_field("status_summary", report.status_summary.as_str())?;
    if report.status_summary != external_upgrade_verification_summary(report.verification_result) {
        return Err(ExternalUpgradeVerificationReportError::SourceMismatch {
            field: "status_summary",
        });
    }
    if report.report_digest != external_upgrade_verification_report_digest(report) {
        return Err(ExternalUpgradeVerificationReportError::DigestMismatch {
            field: "report_digest",
        });
    }
    Ok(())
}

/// Validate that an archived verification report still matches the
/// proposal/receipt pair it claims to summarize.
pub fn validate_external_upgrade_verification_report_for_receipt(
    report: &ExternalUpgradeVerificationReportV1,
    proposal: &ExternalUpgradeProposalV1,
    receipt: &ExternalUpgradeReceiptV1,
) -> Result<(), ExternalUpgradeVerificationReportError> {
    validate_external_upgrade_verification_report(report)?;
    let expected = external_upgrade_verification_report_from_receipt(
        report.report_id.clone(),
        proposal,
        receipt,
    )?;
    if report != &expected {
        return Err(ExternalUpgradeVerificationReportError::SourceMismatch { field: "receipt" });
    }
    Ok(())
}

/// Build a passive live-inventory verification policy from an external
/// lifecycle proposal.
#[must_use]
pub fn external_upgrade_verification_policy_from_proposal(
    policy_id: impl Into<String>,
    proposal: &ExternalUpgradeProposalV1,
) -> ExternalUpgradeVerificationPolicyV1 {
    let mut policy = ExternalUpgradeVerificationPolicyV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        policy_id: policy_id.into(),
        policy_digest: String::new(),
        proposal_id: proposal.proposal_id.clone(),
        proposal_digest: proposal.proposal_digest.clone(),
        deployment_plan_id: proposal.deployment_plan_id.clone(),
        deployment_plan_digest: proposal.deployment_plan_digest.clone(),
        subject: proposal.subject.clone(),
        canister_id: proposal.canister_id.clone(),
        role: proposal.role.clone(),
        required_verification: proposal.verification_requirements.clone(),
        verification_requirements: external_upgrade_verification_policy_requirements(proposal),
        max_observation_age_seconds: None,
        status_summary: external_upgrade_verification_policy_summary(proposal).to_string(),
    };
    policy.policy_digest = external_upgrade_verification_policy_digest(&policy);
    policy
}

/// Validate archived external-upgrade verification policy consistency and
/// digest.
pub fn validate_external_upgrade_verification_policy(
    policy: &ExternalUpgradeVerificationPolicyV1,
) -> Result<(), ExternalUpgradeVerificationPolicyError> {
    if policy.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ExternalUpgradeVerificationPolicyError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                actual: policy.schema_version,
            },
        );
    }
    ensure_external_verification_policy_field("policy_id", policy.policy_id.as_str())?;
    ensure_external_verification_policy_field("policy_digest", policy.policy_digest.as_str())?;
    ensure_external_verification_policy_field("proposal_id", policy.proposal_id.as_str())?;
    ensure_external_verification_policy_field("proposal_digest", policy.proposal_digest.as_str())?;
    ensure_external_verification_policy_field(
        "deployment_plan_id",
        policy.deployment_plan_id.as_str(),
    )?;
    ensure_external_verification_policy_field(
        "deployment_plan_digest",
        policy.deployment_plan_digest.as_str(),
    )?;
    ensure_external_verification_policy_field("subject", policy.subject.as_str())?;
    ensure_external_verification_policy_field("status_summary", policy.status_summary.as_str())?;
    if policy.policy_digest != external_upgrade_verification_policy_digest(policy) {
        return Err(ExternalUpgradeVerificationPolicyError::DigestMismatch {
            field: "policy_digest",
        });
    }
    Ok(())
}

/// Validate that an archived verification policy still matches its source
/// proposal.
pub fn validate_external_upgrade_verification_policy_for_proposal(
    policy: &ExternalUpgradeVerificationPolicyV1,
    proposal: &ExternalUpgradeProposalV1,
) -> Result<(), ExternalUpgradeVerificationPolicyError> {
    validate_external_upgrade_verification_policy(policy)?;
    let expected =
        external_upgrade_verification_policy_from_proposal(policy.policy_id.clone(), proposal);
    if policy != &expected {
        return Err(ExternalUpgradeVerificationPolicyError::SourceMismatch { field: "proposal" });
    }
    Ok(())
}

/// Build a passive verification check from a policy and supplied observation.
///
/// This evaluates caller-supplied observation facts only. It does not query IC
/// state, deliver consent, execute upgrades, or prove live completion.
#[must_use]
pub fn external_upgrade_verification_check_from_policy(
    check_id: impl Into<String>,
    policy: &ExternalUpgradeVerificationPolicyV1,
    observation: ExternalUpgradeVerificationObservationV1,
) -> ExternalUpgradeVerificationCheckV1 {
    let observation_source = observation.source;
    let requirement_results =
        external_upgrade_verification_check_requirements(policy, &observation);
    let verification_result =
        external_upgrade_verification_check_result(observation_source, &requirement_results);
    let mut check = ExternalUpgradeVerificationCheckV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        check_id: check_id.into(),
        check_digest: String::new(),
        policy_id: policy.policy_id.clone(),
        policy_digest: policy.policy_digest.clone(),
        proposal_id: policy.proposal_id.clone(),
        proposal_digest: policy.proposal_digest.clone(),
        subject: policy.subject.clone(),
        canister_id: policy.canister_id.clone(),
        role: policy.role.clone(),
        observation,
        requirement_results,
        verification_result,
        status_summary: external_upgrade_verification_check_summary(
            observation_source,
            verification_result,
        )
        .to_string(),
    };
    check.check_digest = external_upgrade_verification_check_digest(&check);
    check
}

/// Build a verification observation from an existing deployment-truth check.
///
/// This does not crawl IC state. It reuses the inventory already carried by the
/// deployment-truth check and binds the observation to that check's digest so
/// stale archived inventory fails closed when validated against the policy.
pub fn external_upgrade_verification_observation_from_check(
    policy: &ExternalUpgradeVerificationPolicyV1,
    check: &DeploymentCheckV1,
) -> Result<ExternalUpgradeVerificationObservationV1, ExternalUpgradeVerificationCheckError> {
    validate_external_upgrade_verification_policy(policy)
        .map_err(|_| ExternalUpgradeVerificationCheckError::SourceMismatch { field: "policy" })?;
    if policy.deployment_plan_id != check.plan.plan_id
        || policy.deployment_plan_digest != stable_json_sha256_hex(&check.plan)
    {
        return Err(ExternalUpgradeVerificationCheckError::SourceMismatch {
            field: "deployment_plan",
        });
    }

    let observed = observed_canister_for_verification_policy(&check.inventory, policy);
    Ok(ExternalUpgradeVerificationObservationV1 {
        source: ExternalVerificationObservationSourceV1::DeploymentTruthInventory,
        deployment_check_id: Some(check.check_id.clone()),
        deployment_check_digest: Some(stable_json_sha256_hex(check)),
        inventory_id: Some(check.inventory.inventory_id.clone()),
        observed_at: Some(check.inventory.observed_at.clone()),
        live_inventory_observed: true,
        controller_observation_present: observed.is_some_and(|item| !item.controllers.is_empty()),
        observed_control_class: observed.map(|item| item.control_class),
        observed_module_hash: observed.and_then(|item| item.module_hash.clone()),
        observed_canonical_embedded_config_sha256: observed
            .and_then(|item| item.canonical_embedded_config_digest.clone()),
        protected_call_ready: external_upgrade_protected_call_ready(policy, check),
    })
}

/// Validate archived external-upgrade verification check consistency and
/// digest.
pub fn validate_external_upgrade_verification_check(
    check: &ExternalUpgradeVerificationCheckV1,
) -> Result<(), ExternalUpgradeVerificationCheckError> {
    if check.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ExternalUpgradeVerificationCheckError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                actual: check.schema_version,
            },
        );
    }
    ensure_external_verification_check_field("check_id", check.check_id.as_str())?;
    ensure_external_verification_check_field("check_digest", check.check_digest.as_str())?;
    ensure_external_verification_check_field("policy_id", check.policy_id.as_str())?;
    ensure_external_verification_check_field("policy_digest", check.policy_digest.as_str())?;
    ensure_external_verification_check_field("proposal_id", check.proposal_id.as_str())?;
    ensure_external_verification_check_field("proposal_digest", check.proposal_digest.as_str())?;
    ensure_external_verification_check_field("subject", check.subject.as_str())?;
    ensure_external_verification_check_field("status_summary", check.status_summary.as_str())?;
    if check.observation.source == ExternalVerificationObservationSourceV1::DeploymentTruthInventory
    {
        ensure_external_verification_check_option_field(
            "observation.deployment_check_id",
            check.observation.deployment_check_id.as_deref(),
        )?;
        ensure_external_verification_check_option_field(
            "observation.deployment_check_digest",
            check.observation.deployment_check_digest.as_deref(),
        )?;
        ensure_external_verification_check_option_field(
            "observation.inventory_id",
            check.observation.inventory_id.as_deref(),
        )?;
        ensure_external_verification_check_option_field(
            "observation.observed_at",
            check.observation.observed_at.as_deref(),
        )?;
    }
    validate_external_upgrade_verification_check_requirements(
        check.observation.source,
        &check.requirement_results,
        check.verification_result,
    )?;
    if check.status_summary
        != external_upgrade_verification_check_summary(
            check.observation.source,
            check.verification_result,
        )
    {
        return Err(ExternalUpgradeVerificationCheckError::SourceMismatch {
            field: "status_summary",
        });
    }
    if check.check_digest != external_upgrade_verification_check_digest(check) {
        return Err(ExternalUpgradeVerificationCheckError::DigestMismatch {
            field: "check_digest",
        });
    }
    Ok(())
}

/// Validate that an archived verification check still matches the policy and
/// observation it claims to evaluate.
pub fn validate_external_upgrade_verification_check_for_policy(
    check: &ExternalUpgradeVerificationCheckV1,
    policy: &ExternalUpgradeVerificationPolicyV1,
) -> Result<(), ExternalUpgradeVerificationCheckError> {
    validate_external_upgrade_verification_check(check)?;
    let expected = external_upgrade_verification_check_from_policy(
        check.check_id.clone(),
        policy,
        check.observation.clone(),
    );
    if check != &expected {
        return Err(ExternalUpgradeVerificationCheckError::SourceMismatch { field: "policy" });
    }
    Ok(())
}

/// Validate that a deployment-truth inventory verification check still matches
/// the exact deployment check it claims to use.
pub fn validate_external_upgrade_verification_check_for_deployment_check(
    check: &ExternalUpgradeVerificationCheckV1,
    policy: &ExternalUpgradeVerificationPolicyV1,
    deployment_check: &DeploymentCheckV1,
) -> Result<(), ExternalUpgradeVerificationCheckError> {
    validate_external_upgrade_verification_check_for_policy(check, policy)?;
    let observation =
        external_upgrade_verification_observation_from_check(policy, deployment_check)?;
    let expected = external_upgrade_verification_check_from_policy(
        check.check_id.clone(),
        policy,
        observation,
    );
    if check != &expected {
        return Err(ExternalUpgradeVerificationCheckError::SourceMismatch {
            field: "deployment_check",
        });
    }
    Ok(())
}

/// Build a passive completion report for an external lifecycle proposal.
///
/// This report only combines structural evidence. It does not deliver consent,
/// execute upgrades, query live inventory, or mutate deployment state.
pub fn external_upgrade_completion_report_from_evidence(
    report_id: impl Into<String>,
    proposal: &ExternalUpgradeProposalV1,
    consent_evidence: &ExternalUpgradeConsentEvidenceV1,
    verification_check: &ExternalUpgradeVerificationCheckV1,
) -> Result<ExternalUpgradeCompletionReportV1, ExternalUpgradeCompletionReportError> {
    validate_external_upgrade_proposal(proposal)?;
    validate_external_upgrade_consent_evidence(consent_evidence)?;
    validate_external_upgrade_verification_check(verification_check)?;
    ensure_completion_sources_match_proposal(proposal, consent_evidence, verification_check)?;

    let completion_status = external_upgrade_completion_status(
        consent_evidence.consent_state,
        verification_check.verification_result,
        verification_check.observation.source,
    );
    let blockers = external_upgrade_completion_blockers(completion_status);
    let next_actions = external_upgrade_completion_next_actions(completion_status);
    let mut report = ExternalUpgradeCompletionReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        report_digest: String::new(),
        proposal_id: proposal.proposal_id.clone(),
        proposal_digest: proposal.proposal_digest.clone(),
        consent_evidence_id: consent_evidence.evidence_id.clone(),
        consent_evidence_digest: consent_evidence.evidence_digest.clone(),
        verification_check_id: verification_check.check_id.clone(),
        verification_check_digest: verification_check.check_digest.clone(),
        subject: proposal.subject.clone(),
        canister_id: proposal.canister_id.clone(),
        role: proposal.role.clone(),
        consent_state: consent_evidence.consent_state,
        verification_result: verification_check.verification_result,
        verification_observation_source: verification_check.observation.source,
        completion_status,
        blockers,
        next_actions,
        status_summary: external_upgrade_completion_summary(completion_status).to_string(),
    };
    report.report_digest = external_upgrade_completion_report_digest(&report);
    Ok(report)
}

/// Validate archived completion report consistency and digest.
pub fn validate_external_upgrade_completion_report(
    report: &ExternalUpgradeCompletionReportV1,
) -> Result<(), ExternalUpgradeCompletionReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ExternalUpgradeCompletionReportError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                actual: report.schema_version,
            },
        );
    }
    ensure_external_completion_report_field("report_id", report.report_id.as_str())?;
    ensure_external_completion_report_field("report_digest", report.report_digest.as_str())?;
    ensure_external_completion_report_field("proposal_id", report.proposal_id.as_str())?;
    ensure_external_completion_report_field("proposal_digest", report.proposal_digest.as_str())?;
    ensure_external_completion_report_field(
        "consent_evidence_id",
        report.consent_evidence_id.as_str(),
    )?;
    ensure_external_completion_report_field(
        "consent_evidence_digest",
        report.consent_evidence_digest.as_str(),
    )?;
    ensure_external_completion_report_field(
        "verification_check_id",
        report.verification_check_id.as_str(),
    )?;
    ensure_external_completion_report_field(
        "verification_check_digest",
        report.verification_check_digest.as_str(),
    )?;
    ensure_external_completion_report_field("subject", report.subject.as_str())?;
    ensure_external_completion_report_field("status_summary", report.status_summary.as_str())?;
    if report.completion_status
        != external_upgrade_completion_status(
            report.consent_state,
            report.verification_result,
            report.verification_observation_source,
        )
    {
        return Err(ExternalUpgradeCompletionReportError::SourceMismatch {
            field: "completion_status",
        });
    }
    if report.status_summary != external_upgrade_completion_summary(report.completion_status) {
        return Err(ExternalUpgradeCompletionReportError::SourceMismatch {
            field: "status_summary",
        });
    }
    if report.blockers != external_upgrade_completion_blockers(report.completion_status) {
        return Err(ExternalUpgradeCompletionReportError::SourceMismatch { field: "blockers" });
    }
    if report.next_actions != external_upgrade_completion_next_actions(report.completion_status) {
        return Err(ExternalUpgradeCompletionReportError::SourceMismatch {
            field: "next_actions",
        });
    }
    if report.report_digest != external_upgrade_completion_report_digest(report) {
        return Err(ExternalUpgradeCompletionReportError::DigestMismatch {
            field: "report_digest",
        });
    }
    Ok(())
}

/// Validate that an archived completion report still matches its source
/// proposal, consent evidence, and verification check.
pub fn validate_external_upgrade_completion_report_for_evidence(
    report: &ExternalUpgradeCompletionReportV1,
    proposal: &ExternalUpgradeProposalV1,
    consent_evidence: &ExternalUpgradeConsentEvidenceV1,
    verification_check: &ExternalUpgradeVerificationCheckV1,
) -> Result<(), ExternalUpgradeCompletionReportError> {
    validate_external_upgrade_completion_report(report)?;
    let expected = external_upgrade_completion_report_from_evidence(
        report.report_id.clone(),
        proposal,
        consent_evidence,
        verification_check,
    )?;
    if report != &expected {
        return Err(ExternalUpgradeCompletionReportError::SourceMismatch {
            field: "source_evidence",
        });
    }
    Ok(())
}

/// Build passive external-upgrade proposal artifacts from a lifecycle plan.
///
/// This binds current observations to target artifact facts, but does not
/// grant consent, execute installs, or verify completion.
#[must_use]
pub fn external_upgrade_proposal_report_from_lifecycle_plan(
    report_id: impl Into<String>,
    lifecycle_plan: &ExternalLifecyclePlanV1,
    check: &DeploymentCheckV1,
) -> ExternalUpgradeProposalReportV1 {
    let report_id = report_id.into();
    let mut proposals = Vec::new();
    for authority in lifecycle_plan
        .lifecycle_authority_rows
        .iter()
        .filter(|authority| authority.external_action_required && !authority.blocked)
    {
        proposals.push(external_upgrade_proposal(
            &report_id,
            lifecycle_plan,
            check,
            authority,
            observed_canister_for_authority(&check.inventory, authority),
            target_artifact_for_authority(&check.plan, authority),
        ));
    }

    proposals.sort_by(|left, right| left.subject.cmp(&right.subject));

    let mut report = ExternalUpgradeProposalReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id,
        report_digest: String::new(),
        lifecycle_plan_id: lifecycle_plan.lifecycle_plan_id.clone(),
        lifecycle_plan_digest: lifecycle_plan.lifecycle_plan_digest.clone(),
        deployment_plan_id: lifecycle_plan.deployment_plan_id.clone(),
        deployment_plan_digest: lifecycle_plan.deployment_plan_digest.clone(),
        inventory_id: check.inventory.inventory_id.clone(),
        proposals,
        blocked_subjects: lifecycle_plan
            .blocked_role_upgrades
            .iter()
            .map(|upgrade| upgrade.subject.clone())
            .collect(),
    };
    report.report_digest = external_upgrade_proposal_report_digest(&report);
    report
}

/// Validate archived external-upgrade proposal report consistency and digests.
pub fn validate_external_upgrade_proposal_report(
    report: &ExternalUpgradeProposalReportV1,
) -> Result<(), ExternalUpgradeProposalReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(ExternalUpgradeProposalReportError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            actual: report.schema_version,
        });
    }
    ensure_external_proposal_report_field("report_id", report.report_id.as_str())?;
    ensure_external_proposal_report_field("report_digest", report.report_digest.as_str())?;
    ensure_external_proposal_report_field("lifecycle_plan_id", report.lifecycle_plan_id.as_str())?;
    ensure_external_proposal_report_field(
        "lifecycle_plan_digest",
        report.lifecycle_plan_digest.as_str(),
    )?;
    ensure_external_proposal_report_field(
        "deployment_plan_id",
        report.deployment_plan_id.as_str(),
    )?;
    ensure_external_proposal_report_field(
        "deployment_plan_digest",
        report.deployment_plan_digest.as_str(),
    )?;
    ensure_external_proposal_report_field("inventory_id", report.inventory_id.as_str())?;

    let mut subjects = BTreeSet::new();
    for proposal in &report.proposals {
        if !subjects.insert(proposal.subject.clone()) {
            return Err(ExternalUpgradeProposalReportError::DuplicateSubject {
                subject: proposal.subject.clone(),
            });
        }
        validate_external_upgrade_proposal(proposal)?;
    }
    if report.report_digest != external_upgrade_proposal_report_digest(report) {
        return Err(ExternalUpgradeProposalReportError::DigestMismatch {
            field: "report_digest",
        });
    }
    Ok(())
}

/// Validate that an archived external-upgrade proposal report still matches
/// the lifecycle plan and deployment truth check it claims to derive from.
pub fn validate_external_upgrade_proposal_report_for_lifecycle_plan(
    report: &ExternalUpgradeProposalReportV1,
    lifecycle_plan: &ExternalLifecyclePlanV1,
    check: &DeploymentCheckV1,
) -> Result<(), ExternalUpgradeProposalReportError> {
    validate_external_upgrade_proposal_report(report)?;
    if report.lifecycle_plan_id != lifecycle_plan.lifecycle_plan_id {
        return Err(ExternalUpgradeProposalReportError::SourceMismatch {
            field: "lifecycle_plan_id",
        });
    }
    if report.lifecycle_plan_digest != lifecycle_plan.lifecycle_plan_digest {
        return Err(ExternalUpgradeProposalReportError::SourceMismatch {
            field: "lifecycle_plan_digest",
        });
    }
    let expected = external_upgrade_proposal_report_from_lifecycle_plan(
        report.report_id.clone(),
        lifecycle_plan,
        check,
    );
    if report != &expected {
        return Err(ExternalUpgradeProposalReportError::SourceMismatch {
            field: "deployment_check",
        });
    }
    Ok(())
}
fn external_upgrade_proposal(
    report_id: &str,
    lifecycle_plan: &ExternalLifecyclePlanV1,
    check: &DeploymentCheckV1,
    authority: &LifecycleAuthorityV1,
    observed: Option<&ObservedCanisterV1>,
    target_artifact: Option<&RoleArtifactV1>,
) -> ExternalUpgradeProposalV1 {
    let current_module_hash = observed.and_then(|observed| observed.module_hash.clone());
    let current_canonical_embedded_config_sha256 =
        observed.and_then(|observed| observed.canonical_embedded_config_digest.clone());
    let mut proposal = ExternalUpgradeProposalV1 {
        proposal_id: external_upgrade_proposal_id(report_id, authority.subject.as_str()),
        proposal_digest: String::new(),
        deployment_plan_id: lifecycle_plan.deployment_plan_id.clone(),
        deployment_plan_digest: lifecycle_plan.deployment_plan_digest.clone(),
        lifecycle_plan_id: lifecycle_plan.lifecycle_plan_id.clone(),
        lifecycle_plan_digest: lifecycle_plan.lifecycle_plan_digest.clone(),
        promotion_plan_id: None,
        promotion_plan_digest: None,
        promotion_provenance_id: None,
        promotion_provenance_digest: None,
        subject: authority.subject.clone(),
        canister_id: authority.canister_id.clone(),
        role: authority.role.clone(),
        control_class: authority.control_class,
        lifecycle_mode: authority.lifecycle_mode,
        observed_before_digest: observed_before_digest(
            authority,
            current_module_hash.as_ref(),
            current_canonical_embedded_config_sha256.as_ref(),
        ),
        current_module_hash,
        current_canonical_embedded_config_sha256,
        target_wasm_sha256: target_artifact.and_then(|artifact| artifact.wasm_sha256.clone()),
        target_wasm_gz_sha256: target_artifact.and_then(|artifact| artifact.wasm_gz_sha256.clone()),
        target_installed_module_hash: target_artifact
            .and_then(|artifact| artifact.installed_module_hash.clone()),
        target_role_artifact_identity: target_artifact.map(role_artifact_identity),
        target_canonical_embedded_config_sha256: target_artifact
            .and_then(|artifact| artifact.canonical_embedded_config_sha256.clone()),
        root_trust_anchor: check.plan.trust_domain.root_trust_anchor.clone(),
        authority_profile_hash: check
            .plan
            .deployment_identity
            .authority_profile_hash
            .clone(),
        required_external_action: required_external_action(authority.lifecycle_mode).to_string(),
        consent_requirements: authority.consent_requirements.clone(),
        allowed_authorization_modes: external_upgrade_authorization_modes(authority.control_class),
        verification_requirements: authority.verification_requirements.clone(),
        expires_at: None,
        supersedes_proposal_id: None,
    };
    proposal.proposal_digest = external_upgrade_proposal_digest(&proposal);
    proposal
}

fn validate_external_upgrade_proposal(
    proposal: &ExternalUpgradeProposalV1,
) -> Result<(), ExternalUpgradeProposalReportError> {
    ensure_external_proposal_report_field("proposal_id", proposal.proposal_id.as_str())?;
    ensure_external_proposal_report_field("proposal_digest", proposal.proposal_digest.as_str())?;
    ensure_external_proposal_report_field(
        "proposal.deployment_plan_id",
        proposal.deployment_plan_id.as_str(),
    )?;
    ensure_external_proposal_report_field(
        "proposal.deployment_plan_digest",
        proposal.deployment_plan_digest.as_str(),
    )?;
    ensure_external_proposal_report_field(
        "proposal.lifecycle_plan_id",
        proposal.lifecycle_plan_id.as_str(),
    )?;
    ensure_external_proposal_report_field(
        "proposal.lifecycle_plan_digest",
        proposal.lifecycle_plan_digest.as_str(),
    )?;
    ensure_external_proposal_report_field(
        "proposal.observed_before_digest",
        proposal.observed_before_digest.as_str(),
    )?;
    ensure_external_proposal_report_field("proposal.subject", proposal.subject.as_str())?;
    if proposal.lifecycle_mode == LifecycleModeV1::DirectDeploymentAuthority {
        return Err(
            ExternalUpgradeProposalReportError::DirectLifecycleProposal {
                subject: proposal.subject.clone(),
            },
        );
    }
    if proposal.proposal_digest != external_upgrade_proposal_digest(proposal) {
        return Err(ExternalUpgradeProposalReportError::DigestMismatch {
            field: "proposal_digest",
        });
    }
    Ok(())
}

fn observed_canister_for_authority<'a>(
    inventory: &'a DeploymentInventoryV1,
    authority: &LifecycleAuthorityV1,
) -> Option<&'a ObservedCanisterV1> {
    if let Some(canister_id) = &authority.canister_id
        && let Some(observed) = inventory
            .observed_canisters
            .iter()
            .find(|observed| &observed.canister_id == canister_id)
    {
        return Some(observed);
    }
    inventory
        .observed_canisters
        .iter()
        .find(|observed| observed.role == authority.role)
}

fn observed_canister_for_verification_policy<'a>(
    inventory: &'a DeploymentInventoryV1,
    policy: &ExternalUpgradeVerificationPolicyV1,
) -> Option<&'a ObservedCanisterV1> {
    if let Some(canister_id) = &policy.canister_id
        && let Some(observed) = inventory
            .observed_canisters
            .iter()
            .find(|observed| &observed.canister_id == canister_id)
    {
        return Some(observed);
    }
    inventory
        .observed_canisters
        .iter()
        .find(|observed| observed.role == policy.role)
}

fn external_upgrade_protected_call_ready(
    policy: &ExternalUpgradeVerificationPolicyV1,
    check: &DeploymentCheckV1,
) -> Option<bool> {
    policy
        .required_verification
        .contains(&LifecycleVerificationRequirementV1::ProtectedCallReadiness)
        .then_some(
            check.inventory.observed_verifier_readiness.status == ObservationStatusV1::Observed,
        )
}

fn target_artifact_for_authority<'a>(
    plan: &'a DeploymentPlanV1,
    authority: &LifecycleAuthorityV1,
) -> Option<&'a RoleArtifactV1> {
    let role = authority.role.as_ref()?;
    plan.role_artifacts
        .iter()
        .find(|artifact| &artifact.role == role)
}

const fn required_external_action(lifecycle_mode: LifecycleModeV1) -> &'static str {
    match lifecycle_mode {
        LifecycleModeV1::DirectDeploymentAuthority => "none",
        LifecycleModeV1::ProposalRequired => "proposal_and_consent",
        LifecycleModeV1::DelegatedInstallRequired => "delegated_install_or_pool_policy",
        LifecycleModeV1::ExternalCompletionOnly => "external_controller_execution",
        LifecycleModeV1::VerifyOnly => "verify_external_completion",
        LifecycleModeV1::MustNotTouch | LifecycleModeV1::UnknownUnsafeBlocked => "blocked",
    }
}

fn role_artifact_identity(artifact: &RoleArtifactV1) -> String {
    stable_json_sha256_hex(&(
        artifact.role.as_str(),
        artifact.wasm_sha256.as_deref(),
        artifact.wasm_gz_sha256.as_deref(),
        artifact.installed_module_hash.as_deref(),
        artifact.candid_sha256.as_deref(),
        artifact.canonical_embedded_config_sha256.as_deref(),
    ))
}

fn external_upgrade_authorization_modes(
    control_class: CanisterControlClassV1,
) -> Vec<ExternalUpgradeAuthorizationModeV1> {
    match control_class {
        CanisterControlClassV1::JointlyControlled => vec![
            ExternalUpgradeAuthorizationModeV1::ConsentForDirectInstall,
            ExternalUpgradeAuthorizationModeV1::DelegatedInstallAuthority,
            ExternalUpgradeAuthorizationModeV1::ExternalControllerExecution,
            ExternalUpgradeAuthorizationModeV1::ObserveAndVerifyOnly,
        ],
        CanisterControlClassV1::CanicManagedPool
        | CanisterControlClassV1::ExternallyImported
        | CanisterControlClassV1::UserControlled => vec![
            ExternalUpgradeAuthorizationModeV1::DelegatedInstallAuthority,
            ExternalUpgradeAuthorizationModeV1::ExternalControllerExecution,
            ExternalUpgradeAuthorizationModeV1::ObserveAndVerifyOnly,
        ],
        CanisterControlClassV1::DeploymentControlled | CanisterControlClassV1::UnknownUnsafe => {
            Vec::new()
        }
    }
}

fn external_upgrade_proposal_id(report_id: &str, subject: &str) -> String {
    let subject = subject.replace([':', '/'], "-");
    format!("{report_id}:{subject}")
}

fn external_upgrade_verification_result(
    consent_state: ExternalUpgradeConsentStateV1,
    proposal: &ExternalUpgradeProposalV1,
    observed_after_module_hash: Option<&str>,
    observed_after_config: Option<&str>,
) -> ExternalUpgradeVerificationResultV1 {
    match consent_state {
        ExternalUpgradeConsentStateV1::Pending => ExternalUpgradeVerificationResultV1::Pending,
        ExternalUpgradeConsentStateV1::Refused => ExternalUpgradeVerificationResultV1::Refused,
        ExternalUpgradeConsentStateV1::Delegated
        | ExternalUpgradeConsentStateV1::ExecutedExternally => {
            if external_upgrade_observation_matches(
                proposal.target_installed_module_hash.as_deref(),
                observed_after_module_hash,
            ) && external_upgrade_observation_matches(
                proposal.target_canonical_embedded_config_sha256.as_deref(),
                observed_after_config,
            ) {
                ExternalUpgradeVerificationResultV1::Verified
            } else {
                ExternalUpgradeVerificationResultV1::Mismatch
            }
        }
    }
}

fn external_upgrade_verification_notes(
    verification_result: ExternalUpgradeVerificationResultV1,
    proposal: &ExternalUpgradeProposalV1,
    observed_after_module_hash: Option<&str>,
    observed_after_config: Option<&str>,
) -> Vec<String> {
    let mut notes = Vec::new();
    if verification_result == ExternalUpgradeVerificationResultV1::Mismatch {
        if !external_upgrade_observation_matches(
            proposal.target_installed_module_hash.as_deref(),
            observed_after_module_hash,
        ) {
            notes.push("observed module hash does not match proposal target".to_string());
        }
        if !external_upgrade_observation_matches(
            proposal.target_canonical_embedded_config_sha256.as_deref(),
            observed_after_config,
        ) {
            notes.push("observed embedded config does not match proposal target".to_string());
        }
    }
    notes
}
const fn external_upgrade_verification_summary(
    result: ExternalUpgradeVerificationResultV1,
) -> &'static str {
    match result {
        ExternalUpgradeVerificationResultV1::Pending => {
            "external action has not been reported as complete"
        }
        ExternalUpgradeVerificationResultV1::Refused => "external consent was refused",
        ExternalUpgradeVerificationResultV1::Verified => {
            "reported external completion matches proposal target facts"
        }
        ExternalUpgradeVerificationResultV1::Mismatch => {
            "reported external completion does not match proposal target facts"
        }
    }
}

fn external_upgrade_verification_policy_summary(
    proposal: &ExternalUpgradeProposalV1,
) -> &'static str {
    if proposal
        .verification_requirements
        .contains(&LifecycleVerificationRequirementV1::ProtectedCallReadiness)
    {
        "fresh live inventory, module/config facts, controller observation, and protected-call readiness are required"
    } else {
        "fresh live inventory, module/config facts, and controller observation are required"
    }
}

fn external_upgrade_verification_policy_requirements(
    proposal: &ExternalUpgradeProposalV1,
) -> Vec<ExternalUpgradeVerificationPolicyRequirementV1> {
    [
        (LifecycleVerificationRequirementV1::LiveInventory, None),
        (
            LifecycleVerificationRequirementV1::ControllerObservation,
            Some(control_class_value(proposal.control_class)),
        ),
        (
            LifecycleVerificationRequirementV1::ModuleHash,
            proposal.target_installed_module_hash.clone(),
        ),
        (
            LifecycleVerificationRequirementV1::CanonicalEmbeddedConfig,
            proposal.target_canonical_embedded_config_sha256.clone(),
        ),
        (
            LifecycleVerificationRequirementV1::ProtectedCallReadiness,
            None,
        ),
    ]
    .into_iter()
    .map(
        |(requirement, expected_value)| ExternalUpgradeVerificationPolicyRequirementV1 {
            requirement,
            status: if proposal.verification_requirements.contains(&requirement) {
                ExternalUpgradeVerificationRequirementStatusV1::Required
            } else {
                ExternalUpgradeVerificationRequirementStatusV1::NotRequired
            },
            expected_value,
        },
    )
    .collect()
}

fn external_upgrade_verification_check_requirements(
    policy: &ExternalUpgradeVerificationPolicyV1,
    observation: &ExternalUpgradeVerificationObservationV1,
) -> Vec<ExternalUpgradeVerificationCheckRequirementV1> {
    policy
        .verification_requirements
        .iter()
        .map(|row| {
            let observed_value =
                external_upgrade_verification_observed_value(row.requirement, observation);
            let satisfied =
                if row.status == ExternalUpgradeVerificationRequirementStatusV1::Required {
                    Some(external_upgrade_verification_requirement_satisfied(
                        row.requirement,
                        row.expected_value.as_deref(),
                        observed_value.as_deref(),
                        observation,
                    ))
                } else {
                    None
                };
            ExternalUpgradeVerificationCheckRequirementV1 {
                requirement: row.requirement,
                status: row.status,
                expected_value: row.expected_value.clone(),
                observed_value,
                satisfied,
            }
        })
        .collect()
}

fn external_upgrade_verification_observed_value(
    requirement: LifecycleVerificationRequirementV1,
    observation: &ExternalUpgradeVerificationObservationV1,
) -> Option<String> {
    match requirement {
        LifecycleVerificationRequirementV1::LiveInventory => {
            Some(observation.live_inventory_observed.to_string())
        }
        LifecycleVerificationRequirementV1::ControllerObservation => {
            observation.observed_control_class.map(control_class_value)
        }
        LifecycleVerificationRequirementV1::ModuleHash => observation.observed_module_hash.clone(),
        LifecycleVerificationRequirementV1::CanonicalEmbeddedConfig => observation
            .observed_canonical_embedded_config_sha256
            .clone(),
        LifecycleVerificationRequirementV1::ProtectedCallReadiness => observation
            .protected_call_ready
            .map(|value| value.to_string()),
    }
}

fn external_upgrade_verification_requirement_satisfied(
    requirement: LifecycleVerificationRequirementV1,
    expected_value: Option<&str>,
    observed_value: Option<&str>,
    observation: &ExternalUpgradeVerificationObservationV1,
) -> bool {
    match requirement {
        LifecycleVerificationRequirementV1::LiveInventory => observation.live_inventory_observed,
        LifecycleVerificationRequirementV1::ControllerObservation => {
            observation.controller_observation_present
                && expected_value.is_some_and(|expected| observed_value == Some(expected))
        }
        LifecycleVerificationRequirementV1::ModuleHash
        | LifecycleVerificationRequirementV1::CanonicalEmbeddedConfig => {
            expected_value.is_some_and(|expected| observed_value == Some(expected))
        }
        LifecycleVerificationRequirementV1::ProtectedCallReadiness => {
            observation.protected_call_ready == Some(true)
        }
    }
}

fn external_upgrade_verification_check_result(
    source: ExternalVerificationObservationSourceV1,
    requirements: &[ExternalUpgradeVerificationCheckRequirementV1],
) -> ExternalUpgradeVerificationResultV1 {
    if !requirements
        .iter()
        .filter(|row| row.status == ExternalUpgradeVerificationRequirementStatusV1::Required)
        .all(|row| row.satisfied == Some(true))
    {
        return ExternalUpgradeVerificationResultV1::Mismatch;
    }

    match source {
        ExternalVerificationObservationSourceV1::DeploymentTruthInventory => {
            ExternalUpgradeVerificationResultV1::Verified
        }
        ExternalVerificationObservationSourceV1::SuppliedObservation => {
            ExternalUpgradeVerificationResultV1::Pending
        }
    }
}

fn control_class_value(control_class: CanisterControlClassV1) -> String {
    format!("{control_class:?}")
}

fn validate_external_upgrade_verification_check_requirements(
    source: ExternalVerificationObservationSourceV1,
    requirements: &[ExternalUpgradeVerificationCheckRequirementV1],
    result: ExternalUpgradeVerificationResultV1,
) -> Result<(), ExternalUpgradeVerificationCheckError> {
    if requirements.is_empty() {
        return Err(
            ExternalUpgradeVerificationCheckError::MissingRequiredField {
                field: "requirement_results",
            },
        );
    }
    let mut seen = BTreeSet::new();
    for row in requirements {
        if !seen.insert(row.requirement) {
            return Err(
                ExternalUpgradeVerificationCheckError::DuplicateRequirement {
                    requirement: row.requirement,
                },
            );
        }
        match row.status {
            ExternalUpgradeVerificationRequirementStatusV1::Required => {
                if row.satisfied.is_none() {
                    return Err(
                        ExternalUpgradeVerificationCheckError::RequirementStatusMismatch {
                            requirement: row.requirement,
                        },
                    );
                }
            }
            ExternalUpgradeVerificationRequirementStatusV1::NotRequired => {
                if row.satisfied.is_some() {
                    return Err(
                        ExternalUpgradeVerificationCheckError::RequirementStatusMismatch {
                            requirement: row.requirement,
                        },
                    );
                }
            }
        }
    }
    if external_upgrade_verification_check_result(source, requirements) != result {
        return Err(ExternalUpgradeVerificationCheckError::SourceMismatch {
            field: "verification_result",
        });
    }
    Ok(())
}

const fn external_upgrade_completion_status(
    consent_state: ExternalUpgradeConsentStateV1,
    verification_result: ExternalUpgradeVerificationResultV1,
    source: ExternalVerificationObservationSourceV1,
) -> ExternalUpgradeCompletionStatusV1 {
    match consent_state {
        ExternalUpgradeConsentStateV1::Pending => {
            ExternalUpgradeCompletionStatusV1::AwaitingConsent
        }
        ExternalUpgradeConsentStateV1::Refused => ExternalUpgradeCompletionStatusV1::ConsentRefused,
        ExternalUpgradeConsentStateV1::Delegated
        | ExternalUpgradeConsentStateV1::ExecutedExternally => match verification_result {
            ExternalUpgradeVerificationResultV1::Verified => match source {
                ExternalVerificationObservationSourceV1::DeploymentTruthInventory => {
                    ExternalUpgradeCompletionStatusV1::VerifiedComplete
                }
                ExternalVerificationObservationSourceV1::SuppliedObservation => {
                    ExternalUpgradeCompletionStatusV1::SuppliedEvidenceConsistent
                }
            },
            ExternalUpgradeVerificationResultV1::Mismatch => {
                ExternalUpgradeCompletionStatusV1::VerificationFailed
            }
            ExternalUpgradeVerificationResultV1::Pending
            | ExternalUpgradeVerificationResultV1::Refused => {
                ExternalUpgradeCompletionStatusV1::AwaitingVerification
            }
        },
    }
}

fn external_upgrade_completion_blockers(status: ExternalUpgradeCompletionStatusV1) -> Vec<String> {
    match status {
        ExternalUpgradeCompletionStatusV1::AwaitingConsent => {
            vec!["external consent or action has not been reported".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::ConsentRefused => {
            vec!["external consent was refused".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::SuppliedEvidenceConsistent => {
            vec!["supplied evidence is consistent but not live inventory proof".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::AwaitingVerification => {
            vec!["external action requires verification against live inventory".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::VerificationFailed => {
            vec!["supplied observation does not satisfy verification policy".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::VerifiedComplete => Vec::new(),
    }
}

fn external_upgrade_completion_next_actions(
    status: ExternalUpgradeCompletionStatusV1,
) -> Vec<String> {
    match status {
        ExternalUpgradeCompletionStatusV1::AwaitingConsent => {
            vec!["obtain external consent or reported external execution".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::ConsentRefused => {
            vec!["do not execute; supersede the proposal before retry".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::SuppliedEvidenceConsistent => {
            vec!["collect deployment-truth inventory and run verification check".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::AwaitingVerification => {
            vec!["collect fresh inventory observations and run verification check".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::VerificationFailed => {
            vec!["resolve observed module/config/readiness mismatch".to_string()]
        }
        ExternalUpgradeCompletionStatusV1::VerifiedComplete => {
            vec!["record external lifecycle item as verified complete".to_string()]
        }
    }
}

const fn external_upgrade_completion_summary(
    status: ExternalUpgradeCompletionStatusV1,
) -> &'static str {
    match status {
        ExternalUpgradeCompletionStatusV1::AwaitingConsent => {
            "external lifecycle item is waiting for consent or external action"
        }
        ExternalUpgradeCompletionStatusV1::ConsentRefused => "external lifecycle item was refused",
        ExternalUpgradeCompletionStatusV1::SuppliedEvidenceConsistent => {
            "external lifecycle supplied evidence is consistent but awaits inventory verification"
        }
        ExternalUpgradeCompletionStatusV1::AwaitingVerification => {
            "external lifecycle item needs verification before completion"
        }
        ExternalUpgradeCompletionStatusV1::VerifiedComplete => {
            "external lifecycle item is structurally verified complete"
        }
        ExternalUpgradeCompletionStatusV1::VerificationFailed => {
            "external lifecycle item failed supplied verification"
        }
    }
}

const fn external_upgrade_verification_check_summary(
    source: ExternalVerificationObservationSourceV1,
    result: ExternalUpgradeVerificationResultV1,
) -> &'static str {
    match result {
        ExternalUpgradeVerificationResultV1::Verified => {
            "deployment-truth inventory satisfies required verification postconditions"
        }
        ExternalUpgradeVerificationResultV1::Mismatch => {
            "verification observation does not satisfy required verification postconditions"
        }
        ExternalUpgradeVerificationResultV1::Pending => match source {
            ExternalVerificationObservationSourceV1::SuppliedObservation => {
                "supplied observation is consistent; deployment-truth inventory verification is required"
            }
            ExternalVerificationObservationSourceV1::DeploymentTruthInventory => {
                "external verification check is pending inventory observation"
            }
        },
        ExternalUpgradeVerificationResultV1::Refused => {
            "external verification check reflects refused consent"
        }
    }
}

const fn external_upgrade_consent_summary(state: ExternalUpgradeConsentStateV1) -> &'static str {
    match state {
        ExternalUpgradeConsentStateV1::Pending => {
            "external consent or action has not been reported"
        }
        ExternalUpgradeConsentStateV1::Refused => "external consent was refused",
        ExternalUpgradeConsentStateV1::Delegated => "delegated install authority was reported",
        ExternalUpgradeConsentStateV1::ExecutedExternally => {
            "external controller execution was reported"
        }
    }
}

fn external_upgrade_observation_matches(expected: Option<&str>, observed: Option<&str>) -> bool {
    expected.is_none_or(|expected| observed == Some(expected))
}

fn ensure_external_receipt_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalUpgradeReceiptError> {
    if value.trim().is_empty() {
        return Err(ExternalUpgradeReceiptError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_external_receipt_matches_proposal(
    field: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), ExternalUpgradeReceiptError> {
    if actual != expected {
        return Err(ExternalUpgradeReceiptError::SourceMismatch { field });
    }
    Ok(())
}

fn ensure_external_receipt_option_matches_proposal(
    field: &'static str,
    actual: Option<&str>,
    expected: Option<&str>,
) -> Result<(), ExternalUpgradeReceiptError> {
    if actual != expected {
        return Err(ExternalUpgradeReceiptError::SourceMismatch { field });
    }
    Ok(())
}

fn ensure_external_consent_evidence_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalUpgradeConsentEvidenceError> {
    if value.trim().is_empty() {
        return Err(ExternalUpgradeConsentEvidenceError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_external_verification_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalUpgradeVerificationReportError> {
    if value.trim().is_empty() {
        return Err(ExternalUpgradeVerificationReportError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_external_verification_policy_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalUpgradeVerificationPolicyError> {
    if value.trim().is_empty() {
        return Err(ExternalUpgradeVerificationPolicyError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_external_verification_check_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalUpgradeVerificationCheckError> {
    if value.trim().is_empty() {
        return Err(ExternalUpgradeVerificationCheckError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_external_verification_check_option_field(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), ExternalUpgradeVerificationCheckError> {
    if value.is_none_or(|value| value.trim().is_empty()) {
        return Err(ExternalUpgradeVerificationCheckError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_external_completion_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalUpgradeCompletionReportError> {
    if value.trim().is_empty() {
        return Err(ExternalUpgradeCompletionReportError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_completion_sources_match_proposal(
    proposal: &ExternalUpgradeProposalV1,
    consent_evidence: &ExternalUpgradeConsentEvidenceV1,
    verification_check: &ExternalUpgradeVerificationCheckV1,
) -> Result<(), ExternalUpgradeCompletionReportError> {
    ensure_completion_source_field(
        "consent_evidence.proposal_id",
        consent_evidence.proposal_id.as_str(),
        proposal.proposal_id.as_str(),
    )?;
    ensure_completion_source_field(
        "consent_evidence.proposal_digest",
        consent_evidence.proposal_digest.as_str(),
        proposal.proposal_digest.as_str(),
    )?;
    ensure_completion_source_field(
        "verification_check.proposal_id",
        verification_check.proposal_id.as_str(),
        proposal.proposal_id.as_str(),
    )?;
    ensure_completion_source_field(
        "verification_check.proposal_digest",
        verification_check.proposal_digest.as_str(),
        proposal.proposal_digest.as_str(),
    )?;
    ensure_completion_source_field(
        "consent_evidence.subject",
        consent_evidence.subject.as_str(),
        proposal.subject.as_str(),
    )?;
    ensure_completion_source_field(
        "verification_check.subject",
        verification_check.subject.as_str(),
        proposal.subject.as_str(),
    )?;
    ensure_completion_option_source_field(
        "consent_evidence.canister_id",
        consent_evidence.canister_id.as_deref(),
        proposal.canister_id.as_deref(),
    )?;
    ensure_completion_option_source_field(
        "verification_check.canister_id",
        verification_check.canister_id.as_deref(),
        proposal.canister_id.as_deref(),
    )?;
    ensure_completion_option_source_field(
        "consent_evidence.role",
        consent_evidence.role.as_deref(),
        proposal.role.as_deref(),
    )?;
    ensure_completion_option_source_field(
        "verification_check.role",
        verification_check.role.as_deref(),
        proposal.role.as_deref(),
    )
}

fn ensure_completion_source_field(
    field: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), ExternalUpgradeCompletionReportError> {
    if actual != expected {
        return Err(ExternalUpgradeCompletionReportError::SourceMismatch { field });
    }
    Ok(())
}

fn ensure_completion_option_source_field(
    field: &'static str,
    actual: Option<&str>,
    expected: Option<&str>,
) -> Result<(), ExternalUpgradeCompletionReportError> {
    if actual != expected {
        return Err(ExternalUpgradeCompletionReportError::SourceMismatch { field });
    }
    Ok(())
}

fn ensure_external_proposal_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalUpgradeProposalReportError> {
    if value.trim().is_empty() {
        return Err(ExternalUpgradeProposalReportError::MissingRequiredField { field });
    }
    Ok(())
}
