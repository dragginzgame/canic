//! Module: restore::apply::journal
//!
//! Responsibility: persist and advance restore apply operation state.
//! Does not own: restore plan construction, command execution, or backup artifact checksums.
//! Boundary: turns dry-runs into durable journals, reports, previews, and receipts.

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

use crate::restore::{RestoreApplyDryRun, RestoreApplyDryRunOperation};

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use types::{
    restore_apply_blocked_reasons, validate_apply_journal_count, validate_apply_journal_nonempty,
    validate_apply_journal_version,
};

///
/// RestoreApplyJournal
///
/// Durable restore apply operation journal.
/// Owned by restore apply journaling and consumed by restore runners and reports.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreApplyJournal {
    pub journal_version: u16,
    pub backup_id: String,
    pub ready: bool,
    pub blocked_reasons: Vec<String>,
    #[serde(deserialize_with = "crate::serialization::required_option")]
    pub backup_root: Option<String>,
    pub operation_count: usize,
    pub operation_counts: RestoreApplyOperationKindCounts,
    pub pending_operations: usize,
    pub ready_operations: usize,
    pub blocked_operations: usize,
    pub completed_operations: usize,
    pub failed_operations: usize,
    pub operations: Vec<RestoreApplyJournalOperation>,
    pub command_custody: Vec<ic_backup::model::command_custody::CommandCustodyRecord>,
    pub operation_receipts: Vec<RestoreApplyOperationReceipt>,
}

impl RestoreApplyJournal {
    /// Retain one exact original command identity before any external dispatch.
    pub(crate) fn retain_command_custody(
        &mut self,
        record: &ic_backup::model::command_custody::CommandCustodyRecord,
    ) -> Result<(), RestoreApplyJournalError> {
        if let Some(original) = self
            .command_custody
            .iter()
            .find(|original| original.operation_sequence() == record.operation_sequence())
        {
            return if original == record {
                Ok(())
            } else {
                Err(RestoreApplyJournalError::InvalidCommandCustody)
            };
        }
        self.command_custody.push(record.clone());
        if let Err(error) = self.validate() {
            self.command_custody.pop();
            return Err(error);
        }
        Ok(())
    }

    /// Build the initial no-mutation restore apply journal from a dry-run.
    pub fn from_dry_run(dry_run: &RestoreApplyDryRun) -> Result<Self, RestoreApplyJournalError> {
        dry_run.validate()?;
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

        let journal = Self {
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
            command_custody: Vec::new(),
            operation_receipts: Vec::new(),
        };
        journal.validate()?;
        Ok(journal)
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

        self.next_effect_sequence()?;
        for operation in &self.operations {
            operation.validate()?;
        }
        let mut command_sequences = BTreeSet::new();
        for record in &self.command_custody {
            let operation = self
                .operations
                .iter()
                .find(|operation| operation.sequence as u64 == record.operation_sequence());
            let external_operation = operation.is_some_and(|operation| {
                matches!(
                    operation.operation,
                    RestoreApplyOperationKind::UploadSnapshot
                        | RestoreApplyOperationKind::StopCanister
                        | RestoreApplyOperationKind::LoadSnapshot
                        | RestoreApplyOperationKind::StartCanister
                )
            });
            if !external_operation || !command_sequences.insert(record.operation_sequence()) {
                return Err(RestoreApplyJournalError::InvalidCommandCustody);
            }
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
        let sequence = self.next_effect_sequence().ok().flatten()?;
        self.operations.iter().find(|operation| {
            operation.sequence == sequence && operation.state != RestoreApplyOperationState::Blocked
        })
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
            .next_effect_sequence()?
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

    /// Mark one upload pending with its exact pre-effect snapshot inventory.
    pub(in crate::restore) fn mark_upload_snapshot_pending_at(
        &mut self,
        sequence: usize,
        updated_at: Option<String>,
        snapshot_ids_before: Vec<String>,
    ) -> Result<(), RestoreApplyJournalError> {
        let index = self
            .operations
            .iter()
            .position(|operation| operation.sequence == sequence)
            .ok_or(RestoreApplyJournalError::OperationNotFound(sequence))?;
        if self.operations[index].operation != RestoreApplyOperationKind::UploadSnapshot {
            return Err(RestoreApplyJournalError::UnexpectedSnapshotInventory(
                sequence,
            ));
        }
        let previous = self.operations[index].snapshot_ids_before.clone();
        self.operations[index].snapshot_ids_before = Some(snapshot_ids_before);
        if let Err(error) = self.transition_operation(
            sequence,
            RestoreApplyOperationState::Pending,
            Vec::new(),
            updated_at,
        ) {
            self.operations[index].snapshot_ids_before = previous;
            return Err(error);
        }
        Ok(())
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

        if operation.state != next_state {
            let next = self
                .next_effect_sequence()?
                .ok_or(RestoreApplyJournalError::NoTransitionableOperation)?;
            if sequence != next {
                return Err(RestoreApplyJournalError::OutOfOrderOperationTransition {
                    requested: sequence,
                    next,
                });
            }
        }

        let operation = &mut self.operations[index];
        operation.state = next_state;
        operation.blocking_reasons = blocking_reasons;
        operation.state_updated_at = updated_at;
        self.refresh_operation_counts();
        self.validate()
    }

    // Validate the existing runner's serial journal; these states are not upstream receipts.
    fn next_effect_sequence(&self) -> Result<Option<usize>, RestoreApplyJournalError> {
        let Some(graph) = crate::restore::ordering::serial_effect_graph(
            self.operations.iter().map(|operation| operation.sequence),
        )?
        else {
            return Ok(None);
        };
        let completed: BTreeSet<_> = self
            .operations
            .iter()
            .filter(|operation| operation.state == RestoreApplyOperationState::Completed)
            .map(|operation| operation.sequence as u64)
            .collect();
        let mut next = None;
        for node in graph.ordered_nodes() {
            let sequence = node.operation_sequence();
            if completed.contains(&sequence) {
                if let Some(&dependency) =
                    node.depends_on().iter().find(|id| !completed.contains(id))
                {
                    return Err(RestoreApplyJournalError::UnmetCompletedDependency {
                        operation_sequence: sequence,
                        dependency,
                    });
                }
            } else if next.is_none() {
                next = Some(
                    usize::try_from(sequence).expect("operation identity originated as usize"),
                );
            }
        }
        Ok(next)
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
            if receipt.attempt == 0 {
                return Err(RestoreApplyJournalError::InvalidOperationReceiptAttempt {
                    sequence: receipt.sequence,
                    attempt: receipt.attempt,
                });
            }
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
