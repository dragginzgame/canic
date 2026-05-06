use crate::{output, version_text};
use canic_backup::{
    manifest::FleetBackupManifest,
    persistence::{BackupLayout, PersistenceError},
    restore::{
        RESTORE_RUN_RECEIPT_COMPLETED, RESTORE_RUN_RECEIPT_FAILED,
        RESTORE_RUN_RECEIPT_RECOVERED_PENDING, RestoreApplyCommandConfig, RestoreApplyDryRun,
        RestoreApplyDryRunError, RestoreApplyJournal, RestoreApplyJournalError,
        RestoreApplyJournalReport, RestoreApplyJournalStatus, RestoreApplyPendingSummary,
        RestoreApplyProgressSummary, RestoreMapping, RestorePlan, RestorePlanError, RestorePlanner,
        RestoreRunOperationReceipt, RestoreRunResponse, RestoreRunnerConfig, RestoreRunnerError,
        RestoreStatus,
    },
};
use clap::{Arg, ArgAction, Command as ClapCommand};
use std::{ffi::OsString, fs, path::PathBuf};
use thiserror::Error as ThisError;

pub use canic_backup::restore::parse_uploaded_snapshot_id;

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

    #[error("restore apply journal is locked: {lock_path}")]
    RestoreApplyJournalLocked { lock_path: String },

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

    #[error(
        "restore run for backup {backup_id} wrote {actual_receipts} receipts with {mismatched_receipts} updated_at mismatches, expected {expected}"
    )]
    RestoreRunReceiptUpdatedAtMismatch {
        backup_id: String,
        expected: String,
        actual_receipts: usize,
        mismatched_receipts: usize,
    },

    #[error(
        "restore run for backup {backup_id} reported requested_state_updated_at={actual:?}, expected {expected}"
    )]
    RestoreRunStateUpdatedAtMismatch {
        backup_id: String,
        expected: String,
        actual: Option<String>,
    },

    #[error("restore plan for backup {backup_id} is not restore-ready: reasons={reasons:?}")]
    RestoreNotReady {
        backup_id: String,
        reasons: Vec<String>,
    },

    #[error("restore manifest {backup_id} is not design ready")]
    DesignConformanceNotReady { backup_id: String },

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
        "restore apply journal next operation changed before claim: expected={expected}, actual={actual:?}"
    )]
    RestoreRunClaimSequenceMismatch {
        expected: usize,
        actual: Option<usize>,
    },

    #[error("unknown option {0}")]
    UnknownOption(String),

    #[error("option --sequence requires a non-negative integer value")]
    InvalidSequence,

    #[error("option {option} requires a positive integer value")]
    InvalidPositiveInteger { option: &'static str },

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

impl From<RestoreRunnerError> for RestoreCommandError {
    // Preserve the CLI-facing error variants while delegating runner ownership downward.
    fn from(error: RestoreRunnerError) -> Self {
        match error {
            RestoreRunnerError::CommandFailed { sequence, status } => {
                Self::RestoreRunCommandFailed { sequence, status }
            }
            RestoreRunnerError::JournalLocked { lock_path } => {
                Self::RestoreApplyJournalLocked { lock_path }
            }
            RestoreRunnerError::Pending {
                backup_id,
                pending_operations,
                next_transition_sequence,
            } => Self::RestoreApplyPending {
                backup_id,
                pending_operations,
                next_transition_sequence,
            },
            RestoreRunnerError::Failed {
                backup_id,
                failed_operations,
            } => Self::RestoreApplyFailed {
                backup_id,
                failed_operations,
            },
            RestoreRunnerError::NotReady { backup_id, reasons } => {
                Self::RestoreApplyNotReady { backup_id, reasons }
            }
            RestoreRunnerError::CommandUnavailable {
                backup_id,
                operation_available,
                complete,
                blocked_reasons,
            } => Self::RestoreApplyCommandUnavailable {
                backup_id,
                operation_available,
                complete,
                blocked_reasons,
            },
            RestoreRunnerError::ClaimSequenceMismatch { expected, actual } => {
                Self::RestoreRunClaimSequenceMismatch { expected, actual }
            }
            RestoreRunnerError::Io(error) => Self::Io(error),
            RestoreRunnerError::Json(error) => Self::Json(error),
            RestoreRunnerError::Journal(error) => Self::RestoreApplyJournal(error),
        }
    }
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
    pub require_design_v1: bool,
    pub require_restore_ready: bool,
}

impl RestorePlanOptions {
    /// Parse restore planning options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = restore_plan_command()
            .try_get_matches_from(std::iter::once(OsString::from("restore-plan")).chain(args))
            .map_err(|_| RestoreCommandError::Usage(usage()))?;

        let manifest = path_option(&matches, "manifest");
        let backup_dir = path_option(&matches, "backup-dir");
        let require_verified = matches.get_flag("require-verified");

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
            mapping: path_option(&matches, "mapping"),
            out: path_option(&matches, "out"),
            require_verified,
            require_design_v1: matches.get_flag("require-design"),
            require_restore_ready: matches.get_flag("require-restore-ready"),
        })
    }
}

