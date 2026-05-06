use super::{BackupCommandError, usage};
use clap::{Arg, ArgAction, ArgMatches, Command as ClapCommand};
use std::{ffi::OsString, path::PathBuf};

///
/// BackupPreflightOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupPreflightOptions {
    pub dir: PathBuf,
    pub out_dir: PathBuf,
    pub mapping: Option<PathBuf>,
    pub require_design_v1: bool,
    pub require_restore_ready: bool,
}

impl BackupPreflightOptions {
    /// Parse backup preflight options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_preflight_command(), args)?;

        Ok(Self {
            dir: required_path_option(&matches, "dir", "--dir")?,
            out_dir: required_path_option(&matches, "out-dir", "--out-dir")?,
            mapping: path_option(&matches, "mapping"),
            require_design_v1: matches.get_flag("require-design"),
            require_restore_ready: matches.get_flag("require-restore-ready"),
        })
    }
}

// Build the backup preflight parser.
fn backup_preflight_command() -> ClapCommand {
    backup_dir_out_dir_command("backup-preflight")
        .arg(value_arg("mapping").long("mapping"))
        .arg(require_design_arg())
        .arg(flag_arg("require-restore-ready").long("require-restore-ready"))
}

///
/// BackupSmokeOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupSmokeOptions {
    pub dir: PathBuf,
    pub out_dir: PathBuf,
    pub mapping: Option<PathBuf>,
    pub dfx: String,
    pub network: Option<String>,
    pub require_design_v1: bool,
    pub require_restore_ready: bool,
}

impl BackupSmokeOptions {
    /// Parse backup smoke-check options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_smoke_command(), args)?;

        Ok(Self {
            dir: required_path_option(&matches, "dir", "--dir")?,
            out_dir: required_path_option(&matches, "out-dir", "--out-dir")?,
            mapping: path_option(&matches, "mapping"),
            dfx: string_option(&matches, "dfx").unwrap_or_else(|| "dfx".to_string()),
            network: string_option(&matches, "network"),
            require_design_v1: matches.get_flag("require-design"),
            require_restore_ready: matches.get_flag("require-restore-ready"),
        })
    }
}

// Build the backup smoke parser.
fn backup_smoke_command() -> ClapCommand {
    backup_dir_out_dir_command("backup-smoke")
        .arg(value_arg("mapping").long("mapping"))
        .arg(value_arg("dfx").long("dfx"))
        .arg(value_arg("network").long("network"))
        .arg(require_design_arg())
        .arg(flag_arg("require-restore-ready").long("require-restore-ready"))
}

///
/// BackupInspectOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupInspectOptions {
    pub dir: PathBuf,
    pub out: Option<PathBuf>,
    pub require_ready: bool,
}

impl BackupInspectOptions {
    /// Parse backup inspection options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_inspect_command(), args)?;

        Ok(Self {
            dir: required_path_option(&matches, "dir", "--dir")?,
            out: path_option(&matches, "out"),
            require_ready: matches.get_flag("require-ready"),
        })
    }
}

// Build the backup inspect parser.
fn backup_inspect_command() -> ClapCommand {
    backup_dir_out_command("backup-inspect").arg(flag_arg("require-ready").long("require-ready"))
}

///
/// BackupProvenanceOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupProvenanceOptions {
    pub dir: PathBuf,
    pub out: Option<PathBuf>,
    pub require_consistent: bool,
}

impl BackupProvenanceOptions {
    /// Parse backup provenance options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_provenance_command(), args)?;

        Ok(Self {
            dir: required_path_option(&matches, "dir", "--dir")?,
            out: path_option(&matches, "out"),
            require_consistent: matches.get_flag("require-consistent"),
        })
    }
}

// Build the backup provenance parser.
fn backup_provenance_command() -> ClapCommand {
    backup_dir_out_command("backup-provenance")
        .arg(flag_arg("require-consistent").long("require-consistent"))
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
    let name = command.get_name().to_string();
    command
        .try_get_matches_from(std::iter::once(OsString::from(name)).chain(args))
        .map_err(|_| BackupCommandError::Usage(usage()))
}

// Build the common --dir/--out parser shape.
fn backup_dir_out_command(name: &'static str) -> ClapCommand {
    ClapCommand::new(name)
        .disable_help_flag(true)
        .arg(value_arg("dir").long("dir"))
        .arg(value_arg("out").long("out"))
}

// Build the common --dir/--out-dir parser shape.
fn backup_dir_out_dir_command(name: &'static str) -> ClapCommand {
    ClapCommand::new(name)
        .disable_help_flag(true)
        .arg(value_arg("dir").long("dir"))
        .arg(value_arg("out-dir").long("out-dir"))
}

// Build one string-valued Clap argument.
fn value_arg(id: &'static str) -> Arg {
    Arg::new(id).num_args(1)
}

// Build one boolean Clap argument.
fn flag_arg(id: &'static str) -> Arg {
    Arg::new(id).action(ArgAction::SetTrue)
}

// Build the current design-conformance flag.
fn require_design_arg() -> Arg {
    flag_arg("require-design").long("require-design")
}

// Read one string option from Clap matches.
fn string_option(matches: &ArgMatches, id: &str) -> Option<String> {
    matches.get_one::<String>(id).cloned()
}

// Read one optional path from Clap matches.
fn path_option(matches: &ArgMatches, id: &str) -> Option<PathBuf> {
    string_option(matches, id).map(PathBuf::from)
}

// Read one required path from Clap matches.
fn required_path_option(
    matches: &ArgMatches,
    id: &str,
    option: &'static str,
) -> Result<PathBuf, BackupCommandError> {
    path_option(matches, id).ok_or(BackupCommandError::MissingOption(option))
}
