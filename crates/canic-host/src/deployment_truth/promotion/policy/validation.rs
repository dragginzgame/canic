use crate::deployment_truth::{
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, PromotionArtifactLevelV1, PromotionPolicyCheckV1,
    PromotionPolicyClaimV1, PromotionPolicyRequirementV1, PromotionReadinessStatusV1,
    RolePromotionPolicyDecisionV1, RolePromotionPolicyV1, SafetyFindingV1, SafetySeverityV1,
};
use std::collections::BTreeSet;

use super::super::digest::promotion_policy_check_digest;
use super::super::ensure::{ensure_policy_field, ensure_policy_sha256};
use super::super::error::PromotionPolicyCheckError;

pub fn validate_promotion_policy_check(
    check: &PromotionPolicyCheckV1,
) -> Result<(), PromotionPolicyCheckError> {
    if check.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(PromotionPolicyCheckError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found: check.schema_version,
        });
    }
    ensure_policy_field("check_id", &check.check_id)?;
    ensure_policy_sha256(
        "promotion_policy_check_digest",
        &check.promotion_policy_check_digest,
    )?;
    ensure_policy_status_matches_blockers(check)?;
    ensure_unique_policy_decision_roles(&check.roles)?;
    for role in &check.roles {
        validate_role_promotion_policy_decision(role)?;
    }
    validate_policy_blockers(&check.blockers)?;
    if check.promotion_policy_check_digest != promotion_policy_check_digest(check) {
        return Err(PromotionPolicyCheckError::LinkageMismatch {
            field: "promotion_policy_check_digest",
        });
    }
    Ok(())
}

pub fn validate_role_promotion_policy(
    policy: &RolePromotionPolicyV1,
) -> Result<(), PromotionPolicyCheckError> {
    ensure_policy_field("role", &policy.role)?;
    if policy.allowed_promotion_levels.is_empty() {
        return Err(PromotionPolicyCheckError::EmptyAllowedLevels {
            role: policy.role.clone(),
        });
    }
    let mut seen = BTreeSet::new();
    for level in &policy.allowed_promotion_levels {
        if !seen.insert(*level) {
            return Err(PromotionPolicyCheckError::DuplicateAllowedLevel {
                role: policy.role.clone(),
                level: *level,
            });
        }
    }
    let mut seen_requirements = BTreeSet::new();
    for requirement in &policy.requirements {
        if !seen_requirements.insert(*requirement) {
            return Err(PromotionPolicyCheckError::DecisionMismatch {
                role: policy.role.clone(),
                field: "requirements",
            });
        }
    }
    if policy
        .requirements
        .contains(&PromotionPolicyRequirementV1::SealedBytes)
        && policy
            .allowed_promotion_levels
            .iter()
            .any(|level| *level != PromotionArtifactLevelV1::SealedWasm)
    {
        return Err(PromotionPolicyCheckError::DecisionMismatch {
            role: policy.role.clone(),
            field: "sealed_bytes",
        });
    }
    Ok(())
}

fn validate_role_promotion_policy_decision(
    decision: &RolePromotionPolicyDecisionV1,
) -> Result<(), PromotionPolicyCheckError> {
    ensure_policy_field("role", &decision.role)?;
    if decision.allowed_promotion_levels.is_empty() {
        return Err(PromotionPolicyCheckError::EmptyAllowedLevels {
            role: decision.role.clone(),
        });
    }
    let mut seen = BTreeSet::new();
    for level in &decision.allowed_promotion_levels {
        if !seen.insert(*level) {
            return Err(PromotionPolicyCheckError::DuplicateAllowedLevel {
                role: decision.role.clone(),
                level: *level,
            });
        }
    }
    let mut seen_requirements = BTreeSet::new();
    for requirement in &decision.requirements {
        if !seen_requirements.insert(*requirement) {
            return Err(PromotionPolicyCheckError::DecisionMismatch {
                role: decision.role.clone(),
                field: "requirements",
            });
        }
    }
    let mut seen_claims = BTreeSet::new();
    for claim in &decision.claims {
        if !seen_claims.insert(*claim) {
            return Err(PromotionPolicyCheckError::DecisionMismatch {
                role: decision.role.clone(),
                field: "claims",
            });
        }
    }
    ensure_policy_decision(
        decision,
        "level_allowed",
        decision
            .allowed_promotion_levels
            .contains(&decision.requested_promotion_level)
            == decision.level_allowed,
    )?;
    ensure_policy_decision(
        decision,
        "policy_satisfied",
        promotion_policy_decision_satisfied(decision) == decision.policy_satisfied,
    )?;
    Ok(())
}