// Build the restore plan parser.
fn restore_plan_command() -> ClapCommand {
    ClapCommand::new("restore-plan")
        .disable_help_flag(true)
        .arg(value_arg("manifest").long("manifest"))
        .arg(value_arg("backup-dir").long("backup-dir"))
        .arg(value_arg("mapping").long("mapping"))
        .arg(value_arg("out").long("out"))
        .arg(flag_arg("require-verified").long("require-verified"))
        .arg(flag_arg("require-design").long("require-design"))
        .arg(flag_arg("require-restore-ready").long("require-restore-ready"))
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
        let matches = restore_status_command()
            .try_get_matches_from(std::iter::once(OsString::from("restore-status")).chain(args))
            .map_err(|_| RestoreCommandError::Usage(usage()))?;

        Ok(Self {
            plan: path_option(&matches, "plan")
                .ok_or(RestoreCommandError::MissingOption("--plan"))?,
            out: path_option(&matches, "out"),
        })
    }
}

// Build the restore status parser.
fn restore_status_command() -> ClapCommand {
    ClapCommand::new("restore-status")
        .disable_help_flag(true)
        .arg(value_arg("plan").long("plan"))
        .arg(value_arg("out").long("out"))
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
        let matches = restore_apply_command()
            .try_get_matches_from(std::iter::once(OsString::from("restore-apply")).chain(args))
            .map_err(|_| RestoreCommandError::Usage(usage()))?;
        let dry_run = matches.get_flag("dry-run");

        if !dry_run {
            return Err(RestoreCommandError::ApplyRequiresDryRun);
        }

        Ok(Self {
            plan: path_option(&matches, "plan")
                .ok_or(RestoreCommandError::MissingOption("--plan"))?,
            status: path_option(&matches, "status"),
            backup_dir: path_option(&matches, "backup-dir"),
            out: path_option(&matches, "out"),
            journal_out: path_option(&matches, "journal-out"),
            dry_run,
        })
    }
}

// Build the restore apply dry-run parser.
fn restore_apply_command() -> ClapCommand {
    ClapCommand::new("restore-apply")
        .disable_help_flag(true)
        .arg(value_arg("plan").long("plan"))
        .arg(value_arg("status").long("status"))
        .arg(value_arg("backup-dir").long("backup-dir"))
        .arg(value_arg("out").long("out"))
        .arg(value_arg("journal-out").long("journal-out"))
        .arg(flag_arg("dry-run").long("dry-run"))
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
        let matches = restore_apply_status_command()
            .try_get_matches_from(
                std::iter::once(OsString::from("restore-apply-status")).chain(args),
            )
            .map_err(|_| RestoreCommandError::Usage(usage()))?;

        Ok(Self {
            journal: path_option(&matches, "journal")
                .ok_or(RestoreCommandError::MissingOption("--journal"))?,
            require_ready: matches.get_flag("require-ready"),
            require_no_pending: matches.get_flag("require-no-pending"),
            require_no_failed: matches.get_flag("require-no-failed"),
            require_complete: matches.get_flag("require-complete"),
            require_remaining_count: sequence_option(&matches, "require-remaining-count")?,
            require_attention_count: sequence_option(&matches, "require-attention-count")?,
            require_completion_basis_points: sequence_option(
                &matches,
                "require-completion-basis-points",
            )?,
            require_no_pending_before: string_option(&matches, "require-no-pending-before"),
            out: path_option(&matches, "out"),
        })
    }
}

