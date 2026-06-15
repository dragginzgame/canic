use super::super::error::{
    ArtifactPromotionExecutionReceiptError, ArtifactPromotionProvenanceReportError,
};
use super::is_lower_hex_sha256;

pub(in crate::deployment_truth::promotion) fn ensure_provenance_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    if value.trim().is_empty() {
        return Err(ArtifactPromotionProvenanceReportError::MissingRequiredField { field });
    }
    Ok(())
}

pub(in crate::deployment_truth::promotion) fn ensure_provenance_report_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(ArtifactPromotionProvenanceReportError::InvalidSha256Digest { field })
    }
}

pub(in crate::deployment_truth::promotion) fn ensure_execution_receipt_field(
    field: &'static str,
    value: &str,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    if value.trim().is_empty() {
        return Err(ArtifactPromotionExecutionReceiptError::MissingRequiredField { field });
    }
    Ok(())
}

pub(in crate::deployment_truth::promotion) fn ensure_execution_receipt_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch { field })
    }
}
