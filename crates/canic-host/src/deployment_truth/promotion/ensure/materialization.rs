use super::super::error::{
    PromotionMaterializationIdentityError, PromotionMaterializationIdentityReportError,
};
use super::is_lower_hex_sha256;

pub(in crate::deployment_truth::promotion) fn ensure_materialization_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    if value.trim().is_empty() {
        return Err(PromotionMaterializationIdentityReportError::MissingRequiredField { field });
    }
    Ok(())
}

pub(in crate::deployment_truth::promotion) fn ensure_materialization_report_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    ensure_materialization_report_field(field, value)?;
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(
            PromotionMaterializationIdentityReportError::Materialization(
                PromotionMaterializationIdentityError::InvalidSha256Digest { field },
            ),
        )
    }
}

pub(in crate::deployment_truth::promotion) fn ensure_materialization_field(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionMaterializationIdentityError> {
    if value.trim().is_empty() {
        return Err(PromotionMaterializationIdentityError::MissingRequiredField { field });
    }
    Ok(())
}

pub(in crate::deployment_truth::promotion) fn ensure_materialization_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), PromotionMaterializationIdentityError> {
    ensure_materialization_field(field, value)?;
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionMaterializationIdentityError::InvalidSha256Digest { field })
    }
}

pub(in crate::deployment_truth::promotion) const fn ensure_materialization_link(
    field: &'static str,
    valid: bool,
) -> Result<(), PromotionMaterializationIdentityError> {
    if valid {
        Ok(())
    } else {
        Err(PromotionMaterializationIdentityError::LinkageMismatch { field })
    }
}
