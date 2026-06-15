mod artifact_plan;
mod identity;
mod materialization;
mod policy;
mod provenance;
mod readiness;
mod source;
mod transform;
mod wasm_store;

pub(super) use artifact_plan::{
    ensure_artifact_promotion_plan_field, ensure_artifact_promotion_plan_sha256,
};
pub(super) use identity::{
    ensure_identity_optional_sha256, ensure_identity_report_field, ensure_identity_report_sha256,
};
pub(super) use materialization::{
    ensure_materialization_field, ensure_materialization_link, ensure_materialization_report_field,
    ensure_materialization_report_sha256, ensure_materialization_sha256,
};
pub(super) use policy::{ensure_policy_field, ensure_policy_sha256};
pub(super) use provenance::{
    ensure_execution_receipt_field, ensure_execution_receipt_sha256,
    ensure_provenance_report_field, ensure_provenance_report_sha256,
};
pub(super) use readiness::{
    ensure_readiness_field, ensure_readiness_optional_sha256, ensure_readiness_sha256,
};
pub(super) use source::{
    ensure_digest_requirement, ensure_field, ensure_locator_requirement, ensure_optional_sha256,
    ensure_previous_receipt_lineage_digest_requirement, ensure_previous_receipt_requirement,
};
pub(super) use transform::{
    ensure_evidence_field, ensure_evidence_sha256, ensure_target_execution_lineage_field,
    ensure_target_execution_lineage_sha256, ensure_transform_field,
};
pub(super) use wasm_store::{
    ensure_wasm_store_catalog_verification_field, ensure_wasm_store_catalog_verification_sha256,
    ensure_wasm_store_identity_report_field, ensure_wasm_store_identity_report_sha256,
};

fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
