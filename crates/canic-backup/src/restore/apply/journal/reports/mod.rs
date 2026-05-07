use super::{
    RestoreApplyJournal, RestoreApplyJournalOperation, RestoreApplyOperationKind,
    RestoreApplyOperationKindCounts, RestoreApplyOperationState,
};
use serde::{Deserialize, Serialize};

///
/// RestoreApplyJournalReport
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(in crate::restore) struct RestoreApplyJournalReport {
    pub report_version: u16,
    pub backup_id: String,
    pub outcome: RestoreApplyReportOutcome,
    pub attention_required: bool,
    pub ready: bool,
    pub complete: bool,
    pub blocked_reasons: Vec<String>,
    pub operation_count: usize,
    pub operation_counts: RestoreApplyOperationKindCounts,
    pub progress: RestoreApplyProgressSummary,
    pub pending_summary: RestoreApplyPendingSummary,
    pub pending_operations: usize,
    pub ready_operations: usize,
    pub blocked_operations: usize,
    pub completed_operations: usize,
    pub failed_operations: usize,
    pub next_transition: Option<RestoreApplyReportOperation>,
    pub pending: Vec<RestoreApplyReportOperation>,
    pub failed: Vec<RestoreApplyReportOperation>,
    pub blocked: Vec<RestoreApplyReportOperation>,
}

impl RestoreApplyJournalReport {
    /// Build a compact operator report from a restore apply journal.
    #[must_use]
    pub(in crate::restore) fn from_journal(journal: &RestoreApplyJournal) -> Self {
        let complete = journal.is_complete();
        let outcome = RestoreApplyReportOutcome::from_journal(journal, complete);
        let pending = report_operations_with_state(journal, RestoreApplyOperationState::Pending);
        let failed = report_operations_with_state(journal, RestoreApplyOperationState::Failed);
        let blocked = report_operations_with_state(journal, RestoreApplyOperationState::Blocked);

        Self {
            report_version: 1,
            backup_id: journal.backup_id.clone(),
            outcome: outcome.clone(),
            attention_required: outcome.attention_required(),
            ready: journal.ready,
            complete,
            blocked_reasons: journal.blocked_reasons.clone(),
            operation_count: journal.operation_count,
            operation_counts: journal.operation_kind_counts(),
            progress: RestoreApplyProgressSummary::from_journal(journal),
            pending_summary: RestoreApplyPendingSummary::from_journal(journal),
            pending_operations: journal.pending_operations,
            ready_operations: journal.ready_operations,
            blocked_operations: journal.blocked_operations,
            completed_operations: journal.completed_operations,
            failed_operations: journal.failed_operations,
            next_transition: journal
                .next_transition_operation()
                .map(RestoreApplyReportOperation::from_journal_operation),
            pending,
            failed,
            blocked,
        }
    }
}

///
/// RestoreApplyPendingSummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyPendingSummary {
    pub pending_operations: usize,
    pub pending_operation_available: bool,
    pub pending_sequence: Option<usize>,
    pub pending_operation: Option<RestoreApplyOperationKind>,
    pub pending_updated_at: Option<String>,
    pub pending_updated_at_known: bool,
}

impl RestoreApplyPendingSummary {
    /// Build a compact pending-operation summary from a restore apply journal.
    #[must_use]
    pub fn from_journal(journal: &RestoreApplyJournal) -> Self {
        let pending = journal
            .operations
            .iter()
            .filter(|operation| operation.state == RestoreApplyOperationState::Pending)
            .min_by_key(|operation| operation.sequence);
        let pending_updated_at = pending.and_then(|operation| operation.state_updated_at.clone());
        let pending_updated_at_known = pending_updated_at
            .as_deref()
            .is_some_and(known_state_update_marker);

        Self {
            pending_operations: journal.pending_operations,
            pending_operation_available: pending.is_some(),
            pending_sequence: pending.map(|operation| operation.sequence),
            pending_operation: pending.map(|operation| operation.operation.clone()),
            pending_updated_at,
            pending_updated_at_known,
        }
    }
}

