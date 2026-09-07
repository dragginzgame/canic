mod command;
mod create;
mod dispatch;
mod error;
mod inspect;
mod labels;
mod layout;
mod manifest;
mod model;
mod options;
mod prune;
mod reference;
mod render;
mod status;
mod verify;

use command::{
    backup_command, create_usage, inspect_usage, list_usage, prune_usage, status_usage, usage,
    verify_usage,
};
use create::backup_create;
#[cfg(test)]
use create::{persist_backup_create_dry_run, persist_backup_create_dry_run_with_layout};
pub use dispatch::run;
pub use error::BackupCommandError;
use inspect::backup_inspect;
use manifest::run as run_manifest;
pub use model::{
    BackupCreateLayout, BackupCreateMode, BackupCreateReport, BackupDryRunStatusReport,
    BackupExecutionLayoutStatus, BackupInspectOperation, BackupInspectReport, BackupInspectTarget,
    BackupListEntry, BackupListStatus, BackupPruneAction, BackupPruneEntry, BackupPruneReport,
    BackupRunStatus, BackupStatusReport,
};
use options::{
    BackupCreateOptions, BackupInspectOptions, BackupListOptions, BackupPruneOptions,
    BackupStatusOptions, BackupVerifyOptions,
};
use prune::backup_prune;
use reference::backup_list;
pub use reference::resolve_backup_reference;
#[cfg(test)]
use reference::resolve_backup_reference_in;
use render::{
    write_create_report, write_inspect_report, write_list_report, write_prune_report,
    write_status_report, write_verify_report,
};
use status::{backup_status, enforce_status_requirements};
use verify::verify_backup;

#[cfg(test)]
mod tests;
