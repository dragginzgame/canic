use crate::{
    cli::clap::{flag_arg, parse_matches, path_option, string_option, value_arg},
    cli::defaults::default_icp,
    cli::globals::{internal_icp_arg, internal_network_arg},
};
use clap::{ArgGroup, Command as ClapCommand};
use std::{ffi::OsString, path::PathBuf};

use super::{RestoreCommandError, apply_usage, plan_usage, prepare_usage, run_usage, status_usage};

const BACKUP_REF: &str = "backup-ref";

///
/// RestorePlanOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestorePlanOptions {
    pub backup_ref: Option<String>,
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

        let options = Self {
            manifest: path_option(&matches, "manifest"),
            backup_ref: string_option(&matches, BACKUP_REF),
            backup_dir: path_option(&matches, "backup-dir"),
            mapping: path_option(&matches, "mapping"),
            out: path_option(&matches, "out"),
            require_verified: matches.get_flag("require-verified"),
            require_restore_ready: matches.get_flag("require-restore-ready"),
        };
        if options.require_verified && options.manifest.is_some() {
            return Err(RestoreCommandError::Usage(plan_usage()));
        }

        Ok(options)
    }
}

pub(super) fn restore_plan_command() -> ClapCommand {
    ClapCommand::new("plan")
        .bin_name("canic restore plan")
        .about("Build a no-mutation restore plan")
        .disable_help_flag(true)
        .group(
            ArgGroup::new("manifest-source")
                .args([BACKUP_REF, "manifest", "backup-dir"])
                .required(true)
                .multiple(false),
        )
        .arg(value_arg(BACKUP_REF).value_name(BACKUP_REF))
        .arg(value_arg("manifest").long("manifest").value_name("file"))
        .arg(value_arg("backup-dir").long("backup-dir").value_name("dir"))
        .arg(value_arg("mapping").long("mapping").value_name("file"))
        .arg(value_arg("out").long("out").value_name("file"))
        .arg(flag_arg("require-verified").long("require-verified"))
        .arg(flag_arg("require-restore-ready").long("require-restore-ready"))
}

///
/// RestorePrepareOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestorePrepareOptions {
    pub backup_ref: Option<String>,
    pub backup_dir: Option<PathBuf>,
    pub mapping: Option<PathBuf>,
    pub out: Option<PathBuf>,
    pub plan_out: Option<PathBuf>,
    pub journal_out: Option<PathBuf>,
    pub require_verified: bool,
    pub require_restore_ready: bool,
}

impl RestorePrepareOptions {
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(restore_prepare_command(), args)
            .map_err(|_| RestoreCommandError::Usage(prepare_usage()))?;

        Ok(Self {
            backup_ref: string_option(&matches, BACKUP_REF),
            backup_dir: path_option(&matches, "backup-dir"),
            mapping: path_option(&matches, "mapping"),
            out: path_option(&matches, "out"),
            plan_out: path_option(&matches, "plan-out"),
            journal_out: path_option(&matches, "journal-out"),
            require_verified: matches.get_flag("require-verified"),
            require_restore_ready: matches.get_flag("require-restore-ready"),
        })
    }
}

pub(super) fn restore_prepare_command() -> ClapCommand {
    ClapCommand::new("prepare")
        .bin_name("canic restore prepare")
        .about("Prepare a backup layout for restore")
        .disable_help_flag(true)
        .group(
            ArgGroup::new("backup-source")
                .args([BACKUP_REF, "backup-dir"])
                .required(true)
                .multiple(false),
        )
        .arg(value_arg(BACKUP_REF).value_name(BACKUP_REF))
        .arg(value_arg("backup-dir").long("backup-dir").value_name("dir"))
        .arg(value_arg("mapping").long("mapping").value_name("file"))
        .arg(value_arg("out").long("out").value_name("file"))
        .arg(value_arg("plan-out").long("plan-out").value_name("file"))
        .arg(
            value_arg("journal-out")
                .long("journal-out")
                .value_name("file"),
        )
        .arg(flag_arg("require-verified").long("require-verified"))
        .arg(flag_arg("require-restore-ready").long("require-restore-ready"))
}

///
/// RestoreApplyOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreApplyOptions {
    pub backup_ref: Option<String>,
    pub plan: Option<PathBuf>,
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

        Ok(Self {
            backup_ref: string_option(&matches, BACKUP_REF),
            plan: path_option(&matches, "plan"),
            backup_dir: path_option(&matches, "backup-dir"),
            out: path_option(&matches, "out"),
            journal_out: path_option(&matches, "journal-out"),
            dry_run: matches.get_flag("dry-run"),
        })
    }
}

