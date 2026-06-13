use super::super::{
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, PromotionArtifactLevelV1, PromotionPolicyCheckV1,
    PromotionPolicyClaimV1, PromotionPolicyRequirementV1, PromotionReadinessStatusV1,
    RolePromotionInputV1, RolePromotionPolicyDecisionV1, RolePromotionPolicyV1, SafetyFindingV1,
    SafetySeverityV1,
};
use super::digest::promotion_policy_check_digest;
use super::ensure::{ensure_policy_field, ensure_policy_sha256};
use super::error::PromotionPolicyCheckError;
use super::request::PromotionPolicyCheckRequest;
use std::collections::BTreeSet;

pub fn check_promotion_policy(
    request: PromotionPolicyCheckRequest,
) -> Result<PromotionPolicyCheckV1, PromotionPolicyCheckError> {
    ensure_policy_field("check_id", &request.check_id)?;
    let check =
        promotion_policy_check_from_inputs(&request.check_id, &request.inputs, &request.policies);
    validate_promotion_policy_check(&check)?;
    Ok(check)
}

#[must_use]
pub fn promotion_policy_check_from_inputs(
    check_id: impl Into<String>,
    inputs: &[RolePromotionInputV1],
    policies: &[RolePromotionPolicyV1],
) -> PromotionPolicyCheckV1 {
    let mut roles = Vec::with_capacity(inputs.len());
    let mut blockers = Vec::new();
    let mut seen_policy_roles = BTreeSet::new();
    for policy in policies {
        if !seen_policy_roles.insert(policy.role.as_str()) {
            blockers.push(super::promotion_finding(
                "promotion_policy_duplicate",
                format!("multiple promotion policies exist for role {}", policy.role),
                SafetySeverityV1::HardFailure,
                &policy.role,
            ));
        }
        if let Err(err) = validate_role_promotion_policy(policy) {
            blockers.push(super::promotion_finding(
                "promotion_policy_invalid",
                err.to_string(),
                SafetySeverityV1::HardFailure,
                &policy.role,
            ));
        }
    }
    for input in inputs {
        let matching_policies = policies
            .iter()
            .filter(|policy| policy.role == input.role)
            .collect::<Vec<_>>();
        match matching_policies.as_slice() {
            [] => {
                blockers.push(super::promotion_finding(
                    "promotion_policy_missing",
                    format!("no promotion policy exists for role {}", input.role),
                    SafetySeverityV1::HardFailure,
                    &input.role,
                ));
            }
            [policy] => {
                let decision = role_promotion_policy_decision(input, policy);
                collect_policy_findings(&decision, &mut blockers);
                roles.push(decision);
            }
            _ => blockers.push(super::promotion_finding(
                "promotion_policy_duplicate",
                format!("multiple promotion policies exist for role {}", input.role),
                SafetySeverityV1::HardFailure,
                &input.role,
            )),
        }
    }

    let mut check = PromotionPolicyCheckV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        check_id: check_id.into(),
        promotion_policy_check_digest: String::new(),
        status: if blockers.is_empty() {
            PromotionReadinessStatusV1::Ready
        } else {
            PromotionReadinessStatusV1::Blocked
        },
        roles,
        blockers,
    };
    check.promotion_policy_check_digest = promotion_policy_check_digest(&check);
    check
}

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

fn role_promotion_policy_decision(
    input: &RolePromotionInputV1,
    policy: &RolePromotionPolicyV1,
) -> RolePromotionPolicyDecisionV1 {
    let level_allowed = policy
        .allowed_promotion_levels
        .contains(&input.promotion_level);
    let claims = promotion_policy_claims_for_input(input);
    let policy_satisfied = level_allowed
        && (!policy
            .requirements
            .contains(&PromotionPolicyRequirementV1::SealedBytes)
            || input.promotion_level == PromotionArtifactLevelV1::SealedWasm)
        && (!policy
            .requirements
            .contains(&PromotionPolicyRequirementV1::ByteIdenticalWasm)
            || claims.contains(&PromotionPolicyClaimV1::ByteIdenticalWasm))
        && (!policy
            .requirements
            .contains(&PromotionPolicyRequirementV1::TargetConfigDigest)
            || claims.contains(&PromotionPolicyClaimV1::TargetConfigDigest));
    RolePromotionPolicyDecisionV1 {
        role: input.role.clone(),
        requested_promotion_level: input.promotion_level,
        allowed_promotion_levels: policy.allowed_promotion_levels.clone(),
        requirements: policy.requirements.clone(),
        claims,
        level_allowed,
        policy_satisfied,
    }
}

fn promotion_policy_claims_for_input(input: &RolePromotionInputV1) -> Vec<PromotionPolicyClaimV1> {
    let mut claims = Vec::with_capacity(2);
    if input.require_byte_identical_wasm {
        claims.push(PromotionPolicyClaimV1::ByteIdenticalWasm);
    }
    if input.require_target_embedded_config {
        claims.push(PromotionPolicyClaimV1::TargetConfigDigest);
    }
    claims
}

fn collect_policy_findings(
    decision: &RolePromotionPolicyDecisionV1,
    blockers: &mut Vec<SafetyFindingV1>,
) {
    if !decision.level_allowed {
        blockers.push(super::promotion_finding(
            "promotion_policy_level_not_allowed",
            format!(
                "role {} cannot use promotion level {:?}",
                decision.role, decision.requested_promotion_level
            ),
            SafetySeverityV1::HardFailure,
            &decision.role,
        ));
    }
    if decision
        .requirements
        .contains(&PromotionPolicyRequirementV1::SealedBytes)
        && decision.requested_promotion_level != PromotionArtifactLevelV1::SealedWasm
    {
        blockers.push(super::promotion_finding(
            "promotion_policy_must_use_sealed_bytes",
            format!("role {} must use sealed bytes", decision.role),
            SafetySeverityV1::HardFailure,
            &decision.role,
        ));
    }
    if decision
        .requirements
        .contains(&PromotionPolicyRequirementV1::ByteIdenticalWasm)
        && !decision
            .claims
            .contains(&PromotionPolicyClaimV1::ByteIdenticalWasm)
    {
        blockers.push(super::promotion_finding(
            "promotion_policy_byte_identity_required",
            format!("role {} requires byte-identical wasm", decision.role),
            SafetySeverityV1::HardFailure,
            &decision.role,
        ));
    }
    if decision
        .requirements
        .contains(&PromotionPolicyRequirementV1::TargetConfigDigest)
        && !decision
            .claims
            .contains(&PromotionPolicyClaimV1::TargetConfigDigest)
    {
        blockers.push(super::promotion_finding(
            "promotion_policy_target_config_digest_required",
            format!("role {} requires target config digest", decision.role),
            SafetySeverityV1::HardFailure,
            &decision.role,
        ));
    }
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
