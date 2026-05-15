use super::{
    BackupExecutionJournalError, BackupExecutionJournalOperation, BackupExecutionOperationState,
    validation::{validate_nonempty, validate_optional_nonempty},
};

impl BackupExecutionJournalOperation {
    pub(super) fn from_plan_operation(operation: &crate::plan::BackupOperation) -> Self {
        Self {
            sequence: usize::try_from(operation.order).unwrap_or(usize::MAX),
            operation_id: operation.operation_id.clone(),
            kind: operation.kind.clone(),
            target_canister_id: operation.target_canister_id.clone(),
            state: BackupExecutionOperationState::Ready,
            state_updated_at: None,
            blocking_reasons: Vec::new(),
        }
    }

    pub(super) fn validate(&self) -> Result<(), BackupExecutionJournalError> {
        validate_nonempty("operations[].operation_id", &self.operation_id)?;
        validate_optional_nonempty(
            "operations[].state_updated_at",
            self.state_updated_at.as_deref(),
        )?;
        validate_optional_nonempty(
            "operations[].target_canister_id",
            self.target_canister_id.as_deref(),
        )?;
        if matches!(
            self.state,
            BackupExecutionOperationState::Pending
                | BackupExecutionOperationState::Completed
                | BackupExecutionOperationState::Failed
                | BackupExecutionOperationState::Skipped
        ) && self.state_updated_at.is_none()
        {
            return Err(BackupExecutionJournalError::MissingField(
                "operations[].state_updated_at",
            ));
        }
        match self.state {
            BackupExecutionOperationState::Blocked | BackupExecutionOperationState::Failed
                if self.blocking_reasons.is_empty() =>
            {
                Err(BackupExecutionJournalError::OperationMissingReason(
                    self.sequence,
                ))
            }
            BackupExecutionOperationState::Ready
            | BackupExecutionOperationState::Pending
            | BackupExecutionOperationState::Completed
            | BackupExecutionOperationState::Skipped
                if !self.blocking_reasons.is_empty() =>
            {
                Err(BackupExecutionJournalError::UnblockedOperationHasReasons(
                    self.sequence,
                ))
            }
            BackupExecutionOperationState::Ready
            | BackupExecutionOperationState::Pending
            | BackupExecutionOperationState::Blocked
            | BackupExecutionOperationState::Completed
            | BackupExecutionOperationState::Failed
            | BackupExecutionOperationState::Skipped => Ok(()),
        }
    }
}
