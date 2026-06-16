//! Module: backup::options::shared
//!
//! Responsibility: shared backup option parser helpers.
//! Does not own: subcommand-specific option construction.
//! Boundary: reusable Clap command fragments and selector extraction.

use crate::{
    backup::BackupCommandError,
    cli::clap::{parse_matches, path_option, string_option, value_arg},
};
use clap::{ArgGroup, ArgMatches, Command as ClapCommand};
use std::{ffi::OsString, path::PathBuf};

pub(super) const BACKUP_REF: &str = "backup-ref";
pub(super) const DEFAULT_BACKUP_DIR: &str = "backups";

pub(super) fn parse_backup_options<I>(
    command: ClapCommand,
    usage: fn() -> String,
    args: I,
) -> Result<ArgMatches, BackupCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    parse_matches(command, args).map_err(|_| BackupCommandError::Usage(usage()))
}

pub(super) fn backup_dir_out_command(
    name: &'static str,
    bin_name: &'static str,
    about: &'static str,
) -> ClapCommand {
    ClapCommand::new(name)
        .bin_name(bin_name)
        .about(about)
        .disable_help_flag(true)
        .arg(
            value_arg(BACKUP_REF)
                .value_name("backup-ref")
                .help("Backup row number or BACKUP_ID from `canic backup list`"),
        )
        .arg(
            value_arg("dir")
                .long("dir")
                .value_name("dir")
                .help("Explicit backup directory path"),
        )
        .arg(value_arg("out").long("out").value_name("file"))
        .group(
            ArgGroup::new("backup-source")
                .args([BACKUP_REF, "dir"])
                .required(true)
                .multiple(false),
        )
}

pub(super) fn backup_target(matches: &ArgMatches) -> (Option<String>, Option<PathBuf>) {
    let backup_ref = string_option(matches, BACKUP_REF);
    let dir = path_option(matches, "dir");
    (backup_ref, dir)
}
