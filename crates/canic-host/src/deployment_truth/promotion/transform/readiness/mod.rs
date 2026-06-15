mod build;
mod findings;
mod validation;

use super::super::{
    ensure::ensure_readiness_field,
    error::PromotionReadinessError,
    request::{PromotionReadinessRequest, PromotionReadinessWithPolicyRequest},
};
use crate::deployment_truth::PromotionReadinessV1;

pub use build::{promotion_readiness_from_inputs, promotion_readiness_from_inputs_with_policy};
pub use validation::validate_promotion_readiness;

pub fn check_promotion_readiness(
    request: &PromotionReadinessRequest,
) -> Result<PromotionReadinessV1, PromotionReadinessError> {
    ensure_readiness_field("readiness_id", &request.readiness_id)?;
    let readiness = promotion_readiness_from_inputs(
        &request.readiness_id,
        &request.target_plan,
        &request.inputs,
    );
    validate_promotion_readiness(&readiness)?;
    Ok(readiness)
}

pub fn check_promotion_readiness_with_policy(
    request: &PromotionReadinessWithPolicyRequest,
) -> Result<PromotionReadinessV1, PromotionReadinessError> {
    ensure_readiness_field("readiness_id", &request.readiness_id)?;
    let readiness = promotion_readiness_from_inputs_with_policy(
        &request.readiness_id,
        &request.target_plan,
        &request.inputs,
        &request.policies,
    );
    validate_promotion_readiness(&readiness)?;
    Ok(readiness)
}
