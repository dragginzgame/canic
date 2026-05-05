use canic_backup::{
    manifest::FleetBackupManifest,
    persistence::{BackupLayout, PersistenceError},
    restore::{
        RestoreApplyCommandConfig, RestoreApplyCommandPreview, RestoreApplyDryRun,
        RestoreApplyDryRunError, RestoreApplyJournal, RestoreApplyJournalError,
        RestoreApplyJournalOperation, RestoreApplyJournalReport, RestoreApplyJournalStatus,
        RestoreApplyNextOperation, RestoreApplyOperationKind, RestoreApplyOperationKindCounts,
        RestoreApplyOperationState, RestoreApplyPendingSummary, RestoreApplyProgressSummary,
        RestoreApplyReportOperation, RestoreApplyReportOutcome, RestoreApplyRunnerCommand,
        RestoreMapping, RestorePlan, RestorePlanError, RestorePlanner, RestoreStatus,
    },
};
use serde::Serialize;
use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    path::PathBuf,
    process::Command,
};
use thiserror::Error as ThisError;

///
/// RestoreCommandError
///

#[derive(Debug, ThisError)]
pub enum RestoreCommandError {
    #[error("{0}")]
    Usage(&'static str),

    #[error("missing required option {0}")]
    MissingOption(&'static str),

    #[error("use either --manifest or --backup-dir, not both")]
    ConflictingManifestSources,

    #[error("--require-verified requires --backup-dir")]
    RequireVerifiedNeedsBackupDir,

    #[error("restore apply currently requires --dry-run")]
    ApplyRequiresDryRun,

    #[error("restore run requires --dry-run, --execute, or --unclaim-pending")]
    RestoreRunRequiresMode,

    #[error("use only one restore run mode: --dry-run, --execute, or --unclaim-pending")]
    RestoreRunConflictingModes,

    #[error("restore run command failed for operation {sequence}: status={status}")]
    RestoreRunCommandFailed { sequence: usize, status: String },

    #[error("restore run for backup {backup_id} used run_mode={actual}, expected {expected}")]
    RestoreRunModeMismatch {
        backup_id: String,
        expected: String,
        actual: String,
    },

    #[error(
        "restore run for backup {backup_id} stopped for {actual}, expected stopped_reason={expected}"
    )]
    RestoreRunStoppedReasonMismatch {
        backup_id: String,
        expected: String,
        actual: String,
    },

    #[error(
        "restore run for backup {backup_id} reported next_action={actual}, expected {expected}"
    )]
    RestoreRunNextActionMismatch {
        backup_id: String,
        expected: String,
        actual: String,
    },

    #[error("restore run for backup {backup_id} executed {actual} operations, expected {expected}")]
    RestoreRunExecutedCountMismatch {
        backup_id: String,
        expected: usize,
        actual: usize,
    },

    #[error("restore run for backup {backup_id} wrote {actual} receipts, expected {expected}")]
    RestoreRunReceiptCountMismatch {
        backup_id: String,
        expected: usize,
        actual: usize,
    },

    #[error(
        "restore run for backup {backup_id} wrote {actual} {receipt_kind} receipts, expected {expected}"
    )]
    RestoreRunReceiptKindCountMismatch {
        backup_id: String,
        receipt_kind: &'static str,
        expected: usize,
        actual: usize,
    },

    #[error("restore plan for backup {backup_id} is not restore-ready: reasons={reasons:?}")]
    RestoreNotReady {
        backup_id: String,
        reasons: Vec<String>,
    },

    #[error(
        "restore apply journal for backup {backup_id} has pending operations: pending={pending_operations}, next={next_transition_sequence:?}"
    )]
    RestoreApplyPending {
        backup_id: String,
        pending_operations: usize,
        next_transition_sequence: Option<usize>,
    },

    #[error(
        "restore apply journal for backup {backup_id} has stale or untracked pending work before {cutoff_updated_at}: pending_sequence={pending_sequence:?}, pending_updated_at={pending_updated_at:?}"
    )]
    RestoreApplyPendingStale {
        backup_id: String,
        cutoff_updated_at: String,
        pending_sequence: Option<usize>,
        pending_updated_at: Option<String>,
    },

    #[error(
        "restore apply journal for backup {backup_id} is incomplete: completed={completed_operations}, total={operation_count}"
    )]
    RestoreApplyIncomplete {
        backup_id: String,
        completed_operations: usize,
        operation_count: usize,
    },

    #[error(
        "restore apply journal for backup {backup_id} has failed operations: failed={failed_operations}"
    )]
    RestoreApplyFailed {
        backup_id: String,
        failed_operations: usize,
    },

    #[error("restore apply journal for backup {backup_id} is not ready: reasons={reasons:?}")]
    RestoreApplyNotReady {
        backup_id: String,
        reasons: Vec<String>,
    },

    #[error("restore apply report for backup {backup_id} requires attention: outcome={outcome:?}")]
    RestoreApplyReportNeedsAttention {
        backup_id: String,
        outcome: canic_backup::restore::RestoreApplyReportOutcome,
    },

    #[error(
        "restore apply progress for backup {backup_id} has unexpected {field}: expected={expected}, actual={actual}"
    )]
    RestoreApplyProgressMismatch {
        backup_id: String,
        field: &'static str,
        expected: usize,
        actual: usize,
    },

    #[error(
        "restore apply journal for backup {backup_id} has no executable command: operation_available={operation_available}, complete={complete}, blocked_reasons={blocked_reasons:?}"
    )]
    RestoreApplyCommandUnavailable {
        backup_id: String,
        operation_available: bool,
        complete: bool,
        blocked_reasons: Vec<String>,
    },

    #[error(
        "restore apply journal operation {sequence} must be pending before apply-mark: state={state:?}"
    )]
    RestoreApplyMarkRequiresPending {
        sequence: usize,
        state: RestoreApplyOperationState,
    },

    #[error(
        "restore apply journal next operation changed before claim: expected={expected}, actual={actual:?}"
    )]
    RestoreApplyClaimSequenceMismatch {
        expected: usize,
        actual: Option<usize>,
    },

    #[error(
        "restore apply journal pending operation changed before unclaim: expected={expected}, actual={actual:?}"
    )]
    RestoreApplyUnclaimSequenceMismatch {
        expected: usize,
        actual: Option<usize>,
    },

    #[error("unknown option {0}")]
    UnknownOption(String),

    #[error("option {0} requires a value")]
    MissingValue(&'static str),

    #[error("option --sequence requires a non-negative integer value")]
    InvalidSequence,

    #[error("unsupported apply-mark state {0}; use completed or failed")]
    InvalidApplyMarkState(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),

    #[error(transparent)]
    RestorePlan(#[from] RestorePlanError),

    #[error(transparent)]
    RestoreApplyDryRun(#[from] RestoreApplyDryRunError),

    #[error(transparent)]
    RestoreApplyJournal(#[from] RestoreApplyJournalError),
}

///
/// RestorePlanOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestorePlanOptions {
    pub manifest: Option<PathBuf>,
    pub backup_dir: Option<PathBuf>,
    pub mapping: Option<PathBuf>,
    pub out: Option<PathBuf>,
    pub require_verified: bool,
    pub require_restore_ready: bool,
}

impl RestorePlanOptions {
    /// Parse restore planning options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut manifest = None;
        let mut backup_dir = None;
        let mut mapping = None;
        let mut out = None;
        let mut require_verified = false;
        let mut require_restore_ready = false;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| RestoreCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--manifest" => {
                    manifest = Some(PathBuf::from(next_value(&mut args, "--manifest")?));
                }
                "--backup-dir" => {
                    backup_dir = Some(PathBuf::from(next_value(&mut args, "--backup-dir")?));
                }
                "--mapping" => mapping = Some(PathBuf::from(next_value(&mut args, "--mapping")?)),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--require-verified" => require_verified = true,
                "--require-restore-ready" => require_restore_ready = true,
                "--help" | "-h" => return Err(RestoreCommandError::Usage(usage())),
                _ => return Err(RestoreCommandError::UnknownOption(arg)),
            }
        }

        if manifest.is_some() && backup_dir.is_some() {
            return Err(RestoreCommandError::ConflictingManifestSources);
        }

        if manifest.is_none() && backup_dir.is_none() {
            return Err(RestoreCommandError::MissingOption(
                "--manifest or --backup-dir",
            ));
        }

        if require_verified && backup_dir.is_none() {
            return Err(RestoreCommandError::RequireVerifiedNeedsBackupDir);
        }

        Ok(Self {
            manifest,
            backup_dir,
            mapping,
            out,
            require_verified,
            require_restore_ready,
        })
    }
}

///
/// RestoreStatusOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreStatusOptions {
    pub plan: PathBuf,
    pub out: Option<PathBuf>,
}

impl RestoreStatusOptions {
    /// Parse restore status options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut plan = None;
        let mut out = None;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| RestoreCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--plan" => plan = Some(PathBuf::from(next_value(&mut args, "--plan")?)),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--help" | "-h" => return Err(RestoreCommandError::Usage(usage())),
                _ => return Err(RestoreCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self {
            plan: plan.ok_or(RestoreCommandError::MissingOption("--plan"))?,
            out,
        })
    }
}

///
/// RestoreApplyOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreApplyOptions {
    pub plan: PathBuf,
    pub status: Option<PathBuf>,
    pub backup_dir: Option<PathBuf>,
    pub out: Option<PathBuf>,
    pub journal_out: Option<PathBuf>,
    pub dry_run: bool,
}

impl RestoreApplyOptions {
    /// Parse restore apply options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut plan = None;
        let mut status = None;
        let mut backup_dir = None;
        let mut out = None;
        let mut journal_out = None;
        let mut dry_run = false;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| RestoreCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--plan" => plan = Some(PathBuf::from(next_value(&mut args, "--plan")?)),
                "--status" => status = Some(PathBuf::from(next_value(&mut args, "--status")?)),
                "--backup-dir" => {
                    backup_dir = Some(PathBuf::from(next_value(&mut args, "--backup-dir")?));
                }
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--journal-out" => {
                    journal_out = Some(PathBuf::from(next_value(&mut args, "--journal-out")?));
                }
                "--dry-run" => dry_run = true,
                "--help" | "-h" => return Err(RestoreCommandError::Usage(usage())),
                _ => return Err(RestoreCommandError::UnknownOption(arg)),
            }
        }

        if !dry_run {
            return Err(RestoreCommandError::ApplyRequiresDryRun);
        }

        Ok(Self {
            plan: plan.ok_or(RestoreCommandError::MissingOption("--plan"))?,
            status,
            backup_dir,
            out,
            journal_out,
            dry_run,
        })
    }
}

///
/// RestoreApplyStatusOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "CLI status options mirror independent fail-closed guard flags"
)]
pub struct RestoreApplyStatusOptions {
    pub journal: PathBuf,
    pub require_ready: bool,
    pub require_no_pending: bool,
    pub require_no_failed: bool,
    pub require_complete: bool,
    pub require_remaining_count: Option<usize>,
    pub require_attention_count: Option<usize>,
    pub require_completion_basis_points: Option<usize>,
    pub require_no_pending_before: Option<String>,
    pub out: Option<PathBuf>,
}

impl RestoreApplyStatusOptions {
    /// Parse restore apply-status options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut journal = None;
        let mut require_ready = false;
        let mut require_no_pending = false;
        let mut require_no_failed = false;
        let mut require_complete = false;
        let mut require_remaining_count = None;
        let mut require_attention_count = None;
        let mut require_completion_basis_points = None;
        let mut require_no_pending_before = None;
        let mut out = None;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| RestoreCommandError::Usage(usage()))?;
            if parse_progress_requirement_option(
                arg.as_str(),
                &mut args,
                &mut require_remaining_count,
                &mut require_attention_count,
                &mut require_completion_basis_points,
            )? {
                continue;
            }
            if parse_pending_requirement_option(
                arg.as_str(),
                &mut args,
                &mut require_no_pending_before,
            )? {
                continue;
            }
            match arg.as_str() {
                "--journal" => journal = Some(PathBuf::from(next_value(&mut args, "--journal")?)),
                "--require-ready" => require_ready = true,
                "--require-no-pending" => require_no_pending = true,
                "--require-no-failed" => require_no_failed = true,
                "--require-complete" => require_complete = true,
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--help" | "-h" => return Err(RestoreCommandError::Usage(usage())),
                _ => return Err(RestoreCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self {
            journal: journal.ok_or(RestoreCommandError::MissingOption("--journal"))?,
            require_ready,
            require_no_pending,
            require_no_failed,
            require_complete,
            require_remaining_count,
            require_attention_count,
            require_completion_basis_points,
            require_no_pending_before,
            out,
        })
    }
}

///
/// RestoreApplyReportOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreApplyReportOptions {
    pub journal: PathBuf,
    pub require_no_attention: bool,
    pub require_remaining_count: Option<usize>,
    pub require_attention_count: Option<usize>,
    pub require_completion_basis_points: Option<usize>,
    pub require_no_pending_before: Option<String>,
    pub out: Option<PathBuf>,
}

impl RestoreApplyReportOptions {
    /// Parse restore apply-report options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut journal = None;
        let mut require_no_attention = false;
        let mut require_remaining_count = None;
        let mut require_attention_count = None;
        let mut require_completion_basis_points = None;
        let mut require_no_pending_before = None;
        let mut out = None;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| RestoreCommandError::Usage(usage()))?;
            if parse_progress_requirement_option(
                arg.as_str(),
                &mut args,
                &mut require_remaining_count,
                &mut require_attention_count,
                &mut require_completion_basis_points,
            )? {
                continue;
            }
            if parse_pending_requirement_option(
                arg.as_str(),
                &mut args,
                &mut require_no_pending_before,
            )? {
                continue;
            }
            match arg.as_str() {
                "--journal" => journal = Some(PathBuf::from(next_value(&mut args, "--journal")?)),
                "--require-no-attention" => require_no_attention = true,
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--help" | "-h" => return Err(RestoreCommandError::Usage(usage())),
                _ => return Err(RestoreCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self {
            journal: journal.ok_or(RestoreCommandError::MissingOption("--journal"))?,
            require_no_attention,
            require_remaining_count,
            require_attention_count,
            require_completion_basis_points,
            require_no_pending_before,
            out,
        })
    }
}

///
/// RestoreRunOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "CLI runner options mirror independent mode and fail-closed guard flags"
)]
pub struct RestoreRunOptions {
    pub journal: PathBuf,
    pub dfx: String,
    pub network: Option<String>,
    pub out: Option<PathBuf>,
    pub dry_run: bool,
    pub execute: bool,
    pub unclaim_pending: bool,
    pub max_steps: Option<usize>,
    pub require_complete: bool,
    pub require_no_attention: bool,
    pub require_run_mode: Option<String>,
    pub require_stopped_reason: Option<String>,
    pub require_next_action: Option<String>,
    pub require_executed_count: Option<usize>,
    pub require_receipt_count: Option<usize>,
    pub require_completed_receipt_count: Option<usize>,
    pub require_failed_receipt_count: Option<usize>,
    pub require_recovered_receipt_count: Option<usize>,
    pub require_remaining_count: Option<usize>,
    pub require_attention_count: Option<usize>,
    pub require_completion_basis_points: Option<usize>,
    pub require_no_pending_before: Option<String>,
}

impl RestoreRunOptions {
    /// Parse restore run options from CLI arguments.
    #[expect(
        clippy::too_many_lines,
        reason = "Restore runner options intentionally parse a broad flat CLI surface"
    )]
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut journal = None;
        let mut dfx = "dfx".to_string();
        let mut network = None;
        let mut out = None;
        let mut dry_run = false;
        let mut execute = false;
        let mut unclaim_pending = false;
        let mut max_steps = None;
        let mut require_complete = false;
        let mut require_no_attention = false;
        let mut require_run_mode = None;
        let mut require_stopped_reason = None;
        let mut require_next_action = None;
        let mut require_executed_count = None;
        let mut require_receipt_count = None;
        let mut require_completed_receipt_count = None;
        let mut require_failed_receipt_count = None;
        let mut require_recovered_receipt_count = None;
        let mut require_remaining_count = None;
        let mut require_attention_count = None;
        let mut require_completion_basis_points = None;
        let mut require_no_pending_before = None;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| RestoreCommandError::Usage(usage()))?;
            if parse_progress_requirement_option(
                arg.as_str(),
                &mut args,
                &mut require_remaining_count,
                &mut require_attention_count,
                &mut require_completion_basis_points,
            )? {
                continue;
            }
            if parse_pending_requirement_option(
                arg.as_str(),
                &mut args,
                &mut require_no_pending_before,
            )? {
                continue;
            }
            if parse_run_count_requirement_option(
                arg.as_str(),
                &mut args,
                &mut require_executed_count,
                &mut require_receipt_count,
            )? {
                continue;
            }
            if parse_run_receipt_kind_requirement_option(
                arg.as_str(),
                &mut args,
                &mut require_completed_receipt_count,
                &mut require_failed_receipt_count,
                &mut require_recovered_receipt_count,
            )? {
                continue;
            }

            match arg.as_str() {
                "--journal" => journal = Some(PathBuf::from(next_value(&mut args, "--journal")?)),
                "--dfx" => dfx = next_value(&mut args, "--dfx")?,
                "--network" => network = Some(next_value(&mut args, "--network")?),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--dry-run" => dry_run = true,
                "--execute" => execute = true,
                "--unclaim-pending" => unclaim_pending = true,
                "--max-steps" => {
                    max_steps = Some(parse_sequence(next_value(&mut args, "--max-steps")?)?);
                }
                "--require-complete" => require_complete = true,
                "--require-no-attention" => require_no_attention = true,
                "--require-run-mode" => {
                    require_run_mode = Some(next_value(&mut args, "--require-run-mode")?);
                }
                "--require-stopped-reason" => {
                    require_stopped_reason =
                        Some(next_value(&mut args, "--require-stopped-reason")?);
                }
                "--require-next-action" => {
                    require_next_action = Some(next_value(&mut args, "--require-next-action")?);
                }
                "--help" | "-h" => return Err(RestoreCommandError::Usage(usage())),
                _ => return Err(RestoreCommandError::UnknownOption(arg)),
            }
        }

        validate_restore_run_mode_selection(dry_run, execute, unclaim_pending)?;

        Ok(Self {
            journal: journal.ok_or(RestoreCommandError::MissingOption("--journal"))?,
            dfx,
            network,
            out,
            dry_run,
            execute,
            unclaim_pending,
            max_steps,
            require_complete,
            require_no_attention,
            require_run_mode,
            require_stopped_reason,
            require_next_action,
            require_executed_count,
            require_receipt_count,
            require_completed_receipt_count,
            require_failed_receipt_count,
            require_recovered_receipt_count,
            require_remaining_count,
            require_attention_count,
            require_completion_basis_points,
            require_no_pending_before,
        })
    }
}

// Validate that restore run received exactly one execution mode.
fn validate_restore_run_mode_selection(
    dry_run: bool,
    execute: bool,
    unclaim_pending: bool,
) -> Result<(), RestoreCommandError> {
    let mode_count = [dry_run, execute, unclaim_pending]
        .into_iter()
        .filter(|enabled| *enabled)
        .count();
    if mode_count > 1 {
        return Err(RestoreCommandError::RestoreRunConflictingModes);
    }

    if mode_count == 0 {
        return Err(RestoreCommandError::RestoreRunRequiresMode);
    }

    Ok(())
}

///
/// RestoreRunResult
///

struct RestoreRunResult {
    response: RestoreRunResponse,
    error: Option<RestoreCommandError>,
}

impl RestoreRunResult {
    // Build a successful runner response with no deferred error.
    const fn ok(response: RestoreRunResponse) -> Self {
        Self {
            response,
            error: None,
        }
    }
}

const RESTORE_RUN_MODE_DRY_RUN: &str = "dry-run";
const RESTORE_RUN_MODE_EXECUTE: &str = "execute";
const RESTORE_RUN_MODE_UNCLAIM_PENDING: &str = "unclaim-pending";

const RESTORE_RUN_STOPPED_BLOCKED: &str = "blocked";
const RESTORE_RUN_STOPPED_COMMAND_FAILED: &str = "command-failed";
const RESTORE_RUN_STOPPED_COMPLETE: &str = "complete";
const RESTORE_RUN_STOPPED_MAX_STEPS: &str = "max-steps-reached";
const RESTORE_RUN_STOPPED_PENDING: &str = "pending";
const RESTORE_RUN_STOPPED_PREVIEW: &str = "preview";
const RESTORE_RUN_STOPPED_READY: &str = "ready";
const RESTORE_RUN_STOPPED_RECOVERED_PENDING: &str = "recovered-pending";

