use crate::args::{flag_arg, parse_matches, path_option, string_option, value_arg};
use clap::{ArgMatches, Command as ClapCommand};
use std::{ffi::OsString, path::PathBuf};

use super::{RestoreCommandError, usage};

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
    /// Parse restore planning options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(restore_plan_command(), args)
            .map_err(|_| RestoreCommandError::Usage(usage()))?;

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

// Build the restore plan parser.
fn restore_plan_command() -> ClapCommand {
    ClapCommand::new("restore-plan")
        .disable_help_flag(true)
        .arg(value_arg("manifest").long("manifest"))
        .arg(value_arg("backup-dir").long("backup-dir"))
        .arg(value_arg("mapping").long("mapping"))
        .arg(value_arg("out").long("out"))
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
    /// Parse restore apply options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(restore_apply_command(), args)
            .map_err(|_| RestoreCommandError::Usage(usage()))?;
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

// Build the restore apply dry-run parser.
fn restore_apply_command() -> ClapCommand {
    ClapCommand::new("restore-apply")
        .disable_help_flag(true)
        .arg(value_arg("plan").long("plan"))
        .arg(value_arg("backup-dir").long("backup-dir"))
        .arg(value_arg("out").long("out"))
        .arg(value_arg("journal-out").long("journal-out"))
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
    pub dfx: String,
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
    /// Parse restore run options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(restore_run_command(), args)
            .map_err(|_| RestoreCommandError::Usage(usage()))?;

        let dry_run = matches.get_flag("dry-run");
        let execute = matches.get_flag("execute");
        let unclaim_pending = matches.get_flag("unclaim-pending");

        validate_restore_run_mode_selection(dry_run, execute, unclaim_pending)?;

        Ok(Self {
            journal: path_option(&matches, "journal")
                .ok_or(RestoreCommandError::MissingOption("--journal"))?,
            dfx: string_option(&matches, "dfx").unwrap_or_else(|| "dfx".to_string()),
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

// Build the native restore runner parser.
fn restore_run_command() -> ClapCommand {
    ClapCommand::new("restore-run")
        .disable_help_flag(true)
        .arg(value_arg("journal").long("journal"))
        .arg(value_arg("dfx").long("dfx"))
        .arg(value_arg("network").long("network"))
        .arg(value_arg("out").long("out"))
        .arg(flag_arg("dry-run").long("dry-run"))
        .arg(flag_arg("execute").long("execute"))
        .arg(flag_arg("unclaim-pending").long("unclaim-pending"))
        .arg(value_arg("max-steps").long("max-steps"))
        .arg(flag_arg("require-complete").long("require-complete"))
        .arg(flag_arg("require-no-attention").long("require-no-attention"))
}

// Read one positive integer option from Clap matches.
fn positive_integer_option(
    matches: &ArgMatches,
    id: &str,
    option: &'static str,
) -> Result<Option<usize>, RestoreCommandError> {
    string_option(matches, id)
        .map(|value| parse_positive_integer(option, value))
        .transpose()
}

// Validate that restore run received exactly one execution mode.
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

// Parse a restore apply journal operation sequence value.
fn parse_sequence(value: String) -> Result<usize, RestoreCommandError> {
    value
        .parse::<usize>()
        .map_err(|_| RestoreCommandError::InvalidSequence)
}

// Parse a positive integer CLI value for options where zero is not meaningful.
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
