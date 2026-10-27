mod model;
mod render;
mod transport;

use crate::{
    cli::{
        clap::{flag_arg, parse_matches, render_usage, required_string, string_option, value_arg},
        globals::{internal_environment_arg, internal_icp_arg},
        help::print_help_or_version,
    },
    endpoints::{render::render_plain_endpoints, transport::endpoint_report},
    version_text,
};
use canic_contracts::ids::ReleaseBuildId;
use canic_host::icp_config::IcpConfigError;
use clap::Command as ClapCommand;
use std::ffi::OsString;
use thiserror::Error as ThisError;

const INFO_HELP_AFTER: &str = "\
Examples:
  canic info endpoints demo-local app
  canic info endpoints demo-local scale_hub --release-build <sha256> --json
  canic info endpoints demo-local tl4x7-vh777-77776-aaacq-cai";

///
/// EndpointsCommandError
///

#[derive(Debug, ThisError)]
pub enum EndpointsCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    BuiltSchema(#[from] canic_host::release_set::BuiltDeclarationError),

    #[error("invalid retained Wire endpoint schema")]
    Schema,

    #[error(transparent)]
    Installed(#[from] canic_host::protocol_binding::InstalledDeclarationError),

    #[error(transparent)]
    IcpConfig(#[from] IcpConfigError),

    #[error("failed to render endpoint output: {0}")]
    Json(#[from] serde_json::Error),
}

///
/// EndpointsOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct EndpointsOptions {
    fleet: String,
    canister: String,
    environment: Option<String>,
    json: bool,
    release_build: Option<ReleaseBuildId>,
}

impl EndpointsOptions {
    fn parse_info<I>(args: I) -> Result<Self, EndpointsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse_with(args, info_command, info_usage, "fleet")
    }

    fn parse_with<I>(
        args: I,
        command: impl FnOnce() -> ClapCommand,
        usage: fn() -> String,
        target_arg: &str,
    ) -> Result<Self, EndpointsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| EndpointsCommandError::Usage(usage()))?;
        Ok(Self {
            fleet: required_string(&matches, target_arg),
            canister: required_string(&matches, "canister"),
            environment: string_option(&matches, "environment"),
            json: matches.get_flag("json"),
            release_build: matches.get_one::<ReleaseBuildId>("release-build").copied(),
        })
    }
}

/// Run the current-Fleet endpoint listing command.
pub fn run_info<I>(args: I) -> Result<(), EndpointsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, info_usage, version_text()) {
        return Ok(());
    }

    let options = EndpointsOptions::parse_info(args)?;
    run_options(&options)
}

fn run_options(options: &EndpointsOptions) -> Result<(), EndpointsCommandError> {
    let report = endpoint_report(options)?;
    if options.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "Source: {}\n{}",
            report.source,
            render_plain_endpoints(&report.endpoints)
        );
    }
    Ok(())
}

fn info_command() -> ClapCommand {
    endpoint_command(
        "canic info endpoints",
        "fleet",
        "Deployed Fleet name; unused with --release-build",
        INFO_HELP_AFTER,
    )
}

fn endpoint_command(
    bin_name: &'static str,
    target_arg: &'static str,
    target_help: &'static str,
    help_after: &'static str,
) -> ClapCommand {
    ClapCommand::new("endpoints")
        .bin_name(bin_name)
        .disable_help_flag(true)
        .about("List methods and codec budgets from the retained Wire schema")
        .arg(
            value_arg(target_arg)
                .value_name(target_arg)
                .required(true)
                .help(target_help),
        )
        .arg(
            value_arg("canister")
                .value_name("canister-or-role")
                .required(true)
                .help("Canister principal or role name to inspect"),
        )
        .arg(
            value_arg("release-build")
                .long("release-build")
                .value_name("sha256")
                .value_parser(clap::value_parser!(ReleaseBuildId))
                .help(
                    "Inspect a finalized build role offline, using the ID emitted by canic build",
                ),
        )
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .arg(flag_arg("json").long("json").help("Print JSON output"))
        .after_help(help_after)
}

fn info_usage() -> String {
    render_usage(info_command)
}

#[cfg(test)]
mod tests;
