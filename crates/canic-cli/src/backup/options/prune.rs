//! Module: backup::options::prune
//!
//! Responsibility: parse `canic backup prune` options.
//! Does not own: backup pruning execution.
//! Boundary: prune subcommand selectors, preview mode, and output path.

use super::shared::{DEFAULT_BACKUP_DIR, parse_backup_options};
use crate::{
    backup::{BackupCommandError, prune_usage},
    cli::clap::{flag_arg, path_option, required_path, typed_option, value_arg},
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

///
/// BackupPruneOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::backup) struct BackupPruneOptions {
    pub(in crate::backup) dir: PathBuf,
    pub(in crate::backup) keep: usize,
    pub(in crate::backup) dry_run: bool,
    pub(in crate::backup) out: Option<PathBuf>,
}

impl BackupPruneOptions {
    pub(in crate::backup) fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_prune_command(), prune_usage, args)?;
        Ok(Self {
            dir: required_path(&matches, "dir"),
            keep: typed_option(&matches, "keep").expect("--keep is required by clap"),
            dry_run: matches.get_flag("dry-run"),
            out: path_option(&matches, "out"),
        })
    }
}

pub(in crate::backup) fn backup_prune_command() -> ClapCommand {
    ClapCommand::new("prune")
        .bin_name("canic backup prune")
        .about("Remove selected backup directories")
        .disable_help_flag(true)
        .arg(
            value_arg("dir")
                .long("dir")
                .value_name("dir")
                .default_value(DEFAULT_BACKUP_DIR)
                .help("Backup root to scan; defaults to backups"),
        )
        .arg(
            value_arg("keep")
                .long("keep")
                .value_name("count")
                .value_parser(clap::value_parser!(usize))
                .required(true)
                .help("Keep the newest count completed backups"),
        )
        .arg(
            flag_arg("dry-run")
                .long("dry-run")
                .help("Preview selected backup directories without deleting them"),
        )
        .arg(value_arg("out").long("out").value_name("file"))
}
