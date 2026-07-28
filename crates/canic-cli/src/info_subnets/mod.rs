//! Module: canic_cli::info_subnets
//!
//! Responsibility: expose the live Fleet Subnet inventory command.
//! Does not own: Fleet Registry state, root counters, or terminal Fleet discovery.
//! Boundary: parses operator input, delegates live evidence collection, and renders only a
//! complete validated report.

mod model;
mod render;
#[cfg(test)]
mod tests;
mod transport;

use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, render_usage, required_string, string_option_or_else,
            value_arg,
        },
        defaults::{default_icp, local_environment},
        globals::{internal_environment_arg, internal_icp_arg},
        help::print_help_or_version,
    },
    info_subnets::model::SubnetInventoryError,
    output, version_text,
};
use std::{ffi::OsString, io};

use canic_host::{
    icp::{IcpCommandError, IcpJsonResponseError},
    icp_config::IcpConfigError,
    installed_fleet::InstalledFleetError,
};
use clap::Command as ClapCommand;
use thiserror::Error as ThisError;

const HELP_AFTER: &str = "\
Examples:
  canic info subnets demo-local
  canic --environment staging info subnets toko --json

The command prints nothing unless the Coordinator Registry and every current
non-removed Fleet Subnet Root summary form one complete, agreeing snapshot.";

///
/// InfoSubnetsCommandError
///
/// CLI boundary failure for live Fleet Subnet inventory collection.
///

#[derive(Debug, ThisError)]
pub enum InfoSubnetsCommandError {
    #[error("failed to resolve ICP project root: {0}")]
    IcpRoot(#[source] IcpConfigError),

    #[error(transparent)]
    InstalledFleet(#[from] InstalledFleetError),

    #[error(transparent)]
    Inventory(#[from] SubnetInventoryError),

    #[error("failed to query {method} on Canister {canister}: {source}")]
    Query {
        canister: String,
        method: &'static str,
        #[source]
        source: IcpCommandError,
    },

    #[error("invalid {method} response from Canister {canister}: {source}")]
    Response {
        canister: String,
        method: &'static str,
        #[source]
        source: IcpJsonResponseError,
    },

    #[error("Subnet summary query worker panicked for Fleet Subnet Root {root}")]
    SummaryWorkerPanicked { root: String },

    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

///
/// InfoSubnetsOptions
///
/// Parsed operator selection for one live Fleet Subnet inventory.
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct InfoSubnetsOptions {
    fleet: String,
    json: bool,
    environment: String,
    icp: String,
}

impl InfoSubnetsOptions {
    fn parse<I>(args: I) -> Result<Self, InfoSubnetsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| InfoSubnetsCommandError::Usage(usage()))?;
        Ok(Self {
            fleet: required_string(&matches, "fleet"),
            json: matches.get_flag("json"),
            environment: string_option_or_else(&matches, "environment", local_environment),
            icp: string_option_or_else(&matches, "icp", default_icp),
        })
    }
}

/// Query and render one complete live Fleet Subnet inventory.
pub fn run<I>(args: I) -> Result<(), InfoSubnetsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = InfoSubnetsOptions::parse(args)?;
    let report = transport::load_report(&options)?;
    if options.json {
        output::write_pretty_json(None, &report)
    } else {
        output::write_text(None, &render::text_report(&report))
    }
}

fn command() -> ClapCommand {
    ClapCommand::new("subnets")
        .bin_name("canic info subnets")
        .about("Show exact live Fleet-owned Canister counts by physical Subnet")
        .arg(
            value_arg("fleet")
                .required(true)
                .help("Installed Fleet name to inspect"),
        )
        .arg(
            flag_arg("json")
                .long("json")
                .help("Render the schema-versioned report as JSON"),
        )
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .after_help(HELP_AFTER)
}

#[must_use]
fn usage() -> String {
    render_usage(command)
}
