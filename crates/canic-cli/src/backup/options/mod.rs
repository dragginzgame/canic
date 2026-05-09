use crate::args::{flag_arg, parse_matches, path_option, value_arg};

use super::{BackupCommandError, list_usage, status_usage, verify_usage};
use clap::{ArgMatches, Command as ClapCommand};
use std::{ffi::OsString, path::PathBuf};

///
/// BackupListOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupListOptions {
    pub dir: PathBuf,
    pub out: Option<PathBuf>,
}

impl BackupListOptions {
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_list_command(), list_usage, args)?;

        Ok(Self {
            dir: path_option(&matches, "dir").unwrap_or_else(|| PathBuf::from("backups")),
            out: path_option(&matches, "out"),
        })
    }
}

pub(super) fn backup_list_command() -> ClapCommand {
    ClapCommand::new("list")
        .bin_name("canic backup list")
        .about("List backup directories under a backup root")
        .disable_help_flag(true)
        .arg(
            value_arg("dir")
                .long("dir")
                .value_name("dir")
                .help("Backup root to scan; defaults to backups"),
        )
        .arg(value_arg("out").long("out").value_name("file"))
}

///
/// BackupVerifyOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupVerifyOptions {
    pub dir: PathBuf,
    pub out: Option<PathBuf>,
}

impl BackupVerifyOptions {
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_verify_command(), verify_usage, args)?;

        Ok(Self {
            dir: path_option(&matches, "dir").expect("clap requires dir"),
            out: path_option(&matches, "out"),
        })
    }
}

pub(super) fn backup_verify_command() -> ClapCommand {
    backup_dir_out_command(
        "verify",
        "canic backup verify",
        "Verify layout, journal agreement, and durable artifact checksums",
    )
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
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_status_command(), status_usage, args)?;

        Ok(Self {
            dir: path_option(&matches, "dir").expect("clap requires dir"),
            out: path_option(&matches, "out"),
            require_complete: matches.get_flag("require-complete"),
        })
    }
}

pub(super) fn backup_status_command() -> ClapCommand {
    backup_dir_out_command(
        "status",
        "canic backup status",
        "Summarize resumable download journal state",
    )
    .arg(flag_arg("require-complete").long("require-complete"))
}

fn parse_backup_options<I>(
    command: ClapCommand,
    usage: fn() -> String,
    args: I,
) -> Result<ArgMatches, BackupCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    parse_matches(command, args).map_err(|_| BackupCommandError::Usage(usage()))
}

fn backup_dir_out_command(
    name: &'static str,
    bin_name: &'static str,
    about: &'static str,
) -> ClapCommand {
    ClapCommand::new(name)
        .bin_name(bin_name)
        .about(about)
        .disable_help_flag(true)
        .arg(
            value_arg("dir")
                .long("dir")
                .value_name("dir")
                .required(true),
        )
        .arg(value_arg("out").long("out").value_name("file"))
}