const RESTORE_RUN_ACTION_DONE: &str = "done";
const RESTORE_RUN_ACTION_FIX_BLOCKED: &str = "fix-blocked-journal";
const RESTORE_RUN_ACTION_INSPECT_FAILED: &str = "inspect-failed-operation";
const RESTORE_RUN_ACTION_RERUN: &str = "rerun";
const RESTORE_RUN_ACTION_UNCLAIM_PENDING: &str = "unclaim-pending";

const RESTORE_RUN_EXECUTED_COMPLETED: &str = "completed";
const RESTORE_RUN_EXECUTED_FAILED: &str = "failed";
const RESTORE_RUN_RECEIPT_COMPLETED: &str = "command-completed";
const RESTORE_RUN_RECEIPT_FAILED: &str = "command-failed";
const RESTORE_RUN_RECEIPT_RECOVERED_PENDING: &str = "pending-recovered";
const RESTORE_RUN_RECEIPT_STATE_READY: &str = "ready";
const RESTORE_RUN_COMMAND_EXIT_PREFIX: &str = "runner-command-exit";
const RESTORE_RUN_RESPONSE_VERSION: u16 = 1;

///
/// RestoreRunResponse
///

#[derive(Clone, Debug, Serialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Runner response exposes stable JSON status flags for operators and CI"
)]
pub struct RestoreRunResponse {
    run_version: u16,
    backup_id: String,
    run_mode: &'static str,
    dry_run: bool,
    execute: bool,
    unclaim_pending: bool,
    stopped_reason: &'static str,
    next_action: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_steps_reached: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    executed_operations: Vec<RestoreRunExecutedOperation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    operation_receipts: Vec<RestoreRunOperationReceipt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_receipt_count: Option<usize>,
    operation_receipt_summary: RestoreRunReceiptSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    executed_operation_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recovered_operation: Option<RestoreApplyJournalOperation>,
    ready: bool,
    complete: bool,
    attention_required: bool,
    outcome: RestoreApplyReportOutcome,
    operation_count: usize,
    operation_counts: RestoreApplyOperationKindCounts,
    operation_counts_supplied: bool,
    progress: RestoreApplyProgressSummary,
    pending_summary: RestoreApplyPendingSummary,
    pending_operations: usize,
    ready_operations: usize,
    blocked_operations: usize,
    completed_operations: usize,
    failed_operations: usize,
    blocked_reasons: Vec<String>,
    next_transition: Option<RestoreApplyReportOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_available: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command_available: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<RestoreApplyRunnerCommand>,
}

impl RestoreRunResponse {
    // Build the shared native runner response fields from an apply journal report.
    fn from_report(
        backup_id: String,
        report: RestoreApplyJournalReport,
        mode: RestoreRunResponseMode,
    ) -> Self {
        Self {
            run_version: RESTORE_RUN_RESPONSE_VERSION,
            backup_id,
            run_mode: mode.run_mode,
            dry_run: mode.dry_run,
            execute: mode.execute,
            unclaim_pending: mode.unclaim_pending,
            stopped_reason: mode.stopped_reason,
            next_action: mode.next_action,
            max_steps_reached: None,
            executed_operations: Vec::new(),
            operation_receipts: Vec::new(),
            operation_receipt_count: Some(0),
            operation_receipt_summary: RestoreRunReceiptSummary::default(),
            executed_operation_count: None,
            recovered_operation: None,
            ready: report.ready,
            complete: report.complete,
            attention_required: report.attention_required,
            outcome: report.outcome,
            operation_count: report.operation_count,
            operation_counts: report.operation_counts,
            operation_counts_supplied: report.operation_counts_supplied,
            progress: report.progress,
            pending_summary: report.pending_summary,
            pending_operations: report.pending_operations,
            ready_operations: report.ready_operations,
            blocked_operations: report.blocked_operations,
            completed_operations: report.completed_operations,
            failed_operations: report.failed_operations,
            blocked_reasons: report.blocked_reasons,
            next_transition: report.next_transition,
            operation_available: None,
            command_available: None,
            command: None,
        }
    }

    // Replace the detailed receipt stream and refresh the compact counters.
    fn set_operation_receipts(&mut self, receipts: Vec<RestoreRunOperationReceipt>) {
        self.operation_receipt_summary = RestoreRunReceiptSummary::from_receipts(&receipts);
        self.operation_receipt_count = Some(receipts.len());
        self.operation_receipts = receipts;
    }
}

///
/// RestoreRunReceiptSummary
///

#[derive(Clone, Debug, Default, Serialize)]
struct RestoreRunReceiptSummary {
    total_receipts: usize,
    command_completed: usize,
    command_failed: usize,
    pending_recovered: usize,
}

impl RestoreRunReceiptSummary {
    // Count restore runner receipt classes for script-friendly summaries.
    fn from_receipts(receipts: &[RestoreRunOperationReceipt]) -> Self {
        let mut summary = Self {
            total_receipts: receipts.len(),
            ..Self::default()
        };

        for receipt in receipts {
            match receipt.event {
                RESTORE_RUN_RECEIPT_COMPLETED => summary.command_completed += 1,
                RESTORE_RUN_RECEIPT_FAILED => summary.command_failed += 1,
                RESTORE_RUN_RECEIPT_RECOVERED_PENDING => summary.pending_recovered += 1,
                _ => {}
            }
        }

        summary
    }
}

///
/// RestoreRunOperationReceipt
///

#[derive(Clone, Debug, Serialize)]
struct RestoreRunOperationReceipt {
    event: &'static str,
    sequence: usize,
    operation: RestoreApplyOperationKind,
    target_canister: String,
    state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<RestoreApplyRunnerCommand>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

impl RestoreRunOperationReceipt {
    // Build a receipt for a completed runner command.
    fn completed(
        operation: RestoreApplyJournalOperation,
        command: RestoreApplyRunnerCommand,
        status: String,
        updated_at: Option<String>,
    ) -> Self {
        Self::from_operation(
            RESTORE_RUN_RECEIPT_COMPLETED,
            operation,
            RESTORE_RUN_EXECUTED_COMPLETED,
            updated_at,
            Some(command),
            Some(status),
        )
    }

    // Build a receipt for a failed runner command.
    fn failed(
        operation: RestoreApplyJournalOperation,
        command: RestoreApplyRunnerCommand,
        status: String,
        updated_at: Option<String>,
    ) -> Self {
        Self::from_operation(
            RESTORE_RUN_RECEIPT_FAILED,
            operation,
            RESTORE_RUN_EXECUTED_FAILED,
            updated_at,
            Some(command),
            Some(status),
        )
    }

    // Build a receipt for a recovered pending operation.
    fn recovered_pending(
        operation: RestoreApplyJournalOperation,
        updated_at: Option<String>,
    ) -> Self {
        Self::from_operation(
            RESTORE_RUN_RECEIPT_RECOVERED_PENDING,
            operation,
            RESTORE_RUN_RECEIPT_STATE_READY,
            updated_at,
            None,
            None,
        )
    }

    // Map one operation event into a compact audit receipt.
    fn from_operation(
        event: &'static str,
        operation: RestoreApplyJournalOperation,
        state: &'static str,
        updated_at: Option<String>,
        command: Option<RestoreApplyRunnerCommand>,
        status: Option<String>,
    ) -> Self {
        Self {
            event,
            sequence: operation.sequence,
            operation: operation.operation,
            target_canister: operation.target_canister,
            state,
            updated_at,
            command,
            status,
        }
    }
}

///
/// RestoreRunExecutedOperation
///

#[derive(Clone, Debug, Serialize)]
struct RestoreRunExecutedOperation {
    sequence: usize,
    operation: RestoreApplyOperationKind,
    target_canister: String,
    command: RestoreApplyRunnerCommand,
    status: String,
    state: &'static str,
}

impl RestoreRunExecutedOperation {
    // Build a completed executed-operation summary row from a runner operation.
    fn completed(
        operation: RestoreApplyJournalOperation,
        command: RestoreApplyRunnerCommand,
        status: String,
    ) -> Self {
        Self::from_operation(operation, command, status, RESTORE_RUN_EXECUTED_COMPLETED)
    }

    // Build a failed executed-operation summary row from a runner operation.
    fn failed(
        operation: RestoreApplyJournalOperation,
        command: RestoreApplyRunnerCommand,
        status: String,
    ) -> Self {
        Self::from_operation(operation, command, status, RESTORE_RUN_EXECUTED_FAILED)
    }

    // Map a journal operation into the compact runner execution row.
    fn from_operation(
        operation: RestoreApplyJournalOperation,
        command: RestoreApplyRunnerCommand,
        status: String,
        state: &'static str,
    ) -> Self {
        Self {
            sequence: operation.sequence,
            operation: operation.operation,
            target_canister: operation.target_canister,
            command,
            status,
            state,
        }
    }
}

///
/// RestoreRunResponseMode
///

struct RestoreRunResponseMode {
    run_mode: &'static str,
    dry_run: bool,
    execute: bool,
    unclaim_pending: bool,
    stopped_reason: &'static str,
    next_action: &'static str,
}

impl RestoreRunResponseMode {
    // Build a response mode from the stable JSON mode flags and action labels.
    const fn new(
        run_mode: &'static str,
        dry_run: bool,
        execute: bool,
        unclaim_pending: bool,
        stopped_reason: &'static str,
        next_action: &'static str,
    ) -> Self {
        Self {
            run_mode,
            dry_run,
            execute,
            unclaim_pending,
            stopped_reason,
            next_action,
        }
    }

    // Build a dry-run response mode with a computed stop reason and action.
    const fn dry_run(stopped_reason: &'static str, next_action: &'static str) -> Self {
        Self::new(
            RESTORE_RUN_MODE_DRY_RUN,
            true,
            false,
            false,
            stopped_reason,
            next_action,
        )
    }

    // Build an execute response mode with a computed stop reason and action.
    const fn execute(stopped_reason: &'static str, next_action: &'static str) -> Self {
        Self::new(
            RESTORE_RUN_MODE_EXECUTE,
            false,
            true,
            false,
            stopped_reason,
            next_action,
        )
    }

    // Build the pending-operation recovery response mode.
    const fn unclaim_pending(next_action: &'static str) -> Self {
        Self::new(
            RESTORE_RUN_MODE_UNCLAIM_PENDING,
            false,
            false,
            true,
            RESTORE_RUN_STOPPED_RECOVERED_PENDING,
            next_action,
        )
    }
}

///
/// RestoreApplyNextOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreApplyNextOptions {
    pub journal: PathBuf,
    pub out: Option<PathBuf>,
}

impl RestoreApplyNextOptions {
    /// Parse restore apply-next options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut journal = None;
        let mut out = None;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| RestoreCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--journal" => journal = Some(PathBuf::from(next_value(&mut args, "--journal")?)),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--help" | "-h" => return Err(RestoreCommandError::Usage(usage())),
                _ => return Err(RestoreCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self {
            journal: journal.ok_or(RestoreCommandError::MissingOption("--journal"))?,
            out,
        })
    }
}

///
/// RestoreApplyCommandOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreApplyCommandOptions {
    pub journal: PathBuf,
    pub dfx: String,
    pub network: Option<String>,
    pub out: Option<PathBuf>,
    pub require_command: bool,
}

impl RestoreApplyCommandOptions {
    /// Parse restore apply-command options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut journal = None;
        let mut dfx = "dfx".to_string();
        let mut network = None;
        let mut out = None;
        let mut require_command = false;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| RestoreCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--journal" => journal = Some(PathBuf::from(next_value(&mut args, "--journal")?)),
                "--dfx" => dfx = next_value(&mut args, "--dfx")?,
                "--network" => network = Some(next_value(&mut args, "--network")?),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--require-command" => require_command = true,
                "--help" | "-h" => return Err(RestoreCommandError::Usage(usage())),
                _ => return Err(RestoreCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self {
            journal: journal.ok_or(RestoreCommandError::MissingOption("--journal"))?,
            dfx,
            network,
            out,
            require_command,
        })
    }
}

///
/// RestoreApplyClaimOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreApplyClaimOptions {
    pub journal: PathBuf,
    pub sequence: Option<usize>,
    pub updated_at: Option<String>,
    pub out: Option<PathBuf>,
}

impl RestoreApplyClaimOptions {
    /// Parse restore apply-claim options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut journal = None;
        let mut sequence = None;
        let mut updated_at = None;
        let mut out = None;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| RestoreCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--journal" => journal = Some(PathBuf::from(next_value(&mut args, "--journal")?)),
                "--sequence" => {
                    sequence = Some(parse_sequence(next_value(&mut args, "--sequence")?)?);
                }
                "--updated-at" => updated_at = Some(next_value(&mut args, "--updated-at")?),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--help" | "-h" => return Err(RestoreCommandError::Usage(usage())),
                _ => return Err(RestoreCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self {
            journal: journal.ok_or(RestoreCommandError::MissingOption("--journal"))?,
            sequence,
            updated_at,
            out,
        })
    }
}

///
/// RestoreApplyMarkOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreApplyUnclaimOptions {
    pub journal: PathBuf,
    pub sequence: Option<usize>,
    pub updated_at: Option<String>,
    pub out: Option<PathBuf>,
}

impl RestoreApplyUnclaimOptions {
    /// Parse restore apply-unclaim options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut journal = None;
        let mut sequence = None;
        let mut updated_at = None;
        let mut out = None;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| RestoreCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--journal" => journal = Some(PathBuf::from(next_value(&mut args, "--journal")?)),
                "--sequence" => {
                    sequence = Some(parse_sequence(next_value(&mut args, "--sequence")?)?);
                }
                "--updated-at" => updated_at = Some(next_value(&mut args, "--updated-at")?),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--help" | "-h" => return Err(RestoreCommandError::Usage(usage())),
                _ => return Err(RestoreCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self {
            journal: journal.ok_or(RestoreCommandError::MissingOption("--journal"))?,
            sequence,
            updated_at,
            out,
        })
    }
}

///
/// RestoreApplyMarkOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreApplyMarkOptions {
    pub journal: PathBuf,
    pub sequence: usize,
    pub state: RestoreApplyMarkState,
    pub reason: Option<String>,
    pub updated_at: Option<String>,
    pub out: Option<PathBuf>,
    pub require_pending: bool,
}

impl RestoreApplyMarkOptions {
    /// Parse restore apply-mark options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut journal = None;
        let mut sequence = None;
        let mut state = None;
        let mut reason = None;
        let mut updated_at = None;
        let mut out = None;
        let mut require_pending = false;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| RestoreCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--journal" => journal = Some(PathBuf::from(next_value(&mut args, "--journal")?)),
                "--sequence" => {
                    sequence = Some(parse_sequence(next_value(&mut args, "--sequence")?)?);
                }
                "--state" => {
                    state = Some(RestoreApplyMarkState::parse(next_value(
                        &mut args, "--state",
                    )?)?);
                }
                "--reason" => reason = Some(next_value(&mut args, "--reason")?),
                "--updated-at" => updated_at = Some(next_value(&mut args, "--updated-at")?),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--require-pending" => require_pending = true,
                "--help" | "-h" => return Err(RestoreCommandError::Usage(usage())),
                _ => return Err(RestoreCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self {
            journal: journal.ok_or(RestoreCommandError::MissingOption("--journal"))?,
            sequence: sequence.ok_or(RestoreCommandError::MissingOption("--sequence"))?,
            state: state.ok_or(RestoreCommandError::MissingOption("--state"))?,
            reason,
            updated_at,
            out,
            require_pending,
        })
    }
}

///
/// RestoreApplyMarkState
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RestoreApplyMarkState {
    Completed,
    Failed,
}

impl RestoreApplyMarkState {
    // Parse the restricted operation states accepted by apply-mark.
    fn parse(value: String) -> Result<Self, RestoreCommandError> {
        match value.as_str() {
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(RestoreCommandError::InvalidApplyMarkState(value)),
        }
    }
}

/// Run a restore subcommand.
pub fn run<I>(args: I) -> Result<(), RestoreCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let Some(command) = args.next().and_then(|arg| arg.into_string().ok()) else {
        return Err(RestoreCommandError::Usage(usage()));
    };

    match command.as_str() {
        "plan" => {
            let options = RestorePlanOptions::parse(args)?;
            let plan = plan_restore(&options)?;
            write_plan(&options, &plan)?;
            enforce_restore_plan_requirements(&options, &plan)?;
            Ok(())
        }
        "status" => {
            let options = RestoreStatusOptions::parse(args)?;
            let status = restore_status(&options)?;
            write_status(&options, &status)?;
            Ok(())
        }
        "apply" => {
            let options = RestoreApplyOptions::parse(args)?;
            let dry_run = restore_apply_dry_run(&options)?;
            write_apply_dry_run(&options, &dry_run)?;
            write_apply_journal_if_requested(&options, &dry_run)?;
            Ok(())
        }
        "apply-status" => {
            let options = RestoreApplyStatusOptions::parse(args)?;
            let status = restore_apply_status(&options)?;
            write_apply_status(&options, &status)?;
            enforce_apply_status_requirements(&options, &status)?;
            Ok(())
        }
        "apply-report" => {
            let options = RestoreApplyReportOptions::parse(args)?;
            let report = restore_apply_report(&options)?;
            write_apply_report(&options, &report)?;
            enforce_apply_report_requirements(&options, &report)?;
            Ok(())
        }
        "run" => {
            let options = RestoreRunOptions::parse(args)?;
            let run = if options.execute {
                restore_run_execute_result(&options)?
            } else if options.unclaim_pending {
                RestoreRunResult::ok(restore_run_unclaim_pending(&options)?)
            } else {
                RestoreRunResult::ok(restore_run_dry_run(&options)?)
            };
            write_restore_run(&options, &run.response)?;
            if let Some(error) = run.error {
                return Err(error);
            }
            enforce_restore_run_requirements(&options, &run.response)?;
            Ok(())
        }
        "apply-next" => {
            let options = RestoreApplyNextOptions::parse(args)?;
            let next = restore_apply_next(&options)?;
            write_apply_next(&options, &next)?;
            Ok(())
        }
        "apply-command" => {
            let options = RestoreApplyCommandOptions::parse(args)?;
            let preview = restore_apply_command(&options)?;
            write_apply_command(&options, &preview)?;
            enforce_apply_command_requirements(&options, &preview)?;
            Ok(())
        }
        "apply-claim" => {
            let options = RestoreApplyClaimOptions::parse(args)?;
            let journal = restore_apply_claim(&options)?;
            write_apply_claim(&options, &journal)?;
            Ok(())
        }
        "apply-unclaim" => {
            let options = RestoreApplyUnclaimOptions::parse(args)?;
            let journal = restore_apply_unclaim(&options)?;
            write_apply_unclaim(&options, &journal)?;
            Ok(())
        }
        "apply-mark" => {
            let options = RestoreApplyMarkOptions::parse(args)?;
            let journal = restore_apply_mark(&options)?;
            write_apply_mark(&options, &journal)?;
            Ok(())
        }
        "help" | "--help" | "-h" => Err(RestoreCommandError::Usage(usage())),
        _ => Err(RestoreCommandError::UnknownOption(command)),
    }
}

/// Build a no-mutation restore plan from a manifest and optional mapping.
pub fn plan_restore(options: &RestorePlanOptions) -> Result<RestorePlan, RestoreCommandError> {
    verify_backup_layout_if_required(options)?;

    let manifest = read_manifest_source(options)?;
    let mapping = options.mapping.as_ref().map(read_mapping).transpose()?;

    RestorePlanner::plan(&manifest, mapping.as_ref()).map_err(RestoreCommandError::from)
}

/// Build the initial no-mutation restore status from a restore plan.
pub fn restore_status(
    options: &RestoreStatusOptions,
) -> Result<RestoreStatus, RestoreCommandError> {
    let plan = read_plan(&options.plan)?;
    Ok(RestoreStatus::from_plan(&plan))
}

/// Build a no-mutation restore apply dry-run from a restore plan.
pub fn restore_apply_dry_run(
    options: &RestoreApplyOptions,
) -> Result<RestoreApplyDryRun, RestoreCommandError> {
    let plan = read_plan(&options.plan)?;
    let status = options.status.as_ref().map(read_status).transpose()?;
    if let Some(backup_dir) = &options.backup_dir {
        return RestoreApplyDryRun::try_from_plan_with_artifacts(
            &plan,
            status.as_ref(),
            backup_dir,
        )
        .map_err(RestoreCommandError::from);
    }

    RestoreApplyDryRun::try_from_plan(&plan, status.as_ref()).map_err(RestoreCommandError::from)
}

/// Build a compact restore apply status from a journal file.
pub fn restore_apply_status(
    options: &RestoreApplyStatusOptions,
) -> Result<RestoreApplyJournalStatus, RestoreCommandError> {
    let journal = read_apply_journal(&options.journal)?;
    Ok(journal.status())
}

