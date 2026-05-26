use super::*;
use serde::Serialize;
use std::collections::BTreeSet;

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
struct ObservedBeforeDigestInput<'a> {
    subject: &'a str,
    canister_id: &'a Option<String>,
    role: &'a Option<String>,
    observed_controllers: &'a [String],
    current_module_hash: Option<&'a String>,
    current_canonical_embedded_config_sha256: Option<&'a String>,
}

///
/// ExternalUpgradeReceiptError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ExternalUpgradeReceiptError {
    #[error("external upgrade receipt schema version {actual} does not match expected {expected}")]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("external upgrade receipt field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("external upgrade receipt field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("external upgrade receipt field `{field}` does not match proposal source")]
    SourceMismatch { field: &'static str },
    #[error("external upgrade receipt verification result does not match observations")]
    VerificationMismatch,
    #[error("external upgrade receipt refused consent cannot be verified")]
    RefusedConsentVerified,
}

///
/// ExternalUpgradeConsentEvidenceError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ExternalUpgradeConsentEvidenceError {
    #[error(
        "external upgrade consent evidence schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("external upgrade consent evidence field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("external upgrade consent evidence field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("external upgrade consent evidence field `{field}` no longer matches source receipt")]
    SourceMismatch { field: &'static str },
    #[error(transparent)]
    Receipt(#[from] ExternalUpgradeReceiptError),
}

///
/// ExternalUpgradeVerificationReportError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ExternalUpgradeVerificationReportError {
    #[error(
        "external upgrade verification report schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("external upgrade verification report field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("external upgrade verification report field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("external upgrade verification report field `{field}` does not match source evidence")]
    SourceMismatch { field: &'static str },
    #[error(transparent)]
    Receipt(#[from] ExternalUpgradeReceiptError),
}

///
/// LifecycleAuthorityReportError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum LifecycleAuthorityReportError {
    #[error(
        "lifecycle authority report schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("lifecycle authority report field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("lifecycle authority report field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("lifecycle authority report contains duplicate subject `{subject}`")]
    DuplicateSubject { subject: String },
    #[error("lifecycle authority report counters do not match authority rows")]
    CountMismatch,
}

///
/// ExternalLifecyclePlanError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ExternalLifecyclePlanError {
    #[error("external lifecycle plan schema version {actual} does not match expected {expected}")]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("external lifecycle plan field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("external lifecycle plan field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("external lifecycle plan field `{field}` does not match deployment truth source")]
    SourceMismatch { field: &'static str },
    #[error("external lifecycle plan status does not match role partitioning")]
    StatusMismatch,
    #[error("external lifecycle plan contains duplicate subject `{subject}`")]
    DuplicateSubject { subject: String },
}

///
/// ExternalUpgradeProposalReportError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ExternalUpgradeProposalReportError {
    #[error(
        "external upgrade proposal report schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("external upgrade proposal report field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("external upgrade proposal report field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("external upgrade proposal report field `{field}` does not match lifecycle source")]
    SourceMismatch { field: &'static str },
    #[error(
        "external upgrade proposal report contains proposal for directly controlled row `{subject}`"
    )]
    DirectLifecycleProposal { subject: String },
    #[error("external upgrade proposal report contains duplicate subject `{subject}`")]
    DuplicateSubject { subject: String },
}

///
/// ExternalLifecyclePendingReportError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ExternalLifecyclePendingReportError {
    #[error(
        "external lifecycle pending report schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("external lifecycle pending report field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("external lifecycle pending report field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("external lifecycle pending report field `{field}` does not match lifecycle source")]
    SourceMismatch { field: &'static str },
    #[error("external lifecycle pending report counters do not match action rows")]
    CountMismatch,
    #[error("external lifecycle pending report contains duplicate subject `{subject}`")]
    DuplicateSubject { subject: String },
}

///
/// CriticalExternalFixReportError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum CriticalExternalFixReportError {
    #[error(
        "critical external fix report schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("critical external fix report field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("critical external fix report field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("critical external fix report field `{field}` does not match lifecycle source")]
    SourceMismatch { field: &'static str },
}

/// Project the existing deployment truth control classifications into the 0.45
/// lifecycle-authority view. This is observational and must not mutate IC or
/// local deployment state.
#[must_use]
pub fn lifecycle_authority_report_from_check(
    report_id: impl Into<String>,
    check: &DeploymentCheckV1,
) -> LifecycleAuthorityReportV1 {
    let mut authorities = Vec::new();
    let mut seen_subjects = BTreeSet::new();

    for expected in &check.plan.expected_canisters {
        let observed = observed_canister_for_expected(&check.inventory, expected);
        let authority = lifecycle_authority_for_expected_canister(&check.plan, expected, observed);
        seen_subjects.insert(authority.subject.clone());
        authorities.push(authority);
    }

    for expected in &check.plan.expected_pool {
        let observed = observed_pool_for_expected(&check.inventory, expected);
        let authority = lifecycle_authority_for_expected_pool(expected, observed);
        seen_subjects.insert(authority.subject.clone());
        authorities.push(authority);
    }

    for observed in &check.inventory.observed_canisters {
        let subject = lifecycle_subject(observed.canister_id.as_str(), observed.role.as_deref());
        if seen_subjects.contains(&subject) {
            continue;
        }
        authorities.push(lifecycle_authority_for_unplanned_canister(observed));
    }

    for observed in &check.inventory.observed_pool {
        let subject = lifecycle_subject(observed.canister_id.as_str(), observed.role.as_deref());
        if seen_subjects.contains(&subject) {
            continue;
        }
        authorities.push(lifecycle_authority_for_unplanned_pool(observed));
    }

    authorities.sort_by(|left, right| left.subject.cmp(&right.subject));
    let external_action_required_count = authorities
        .iter()
        .filter(|authority| authority.external_action_required)
        .count();
    let blocked_count = authorities
        .iter()
        .filter(|authority| authority.blocked)
        .count();

    let mut report = LifecycleAuthorityReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        report_digest: String::new(),
        check_id: check.check_id.clone(),
        plan_id: check.plan.plan_id.clone(),
        inventory_id: check.inventory.inventory_id.clone(),
        authorities,
        external_action_required_count,
        blocked_count,
    };
    report.report_digest = lifecycle_authority_report_digest(&report);
    report
}

/// Validate archived lifecycle authority report consistency and digests.
pub fn validate_lifecycle_authority_report(
    report: &LifecycleAuthorityReportV1,
) -> Result<(), LifecycleAuthorityReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(LifecycleAuthorityReportError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            actual: report.schema_version,
        });
    }
    ensure_lifecycle_authority_report_field("report_id", report.report_id.as_str())?;
    ensure_lifecycle_authority_report_field("report_digest", report.report_digest.as_str())?;
    ensure_lifecycle_authority_report_field("check_id", report.check_id.as_str())?;
    ensure_lifecycle_authority_report_field("plan_id", report.plan_id.as_str())?;
    ensure_lifecycle_authority_report_field("inventory_id", report.inventory_id.as_str())?;
    ensure_unique_authority_subjects(&report.authorities)?;
    if report.external_action_required_count
        != report
            .authorities
            .iter()
            .filter(|authority| authority.external_action_required)
            .count()
        || report.blocked_count
            != report
                .authorities
                .iter()
                .filter(|authority| authority.blocked)
                .count()
    {
        return Err(LifecycleAuthorityReportError::CountMismatch);
    }
    if report.report_digest != lifecycle_authority_report_digest(report) {
        return Err(LifecycleAuthorityReportError::DigestMismatch {
            field: "report_digest",
        });
    }
    Ok(())
}

/// Build the central 0.45 lifecycle plan from deployment truth.
///
/// This partitions roles into directly executable, externally proposed, and
/// blocked lifecycle rows. It is passive and does not perform proposal
/// delivery, consent, or execution.
#[must_use]
pub fn external_lifecycle_plan_from_check(
    lifecycle_plan_id: impl Into<String>,
    lifecycle_authority_report_id: impl Into<String>,
    check: &DeploymentCheckV1,
) -> ExternalLifecyclePlanV1 {
    let lifecycle_authority_report =
        lifecycle_authority_report_from_check(lifecycle_authority_report_id, check);
    let lifecycle_authority_rows = lifecycle_authority_report.authorities;
    let directly_executable_role_upgrades = lifecycle_authority_rows
        .iter()
        .filter(|authority| {
            authority.lifecycle_mode == LifecycleModeV1::DirectDeploymentAuthority
                && !authority.blocked
        })
        .map(external_lifecycle_role_upgrade)
        .collect::<Vec<_>>();
    let proposed_external_role_upgrades = lifecycle_authority_rows
        .iter()
        .filter(|authority| authority.external_action_required && !authority.blocked)
        .map(external_lifecycle_role_upgrade)
        .collect::<Vec<_>>();
    let blocked_role_upgrades = lifecycle_authority_rows
        .iter()
        .filter(|authority| authority.blocked)
        .map(external_lifecycle_role_upgrade)
        .collect::<Vec<_>>();
    let residual_exposure = proposed_external_role_upgrades
        .iter()
        .map(|upgrade| {
            format!(
                "{} remains pending external lifecycle action",
                upgrade.subject
            )
        })
        .collect::<Vec<_>>();
    let status = if !blocked_role_upgrades.is_empty() {
        ExternalLifecyclePlanStatusV1::Blocked
    } else if !proposed_external_role_upgrades.is_empty() {
        ExternalLifecyclePlanStatusV1::PendingExternalAction
    } else {
        ExternalLifecyclePlanStatusV1::Ready
    };
    let deployment_plan_digest = stable_json_sha256_hex(&check.plan);
    let mut plan = ExternalLifecyclePlanV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        lifecycle_plan_id: lifecycle_plan_id.into(),
        lifecycle_plan_digest: String::new(),
        lifecycle_authority_report_id: lifecycle_authority_report.report_id,
        deployment_plan_id: check.plan.plan_id.clone(),
        deployment_plan_digest,
        inventory_id: check.inventory.inventory_id.clone(),
        lifecycle_authority_rows,
        directly_executable_role_upgrades,
        proposed_external_role_upgrades,
        blocked_role_upgrades,
        dependency_blockers: Vec::new(),
        protected_call_implications: protected_call_implications_for_check(check),
        residual_exposure,
        status,
    };
    plan.lifecycle_plan_digest = external_lifecycle_plan_digest(&plan);
    plan
}

