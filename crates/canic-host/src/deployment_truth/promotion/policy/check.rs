use crate::deployment_truth::{
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, PromotionPolicyCheckV1, PromotionReadinessStatusV1,
    RolePromotionInputV1, RolePromotionPolicyV1, SafetySeverityV1,
};
use std::collections::BTreeSet;

use super::super::digest::promotion_policy_check_digest;
use super::super::ensure::ensure_policy_field;
use super::super::error::PromotionPolicyCheckError;
use super::super::request::PromotionPolicyCheckRequest;
use super::decision::{collect_policy_findings, role_promotion_policy_decision};
use super::validation::{validate_promotion_policy_check, validate_role_promotion_policy};

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
            blockers.push(super::super::promotion_finding(
                "promotion_policy_duplicate",
                format!("multiple promotion policies exist for role {}", policy.role),
                SafetySeverityV1::HardFailure,
                &policy.role,
            ));
        }
        if let Err(err) = validate_role_promotion_policy(policy) {
            blockers.push(super::super::promotion_finding(
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
                blockers.push(super::super::promotion_finding(
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
            _ => blockers.push(super::super::promotion_finding(
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
