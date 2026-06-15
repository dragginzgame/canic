use super::super::error::{
    CriticalExternalFixReportError, ExternalLifecycleCheckError, ExternalLifecycleHandoffError,
    ExternalLifecyclePendingReportError,
};

pub(super) fn ensure_external_pending_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalLifecyclePendingReportError> {
    if value.trim().is_empty() {
        return Err(ExternalLifecyclePendingReportError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_external_lifecycle_check_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalLifecycleCheckError> {
    if value.trim().is_empty() {
        return Err(ExternalLifecycleCheckError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_external_lifecycle_handoff_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalLifecycleHandoffError> {
    if value.trim().is_empty() {
        return Err(ExternalLifecycleHandoffError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_critical_fix_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), CriticalExternalFixReportError> {
    if value.trim().is_empty() {
        return Err(CriticalExternalFixReportError::MissingRequiredField { field });
    }
    Ok(())
}
