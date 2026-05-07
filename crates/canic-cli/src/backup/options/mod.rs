use crate::args::{flag_arg, parse_matches, path_option, value_arg};

use super::{BackupCommandError, usage};
use clap::{ArgMatches, Command as ClapCommand};
use std::{ffi::OsString, path::PathBuf};

///
/// BackupVerifyOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupVerifyOptions {
    pub dir: PathBuf,
    pub out: Option<PathBuf>,
}

impl BackupVerifyOptions {
    /// Parse backup verification options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_verify_command(), args)?;

        Ok(Self {
            dir: required_path_option(&matches, "dir", "--dir")?,
            out: path_option(&matches, "out"),
        })
    }
}

// Build the backup verify parser.
fn backup_verify_command() -> ClapCommand {
    backup_dir_out_command("backup-verify")
}

///
/// BackupStatusOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupStatusOptions {
    pub dir: PathBuf,
    pub out: Option<PathBuf>,
    pub require_complete: bool,
}

impl BackupStatusOptions {
    /// Parse backup status options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_status_command(), args)?;

        Ok(Self {
            dir: required_path_option(&matches, "dir", "--dir")?,
            out: path_option(&matches, "out"),
            require_complete: matches.get_flag("require-complete"),
        })
    }
}

// Build the backup status parser.
fn backup_status_command() -> ClapCommand {
    backup_dir_out_command("backup-status")
        .arg(flag_arg("require-complete").long("require-complete"))
}

// Parse one backup command option set.
fn parse_backup_options<I>(command: ClapCommand, args: I) -> Result<ArgMatches, BackupCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    parse_matches(command, args).map_err(|_| BackupCommandError::Usage(usage()))
}

// Build the common --dir/--out parser shape.
fn backup_dir_out_command(name: &'static str) -> ClapCommand {
    ClapCommand::new(name)
        .disable_help_flag(true)
        .arg(value_arg("dir").long("dir"))
        .arg(value_arg("out").long("out"))
}

// Read one required path from Clap matches.
fn required_path_option(
    matches: &ArgMatches,
    id: &str,
    option: &'static str,
) -> Result<PathBuf, BackupCommandError> {
    path_option(matches, id).ok_or(BackupCommandError::MissingOption(option))
}
