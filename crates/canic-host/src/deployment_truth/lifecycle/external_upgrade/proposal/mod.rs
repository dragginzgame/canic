use super::super::super::*;
use super::super::digest::*;
use super::super::error::ExternalUpgradeProposalReportError;
use super::validation::ensure_external_proposal_report_field;
use std::collections::BTreeSet;

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

pub(super) fn validate_external_upgrade_proposal(
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
