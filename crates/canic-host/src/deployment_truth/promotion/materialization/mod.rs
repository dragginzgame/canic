mod evidence;
mod report;
mod transform_link;
mod validation;

pub use evidence::{
    build_materialization_evidence, validate_build_materialization_evidence,
    validate_build_materialization_input, validate_build_materialization_result,
    validate_build_recipe_identity,
};
pub use report::{
    promotion_materialization_identity_report,
    promotion_materialization_identity_report_from_evidence,
};
pub use validation::validate_promotion_materialization_identity_report;

pub(super) use transform_link::{
    attach_source_build_materialization, validate_role_materialization_link,
};
