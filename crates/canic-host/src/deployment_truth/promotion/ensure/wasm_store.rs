use super::super::error::{
    PromotionWasmStoreCatalogVerificationError, PromotionWasmStoreIdentityReportError,
};
use super::is_lower_hex_sha256;

pub(in crate::deployment_truth::promotion) fn ensure_wasm_store_identity_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionWasmStoreIdentityReportError> {
    if value.trim().is_empty() {
        return Err(PromotionWasmStoreIdentityReportError::MissingRequiredField { field });
    }
    Ok(())
}

pub(in crate::deployment_truth::promotion) fn ensure_wasm_store_identity_report_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionWasmStoreIdentityReportError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionWasmStoreIdentityReportError::InvalidSha256Digest { field })
    }
}

pub(in crate::deployment_truth::promotion) fn ensure_wasm_store_catalog_verification_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionWasmStoreCatalogVerificationError> {
    if value.trim().is_empty() {
        return Err(PromotionWasmStoreCatalogVerificationError::MissingRequiredField { field });
    }
    Ok(())
}

pub(in crate::deployment_truth::promotion) fn ensure_wasm_store_catalog_verification_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionWasmStoreCatalogVerificationError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionWasmStoreCatalogVerificationError::InvalidSha256Digest { field })
    }
}
