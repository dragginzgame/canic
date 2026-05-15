use super::{RestoreApplyDryRun, RestoreApplyDryRunOperation};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

mod commands;
mod counts;
mod receipts;
mod reports;
mod types;

pub use commands::{
    RestoreApplyCommandConfig, RestoreApplyCommandPreview, RestoreApplyRunnerCommand,
};
use counts::RestoreApplyJournalStateCounts;
pub use counts::RestoreApplyOperationKindCounts;
pub(in crate::restore) use receipts::RestoreApplyCommandOutputPair;
pub use receipts::{
    RestoreApplyCommandOutput, RestoreApplyOperationReceipt, RestoreApplyOperationReceiptOutcome,
};
pub(in crate::restore) use reports::RestoreApplyJournalReport;
pub use reports::{
    RestoreApplyPendingSummary, RestoreApplyProgressSummary, RestoreApplyReportOperation,
    RestoreApplyReportOutcome,
};
pub use types::{
    RestoreApplyJournalError, RestoreApplyJournalOperation, RestoreApplyOperationKind,
    RestoreApplyOperationState,
};
use types::{
    restore_apply_blocked_reasons, validate_apply_journal_count, validate_apply_journal_nonempty,
    validate_apply_journal_sequences, validate_apply_journal_version,
};

///
/// RestoreApplyJournal
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyJournal {
    pub journal_version: u16,
    pub backup_id: String,
    pub ready: bool,
    pub blocked_reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_root: Option<String>,
    pub operation_count: usize,
    pub operation_counts: RestoreApplyOperationKindCounts,
    pub pending_operations: usize,
    pub ready_operations: usize,
    pub blocked_operations: usize,
    pub completed_operations: usize,
    pub failed_operations: usize,
    pub operations: Vec<RestoreApplyJournalOperation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operation_receipts: Vec<RestoreApplyOperationReceipt>,
}

impl RestoreApplyJournal {
    /// Build the initial no-mutation restore apply journal from a dry-run.
    #[must_use]
    pub fn from_dry_run(dry_run: &RestoreApplyDryRun) -> Self {
        let blocked_reasons = restore_apply_blocked_reasons(dry_run);
        let initial_state = if blocked_reasons.is_empty() {
            RestoreApplyOperationState::Ready
        } else {
            RestoreApplyOperationState::Blocked
        };
        let operations = dry_run
            .operations
            .iter()
            .map(|operation| {
                RestoreApplyJournalOperation::from_dry_run_operation(
                    operation,
                    initial_state.clone(),
                    &blocked_reasons,
                )
            })
            .collect::<Vec<_>>();
        let ready_operations = operations
            .iter()
            .filter(|operation| operation.state == RestoreApplyOperationState::Ready)
            .count();
        let blocked_operations = operations
            .iter()
            .filter(|operation| operation.state == RestoreApplyOperationState::Blocked)
            .count();
        let operation_counts = RestoreApplyOperationKindCounts::from_operations(&operations);

        Self {
            journal_version: 1,
            backup_id: dry_run.backup_id.clone(),
            ready: blocked_reasons.is_empty(),
            blocked_reasons,
            backup_root: dry_run
                .artifact_validation
                .as_ref()
                .map(|validation| validation.backup_root.clone()),
            operation_count: operations.len(),
            operation_counts,
            pending_operations: 0,
            ready_operations,
            blocked_operations,
            completed_operations: 0,
            failed_operations: 0,
            operations,
            operation_receipts: Vec::new(),
        }
    }

    /// Validate the structural consistency of a restore apply journal.
    pub fn validate(&self) -> Result<(), RestoreApplyJournalError> {
        validate_apply_journal_version(self.journal_version)?;
        validate_apply_journal_nonempty("backup_id", &self.backup_id)?;
        if let Some(backup_root) = &self.backup_root {
            validate_apply_journal_nonempty("backup_root", backup_root)?;
        }
        validate_apply_journal_count(
            "operation_count",
            self.operation_count,
            self.operations.len(),
        )?;

        let state_counts = RestoreApplyJournalStateCounts::from_operations(&self.operations);
        let operation_counts = RestoreApplyOperationKindCounts::from_operations(&self.operations);
        self.operation_counts.validate_matches(&operation_counts)?;
        validate_apply_journal_count(
            "pending_operations",
            self.pending_operations,
            state_counts.pending,
        )?;
        validate_apply_journal_count(
            "ready_operations",
            self.ready_operations,
            state_counts.ready,
        )?;
        validate_apply_journal_count(
            "blocked_operations",
            self.blocked_operations,
            state_counts.blocked,
        )?;
        validate_apply_journal_count(
            "completed_operations",
            self.completed_operations,
            state_counts.completed,
        )?;
        validate_apply_journal_count(
            "failed_operations",
            self.failed_operations,
            state_counts.failed,
        )?;

        if self.ready && (!self.blocked_reasons.is_empty() || self.blocked_operations > 0) {
            return Err(RestoreApplyJournalError::ReadyJournalHasBlockingState);
        }

        validate_apply_journal_sequences(&self.operations)?;
        for operation in &self.operations {
            operation.validate()?;
        }
        self.validate_operation_receipt_attempts()?;
        for receipt in &self.operation_receipts {
            receipt.validate_against(self)?;
        }

        Ok(())
    }

