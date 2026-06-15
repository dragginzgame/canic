use super::super::error::PromotionPolicyCheckError;
use super::is_lower_hex_sha256;

pub(in crate::deployment_truth::promotion) fn ensure_policy_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionPolicyCheckError> {
    if value.trim().is_empty() {
        return Err(PromotionPolicyCheckError::MissingRequiredField { field });
    }
    Ok(())
}

pub(in crate::deployment_truth::promotion) fn ensure_policy_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionPolicyCheckError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionPolicyCheckError::InvalidSha256Digest { field })
    }
}
