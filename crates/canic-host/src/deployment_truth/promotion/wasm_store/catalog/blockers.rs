use crate::deployment_truth::{
    RolePromotionWasmStoreCatalogVerificationV1, SafetyFindingV1, SafetySeverityV1,
};

pub(super) fn wasm_store_catalog_verification_blockers(
    roles: &[RolePromotionWasmStoreCatalogVerificationV1],
) -> Vec<SafetyFindingV1> {
    let mut blockers = Vec::new();
    for role in roles {
        if role.wasm_store_locator.is_empty() {
            blockers.push(super::super::super::promotion_finding(
                "promotion_wasm_store_catalog_locator_missing",
                format!("role {} does not record a wasm_store locator", role.role),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        } else if !role.catalog_entry_present {
            blockers.push(super::super::super::promotion_finding(
                "promotion_wasm_store_catalog_entry_missing",
                format!(
                    "role {} locator {} was not present in the wasm_store catalog observation",
                    role.role, role.wasm_store_locator
                ),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        }
        if let Some(observed) = &role.observed_artifact_identity
            && observed != &role.expected_artifact_identity
        {
            blockers.push(super::super::super::promotion_finding(
                "promotion_wasm_store_catalog_artifact_mismatch",
                format!(
                    "role {} expected artifact {} at {}, observed {}",
                    role.role, role.expected_artifact_identity, role.wasm_store_locator, observed
                ),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        }
        if let Some(observed) = role.observed_published_chunk_count
            && observed != role.expected_published_chunk_count
        {
            blockers.push(super::super::super::promotion_finding(
                "promotion_wasm_store_catalog_chunk_count_mismatch",
                format!(
                    "role {} expected {} published chunk(s) at {}, observed {}",
                    role.role,
                    role.expected_published_chunk_count,
                    role.wasm_store_locator,
                    observed
                ),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        }
    }
    blockers
}
