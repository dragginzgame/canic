use super::super::error::PromotionArtifactIdentityReportError;
use super::is_lower_hex_sha256;

pub(in crate::deployment_truth::promotion) fn ensure_identity_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionArtifactIdentityReportError> {
    if value.trim().is_empty() {
        return Err(PromotionArtifactIdentityReportError::MissingRequiredField { field });
    }
    Ok(())
}

pub(in crate::deployment_truth::promotion) fn ensure_identity_optional_sha256(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), PromotionArtifactIdentityReportError> {
    let Some(value) = value else {
        return Ok(());
    };
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionArtifactIdentityReportError::InvalidSha256Digest { field })
    }
}

pub(in crate::deployment_truth::promotion) fn ensure_identity_report_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionArtifactIdentityReportError> {
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionArtifactIdentityReportError::InvalidSha256Digest { field })
    }
}
