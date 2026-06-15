use super::super::error::ArtifactPromotionPlanError;
use super::is_lower_hex_sha256;

pub(in crate::deployment_truth::promotion) fn ensure_artifact_promotion_plan_field(
    field: &'static str,
    value: &str,
) -> Result<(), ArtifactPromotionPlanError> {
    if value.trim().is_empty() {
        return Err(ArtifactPromotionPlanError::MissingRequiredField { field });
    }
    Ok(())
}

pub(in crate::deployment_truth::promotion) fn ensure_artifact_promotion_plan_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), ArtifactPromotionPlanError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(ArtifactPromotionPlanError::InvalidSha256Digest { field })
    }
}