fn promotion_policy_decision_satisfied(decision: &RolePromotionPolicyDecisionV1) -> bool {
    decision.level_allowed
        && (!contains_policy_requirement(
            &decision.requirements,
            PromotionPolicyRequirementV1::SealedBytes,
        ) || matches!(
            decision.requested_promotion_level,
            PromotionArtifactLevelV1::SealedWasm
        ))
        && (!contains_policy_requirement(
            &decision.requirements,
            PromotionPolicyRequirementV1::ByteIdenticalWasm,
        ) || contains_policy_claim(&decision.claims, PromotionPolicyClaimV1::ByteIdenticalWasm))
        && (!contains_policy_requirement(
            &decision.requirements,
            PromotionPolicyRequirementV1::TargetConfigDigest,
        ) || contains_policy_claim(
            &decision.claims,
            PromotionPolicyClaimV1::TargetConfigDigest,
        ))
}

fn contains_policy_requirement(
    requirements: &[PromotionPolicyRequirementV1],
    needle: PromotionPolicyRequirementV1,
) -> bool {
    let mut index = 0;
    while index < requirements.len() {
        if requirements[index] as u8 == needle as u8 {
            return true;
        }
        index += 1;
    }
    false
}

fn contains_policy_claim(
    claims: &[PromotionPolicyClaimV1],
    needle: PromotionPolicyClaimV1,
) -> bool {
    let mut index = 0;
    while index < claims.len() {
        if claims[index] as u8 == needle as u8 {
            return true;
        }
        index += 1;
    }
    false
}

fn ensure_policy_decision(
    decision: &RolePromotionPolicyDecisionV1,
    field: &'static str,
    valid: bool,
) -> Result<(), PromotionPolicyCheckError> {
    if valid {
        Ok(())
    } else {
        Err(PromotionPolicyCheckError::DecisionMismatch {
            role: decision.role.clone(),
            field,
        })
    }
}

const fn ensure_policy_status_matches_blockers(
    check: &PromotionPolicyCheckV1,
) -> Result<(), PromotionPolicyCheckError> {
    match (check.status, check.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => {
            Err(PromotionPolicyCheckError::StatusBlockerMismatch {
                status: check.status,
                blocker_count: check.blockers.len(),
            })
        }
        _ => Ok(()),
    }
}

fn ensure_unique_policy_decision_roles(
    roles: &[RolePromotionPolicyDecisionV1],
) -> Result<(), PromotionPolicyCheckError> {
    let mut seen = BTreeSet::new();
    for role in roles {
        if !seen.insert(role.role.as_str()) {
            return Err(PromotionPolicyCheckError::DuplicateRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

fn validate_policy_blockers(blockers: &[SafetyFindingV1]) -> Result<(), PromotionPolicyCheckError> {
    for blocker in blockers {
        ensure_policy_field("blocker.code", &blocker.code)?;
        ensure_policy_field("blocker.message", &blocker.message)?;
        if blocker.severity != SafetySeverityV1::HardFailure {
            return Err(PromotionPolicyCheckError::BlockerSeverityMismatch {
                severity: blocker.severity,
            });
        }
    }
    Ok(())
}
