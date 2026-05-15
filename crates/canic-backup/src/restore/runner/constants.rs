pub(super) const RESTORE_RUN_MODE_DRY_RUN: &str = "dry-run";
pub(super) const RESTORE_RUN_MODE_EXECUTE: &str = "execute";
pub(super) const RESTORE_RUN_MODE_RETRY_FAILED: &str = "retry-failed";
pub(super) const RESTORE_RUN_MODE_UNCLAIM_PENDING: &str = "unclaim-pending";

pub(super) const RESTORE_RUN_STOPPED_BLOCKED: &str = "blocked";
pub(super) const RESTORE_RUN_STOPPED_COMMAND_FAILED: &str = "command-failed";
pub(super) const RESTORE_RUN_STOPPED_COMPLETE: &str = "complete";
pub(super) const RESTORE_RUN_STOPPED_MAX_STEPS: &str = "max-steps-reached";
pub(super) const RESTORE_RUN_STOPPED_PENDING: &str = "pending";
pub(super) const RESTORE_RUN_STOPPED_PREVIEW: &str = "preview";
pub(super) const RESTORE_RUN_STOPPED_READY: &str = "ready";
pub(super) const RESTORE_RUN_STOPPED_RECOVERED_FAILED: &str = "recovered-failed";
pub(super) const RESTORE_RUN_STOPPED_RECOVERED_PENDING: &str = "recovered-pending";

pub(super) const RESTORE_RUN_ACTION_DONE: &str = "done";
pub(super) const RESTORE_RUN_ACTION_FIX_BLOCKED: &str = "fix-blocked-journal";
pub(super) const RESTORE_RUN_ACTION_RETRY_FAILED: &str = "retry-failed";
pub(super) const RESTORE_RUN_ACTION_RERUN: &str = "rerun";
pub(super) const RESTORE_RUN_ACTION_UNCLAIM_PENDING: &str = "unclaim-pending";

pub const RESTORE_RUN_RECEIPT_COMPLETED: &str = "command-completed";
pub const RESTORE_RUN_RECEIPT_FAILED: &str = "command-failed";
pub const RESTORE_RUN_RECEIPT_RECOVERED_FAILED: &str = "failed-recovered";
pub const RESTORE_RUN_RECEIPT_RECOVERED_PENDING: &str = "pending-recovered";

pub(super) const RESTORE_RUN_EXECUTED_COMPLETED: &str = "completed";
pub(super) const RESTORE_RUN_EXECUTED_FAILED: &str = "failed";
pub(super) const RESTORE_RUN_RECEIPT_STATE_READY: &str = "ready";
pub(super) const RESTORE_RUN_COMMAND_EXIT_PREFIX: &str = "runner-command-exit";
pub(super) const RESTORE_RUN_MISSING_UPLOADED_SNAPSHOT_ID: &str = "missing-uploaded-snapshot-id";
pub(super) const RESTORE_RUN_STOPPED_PRECONDITION_FAILED: &str = "stopped-precondition-failed";
pub(super) const RESTORE_RUN_RESPONSE_VERSION: u16 = 1;
pub(super) const RESTORE_RUN_OUTPUT_RECEIPT_LIMIT: usize = 64 * 1024;