// Build the restore apply-status parser.
fn restore_apply_status_command() -> ClapCommand {
    ClapCommand::new("restore-apply-status")
        .disable_help_flag(true)
        .arg(value_arg("journal").long("journal"))
        .arg(flag_arg("require-ready").long("require-ready"))
        .arg(flag_arg("require-no-pending").long("require-no-pending"))
        .arg(flag_arg("require-no-failed").long("require-no-failed"))
        .arg(flag_arg("require-complete").long("require-complete"))
        .arg(value_arg("require-remaining-count").long("require-remaining-count"))
        .arg(value_arg("require-attention-count").long("require-attention-count"))
        .arg(value_arg("require-completion-basis-points").long("require-completion-basis-points"))
        .arg(value_arg("require-no-pending-before").long("require-no-pending-before"))
        .arg(value_arg("out").long("out"))
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
        let matches = restore_apply_report_command()
            .try_get_matches_from(
                std::iter::once(OsString::from("restore-apply-report")).chain(args),
            )
            .map_err(|_| RestoreCommandError::Usage(usage()))?;

        Ok(Self {
            journal: path_option(&matches, "journal")
                .ok_or(RestoreCommandError::MissingOption("--journal"))?,
            require_no_attention: matches.get_flag("require-no-attention"),
            require_remaining_count: sequence_option(&matches, "require-remaining-count")?,
            require_attention_count: sequence_option(&matches, "require-attention-count")?,
            require_completion_basis_points: sequence_option(
                &matches,
                "require-completion-basis-points",
            )?,
            require_no_pending_before: string_option(&matches, "require-no-pending-before"),
            out: path_option(&matches, "out"),
        })
    }
}

// Build the restore apply-report parser.
fn restore_apply_report_command() -> ClapCommand {
    ClapCommand::new("restore-apply-report")
        .disable_help_flag(true)
        .arg(value_arg("journal").long("journal"))
        .arg(flag_arg("require-no-attention").long("require-no-attention"))
        .arg(value_arg("require-remaining-count").long("require-remaining-count"))
        .arg(value_arg("require-attention-count").long("require-attention-count"))
        .arg(value_arg("require-completion-basis-points").long("require-completion-basis-points"))
        .arg(value_arg("require-no-pending-before").long("require-no-pending-before"))
        .arg(value_arg("out").long("out"))
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
    pub updated_at: Option<String>,
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
    pub require_receipt_updated_at: Option<String>,
    pub require_state_updated_at: Option<String>,
    pub require_remaining_count: Option<usize>,
    pub require_attention_count: Option<usize>,
    pub require_completion_basis_points: Option<usize>,
    pub require_no_pending_before: Option<String>,
}

impl RestoreRunOptions {
    /// Parse restore run options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = restore_run_command()
            .try_get_matches_from(std::iter::once(OsString::from("restore-run")).chain(args))
            .map_err(|_| RestoreCommandError::Usage(usage()))?;

        let dry_run = matches.get_flag("dry-run");
        let execute = matches.get_flag("execute");
        let unclaim_pending = matches.get_flag("unclaim-pending");

        validate_restore_run_mode_selection(dry_run, execute, unclaim_pending)?;

