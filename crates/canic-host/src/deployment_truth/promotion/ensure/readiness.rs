use super::super::error::PromotionReadinessError;
use super::is_lower_hex_sha256;

pub(in crate::deployment_truth::promotion) fn ensure_readiness_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionReadinessError> {
    if value.trim().is_empty() {
        return Err(PromotionReadinessError::MissingRequiredField { field });
    }
    Ok(())
}

pub(in crate::deployment_truth::promotion) fn ensure_readiness_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionReadinessError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionReadinessError::InvalidSha256Digest { field })
    }
}

pub(in crate::deployment_truth::promotion) fn ensure_readiness_optional_sha256(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), PromotionReadinessError> {
    let Some(value) = value else {
        return Ok(());
    };
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionReadinessError::InvalidSha256Digest { field })
    }
}
