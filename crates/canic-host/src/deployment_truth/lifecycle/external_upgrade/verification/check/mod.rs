use super::super::super::super::*;
use super::super::super::digest::*;
use super::super::super::error::ExternalUpgradeVerificationCheckError;
use super::super::validation::{
    ensure_external_verification_check_field, ensure_external_verification_check_option_field,
};
use super::policy::validate_external_upgrade_verification_policy;
use super::shared::control_class_value;
use std::collections::BTreeSet;

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
