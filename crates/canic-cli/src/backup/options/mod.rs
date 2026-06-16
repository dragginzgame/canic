//! Module: backup::options
//!
//! Responsibility: typed option parsing for `canic backup` subcommands.
//! Does not own: backup command dispatch or command execution.
//! Boundary: command-line argument parsing into backup option structs.

mod create;
mod inspect;
mod list;
mod prune;
mod shared;
mod status;
mod verify;

pub(super) use create::{BackupCreateOptions, backup_create_command};
pub(super) use inspect::{BackupInspectOptions, backup_inspect_command};
pub(super) use list::{BackupListOptions, backup_list_command};
pub(super) use prune::{BackupPruneOptions, backup_prune_command};
pub(super) use status::{BackupStatusOptions, backup_status_command};
pub(super) use verify::{BackupVerifyOptions, backup_verify_command};