    /// Build an operator-oriented report from this apply journal.
    #[must_use]
    pub(in crate::restore) fn report(&self) -> RestoreApplyJournalReport {
        RestoreApplyJournalReport::from_journal(self)
    }

    /// Return the next ready or pending operation that controls runner progress.
    #[must_use]
    pub(in crate::restore) fn next_transition_operation(
        &self,
    ) -> Option<&RestoreApplyJournalOperation> {
        self.operations
            .iter()
            .filter(|operation| {
                matches!(
                    operation.state,
                    RestoreApplyOperationState::Ready
                        | RestoreApplyOperationState::Pending
                        | RestoreApplyOperationState::Failed
                )
            })
            .min_by_key(|operation| operation.sequence)
    }

    /// Render the next transitionable operation as a no-execute command preview.
    #[must_use]
    pub fn next_command_preview(&self) -> RestoreApplyCommandPreview {
        RestoreApplyCommandPreview::from_journal(self)
    }

    /// Render the next transitionable operation with a configured command preview.
    #[must_use]
    pub(in crate::restore) fn next_command_preview_with_config(
        &self,
        config: &RestoreApplyCommandConfig,
    ) -> RestoreApplyCommandPreview {
        RestoreApplyCommandPreview::from_journal_with_config(self, config)
    }

    /// Store one durable operation receipt/output and revalidate the journal.
    pub(in crate::restore) fn record_operation_receipt(
        &mut self,
        receipt: RestoreApplyOperationReceipt,
    ) -> Result<(), RestoreApplyJournalError> {
        self.operation_receipts.push(receipt);
        if let Err(error) = self.validate() {
            self.operation_receipts.pop();
            return Err(error);
        }

        Ok(())
    }

    /// Mark the next transitionable operation pending with an update marker.
    pub fn mark_next_operation_pending_at(
        &mut self,
        updated_at: Option<String>,
    ) -> Result<(), RestoreApplyJournalError> {
        let sequence = self
            .next_transition_sequence()
            .ok_or(RestoreApplyJournalError::NoTransitionableOperation)?;
        self.mark_operation_pending_at(sequence, updated_at)
    }

    /// Mark one restore apply operation pending with an update marker.
    pub(in crate::restore) fn mark_operation_pending_at(
        &mut self,
        sequence: usize,
        updated_at: Option<String>,
    ) -> Result<(), RestoreApplyJournalError> {
        self.transition_operation(
            sequence,
            RestoreApplyOperationState::Pending,
            Vec::new(),
            updated_at,
        )
    }

    /// Mark the current pending operation ready again with an update marker.
    pub(in crate::restore) fn mark_next_operation_ready_at(
        &mut self,
        updated_at: Option<String>,
    ) -> Result<(), RestoreApplyJournalError> {
        let operation = self
            .next_transition_operation()
            .ok_or(RestoreApplyJournalError::NoTransitionableOperation)?;
        if operation.state != RestoreApplyOperationState::Pending {
            return Err(RestoreApplyJournalError::NoPendingOperation);
        }

        self.mark_operation_ready_at(operation.sequence, updated_at)
    }

    /// Mark one restore apply operation ready again with an update marker.
    pub(in crate::restore) fn mark_operation_ready_at(
        &mut self,
        sequence: usize,
        updated_at: Option<String>,
    ) -> Result<(), RestoreApplyJournalError> {
        self.transition_operation(
            sequence,
            RestoreApplyOperationState::Ready,
            Vec::new(),
            updated_at,
        )
    }

    /// Retry one failed restore apply operation by moving it back to ready.
    pub fn retry_failed_operation_at(
        &mut self,
        sequence: usize,
        updated_at: Option<String>,
    ) -> Result<(), RestoreApplyJournalError> {
        self.transition_operation(
            sequence,
            RestoreApplyOperationState::Ready,
            Vec::new(),
            updated_at,
        )
    }

    /// Mark one restore apply operation completed with an update marker.
    pub(in crate::restore) fn mark_operation_completed_at(
        &mut self,
        sequence: usize,
        updated_at: Option<String>,
    ) -> Result<(), RestoreApplyJournalError> {
        self.transition_operation(
            sequence,
            RestoreApplyOperationState::Completed,
            Vec::new(),
            updated_at,
        )
    }