        Ok(Self {
            journal: path_option(&matches, "journal")
                .ok_or(RestoreCommandError::MissingOption("--journal"))?,
            dfx: string_option(&matches, "dfx").unwrap_or_else(|| "dfx".to_string()),
            network: string_option(&matches, "network"),
            out: path_option(&matches, "out"),
            dry_run,
            execute,
            unclaim_pending,
            max_steps: positive_integer_option(&matches, "max-steps", "--max-steps")?,
            updated_at: string_option(&matches, "updated-at"),
            require_complete: matches.get_flag("require-complete"),
            require_no_attention: matches.get_flag("require-no-attention"),
            require_run_mode: string_option(&matches, "require-run-mode"),
            require_stopped_reason: string_option(&matches, "require-stopped-reason"),
            require_next_action: string_option(&matches, "require-next-action"),
            require_executed_count: sequence_option(&matches, "require-executed-count")?,
            require_receipt_count: sequence_option(&matches, "require-receipt-count")?,
            require_completed_receipt_count: sequence_option(
                &matches,
                "require-completed-receipt-count",
            )?,
            require_failed_receipt_count: sequence_option(
                &matches,
                "require-failed-receipt-count",
            )?,
            require_recovered_receipt_count: sequence_option(
                &matches,
                "require-recovered-receipt-count",
            )?,
            require_receipt_updated_at: string_option(&matches, "require-receipt-updated-at"),
            require_state_updated_at: string_option(&matches, "require-state-updated-at"),
            require_remaining_count: sequence_option(&matches, "require-remaining-count")?,
            require_attention_count: sequence_option(&matches, "require-attention-count")?,
            require_completion_basis_points: sequence_option(
                &matches,
                "require-completion-basis-points",
            )?,
            require_no_pending_before: string_option(&matches, "require-no-pending-before"),
        })
    }
}

// Build the native restore runner parser.
fn restore_run_command() -> ClapCommand {
    ClapCommand::new("restore-run")
        .disable_help_flag(true)
        .arg(value_arg("journal").long("journal"))
        .arg(value_arg("dfx").long("dfx"))
        .arg(value_arg("network").long("network"))
        .arg(value_arg("out").long("out"))
        .arg(flag_arg("dry-run").long("dry-run"))
        .arg(flag_arg("execute").long("execute"))
        .arg(flag_arg("unclaim-pending").long("unclaim-pending"))
        .arg(value_arg("max-steps").long("max-steps"))
        .arg(value_arg("updated-at").long("updated-at"))
        .arg(flag_arg("require-complete").long("require-complete"))
        .arg(flag_arg("require-no-attention").long("require-no-attention"))
        .arg(value_arg("require-run-mode").long("require-run-mode"))
        .arg(value_arg("require-stopped-reason").long("require-stopped-reason"))
        .arg(value_arg("require-next-action").long("require-next-action"))
        .arg(value_arg("require-executed-count").long("require-executed-count"))
        .arg(value_arg("require-receipt-count").long("require-receipt-count"))
        .arg(value_arg("require-completed-receipt-count").long("require-completed-receipt-count"))
        .arg(value_arg("require-failed-receipt-count").long("require-failed-receipt-count"))
        .arg(value_arg("require-recovered-receipt-count").long("require-recovered-receipt-count"))
        .arg(value_arg("require-receipt-updated-at").long("require-receipt-updated-at"))
        .arg(value_arg("require-state-updated-at").long("require-state-updated-at"))
        .arg(value_arg("require-remaining-count").long("require-remaining-count"))
        .arg(value_arg("require-attention-count").long("require-attention-count"))
        .arg(value_arg("require-completion-basis-points").long("require-completion-basis-points"))
        .arg(value_arg("require-no-pending-before").long("require-no-pending-before"))
}

// Build one string-valued Clap argument.
fn value_arg(id: &'static str) -> Arg {
    Arg::new(id).num_args(1)
}

// Build one boolean Clap argument.
fn flag_arg(id: &'static str) -> Arg {
    Arg::new(id).action(ArgAction::SetTrue)
}

// Read one string option from Clap matches.
fn string_option(matches: &clap::ArgMatches, id: &str) -> Option<String> {
    matches.get_one::<String>(id).cloned()
}

