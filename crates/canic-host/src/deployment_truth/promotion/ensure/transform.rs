use super::super::error::{
    PromotionPlanTransformError, PromotionPlanTransformEvidenceError,
    PromotionTargetExecutionLineageError,
};
use super::is_lower_hex_sha256;

pub(in crate::deployment_truth::promotion) fn ensure_transform_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionPlanTransformError> {
    if value.trim().is_empty() {
        return Err(PromotionPlanTransformError::MissingRequiredField { field });
    }
    Ok(())
}

pub(in crate::deployment_truth::promotion) fn ensure_evidence_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionPlanTransformEvidenceError> {
    if value.trim().is_empty() {
        return Err(PromotionPlanTransformEvidenceError::MissingRequiredField { field });
    }
    Ok(())
}

pub(in crate::deployment_truth::promotion) fn ensure_evidence_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionPlanTransformEvidenceError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionPlanTransformEvidenceError::InvalidSha256Digest { field })
    }
}

pub(in crate::deployment_truth::promotion) fn ensure_target_execution_lineage_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionTargetExecutionLineageError> {
    if value.trim().is_empty() {
        return Err(PromotionTargetExecutionLineageError::MissingRequiredField { field });
    }
    Ok(())
}

pub(in crate::deployment_truth::promotion) fn ensure_target_execution_lineage_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionTargetExecutionLineageError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionTargetExecutionLineageError::InvalidSha256Digest { field })
    }
}
