mod blockers;
mod build;
mod validation;

use super::super::{
    ensure::ensure_wasm_store_catalog_verification_field,
    error::PromotionWasmStoreCatalogVerificationError,
    request::PromotionWasmStoreCatalogVerificationRequest,
};
use super::identity::validate_promotion_wasm_store_identity_report;
use crate::deployment_truth::PromotionWasmStoreCatalogVerificationV1;

use build::build_wasm_store_catalog_verification;
use validation::ensure_unique_wasm_store_catalog_entries;

pub use validation::validate_promotion_wasm_store_catalog_verification;

pub fn promotion_wasm_store_catalog_verification(
    request: PromotionWasmStoreCatalogVerificationRequest,
) -> Result<PromotionWasmStoreCatalogVerificationV1, PromotionWasmStoreCatalogVerificationError> {
    ensure_wasm_store_catalog_verification_field("verification_id", &request.verification_id)?;
    validate_promotion_wasm_store_identity_report(&request.wasm_store_identity_report)?;
    ensure_unique_wasm_store_catalog_entries(&request.catalog_entries)?;
    let verification = build_wasm_store_catalog_verification(request);
    validate_promotion_wasm_store_catalog_verification(&verification)?;
    Ok(verification)
}
