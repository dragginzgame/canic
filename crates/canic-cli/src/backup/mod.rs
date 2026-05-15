use crate::{cli::clap::parse_subcommand, cli::help::print_help_or_version, version_text};
use canic_backup::{
    discovery::DiscoveryError, execution::BackupExecutionJournalError,
    persistence::PersistenceError, plan::BackupPlanError, runner::BackupRunnerError,
};
#[cfg(test)]
use canic_backup::{
    persistence::BackupLayout,
    plan::{
        AuthorityEvidence, BackupPlan, BackupPlanBuildInput, BackupScopeKind, ControlAuthority,
        SnapshotReadAuthority, build_backup_plan,
    },
    registry::RegistryEntry,
};
use canic_host::registry::RegistryParseError;
use std::ffi::OsString;
use thiserror::Error as ThisError;

mod command;
mod create;
mod inspect;
mod labels;
mod model;
mod options;
mod reference;
mod render;
mod status;
mod verify;

use command::{
    backup_command, create_usage, inspect_usage, list_usage, status_usage, usage, verify_usage,
};
use create::backup_create;
#[cfg(test)]
use create::{persist_backup_create_dry_run, persist_backup_create_dry_run_with_layout};
use inspect::backup_inspect;
pub use model::{
    BackupCreateReport, BackupDryRunStatusReport, BackupInspectOperation, BackupInspectReport,
    BackupInspectTarget, BackupListEntry, BackupStatusReport,
};
pub use options::{
    BackupCreateOptions, BackupInspectOptions, BackupListOptions, BackupStatusOptions,
    BackupVerifyOptions,
};
use reference::backup_list;
#[cfg(test)]
use reference::resolve_backup_reference_in;
#[cfg(test)]
use render::{render_backup_list, render_create_report, render_inspect_report};
use render::{
    write_create_report, write_inspect_report, write_list_report, write_status_report,
    write_verify_report,
};
use status::{backup_status, enforce_status_requirements};
use verify::verify_backup;

///
/// BackupCommandError
///

#[derive(Debug, ThisError)]
pub enum BackupCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(
        "backup journal {backup_id} is incomplete: {pending_artifacts}/{total_artifacts} artifacts still require resume work"
    )]
    IncompleteJournal {
        backup_id: String,
        total_artifacts: usize,
        pending_artifacts: usize,
    },

    #[error("backup plan {plan_id} is a dry-run layout, not a complete backup")]
    DryRunNotComplete { plan_id: String },

    #[error("backup reference {reference} was not found under backups; run `canic backup list`")]
    BackupReferenceNotFound { reference: String },

    #[error("backup reference {reference} is ambiguous under backups; use `--dir <dir>`")]
    BackupReferenceAmbiguous { reference: String },

    #[error(
        "backup layout at --out is for a different request: {field} existing={existing}, requested={requested}"
    )]
    BackupLayoutMismatch {
        field: &'static str,
        existing: String,
        requested: String,
    },

    #[error("backup layout at --out is incomplete: missing {missing}")]
    BackupLayoutIncomplete { missing: &'static str },

    #[error(
        "fleet {fleet} is not installed on network {network}; run `canic install {fleet}` before planning a backup"
    )]
    NoInstalledFleet { network: String, fleet: String },

    #[error(
        "fleet {fleet} points to root {root}, but that canister is not present on local network {network}. Local ICP CLI replica state is not persistent; run `canic install {fleet}` to recreate it."
    )]
    LostLocalFleet {
        network: String,
        fleet: String,
        root: String,
    },

    #[error("failed to read canic fleet state: {0}")]
    InstallState(String),

    #[error("local replica query failed: {0}")]
    ReplicaQuery(String),

    #[error("icp command failed: {command}\n{stderr}")]
    IcpFailed { command: String, stderr: String },

    #[error("registry entry {canister_id} is not a valid principal")]
    InvalidRegistryPrincipal { canister_id: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Registry(#[from] RegistryParseError),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),

    #[error(transparent)]
    BackupPlan(#[from] BackupPlanError),

    #[error(transparent)]
    BackupExecutionJournal(#[from] BackupExecutionJournalError),

    #[error(transparent)]
    BackupRunner(#[from] BackupRunnerError),
}

pub fn run<I>(args: I) -> Result<(), BackupCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let Some((command, args)) =
        parse_subcommand(backup_command(), args).map_err(|_| BackupCommandError::Usage(usage()))?
    else {
        return Err(BackupCommandError::Usage(usage()));
    };

    match command.as_str() {
        "create" => {
            if print_help_or_version(&args, create_usage, version_text()) {
                return Ok(());
            }
            let options = BackupCreateOptions::parse(args)?;
            let report = backup_create(&options)?;
            write_create_report(&report);
            Ok(())
        }
        "list" => {
            if print_help_or_version(&args, list_usage, version_text()) {
                return Ok(());
            }
            let options = BackupListOptions::parse(args)?;
            let entries = backup_list(&options)?;
            write_list_report(&options, &entries)?;
            Ok(())
        }
        "inspect" => {
            if print_help_or_version(&args, inspect_usage, version_text()) {
                return Ok(());
            }
            let options = BackupInspectOptions::parse(args)?;
            let report = backup_inspect(&options)?;
            write_inspect_report(&options, &report)?;
            Ok(())
        }
        "status" => {
            if print_help_or_version(&args, status_usage, version_text()) {
                return Ok(());
            }
            let options = BackupStatusOptions::parse(args)?;
            let report = backup_status(&options)?;
            write_status_report(&options, &report)?;
            enforce_status_requirements(&options, &report)?;
            Ok(())
        }
        "verify" => {
            if print_help_or_version(&args, verify_usage, version_text()) {
                return Ok(());
            }
            let options = BackupVerifyOptions::parse(args)?;
            let report = verify_backup(&options)?;
            write_verify_report(&options, &report)?;
            Ok(())
        }
        _ => unreachable!("backup dispatch command only defines known commands"),
    }
}

#[cfg(test)]
mod tests;
