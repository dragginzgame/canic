//! Module: backup::options::create
//!
//! Responsibility: parse `canic backup create` options.
//! Does not own: backup planning or execution.
//! Boundary: create subcommand argument shape and defaults.

use super::shared::parse_backup_options;
use crate::{
    backup::{BackupCommandError, create_usage},
    cli::{
        clap::{
            flag_arg, path_option, required_string, string_option, string_option_or_else, value_arg,
        },
        defaults::{default_icp, local_environment},
        globals::{internal_environment_arg, internal_icp_arg},
    },
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

///
/// BackupCreateOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::backup) struct BackupCreateOptions {
    pub(in crate::backup) deployment: String,
    pub(in crate::backup) subtree: Option<String>,
    pub(in crate::backup) out: Option<PathBuf>,
    pub(in crate::backup) dry_run: bool,
    pub(in crate::backup) environment: String,
    pub(in crate::backup) icp: String,
}

impl BackupCreateOptions {
    pub(in crate::backup) fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_create_command(), create_usage, args)?;

        Ok(Self {
            deployment: required_string(&matches, "deployment"),
            subtree: string_option(&matches, "subtree"),
            out: path_option(&matches, "out"),
            dry_run: matches.get_flag("dry-run"),
            environment: string_option_or_else(&matches, "environment", local_environment),
            icp: string_option_or_else(&matches, "icp", default_icp),
        })
    }
}

pub(in crate::backup) fn backup_create_command() -> ClapCommand {
    ClapCommand::new("create")
        .bin_name("canic backup create")
        .about("Create a topology-aware deployment backup")
        .disable_help_flag(true)
        .arg(
            value_arg("deployment")
                .value_name("deployment")
                .required(true)
                .help("Installed deployment target name to back up"),
        )
        .arg(
            value_arg("subtree")
                .long("subtree")
                .value_name("role-or-principal")
                .help("Plan only one connected subtree"),
        )
        .arg(
            value_arg("out").long("out").value_name("dir").help(
                "Backup output directory; defaults to backups/deployment-<name>-YYYYMMDD-HHMMSS",
            ),
        )
        .arg(
            flag_arg("dry-run")
                .long("dry-run")
                .help("Write the backup plan and execution journal without running it"),
        )
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
}
