//! Module: backup::options::list
//!
//! Responsibility: parse `canic backup list` options.
//! Does not own: backup directory scanning.
//! Boundary: list subcommand argument shape and defaults.

use super::shared::{DEFAULT_BACKUP_DIR, parse_backup_options};
use crate::{
    backup::{BackupCommandError, list_usage},
    cli::clap::{path_option, required_path, value_arg},
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

///
/// BackupListOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::backup) struct BackupListOptions {
    pub(in crate::backup) dir: PathBuf,
    pub(in crate::backup) out: Option<PathBuf>,
}

impl BackupListOptions {
    pub(in crate::backup) fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_list_command(), list_usage, args)?;

        Ok(Self {
            dir: required_path(&matches, "dir"),
            out: path_option(&matches, "out"),
        })
    }
}

pub(in crate::backup) fn backup_list_command() -> ClapCommand {
    ClapCommand::new("list")
        .bin_name("canic backup list")
        .about("List backup directories under a backup root")
        .disable_help_flag(true)
        .arg(
            value_arg("dir")
                .long("dir")
                .value_name("dir")
                .default_value(DEFAULT_BACKUP_DIR)
                .help("Backup root to scan; defaults to backups"),
        )
        .arg(value_arg("out").long("out").value_name("file"))
}
