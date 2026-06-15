mod evidence;
mod plan;
mod readiness;
mod source;

pub use evidence::{promotion_plan_transform_evidence, validate_promotion_plan_transform_evidence};
pub use plan::{
    promoted_deployment_plan_from_inputs, promoted_deployment_plan_transform_from_inputs,
    promoted_deployment_plan_transform_from_inputs_with_materialization,
    validate_promotion_plan_transform,
};
pub use readiness::{
    check_promotion_readiness, check_promotion_readiness_with_policy,
    promotion_readiness_from_inputs, promotion_readiness_from_inputs_with_policy,
    validate_promotion_readiness,
};
pub use source::validate_role_artifact_source;

pub(super) use plan::ensure_role_field_matches;