// Read one path option from Clap matches.
fn path_option(matches: &clap::ArgMatches, id: &str) -> Option<PathBuf> {
    string_option(matches, id).map(PathBuf::from)
}

// Read one usize option from Clap matches.
fn sequence_option(
    matches: &clap::ArgMatches,
    id: &str,
) -> Result<Option<usize>, RestoreCommandError> {
    string_option(matches, id).map(parse_sequence).transpose()
}

// Read one positive integer option from Clap matches.
fn positive_integer_option(
    matches: &clap::ArgMatches,
    id: &str,
    option: &'static str,
) -> Result<Option<usize>, RestoreCommandError> {
    string_option(matches, id)
        .map(|value| parse_positive_integer(option, value))
        .transpose()
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
                canic_backup::restore::RestoreRunnerOutcome {
                    response: restore_run_unclaim_pending(&options)?,
                    error: None,
                }
            } else {
                canic_backup::restore::RestoreRunnerOutcome {
                    response: restore_run_dry_run(&options)?,
                    error: None,
                }
            };
            write_restore_run(&options, &run.response)?;
            if let Some(error) = run.error {
                return Err(error.into());
            }
            enforce_restore_run_requirements(&options, &run.response)?;
            Ok(())
        }
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("{}", version_text());
            Ok(())
        }
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
    canic_backup::restore::restore_run_dry_run(&restore_runner_config(options))
        .map_err(RestoreCommandError::from)
}

/// Recover an interrupted restore runner by unclaiming the pending operation.
pub fn restore_run_unclaim_pending(
    options: &RestoreRunOptions,
) -> Result<RestoreRunResponse, RestoreCommandError> {
    canic_backup::restore::restore_run_unclaim_pending(&restore_runner_config(options))
        .map_err(RestoreCommandError::from)
}

// Execute ready restore apply operations and retain any deferred runner error.
fn restore_run_execute_result(
    options: &RestoreRunOptions,
) -> Result<canic_backup::restore::RestoreRunnerOutcome, RestoreCommandError> {
    canic_backup::restore::restore_run_execute_result(&restore_runner_config(options))
        .map_err(RestoreCommandError::from)
}

// Build command-preview configuration from common dfx/network inputs.
fn restore_command_config(program: &str, network: Option<&str>) -> RestoreApplyCommandConfig {
    RestoreApplyCommandConfig {
        program: program.to_string(),
        network: network.map(str::to_string),
    }
}