pub(super) fn restore_apply_command() -> ClapCommand {
    ClapCommand::new("apply")
        .bin_name("canic restore apply")
        .about("Render restore operations and optionally write an apply journal")
        .disable_help_flag(true)
        .group(
            ArgGroup::new("plan-source")
                .args([BACKUP_REF, "plan"])
                .required(true)
                .multiple(false),
        )
        .arg(value_arg(BACKUP_REF).value_name(BACKUP_REF))
        .arg(value_arg("plan").long("plan").value_name("file"))
        .arg(value_arg("backup-dir").long("backup-dir").value_name("dir"))
        .arg(value_arg("out").long("out").value_name("file"))
        .arg(
            value_arg("journal-out")
                .long("journal-out")
                .value_name("file"),
        )
        .arg(flag_arg("dry-run").long("dry-run").required(true))
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
    pub backup_ref: Option<String>,
    pub journal: Option<PathBuf>,
    pub icp: String,
    pub network: Option<String>,
    pub out: Option<PathBuf>,
    pub dry_run: bool,
    pub execute: bool,
    pub retry_failed: bool,
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

        Ok(Self {
            backup_ref: string_option(&matches, BACKUP_REF),
            journal: path_option(&matches, "journal"),
            icp: string_option(&matches, "icp").unwrap_or_else(default_icp),
            network: string_option(&matches, "network"),
            out: path_option(&matches, "out"),
            dry_run: matches.get_flag("dry-run"),
            execute: matches.get_flag("execute"),
            retry_failed: matches.get_flag("retry-failed"),
            unclaim_pending: matches.get_flag("unclaim-pending"),
            max_steps: matches.get_one::<usize>("max-steps").copied(),
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
        .group(
            ArgGroup::new("mode")
                .args(["dry-run", "execute", "retry-failed", "unclaim-pending"])
                .required(true)
                .multiple(false),
        )
        .group(
            ArgGroup::new("journal-source")
                .args([BACKUP_REF, "journal"])
                .required(true)
                .multiple(false),
        )
        .arg(value_arg(BACKUP_REF).value_name(BACKUP_REF))
        .arg(value_arg("journal").long("journal").value_name("file"))
        .arg(internal_icp_arg())
        .arg(internal_network_arg())
        .arg(value_arg("out").long("out").value_name("file"))
        .arg(flag_arg("dry-run").long("dry-run"))
        .arg(flag_arg("execute").long("execute"))
        .arg(flag_arg("retry-failed").long("retry-failed"))
        .arg(flag_arg("unclaim-pending").long("unclaim-pending"))
        .arg(
            value_arg("max-steps")
                .long("max-steps")
                .value_name("count")
                .value_parser(clap::builder::ValueParser::new(parse_positive_usize)),
        )
        .arg(flag_arg("require-complete").long("require-complete"))
        .arg(flag_arg("require-no-attention").long("require-no-attention"))
}

///
/// RestoreStatusOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreStatusOptions {
    pub backup_ref: Option<String>,
    pub journal: Option<PathBuf>,
    pub icp: String,
    pub network: Option<String>,
    pub out: Option<PathBuf>,
    pub require_complete: bool,
    pub require_no_attention: bool,
}

impl RestoreStatusOptions {
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(restore_status_command(), args)
            .map_err(|_| RestoreCommandError::Usage(status_usage()))?;

        Ok(Self {
            backup_ref: string_option(&matches, BACKUP_REF),
            journal: path_option(&matches, "journal"),
            icp: string_option(&matches, "icp").unwrap_or_else(default_icp),
            network: string_option(&matches, "network"),
            out: path_option(&matches, "out"),
            require_complete: matches.get_flag("require-complete"),
            require_no_attention: matches.get_flag("require-no-attention"),
        })
    }
}

pub(super) fn restore_status_command() -> ClapCommand {
    ClapCommand::new("status")
        .bin_name("canic restore status")
        .about("Summarize restore runner journal state")
        .disable_help_flag(true)
        .group(
            ArgGroup::new("journal-source")
                .args([BACKUP_REF, "journal"])
                .required(true)
                .multiple(false),
        )
        .arg(value_arg(BACKUP_REF).value_name(BACKUP_REF))
        .arg(value_arg("journal").long("journal").value_name("file"))
        .arg(internal_icp_arg())
        .arg(internal_network_arg())
        .arg(value_arg("out").long("out").value_name("file"))
        .arg(flag_arg("require-complete").long("require-complete"))
        .arg(flag_arg("require-no-attention").long("require-no-attention"))
}

fn parse_positive_usize(value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| "expected a positive integer".to_string())?;
    if parsed == 0 {
        return Err("expected a positive integer".to_string());
    }

    Ok(parsed)
}
