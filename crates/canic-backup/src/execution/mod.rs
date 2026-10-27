//! Module: execution
//!
//! Responsibility: initialize and inspect retained backup execution journals.
//! Does not own: backup plan construction, artifact IO, or manifest storage.
//! Boundary: validates retained records without granting capture execution authority.

mod operation;
mod receipt;
#[cfg(test)]
mod tests;
mod types;
mod validation;

pub use types::*;

use crate::plan::{BackupOperationKind, BackupPlan};
use validation::{operation_kind_is_mutating, validate_nonempty, validate_operation_sequences};

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
            command_custody: Vec::new(),
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
        if self.restart_required != self.derived_restart_required() {
            return Err(BackupExecutionJournalError::RestartRequiredMismatch);
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
        let mut command_sequences = std::collections::BTreeSet::new();
        for record in &self.command_custody {
            let operation = self
                .operations
                .iter()
                .find(|operation| operation.sequence as u64 == record.operation_sequence());
            let external_operation = operation.is_some_and(|operation| {
                matches!(
                    operation.kind,
                    BackupOperationKind::Stop
                        | BackupOperationKind::CreateSnapshot
                        | BackupOperationKind::Start
                        | BackupOperationKind::DownloadSnapshot
                )
            });
            if !external_operation || !command_sequences.insert(record.operation_sequence()) {
                return Err(BackupExecutionJournalError::InvalidCommandCustody);
            }
        }
        for receipt in &self.operation_receipts {
            receipt.validate_against(self)?;
        }
        Ok(())
    }

    /// Return the earliest unresolved operation for status reporting.
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

    fn derived_restart_required(&self) -> bool {
        self.operations.iter().any(|stop| {
            stop.kind == BackupOperationKind::Stop
                && stop.state == BackupExecutionOperationState::Completed
                && self.operations.iter().any(|start| {
                    start.kind == BackupOperationKind::Start
                        && start.target_canister_id == stop.target_canister_id
                        && !matches!(
                            start.state,
                            BackupExecutionOperationState::Completed
                                | BackupExecutionOperationState::Skipped
                        )
                })
        })
    }
}