/// Validate archived external lifecycle plan consistency and digests.
pub fn validate_external_lifecycle_plan(
    plan: &ExternalLifecyclePlanV1,
) -> Result<(), ExternalLifecyclePlanError> {
    if plan.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(ExternalLifecyclePlanError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            actual: plan.schema_version,
        });
    }
    ensure_external_lifecycle_plan_field("lifecycle_plan_id", plan.lifecycle_plan_id.as_str())?;
    ensure_external_lifecycle_plan_field(
        "lifecycle_authority_report_id",
        plan.lifecycle_authority_report_id.as_str(),
    )?;
    ensure_external_lifecycle_plan_field("deployment_plan_id", plan.deployment_plan_id.as_str())?;
    ensure_external_lifecycle_plan_field("inventory_id", plan.inventory_id.as_str())?;
    if plan.lifecycle_plan_digest != external_lifecycle_plan_digest(plan) {
        return Err(ExternalLifecyclePlanError::DigestMismatch {
            field: "lifecycle_plan_digest",
        });
    }
    if plan.status != expected_lifecycle_plan_status(plan) {
        return Err(ExternalLifecyclePlanError::StatusMismatch);
    }
    ensure_unique_lifecycle_subjects(&plan.lifecycle_authority_rows)?;
    ensure_unique_role_upgrade_subjects(&plan.directly_executable_role_upgrades)?;
    ensure_unique_role_upgrade_subjects(&plan.proposed_external_role_upgrades)?;
    ensure_unique_role_upgrade_subjects(&plan.blocked_role_upgrades)?;
    Ok(())
}