/// Build an operator-oriented restore apply report from a journal file.
pub fn restore_apply_report(
    options: &RestoreApplyReportOptions,
) -> Result<RestoreApplyJournalReport, RestoreCommandError> {
    let journal = read_apply_journal(&options.journal)?;
    Ok(journal.report())
}

/// Build a no-mutation native restore runner preview from a journal file.
pub fn restore_run_dry_run(
    options: &RestoreRunOptions,
) -> Result<RestoreRunResponse, RestoreCommandError> {
    let journal = read_apply_journal(&options.journal)?;
    let report = journal.report();
    let preview = journal.next_command_preview_with_config(&restore_run_command_config(options));
    let stopped_reason = restore_run_stopped_reason(&report, false, false);
    let next_action = restore_run_next_action(&report, false);

    let mut response = RestoreRunResponse::from_report(
        journal.backup_id,
        report,
        RestoreRunResponseMode::dry_run(stopped_reason, next_action),
    );
    response.operation_available = Some(preview.operation_available);
    response.command_available = Some(preview.command_available);
    response.command = preview.command;
    Ok(response)
}

/// Recover an interrupted restore runner by unclaiming the pending operation.
pub fn restore_run_unclaim_pending(
    options: &RestoreRunOptions,
) -> Result<RestoreRunResponse, RestoreCommandError> {
    let mut journal = read_apply_journal(&options.journal)?;
    let recovered_operation = journal
        .next_transition_operation()
        .filter(|operation| operation.state == RestoreApplyOperationState::Pending)
        .cloned()
        .ok_or(RestoreApplyJournalError::NoPendingOperation)?;

    let recovered_updated_at = timestamp_placeholder();
    journal.mark_next_operation_ready_at(Some(recovered_updated_at.clone()))?;
    write_apply_journal_file(&options.journal, &journal)?;

    let report = journal.report();
    let next_action = restore_run_next_action(&report, true);
    let mut response = RestoreRunResponse::from_report(
        journal.backup_id,
        report,
        RestoreRunResponseMode::unclaim_pending(next_action),
    );
    response.set_operation_receipts(vec![RestoreRunOperationReceipt::recovered_pending(
        recovered_operation.clone(),
        Some(recovered_updated_at),
    )]);
    response.recovered_operation = Some(recovered_operation);
    Ok(response)
}

/// Execute ready restore apply journal operations through generated runner commands.
pub fn restore_run_execute(
    options: &RestoreRunOptions,
) -> Result<RestoreRunResponse, RestoreCommandError> {
    let run = restore_run_execute_result(options)?;
    if let Some(error) = run.error {
        return Err(error);
    }

    Ok(run.response)
}

// Execute ready restore apply operations and retain any deferred runner error.
fn restore_run_execute_result(
    options: &RestoreRunOptions,
) -> Result<RestoreRunResult, RestoreCommandError> {
    let mut journal = read_apply_journal(&options.journal)?;
    let mut executed_operations = Vec::new();
    let mut operation_receipts = Vec::new();
    let config = restore_run_command_config(options);

    loop {
        let report = journal.report();
        let max_steps_reached =
            restore_run_max_steps_reached(options, executed_operations.len(), &report);
        if report.complete || max_steps_reached {
            return Ok(RestoreRunResult::ok(restore_run_execute_summary(
                &journal,
                executed_operations,
                operation_receipts,
                max_steps_reached,
            )));
        }

        enforce_restore_run_executable(&journal, &report)?;
        let preview = journal.next_command_preview_with_config(&config);
        enforce_restore_run_command_available(&preview)?;

        let operation = preview
            .operation
            .clone()
            .ok_or_else(|| restore_command_unavailable_error(&preview))?;
        let command = preview
            .command
            .clone()
            .ok_or_else(|| restore_command_unavailable_error(&preview))?;
        let sequence = operation.sequence;

        enforce_apply_claim_sequence(sequence, &journal)?;
        journal.mark_operation_pending_at(sequence, Some(timestamp_placeholder()))?;
        write_apply_journal_file(&options.journal, &journal)?;

        let status = Command::new(&command.program)
            .args(&command.args)
            .status()?;
        let status_label = exit_status_label(status);
        if status.success() {
            let completed_updated_at = timestamp_placeholder();
            journal.mark_operation_completed_at(sequence, Some(completed_updated_at.clone()))?;
            write_apply_journal_file(&options.journal, &journal)?;
            executed_operations.push(RestoreRunExecutedOperation::completed(
                operation.clone(),
                command.clone(),
                status_label.clone(),
            ));
            operation_receipts.push(RestoreRunOperationReceipt::completed(
                operation,
                command,
                status_label,
                Some(completed_updated_at),
            ));
            continue;
        }

        let failed_updated_at = timestamp_placeholder();
        journal.mark_operation_failed_at(
            sequence,
            format!("{RESTORE_RUN_COMMAND_EXIT_PREFIX}-{status_label}"),
            Some(failed_updated_at.clone()),
        )?;
        write_apply_journal_file(&options.journal, &journal)?;
        executed_operations.push(RestoreRunExecutedOperation::failed(
            operation.clone(),
            command.clone(),
            status_label.clone(),
        ));
        operation_receipts.push(RestoreRunOperationReceipt::failed(
            operation,
            command,
            status_label.clone(),
            Some(failed_updated_at),
        ));
        let response =
            restore_run_execute_summary(&journal, executed_operations, operation_receipts, false);
        return Ok(RestoreRunResult {
            response,
            error: Some(RestoreCommandError::RestoreRunCommandFailed {
                sequence,
                status: status_label,
            }),
        });
    }
}

// Build the shared runner command-preview configuration from CLI options.
fn restore_run_command_config(options: &RestoreRunOptions) -> RestoreApplyCommandConfig {
    restore_command_config(&options.dfx, options.network.as_deref())
}

// Build the shared apply-command preview configuration from CLI options.
fn restore_apply_command_config(options: &RestoreApplyCommandOptions) -> RestoreApplyCommandConfig {
    restore_command_config(&options.dfx, options.network.as_deref())
}

// Build command-preview configuration from common dfx/network inputs.
fn restore_command_config(program: &str, network: Option<&str>) -> RestoreApplyCommandConfig {
    RestoreApplyCommandConfig {
        program: program.to_string(),
        network: network.map(str::to_string),
    }
}

// Check whether execute mode has reached its requested operation batch size.
fn restore_run_max_steps_reached(
    options: &RestoreRunOptions,
    executed_operation_count: usize,
    report: &RestoreApplyJournalReport,
) -> bool {
    options.max_steps == Some(executed_operation_count) && !report.complete
}

// Build the final native runner execution summary.
fn restore_run_execute_summary(
    journal: &RestoreApplyJournal,
    executed_operations: Vec<RestoreRunExecutedOperation>,
    operation_receipts: Vec<RestoreRunOperationReceipt>,
    max_steps_reached: bool,
) -> RestoreRunResponse {
    let report = journal.report();
    let executed_operation_count = executed_operations.len();
    let stopped_reason = restore_run_stopped_reason(&report, max_steps_reached, true);
    let next_action = restore_run_next_action(&report, false);

    let mut response = RestoreRunResponse::from_report(
        journal.backup_id.clone(),
        report,
        RestoreRunResponseMode::execute(stopped_reason, next_action),
    );
    response.max_steps_reached = Some(max_steps_reached);
    response.executed_operation_count = Some(executed_operation_count);
    response.executed_operations = executed_operations;
    response.set_operation_receipts(operation_receipts);
    response
}

// Classify why the native runner stopped for operator summaries.
const fn restore_run_stopped_reason(
    report: &RestoreApplyJournalReport,
    max_steps_reached: bool,
    executed: bool,
) -> &'static str {
    if report.complete {
        return RESTORE_RUN_STOPPED_COMPLETE;
    }
    if report.failed_operations > 0 {
        return RESTORE_RUN_STOPPED_COMMAND_FAILED;
    }
    if report.pending_operations > 0 {
        return RESTORE_RUN_STOPPED_PENDING;
    }
    if !report.ready || report.blocked_operations > 0 {
        return RESTORE_RUN_STOPPED_BLOCKED;
    }
    if max_steps_reached {
        return RESTORE_RUN_STOPPED_MAX_STEPS;
    }
    if executed {
        return RESTORE_RUN_STOPPED_READY;
    }
    RESTORE_RUN_STOPPED_PREVIEW
}

// Recommend the next operator action for the native runner summary.
const fn restore_run_next_action(
    report: &RestoreApplyJournalReport,
    recovered_pending: bool,
) -> &'static str {
    if report.complete {
        return RESTORE_RUN_ACTION_DONE;
    }
    if report.failed_operations > 0 {
        return RESTORE_RUN_ACTION_INSPECT_FAILED;
    }
    if report.pending_operations > 0 {
        return RESTORE_RUN_ACTION_UNCLAIM_PENDING;
    }
    if !report.ready || report.blocked_operations > 0 {
        return RESTORE_RUN_ACTION_FIX_BLOCKED;
    }
    if recovered_pending {
        return RESTORE_RUN_ACTION_RERUN;
    }
    RESTORE_RUN_ACTION_RERUN
}

// Ensure the journal can be advanced by the native restore runner.
fn enforce_restore_run_executable(
    journal: &RestoreApplyJournal,
    report: &RestoreApplyJournalReport,
) -> Result<(), RestoreCommandError> {
    if report.pending_operations > 0 {
        return Err(RestoreCommandError::RestoreApplyPending {
            backup_id: report.backup_id.clone(),
            pending_operations: report.pending_operations,
            next_transition_sequence: report
                .next_transition
                .as_ref()
                .map(|operation| operation.sequence),
        });
    }

    if report.failed_operations > 0 {
        return Err(RestoreCommandError::RestoreApplyFailed {
            backup_id: report.backup_id.clone(),
            failed_operations: report.failed_operations,
        });
    }

    if report.ready {
        return Ok(());
    }

    Err(RestoreCommandError::RestoreApplyNotReady {
        backup_id: journal.backup_id.clone(),
        reasons: report.blocked_reasons.clone(),
    })
}

// Convert an unavailable native runner command into the shared fail-closed error.
fn enforce_restore_run_command_available(
    preview: &RestoreApplyCommandPreview,
) -> Result<(), RestoreCommandError> {
    if preview.command_available {
        return Ok(());
    }

    Err(restore_command_unavailable_error(preview))
}

// Build a shared command-unavailable error from a preview.
fn restore_command_unavailable_error(preview: &RestoreApplyCommandPreview) -> RestoreCommandError {
    RestoreCommandError::RestoreApplyCommandUnavailable {
        backup_id: preview.backup_id.clone(),
        operation_available: preview.operation_available,
        complete: preview.complete,
        blocked_reasons: preview.blocked_reasons.clone(),
    }
}

// Render process exit status without relying on platform-specific internals.
fn exit_status_label(status: std::process::ExitStatus) -> String {
    status
        .code()
        .map_or_else(|| "signal".to_string(), |code| code.to_string())
}

// Enforce caller-requested native runner requirements after output is emitted.
fn enforce_restore_run_requirements(
    options: &RestoreRunOptions,
    run: &RestoreRunResponse,
) -> Result<(), RestoreCommandError> {
    if options.require_complete && !run.complete {
        return Err(RestoreCommandError::RestoreApplyIncomplete {
            backup_id: run.backup_id.clone(),
            completed_operations: run.completed_operations,
            operation_count: run.operation_count,
        });
    }

    if options.require_no_attention && run.attention_required {
        return Err(RestoreCommandError::RestoreApplyReportNeedsAttention {
            backup_id: run.backup_id.clone(),
            outcome: run.outcome.clone(),
        });
    }

    if let Some(expected) = &options.require_run_mode
        && run.run_mode != expected
    {
        return Err(RestoreCommandError::RestoreRunModeMismatch {
            backup_id: run.backup_id.clone(),
            expected: expected.clone(),
            actual: run.run_mode.to_string(),
        });
    }

    if let Some(expected) = &options.require_stopped_reason
        && run.stopped_reason != expected
    {
        return Err(RestoreCommandError::RestoreRunStoppedReasonMismatch {
            backup_id: run.backup_id.clone(),
            expected: expected.clone(),
            actual: run.stopped_reason.to_string(),
        });
    }

    if let Some(expected) = &options.require_next_action
        && run.next_action != expected
    {
        return Err(RestoreCommandError::RestoreRunNextActionMismatch {
            backup_id: run.backup_id.clone(),
            expected: expected.clone(),
            actual: run.next_action.to_string(),
        });
    }

    if let Some(expected) = options.require_executed_count {
        let actual = run.executed_operation_count.unwrap_or(0);
        if actual != expected {
            return Err(RestoreCommandError::RestoreRunExecutedCountMismatch {
                backup_id: run.backup_id.clone(),
                expected,
                actual,
            });
        }
    }

    if let Some(expected) = options.require_receipt_count {
        let actual = run.operation_receipt_count.unwrap_or(0);
        if actual != expected {
            return Err(RestoreCommandError::RestoreRunReceiptCountMismatch {
                backup_id: run.backup_id.clone(),
                expected,
                actual,
            });
        }
    }

    enforce_restore_run_receipt_kind_requirement(
        &run.backup_id,
        RESTORE_RUN_RECEIPT_COMPLETED,
        options.require_completed_receipt_count,
        run.operation_receipt_summary.command_completed,
    )?;
    enforce_restore_run_receipt_kind_requirement(
        &run.backup_id,
        RESTORE_RUN_RECEIPT_FAILED,
        options.require_failed_receipt_count,
        run.operation_receipt_summary.command_failed,
    )?;
    enforce_restore_run_receipt_kind_requirement(
        &run.backup_id,
        RESTORE_RUN_RECEIPT_RECOVERED_PENDING,
        options.require_recovered_receipt_count,
        run.operation_receipt_summary.pending_recovered,
    )?;

    enforce_progress_requirements(
        &run.backup_id,
        &run.progress,
        options.require_remaining_count,
        options.require_attention_count,
        options.require_completion_basis_points,
    )?;
    enforce_pending_before_requirement(
        &run.backup_id,
        &run.pending_summary,
        options.require_no_pending_before.as_deref(),
    )?;

    Ok(())
}

// Fail when a runner receipt-kind count differs from the requested value.
fn enforce_restore_run_receipt_kind_requirement(
    backup_id: &str,
    receipt_kind: &'static str,
    expected: Option<usize>,
    actual: usize,
) -> Result<(), RestoreCommandError> {
    if let Some(expected) = expected
        && actual != expected
    {
        return Err(RestoreCommandError::RestoreRunReceiptKindCountMismatch {
            backup_id: backup_id.to_string(),
            receipt_kind,
            expected,
            actual,
        });
    }

    Ok(())
}

// Enforce caller-requested integer progress requirements after output is emitted.
fn enforce_progress_requirements(
    backup_id: &str,
    progress: &RestoreApplyProgressSummary,
    require_remaining_count: Option<usize>,
    require_attention_count: Option<usize>,
    require_completion_basis_points: Option<usize>,
) -> Result<(), RestoreCommandError> {
    if let Some(expected) = require_remaining_count
        && progress.remaining_operations != expected
    {
        return Err(RestoreCommandError::RestoreApplyProgressMismatch {
            backup_id: backup_id.to_string(),
            field: "remaining_operations",
            expected,
            actual: progress.remaining_operations,
        });
    }

    if let Some(expected) = require_attention_count
        && progress.attention_operations != expected
    {
        return Err(RestoreCommandError::RestoreApplyProgressMismatch {
            backup_id: backup_id.to_string(),
            field: "attention_operations",
            expected,
            actual: progress.attention_operations,
        });
    }

    if let Some(expected) = require_completion_basis_points
        && progress.completion_basis_points != expected
    {
        return Err(RestoreCommandError::RestoreApplyProgressMismatch {
            backup_id: backup_id.to_string(),
            field: "completion_basis_points",
            expected,
            actual: progress.completion_basis_points,
        });
    }

    Ok(())
}

// Enforce pending-work freshness using caller-supplied comparable update markers.
fn enforce_pending_before_requirement(
    backup_id: &str,
    pending: &RestoreApplyPendingSummary,
    require_no_pending_before: Option<&str>,
) -> Result<(), RestoreCommandError> {
    let Some(cutoff_updated_at) = require_no_pending_before else {
        return Ok(());
    };

    if pending.pending_operations == 0 {
        return Ok(());
    }

    if pending.pending_updated_at_known
        && pending
            .pending_updated_at
            .as_deref()
            .is_some_and(|updated_at| updated_at >= cutoff_updated_at)
    {
        return Ok(());
    }

    Err(RestoreCommandError::RestoreApplyPendingStale {
        backup_id: backup_id.to_string(),
        cutoff_updated_at: cutoff_updated_at.to_string(),
        pending_sequence: pending.pending_sequence,
        pending_updated_at: pending.pending_updated_at.clone(),
    })
}

// Enforce caller-requested apply report requirements after report output is emitted.
fn enforce_apply_report_requirements(
    options: &RestoreApplyReportOptions,
    report: &RestoreApplyJournalReport,
) -> Result<(), RestoreCommandError> {
    if options.require_no_attention && report.attention_required {
        return Err(RestoreCommandError::RestoreApplyReportNeedsAttention {
            backup_id: report.backup_id.clone(),
            outcome: report.outcome.clone(),
        });
    }

    enforce_progress_requirements(
        &report.backup_id,
        &report.progress,
        options.require_remaining_count,
        options.require_attention_count,
        options.require_completion_basis_points,
    )?;
    enforce_pending_before_requirement(
        &report.backup_id,
        &report.pending_summary,
        options.require_no_pending_before.as_deref(),
    )
}

// Enforce caller-requested apply journal requirements after status is emitted.
fn enforce_apply_status_requirements(
    options: &RestoreApplyStatusOptions,
    status: &RestoreApplyJournalStatus,
) -> Result<(), RestoreCommandError> {
    if options.require_ready && !status.ready {
        return Err(RestoreCommandError::RestoreApplyNotReady {
            backup_id: status.backup_id.clone(),
            reasons: status.blocked_reasons.clone(),
        });
    }

    if options.require_no_pending && status.pending_operations > 0 {
        return Err(RestoreCommandError::RestoreApplyPending {
            backup_id: status.backup_id.clone(),
            pending_operations: status.pending_operations,
            next_transition_sequence: status.next_transition_sequence,
        });
    }

    if options.require_no_failed && status.failed_operations > 0 {
        return Err(RestoreCommandError::RestoreApplyFailed {
            backup_id: status.backup_id.clone(),
            failed_operations: status.failed_operations,
        });
    }

    if options.require_complete && !status.complete {
        return Err(RestoreCommandError::RestoreApplyIncomplete {
            backup_id: status.backup_id.clone(),
            completed_operations: status.completed_operations,
            operation_count: status.operation_count,
        });
    }

    enforce_progress_requirements(
        &status.backup_id,
        &status.progress,
        options.require_remaining_count,
        options.require_attention_count,
        options.require_completion_basis_points,
    )?;
    enforce_pending_before_requirement(
        &status.backup_id,
        &status.pending_summary,
        options.require_no_pending_before.as_deref(),
    )?;

    Ok(())
}

/// Build the next restore apply operation response from a journal file.
pub fn restore_apply_next(
    options: &RestoreApplyNextOptions,
) -> Result<RestoreApplyNextOperation, RestoreCommandError> {
    let journal = read_apply_journal(&options.journal)?;
    Ok(journal.next_operation())
}

/// Build the next restore apply command preview from a journal file.
pub fn restore_apply_command(
    options: &RestoreApplyCommandOptions,
) -> Result<RestoreApplyCommandPreview, RestoreCommandError> {
    let journal = read_apply_journal(&options.journal)?;
    Ok(journal.next_command_preview_with_config(&restore_apply_command_config(options)))
}

// Enforce caller-requested command preview requirements after preview output is emitted.
fn enforce_apply_command_requirements(
    options: &RestoreApplyCommandOptions,
    preview: &RestoreApplyCommandPreview,
) -> Result<(), RestoreCommandError> {
    if !options.require_command || preview.command_available {
        return Ok(());
    }

    Err(restore_command_unavailable_error(preview))
}

/// Mark the next restore apply journal operation pending.
pub fn restore_apply_claim(
    options: &RestoreApplyClaimOptions,
) -> Result<RestoreApplyJournal, RestoreCommandError> {
    let mut journal = read_apply_journal(&options.journal)?;
    let updated_at = Some(state_updated_at(options.updated_at.as_ref()));

    if let Some(sequence) = options.sequence {
        enforce_apply_claim_sequence(sequence, &journal)?;
        journal.mark_operation_pending_at(sequence, updated_at)?;
        return Ok(journal);
    }

    journal.mark_next_operation_pending_at(updated_at)?;
    Ok(journal)
}

