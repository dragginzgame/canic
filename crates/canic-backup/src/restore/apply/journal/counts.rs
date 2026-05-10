use super::{
    RestoreApplyDryRunOperation, RestoreApplyJournalError, RestoreApplyJournalOperation,
    RestoreApplyOperationKind, RestoreApplyOperationState, validate_apply_journal_count,
};
use serde::{Deserialize, Serialize};

///
/// RestoreApplyJournalStateCounts
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
    // Count operation states from concrete journal operation rows.
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

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyOperationKindCounts {
    pub snapshot_uploads: usize,
    pub snapshot_loads: usize,
    pub member_verifications: usize,
    pub fleet_verifications: usize,
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
            "operation_counts.fleet_verifications",
            self.fleet_verifications,
            expected.fleet_verifications,
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

    // Record one operation kind in the aggregate count object.
    const fn record(&mut self, operation: &RestoreApplyOperationKind) {
        match operation {
            RestoreApplyOperationKind::UploadSnapshot => self.snapshot_uploads += 1,
            RestoreApplyOperationKind::LoadSnapshot => self.snapshot_loads += 1,
            RestoreApplyOperationKind::VerifyMember => {
                self.member_verifications += 1;
                self.verification_operations += 1;
            }
            RestoreApplyOperationKind::VerifyFleet => {
                self.fleet_verifications += 1;
                self.verification_operations += 1;
            }
        }
    }
}