/// Validate that an archived external lifecycle plan still matches its source
/// deployment truth check.
pub fn validate_external_lifecycle_plan_for_check(
    plan: &ExternalLifecyclePlanV1,
    check: &DeploymentCheckV1,
) -> Result<(), ExternalLifecyclePlanError> {
    validate_external_lifecycle_plan(plan)?;
    let expected = external_lifecycle_plan_from_check(
        plan.lifecycle_plan_id.clone(),
        plan.lifecycle_authority_report_id.clone(),
        check,
    );
    if plan != &expected {
        return Err(ExternalLifecyclePlanError::SourceMismatch {
            field: "deployment_check",
        });
    }
    Ok(())
}

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

/// Build a passive summary of external lifecycle work still pending after a
/// plan/proposal pass.
#[must_use]
pub fn external_lifecycle_pending_report_from_plan(
    report_id: impl Into<String>,
    lifecycle_plan: &ExternalLifecyclePlanV1,
    proposal_report: &ExternalUpgradeProposalReportV1,
) -> ExternalLifecyclePendingReportV1 {
    let report_id = report_id.into();
    let pending_external_actions = proposal_report
        .proposals
        .iter()
        .map(external_lifecycle_pending_action)
        .collect::<Vec<_>>();
    let blocked_subjects = lifecycle_plan
        .blocked_role_upgrades
        .iter()
        .map(|upgrade| upgrade.subject.clone())
        .collect::<Vec<_>>();
    let mut report = ExternalLifecyclePendingReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id,
        report_digest: String::new(),
        lifecycle_plan_id: lifecycle_plan.lifecycle_plan_id.clone(),
        lifecycle_plan_digest: lifecycle_plan.lifecycle_plan_digest.clone(),
        proposal_report_id: proposal_report.report_id.clone(),
        proposal_report_digest: proposal_report.report_digest.clone(),
        deployment_plan_id: lifecycle_plan.deployment_plan_id.clone(),
        deployment_plan_digest: lifecycle_plan.deployment_plan_digest.clone(),
        inventory_id: lifecycle_plan.inventory_id.clone(),
        direct_upgrade_count: lifecycle_plan.directly_executable_role_upgrades.len(),
        pending_external_count: pending_external_actions.len(),
        blocked_count: blocked_subjects.len(),
        pending_external_actions,
        blocked_subjects,
        residual_exposure: lifecycle_plan.residual_exposure.clone(),
        status: lifecycle_plan.status,
    };
    report.report_digest = external_lifecycle_pending_report_digest(&report);
    report
}

/// Validate archived external lifecycle pending report consistency and digest.
pub fn validate_external_lifecycle_pending_report(
    report: &ExternalLifecyclePendingReportV1,
) -> Result<(), ExternalLifecyclePendingReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(ExternalLifecyclePendingReportError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            actual: report.schema_version,
        });
    }
    ensure_external_pending_report_field("report_id", report.report_id.as_str())?;
    ensure_external_pending_report_field("report_digest", report.report_digest.as_str())?;
    ensure_external_pending_report_field("lifecycle_plan_id", report.lifecycle_plan_id.as_str())?;
    ensure_external_pending_report_field(
        "lifecycle_plan_digest",
        report.lifecycle_plan_digest.as_str(),
    )?;
    ensure_external_pending_report_field("proposal_report_id", report.proposal_report_id.as_str())?;
    ensure_external_pending_report_field(
        "proposal_report_digest",
        report.proposal_report_digest.as_str(),
    )?;
    ensure_external_pending_report_field("deployment_plan_id", report.deployment_plan_id.as_str())?;
    ensure_external_pending_report_field(
        "deployment_plan_digest",
        report.deployment_plan_digest.as_str(),
    )?;
    ensure_external_pending_report_field("inventory_id", report.inventory_id.as_str())?;
    if report.pending_external_count != report.pending_external_actions.len()
        || report.blocked_count != report.blocked_subjects.len()
    {
        return Err(ExternalLifecyclePendingReportError::CountMismatch);
    }
    let mut subjects = BTreeSet::new();
    for action in &report.pending_external_actions {
        ensure_external_pending_report_field("pending_action.subject", action.subject.as_str())?;
        ensure_external_pending_report_field(
            "pending_action.proposal_id",
            action.proposal_id.as_str(),
        )?;
        ensure_external_pending_report_field(
            "pending_action.proposal_digest",
            action.proposal_digest.as_str(),
        )?;
        ensure_external_pending_report_field(
            "pending_action.required_external_action",
            action.required_external_action.as_str(),
        )?;
        if !subjects.insert(action.subject.clone()) {
            return Err(ExternalLifecyclePendingReportError::DuplicateSubject {
                subject: action.subject.clone(),
            });
        }
    }
    if report.report_digest != external_lifecycle_pending_report_digest(report) {
        return Err(ExternalLifecyclePendingReportError::DigestMismatch {
            field: "report_digest",
        });
    }
    Ok(())
}

