use super::{
    BackupExecutionJournal, BackupExecutionJournalError, BackupExecutionJournalOperation,
    BackupExecutionOperationReceipt, BackupExecutionOperationReceiptOutcome,
    validation::{operation_kind_is_mutating, validate_nonempty, validate_optional_nonempty},
};
use crate::plan::BackupOperationKind;

impl BackupExecutionOperationReceipt {
    /// Build a completed operation receipt from one journal operation.
    #[must_use]
    pub fn completed(
        journal: &BackupExecutionJournal,
        operation: &BackupExecutionJournalOperation,
        updated_at: Option<String>,
    ) -> Self {
        Self::from_operation(
            journal,
            operation,
            BackupExecutionOperationReceiptOutcome::Completed,
            updated_at,
            None,
        )
    }

    /// Build a failed operation receipt from one journal operation.
    #[must_use]
    pub fn failed(
        journal: &BackupExecutionJournal,
        operation: &BackupExecutionJournalOperation,
        updated_at: Option<String>,
        failure_reason: String,
    ) -> Self {
        Self::from_operation(
            journal,
            operation,
            BackupExecutionOperationReceiptOutcome::Failed,
            updated_at,
            Some(failure_reason),
        )
    }

    fn from_operation(
        journal: &BackupExecutionJournal,
        operation: &BackupExecutionJournalOperation,
        outcome: BackupExecutionOperationReceiptOutcome,
        updated_at: Option<String>,
        failure_reason: Option<String>,
    ) -> Self {
        Self {
            plan_id: journal.plan_id.clone(),
            run_id: journal.run_id.clone(),
            preflight_id: journal.preflight_id.clone(),
            sequence: operation.sequence,
            operation_id: operation.operation_id.clone(),
            kind: operation.kind.clone(),
            target_canister_id: operation.target_canister_id.clone(),
            outcome,
            updated_at,
            snapshot_id: None,
            snapshot_taken_at_timestamp: None,
            snapshot_total_size_bytes: None,
            artifact_path: None,
            checksum: None,
            failure_reason,
        }
    }

    pub(super) fn validate_against(
        &self,
        journal: &BackupExecutionJournal,
    ) -> Result<(), BackupExecutionJournalError> {
        validate_nonempty("operation_receipts[].plan_id", &self.plan_id)?;
        validate_nonempty("operation_receipts[].run_id", &self.run_id)?;
        validate_nonempty("operation_receipts[].operation_id", &self.operation_id)?;
        validate_nonempty(
            "operation_receipts[].updated_at",
            self.updated_at.as_deref().unwrap_or_default(),
        )?;
        validate_optional_nonempty(
            "operation_receipts[].snapshot_id",
            self.snapshot_id.as_deref(),
        )?;
        validate_optional_nonempty(
            "operation_receipts[].artifact_path",
            self.artifact_path.as_deref(),
        )?;
        validate_optional_nonempty("operation_receipts[].checksum", self.checksum.as_deref())?;

        if self.plan_id != journal.plan_id || self.run_id != journal.run_id {
            return Err(BackupExecutionJournalError::ReceiptJournalMismatch {
                sequence: self.sequence,
            });
        }
        let operation = journal
            .operations
            .iter()
            .find(|operation| operation.sequence == self.sequence)
            .ok_or(BackupExecutionJournalError::ReceiptOperationNotFound(
                self.sequence,
            ))?;
        if operation.operation_id != self.operation_id
            || operation.kind != self.kind
            || operation.target_canister_id != self.target_canister_id
        {
            return Err(BackupExecutionJournalError::ReceiptOperationMismatch {
                sequence: self.sequence,
            });
        }
        if operation_kind_is_mutating(&operation.kind) && self.preflight_id != journal.preflight_id
        {
            return Err(BackupExecutionJournalError::ReceiptPreflightMismatch {
                sequence: self.sequence,
            });
        }
        if self.outcome == BackupExecutionOperationReceiptOutcome::Failed {
            validate_nonempty(
                "operation_receipts[].failure_reason",
                self.failure_reason.as_deref().unwrap_or_default(),
            )?;
        }
        if self.kind == BackupOperationKind::CreateSnapshot
            && self.outcome == BackupExecutionOperationReceiptOutcome::Completed
        {
            validate_nonempty(
                "operation_receipts[].snapshot_id",
                self.snapshot_id.as_deref().unwrap_or_default(),
            )?;
        }
        if self.kind == BackupOperationKind::DownloadSnapshot
            && self.outcome == BackupExecutionOperationReceiptOutcome::Completed
        {
            validate_nonempty(
                "operation_receipts[].artifact_path",
                self.artifact_path.as_deref().unwrap_or_default(),
            )?;
        }
        if self.kind == BackupOperationKind::VerifyArtifact
            && self.outcome == BackupExecutionOperationReceiptOutcome::Completed
        {
            validate_nonempty(
                "operation_receipts[].checksum",
                self.checksum.as_deref().unwrap_or_default(),
            )?;
        }

        Ok(())
    }
}
