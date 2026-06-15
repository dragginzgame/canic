use super::super::super::*;
use super::super::error::{ExternalLifecyclePlanError, LifecycleAuthorityReportError};
use std::collections::BTreeSet;

pub(super) fn ensure_unique_lifecycle_subjects(
    rows: &[LifecycleAuthorityV1],
) -> Result<(), ExternalLifecyclePlanError> {
    let mut subjects = BTreeSet::new();
    for row in rows {
        if !subjects.insert(row.subject.clone()) {
            return Err(ExternalLifecyclePlanError::DuplicateSubject {
                subject: row.subject.clone(),
            });
        }
    }
    Ok(())
}

pub(super) fn ensure_unique_authority_subjects(
    rows: &[LifecycleAuthorityV1],
) -> Result<(), LifecycleAuthorityReportError> {
    let mut subjects = BTreeSet::new();
    for row in rows {
        if !subjects.insert(row.subject.clone()) {
            return Err(LifecycleAuthorityReportError::DuplicateSubject {
                subject: row.subject.clone(),
            });
        }
    }
    Ok(())
}

pub(super) fn ensure_unique_role_upgrade_subjects(
    rows: &[ExternalLifecycleRoleUpgradeV1],
) -> Result<(), ExternalLifecyclePlanError> {
    let mut subjects = BTreeSet::new();
    for row in rows {
        if !subjects.insert(row.subject.clone()) {
            return Err(ExternalLifecyclePlanError::DuplicateSubject {
                subject: row.subject.clone(),
            });
        }
    }
    Ok(())
}

pub(super) fn ensure_external_lifecycle_plan_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalLifecyclePlanError> {
    if value.trim().is_empty() {
        return Err(ExternalLifecyclePlanError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_lifecycle_authority_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), LifecycleAuthorityReportError> {
    if value.trim().is_empty() {
        return Err(LifecycleAuthorityReportError::MissingRequiredField { field });
    }
    Ok(())
}