/// Validate that an archived external lifecycle pending report still matches
/// the lifecycle and proposal artifacts it claims to derive from.
pub fn validate_external_lifecycle_pending_report_for_plan(
    report: &ExternalLifecyclePendingReportV1,
    lifecycle_plan: &ExternalLifecyclePlanV1,
    proposal_report: &ExternalUpgradeProposalReportV1,
) -> Result<(), ExternalLifecyclePendingReportError> {
    validate_external_lifecycle_pending_report(report)?;
    if report.lifecycle_plan_id != lifecycle_plan.lifecycle_plan_id {
        return Err(ExternalLifecyclePendingReportError::SourceMismatch {
            field: "lifecycle_plan_id",
        });
    }
    if report.lifecycle_plan_digest != lifecycle_plan.lifecycle_plan_digest {
        return Err(ExternalLifecyclePendingReportError::SourceMismatch {
            field: "lifecycle_plan_digest",
        });
    }
    if report.proposal_report_id != proposal_report.report_id {
        return Err(ExternalLifecyclePendingReportError::SourceMismatch {
            field: "proposal_report_id",
        });
    }
    if report.proposal_report_digest != proposal_report.report_digest {
        return Err(ExternalLifecyclePendingReportError::SourceMismatch {
            field: "proposal_report_digest",
        });
    }
    let expected = external_lifecycle_pending_report_from_plan(
        report.report_id.clone(),
        lifecycle_plan,
        proposal_report,
    );
    if report != &expected {
        return Err(ExternalLifecyclePendingReportError::SourceMismatch {
            field: "lifecycle_plan",
        });
    }
    Ok(())
}

fn external_lifecycle_pending_action(
    proposal: &ExternalUpgradeProposalV1,
) -> ExternalLifecyclePendingActionV1 {
    ExternalLifecyclePendingActionV1 {
        subject: proposal.subject.clone(),
        proposal_id: proposal.proposal_id.clone(),
        proposal_digest: proposal.proposal_digest.clone(),
        canister_id: proposal.canister_id.clone(),
        role: proposal.role.clone(),
        control_class: proposal.control_class,
        lifecycle_mode: proposal.lifecycle_mode,
        required_external_action: proposal.required_external_action.clone(),
        consent_requirements: proposal.consent_requirements.clone(),
        verification_requirements: proposal.verification_requirements.clone(),
    }
}

