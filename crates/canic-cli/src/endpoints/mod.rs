mod model;
mod parse;
mod render;
mod transport;

#[cfg(test)]
use crate::endpoints::{
    model::{EndpointCardinality, EndpointEntry, EndpointMode, EndpointType},
    parse::parse_candid_service_endpoints,
};
use crate::{
    cli::clap::{flag_arg, parse_matches, value_arg},
    cli::defaults::default_icp,
    cli::globals::{internal_icp_arg, internal_network_arg},
    cli::help::print_help_or_version,
    endpoints::{render::render_plain_endpoints, transport::endpoint_report},
    version_text,
};
use clap::Command as ClapCommand;
use std::ffi::OsString;
use thiserror::Error as ThisError;

const CANDID_SERVICE_METADATA: &str = "candid:service";
const HELP_AFTER: &str = "\
Examples:
  canic endpoints test app
  canic endpoints test scale_hub --json
  canic endpoints test tl4x7-vh777-77776-aaacq-cai";

///
/// EndpointsCommandError
///

#[derive(Debug, ThisError)]
pub enum EndpointsCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("canister interface did not contain a service block")]
    MissingService,

    #[error("failed to parse Candid interface: {0}")]
    InvalidCandid(String),

    #[error(
        "live metadata was unavailable for {canister} in fleet {fleet} and no local Candid artifact could be resolved"
    )]
    NoInterfaceArtifact { fleet: String, canister: String },

    #[error("local Candid artifact not found for role {role}; looked under {root}")]
    MissingRoleArtifact { role: String, root: String },

    #[error("failed to read local Candid artifact {path}: {source}")]
    ReadDid {
        path: String,
        source: std::io::Error,
    },

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
    network: Option<String>,
    icp: String,
    json: bool,
}

impl EndpointsOptions {
    fn parse<I>(args: I) -> Result<Self, EndpointsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| EndpointsCommandError::Usage(usage()))?;
        Ok(Self {
            fleet: string_value(&matches, "fleet").expect("clap requires fleet"),
            canister: string_value(&matches, "canister").expect("clap requires canister"),
            network: string_value(&matches, "network"),
            icp: string_value(&matches, "icp").unwrap_or_else(default_icp),
            json: matches.get_flag("json"),
        })
    }
}

/// Run the canister endpoint listing command.
pub fn run<I>(args: I) -> Result<(), EndpointsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = EndpointsOptions::parse(args)?;
    let report = endpoint_report(&options)?;
    if options.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", render_plain_endpoints(&report.endpoints));
    }
    Ok(())
}

fn string_value(matches: &clap::ArgMatches, id: &str) -> Option<String> {
    matches.try_get_one::<String>(id).ok().flatten().cloned()
}

fn command() -> ClapCommand {
    ClapCommand::new("endpoints")
        .bin_name("canic endpoints")
        .disable_help_flag(true)
        .about("List callable methods exposed by a canister Candid interface")
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Fleet name to inspect"),
        )
        .arg(
            value_arg("canister")
                .value_name("canister-or-role")
                .required(true)
                .help("Canister principal or role name to inspect"),
        )
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
        .arg(flag_arg("json").long("json").help("Print JSON output"))
        .after_help(HELP_AFTER)
}

fn usage() -> String {
    let mut command = command();
    command.render_help().to_string()
}

#[cfg(test)]
mod tests;
