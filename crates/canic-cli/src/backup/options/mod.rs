use crate::{
    cli::clap::{flag_arg, parse_matches, path_option, string_option, value_arg},
    cli::defaults::{default_icp, local_network},
    cli::globals::{internal_icp_arg, internal_network_arg},
};

use super::{
    BackupCommandError, create_usage, inspect_usage, list_usage, status_usage, verify_usage,
};
use clap::{ArgMatches, Command as ClapCommand};
use std::{ffi::OsString, path::PathBuf};

const BACKUP_REF: &str = "backup-ref";

///
/// BackupCreateOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupCreateOptions {
    pub fleet: String,
    pub subtree: Option<String>,
    pub out: Option<PathBuf>,
    pub dry_run: bool,
    pub network: String,
    pub icp: String,
}

impl BackupCreateOptions {
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_create_command(), create_usage, args)?;

        Ok(Self {
            fleet: string_option(&matches, "fleet").expect("clap requires fleet"),
            subtree: string_option(&matches, "subtree"),
            out: path_option(&matches, "out"),
            dry_run: matches.get_flag("dry-run"),
            network: string_option(&matches, "network").unwrap_or_else(local_network),
            icp: string_option(&matches, "icp").unwrap_or_else(default_icp),
        })
    }
}

pub(super) fn backup_create_command() -> ClapCommand {
    ClapCommand::new("create")
        .bin_name("canic backup create")
        .about("Create a topology-aware fleet backup")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Installed fleet name to back up"),
        )
        .arg(
            value_arg("subtree")
                .long("subtree")
                .value_name("role-or-principal")
                .help("Plan only one connected subtree"),
        )
        .arg(
            value_arg("out")
                .long("out")
                .value_name("dir")
                .help("Backup output directory; defaults to backups/fleet-<name>-YYYYMMDD-HHMMSS"),
        )
        .arg(
            flag_arg("dry-run")
                .long("dry-run")
                .help("Write the backup plan and execution journal without running it"),
        )
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

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
/// BackupInspectOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupInspectOptions {
    pub backup_ref: Option<String>,
    pub dir: Option<PathBuf>,
    pub out: Option<PathBuf>,
    pub json: bool,
}

impl BackupInspectOptions {
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_inspect_command(), inspect_usage, args)?;
        let (backup_ref, dir) = parse_backup_target(&matches, inspect_usage)?;

        Ok(Self {
            backup_ref,
            dir,
            out: path_option(&matches, "out"),
            json: matches.get_flag("json"),
        })
    }
}

pub(super) fn backup_inspect_command() -> ClapCommand {
    backup_dir_out_command(
        "inspect",
        "canic backup inspect",
        "Inspect a backup or dry-run plan layout",
    )
    .arg(flag_arg("json").long("json"))
}

///
/// BackupVerifyOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupVerifyOptions {
    pub backup_ref: Option<String>,
    pub dir: Option<PathBuf>,
    pub out: Option<PathBuf>,
}

impl BackupVerifyOptions {
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_verify_command(), verify_usage, args)?;
        let (backup_ref, dir) = parse_backup_target(&matches, verify_usage)?;

        Ok(Self {
            backup_ref,
            dir,
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
    pub backup_ref: Option<String>,
    pub dir: Option<PathBuf>,
    pub out: Option<PathBuf>,
    pub require_complete: bool,
}

impl BackupStatusOptions {
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_status_command(), status_usage, args)?;
        let (backup_ref, dir) = parse_backup_target(&matches, status_usage)?;

        Ok(Self {
            backup_ref,
            dir,
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
}

fn parse_backup_target(
    matches: &ArgMatches,
    usage: fn() -> String,
) -> Result<(Option<String>, Option<PathBuf>), BackupCommandError> {
    let backup_ref = string_option(matches, BACKUP_REF);
    let dir = path_option(matches, "dir");
    match (&backup_ref, &dir) {
        (Some(_), Some(_)) | (None, None) => Err(BackupCommandError::Usage(usage())),
        _ => Ok((backup_ref, dir)),
    }
}
