//! Module: restore
//!
//! Responsibility: plan, preview, journal, and execute restore workflows.
//! Does not own: backup capture, backup persistence, or canister snapshot download.
//! Boundary: consumes backup manifests and artifacts to produce restore plans and runner state.

mod apply;
mod persistence;
mod plan;
mod runner;

pub(in crate::restore) use apply::RestoreApplyCommandOutputPair;
use apply::RestoreApplyJournalReport;
pub use apply::{
    RestoreApplyArtifactCheck, RestoreApplyArtifactValidation, RestoreApplyCommandConfig,
    RestoreApplyCommandOutput, RestoreApplyCommandPreview, RestoreApplyDryRun,
    RestoreApplyDryRunError, RestoreApplyDryRunOperation, RestoreApplyDryRunValidationError,
    RestoreApplyJournal, RestoreApplyJournalError, RestoreApplyJournalOperation,
    RestoreApplyOperationKind, RestoreApplyOperationKindCounts, RestoreApplyOperationReceipt,
    RestoreApplyOperationReceiptOutcome, RestoreApplyOperationState, RestoreApplyPendingSummary,
    RestoreApplyProgressSummary, RestoreApplyReportOperation, RestoreApplyReportOutcome,
    RestoreApplyRunnerCommand,
};
pub use persistence::{
    RestorePersistenceError, create_or_adopt_restore_apply_journal, create_or_adopt_restore_plan,
    write_restore_apply_journal, write_restore_plan,
};
pub use plan::{
    RestoreIdentitySummary, RestoreMapping, RestoreMappingEntry, RestoreOperationSummary,
    RestoreOrderingDependency, RestoreOrderingRelationship, RestoreOrderingSummary, RestorePlan,
    RestorePlanError, RestorePlanMember, RestorePlanner, RestoreReadinessSummary,
    RestoreSnapshotSummary, RestoreVerificationSummary,
};
pub use runner::{
    RESTORE_RUN_RECEIPT_COMPLETED, RESTORE_RUN_RECEIPT_FAILED,
    RESTORE_RUN_RECEIPT_RECOVERED_FAILED, RestoreRunExecutedOperation, RestoreRunOperationReceipt,
    RestoreRunReceiptSummary, RestoreRunResponse, RestoreRunnerCommandExecutor,
    RestoreRunnerCommandOutput, RestoreRunnerConfig, RestoreRunnerError, RestoreRunnerOutcome,
    parse_uploaded_snapshot_id, restore_run_dry_run, restore_run_execute_result_with_executor,
    restore_run_execute_with_executor, restore_run_retry_failed,
};
#[cfg(test)]
mod tests;