fn lifecycle_roles(lifecycle_plan: &ExternalLifecyclePlanV1) -> Vec<String> {
    lifecycle_plan
        .lifecycle_authority_rows
        .iter()
        .filter_map(|authority| authority.role.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn lifecycle_canisters(lifecycle_plan: &ExternalLifecyclePlanV1) -> Vec<String> {
    lifecycle_plan
        .lifecycle_authority_rows
        .iter()
        .filter_map(|authority| authority.canister_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn role_names(upgrades: &[ExternalLifecycleRoleUpgradeV1]) -> Vec<String> {
    upgrades
        .iter()
        .filter_map(|upgrade| upgrade.role.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn critical_fix_next_steps(
    pending_external_count: usize,
    blocked_count: usize,
    protected_call_implications: &[String],
) -> Vec<String> {
    let mut steps = Vec::new();
    if pending_external_count > 0 {
        steps.push(
            "request external consent or completion for externally controlled roles".to_string(),
        );
    }
    if blocked_count > 0 {
        steps.push(
            "resolve blocked lifecycle rows before reporting the deployment fully patched"
                .to_string(),
        );
    }
    if !protected_call_implications.is_empty() {
        steps.push(
            "review protected-call readiness and role epoch implications before closure"
                .to_string(),
        );
    }
    if steps.is_empty() {
        steps.push("no external lifecycle work remains for this critical fix".to_string());
    }
    steps
}

/// Build a passive critical-fix residual exposure report from lifecycle
/// evidence.
#[must_use]
pub fn critical_external_fix_report_from_pending(
    report_id: impl Into<String>,
    fix_id: impl Into<String>,
    severity: impl Into<String>,
    lifecycle_plan: &ExternalLifecyclePlanV1,
    pending_report: &ExternalLifecyclePendingReportV1,
) -> CriticalExternalFixReportV1 {
    let report_id = report_id.into();
    let fix_id = fix_id.into();
    let severity = severity.into();
    let affected_roles = lifecycle_roles(lifecycle_plan);
    let affected_canisters = lifecycle_canisters(lifecycle_plan);
    let directly_patchable_roles = role_names(&lifecycle_plan.directly_executable_role_upgrades);
    let externally_blocked_roles = pending_report
        .pending_external_actions
        .iter()
        .filter_map(|action| action.role.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let dependency_blocked_roles = role_names(&lifecycle_plan.blocked_role_upgrades);
    let required_external_actions = pending_report
        .pending_external_actions
        .iter()
        .map(|action| format!("{}: {}", action.subject, action.required_external_action))
        .collect::<Vec<_>>();
    let operator_next_steps = critical_fix_next_steps(
        pending_report.pending_external_count,
        pending_report.blocked_count,
        lifecycle_plan.protected_call_implications.as_slice(),
    );
    let mut report = CriticalExternalFixReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id,
        report_digest: String::new(),
        fix_id,
        severity,
        lifecycle_plan_id: lifecycle_plan.lifecycle_plan_id.clone(),
        lifecycle_plan_digest: lifecycle_plan.lifecycle_plan_digest.clone(),
        pending_report_id: pending_report.report_id.clone(),
        pending_report_digest: pending_report.report_digest.clone(),
        deployment_plan_id: lifecycle_plan.deployment_plan_id.clone(),
        deployment_plan_digest: lifecycle_plan.deployment_plan_digest.clone(),
        inventory_id: lifecycle_plan.inventory_id.clone(),
        affected_roles,
        affected_canisters,
        directly_patchable_roles,
        externally_blocked_roles,
        dependency_blocked_roles,
        required_external_actions,
        protected_call_implications: lifecycle_plan.protected_call_implications.clone(),
        residual_exposure: pending_report.residual_exposure.clone(),
        operator_next_steps,
    };
    report.report_digest = critical_external_fix_report_digest(&report);
    report
}

/// Validate archived critical external fix report consistency and digest.
pub fn validate_critical_external_fix_report(
    report: &CriticalExternalFixReportV1,
) -> Result<(), CriticalExternalFixReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(CriticalExternalFixReportError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            actual: report.schema_version,
        });
    }
    ensure_critical_fix_report_field("report_id", report.report_id.as_str())?;
    ensure_critical_fix_report_field("report_digest", report.report_digest.as_str())?;
    ensure_critical_fix_report_field("fix_id", report.fix_id.as_str())?;
    ensure_critical_fix_report_field("severity", report.severity.as_str())?;
    ensure_critical_fix_report_field("lifecycle_plan_id", report.lifecycle_plan_id.as_str())?;
    ensure_critical_fix_report_field(
        "lifecycle_plan_digest",
        report.lifecycle_plan_digest.as_str(),
    )?;
    ensure_critical_fix_report_field("pending_report_id", report.pending_report_id.as_str())?;
    ensure_critical_fix_report_field(
        "pending_report_digest",
        report.pending_report_digest.as_str(),
    )?;
    ensure_critical_fix_report_field("deployment_plan_id", report.deployment_plan_id.as_str())?;
    ensure_critical_fix_report_field(
        "deployment_plan_digest",
        report.deployment_plan_digest.as_str(),
    )?;
    ensure_critical_fix_report_field("inventory_id", report.inventory_id.as_str())?;
    if report.report_digest != critical_external_fix_report_digest(report) {
        return Err(CriticalExternalFixReportError::DigestMismatch {
            field: "report_digest",
        });
    }
    Ok(())
}

/// Validate that an archived critical external fix report still matches the
/// lifecycle artifacts it claims to summarize.
pub fn validate_critical_external_fix_report_for_pending(
    report: &CriticalExternalFixReportV1,
    lifecycle_plan: &ExternalLifecyclePlanV1,
    pending_report: &ExternalLifecyclePendingReportV1,
) -> Result<(), CriticalExternalFixReportError> {
    validate_critical_external_fix_report(report)?;
    if report.lifecycle_plan_id != lifecycle_plan.lifecycle_plan_id {
        return Err(CriticalExternalFixReportError::SourceMismatch {
            field: "lifecycle_plan_id",
        });
    }
    if report.lifecycle_plan_digest != lifecycle_plan.lifecycle_plan_digest {
        return Err(CriticalExternalFixReportError::SourceMismatch {
            field: "lifecycle_plan_digest",
        });
    }
    if report.pending_report_id != pending_report.report_id {
        return Err(CriticalExternalFixReportError::SourceMismatch {
            field: "pending_report_id",
        });
    }
    if report.pending_report_digest != pending_report.report_digest {
        return Err(CriticalExternalFixReportError::SourceMismatch {
            field: "pending_report_digest",
        });
    }
    let expected = critical_external_fix_report_from_pending(
        report.report_id.clone(),
        report.fix_id.clone(),
        report.severity.clone(),
        lifecycle_plan,
        pending_report,
    );
    if report != &expected {
        return Err(CriticalExternalFixReportError::SourceMismatch {
            field: "lifecycle_plan",
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

fn lifecycle_authority_for_expected_canister(
    plan: &DeploymentPlanV1,
    expected: &ExpectedCanisterV1,
    observed: Option<&ObservedCanisterV1>,
) -> LifecycleAuthorityV1 {
    let canister_id = expected
        .canister_id
        .clone()
        .or_else(|| observed.map(|observed| observed.canister_id.clone()));
    let role = Some(expected.role.clone());
    let control_class = observed.map_or(expected.control_class, |observed| observed.control_class);
    let observed_controllers =
        observed.map_or_else(Vec::new, |observed| observed.controllers.clone());
    lifecycle_authority(
        lifecycle_subject_for_parts(canister_id.as_deref(), role.as_deref()),
        canister_id,
        role,
        control_class,
        observed_controllers,
        &plan.authority_profile.expected_controllers,
        plan.expected_verifier_readiness.required,
    )
}

fn lifecycle_authority_for_expected_pool(
    expected: &ExpectedPoolCanisterV1,
    observed: Option<&ObservedPoolCanisterV1>,
) -> LifecycleAuthorityV1 {
    let canister_id = expected
        .canister_id
        .clone()
        .or_else(|| observed.map(|observed| observed.canister_id.clone()));
    let role = expected
        .role
        .clone()
        .or_else(|| observed.and_then(|observed| observed.role.clone()));
    let control_class = observed.map_or(CanisterControlClassV1::CanicManagedPool, |observed| {
        observed.control_class
    });
    lifecycle_authority(
        lifecycle_subject_for_parts(canister_id.as_deref(), role.as_deref()),
        canister_id,
        role,
        control_class,
        Vec::new(),
        &[],
        false,
    )
}

fn lifecycle_authority_for_unplanned_canister(
    observed: &ObservedCanisterV1,
) -> LifecycleAuthorityV1 {
    lifecycle_authority(
        lifecycle_subject(observed.canister_id.as_str(), observed.role.as_deref()),
        Some(observed.canister_id.clone()),
        observed.role.clone(),
        observed.control_class,
        observed.controllers.clone(),
        &[],
        false,
    )
}

fn lifecycle_authority_for_unplanned_pool(
    observed: &ObservedPoolCanisterV1,
) -> LifecycleAuthorityV1 {
    lifecycle_authority(
        lifecycle_subject(observed.canister_id.as_str(), observed.role.as_deref()),
        Some(observed.canister_id.clone()),
        observed.role.clone(),
        observed.control_class,
        Vec::new(),
        &[],
        false,
    )
}

fn lifecycle_authority(
    subject: String,
    canister_id: Option<String>,
    role: Option<String>,
    control_class: CanisterControlClassV1,
    observed_controllers: Vec<String>,
    expected_controllers: &[String],
    verifier_required: bool,
) -> LifecycleAuthorityV1 {
    let required_controllers = required_lifecycle_controllers(control_class, expected_controllers);
    let external_controllers =
        external_lifecycle_controllers(control_class, &observed_controllers, &required_controllers);
    let consent_requirements = lifecycle_consent_requirements(control_class, &external_controllers);
    let allowed_upgrade_modes = lifecycle_upgrade_modes(control_class);
    let verification_requirements = lifecycle_verification_requirements(verifier_required);
    let external_action_required = lifecycle_external_action_required(control_class);
    let blocked = control_class == CanisterControlClassV1::UnknownUnsafe;
    let lifecycle_mode = lifecycle_mode(control_class);
    let blockers = lifecycle_blockers(control_class);
    let warnings = lifecycle_warnings(control_class);
    let reason = lifecycle_reason(control_class);
    LifecycleAuthorityV1 {
        subject,
        canister_id,
        role,
        control_class,
        lifecycle_mode,
        observed_controllers,
        expected_deployment_controllers: sorted_unique(expected_controllers.to_vec()),
        external_controllers,
        required_controllers,
        consent_requirements,
        allowed_upgrade_modes,
        verification_requirements,
        external_action_required,
        blocked,
        blockers,
        warnings,
        reason,
    }
}

fn required_lifecycle_controllers(
    control_class: CanisterControlClassV1,
    expected_controllers: &[String],
) -> Vec<String> {
    match control_class {
        CanisterControlClassV1::DeploymentControlled
        | CanisterControlClassV1::JointlyControlled => sorted_unique(expected_controllers.to_vec()),
        CanisterControlClassV1::CanicManagedPool
        | CanisterControlClassV1::ExternallyImported
        | CanisterControlClassV1::UserControlled
        | CanisterControlClassV1::UnknownUnsafe => Vec::new(),
    }
}

fn external_lifecycle_controllers(
    control_class: CanisterControlClassV1,
    observed_controllers: &[String],
    required_controllers: &[String],
) -> Vec<String> {
    match control_class {
        CanisterControlClassV1::DeploymentControlled | CanisterControlClassV1::UnknownUnsafe => {
            Vec::new()
        }
        CanisterControlClassV1::JointlyControlled => {
            let required = required_controllers.iter().collect::<BTreeSet<_>>();
            sorted_unique(
                observed_controllers
                    .iter()
                    .filter(|controller| !required.contains(controller))
                    .cloned()
                    .collect(),
            )
        }
        CanisterControlClassV1::CanicManagedPool
        | CanisterControlClassV1::ExternallyImported
        | CanisterControlClassV1::UserControlled => sorted_unique(observed_controllers.to_vec()),
    }
}

fn lifecycle_consent_requirements(
    control_class: CanisterControlClassV1,
    external_controllers: &[String],
) -> Vec<ConsentRequirementV1> {
    if !lifecycle_external_action_required(control_class) {
        return Vec::new();
    }
    vec![ConsentRequirementV1 {
        consent_subject_kind: consent_subject_kind(control_class),
        required_principals: sorted_unique(external_controllers.to_vec()),
        required_controller_set_digest: Some(stable_json_sha256_hex(&external_controllers)),
        consent_channel_kind: consent_channel_kind(control_class),
        required_action: required_consent_action(control_class),
    }]
}

const fn consent_subject_kind(control_class: CanisterControlClassV1) -> ConsentSubjectKindV1 {
    match control_class {
        CanisterControlClassV1::CanicManagedPool => ConsentSubjectKindV1::ProjectHub,
        CanisterControlClassV1::ExternallyImported | CanisterControlClassV1::JointlyControlled => {
            ConsentSubjectKindV1::CustomerController
        }
        CanisterControlClassV1::UserControlled => ConsentSubjectKindV1::UserPrincipal,
        CanisterControlClassV1::DeploymentControlled | CanisterControlClassV1::UnknownUnsafe => {
            ConsentSubjectKindV1::UnknownExternalController
        }
    }
}

const fn consent_channel_kind(control_class: CanisterControlClassV1) -> ConsentChannelKindV1 {
    match control_class {
        CanisterControlClassV1::CanicManagedPool => ConsentChannelKindV1::DelegatedInstall,
        CanisterControlClassV1::ExternallyImported
        | CanisterControlClassV1::JointlyControlled
        | CanisterControlClassV1::UserControlled => ConsentChannelKindV1::GeneratedCommand,
        CanisterControlClassV1::DeploymentControlled | CanisterControlClassV1::UnknownUnsafe => {
            ConsentChannelKindV1::OutOfBand
        }
    }
}

const fn required_consent_action(
    control_class: CanisterControlClassV1,
) -> ExternalUpgradeAuthorizationModeV1 {
    match control_class {
        CanisterControlClassV1::JointlyControlled => {
            ExternalUpgradeAuthorizationModeV1::ConsentForDirectInstall
        }
        CanisterControlClassV1::CanicManagedPool => {
            ExternalUpgradeAuthorizationModeV1::DelegatedInstallAuthority
        }
        CanisterControlClassV1::ExternallyImported | CanisterControlClassV1::UserControlled => {
            ExternalUpgradeAuthorizationModeV1::ExternalControllerExecution
        }
        CanisterControlClassV1::DeploymentControlled | CanisterControlClassV1::UnknownUnsafe => {
            ExternalUpgradeAuthorizationModeV1::ObserveAndVerifyOnly
        }
    }
}

const fn lifecycle_mode(control_class: CanisterControlClassV1) -> LifecycleModeV1 {
    match control_class {
        CanisterControlClassV1::DeploymentControlled => LifecycleModeV1::DirectDeploymentAuthority,
        CanisterControlClassV1::CanicManagedPool => LifecycleModeV1::DelegatedInstallRequired,
        CanisterControlClassV1::ExternallyImported | CanisterControlClassV1::UserControlled => {
            LifecycleModeV1::ExternalCompletionOnly
        }
        CanisterControlClassV1::JointlyControlled => LifecycleModeV1::ProposalRequired,
        CanisterControlClassV1::UnknownUnsafe => LifecycleModeV1::UnknownUnsafeBlocked,
    }
}

fn lifecycle_blockers(control_class: CanisterControlClassV1) -> Vec<String> {
    if control_class == CanisterControlClassV1::UnknownUnsafe {
        vec!["unknown unsafe controller state blocks lifecycle action".to_string()]
    } else {
        Vec::new()
    }
}

fn lifecycle_warnings(control_class: CanisterControlClassV1) -> Vec<String> {
    match control_class {
        CanisterControlClassV1::CanicManagedPool => {
            vec!["pool-aware lifecycle policy is required before mutation".to_string()]
        }
        CanisterControlClassV1::ExternallyImported => {
            vec!["external controller action or verification is required".to_string()]
        }
        CanisterControlClassV1::JointlyControlled => {
            vec!["joint controller consent or delegation is required".to_string()]
        }
        CanisterControlClassV1::UserControlled => {
            vec!["user or delegated lifecycle action is required".to_string()]
        }
        CanisterControlClassV1::DeploymentControlled | CanisterControlClassV1::UnknownUnsafe => {
            Vec::new()
        }
    }
}

fn lifecycle_upgrade_modes(control_class: CanisterControlClassV1) -> Vec<LifecycleUpgradeModeV1> {
    match control_class {
        CanisterControlClassV1::DeploymentControlled => vec![
            LifecycleUpgradeModeV1::DirectByDeploymentAuthority,
            LifecycleUpgradeModeV1::VerifyExternalCompletion,
        ],
        CanisterControlClassV1::CanicManagedPool
        | CanisterControlClassV1::ExternallyImported
        | CanisterControlClassV1::JointlyControlled
        | CanisterControlClassV1::UserControlled => vec![
            LifecycleUpgradeModeV1::ExternalProposal,
            LifecycleUpgradeModeV1::ExternalExecution,
            LifecycleUpgradeModeV1::VerifyExternalCompletion,
            LifecycleUpgradeModeV1::ObserveOnly,
        ],
        CanisterControlClassV1::UnknownUnsafe => vec![LifecycleUpgradeModeV1::Blocked],
    }
}

fn lifecycle_verification_requirements(
    verifier_required: bool,
) -> Vec<LifecycleVerificationRequirementV1> {
    let mut requirements = vec![
        LifecycleVerificationRequirementV1::LiveInventory,
        LifecycleVerificationRequirementV1::ControllerObservation,
        LifecycleVerificationRequirementV1::ModuleHash,
        LifecycleVerificationRequirementV1::CanonicalEmbeddedConfig,
    ];
    if verifier_required {
        requirements.push(LifecycleVerificationRequirementV1::ProtectedCallReadiness);
    }
    requirements
}

const fn lifecycle_external_action_required(control_class: CanisterControlClassV1) -> bool {
    matches!(
        control_class,
        CanisterControlClassV1::CanicManagedPool
            | CanisterControlClassV1::ExternallyImported
            | CanisterControlClassV1::JointlyControlled
            | CanisterControlClassV1::UserControlled
    )
}

fn lifecycle_reason(control_class: CanisterControlClassV1) -> String {
    match control_class {
        CanisterControlClassV1::DeploymentControlled => {
            "deployment authority can execute lifecycle directly".to_string()
        }
        CanisterControlClassV1::CanicManagedPool => {
            "Canic-managed pool lifecycle requires pool-aware external action".to_string()
        }
        CanisterControlClassV1::ExternallyImported => {
            "externally imported canister requires external controller action".to_string()
        }
        CanisterControlClassV1::JointlyControlled => {
            "jointly controlled canister requires non-deployment-controller consent".to_string()
        }
        CanisterControlClassV1::UserControlled => {
            "user-controlled canister requires user or delegated lifecycle action".to_string()
        }
        CanisterControlClassV1::UnknownUnsafe => {
            "unknown or unsafe controller state blocks lifecycle action".to_string()
        }
    }
}

fn observed_canister_for_expected<'a>(
    inventory: &'a DeploymentInventoryV1,
    expected: &ExpectedCanisterV1,
) -> Option<&'a ObservedCanisterV1> {
    if let Some(canister_id) = &expected.canister_id
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
        .find(|observed| observed.role.as_deref() == Some(expected.role.as_str()))
}

fn observed_pool_for_expected<'a>(
    inventory: &'a DeploymentInventoryV1,
    expected: &ExpectedPoolCanisterV1,
) -> Option<&'a ObservedPoolCanisterV1> {
    if let Some(canister_id) = &expected.canister_id
        && let Some(observed) = inventory
            .observed_pool
            .iter()
            .find(|observed| &observed.canister_id == canister_id)
    {
        return Some(observed);
    }
    inventory.observed_pool.iter().find(|observed| {
        observed.pool == expected.pool && observed.role.as_deref() == expected.role.as_deref()
    })
}

fn lifecycle_subject(canister_id: &str, role: Option<&str>) -> String {
    lifecycle_subject_for_parts(Some(canister_id), role)
}

fn lifecycle_subject_for_parts(canister_id: Option<&str>, role: Option<&str>) -> String {
    match (role, canister_id) {
        (Some(role), Some(canister_id)) => format!("{role}:{canister_id}"),
        (Some(role), None) => format!("{role}:unassigned"),
        (None, Some(canister_id)) => canister_id.to_string(),
        (None, None) => "unknown".to_string(),
    }
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

fn target_artifact_for_authority<'a>(
    plan: &'a DeploymentPlanV1,
    authority: &LifecycleAuthorityV1,
) -> Option<&'a RoleArtifactV1> {
    let role = authority.role.as_ref()?;
    plan.role_artifacts
        .iter()
        .find(|artifact| &artifact.role == role)
}

fn external_lifecycle_role_upgrade(
    authority: &LifecycleAuthorityV1,
) -> ExternalLifecycleRoleUpgradeV1 {
    ExternalLifecycleRoleUpgradeV1 {
        subject: authority.subject.clone(),
        canister_id: authority.canister_id.clone(),
        role: authority.role.clone(),
        control_class: authority.control_class,
        lifecycle_mode: authority.lifecycle_mode,
        required_external_action: authority
            .external_action_required
            .then(|| required_external_action(authority.lifecycle_mode).to_string()),
        blockers: authority.blockers.clone(),
        warnings: authority.warnings.clone(),
    }
}

fn protected_call_implications_for_check(check: &DeploymentCheckV1) -> Vec<String> {
    if check.plan.expected_verifier_readiness.required {
        vec!["protected-call verifier readiness must be checked before completion".to_string()]
    } else {
        Vec::new()
    }
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

fn external_lifecycle_plan_digest(plan: &ExternalLifecyclePlanV1) -> String {
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

fn lifecycle_authority_report_digest(report: &LifecycleAuthorityReportV1) -> String {
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

const fn expected_lifecycle_plan_status(
    plan: &ExternalLifecyclePlanV1,
) -> ExternalLifecyclePlanStatusV1 {
    if !plan.blocked_role_upgrades.is_empty() {
        ExternalLifecyclePlanStatusV1::Blocked
    } else if !plan.proposed_external_role_upgrades.is_empty() {
        ExternalLifecyclePlanStatusV1::PendingExternalAction
    } else {
        ExternalLifecyclePlanStatusV1::Ready
    }
}

fn ensure_unique_lifecycle_subjects(
    rows: &[LifecycleAuthorityV1],
) -> Result<(), ExternalLifecyclePlanError> {
    let mut subjects = BTreeSet::new();
    for row in rows {
        if !subjects.insert(row.subject.clone()) {
            return Err(ExternalLifecyclePlanError::DuplicateSubject {
                subject: row.subject.clone(),
            });
        }
    }
    Ok(())
}

fn ensure_unique_authority_subjects(
    rows: &[LifecycleAuthorityV1],
) -> Result<(), LifecycleAuthorityReportError> {
    let mut subjects = BTreeSet::new();
    for row in rows {
        if !subjects.insert(row.subject.clone()) {
            return Err(LifecycleAuthorityReportError::DuplicateSubject {
                subject: row.subject.clone(),
            });
        }
    }
    Ok(())
}

fn ensure_unique_role_upgrade_subjects(
    rows: &[ExternalLifecycleRoleUpgradeV1],
) -> Result<(), ExternalLifecyclePlanError> {
    let mut subjects = BTreeSet::new();
    for row in rows {
        if !subjects.insert(row.subject.clone()) {
            return Err(ExternalLifecyclePlanError::DuplicateSubject {
                subject: row.subject.clone(),
            });
        }
    }
    Ok(())
}

fn external_upgrade_proposal_digest(proposal: &ExternalUpgradeProposalV1) -> String {
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

fn external_upgrade_proposal_report_digest(report: &ExternalUpgradeProposalReportV1) -> String {
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

fn external_lifecycle_pending_report_digest(report: &ExternalLifecyclePendingReportV1) -> String {
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

fn critical_external_fix_report_digest(report: &CriticalExternalFixReportV1) -> String {
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

fn external_upgrade_receipt_digest(receipt: &ExternalUpgradeReceiptV1) -> String {
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

fn external_upgrade_consent_evidence_digest(evidence: &ExternalUpgradeConsentEvidenceV1) -> String {
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

fn external_upgrade_verification_report_digest(
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

fn observed_before_digest(
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

fn ensure_external_lifecycle_plan_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalLifecyclePlanError> {
    if value.trim().is_empty() {
        return Err(ExternalLifecyclePlanError::MissingRequiredField { field });
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

fn ensure_external_pending_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalLifecyclePendingReportError> {
    if value.trim().is_empty() {
        return Err(ExternalLifecyclePendingReportError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_critical_fix_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), CriticalExternalFixReportError> {
    if value.trim().is_empty() {
        return Err(CriticalExternalFixReportError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_lifecycle_authority_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), LifecycleAuthorityReportError> {
    if value.trim().is_empty() {
        return Err(LifecycleAuthorityReportError::MissingRequiredField { field });
    }
    Ok(())
}

fn sorted_unique(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
