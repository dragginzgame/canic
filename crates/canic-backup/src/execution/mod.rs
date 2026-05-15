mod operation;
mod receipt;
mod types;
mod validation;

pub use types::*;

use crate::plan::{BackupExecutionPreflightReceipts, BackupOperationKind, BackupPlan};
use validation::{
    operation_kind_is_mutating, operation_kind_is_preflight, validate_nonempty,
    validate_operation_sequences, validate_optional_nonempty,
};

const BACKUP_EXECUTION_JOURNAL_VERSION: u16 = 1;
const PREFLIGHT_NOT_ACCEPTED: &str = "preflight-not-accepted";

impl BackupExecutionJournal {
    /// Build an execution journal from a validated backup plan.
    pub fn from_plan(plan: &BackupPlan) -> Result<Self, BackupExecutionJournalError> {
        plan.validate()
            .map_err(|error| BackupExecutionJournalError::InvalidPlan(error.to_string()))?;
        let operations = plan
            .phases
            .iter()
            .map(BackupExecutionJournalOperation::from_plan_operation)
            .collect::<Vec<_>>();
        let mut journal = Self {
            journal_version: BACKUP_EXECUTION_JOURNAL_VERSION,
            plan_id: plan.plan_id.clone(),
            run_id: plan.run_id.clone(),
            preflight_id: None,
            preflight_accepted: false,
            restart_required: false,
            operations,
            operation_receipts: Vec::new(),
        };
        journal.refresh_blocked_operations();
        journal.validate()?;
        Ok(journal)
    }

    /// Validate journal structure and operation receipts.
    pub fn validate(&self) -> Result<(), BackupExecutionJournalError> {
        if self.journal_version != BACKUP_EXECUTION_JOURNAL_VERSION {
            return Err(BackupExecutionJournalError::UnsupportedVersion(
                self.journal_version,
            ));
        }
        validate_nonempty("plan_id", &self.plan_id)?;
        validate_nonempty("run_id", &self.run_id)?;
        if let Some(preflight_id) = &self.preflight_id {
            validate_nonempty("preflight_id", preflight_id)?;
        } else if self.preflight_accepted {
            return Err(BackupExecutionJournalError::AcceptedPreflightMissingId);
        }
        validate_operation_sequences(&self.operations)?;
        for operation in &self.operations {
            operation.validate()?;
            if !self.preflight_accepted && operation_kind_is_mutating(&operation.kind) {
                match operation.state {
                    BackupExecutionOperationState::Blocked => {}
                    BackupExecutionOperationState::Ready
                    | BackupExecutionOperationState::Pending
                    | BackupExecutionOperationState::Completed
                    | BackupExecutionOperationState::Failed
                    | BackupExecutionOperationState::Skipped => {
                        return Err(BackupExecutionJournalError::MutationReadyBeforePreflight {
                            sequence: operation.sequence,
                        });
                    }
                }
            }
        }
        for receipt in &self.operation_receipts {
            receipt.validate_against(self)?;
        }
        Ok(())
    }

    /// Mark all preflight operations completed and unblock mutating operations.
    pub fn accept_preflight_bundle_at(
        &mut self,
        preflight_id: String,
        updated_at: Option<String>,
    ) -> Result<(), BackupExecutionJournalError> {
        validate_nonempty("preflight_id", &preflight_id)?;
        validate_optional_nonempty("updated_at", updated_at.as_deref())?;
        if let Some(existing) = &self.preflight_id
            && existing != &preflight_id
        {
            return Err(BackupExecutionJournalError::PreflightAlreadyAccepted {
                existing: existing.clone(),
                attempted: preflight_id,
            });
        }

        self.preflight_id = Some(preflight_id);
        self.preflight_accepted = true;
        for operation in &mut self.operations {
            if operation_kind_is_preflight(&operation.kind) {
                operation.state = BackupExecutionOperationState::Completed;
                operation.state_updated_at.clone_from(&updated_at);
                operation.blocking_reasons.clear();
            } else if operation.state == BackupExecutionOperationState::Blocked {
                operation.state = BackupExecutionOperationState::Ready;
                operation.blocking_reasons.clear();
            }
        }
        self.refresh_restart_required();
        self.validate()
    }