// Ensure a runner claim still matches the operation it previewed.
fn enforce_apply_claim_sequence(
    expected: usize,
    journal: &RestoreApplyJournal,
) -> Result<(), RestoreCommandError> {
    let actual = journal
        .next_transition_operation()
        .map(|operation| operation.sequence);

    if actual == Some(expected) {
        return Ok(());
    }

    Err(RestoreCommandError::RestoreApplyClaimSequenceMismatch { expected, actual })
}

/// Mark the current pending restore apply journal operation ready again.
pub fn restore_apply_unclaim(
    options: &RestoreApplyUnclaimOptions,
) -> Result<RestoreApplyJournal, RestoreCommandError> {
    let mut journal = read_apply_journal(&options.journal)?;
    if let Some(sequence) = options.sequence {
        enforce_apply_unclaim_sequence(sequence, &journal)?;
    }

    journal.mark_next_operation_ready_at(Some(state_updated_at(options.updated_at.as_ref())))?;
    Ok(journal)
}

// Ensure a runner unclaim still matches the pending operation it recovered.
fn enforce_apply_unclaim_sequence(
    expected: usize,
    journal: &RestoreApplyJournal,
) -> Result<(), RestoreCommandError> {
    let actual = journal
        .next_transition_operation()
        .map(|operation| operation.sequence);

    if actual == Some(expected) {
        return Ok(());
    }

    Err(RestoreCommandError::RestoreApplyUnclaimSequenceMismatch { expected, actual })
}

/// Mark one restore apply journal operation completed or failed.
pub fn restore_apply_mark(
    options: &RestoreApplyMarkOptions,
) -> Result<RestoreApplyJournal, RestoreCommandError> {
    let mut journal = read_apply_journal(&options.journal)?;
    enforce_apply_mark_pending_requirement(options, &journal)?;

    match options.state {
        RestoreApplyMarkState::Completed => {
            journal.mark_operation_completed_at(
                options.sequence,
                Some(state_updated_at(options.updated_at.as_ref())),
            )?;
        }
        RestoreApplyMarkState::Failed => {
            let reason =
                options
                    .reason
                    .clone()
                    .ok_or(RestoreApplyJournalError::FailureReasonRequired(
                        options.sequence,
                    ))?;
            journal.mark_operation_failed_at(
                options.sequence,
                reason,
                Some(state_updated_at(options.updated_at.as_ref())),
            )?;
        }
    }

    Ok(journal)
}

// Enforce that apply-mark only records an already claimed operation when requested.
fn enforce_apply_mark_pending_requirement(
    options: &RestoreApplyMarkOptions,
    journal: &RestoreApplyJournal,
) -> Result<(), RestoreCommandError> {
    if !options.require_pending {
        return Ok(());
    }

    let state = journal
        .operations
        .iter()
        .find(|operation| operation.sequence == options.sequence)
        .map(|operation| operation.state.clone())
        .ok_or(RestoreApplyJournalError::OperationNotFound(
            options.sequence,
        ))?;

    if state == RestoreApplyOperationState::Pending {
        return Ok(());
    }

    Err(RestoreCommandError::RestoreApplyMarkRequiresPending {
        sequence: options.sequence,
        state,
    })
}

// Enforce caller-requested restore plan requirements after the plan is emitted.
fn enforce_restore_plan_requirements(
    options: &RestorePlanOptions,
    plan: &RestorePlan,
) -> Result<(), RestoreCommandError> {
    if !options.require_restore_ready || plan.readiness_summary.ready {
        return Ok(());
    }

    Err(RestoreCommandError::RestoreNotReady {
        backup_id: plan.backup_id.clone(),
        reasons: plan.readiness_summary.reasons.clone(),
    })
}

// Verify backup layout integrity before restore planning when requested.
fn verify_backup_layout_if_required(
    options: &RestorePlanOptions,
) -> Result<(), RestoreCommandError> {
    if !options.require_verified {
        return Ok(());
    }

    let Some(dir) = &options.backup_dir else {
        return Err(RestoreCommandError::RequireVerifiedNeedsBackupDir);
    };

    BackupLayout::new(dir.clone()).verify_integrity()?;
    Ok(())
}

// Read the manifest from a direct path or canonical backup layout.
fn read_manifest_source(
    options: &RestorePlanOptions,
) -> Result<FleetBackupManifest, RestoreCommandError> {
    if let Some(path) = &options.manifest {
        return read_manifest(path);
    }

    let Some(dir) = &options.backup_dir else {
        return Err(RestoreCommandError::MissingOption(
            "--manifest or --backup-dir",
        ));
    };

    BackupLayout::new(dir.clone())
        .read_manifest()
        .map_err(RestoreCommandError::from)
}

// Read and decode a fleet backup manifest from disk.
fn read_manifest(path: &PathBuf) -> Result<FleetBackupManifest, RestoreCommandError> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(RestoreCommandError::from)
}

// Read and decode an optional source-to-target restore mapping from disk.
fn read_mapping(path: &PathBuf) -> Result<RestoreMapping, RestoreCommandError> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(RestoreCommandError::from)
}

// Read and decode a restore plan from disk.
fn read_plan(path: &PathBuf) -> Result<RestorePlan, RestoreCommandError> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(RestoreCommandError::from)
}

// Read and decode a restore status from disk.
fn read_status(path: &PathBuf) -> Result<RestoreStatus, RestoreCommandError> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(RestoreCommandError::from)
}

// Read and decode a restore apply journal from disk.
fn read_apply_journal(path: &PathBuf) -> Result<RestoreApplyJournal, RestoreCommandError> {
    let data = fs::read_to_string(path)?;
    let journal: RestoreApplyJournal = serde_json::from_str(&data)?;
    journal.validate()?;
    Ok(journal)
}

// Parse shared restore apply progress requirement flags.
fn parse_progress_requirement_option<I>(
    arg: &str,
    args: &mut I,
    require_remaining_count: &mut Option<usize>,
    require_attention_count: &mut Option<usize>,
    require_completion_basis_points: &mut Option<usize>,
) -> Result<bool, RestoreCommandError>
where
    I: Iterator<Item = OsString>,
{
    match arg {
        "--require-remaining-count" => {
            *require_remaining_count = Some(parse_sequence(next_value(
                args,
                "--require-remaining-count",
            )?)?);
            Ok(true)
        }
        "--require-attention-count" => {
            *require_attention_count = Some(parse_sequence(next_value(
                args,
                "--require-attention-count",
            )?)?);
            Ok(true)
        }
        "--require-completion-basis-points" => {
            *require_completion_basis_points = Some(parse_sequence(next_value(
                args,
                "--require-completion-basis-points",
            )?)?);
            Ok(true)
        }
        _ => Ok(false),
    }
}

// Parse shared restore apply pending freshness requirement flags.
fn parse_pending_requirement_option<I>(
    arg: &str,
    args: &mut I,
    require_no_pending_before: &mut Option<String>,
) -> Result<bool, RestoreCommandError>
where
    I: Iterator<Item = OsString>,
{
    match arg {
        "--require-no-pending-before" => {
            *require_no_pending_before = Some(next_value(args, "--require-no-pending-before")?);
            Ok(true)
        }
        _ => Ok(false),
    }
}

// Parse restore-run count requirement flags.
fn parse_run_count_requirement_option<I>(
    arg: &str,
    args: &mut I,
    require_executed_count: &mut Option<usize>,
    require_receipt_count: &mut Option<usize>,
) -> Result<bool, RestoreCommandError>
where
    I: Iterator<Item = OsString>,
{
    match arg {
        "--require-executed-count" => {
            *require_executed_count = Some(parse_sequence(next_value(
                args,
                "--require-executed-count",
            )?)?);
            Ok(true)
        }
        "--require-receipt-count" => {
            *require_receipt_count = Some(parse_sequence(next_value(
                args,
                "--require-receipt-count",
            )?)?);
            Ok(true)
        }
        _ => Ok(false),
    }
}

// Parse restore-run receipt-kind count requirement flags.
fn parse_run_receipt_kind_requirement_option<I>(
    arg: &str,
    args: &mut I,
    require_completed_receipt_count: &mut Option<usize>,
    require_failed_receipt_count: &mut Option<usize>,
    require_recovered_receipt_count: &mut Option<usize>,
) -> Result<bool, RestoreCommandError>
where
    I: Iterator<Item = OsString>,
{
    match arg {
        "--require-completed-receipt-count" => {
            *require_completed_receipt_count = Some(parse_sequence(next_value(
                args,
                "--require-completed-receipt-count",
            )?)?);
            Ok(true)
        }
        "--require-failed-receipt-count" => {
            *require_failed_receipt_count = Some(parse_sequence(next_value(
                args,
                "--require-failed-receipt-count",
            )?)?);
            Ok(true)
        }
        "--require-recovered-receipt-count" => {
            *require_recovered_receipt_count = Some(parse_sequence(next_value(
                args,
                "--require-recovered-receipt-count",
            )?)?);
            Ok(true)
        }
        _ => Ok(false),
    }
}

// Parse a restore apply journal operation sequence value.
fn parse_sequence(value: String) -> Result<usize, RestoreCommandError> {
    value
        .parse::<usize>()
        .map_err(|_| RestoreCommandError::InvalidSequence)
}

// Return the caller-supplied journal update marker or the current placeholder.
fn state_updated_at(updated_at: Option<&String>) -> String {
    updated_at.cloned().unwrap_or_else(timestamp_placeholder)
}

// Return a placeholder timestamp until the CLI owns a clock abstraction.
fn timestamp_placeholder() -> String {
    "unknown".to_string()
}

// Write the computed plan to stdout or a requested output file.
fn write_plan(options: &RestorePlanOptions, plan: &RestorePlan) -> Result<(), RestoreCommandError> {
    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(plan)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, plan)?;
    writeln!(handle)?;
    Ok(())
}

// Write the computed status to stdout or a requested output file.
fn write_status(
    options: &RestoreStatusOptions,
    status: &RestoreStatus,
) -> Result<(), RestoreCommandError> {
    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(status)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, status)?;
    writeln!(handle)?;
    Ok(())
}

// Write the computed apply dry-run to stdout or a requested output file.
fn write_apply_dry_run(
    options: &RestoreApplyOptions,
    dry_run: &RestoreApplyDryRun,
) -> Result<(), RestoreCommandError> {
    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(dry_run)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, dry_run)?;
    writeln!(handle)?;
    Ok(())
}

// Write the initial apply journal when the caller requests one.
fn write_apply_journal_if_requested(
    options: &RestoreApplyOptions,
    dry_run: &RestoreApplyDryRun,
) -> Result<(), RestoreCommandError> {
    let Some(path) = &options.journal_out else {
        return Ok(());
    };

    let journal = RestoreApplyJournal::from_dry_run(dry_run);
    let data = serde_json::to_vec_pretty(&journal)?;
    fs::write(path, data)?;
    Ok(())
}

// Write the computed apply journal status to stdout or a requested output file.
fn write_apply_status(
    options: &RestoreApplyStatusOptions,
    status: &RestoreApplyJournalStatus,
) -> Result<(), RestoreCommandError> {
    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(status)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, status)?;
    writeln!(handle)?;
    Ok(())
}

// Write the computed apply journal report to stdout or a requested output file.
fn write_apply_report(
    options: &RestoreApplyReportOptions,
    report: &RestoreApplyJournalReport,
) -> Result<(), RestoreCommandError> {
    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(report)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, report)?;
    writeln!(handle)?;
    Ok(())
}

// Write the restore runner response to stdout or a requested output file.
fn write_restore_run(
    options: &RestoreRunOptions,
    run: &RestoreRunResponse,
) -> Result<(), RestoreCommandError> {
    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(run)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, run)?;
    writeln!(handle)?;
    Ok(())
}

// Persist the restore apply journal to its canonical runner path.
fn write_apply_journal_file(
    path: &PathBuf,
    journal: &RestoreApplyJournal,
) -> Result<(), RestoreCommandError> {
    let data = serde_json::to_vec_pretty(journal)?;
    fs::write(path, data)?;
    Ok(())
}

// Write the computed apply next-operation response to stdout or a requested output file.
fn write_apply_next(
    options: &RestoreApplyNextOptions,
    next: &RestoreApplyNextOperation,
) -> Result<(), RestoreCommandError> {
    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(next)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, next)?;
    writeln!(handle)?;
    Ok(())
}

// Write the computed apply command preview to stdout or a requested output file.
fn write_apply_command(
    options: &RestoreApplyCommandOptions,
    preview: &RestoreApplyCommandPreview,
) -> Result<(), RestoreCommandError> {
    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(preview)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, preview)?;
    writeln!(handle)?;
    Ok(())
}

// Write the claimed apply journal to stdout or a requested output file.
fn write_apply_claim(
    options: &RestoreApplyClaimOptions,
    journal: &RestoreApplyJournal,
) -> Result<(), RestoreCommandError> {
    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(journal)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, journal)?;
    writeln!(handle)?;
    Ok(())
}

// Write the unclaimed apply journal to stdout or a requested output file.
fn write_apply_unclaim(
    options: &RestoreApplyUnclaimOptions,
    journal: &RestoreApplyJournal,
) -> Result<(), RestoreCommandError> {
    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(journal)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, journal)?;
    writeln!(handle)?;
    Ok(())
}

// Write the updated apply journal to stdout or a requested output file.
fn write_apply_mark(
    options: &RestoreApplyMarkOptions,
    journal: &RestoreApplyJournal,
) -> Result<(), RestoreCommandError> {
    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(journal)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, journal)?;
    writeln!(handle)?;
    Ok(())
}