// Build the lower-level restore runner configuration from CLI flags.
fn restore_runner_config(options: &RestoreRunOptions) -> RestoreRunnerConfig {
    RestoreRunnerConfig {
        journal: options.journal.clone(),
        command: restore_command_config(&options.dfx, options.network.as_deref()),
        max_steps: options.max_steps,
        updated_at: options.updated_at.clone(),
    }
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

    enforce_restore_run_receipt_requirements(options, run)?;

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

// Enforce caller-requested native runner receipt and marker requirements.
fn enforce_restore_run_receipt_requirements(
    options: &RestoreRunOptions,
    run: &RestoreRunResponse,
) -> Result<(), RestoreCommandError> {
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
    enforce_restore_run_receipt_updated_at_requirement(
        &run.backup_id,
        &run.operation_receipts,
        options.require_receipt_updated_at.as_deref(),
    )?;
    enforce_restore_run_state_updated_at_requirement(
        &run.backup_id,
        run.requested_state_updated_at.as_deref(),
        options.require_state_updated_at.as_deref(),
    )?;

    Ok(())
}

// Fail when a runner summary does not echo the requested state marker.
fn enforce_restore_run_state_updated_at_requirement(
    backup_id: &str,
    actual: Option<&str>,
    expected: Option<&str>,
) -> Result<(), RestoreCommandError> {
    if let Some(expected) = expected
        && actual != Some(expected)
    {
        return Err(RestoreCommandError::RestoreRunStateUpdatedAtMismatch {
            backup_id: backup_id.to_string(),
            expected: expected.to_string(),
            actual: actual.map(str::to_string),
        });
    }

    Ok(())
}

// Fail when emitted runner receipts are missing the requested state marker.
fn enforce_restore_run_receipt_updated_at_requirement(
    backup_id: &str,
    receipts: &[RestoreRunOperationReceipt],
    expected: Option<&str>,
) -> Result<(), RestoreCommandError> {
    let Some(expected) = expected else {
        return Ok(());
    };

    let actual_receipts = receipts.len();
    let mismatched_receipts = receipts
        .iter()
        .filter(|receipt| receipt.updated_at.as_deref() != Some(expected))
        .count();
    if actual_receipts == 0 || mismatched_receipts > 0 {
        return Err(RestoreCommandError::RestoreRunReceiptUpdatedAtMismatch {
            backup_id: backup_id.to_string(),
            expected: expected.to_string(),
            actual_receipts,
            mismatched_receipts,
        });
    }

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

// Enforce caller-requested restore plan requirements after the plan is emitted.
fn enforce_restore_plan_requirements(
    options: &RestorePlanOptions,
    plan: &RestorePlan,
) -> Result<(), RestoreCommandError> {
    if options.require_design_v1 {
        let manifest = read_manifest_source(options)?;
        if !manifest.design_conformance_report().design_v1_ready {
            return Err(RestoreCommandError::DesignConformanceNotReady {
                backup_id: plan.backup_id.clone(),
            });
        }
    }

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

// Parse a restore apply journal operation sequence value.
fn parse_sequence(value: String) -> Result<usize, RestoreCommandError> {
    value
        .parse::<usize>()
        .map_err(|_| RestoreCommandError::InvalidSequence)
}

// Parse a positive integer CLI value for options where zero is not meaningful.
fn parse_positive_integer(
    option: &'static str,
    value: String,
) -> Result<usize, RestoreCommandError> {
    let parsed = parse_sequence(value)?;
    if parsed == 0 {
        return Err(RestoreCommandError::InvalidPositiveInteger { option });
    }

    Ok(parsed)
}

// Write the computed plan to stdout or a requested output file.
fn write_plan(options: &RestorePlanOptions, plan: &RestorePlan) -> Result<(), RestoreCommandError> {
    output::write_pretty_json(options.out.as_ref(), plan)
}

// Write the computed status to stdout or a requested output file.
fn write_status(
    options: &RestoreStatusOptions,
    status: &RestoreStatus,
) -> Result<(), RestoreCommandError> {
    output::write_pretty_json(options.out.as_ref(), status)
}

// Write the computed apply dry-run to stdout or a requested output file.
fn write_apply_dry_run(
    options: &RestoreApplyOptions,
    dry_run: &RestoreApplyDryRun,
) -> Result<(), RestoreCommandError> {
    output::write_pretty_json(options.out.as_ref(), dry_run)
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
    output::write_pretty_json(options.out.as_ref(), status)
}

// Write the computed apply journal report to stdout or a requested output file.
fn write_apply_report(
    options: &RestoreApplyReportOptions,
    report: &RestoreApplyJournalReport,
) -> Result<(), RestoreCommandError> {
    output::write_pretty_json(options.out.as_ref(), report)
}

// Write the restore runner response to stdout or a requested output file.
fn write_restore_run(
    options: &RestoreRunOptions,
    run: &RestoreRunResponse,
) -> Result<(), RestoreCommandError> {
    output::write_pretty_json(options.out.as_ref(), run)
}

// Return restore command usage text.
const fn usage() -> &'static str {
    "usage: canic restore <command> [<args>]\n\ncommands:\n  plan           Build a no-mutation restore plan.\n  status         Build initial restore status from a plan.\n  apply          Render restore operations and optionally write an apply journal.\n  apply-status   Summarize apply journal state for scripts.\n  apply-report   Write an operator-focused apply journal report.\n  run            Preview, execute, or recover the native restore runner."
}

#[cfg(test)]
mod tests;
