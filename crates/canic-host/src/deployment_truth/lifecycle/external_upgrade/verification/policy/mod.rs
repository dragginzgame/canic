use super::super::super::super::DEPLOYMENT_TRUTH_SCHEMA_VERSION;
use super::super::super::digest::external_upgrade_verification_policy_digest;
use super::super::super::error::ExternalUpgradeVerificationPolicyError;
use super::super::validation::ensure_external_verification_policy_field;
use super::shared::control_class_value;
use crate::deployment_truth::{
    ExternalUpgradeProposalV1, ExternalUpgradeVerificationPolicyRequirementV1,
    ExternalUpgradeVerificationPolicyV1, ExternalUpgradeVerificationRequirementStatusV1,
    LifecycleVerificationRequirementV1,
};

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
