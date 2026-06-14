mod artifact_plan;
mod digest;
mod ensure;
mod error;
mod identity;
mod materialization;
mod policy;
mod provenance;
mod request;
mod transform;
mod wasm_store;

use super::{SafetyFindingV1, SafetySeverityV1};
pub use artifact_plan::{
    artifact_promotion_plan, promotion_target_execution_lineage, validate_artifact_promotion_plan,
    validate_artifact_promotion_plan_for_check, validate_promotion_target_execution_lineage,
};
pub use digest::{
    build_materialization_input_digest, promotion_plan_lineage_digest,
    promotion_target_execution_lineage_digest,
};
pub use error::*;
pub use identity::{
    promotion_artifact_identity_report, promotion_artifact_identity_report_from_inputs,
    validate_promotion_artifact_identity_report,
};
pub use materialization::{
    build_materialization_evidence, promotion_materialization_identity_report,
    promotion_materialization_identity_report_from_evidence,
    validate_build_materialization_evidence, validate_build_materialization_input,
    validate_build_materialization_result, validate_build_recipe_identity,
    validate_promotion_materialization_identity_report,
};
pub use policy::{
    check_promotion_policy, promotion_policy_check_from_inputs, validate_promotion_policy_check,
    validate_role_promotion_policy,
};
pub use provenance::{
    artifact_promotion_execution_receipt, artifact_promotion_provenance_report,
    validate_artifact_promotion_execution_receipt, validate_artifact_promotion_provenance_report,
};
pub use request::*;
pub use transform::{
    check_promotion_readiness, check_promotion_readiness_with_policy,
    promoted_deployment_plan_from_inputs, promoted_deployment_plan_transform_from_inputs,
    promoted_deployment_plan_transform_from_inputs_with_materialization,
    promotion_plan_transform_evidence, promotion_readiness_from_inputs,
    promotion_readiness_from_inputs_with_policy, validate_promotion_plan_transform,
    validate_promotion_plan_transform_evidence, validate_promotion_readiness,
    validate_role_artifact_source,
};
pub use wasm_store::{
    promotion_wasm_store_catalog_verification, promotion_wasm_store_identity_report,
    promotion_wasm_store_identity_report_from_staging,
    validate_promotion_wasm_store_catalog_verification,
    validate_promotion_wasm_store_identity_report,
};

fn promotion_finding(
    code: impl Into<String>,
    message: impl Into<String>,
    severity: SafetySeverityV1,
    role: &str,
) -> SafetyFindingV1 {
    SafetyFindingV1 {
        code: code.into(),
        message: message.into(),
        severity,
        subject: Some(role.to_string()),
    }
}
