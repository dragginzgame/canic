//! Module: canic_cli::medic::command
//!
//! Responsibility: parse and dispatch the read-only medic CLI command.
//! Does not own: diagnostic collection, report aggregation, or output formatting rules.
//! Boundary: turns CLI arguments into medic options and renders the completed report.

use super::{
    build_medic_report,
    render::{render_medic_ci_text, render_medic_json, render_medic_text},
    report::{MedicScope, MedicStatus},
};
use crate::{
    cli::{
        clap::{flag_arg, parse_matches, render_usage, required_string, string_option, value_arg},
        defaults::{default_icp, local_environment},
        globals::{
            INTERNAL_ENVIRONMENT_OPTION, INTERNAL_ICP_OPTION, internal_environment_arg,
            internal_icp_arg,
        },
        help::print_help_or_version,
    },
    version_text,
};
use clap::Command as ClapCommand;
use std::ffi::OsString;
use thiserror::Error as ThisError;

const FLEET_COMMAND: &str = "fleet";
const FLEET_ARG: &str = "fleet";
const JSON_ARG: &str = "json";
const CI_ARG: &str = "ci";
const BLOB_STORAGE_ARG: &str = "blob-storage";
const AUTH_RENEWAL_ARG: &str = "auth-renewal";
const MEDIC_HELP_AFTER: &str = "\
Examples:
  canic medic
  canic medic --ci
  canic medic fleet test
";

/// An error while parsing, running, or rendering the medic command.
#[derive(Debug, ThisError)]
pub enum MedicCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("failed to render medic JSON output: {0}")]
    Json(#[from] serde_json::Error),

    #[error("blocking preflight issues found")]
    ReportFailed,
}

impl MedicCommandError {
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::ReportFailed => 1,
            Self::Json(_) => 3,
        }
    }

    pub const fn suppress_stderr(&self) -> bool {
        matches!(self, Self::ReportFailed)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct MedicOptions {
    pub(super) scope: MedicScope,
    pub(super) fleet: Option<String>,
    pub(super) blob_storage: Option<String>,
    pub(super) auth_renewal: Option<String>,
    pub(super) json: bool,
    pub(super) ci: bool,
    pub(super) environment: Option<String>,
    pub(super) icp: String,
}

impl MedicOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, MedicCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(medic_command(), args).map_err(|_| MedicCommandError::Usage(usage()))?;
        let json = matches.get_flag(JSON_ARG);
        let ci = matches.get_flag(CI_ARG);
        let environment = string_option(&matches, "environment");
        let icp = string_option(&matches, "icp").unwrap_or_else(default_icp);

        match matches.subcommand() {
            Some((FLEET_COMMAND, matches)) => Ok(Self {
                scope: MedicScope::Fleet,
                fleet: Some(required_string(matches, FLEET_ARG)),
                blob_storage: string_option(matches, BLOB_STORAGE_ARG),
                auth_renewal: string_option(matches, AUTH_RENEWAL_ARG),
                json,
                ci,
                environment,
                icp,
            }),
            None => Ok(Self::workspace(json, ci, environment, icp)),
            Some(_) => Err(MedicCommandError::Usage(usage())),
        }
    }

    pub(super) const fn workspace(
        json: bool,
        ci: bool,
        environment: Option<String>,
        icp: String,
    ) -> Self {
        Self {
            scope: MedicScope::Workspace,
            fleet: None,
            blob_storage: None,
            auth_renewal: None,
            json,
            ci,
            environment,
            icp,
        }
    }

    pub(super) fn command_label(&self) -> String {
        match (&self.scope, &self.fleet) {
            (MedicScope::Workspace, _) => "canic medic".to_string(),
            (MedicScope::Fleet, Some(fleet)) => {
                format!("canic medic fleet {fleet}")
            }
            (MedicScope::Fleet, None) => "canic medic fleet".to_string(),
        }
    }

    pub(super) fn fleet_name(&self) -> &str {
        self.fleet
            .as_deref()
            .expect("Fleet scope requires Fleet name")
    }

    pub(super) fn fleet_environment(&self) -> String {
        self.environment.clone().unwrap_or_else(local_environment)
    }
}

pub fn run<I>(args: I) -> Result<(), MedicCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }
    if medic_subcommand_help_requested(&args) {
        println!("{}", usage());
        return Ok(());
    }

    let options = MedicOptions::parse(args)?;
    let report = build_medic_report(&options);
    if options.json {
        println!("{}", render_medic_json(&report)?);
    } else if options.ci {
        println!("{}", render_medic_ci_text(&report));
    } else {
        println!("{}", render_medic_text(&report));
    }
    if report.status == MedicStatus::Fail {
        return Err(MedicCommandError::ReportFailed);
    }
    Ok(())
}

pub(super) fn medic_subcommand_help_requested(args: &[OsString]) -> bool {
    let mut index = skip_medic_options(args, 0);
    let Some(FLEET_COMMAND) = args.get(index).and_then(|arg| arg.to_str()) else {
        return false;
    };
    index = skip_medic_options(args, index + 1);
    args.get(index).is_some_and(is_medic_help_arg)
}

fn skip_medic_options(args: &[OsString], mut index: usize) -> usize {
    while let Some(arg) = args.get(index).and_then(|arg| arg.to_str()) {
        match arg {
            "--json" | "--ci" => index += 1,
            INTERNAL_ICP_OPTION | INTERNAL_ENVIRONMENT_OPTION => index += 2,
            _ => break,
        }
    }
    index
}

fn is_medic_help_arg(arg: &OsString) -> bool {
    matches!(arg.to_str(), Some("--help" | "-h"))
}

fn medic_command() -> ClapCommand {
    ClapCommand::new("medic")
        .bin_name("canic medic")
        .disable_help_flag(true)
        .about("Diagnose local workspace and current-Fleet readiness")
        .arg(
            flag_arg(JSON_ARG)
                .long(JSON_ARG)
                .global(true)
                .help("Print JSON output"),
        )
        .arg(
            flag_arg(CI_ARG)
                .long(CI_ARG)
                .global(true)
                .help("Print concise fail-only text output for CI logs"),
        )
        .arg(internal_environment_arg().global(true))
        .arg(internal_icp_arg().global(true))
        .subcommand(fleet_command())
        .after_help(MEDIC_HELP_AFTER)
}

fn fleet_command() -> ClapCommand {
    ClapCommand::new(FLEET_COMMAND)
        .disable_help_flag(true)
        .about("Run Fleet-level medic checks")
        .arg(
            value_arg(FLEET_ARG)
                .value_name(FLEET_ARG)
                .required(true)
                .help("Current Fleet identity"),
        )
        .arg(
            value_arg(BLOB_STORAGE_ARG)
                .long(BLOB_STORAGE_ARG)
                .value_name("canister-or-role")
                .help("Run targeted blob-storage billing readiness diagnostics"),
        )
        .arg(
            value_arg(AUTH_RENEWAL_ARG)
                .long(AUTH_RENEWAL_ARG)
                .value_name("issuer-principal")
                .help("Run targeted chain-key auth renewal drift diagnostics"),
        )
}

pub(super) fn usage() -> String {
    render_usage(medic_command)
}
