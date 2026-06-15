mod build;
mod validation;

pub use build::{
    promoted_deployment_plan_from_inputs, promoted_deployment_plan_transform_from_inputs,
    promoted_deployment_plan_transform_from_inputs_with_materialization,
};
pub use validation::validate_promotion_plan_transform;

pub(in crate::deployment_truth::promotion) use validation::ensure_role_field_matches;