    /// Accept a typed preflight receipt bundle and unblock mutating operations.
    pub fn accept_preflight_receipts_at(
        &mut self,
        receipts: &BackupExecutionPreflightReceipts,
        updated_at: Option<String>,
    ) -> Result<(), BackupExecutionJournalError> {
        validate_nonempty("preflight_receipts.plan_id", &receipts.plan_id)?;
        if receipts.plan_id != self.plan_id {
            return Err(BackupExecutionJournalError::PreflightPlanMismatch {
                expected: self.plan_id.clone(),
                actual: receipts.plan_id.clone(),
            });
        }
        self.accept_preflight_bundle_at(receipts.preflight_id.clone(), updated_at)
    }

    /// Return the next operation that should control runner progress.
    #[must_use]
    pub fn next_ready_operation(&self) -> Option<&BackupExecutionJournalOperation> {
        self.operations
            .iter()
            .filter(|operation| {
                matches!(
                    operation.state,
                    BackupExecutionOperationState::Ready
                        | BackupExecutionOperationState::Pending
                        | BackupExecutionOperationState::Failed
                )
            })
            .min_by_key(|operation| operation.sequence)
    }

    /// Mark the next transitionable operation pending.
    pub fn mark_next_operation_pending_at(
        &mut self,
        updated_at: Option<String>,
    ) -> Result<(), BackupExecutionJournalError> {
        let sequence = self
            .next_ready_operation()
            .ok_or(BackupExecutionJournalError::NoTransitionableOperation)?
            .sequence;
        self.mark_operation_pending_at(sequence, updated_at)
    }

    /// Mark one operation pending.
    pub fn mark_operation_pending_at(
        &mut self,
        sequence: usize,
        updated_at: Option<String>,
    ) -> Result<(), BackupExecutionJournalError> {
        validate_optional_nonempty("updated_at", updated_at.as_deref())?;
        let expected = self
            .next_ready_operation()
            .ok_or(BackupExecutionJournalError::NoTransitionableOperation)?
            .sequence;
        if sequence != expected {
            return Err(BackupExecutionJournalError::OutOfOrderOperationTransition {
                requested: sequence,
                next: expected,
            });
        }
        let index = self.operation_index(sequence)?;
        let operation = &self.operations[index];
        if operation_kind_is_mutating(&operation.kind) && !self.preflight_accepted {
            return Err(BackupExecutionJournalError::MutationBeforePreflightAccepted { sequence });
        }
        if !matches!(
            operation.state,
            BackupExecutionOperationState::Ready | BackupExecutionOperationState::Failed
        ) {
            return Err(BackupExecutionJournalError::InvalidOperationTransition {
                sequence,
                from: operation.state.clone(),
                to: BackupExecutionOperationState::Pending,
            });
        }

        let operation = &mut self.operations[index];
        operation.state = BackupExecutionOperationState::Pending;
        operation.state_updated_at = updated_at;
        operation.blocking_reasons.clear();
        self.refresh_restart_required();
        self.validate()
    }