// Read the next required option value.
fn next_value<I>(args: &mut I, option: &'static str) -> Result<String, RestoreCommandError>
where
    I: Iterator<Item = OsString>,
{
    args.next()
        .and_then(|value| value.into_string().ok())
        .ok_or(RestoreCommandError::MissingValue(option))
}

// Return restore command usage text.
const fn usage() -> &'static str {
    "usage: canic restore plan (--manifest <file> | --backup-dir <dir>) [--mapping <file>] [--out <file>] [--require-verified] [--require-restore-ready]\n       canic restore status --plan <file> [--out <file>]\n       canic restore apply --plan <file> [--status <file>] [--backup-dir <dir>] --dry-run [--out <file>] [--journal-out <file>]\n       canic restore apply-status --journal <file> [--out <file>] [--require-ready] [--require-no-pending] [--require-no-failed] [--require-complete] [--require-remaining-count <n>] [--require-attention-count <n>] [--require-completion-basis-points <n>] [--require-no-pending-before <text>]\n       canic restore apply-report --journal <file> [--out <file>] [--require-no-attention] [--require-remaining-count <n>] [--require-attention-count <n>] [--require-completion-basis-points <n>] [--require-no-pending-before <text>]\n       canic restore run --journal <file> (--dry-run | --execute | --unclaim-pending) [--dfx <path>] [--network <name>] [--max-steps <n>] [--out <file>] [--require-complete] [--require-no-attention] [--require-run-mode <text>] [--require-stopped-reason <text>] [--require-next-action <text>] [--require-executed-count <n>] [--require-receipt-count <n>] [--require-completed-receipt-count <n>] [--require-failed-receipt-count <n>] [--require-recovered-receipt-count <n>] [--require-remaining-count <n>] [--require-attention-count <n>] [--require-completion-basis-points <n>] [--require-no-pending-before <text>]\n       canic restore apply-next --journal <file> [--out <file>]\n       canic restore apply-command --journal <file> [--dfx <path>] [--network <name>] [--out <file>] [--require-command]\n       canic restore apply-claim --journal <file> [--sequence <n>] [--updated-at <text>] [--out <file>]\n       canic restore apply-unclaim --journal <file> [--sequence <n>] [--updated-at <text>] [--out <file>]\n       canic restore apply-mark --journal <file> --sequence <n> --state completed|failed [--reason <text>] [--updated-at <text>] [--out <file>] [--require-pending]"
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_backup::restore::RestoreApplyOperationState;
    use canic_backup::{
        artifacts::ArtifactChecksum,
        journal::{ArtifactJournalEntry, ArtifactState, DownloadJournal},
        manifest::{
            BackupUnit, BackupUnitKind, ConsistencyMode, ConsistencySection, FleetMember,
            FleetSection, IdentityMode, SourceMetadata, SourceSnapshot, ToolMetadata,
            VerificationCheck, VerificationPlan,
        },
    };
    use serde_json::json;
    use std::{
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    const ROOT: &str = "aaaaa-aa";
    const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
    const MAPPED_CHILD: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    ///
    /// RestoreCliFixture
    ///

    struct RestoreCliFixture {
        root: PathBuf,
        journal_path: PathBuf,
        out_path: PathBuf,
    }

    impl RestoreCliFixture {
        // Create a temp restore CLI fixture with canonical journal and output paths.
        fn new(prefix: &str, out_file: &str) -> Self {
            let root = temp_dir(prefix);
            fs::create_dir_all(&root).expect("create temp root");

            Self {
                journal_path: root.join("restore-apply-journal.json"),
                out_path: root.join(out_file),
                root,
            }
        }

        // Persist a restore apply journal at the fixture journal path.
        fn write_journal(&self, journal: &RestoreApplyJournal) {
            fs::write(
                &self.journal_path,
                serde_json::to_vec(journal).expect("serialize journal"),
            )
            .expect("write journal");
        }

        // Run apply-status against the fixture journal and output paths.
        fn run_apply_status(&self, extra: &[&str]) -> Result<(), RestoreCommandError> {
            self.run_journal_command("apply-status", extra)
        }

        // Run apply-report against the fixture journal and output paths.
        fn run_apply_report(&self, extra: &[&str]) -> Result<(), RestoreCommandError> {
            self.run_journal_command("apply-report", extra)
        }

        // Run restore-run against the fixture journal and output paths.
        fn run_restore_run(&self, extra: &[&str]) -> Result<(), RestoreCommandError> {
            self.run_journal_command("run", extra)
        }

        // Read the fixture output as a typed JSON value.
        fn read_out<T>(&self, label: &str) -> T
        where
            T: serde::de::DeserializeOwned,
        {
            serde_json::from_slice(&fs::read(&self.out_path).expect(label)).expect(label)
        }

        // Build and run one journal-backed restore CLI command.
        fn run_journal_command(
            &self,
            command: &str,
            extra: &[&str],
        ) -> Result<(), RestoreCommandError> {
            let mut args = vec![
                OsString::from(command),
                OsString::from("--journal"),
                OsString::from(self.journal_path.as_os_str()),
                OsString::from("--out"),
                OsString::from(self.out_path.as_os_str()),
            ];
            args.extend(extra.iter().map(OsString::from));
            run(args)
        }
    }

    impl Drop for RestoreCliFixture {
        // Remove the fixture directory after each test completes.
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    // Ensure restore plan options parse the intended no-mutation command.
    #[test]
    fn parses_restore_plan_options() {
        let options = RestorePlanOptions::parse([
            OsString::from("--manifest"),
            OsString::from("manifest.json"),
            OsString::from("--mapping"),
            OsString::from("mapping.json"),
            OsString::from("--out"),
            OsString::from("plan.json"),
            OsString::from("--require-restore-ready"),
        ])
        .expect("parse options");

        assert_eq!(options.manifest, Some(PathBuf::from("manifest.json")));
        assert_eq!(options.backup_dir, None);
        assert_eq!(options.mapping, Some(PathBuf::from("mapping.json")));
        assert_eq!(options.out, Some(PathBuf::from("plan.json")));
        assert!(!options.require_verified);
        assert!(options.require_restore_ready);
    }

    // Ensure verified restore plan options parse with the canonical backup source.
    #[test]
    fn parses_verified_restore_plan_options() {
        let options = RestorePlanOptions::parse([
            OsString::from("--backup-dir"),
            OsString::from("backups/run"),
            OsString::from("--require-verified"),
        ])
        .expect("parse verified options");

        assert_eq!(options.manifest, None);
        assert_eq!(options.backup_dir, Some(PathBuf::from("backups/run")));
        assert_eq!(options.mapping, None);
        assert_eq!(options.out, None);
        assert!(options.require_verified);
        assert!(!options.require_restore_ready);
    }

    // Ensure restore status options parse the intended no-mutation command.
    #[test]
    fn parses_restore_status_options() {
        let options = RestoreStatusOptions::parse([
            OsString::from("--plan"),
            OsString::from("restore-plan.json"),
            OsString::from("--out"),
            OsString::from("restore-status.json"),
        ])
        .expect("parse status options");

        assert_eq!(options.plan, PathBuf::from("restore-plan.json"));
        assert_eq!(options.out, Some(PathBuf::from("restore-status.json")));
    }

    // Ensure restore apply options require the explicit dry-run mode.
    #[test]
    fn parses_restore_apply_dry_run_options() {
        let options = RestoreApplyOptions::parse([
            OsString::from("--plan"),
            OsString::from("restore-plan.json"),
            OsString::from("--status"),
            OsString::from("restore-status.json"),
            OsString::from("--backup-dir"),
            OsString::from("backups/run"),
            OsString::from("--dry-run"),
            OsString::from("--out"),
            OsString::from("restore-apply-dry-run.json"),
            OsString::from("--journal-out"),
            OsString::from("restore-apply-journal.json"),
        ])
        .expect("parse apply options");

        assert_eq!(options.plan, PathBuf::from("restore-plan.json"));
        assert_eq!(options.status, Some(PathBuf::from("restore-status.json")));
        assert_eq!(options.backup_dir, Some(PathBuf::from("backups/run")));
        assert_eq!(
            options.out,
            Some(PathBuf::from("restore-apply-dry-run.json"))
        );
        assert_eq!(
            options.journal_out,
            Some(PathBuf::from("restore-apply-journal.json"))
        );
        assert!(options.dry_run);
    }

    // Ensure restore apply-status options parse the intended journal command.
    #[test]
    fn parses_restore_apply_status_options() {
        let options = RestoreApplyStatusOptions::parse([
            OsString::from("--journal"),
            OsString::from("restore-apply-journal.json"),
            OsString::from("--out"),
            OsString::from("restore-apply-status.json"),
            OsString::from("--require-ready"),
            OsString::from("--require-no-pending"),
            OsString::from("--require-no-failed"),
            OsString::from("--require-complete"),
            OsString::from("--require-remaining-count"),
            OsString::from("7"),
            OsString::from("--require-attention-count"),
            OsString::from("0"),
            OsString::from("--require-completion-basis-points"),
            OsString::from("1250"),
            OsString::from("--require-no-pending-before"),
            OsString::from("2026-05-05T12:00:00Z"),
        ])
        .expect("parse apply-status options");

        assert_eq!(options.journal, PathBuf::from("restore-apply-journal.json"));
        assert!(options.require_ready);
        assert!(options.require_no_pending);
        assert!(options.require_no_failed);
        assert!(options.require_complete);
        assert_eq!(options.require_remaining_count, Some(7));
        assert_eq!(options.require_attention_count, Some(0));
        assert_eq!(options.require_completion_basis_points, Some(1250));
        assert_eq!(
            options.require_no_pending_before.as_deref(),
            Some("2026-05-05T12:00:00Z")
        );
        assert_eq!(
            options.out,
            Some(PathBuf::from("restore-apply-status.json"))
        );
    }

    // Ensure restore apply-report options parse the intended journal command.
    #[test]
    fn parses_restore_apply_report_options() {
        let options = RestoreApplyReportOptions::parse([
            OsString::from("--journal"),
            OsString::from("restore-apply-journal.json"),
            OsString::from("--out"),
            OsString::from("restore-apply-report.json"),
            OsString::from("--require-no-attention"),
            OsString::from("--require-remaining-count"),
            OsString::from("8"),
            OsString::from("--require-attention-count"),
            OsString::from("0"),
            OsString::from("--require-completion-basis-points"),
            OsString::from("0"),
            OsString::from("--require-no-pending-before"),
            OsString::from("2026-05-05T12:00:00Z"),
        ])
        .expect("parse apply-report options");

        assert_eq!(options.journal, PathBuf::from("restore-apply-journal.json"));
        assert!(options.require_no_attention);
        assert_eq!(options.require_remaining_count, Some(8));
        assert_eq!(options.require_attention_count, Some(0));
        assert_eq!(options.require_completion_basis_points, Some(0));
        assert_eq!(
            options.require_no_pending_before.as_deref(),
            Some("2026-05-05T12:00:00Z")
        );
        assert_eq!(
            options.out,
            Some(PathBuf::from("restore-apply-report.json"))
        );
    }

    // Ensure restore run options parse the native runner dry-run command.
    #[test]
    fn parses_restore_run_dry_run_options() {
        let options = RestoreRunOptions::parse([
            OsString::from("--journal"),
            OsString::from("restore-apply-journal.json"),
            OsString::from("--dry-run"),
            OsString::from("--dfx"),
            OsString::from("/tmp/dfx"),
            OsString::from("--network"),
            OsString::from("local"),
            OsString::from("--out"),
            OsString::from("restore-run-dry-run.json"),
            OsString::from("--max-steps"),
            OsString::from("1"),
            OsString::from("--require-complete"),
            OsString::from("--require-no-attention"),
            OsString::from("--require-run-mode"),
            OsString::from("dry-run"),
            OsString::from("--require-stopped-reason"),
            OsString::from("preview"),
            OsString::from("--require-next-action"),
            OsString::from("rerun"),
            OsString::from("--require-executed-count"),
            OsString::from("0"),
            OsString::from("--require-receipt-count"),
            OsString::from("0"),
            OsString::from("--require-completed-receipt-count"),
            OsString::from("0"),
            OsString::from("--require-failed-receipt-count"),
            OsString::from("0"),
            OsString::from("--require-recovered-receipt-count"),
            OsString::from("0"),
            OsString::from("--require-remaining-count"),
            OsString::from("8"),
            OsString::from("--require-attention-count"),
            OsString::from("0"),
            OsString::from("--require-completion-basis-points"),
            OsString::from("0"),
            OsString::from("--require-no-pending-before"),
            OsString::from("2026-05-05T12:00:00Z"),
        ])
        .expect("parse restore run options");

        assert_eq!(options.journal, PathBuf::from("restore-apply-journal.json"));
        assert_eq!(options.dfx, "/tmp/dfx");
        assert_eq!(options.network.as_deref(), Some("local"));
        assert_eq!(options.out, Some(PathBuf::from("restore-run-dry-run.json")));
        assert!(options.dry_run);
        assert!(!options.execute);
        assert!(!options.unclaim_pending);
        assert_eq!(options.max_steps, Some(1));
        assert!(options.require_complete);
        assert!(options.require_no_attention);
        assert_eq!(options.require_run_mode.as_deref(), Some("dry-run"));
        assert_eq!(options.require_stopped_reason.as_deref(), Some("preview"));
        assert_eq!(options.require_next_action.as_deref(), Some("rerun"));
        assert_eq!(options.require_executed_count, Some(0));
        assert_eq!(options.require_receipt_count, Some(0));
        assert_eq!(options.require_completed_receipt_count, Some(0));
        assert_eq!(options.require_failed_receipt_count, Some(0));
        assert_eq!(options.require_recovered_receipt_count, Some(0));
        assert_eq!(options.require_remaining_count, Some(8));
        assert_eq!(options.require_attention_count, Some(0));
        assert_eq!(options.require_completion_basis_points, Some(0));
        assert_eq!(
            options.require_no_pending_before.as_deref(),
            Some("2026-05-05T12:00:00Z")
        );
    }

    // Ensure restore run options parse the native execute command.
    #[test]
    fn parses_restore_run_execute_options() {
        let options = RestoreRunOptions::parse([
            OsString::from("--journal"),
            OsString::from("restore-apply-journal.json"),
            OsString::from("--execute"),
            OsString::from("--dfx"),
            OsString::from("/bin/true"),
            OsString::from("--max-steps"),
            OsString::from("4"),
        ])
        .expect("parse restore run execute options");

        assert_eq!(options.journal, PathBuf::from("restore-apply-journal.json"));
        assert_eq!(options.dfx, "/bin/true");
        assert_eq!(options.network, None);
        assert_eq!(options.out, None);
        assert!(!options.dry_run);
        assert!(options.execute);
        assert!(!options.unclaim_pending);
        assert_eq!(options.max_steps, Some(4));
        assert!(!options.require_complete);
        assert!(!options.require_no_attention);
        assert_eq!(options.require_run_mode, None);
        assert_eq!(options.require_stopped_reason, None);
        assert_eq!(options.require_next_action, None);
        assert_eq!(options.require_executed_count, None);
        assert_eq!(options.require_receipt_count, None);
        assert_eq!(options.require_completed_receipt_count, None);
        assert_eq!(options.require_failed_receipt_count, None);
        assert_eq!(options.require_recovered_receipt_count, None);
    }

    // Ensure restore run options parse the native pending-operation recovery mode.
    #[test]
    fn parses_restore_run_unclaim_pending_options() {
        let options = RestoreRunOptions::parse([
            OsString::from("--journal"),
            OsString::from("restore-apply-journal.json"),
            OsString::from("--unclaim-pending"),
            OsString::from("--out"),
            OsString::from("restore-run.json"),
        ])
        .expect("parse restore run unclaim options");

        assert_eq!(options.journal, PathBuf::from("restore-apply-journal.json"));
        assert_eq!(options.out, Some(PathBuf::from("restore-run.json")));
        assert!(!options.dry_run);
        assert!(!options.execute);
        assert!(options.unclaim_pending);
    }

    // Ensure restore apply-next options parse the intended journal command.
    #[test]
    fn parses_restore_apply_next_options() {
        let options = RestoreApplyNextOptions::parse([
            OsString::from("--journal"),
            OsString::from("restore-apply-journal.json"),
            OsString::from("--out"),
            OsString::from("restore-apply-next.json"),
        ])
        .expect("parse apply-next options");

        assert_eq!(options.journal, PathBuf::from("restore-apply-journal.json"));
        assert_eq!(options.out, Some(PathBuf::from("restore-apply-next.json")));
    }

    // Ensure restore apply-command options parse the intended preview command.
    #[test]
    fn parses_restore_apply_command_options() {
        let options = RestoreApplyCommandOptions::parse([
            OsString::from("--journal"),
            OsString::from("restore-apply-journal.json"),
            OsString::from("--dfx"),
            OsString::from("/tmp/dfx"),
            OsString::from("--network"),
            OsString::from("local"),
            OsString::from("--out"),
            OsString::from("restore-apply-command.json"),
            OsString::from("--require-command"),
        ])
        .expect("parse apply-command options");

        assert_eq!(options.journal, PathBuf::from("restore-apply-journal.json"));
        assert_eq!(options.dfx, "/tmp/dfx");
        assert_eq!(options.network.as_deref(), Some("local"));
        assert!(options.require_command);
        assert_eq!(
            options.out,
            Some(PathBuf::from("restore-apply-command.json"))
        );
    }

    // Ensure restore apply-claim options parse the intended journal command.
    #[test]
    fn parses_restore_apply_claim_options() {
        let options = RestoreApplyClaimOptions::parse([
            OsString::from("--journal"),
            OsString::from("restore-apply-journal.json"),
            OsString::from("--sequence"),
            OsString::from("0"),
            OsString::from("--updated-at"),
            OsString::from("2026-05-04T12:00:00Z"),
            OsString::from("--out"),
            OsString::from("restore-apply-journal.claimed.json"),
        ])
        .expect("parse apply-claim options");

        assert_eq!(options.journal, PathBuf::from("restore-apply-journal.json"));
        assert_eq!(options.sequence, Some(0));
        assert_eq!(options.updated_at.as_deref(), Some("2026-05-04T12:00:00Z"));
        assert_eq!(
            options.out,
            Some(PathBuf::from("restore-apply-journal.claimed.json"))
        );
    }

    // Ensure restore apply-unclaim options parse the intended journal command.
    #[test]
    fn parses_restore_apply_unclaim_options() {
        let options = RestoreApplyUnclaimOptions::parse([
            OsString::from("--journal"),
            OsString::from("restore-apply-journal.json"),
            OsString::from("--sequence"),
            OsString::from("0"),
            OsString::from("--updated-at"),
            OsString::from("2026-05-04T12:01:00Z"),
            OsString::from("--out"),
            OsString::from("restore-apply-journal.unclaimed.json"),
        ])
        .expect("parse apply-unclaim options");

        assert_eq!(options.journal, PathBuf::from("restore-apply-journal.json"));
        assert_eq!(options.sequence, Some(0));
        assert_eq!(options.updated_at.as_deref(), Some("2026-05-04T12:01:00Z"));
        assert_eq!(
            options.out,
            Some(PathBuf::from("restore-apply-journal.unclaimed.json"))
        );
    }

    // Ensure restore apply-mark options parse the intended journal update command.
    #[test]
    fn parses_restore_apply_mark_options() {
        let options = RestoreApplyMarkOptions::parse([
            OsString::from("--journal"),
            OsString::from("restore-apply-journal.json"),
            OsString::from("--sequence"),
            OsString::from("4"),
            OsString::from("--state"),
            OsString::from("failed"),
            OsString::from("--reason"),
            OsString::from("dfx-load-failed"),
            OsString::from("--updated-at"),
            OsString::from("2026-05-04T12:02:00Z"),
            OsString::from("--out"),
            OsString::from("restore-apply-journal.updated.json"),
            OsString::from("--require-pending"),
        ])
        .expect("parse apply-mark options");

        assert_eq!(options.journal, PathBuf::from("restore-apply-journal.json"));
        assert_eq!(options.sequence, 4);
        assert_eq!(options.state, RestoreApplyMarkState::Failed);
        assert_eq!(options.reason.as_deref(), Some("dfx-load-failed"));
        assert_eq!(options.updated_at.as_deref(), Some("2026-05-04T12:02:00Z"));
        assert!(options.require_pending);
        assert_eq!(
            options.out,
            Some(PathBuf::from("restore-apply-journal.updated.json"))
        );
    }

    // Ensure restore apply refuses non-dry-run execution while apply is scaffolded.
    #[test]
    fn restore_apply_requires_dry_run() {
        let err = RestoreApplyOptions::parse([
            OsString::from("--plan"),
            OsString::from("restore-plan.json"),
        ])
        .expect_err("apply without dry-run should fail");

        assert!(matches!(err, RestoreCommandError::ApplyRequiresDryRun));
    }

    // Ensure restore run refuses mutation while native execution is scaffolded.
    #[test]
    fn restore_run_requires_mode() {
        let err = RestoreRunOptions::parse([
            OsString::from("--journal"),
            OsString::from("restore-apply-journal.json"),
        ])
        .expect_err("restore run without dry-run should fail");

        assert!(matches!(err, RestoreCommandError::RestoreRunRequiresMode));
    }

    // Ensure restore run rejects ambiguous execution modes.
    #[test]
    fn restore_run_rejects_conflicting_modes() {
        let err = RestoreRunOptions::parse([
            OsString::from("--journal"),
            OsString::from("restore-apply-journal.json"),
            OsString::from("--dry-run"),
            OsString::from("--execute"),
            OsString::from("--unclaim-pending"),
        ])
        .expect_err("restore run should reject conflicting modes");

        assert!(matches!(
            err,
            RestoreCommandError::RestoreRunConflictingModes
        ));
    }

    // Ensure backup-dir restore planning reads the canonical layout manifest.
    #[test]
    fn plan_restore_reads_manifest_from_backup_dir() {
        let root = temp_dir("canic-cli-restore-plan-layout");
        let layout = BackupLayout::new(root.clone());
        layout
            .write_manifest(&valid_manifest())
            .expect("write manifest");

        let options = RestorePlanOptions {
            manifest: None,
            backup_dir: Some(root.clone()),
            mapping: None,
            out: None,
            require_verified: false,
            require_restore_ready: false,
        };

        let plan = plan_restore(&options).expect("plan restore");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(plan.backup_id, "backup-test");
        assert_eq!(plan.member_count, 2);
    }

    // Ensure restore planning has exactly one manifest source.
    #[test]
    fn parse_rejects_conflicting_manifest_sources() {
        let err = RestorePlanOptions::parse([
            OsString::from("--manifest"),
            OsString::from("manifest.json"),
            OsString::from("--backup-dir"),
            OsString::from("backups/run"),
        ])
        .expect_err("conflicting sources should fail");

        assert!(matches!(
            err,
            RestoreCommandError::ConflictingManifestSources
        ));
    }

    // Ensure verified planning requires the canonical backup layout source.
    #[test]
    fn parse_rejects_require_verified_with_manifest_source() {
        let err = RestorePlanOptions::parse([
            OsString::from("--manifest"),
            OsString::from("manifest.json"),
            OsString::from("--require-verified"),
        ])
        .expect_err("verification should require a backup layout");

        assert!(matches!(
            err,
            RestoreCommandError::RequireVerifiedNeedsBackupDir
        ));
    }

    // Ensure restore planning can require manifest, journal, and artifact integrity.
    #[test]
    fn plan_restore_requires_verified_backup_layout() {
        let root = temp_dir("canic-cli-restore-plan-verified");
        let layout = BackupLayout::new(root.clone());
        let manifest = valid_manifest();
        write_verified_layout(&root, &layout, &manifest);

        let options = RestorePlanOptions {
            manifest: None,
            backup_dir: Some(root.clone()),
            mapping: None,
            out: None,
            require_verified: true,
            require_restore_ready: false,
        };

        let plan = plan_restore(&options).expect("plan verified restore");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(plan.backup_id, "backup-test");
        assert_eq!(plan.member_count, 2);
    }

    // Ensure required verification fails before planning when the layout is incomplete.
    #[test]
    fn plan_restore_rejects_unverified_backup_layout() {
        let root = temp_dir("canic-cli-restore-plan-unverified");
        let layout = BackupLayout::new(root.clone());
        layout
            .write_manifest(&valid_manifest())
            .expect("write manifest");

        let options = RestorePlanOptions {
            manifest: None,
            backup_dir: Some(root.clone()),
            mapping: None,
            out: None,
            require_verified: true,
            require_restore_ready: false,
        };

        let err = plan_restore(&options).expect_err("missing journal should fail");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(err, RestoreCommandError::Persistence(_)));
    }

    // Ensure the CLI planning path validates manifests and applies mappings.
    #[test]
    fn plan_restore_reads_manifest_and_mapping() {
        let root = temp_dir("canic-cli-restore-plan");
        fs::create_dir_all(&root).expect("create temp root");
        let manifest_path = root.join("manifest.json");
        let mapping_path = root.join("mapping.json");

        fs::write(
            &manifest_path,
            serde_json::to_vec(&valid_manifest()).expect("serialize manifest"),
        )
        .expect("write manifest");
        fs::write(
            &mapping_path,
            json!({
                "members": [
                    {"source_canister": ROOT, "target_canister": ROOT},
                    {"source_canister": CHILD, "target_canister": MAPPED_CHILD}
                ]
            })
            .to_string(),
        )
        .expect("write mapping");

        let options = RestorePlanOptions {
            manifest: Some(manifest_path),
            backup_dir: None,
            mapping: Some(mapping_path),
            out: None,
            require_verified: false,
            require_restore_ready: false,
        };

        let plan = plan_restore(&options).expect("plan restore");

        fs::remove_dir_all(root).expect("remove temp root");
        let members = plan.ordered_members();
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].source_canister, ROOT);
        assert_eq!(members[1].target_canister, MAPPED_CHILD);
    }

    // Ensure restore-readiness gating happens after writing the plan artifact.
    #[test]
    fn run_restore_plan_require_restore_ready_writes_plan_then_fails() {
        let root = temp_dir("canic-cli-restore-plan-require-ready");
        fs::create_dir_all(&root).expect("create temp root");
        let manifest_path = root.join("manifest.json");
        let out_path = root.join("plan.json");

        fs::write(
            &manifest_path,
            serde_json::to_vec(&valid_manifest()).expect("serialize manifest"),
        )
        .expect("write manifest");

        let err = run([
            OsString::from("plan"),
            OsString::from("--manifest"),
            OsString::from(manifest_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
            OsString::from("--require-restore-ready"),
        ])
        .expect_err("restore readiness should be enforced");

        assert!(out_path.exists());
        let plan: RestorePlan =
            serde_json::from_slice(&fs::read(&out_path).expect("read plan")).expect("decode plan");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(!plan.readiness_summary.ready);
        assert!(matches!(
            err,
            RestoreCommandError::RestoreNotReady {
                reasons,
                ..
            } if reasons == [
                "missing-module-hash",
                "missing-wasm-hash",
                "missing-snapshot-checksum"
            ]
        ));
    }

    // Ensure restore-readiness gating accepts plans with complete provenance.
    #[test]
    fn run_restore_plan_require_restore_ready_accepts_ready_plan() {
        let root = temp_dir("canic-cli-restore-plan-ready");
        fs::create_dir_all(&root).expect("create temp root");
        let manifest_path = root.join("manifest.json");
        let out_path = root.join("plan.json");

        fs::write(
            &manifest_path,
            serde_json::to_vec(&restore_ready_manifest()).expect("serialize manifest"),
        )
        .expect("write manifest");

        run([
            OsString::from("plan"),
            OsString::from("--manifest"),
            OsString::from(manifest_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
            OsString::from("--require-restore-ready"),
        ])
        .expect("restore-ready plan should pass");

        let plan: RestorePlan =
            serde_json::from_slice(&fs::read(&out_path).expect("read plan")).expect("decode plan");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(plan.readiness_summary.ready);
        assert!(plan.readiness_summary.reasons.is_empty());
    }

    // Ensure restore status writes the initial planned execution journal.
    #[test]
    fn run_restore_status_writes_planned_status() {
        let root = temp_dir("canic-cli-restore-status");
        fs::create_dir_all(&root).expect("create temp root");
        let plan_path = root.join("restore-plan.json");
        let out_path = root.join("restore-status.json");
        let plan = RestorePlanner::plan(&restore_ready_manifest(), None).expect("build plan");

        fs::write(
            &plan_path,
            serde_json::to_vec(&plan).expect("serialize plan"),
        )
        .expect("write plan");

        run([
            OsString::from("status"),
            OsString::from("--plan"),
            OsString::from(plan_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
        ])
        .expect("write restore status");

        let status: RestoreStatus =
            serde_json::from_slice(&fs::read(&out_path).expect("read restore status"))
                .expect("decode restore status");
        let status_json: serde_json::Value = serde_json::to_value(&status).expect("encode status");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(status.status_version, 1);
        assert_eq!(status.backup_id.as_str(), "backup-test");
        assert!(status.ready);
        assert!(status.readiness_reasons.is_empty());
        assert_eq!(status.member_count, 2);
        assert_eq!(status.phase_count, 1);
        assert_eq!(status.planned_snapshot_uploads, 2);
        assert_eq!(status.planned_snapshot_loads, 2);
        assert_eq!(status.planned_code_reinstalls, 2);
        assert_eq!(status.planned_verification_checks, 2);
        assert_eq!(status.planned_operations, 8);
        assert_eq!(status.phases[0].members[0].source_canister, ROOT);
        assert_eq!(status_json["phases"][0]["members"][0]["state"], "planned");
    }

    // Ensure restore apply dry-run writes ordered operations from plan and status.
    #[test]
    fn run_restore_apply_dry_run_writes_operations() {
        let root = temp_dir("canic-cli-restore-apply-dry-run");
        fs::create_dir_all(&root).expect("create temp root");
        let plan_path = root.join("restore-plan.json");
        let status_path = root.join("restore-status.json");
        let out_path = root.join("restore-apply-dry-run.json");
        let plan = RestorePlanner::plan(&restore_ready_manifest(), None).expect("build plan");
        let status = RestoreStatus::from_plan(&plan);

        fs::write(
            &plan_path,
            serde_json::to_vec(&plan).expect("serialize plan"),
        )
        .expect("write plan");
        fs::write(
            &status_path,
            serde_json::to_vec(&status).expect("serialize status"),
        )
        .expect("write status");

        run([
            OsString::from("apply"),
            OsString::from("--plan"),
            OsString::from(plan_path.as_os_str()),
            OsString::from("--status"),
            OsString::from(status_path.as_os_str()),
            OsString::from("--dry-run"),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
        ])
        .expect("write apply dry-run");

        let dry_run: RestoreApplyDryRun =
            serde_json::from_slice(&fs::read(&out_path).expect("read dry-run"))
                .expect("decode dry-run");
        let dry_run_json: serde_json::Value =
            serde_json::to_value(&dry_run).expect("encode dry-run");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(dry_run.dry_run_version, 1);
        assert_eq!(dry_run.backup_id.as_str(), "backup-test");
        assert!(dry_run.ready);
        assert!(dry_run.status_supplied);
        assert_eq!(dry_run.member_count, 2);
        assert_eq!(dry_run.phase_count, 1);
        assert_eq!(dry_run.planned_snapshot_uploads, 2);
        assert_eq!(dry_run.planned_operations, 8);
        assert_eq!(dry_run.rendered_operations, 8);
        assert_eq!(dry_run_json["operation_counts"]["snapshot_uploads"], 2);
        assert_eq!(dry_run_json["operation_counts"]["snapshot_loads"], 2);
        assert_eq!(dry_run_json["operation_counts"]["code_reinstalls"], 2);
        assert_eq!(dry_run_json["operation_counts"]["member_verifications"], 2);
        assert_eq!(dry_run_json["operation_counts"]["fleet_verifications"], 0);
        assert_eq!(
            dry_run_json["operation_counts"]["verification_operations"],
            2
        );
        assert_eq!(
            dry_run_json["phases"][0]["operations"][0]["operation"],
            "upload-snapshot"
        );
        assert_eq!(
            dry_run_json["phases"][0]["operations"][3]["operation"],
            "verify-member"
        );
        assert_eq!(
            dry_run_json["phases"][0]["operations"][3]["verification_kind"],
            "status"
        );
        assert_eq!(
            dry_run_json["phases"][0]["operations"][3]["verification_method"],
            serde_json::Value::Null
        );
    }

    // Ensure restore apply dry-run can validate artifacts under a backup directory.
    #[test]
    fn run_restore_apply_dry_run_validates_backup_dir_artifacts() {
        let root = temp_dir("canic-cli-restore-apply-artifacts");
        fs::create_dir_all(&root).expect("create temp root");
        let plan_path = root.join("restore-plan.json");
        let out_path = root.join("restore-apply-dry-run.json");
        let journal_path = root.join("restore-apply-journal.json");
        let status_path = root.join("restore-apply-status.json");
        let mut manifest = restore_ready_manifest();
        write_manifest_artifacts(&root, &mut manifest);
        let plan = RestorePlanner::plan(&manifest, None).expect("build plan");

        fs::write(
            &plan_path,
            serde_json::to_vec(&plan).expect("serialize plan"),
        )
        .expect("write plan");

        run([
            OsString::from("apply"),
            OsString::from("--plan"),
            OsString::from(plan_path.as_os_str()),
            OsString::from("--backup-dir"),
            OsString::from(root.as_os_str()),
            OsString::from("--dry-run"),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
            OsString::from("--journal-out"),
            OsString::from(journal_path.as_os_str()),
        ])
        .expect("write apply dry-run");
        run([
            OsString::from("apply-status"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(status_path.as_os_str()),
        ])
        .expect("write apply status");

        let dry_run: RestoreApplyDryRun =
            serde_json::from_slice(&fs::read(&out_path).expect("read dry-run"))
                .expect("decode dry-run");
        let validation = dry_run
            .artifact_validation
            .expect("artifact validation should be present");
        let journal_json: serde_json::Value =
            serde_json::from_slice(&fs::read(&journal_path).expect("read journal"))
                .expect("decode journal");
        let status_json: serde_json::Value =
            serde_json::from_slice(&fs::read(&status_path).expect("read apply status"))
                .expect("decode apply status");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(validation.checked_members, 2);
        assert!(validation.artifacts_present);
        assert!(validation.checksums_verified);
        assert_eq!(validation.members_with_expected_checksums, 2);
        assert_eq!(journal_json["ready"], true);
        assert_eq!(journal_json["operation_count"], 8);
        assert_eq!(journal_json["operation_counts"]["snapshot_uploads"], 2);
        assert_eq!(journal_json["operation_counts"]["snapshot_loads"], 2);
        assert_eq!(journal_json["operation_counts"]["code_reinstalls"], 2);
        assert_eq!(journal_json["operation_counts"]["member_verifications"], 2);
        assert_eq!(journal_json["operation_counts"]["fleet_verifications"], 0);
        assert_eq!(
            journal_json["operation_counts"]["verification_operations"],
            2
        );
        assert_eq!(journal_json["ready_operations"], 8);
        assert_eq!(journal_json["blocked_operations"], 0);
        assert_eq!(journal_json["operations"][0]["state"], "ready");
        assert_eq!(status_json["ready"], true);
        assert_eq!(status_json["operation_count"], 8);
        assert_eq!(status_json["operation_counts"]["snapshot_uploads"], 2);
        assert_eq!(status_json["operation_counts"]["snapshot_loads"], 2);
        assert_eq!(status_json["operation_counts"]["code_reinstalls"], 2);
        assert_eq!(status_json["operation_counts"]["member_verifications"], 2);
        assert_eq!(status_json["operation_counts"]["fleet_verifications"], 0);
        assert_eq!(
            status_json["operation_counts"]["verification_operations"],
            2
        );
        assert_eq!(status_json["operation_counts_supplied"], true);
        assert_eq!(status_json["progress"]["operation_count"], 8);
        assert_eq!(status_json["progress"]["completed_operations"], 0);
        assert_eq!(status_json["progress"]["remaining_operations"], 8);
        assert_eq!(status_json["progress"]["transitionable_operations"], 8);
        assert_eq!(status_json["progress"]["attention_operations"], 0);
        assert_eq!(status_json["progress"]["completion_basis_points"], 0);
        assert_eq!(status_json["next_ready_sequence"], 0);
        assert_eq!(status_json["next_ready_operation"], "upload-snapshot");
    }

    // Ensure apply-status rejects structurally inconsistent journals.
    #[test]
    fn run_restore_apply_status_rejects_invalid_journal() {
        let root = temp_dir("canic-cli-restore-apply-status-invalid");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-apply-status.json");
        let plan = RestorePlanner::plan(&restore_ready_manifest(), None).expect("build plan");
        let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run");
        let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
        journal.operation_count += 1;

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        let err = run([
            OsString::from("apply-status"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
        ])
        .expect_err("invalid journal should fail");

        assert!(!out_path.exists());
        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyJournal(RestoreApplyJournalError::CountMismatch {
                field: "operation_count",
                ..
            })
        ));
    }

    // Ensure apply-status can fail closed after writing status for pending work.
    #[test]
    fn run_restore_apply_status_require_no_pending_writes_status_then_fails() {
        let fixture = RestoreCliFixture::new(
            "canic-cli-restore-apply-status-pending",
            "restore-apply-status.json",
        );
        let mut journal = ready_apply_journal();
        journal
            .mark_next_operation_pending_at(Some("2026-05-04T12:00:00Z".to_string()))
            .expect("claim operation");
        fixture.write_journal(&journal);

        let err = fixture
            .run_apply_status(&["--require-no-pending"])
            .expect_err("pending operation should fail requirement");

        assert!(fixture.out_path.exists());
        let status: RestoreApplyJournalStatus = fixture.read_out("read apply status");

        assert_eq!(status.pending_operations, 1);
        assert_eq!(status.next_transition_sequence, Some(0));
        assert_eq!(status.pending_summary.pending_operations, 1);
        assert_eq!(status.pending_summary.pending_sequence, Some(0));
        assert_eq!(
            status.pending_summary.pending_updated_at.as_deref(),
            Some("2026-05-04T12:00:00Z")
        );
        assert!(status.pending_summary.pending_updated_at_known);
        assert_eq!(
            status.next_transition_updated_at.as_deref(),
            Some("2026-05-04T12:00:00Z")
        );
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyPending {
                pending_operations: 1,
                next_transition_sequence: Some(0),
                ..
            }
        ));
    }

    // Ensure apply-status can fail closed when pending work is older than a cutoff.
    #[test]
    fn run_restore_apply_status_require_no_pending_before_writes_status_then_fails() {
        let fixture = RestoreCliFixture::new(
            "canic-cli-restore-apply-status-stale-pending",
            "restore-apply-status.json",
        );
        let mut journal = ready_apply_journal();
        journal
            .mark_next_operation_pending_at(Some("2026-05-04T12:00:00Z".to_string()))
            .expect("claim operation");
        fixture.write_journal(&journal);

        let err = fixture
            .run_apply_status(&["--require-no-pending-before", "2026-05-05T12:00:00Z"])
            .expect_err("stale pending operation should fail requirement");

        let status: RestoreApplyJournalStatus = fixture.read_out("read apply status");

        assert_eq!(status.pending_summary.pending_sequence, Some(0));
        assert_eq!(
            status.pending_summary.pending_updated_at.as_deref(),
            Some("2026-05-04T12:00:00Z")
        );
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyPendingStale {
                cutoff_updated_at,
                pending_sequence: Some(0),
                pending_updated_at,
                ..
            } if cutoff_updated_at == "2026-05-05T12:00:00Z"
                && pending_updated_at.as_deref() == Some("2026-05-04T12:00:00Z")
        ));
    }

    // Ensure apply-status can fail closed on an unexpected progress summary.
    #[test]
    fn run_restore_apply_status_require_progress_writes_status_then_fails() {
        let fixture = RestoreCliFixture::new(
            "canic-cli-restore-apply-status-progress",
            "restore-apply-status.json",
        );
        let journal = ready_apply_journal();
        fixture.write_journal(&journal);

        let err = fixture
            .run_apply_status(&[
                "--require-remaining-count",
                "7",
                "--require-attention-count",
                "0",
                "--require-completion-basis-points",
                "0",
            ])
            .expect_err("remaining progress mismatch should fail requirement");

        let status: RestoreApplyJournalStatus = fixture.read_out("read apply status");

        assert_eq!(status.progress.remaining_operations, 8);
        assert_eq!(status.progress.attention_operations, 0);
        assert_eq!(status.progress.completion_basis_points, 0);
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyProgressMismatch {
                field: "remaining_operations",
                expected: 7,
                actual: 8,
                ..
            }
        ));
    }

    // Ensure apply-status can fail closed after writing status for unready work.
    #[test]
    fn run_restore_apply_status_require_ready_writes_status_then_fails() {
        let root = temp_dir("canic-cli-restore-apply-status-ready");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-apply-status.json");
        let plan = RestorePlanner::plan(&valid_manifest(), None).expect("build plan");
        let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run");
        let journal = RestoreApplyJournal::from_dry_run(&dry_run);

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        let err = run([
            OsString::from("apply-status"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
            OsString::from("--require-ready"),
        ])
        .expect_err("unready journal should fail requirement");

        let status: RestoreApplyJournalStatus =
            serde_json::from_slice(&fs::read(&out_path).expect("read apply status"))
                .expect("decode apply status");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(!status.ready);
        assert_eq!(status.blocked_operations, status.operation_count);
        assert!(
            status
                .blocked_reasons
                .contains(&"missing-snapshot-checksum".to_string())
        );
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyNotReady { reasons, .. }
                if reasons.contains(&"missing-snapshot-checksum".to_string())
        ));
    }

    // Ensure apply-report writes the operator-focused journal summary.
    #[test]
    fn run_restore_apply_report_writes_attention_summary() {
        let root = temp_dir("canic-cli-restore-apply-report");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-apply-report.json");
        let mut journal = ready_apply_journal();
        journal
            .mark_operation_failed_at(
                0,
                "dfx-upload-failed".to_string(),
                Some("2026-05-05T12:00:00Z".to_string()),
            )
            .expect("mark failed operation");
        journal
            .mark_next_operation_pending_at(Some("2026-05-05T12:01:00Z".to_string()))
            .expect("mark pending operation");

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        run([
            OsString::from("apply-report"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
        ])
        .expect("write apply report");

        let report: RestoreApplyJournalReport =
            serde_json::from_slice(&fs::read(&out_path).expect("read apply report"))
                .expect("decode apply report");
        let report_json: serde_json::Value =
            serde_json::to_value(&report).expect("encode apply report");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(report.backup_id, "backup-test");
        assert!(report.attention_required);
        assert_eq!(report.failed_operations, 1);
        assert_eq!(report.pending_operations, 1);
        assert_eq!(report.operation_counts.snapshot_uploads, 2);
        assert_eq!(report.operation_counts.snapshot_loads, 2);
        assert_eq!(report.operation_counts.code_reinstalls, 2);
        assert_eq!(report.operation_counts.member_verifications, 2);
        assert_eq!(report.operation_counts.fleet_verifications, 0);
        assert_eq!(report.operation_counts.verification_operations, 2);
        assert!(report.operation_counts_supplied);
        assert_eq!(report.progress.operation_count, 8);
        assert_eq!(report.progress.completed_operations, 0);
        assert_eq!(report.progress.remaining_operations, 8);
        assert_eq!(report.progress.transitionable_operations, 7);
        assert_eq!(report.progress.attention_operations, 2);
        assert_eq!(report.progress.completion_basis_points, 0);
        assert_eq!(report.pending_summary.pending_operations, 1);
        assert_eq!(report.pending_summary.pending_sequence, Some(1));
        assert_eq!(
            report.pending_summary.pending_updated_at.as_deref(),
            Some("2026-05-05T12:01:00Z")
        );
        assert!(report.pending_summary.pending_updated_at_known);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.pending.len(), 1);
        assert_eq!(report.failed[0].sequence, 0);
        assert_eq!(report.pending[0].sequence, 1);
        assert_eq!(
            report.next_transition.as_ref().map(|op| op.sequence),
            Some(1)
        );
        assert_eq!(report_json["outcome"], "failed");
        assert_eq!(report_json["failed"][0]["reasons"][0], "dfx-upload-failed");
    }

    // Ensure apply-report can fail closed on an unexpected progress summary.
    #[test]
    fn run_restore_apply_report_require_progress_writes_report_then_fails() {
        let fixture = RestoreCliFixture::new(
            "canic-cli-restore-apply-report-progress",
            "restore-apply-report.json",
        );
        let journal = ready_apply_journal();
        fixture.write_journal(&journal);

        let err = fixture
            .run_apply_report(&[
                "--require-remaining-count",
                "8",
                "--require-attention-count",
                "1",
                "--require-completion-basis-points",
                "0",
            ])
            .expect_err("attention progress mismatch should fail requirement");

        let report: RestoreApplyJournalReport = fixture.read_out("read apply report");

        assert_eq!(report.progress.remaining_operations, 8);
        assert_eq!(report.progress.attention_operations, 0);
        assert_eq!(report.progress.completion_basis_points, 0);
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyProgressMismatch {
                field: "attention_operations",
                expected: 1,
                actual: 0,
                ..
            }
        ));
    }

    // Ensure apply-report can fail closed when pending work is older than a cutoff.
    #[test]
    fn run_restore_apply_report_require_no_pending_before_writes_report_then_fails() {
        let fixture = RestoreCliFixture::new(
            "canic-cli-restore-apply-report-stale-pending",
            "restore-apply-report.json",
        );
        let mut journal = ready_apply_journal();
        journal
            .mark_next_operation_pending_at(Some("2026-05-04T12:00:00Z".to_string()))
            .expect("mark pending operation");
        fixture.write_journal(&journal);

        let err = fixture
            .run_apply_report(&["--require-no-pending-before", "2026-05-05T12:00:00Z"])
            .expect_err("stale pending report should fail requirement");

        let report: RestoreApplyJournalReport = fixture.read_out("read apply report");

        assert_eq!(report.pending_summary.pending_sequence, Some(0));
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyPendingStale {
                pending_sequence: Some(0),
                ..
            }
        ));
    }

    // Ensure restore run writes a native no-mutation runner preview.
    #[test]
    fn run_restore_run_dry_run_writes_native_runner_preview() {
        let root = temp_dir("canic-cli-restore-run-dry-run");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-run-dry-run.json");
        let journal = ready_apply_journal();

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        run([
            OsString::from("run"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--dry-run"),
            OsString::from("--dfx"),
            OsString::from("/tmp/dfx"),
            OsString::from("--network"),
            OsString::from("local"),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
        ])
        .expect("write restore run dry-run");

        let dry_run: serde_json::Value =
            serde_json::from_slice(&fs::read(&out_path).expect("read dry-run"))
                .expect("decode dry-run");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(dry_run["run_version"], 1);
        assert_eq!(dry_run["backup_id"], "backup-test");
        assert_eq!(dry_run["run_mode"], "dry-run");
        assert_eq!(dry_run["dry_run"], true);
        assert_eq!(dry_run["ready"], true);
        assert_eq!(dry_run["complete"], false);
        assert_eq!(dry_run["attention_required"], false);
        assert_eq!(dry_run["operation_counts"]["snapshot_uploads"], 2);
        assert_eq!(dry_run["operation_counts"]["snapshot_loads"], 2);
        assert_eq!(dry_run["operation_counts"]["code_reinstalls"], 2);
        assert_eq!(dry_run["operation_counts"]["member_verifications"], 2);
        assert_eq!(dry_run["operation_counts"]["fleet_verifications"], 0);
        assert_eq!(dry_run["operation_counts"]["verification_operations"], 2);
        assert_eq!(dry_run["operation_counts_supplied"], true);
        assert_eq!(dry_run["progress"]["operation_count"], 8);
        assert_eq!(dry_run["progress"]["completed_operations"], 0);
        assert_eq!(dry_run["progress"]["remaining_operations"], 8);
        assert_eq!(dry_run["progress"]["transitionable_operations"], 8);
        assert_eq!(dry_run["progress"]["attention_operations"], 0);
        assert_eq!(dry_run["progress"]["completion_basis_points"], 0);
        assert_eq!(dry_run["pending_summary"]["pending_operations"], 0);
        assert_eq!(
            dry_run["pending_summary"]["pending_operation_available"],
            false
        );
        assert_eq!(dry_run["operation_receipt_count"], 0);
        assert_eq!(dry_run["operation_receipt_summary"]["total_receipts"], 0);
        assert_eq!(dry_run["operation_receipt_summary"]["command_completed"], 0);
        assert_eq!(dry_run["operation_receipt_summary"]["command_failed"], 0);
        assert_eq!(dry_run["operation_receipt_summary"]["pending_recovered"], 0);
        assert_eq!(dry_run["stopped_reason"], "preview");
        assert_eq!(dry_run["next_action"], "rerun");
        assert_eq!(dry_run["operation_available"], true);
        assert_eq!(dry_run["command_available"], true);
        assert_eq!(dry_run["next_transition"]["sequence"], 0);
        assert_eq!(dry_run["command"]["program"], "/tmp/dfx");
        assert_eq!(
            dry_run["command"]["args"],
            json!([
                "canister",
                "--network",
                "local",
                "snapshot",
                "upload",
                "--dir",
                "artifacts/root",
                ROOT
            ])
        );
        assert_eq!(dry_run["command"]["mutates"], true);
    }

    // Ensure restore run can recover one interrupted pending operation.
    #[test]
    fn run_restore_run_unclaim_pending_marks_operation_ready() {
        let root = temp_dir("canic-cli-restore-run-unclaim-pending");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-run.json");
        let mut journal = ready_apply_journal();
        journal
            .mark_next_operation_pending_at(Some("2026-05-05T12:01:00Z".to_string()))
            .expect("mark pending operation");

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        run([
            OsString::from("run"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--unclaim-pending"),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
        ])
        .expect("unclaim pending operation");

        let run_summary: serde_json::Value =
            serde_json::from_slice(&fs::read(&out_path).expect("read run summary"))
                .expect("decode run summary");
        let updated: RestoreApplyJournal =
            serde_json::from_slice(&fs::read(&journal_path).expect("read updated journal"))
                .expect("decode updated journal");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(run_summary["run_mode"], "unclaim-pending");
        assert_eq!(run_summary["unclaim_pending"], true);
        assert_eq!(run_summary["stopped_reason"], "recovered-pending");
        assert_eq!(run_summary["next_action"], "rerun");
        assert_eq!(run_summary["recovered_operation"]["sequence"], 0);
        assert_eq!(run_summary["recovered_operation"]["state"], "pending");
        assert_eq!(run_summary["operation_receipt_count"], 1);
        assert_eq!(
            run_summary["operation_receipt_summary"]["total_receipts"],
            1
        );
        assert_eq!(
            run_summary["operation_receipt_summary"]["command_completed"],
            0
        );
        assert_eq!(
            run_summary["operation_receipt_summary"]["command_failed"],
            0
        );
        assert_eq!(
            run_summary["operation_receipt_summary"]["pending_recovered"],
            1
        );
        assert_eq!(
            run_summary["operation_receipts"][0]["event"],
            "pending-recovered"
        );
        assert_eq!(run_summary["operation_receipts"][0]["sequence"], 0);
        assert_eq!(run_summary["operation_receipts"][0]["state"], "ready");
        assert_eq!(
            run_summary["operation_receipts"][0]["updated_at"],
            "unknown"
        );
        assert_eq!(run_summary["pending_operations"], 0);
        assert_eq!(run_summary["ready_operations"], 8);
        assert_eq!(run_summary["attention_required"], false);
        assert_eq!(updated.pending_operations, 0);
        assert_eq!(updated.ready_operations, 8);
        assert_eq!(
            updated.operations[0].state,
            RestoreApplyOperationState::Ready
        );
    }

    // Ensure restore run execute claims and completes one generated command.
    #[test]
    fn run_restore_run_execute_marks_completed_operation() {
        let root = temp_dir("canic-cli-restore-run-execute");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-run.json");
        let journal = ready_apply_journal();

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        run([
            OsString::from("run"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--execute"),
            OsString::from("--dfx"),
            OsString::from("/bin/true"),
            OsString::from("--max-steps"),
            OsString::from("1"),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
        ])
        .expect("execute one restore run step");

        let run_summary: serde_json::Value =
            serde_json::from_slice(&fs::read(&out_path).expect("read run summary"))
                .expect("decode run summary");
        let updated: RestoreApplyJournal =
            serde_json::from_slice(&fs::read(&journal_path).expect("read updated journal"))
                .expect("decode updated journal");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(run_summary["run_mode"], "execute");
        assert_eq!(run_summary["execute"], true);
        assert_eq!(run_summary["dry_run"], false);
        assert_eq!(run_summary["max_steps_reached"], true);
        assert_eq!(run_summary["stopped_reason"], "max-steps-reached");
        assert_eq!(run_summary["next_action"], "rerun");
        assert_eq!(run_summary["executed_operation_count"], 1);
        assert_eq!(run_summary["operation_receipt_count"], 1);
        assert_eq!(
            run_summary["operation_receipt_summary"]["total_receipts"],
            1
        );
        assert_eq!(
            run_summary["operation_receipt_summary"]["command_completed"],
            1
        );
        assert_eq!(
            run_summary["operation_receipt_summary"]["command_failed"],
            0
        );
        assert_eq!(
            run_summary["operation_receipt_summary"]["pending_recovered"],
            0
        );
        assert_eq!(run_summary["executed_operations"][0]["sequence"], 0);
        assert_eq!(
            run_summary["executed_operations"][0]["command"]["program"],
            "/bin/true"
        );
        assert_eq!(
            run_summary["operation_receipts"][0]["event"],
            "command-completed"
        );
        assert_eq!(run_summary["operation_receipts"][0]["sequence"], 0);
        assert_eq!(run_summary["operation_receipts"][0]["state"], "completed");
        assert_eq!(
            run_summary["operation_receipts"][0]["command"]["program"],
            "/bin/true"
        );
        assert_eq!(run_summary["operation_receipts"][0]["status"], "0");
        assert_eq!(
            run_summary["operation_receipts"][0]["updated_at"],
            "unknown"
        );
        assert_eq!(updated.completed_operations, 1);
        assert_eq!(updated.pending_operations, 0);
        assert_eq!(updated.failed_operations, 0);
        assert_eq!(
            updated.operations[0].state,
            RestoreApplyOperationState::Completed
        );
    }

    // Ensure restore run can fail closed after writing an incomplete summary.
    #[test]
    fn run_restore_run_require_complete_writes_summary_then_fails() {
        let root = temp_dir("canic-cli-restore-run-require-complete");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-run.json");
        let journal = ready_apply_journal();

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        let err = run([
            OsString::from("run"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--execute"),
            OsString::from("--dfx"),
            OsString::from("/bin/true"),
            OsString::from("--max-steps"),
            OsString::from("1"),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
            OsString::from("--require-complete"),
        ])
        .expect_err("incomplete run should fail requirement");

        let run_summary: serde_json::Value =
            serde_json::from_slice(&fs::read(&out_path).expect("read run summary"))
                .expect("decode run summary");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(run_summary["executed_operation_count"], 1);
        assert_eq!(run_summary["complete"], false);
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyIncomplete {
                completed_operations: 1,
                operation_count: 8,
                ..
            }
        ));
    }

    // Ensure restore run execute records failed command exits in the journal.
    #[test]
    fn run_restore_run_execute_marks_failed_operation() {
        let root = temp_dir("canic-cli-restore-run-execute-failed");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-run.json");
        let journal = ready_apply_journal();

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        let err = run([
            OsString::from("run"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--execute"),
            OsString::from("--dfx"),
            OsString::from("/bin/false"),
            OsString::from("--max-steps"),
            OsString::from("1"),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
        ])
        .expect_err("failing runner command should fail");

        let run_summary: serde_json::Value =
            serde_json::from_slice(&fs::read(&out_path).expect("read run summary"))
                .expect("decode run summary");
        let updated: RestoreApplyJournal =
            serde_json::from_slice(&fs::read(&journal_path).expect("read updated journal"))
                .expect("decode updated journal");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(
            err,
            RestoreCommandError::RestoreRunCommandFailed {
                sequence: 0,
                status,
            } if status == "1"
        ));
        assert_eq!(updated.failed_operations, 1);
        assert_eq!(updated.pending_operations, 0);
        assert_eq!(
            updated.operations[0].state,
            RestoreApplyOperationState::Failed
        );
        assert_eq!(run_summary["execute"], true);
        assert_eq!(run_summary["attention_required"], true);
        assert_eq!(run_summary["outcome"], "failed");
        assert_eq!(run_summary["stopped_reason"], "command-failed");
        assert_eq!(run_summary["next_action"], "inspect-failed-operation");
        assert_eq!(run_summary["executed_operation_count"], 1);
        assert_eq!(run_summary["operation_receipt_count"], 1);
        assert_eq!(
            run_summary["operation_receipt_summary"]["total_receipts"],
            1
        );
        assert_eq!(
            run_summary["operation_receipt_summary"]["command_completed"],
            0
        );
        assert_eq!(
            run_summary["operation_receipt_summary"]["command_failed"],
            1
        );
        assert_eq!(
            run_summary["operation_receipt_summary"]["pending_recovered"],
            0
        );
        assert_eq!(run_summary["executed_operations"][0]["state"], "failed");
        assert_eq!(run_summary["executed_operations"][0]["status"], "1");
        assert_eq!(
            run_summary["operation_receipts"][0]["event"],
            "command-failed"
        );
        assert_eq!(run_summary["operation_receipts"][0]["sequence"], 0);
        assert_eq!(run_summary["operation_receipts"][0]["state"], "failed");
        assert_eq!(
            run_summary["operation_receipts"][0]["command"]["program"],
            "/bin/false"
        );
        assert_eq!(run_summary["operation_receipts"][0]["status"], "1");
        assert_eq!(
            run_summary["operation_receipts"][0]["updated_at"],
            "unknown"
        );
        assert_eq!(
            updated.operations[0].blocking_reasons,
            vec!["runner-command-exit-1".to_string()]
        );
    }

    // Ensure restore run can fail closed after writing an attention summary.
    #[test]
    fn run_restore_run_require_no_attention_writes_summary_then_fails() {
        let fixture = RestoreCliFixture::new(
            "canic-cli-restore-run-require-attention",
            "restore-run.json",
        );
        let mut journal = ready_apply_journal();
        journal
            .mark_next_operation_pending_at(Some("2026-05-05T12:01:00Z".to_string()))
            .expect("mark pending operation");
        fixture.write_journal(&journal);

        let err = fixture
            .run_restore_run(&["--dry-run", "--require-no-attention"])
            .expect_err("attention run should fail requirement");

        let run_summary: serde_json::Value = fixture.read_out("read run summary");

        assert_eq!(run_summary["attention_required"], true);
        assert_eq!(run_summary["outcome"], "pending");
        assert_eq!(run_summary["stopped_reason"], "pending");
        assert_eq!(run_summary["next_action"], "unclaim-pending");
        assert_eq!(run_summary["pending_summary"]["pending_sequence"], 0);
        assert_eq!(
            run_summary["pending_summary"]["pending_updated_at"],
            "2026-05-05T12:01:00Z"
        );
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyReportNeedsAttention {
                outcome: canic_backup::restore::RestoreApplyReportOutcome::Pending,
                ..
            }
        ));
    }

    // Ensure restore run can fail closed when pending work is older than a cutoff.
    #[test]
    fn run_restore_run_require_no_pending_before_writes_summary_then_fails() {
        let fixture = RestoreCliFixture::new(
            "canic-cli-restore-run-require-stale-pending",
            "restore-run.json",
        );
        let mut journal = ready_apply_journal();
        journal
            .mark_next_operation_pending_at(Some("2026-05-04T12:00:00Z".to_string()))
            .expect("mark pending operation");
        fixture.write_journal(&journal);

        let err = fixture
            .run_restore_run(&[
                "--dry-run",
                "--require-no-pending-before",
                "2026-05-05T12:00:00Z",
            ])
            .expect_err("stale pending run should fail requirement");

        let run_summary: serde_json::Value = fixture.read_out("read run summary");

        assert_eq!(run_summary["pending_summary"]["pending_sequence"], 0);
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyPendingStale {
                pending_sequence: Some(0),
                ..
            }
        ));
    }

    // Ensure restore run can fail closed on an unexpected run mode.
    #[test]
    fn run_restore_run_require_run_mode_writes_summary_then_fails() {
        let fixture =
            RestoreCliFixture::new("canic-cli-restore-run-require-run-mode", "restore-run.json");
        let journal = ready_apply_journal();
        fixture.write_journal(&journal);

        let err = fixture
            .run_restore_run(&["--dry-run", "--require-run-mode", "execute"])
            .expect_err("run mode mismatch should fail requirement");

        let run_summary: serde_json::Value = fixture.read_out("read run summary");

        assert_eq!(run_summary["run_mode"], "dry-run");
        assert!(matches!(
            err,
            RestoreCommandError::RestoreRunModeMismatch {
                expected,
                actual,
                ..
            } if expected == "execute" && actual == "dry-run"
        ));
    }

    // Ensure restore run can fail closed on an unexpected executed operation count.
    #[test]
    fn run_restore_run_require_executed_count_writes_summary_then_fails() {
        let root = temp_dir("canic-cli-restore-run-require-executed-count");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-run.json");
        let journal = ready_apply_journal();

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        let err = run([
            OsString::from("run"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--execute"),
            OsString::from("--dfx"),
            OsString::from("/bin/true"),
            OsString::from("--max-steps"),
            OsString::from("1"),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
            OsString::from("--require-executed-count"),
            OsString::from("2"),
        ])
        .expect_err("executed count mismatch should fail requirement");

        let run_summary: serde_json::Value =
            serde_json::from_slice(&fs::read(&out_path).expect("read run summary"))
                .expect("decode run summary");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(run_summary["executed_operation_count"], 1);
        assert!(matches!(
            err,
            RestoreCommandError::RestoreRunExecutedCountMismatch {
                expected: 2,
                actual: 1,
                ..
            }
        ));
    }

    // Ensure restore run can fail closed on an unexpected operation receipt count.
    #[test]
    fn run_restore_run_require_receipt_count_writes_summary_then_fails() {
        let fixture = RestoreCliFixture::new(
            "canic-cli-restore-run-require-receipt-count",
            "restore-run.json",
        );
        let journal = ready_apply_journal();
        fixture.write_journal(&journal);

        let err = fixture
            .run_restore_run(&[
                "--execute",
                "--dfx",
                "/bin/true",
                "--max-steps",
                "1",
                "--require-receipt-count",
                "2",
            ])
            .expect_err("receipt count mismatch should fail requirement");

        let run_summary: serde_json::Value = fixture.read_out("read run summary");

        assert_eq!(run_summary["operation_receipt_count"], 1);
        assert_eq!(
            run_summary["operation_receipt_summary"]["total_receipts"],
            1
        );
        assert!(matches!(
            err,
            RestoreCommandError::RestoreRunReceiptCountMismatch {
                expected: 2,
                actual: 1,
                ..
            }
        ));
    }

    // Ensure restore run can fail closed on an unexpected receipt-kind count.
    #[test]
    fn run_restore_run_require_receipt_kind_count_writes_summary_then_fails() {
        let fixture = RestoreCliFixture::new(
            "canic-cli-restore-run-require-receipt-kind-count",
            "restore-run.json",
        );
        let journal = ready_apply_journal();
        fixture.write_journal(&journal);

        let err = fixture
            .run_restore_run(&[
                "--execute",
                "--dfx",
                "/bin/true",
                "--max-steps",
                "1",
                "--require-failed-receipt-count",
                "1",
            ])
            .expect_err("receipt kind count mismatch should fail requirement");

        let run_summary: serde_json::Value = fixture.read_out("read run summary");

        assert_eq!(
            run_summary["operation_receipt_summary"]["command_failed"],
            0
        );
        assert_eq!(
            run_summary["operation_receipt_summary"]["command_completed"],
            1
        );
        assert!(matches!(
            err,
            RestoreCommandError::RestoreRunReceiptKindCountMismatch {
                receipt_kind: "command-failed",
                expected: 1,
                actual: 0,
                ..
            }
        ));
    }

    // Ensure restore run can fail closed on an unexpected progress summary.
    #[test]
    fn run_restore_run_require_progress_writes_summary_then_fails() {
        let root = temp_dir("canic-cli-restore-run-require-progress");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-run.json");
        let journal = ready_apply_journal();

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        let err = run([
            OsString::from("run"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--execute"),
            OsString::from("--dfx"),
            OsString::from("/bin/true"),
            OsString::from("--max-steps"),
            OsString::from("1"),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
            OsString::from("--require-remaining-count"),
            OsString::from("7"),
            OsString::from("--require-attention-count"),
            OsString::from("0"),
            OsString::from("--require-completion-basis-points"),
            OsString::from("0"),
        ])
        .expect_err("completion progress mismatch should fail requirement");

        let run_summary: serde_json::Value =
            serde_json::from_slice(&fs::read(&out_path).expect("read run summary"))
                .expect("decode run summary");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(run_summary["progress"]["remaining_operations"], 7);
        assert_eq!(run_summary["progress"]["attention_operations"], 0);
        assert_eq!(run_summary["progress"]["completion_basis_points"], 1250);
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyProgressMismatch {
                field: "completion_basis_points",
                expected: 0,
                actual: 1250,
                ..
            }
        ));
    }

    // Ensure restore run can fail closed on an unexpected stopped reason.
    #[test]
    fn run_restore_run_require_stopped_reason_writes_summary_then_fails() {
        let root = temp_dir("canic-cli-restore-run-require-stopped-reason");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-run.json");
        let journal = ready_apply_journal();

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        let err = run([
            OsString::from("run"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--dry-run"),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
            OsString::from("--require-stopped-reason"),
            OsString::from("complete"),
        ])
        .expect_err("stopped reason mismatch should fail requirement");

        let run_summary: serde_json::Value =
            serde_json::from_slice(&fs::read(&out_path).expect("read run summary"))
                .expect("decode run summary");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(run_summary["stopped_reason"], "preview");
        assert!(matches!(
            err,
            RestoreCommandError::RestoreRunStoppedReasonMismatch {
                expected,
                actual,
                ..
            } if expected == "complete" && actual == "preview"
        ));
    }

    // Ensure restore run can fail closed on an unexpected next action.
    #[test]
    fn run_restore_run_require_next_action_writes_summary_then_fails() {
        let root = temp_dir("canic-cli-restore-run-require-next-action");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-run.json");
        let journal = ready_apply_journal();

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        let err = run([
            OsString::from("run"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--dry-run"),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
            OsString::from("--require-next-action"),
            OsString::from("done"),
        ])
        .expect_err("next action mismatch should fail requirement");

        let run_summary: serde_json::Value =
            serde_json::from_slice(&fs::read(&out_path).expect("read run summary"))
                .expect("decode run summary");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(run_summary["next_action"], "rerun");
        assert!(matches!(
            err,
            RestoreCommandError::RestoreRunNextActionMismatch {
                expected,
                actual,
                ..
            } if expected == "done" && actual == "rerun"
        ));
    }

    // Ensure apply-report can fail closed after writing an attention report.
    #[test]
    fn run_restore_apply_report_require_no_attention_writes_report_then_fails() {
        let root = temp_dir("canic-cli-restore-apply-report-attention");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-apply-report.json");
        let mut journal = ready_apply_journal();
        journal
            .mark_next_operation_pending_at(Some("2026-05-05T12:01:00Z".to_string()))
            .expect("mark pending operation");

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        let err = run([
            OsString::from("apply-report"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
            OsString::from("--require-no-attention"),
        ])
        .expect_err("attention report should fail requirement");

        let report: RestoreApplyJournalReport =
            serde_json::from_slice(&fs::read(&out_path).expect("read apply report"))
                .expect("decode apply report");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(report.attention_required);
        assert_eq!(report.pending_operations, 1);
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyReportNeedsAttention {
                outcome: canic_backup::restore::RestoreApplyReportOutcome::Pending,
                ..
            }
        ));
    }

    // Ensure apply-status can fail closed after writing status for incomplete work.
    #[test]
    fn run_restore_apply_status_require_complete_writes_status_then_fails() {
        let root = temp_dir("canic-cli-restore-apply-status-incomplete");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-apply-status.json");
        let journal = ready_apply_journal();

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        let err = run([
            OsString::from("apply-status"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
            OsString::from("--require-complete"),
        ])
        .expect_err("incomplete journal should fail requirement");

        assert!(out_path.exists());
        let status: RestoreApplyJournalStatus =
            serde_json::from_slice(&fs::read(&out_path).expect("read apply status"))
                .expect("decode apply status");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(!status.complete);
        assert_eq!(status.completed_operations, 0);
        assert_eq!(status.operation_count, 8);
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyIncomplete {
                completed_operations: 0,
                operation_count: 8,
                ..
            }
        ));
    }

    // Ensure apply-status can fail closed after writing status for failed work.
    #[test]
    fn run_restore_apply_status_require_no_failed_writes_status_then_fails() {
        let root = temp_dir("canic-cli-restore-apply-status-failed");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-apply-status.json");
        let mut journal = ready_apply_journal();
        journal
            .mark_operation_failed(0, "dfx-load-failed".to_string())
            .expect("mark failed operation");

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        let err = run([
            OsString::from("apply-status"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
            OsString::from("--require-no-failed"),
        ])
        .expect_err("failed operation should fail requirement");

        assert!(out_path.exists());
        let status: RestoreApplyJournalStatus =
            serde_json::from_slice(&fs::read(&out_path).expect("read apply status"))
                .expect("decode apply status");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(status.failed_operations, 1);
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyFailed {
                failed_operations: 1,
                ..
            }
        ));
    }

    // Ensure apply-status accepts a complete journal when required.
    #[test]
    fn run_restore_apply_status_require_complete_accepts_complete_journal() {
        let root = temp_dir("canic-cli-restore-apply-status-complete");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-apply-status.json");
        let mut journal = ready_apply_journal();
        for sequence in 0..journal.operation_count {
            journal
                .mark_operation_completed(sequence)
                .expect("complete operation");
        }

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        run([
            OsString::from("apply-status"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
            OsString::from("--require-complete"),
        ])
        .expect("complete journal should pass requirement");

        let status: RestoreApplyJournalStatus =
            serde_json::from_slice(&fs::read(&out_path).expect("read apply status"))
                .expect("decode apply status");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(status.complete);
        assert_eq!(status.completed_operations, 8);
        assert_eq!(status.operation_count, 8);
    }

    // Ensure apply-next writes the full next ready operation row for runners.
    #[test]
    fn run_restore_apply_next_writes_next_ready_operation() {
        let root = temp_dir("canic-cli-restore-apply-next");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-apply-next.json");
        let mut journal = ready_apply_journal();
        journal
            .mark_operation_completed(0)
            .expect("mark first operation complete");

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        run([
            OsString::from("apply-next"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
        ])
        .expect("write apply next");

        let next: RestoreApplyNextOperation =
            serde_json::from_slice(&fs::read(&out_path).expect("read next operation"))
                .expect("decode next operation");
        let operation = next.operation.expect("operation should be available");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(next.ready);
        assert!(next.operation_available);
        assert_eq!(operation.sequence, 1);
        assert_eq!(
            operation.operation,
            canic_backup::restore::RestoreApplyOperationKind::LoadSnapshot
        );
    }

    // Ensure apply-command writes a no-execute command preview for the next operation.
    #[test]
    fn run_restore_apply_command_writes_next_command_preview() {
        let root = temp_dir("canic-cli-restore-apply-command");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-apply-command.json");
        let journal = ready_apply_journal();

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        run([
            OsString::from("apply-command"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--dfx"),
            OsString::from("/tmp/dfx"),
            OsString::from("--network"),
            OsString::from("local"),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
        ])
        .expect("write command preview");

        let preview: RestoreApplyCommandPreview =
            serde_json::from_slice(&fs::read(&out_path).expect("read command preview"))
                .expect("decode command preview");
        let command = preview.command.expect("command should be available");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(preview.ready);
        assert!(preview.command_available);
        assert_eq!(command.program, "/tmp/dfx");
        assert_eq!(
            command.args,
            vec![
                "canister".to_string(),
                "--network".to_string(),
                "local".to_string(),
                "snapshot".to_string(),
                "upload".to_string(),
                "--dir".to_string(),
                "artifacts/root".to_string(),
                ROOT.to_string(),
            ]
        );
        assert!(command.mutates);
    }

    // Ensure apply-command can fail closed after writing a command preview.
    #[test]
    fn run_restore_apply_command_require_command_writes_preview_then_fails() {
        let root = temp_dir("canic-cli-restore-apply-command-require");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let out_path = root.join("restore-apply-command.json");
        let mut journal = ready_apply_journal();

        for sequence in 0..journal.operation_count {
            journal
                .mark_operation_completed(sequence)
                .expect("mark operation completed");
        }

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        let err = run([
            OsString::from("apply-command"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
            OsString::from("--require-command"),
        ])
        .expect_err("missing command should fail");

        let preview: RestoreApplyCommandPreview =
            serde_json::from_slice(&fs::read(&out_path).expect("read command preview"))
                .expect("decode command preview");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(preview.complete);
        assert!(!preview.operation_available);
        assert!(!preview.command_available);
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyCommandUnavailable {
                operation_available: false,
                complete: true,
                ..
            }
        ));
    }

    // Ensure apply-claim marks the next operation pending before runner execution.
    #[test]
    fn run_restore_apply_claim_marks_next_operation_pending() {
        let root = temp_dir("canic-cli-restore-apply-claim");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let claimed_path = root.join("restore-apply-journal.claimed.json");
        let journal = ready_apply_journal();

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        run([
            OsString::from("apply-claim"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--sequence"),
            OsString::from("0"),
            OsString::from("--updated-at"),
            OsString::from("2026-05-04T12:00:00Z"),
            OsString::from("--out"),
            OsString::from(claimed_path.as_os_str()),
        ])
        .expect("claim operation");

        let claimed: RestoreApplyJournal =
            serde_json::from_slice(&fs::read(&claimed_path).expect("read claimed journal"))
                .expect("decode claimed journal");
        let status = claimed.status();
        let next = claimed.next_operation();

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(claimed.pending_operations, 1);
        assert_eq!(claimed.ready_operations, 7);
        assert_eq!(
            claimed.operations[0].state,
            RestoreApplyOperationState::Pending
        );
        assert_eq!(
            claimed.operations[0].state_updated_at.as_deref(),
            Some("2026-05-04T12:00:00Z")
        );
        assert_eq!(status.next_transition_sequence, Some(0));
        assert_eq!(
            status.next_transition_state,
            Some(RestoreApplyOperationState::Pending)
        );
        assert_eq!(
            status.next_transition_updated_at.as_deref(),
            Some("2026-05-04T12:00:00Z")
        );
        assert_eq!(
            next.operation.expect("next operation").state,
            RestoreApplyOperationState::Pending
        );
    }

    // Ensure apply-claim can reject a stale command preview sequence.
    #[test]
    fn run_restore_apply_claim_rejects_sequence_mismatch() {
        let root = temp_dir("canic-cli-restore-apply-claim-sequence");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let claimed_path = root.join("restore-apply-journal.claimed.json");
        let journal = ready_apply_journal();

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        let err = run([
            OsString::from("apply-claim"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--sequence"),
            OsString::from("1"),
            OsString::from("--out"),
            OsString::from(claimed_path.as_os_str()),
        ])
        .expect_err("stale sequence should fail claim");

        assert!(!claimed_path.exists());
        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyClaimSequenceMismatch {
                expected: 1,
                actual: Some(0),
            }
        ));
    }

    // Ensure apply-unclaim releases the current pending operation back to ready.
    #[test]
    fn run_restore_apply_unclaim_marks_pending_operation_ready() {
        let root = temp_dir("canic-cli-restore-apply-unclaim");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let unclaimed_path = root.join("restore-apply-journal.unclaimed.json");
        let mut journal = ready_apply_journal();
        journal
            .mark_next_operation_pending()
            .expect("claim operation");

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        run([
            OsString::from("apply-unclaim"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--sequence"),
            OsString::from("0"),
            OsString::from("--updated-at"),
            OsString::from("2026-05-04T12:01:00Z"),
            OsString::from("--out"),
            OsString::from(unclaimed_path.as_os_str()),
        ])
        .expect("unclaim operation");

        let unclaimed: RestoreApplyJournal =
            serde_json::from_slice(&fs::read(&unclaimed_path).expect("read unclaimed journal"))
                .expect("decode unclaimed journal");
        let status = unclaimed.status();

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(unclaimed.pending_operations, 0);
        assert_eq!(unclaimed.ready_operations, 8);
        assert_eq!(
            unclaimed.operations[0].state,
            RestoreApplyOperationState::Ready
        );
        assert_eq!(
            unclaimed.operations[0].state_updated_at.as_deref(),
            Some("2026-05-04T12:01:00Z")
        );
        assert_eq!(status.next_ready_sequence, Some(0));
        assert_eq!(
            status.next_transition_state,
            Some(RestoreApplyOperationState::Ready)
        );
        assert_eq!(
            status.next_transition_updated_at.as_deref(),
            Some("2026-05-04T12:01:00Z")
        );
    }

    // Ensure apply-unclaim can reject a stale pending operation sequence.
    #[test]
    fn run_restore_apply_unclaim_rejects_sequence_mismatch() {
        let root = temp_dir("canic-cli-restore-apply-unclaim-sequence");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let unclaimed_path = root.join("restore-apply-journal.unclaimed.json");
        let mut journal = ready_apply_journal();
        journal
            .mark_next_operation_pending()
            .expect("claim operation");

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        let err = run([
            OsString::from("apply-unclaim"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--sequence"),
            OsString::from("1"),
            OsString::from("--out"),
            OsString::from(unclaimed_path.as_os_str()),
        ])
        .expect_err("stale sequence should fail unclaim");

        assert!(!unclaimed_path.exists());
        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyUnclaimSequenceMismatch {
                expected: 1,
                actual: Some(0),
            }
        ));
    }

    // Ensure apply-mark can advance one journal operation and keep counts consistent.
    #[test]
    fn run_restore_apply_mark_completes_operation() {
        let root = temp_dir("canic-cli-restore-apply-mark-complete");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let updated_path = root.join("restore-apply-journal.updated.json");
        let journal = ready_apply_journal();

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        run([
            OsString::from("apply-mark"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--sequence"),
            OsString::from("0"),
            OsString::from("--state"),
            OsString::from("completed"),
            OsString::from("--updated-at"),
            OsString::from("2026-05-04T12:02:00Z"),
            OsString::from("--out"),
            OsString::from(updated_path.as_os_str()),
        ])
        .expect("mark operation completed");

        let updated: RestoreApplyJournal =
            serde_json::from_slice(&fs::read(&updated_path).expect("read updated journal"))
                .expect("decode updated journal");
        let status = updated.status();

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(updated.completed_operations, 1);
        assert_eq!(updated.ready_operations, 7);
        assert_eq!(
            updated.operations[0].state_updated_at.as_deref(),
            Some("2026-05-04T12:02:00Z")
        );
        assert_eq!(status.next_ready_sequence, Some(1));
    }

    // Ensure apply-mark can require an operation claim before completion.
    #[test]
    fn run_restore_apply_mark_require_pending_rejects_ready_operation() {
        let root = temp_dir("canic-cli-restore-apply-mark-require-pending");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let updated_path = root.join("restore-apply-journal.updated.json");
        let journal = ready_apply_journal();

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        let err = run([
            OsString::from("apply-mark"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--sequence"),
            OsString::from("0"),
            OsString::from("--state"),
            OsString::from("completed"),
            OsString::from("--out"),
            OsString::from(updated_path.as_os_str()),
            OsString::from("--require-pending"),
        ])
        .expect_err("ready operation should fail pending requirement");

        assert!(!updated_path.exists());
        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyMarkRequiresPending {
                sequence: 0,
                state: RestoreApplyOperationState::Ready,
            }
        ));
    }

    // Ensure apply-mark refuses to skip earlier ready operations.
    #[test]
    fn run_restore_apply_mark_rejects_out_of_order_operation() {
        let root = temp_dir("canic-cli-restore-apply-mark-out-of-order");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let updated_path = root.join("restore-apply-journal.updated.json");
        let journal = ready_apply_journal();

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        let err = run([
            OsString::from("apply-mark"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--sequence"),
            OsString::from("1"),
            OsString::from("--state"),
            OsString::from("completed"),
            OsString::from("--out"),
            OsString::from(updated_path.as_os_str()),
        ])
        .expect_err("out-of-order operation should fail");

        assert!(!updated_path.exists());
        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyJournal(
                RestoreApplyJournalError::OutOfOrderOperationTransition {
                    requested: 1,
                    next: 0
                }
            )
        ));
    }

    // Ensure apply-mark requires failure reasons for failed operation state.
    #[test]
    fn run_restore_apply_mark_failed_requires_reason() {
        let root = temp_dir("canic-cli-restore-apply-mark-failed-reason");
        fs::create_dir_all(&root).expect("create temp root");
        let journal_path = root.join("restore-apply-journal.json");
        let journal = ready_apply_journal();

        fs::write(
            &journal_path,
            serde_json::to_vec(&journal).expect("serialize journal"),
        )
        .expect("write journal");

        let err = run([
            OsString::from("apply-mark"),
            OsString::from("--journal"),
            OsString::from(journal_path.as_os_str()),
            OsString::from("--sequence"),
            OsString::from("0"),
            OsString::from("--state"),
            OsString::from("failed"),
        ])
        .expect_err("failed state should require reason");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyJournal(
                RestoreApplyJournalError::FailureReasonRequired(0)
            )
        ));
    }

    // Ensure restore apply dry-run rejects status files from another plan.
    #[test]
    fn run_restore_apply_dry_run_rejects_mismatched_status() {
        let root = temp_dir("canic-cli-restore-apply-dry-run-mismatch");
        fs::create_dir_all(&root).expect("create temp root");
        let plan_path = root.join("restore-plan.json");
        let status_path = root.join("restore-status.json");
        let out_path = root.join("restore-apply-dry-run.json");
        let plan = RestorePlanner::plan(&restore_ready_manifest(), None).expect("build plan");
        let mut status = RestoreStatus::from_plan(&plan);
        status.backup_id = "other-backup".to_string();

        fs::write(
            &plan_path,
            serde_json::to_vec(&plan).expect("serialize plan"),
        )
        .expect("write plan");
        fs::write(
            &status_path,
            serde_json::to_vec(&status).expect("serialize status"),
        )
        .expect("write status");

        let err = run([
            OsString::from("apply"),
            OsString::from("--plan"),
            OsString::from(plan_path.as_os_str()),
            OsString::from("--status"),
            OsString::from(status_path.as_os_str()),
            OsString::from("--dry-run"),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
        ])
        .expect_err("mismatched status should fail");

        assert!(!out_path.exists());
        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyDryRun(RestoreApplyDryRunError::StatusPlanMismatch {
                field: "backup_id",
                ..
            })
        ));
    }

    // Build one manually ready apply journal for runner-focused CLI tests.
    fn ready_apply_journal() -> RestoreApplyJournal {
        let plan = RestorePlanner::plan(&restore_ready_manifest(), None).expect("build plan");
        let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run");
        let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

        journal.ready = true;
        journal.blocked_reasons = Vec::new();
        for operation in &mut journal.operations {
            operation.state = canic_backup::restore::RestoreApplyOperationState::Ready;
            operation.blocking_reasons = Vec::new();
        }
        journal.blocked_operations = 0;
        journal.ready_operations = journal.operation_count;
        journal.validate().expect("journal should validate");
        journal
    }

    // Build one valid manifest for restore planning tests.
    fn valid_manifest() -> FleetBackupManifest {
        FleetBackupManifest {
            manifest_version: 1,
            backup_id: "backup-test".to_string(),
            created_at: "2026-05-03T00:00:00Z".to_string(),
            tool: ToolMetadata {
                name: "canic".to_string(),
                version: "0.30.1".to_string(),
            },
            source: SourceMetadata {
                environment: "local".to_string(),
                root_canister: ROOT.to_string(),
            },
            consistency: ConsistencySection {
                mode: ConsistencyMode::CrashConsistent,
                backup_units: vec![BackupUnit {
                    unit_id: "fleet".to_string(),
                    kind: BackupUnitKind::SubtreeRooted,
                    roles: vec!["root".to_string(), "app".to_string()],
                    consistency_reason: None,
                    dependency_closure: Vec::new(),
                    topology_validation: "subtree-closed".to_string(),
                    quiescence_strategy: None,
                }],
            },
            fleet: FleetSection {
                topology_hash_algorithm: "sha256".to_string(),
                topology_hash_input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
                discovery_topology_hash: HASH.to_string(),
                pre_snapshot_topology_hash: HASH.to_string(),
                topology_hash: HASH.to_string(),
                members: vec![
                    fleet_member("root", ROOT, None, IdentityMode::Fixed),
                    fleet_member("app", CHILD, Some(ROOT), IdentityMode::Relocatable),
                ],
            },
            verification: VerificationPlan::default(),
        }
    }

    // Build one manifest whose restore readiness metadata is complete.
    fn restore_ready_manifest() -> FleetBackupManifest {
        let mut manifest = valid_manifest();
        for member in &mut manifest.fleet.members {
            member.source_snapshot.module_hash = Some(HASH.to_string());
            member.source_snapshot.wasm_hash = Some(HASH.to_string());
            member.source_snapshot.checksum = Some(HASH.to_string());
        }
        manifest
    }

    // Build one valid manifest member.
    fn fleet_member(
        role: &str,
        canister_id: &str,
        parent_canister_id: Option<&str>,
        identity_mode: IdentityMode,
    ) -> FleetMember {
        FleetMember {
            role: role.to_string(),
            canister_id: canister_id.to_string(),
            parent_canister_id: parent_canister_id.map(str::to_string),
            subnet_canister_id: Some(ROOT.to_string()),
            controller_hint: None,
            identity_mode,
            restore_group: 1,
            verification_class: "basic".to_string(),
            verification_checks: vec![VerificationCheck {
                kind: "status".to_string(),
                method: None,
                roles: vec![role.to_string()],
            }],
            source_snapshot: SourceSnapshot {
                snapshot_id: format!("{role}-snapshot"),
                module_hash: None,
                wasm_hash: None,
                code_version: Some("v0.30.1".to_string()),
                artifact_path: format!("artifacts/{role}"),
                checksum_algorithm: "sha256".to_string(),
                checksum: None,
            },
        }
    }

    // Write a canonical backup layout whose journal checksums match the artifacts.
    fn write_verified_layout(root: &Path, layout: &BackupLayout, manifest: &FleetBackupManifest) {
        layout.write_manifest(manifest).expect("write manifest");

        let artifacts = manifest
            .fleet
            .members
            .iter()
            .map(|member| {
                let bytes = format!("{} artifact", member.role);
                let artifact_path = root.join(&member.source_snapshot.artifact_path);
                if let Some(parent) = artifact_path.parent() {
                    fs::create_dir_all(parent).expect("create artifact parent");
                }
                fs::write(&artifact_path, bytes.as_bytes()).expect("write artifact");
                let checksum = ArtifactChecksum::from_bytes(bytes.as_bytes());

                ArtifactJournalEntry {
                    canister_id: member.canister_id.clone(),
                    snapshot_id: member.source_snapshot.snapshot_id.clone(),
                    state: ArtifactState::Durable,
                    temp_path: None,
                    artifact_path: member.source_snapshot.artifact_path.clone(),
                    checksum_algorithm: checksum.algorithm,
                    checksum: Some(checksum.hash),
                    updated_at: "2026-05-03T00:00:00Z".to_string(),
                }
            })
            .collect();

        layout
            .write_journal(&DownloadJournal {
                journal_version: 1,
                backup_id: manifest.backup_id.clone(),
                discovery_topology_hash: Some(manifest.fleet.discovery_topology_hash.clone()),
                pre_snapshot_topology_hash: Some(manifest.fleet.pre_snapshot_topology_hash.clone()),
                operation_metrics: canic_backup::journal::DownloadOperationMetrics::default(),
                artifacts,
            })
            .expect("write journal");
    }

    // Write artifact bytes and update the manifest checksums for apply validation.
    fn write_manifest_artifacts(root: &Path, manifest: &mut FleetBackupManifest) {
        for member in &mut manifest.fleet.members {
            let bytes = format!("{} apply artifact", member.role);
            let artifact_path = root.join(&member.source_snapshot.artifact_path);
            if let Some(parent) = artifact_path.parent() {
                fs::create_dir_all(parent).expect("create artifact parent");
            }
            fs::write(&artifact_path, bytes.as_bytes()).expect("write artifact");
            let checksum = ArtifactChecksum::from_bytes(bytes.as_bytes());
            member.source_snapshot.checksum = Some(checksum.hash);
        }
    }

    // Build a unique temporary directory.
    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }
}
