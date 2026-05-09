use crate::args::{default_icp, flag_arg, parse_matches, path_option, string_option, value_arg};
use clap::{ArgMatches, Command as ClapCommand};
use std::{ffi::OsString, path::PathBuf};

use super::{RestoreCommandError, apply_usage, plan_usage, run_usage};

///
/// RestorePlanOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestorePlanOptions {
    pub manifest: Option<PathBuf>,
    pub backup_dir: Option<PathBuf>,
    pub mapping: Option<PathBuf>,
    pub out: Option<PathBuf>,
    pub require_verified: bool,
    pub require_restore_ready: bool,
}

impl RestorePlanOptions {
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(restore_plan_command(), args)
            .map_err(|_| RestoreCommandError::Usage(plan_usage()))?;

        let manifest = path_option(&matches, "manifest");
        let backup_dir = path_option(&matches, "backup-dir");
        let require_verified = matches.get_flag("require-verified");

        if manifest.is_some() && backup_dir.is_some() {
            return Err(RestoreCommandError::ConflictingManifestSources);
        }

        if manifest.is_none() && backup_dir.is_none() {
            return Err(RestoreCommandError::MissingOption(
                "--manifest or --backup-dir",
            ));
        }

        if require_verified && backup_dir.is_none() {
            return Err(RestoreCommandError::RequireVerifiedNeedsBackupDir);
        }

        Ok(Self {
            manifest,
            backup_dir,
            mapping: path_option(&matches, "mapping"),
            out: path_option(&matches, "out"),
            require_verified,
            require_restore_ready: matches.get_flag("require-restore-ready"),
        })
    }
}

pub(super) fn restore_plan_command() -> ClapCommand {
    ClapCommand::new("plan")
        .bin_name("canic restore plan")
        .about("Build a no-mutation restore plan")
        .disable_help_flag(true)
        .arg(value_arg("manifest").long("manifest").value_name("file"))
        .arg(value_arg("backup-dir").long("backup-dir").value_name("dir"))
        .arg(value_arg("mapping").long("mapping").value_name("file"))
        .arg(value_arg("out").long("out").value_name("file"))
        .arg(flag_arg("require-verified").long("require-verified"))
        .arg(flag_arg("require-restore-ready").long("require-restore-ready"))
}

///
/// RestoreApplyOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreApplyOptions {
    pub plan: PathBuf,
    pub backup_dir: Option<PathBuf>,
    pub out: Option<PathBuf>,
    pub journal_out: Option<PathBuf>,
    pub dry_run: bool,
}

impl RestoreApplyOptions {
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(restore_apply_command(), args)
            .map_err(|_| RestoreCommandError::Usage(apply_usage()))?;
        let dry_run = matches.get_flag("dry-run");

        if !dry_run {
            return Err(RestoreCommandError::ApplyRequiresDryRun);
        }

        Ok(Self {
            plan: path_option(&matches, "plan")
                .ok_or(RestoreCommandError::MissingOption("--plan"))?,
            backup_dir: path_option(&matches, "backup-dir"),
            out: path_option(&matches, "out"),
            journal_out: path_option(&matches, "journal-out"),
            dry_run,
        })
    }
}

pub(super) fn restore_apply_command() -> ClapCommand {
    ClapCommand::new("apply")
        .bin_name("canic restore apply")
        .about("Render restore operations and optionally write an apply journal")
        .disable_help_flag(true)
        .arg(value_arg("plan").long("plan").value_name("file"))
        .arg(value_arg("backup-dir").long("backup-dir").value_name("dir"))
        .arg(value_arg("out").long("out").value_name("file"))
        .arg(
            value_arg("journal-out")
                .long("journal-out")
                .value_name("file"),
        )
        .arg(flag_arg("dry-run").long("dry-run"))
}

///
/// RestoreRunOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "CLI runner options mirror three mutually exclusive mode flags and two operator guard flags"
)]
pub struct RestoreRunOptions {
    pub journal: PathBuf,
    pub icp: String,
    pub network: Option<String>,
    pub out: Option<PathBuf>,
    pub dry_run: bool,
    pub execute: bool,
    pub unclaim_pending: bool,
    pub max_steps: Option<usize>,
    pub require_complete: bool,
    pub require_no_attention: bool,
}

impl RestoreRunOptions {
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(restore_run_command(), args)
            .map_err(|_| RestoreCommandError::Usage(run_usage()))?;

        let dry_run = matches.get_flag("dry-run");
        let execute = matches.get_flag("execute");
        let unclaim_pending = matches.get_flag("unclaim-pending");

        validate_restore_run_mode_selection(dry_run, execute, unclaim_pending)?;

        Ok(Self {
            journal: path_option(&matches, "journal")
                .ok_or(RestoreCommandError::MissingOption("--journal"))?,
            icp: string_option(&matches, "icp").unwrap_or_else(default_icp),
            network: string_option(&matches, "network"),
            out: path_option(&matches, "out"),
            dry_run,
            execute,
            unclaim_pending,
            max_steps: positive_integer_option(&matches, "max-steps", "--max-steps")?,
            require_complete: matches.get_flag("require-complete"),
            require_no_attention: matches.get_flag("require-no-attention"),
        })
    }
}

pub(super) fn restore_run_command() -> ClapCommand {
    ClapCommand::new("run")
        .bin_name("canic restore run")
        .about("Preview, execute, or recover the native restore runner")
        .disable_help_flag(true)
        .arg(value_arg("journal").long("journal").value_name("file"))
        .arg(value_arg("icp").long("icp").value_name("path"))
        .arg(value_arg("network").long("network").value_name("name"))
        .arg(value_arg("out").long("out").value_name("file"))
        .arg(flag_arg("dry-run").long("dry-run"))
        .arg(flag_arg("execute").long("execute"))
        .arg(flag_arg("unclaim-pending").long("unclaim-pending"))
        .arg(value_arg("max-steps").long("max-steps").value_name("count"))
        .arg(flag_arg("require-complete").long("require-complete"))
        .arg(flag_arg("require-no-attention").long("require-no-attention"))
}

fn positive_integer_option(
    matches: &ArgMatches,
    id: &str,
    option: &'static str,
) -> Result<Option<usize>, RestoreCommandError> {
    string_option(matches, id)
        .map(|value| parse_positive_integer(option, value))
        .transpose()
}

fn validate_restore_run_mode_selection(
    dry_run: bool,
    execute: bool,
    unclaim_pending: bool,
) -> Result<(), RestoreCommandError> {
    let mode_count = [dry_run, execute, unclaim_pending]
        .into_iter()
        .filter(|enabled| *enabled)
        .count();
    if mode_count > 1 {
        return Err(RestoreCommandError::RestoreRunConflictingModes);
    }

    if mode_count == 0 {
        return Err(RestoreCommandError::RestoreRunRequiresMode);
    }

    Ok(())
}

fn parse_sequence(value: String) -> Result<usize, RestoreCommandError> {
    value
        .parse::<usize>()
        .map_err(|_| RestoreCommandError::InvalidSequence)
}

fn parse_positive_integer(
    option: &'static str,
    value: String,
) -> Result<usize, RestoreCommandError> {
    let parsed = parse_sequence(value)?;
    if parsed == 0 {
        return Err(RestoreCommandError::InvalidPositiveInteger { option });
    }

    Ok(parsed)
}
