//! Module: backup::options::status
//!
//! Responsibility: parse `canic backup status` options.
//! Does not own: backup status report generation.
//! Boundary: status source selector, completion requirement, and output path.

use super::shared::{backup_dir_out_command, backup_target, parse_backup_options};
use crate::{
    backup::{BackupCommandError, status_usage},
    cli::clap::{flag_arg, path_option},
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

///
/// BackupStatusOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::backup) struct BackupStatusOptions {
    pub(in crate::backup) backup_ref: Option<String>,
    pub(in crate::backup) dir: Option<PathBuf>,
    pub(in crate::backup) out: Option<PathBuf>,
    pub(in crate::backup) require_complete: bool,
}

impl BackupStatusOptions {
    pub(in crate::backup) fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_status_command(), status_usage, args)?;
        let (backup_ref, dir) = backup_target(&matches);

        Ok(Self {
            backup_ref,
            dir,
            out: path_option(&matches, "out"),
            require_complete: matches.get_flag("require-complete"),
        })
    }
}

pub(in crate::backup) fn backup_status_command() -> ClapCommand {
    backup_dir_out_command(
        "status",
        "canic backup status",
        "Summarize resumable download journal state",
    )
    .arg(flag_arg("require-complete").long("require-complete"))
}
