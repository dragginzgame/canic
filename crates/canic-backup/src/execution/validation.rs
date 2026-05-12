use super::{BackupExecutionJournalError, BackupExecutionJournalOperation};
use crate::plan::BackupOperationKind;

pub(super) const fn operation_kind_is_preflight(kind: &BackupOperationKind) -> bool {
    matches!(
        kind,
        BackupOperationKind::ValidateTopology
            | BackupOperationKind::ValidateControlAuthority
            | BackupOperationKind::ValidateSnapshotReadAuthority
            | BackupOperationKind::ValidateQuiescencePolicy
    )
}

pub(super) const fn operation_kind_is_mutating(kind: &BackupOperationKind) -> bool {
    !operation_kind_is_preflight(kind)
}

pub(super) fn validate_operation_sequences(
    operations: &[BackupExecutionJournalOperation],
) -> Result<(), BackupExecutionJournalError> {
    let mut sequences = std::collections::BTreeSet::new();
    for operation in operations {
        if !sequences.insert(operation.sequence) {
            return Err(BackupExecutionJournalError::DuplicateSequence(
                operation.sequence,
            ));
        }
    }
    for expected in 0..operations.len() {
        if !sequences.contains(&expected) {
            return Err(BackupExecutionJournalError::MissingSequence(expected));
        }
    }
    Ok(())
}

pub(super) fn validate_nonempty(
    field: &'static str,
    value: &str,
) -> Result<(), BackupExecutionJournalError> {
    if value.trim().is_empty() {
        Err(BackupExecutionJournalError::MissingField(field))
    } else {
        Ok(())
    }
}

pub(super) fn validate_optional_nonempty(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), BackupExecutionJournalError> {
    match value {
        Some(value) => validate_nonempty(field, value),
        None => Ok(()),
    }
}
