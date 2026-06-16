//! Module: backup::options::inspect
//!
//! Responsibility: parse `canic backup inspect` options.
//! Does not own: backup inspection execution.
//! Boundary: inspect source selector, JSON flag, and output path.

use super::shared::{backup_dir_out_command, backup_target, parse_backup_options};
use crate::{
    backup::{BackupCommandError, inspect_usage},
    cli::clap::{flag_arg, path_option},
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

///
/// BackupInspectOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::backup) struct BackupInspectOptions {
    pub(in crate::backup) backup_ref: Option<String>,
    pub(in crate::backup) dir: Option<PathBuf>,
    pub(in crate::backup) out: Option<PathBuf>,
    pub(in crate::backup) json: bool,
}

impl BackupInspectOptions {
    pub(in crate::backup) fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_inspect_command(), inspect_usage, args)?;
        let (backup_ref, dir) = backup_target(&matches);

        Ok(Self {
            backup_ref,
            dir,
            out: path_option(&matches, "out"),
            json: matches.get_flag("json"),
        })
    }
}

pub(in crate::backup) fn backup_inspect_command() -> ClapCommand {
    backup_dir_out_command(
        "inspect",
        "canic backup inspect",
        "Inspect a backup or dry-run plan layout",
    )
    .arg(flag_arg("json").long("json"))
}