// Return whether a journal update marker can be compared by automation.
fn known_state_update_marker(value: &str) -> bool {
    !value.trim().is_empty() && value != "unknown"
}

///
/// RestoreApplyProgressSummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyProgressSummary {
    pub operation_count: usize,
    pub completed_operations: usize,
    pub remaining_operations: usize,
    pub transitionable_operations: usize,
    pub attention_operations: usize,
    pub completion_basis_points: usize,
}

impl RestoreApplyProgressSummary {
    /// Build a compact progress summary from restore apply journal counters.
    #[must_use]
    pub const fn from_journal(journal: &RestoreApplyJournal) -> Self {
        let remaining_operations = journal
            .operation_count
            .saturating_sub(journal.completed_operations);
        let transitionable_operations = journal.ready_operations + journal.pending_operations;
        let attention_operations =
            journal.pending_operations + journal.blocked_operations + journal.failed_operations;
        let completion_basis_points =
            completion_basis_points(journal.completed_operations, journal.operation_count);

        Self {
            operation_count: journal.operation_count,
            completed_operations: journal.completed_operations,
            remaining_operations,
            transitionable_operations,
            attention_operations,
            completion_basis_points,
        }
    }
}

// Return completion as basis points so JSON stays deterministic and integer-only.
const fn completion_basis_points(completed_operations: usize, operation_count: usize) -> usize {
    if operation_count == 0 {
        return 0;
    }

    completed_operations.saturating_mul(10_000) / operation_count
}

///
/// RestoreApplyReportOutcome
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestoreApplyReportOutcome {
    Empty,
    Complete,
    Failed,
    Blocked,
    Pending,
    InProgress,
}

impl RestoreApplyReportOutcome {
    // Classify the journal into one high-level operator outcome.
    const fn from_journal(journal: &RestoreApplyJournal, complete: bool) -> Self {
        if journal.operation_count == 0 {
            return Self::Empty;
        }
        if complete {
            return Self::Complete;
        }
        if journal.failed_operations > 0 {
            return Self::Failed;
        }
        if !journal.ready || journal.blocked_operations > 0 {
            return Self::Blocked;
        }
        if journal.pending_operations > 0 {
            return Self::Pending;
        }
        Self::InProgress
    }

    // Return whether this outcome needs operator or automation attention.
    const fn attention_required(&self) -> bool {
        matches!(self, Self::Failed | Self::Blocked | Self::Pending)
    }
}

///
/// RestoreApplyReportOperation
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyReportOperation {
    pub sequence: usize,
    pub operation: RestoreApplyOperationKind,
    pub state: RestoreApplyOperationState,
    pub member_order: usize,
    pub role: String,
    pub source_canister: String,
    pub target_canister: String,
    pub state_updated_at: Option<String>,
    pub reasons: Vec<String>,
}

impl RestoreApplyReportOperation {
    // Build one compact report row from one journal operation.
    fn from_journal_operation(operation: &RestoreApplyJournalOperation) -> Self {
        Self {
            sequence: operation.sequence,
            operation: operation.operation.clone(),
            state: operation.state.clone(),
            member_order: operation.member_order,
            role: operation.role.clone(),
            source_canister: operation.source_canister.clone(),
            target_canister: operation.target_canister.clone(),
            state_updated_at: operation.state_updated_at.clone(),
            reasons: operation.blocking_reasons.clone(),
        }
    }
}

// Return compact report rows for operations in one state.
fn report_operations_with_state(
    journal: &RestoreApplyJournal,
    state: RestoreApplyOperationState,
) -> Vec<RestoreApplyReportOperation> {
    journal
        .operations
        .iter()
        .filter(|operation| operation.state == state)
        .map(RestoreApplyReportOperation::from_journal_operation)
        .collect()
}