    /// Mark one restore apply operation failed with an update marker.
    pub(in crate::restore) fn mark_operation_failed_at(
        &mut self,
        sequence: usize,
        reason: String,
        updated_at: Option<String>,
    ) -> Result<(), RestoreApplyJournalError> {
        if reason.trim().is_empty() {
            return Err(RestoreApplyJournalError::FailureReasonRequired(sequence));
        }

        self.transition_operation(
            sequence,
            RestoreApplyOperationState::Failed,
            vec![reason],
            updated_at,
        )
    }

    // Apply one legal operation state transition and revalidate the journal.
    fn transition_operation(
        &mut self,
        sequence: usize,
        next_state: RestoreApplyOperationState,
        blocking_reasons: Vec<String>,
        updated_at: Option<String>,
    ) -> Result<(), RestoreApplyJournalError> {
        let index = self
            .operations
            .iter()
            .position(|operation| operation.sequence == sequence)
            .ok_or(RestoreApplyJournalError::OperationNotFound(sequence))?;
        let operation = &self.operations[index];

        if !operation.can_transition_to(&next_state) {
            return Err(RestoreApplyJournalError::InvalidOperationTransition {
                sequence,
                from: operation.state.clone(),
                to: next_state,
            });
        }

        self.validate_operation_transition_order(operation, &next_state)?;

        let operation = &mut self.operations[index];
        operation.state = next_state;
        operation.blocking_reasons = blocking_reasons;
        operation.state_updated_at = updated_at;
        self.refresh_operation_counts();
        self.validate()
    }

    // Ensure fresh operation transitions advance in journal order.
    fn validate_operation_transition_order(
        &self,
        operation: &RestoreApplyJournalOperation,
        next_state: &RestoreApplyOperationState,
    ) -> Result<(), RestoreApplyJournalError> {
        if operation.state == *next_state {
            return Ok(());
        }

        let next_sequence = self
            .next_transition_sequence()
            .ok_or(RestoreApplyJournalError::NoTransitionableOperation)?;

        if operation.sequence == next_sequence {
            return Ok(());
        }

        Err(RestoreApplyJournalError::OutOfOrderOperationTransition {
            requested: operation.sequence,
            next: next_sequence,
        })
    }

    // Return the next operation sequence that can be advanced by a runner.
    fn next_transition_sequence(&self) -> Option<usize> {
        self.next_transition_operation()
            .map(|operation| operation.sequence)
    }

    // Recompute operation counts after a journal operation state change.
    fn refresh_operation_counts(&mut self) {
        let state_counts = RestoreApplyJournalStateCounts::from_operations(&self.operations);
        self.operation_count = self.operations.len();
        self.operation_counts = RestoreApplyOperationKindCounts::from_operations(&self.operations);
        self.pending_operations = state_counts.pending;
        self.ready_operations = state_counts.ready;
        self.blocked_operations = state_counts.blocked;
        self.completed_operations = state_counts.completed;
        self.failed_operations = state_counts.failed;
    }

    // Return whether every planned operation has completed.
    pub(super) const fn is_complete(&self) -> bool {
        self.operation_count > 0 && self.completed_operations == self.operation_count
    }

    // Recompute operation-kind counts from concrete operation rows.
    pub(super) fn operation_kind_counts(&self) -> RestoreApplyOperationKindCounts {
        RestoreApplyOperationKindCounts::from_operations(&self.operations)
    }

    // Ensure one operation attempt has exactly one durable command outcome.
    fn validate_operation_receipt_attempts(&self) -> Result<(), RestoreApplyJournalError> {
        let mut attempts = BTreeSet::new();
        for receipt in &self.operation_receipts {
            if !attempts.insert((receipt.sequence, receipt.attempt)) {
                return Err(RestoreApplyJournalError::DuplicateOperationReceiptAttempt {
                    sequence: receipt.sequence,
                    attempt: receipt.attempt,
                });
            }
        }

        Ok(())
    }

    // Find the uploaded target snapshot ID required by one load operation.
    pub(super) fn uploaded_snapshot_id_for_load(
        &self,
        load: &RestoreApplyJournalOperation,
    ) -> Option<&str> {
        self.operation_receipts
            .iter()
            .find(|receipt| {
                receipt.matches_load_operation(load)
                    && self.operations.iter().any(|operation| {
                        operation.sequence == receipt.sequence
                            && operation.operation == RestoreApplyOperationKind::UploadSnapshot
                            && operation.state == RestoreApplyOperationState::Completed
                    })
            })
            .and_then(|receipt| receipt.uploaded_snapshot_id.as_deref())
    }
}
