//! Module: restore::apply::journal::counts
//!
//! Responsibility: count restore apply operations by state and operation kind.
//! Does not own: journal transitions, command rendering, or receipt validation.
//! Boundary: validates reported journal counters against concrete operation rows.

use super::{
    RestoreApplyDryRunOperation, RestoreApplyJournalError, RestoreApplyJournalOperation,
    RestoreApplyOperationKind, RestoreApplyOperationState, validate_apply_journal_count,
};

use serde::{Deserialize, Serialize};

///
/// RestoreApplyJournalStateCounts
///
/// Internal state counter projection for restore apply journal operations.
/// Owned by restore apply journaling and used during journal validation.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct RestoreApplyJournalStateCounts {
    pub(super) pending: usize,
    pub(super) ready: usize,
    pub(super) blocked: usize,
    pub(super) completed: usize,
    pub(super) failed: usize,
}

impl RestoreApplyJournalStateCounts {
    pub(super) fn from_operations(operations: &[RestoreApplyJournalOperation]) -> Self {
        let mut counts = Self::default();
        for operation in operations {
            match operation.state {
                RestoreApplyOperationState::Pending => counts.pending += 1,
                RestoreApplyOperationState::Ready => counts.ready += 1,
                RestoreApplyOperationState::Blocked => counts.blocked += 1,
                RestoreApplyOperationState::Completed => counts.completed += 1,
                RestoreApplyOperationState::Failed => counts.failed += 1,
            }
        }
        counts
    }
}

///
/// RestoreApplyOperationKindCounts
///
/// Serializable operation-kind counter projection for restore apply journals.
/// Owned by restore apply journaling and embedded in dry-run and journal outputs.
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreApplyOperationKindCounts {
    pub canister_stops: usize,
    pub canister_starts: usize,
    pub snapshot_uploads: usize,
    pub snapshot_loads: usize,
    pub member_verifications: usize,
    pub deployment_verifications: usize,
    pub verification_operations: usize,
}

impl RestoreApplyOperationKindCounts {
    /// Count restore apply journal operations by runner operation kind.
    #[must_use]
    pub fn from_operations(operations: &[RestoreApplyJournalOperation]) -> Self {
        let mut counts = Self::default();
        for operation in operations {
            counts.record(&operation.operation);
        }
        counts
    }

    /// Validate this count object against concrete operations.
    pub fn validate_matches(&self, expected: &Self) -> Result<(), RestoreApplyJournalError> {
        validate_apply_journal_count(
            "operation_counts.canister_stops",
            self.canister_stops,
            expected.canister_stops,
        )?;
        validate_apply_journal_count(
            "operation_counts.canister_starts",
            self.canister_starts,
            expected.canister_starts,
        )?;
        validate_apply_journal_count(
            "operation_counts.snapshot_uploads",
            self.snapshot_uploads,
            expected.snapshot_uploads,
        )?;
        validate_apply_journal_count(
            "operation_counts.snapshot_loads",
            self.snapshot_loads,
            expected.snapshot_loads,
        )?;
        validate_apply_journal_count(
            "operation_counts.member_verifications",
            self.member_verifications,
            expected.member_verifications,
        )?;
        validate_apply_journal_count(
            "operation_counts.deployment_verifications",
            self.deployment_verifications,
            expected.deployment_verifications,
        )?;
        validate_apply_journal_count(
            "operation_counts.verification_operations",
            self.verification_operations,
            expected.verification_operations,
        )
    }

    /// Count restore apply dry-run operations by runner operation kind.
    #[must_use]
    pub fn from_dry_run_operations(operations: &[RestoreApplyDryRunOperation]) -> Self {
        let mut counts = Self::default();
        for operation in operations {
            counts.record(&operation.operation);
        }
        counts
    }

    const fn record(&mut self, operation: &RestoreApplyOperationKind) {
        match operation {
            RestoreApplyOperationKind::StopCanister => self.canister_stops += 1,
            RestoreApplyOperationKind::StartCanister => self.canister_starts += 1,
            RestoreApplyOperationKind::UploadSnapshot => self.snapshot_uploads += 1,
            RestoreApplyOperationKind::LoadSnapshot => self.snapshot_loads += 1,
            RestoreApplyOperationKind::VerifyMember => {
                self.member_verifications += 1;
                self.verification_operations += 1;
            }
            RestoreApplyOperationKind::VerifyDeployment => {
                self.deployment_verifications += 1;
                self.verification_operations += 1;
            }
        }
    }
}