    /// Record one operation receipt and transition the matching operation.
    pub fn record_operation_receipt(
        &mut self,
        receipt: BackupExecutionOperationReceipt,
    ) -> Result<(), BackupExecutionJournalError> {
        receipt.validate_against(self)?;
        let index = self.operation_index(receipt.sequence)?;
        let operation = &self.operations[index];
        if operation.state != BackupExecutionOperationState::Pending {
            return Err(
                BackupExecutionJournalError::ReceiptWithoutPendingOperation {
                    sequence: receipt.sequence,
                },
            );
        }

        let next_state = match receipt.outcome {
            BackupExecutionOperationReceiptOutcome::Completed => {
                BackupExecutionOperationState::Completed
            }
            BackupExecutionOperationReceiptOutcome::Failed => BackupExecutionOperationState::Failed,
            BackupExecutionOperationReceiptOutcome::Skipped => {
                BackupExecutionOperationState::Skipped
            }
        };
        let failure_reason = receipt.failure_reason.clone();
        let previous_operation = self.operations[index].clone();
        let previous_restart_required = self.restart_required;
        self.operation_receipts.push(receipt);

        let operation = &mut self.operations[index];
        operation.state = next_state;
        operation.state_updated_at = self
            .operation_receipts
            .last()
            .and_then(|receipt| receipt.updated_at.clone());
        operation.blocking_reasons = failure_reason.into_iter().collect();
        self.refresh_restart_required();
        if let Err(error) = self.validate() {
            self.operation_receipts.pop();
            self.operations[index] = previous_operation;
            self.restart_required = previous_restart_required;
            return Err(error);
        }
        Ok(())
    }

    /// Move a failed operation back to ready for retry.
    pub fn retry_failed_operation_at(
        &mut self,
        sequence: usize,
        updated_at: Option<String>,
    ) -> Result<(), BackupExecutionJournalError> {
        validate_optional_nonempty("updated_at", updated_at.as_deref())?;
        let index = self.operation_index(sequence)?;
        if self.operations[index].state != BackupExecutionOperationState::Failed {
            return Err(BackupExecutionJournalError::OperationNotFailed(sequence));
        }
        self.operations[index].state = BackupExecutionOperationState::Ready;
        self.operations[index].state_updated_at = updated_at;
        self.operations[index].blocking_reasons.clear();
        self.refresh_restart_required();
        self.validate()
    }

    /// Build a compact resumability summary.
    #[must_use]
    pub fn resume_summary(&self) -> BackupExecutionResumeSummary {
        let mut summary = BackupExecutionResumeSummary {
            plan_id: self.plan_id.clone(),
            run_id: self.run_id.clone(),
            preflight_id: self.preflight_id.clone(),
            preflight_accepted: self.preflight_accepted,
            restart_required: self.restart_required,
            total_operations: self.operations.len(),
            ready_operations: 0,
            pending_operations: 0,
            blocked_operations: 0,
            completed_operations: 0,
            failed_operations: 0,
            skipped_operations: 0,
            next_operation: self.next_ready_operation().cloned(),
        };
        for operation in &self.operations {
            match operation.state {
                BackupExecutionOperationState::Ready => summary.ready_operations += 1,
                BackupExecutionOperationState::Pending => summary.pending_operations += 1,
                BackupExecutionOperationState::Blocked => summary.blocked_operations += 1,
                BackupExecutionOperationState::Completed => summary.completed_operations += 1,
                BackupExecutionOperationState::Failed => summary.failed_operations += 1,
                BackupExecutionOperationState::Skipped => summary.skipped_operations += 1,
            }
        }
        summary
    }

    fn operation_index(&self, sequence: usize) -> Result<usize, BackupExecutionJournalError> {
        self.operations
            .iter()
            .position(|operation| operation.sequence == sequence)
            .ok_or(BackupExecutionJournalError::OperationNotFound(sequence))
    }

    fn refresh_blocked_operations(&mut self) {
        if self.preflight_accepted {
            return;
        }
        for operation in &mut self.operations {
            if operation_kind_is_mutating(&operation.kind) {
                operation.state = BackupExecutionOperationState::Blocked;
                operation.blocking_reasons = vec![PREFLIGHT_NOT_ACCEPTED.to_string()];
            }
        }
    }

    fn refresh_restart_required(&mut self) {
        let stopped = self.operations.iter().any(|operation| {
            operation.kind == BackupOperationKind::Stop
                && operation.state == BackupExecutionOperationState::Completed
        });
        let unstarted = self.operations.iter().any(|operation| {
            operation.kind == BackupOperationKind::Start
                && !matches!(
                    operation.state,
                    BackupExecutionOperationState::Completed
                        | BackupExecutionOperationState::Skipped
                )
        });
        self.restart_required = stopped && unstarted;
    }
}

#[cfg(test)]
mod tests;
