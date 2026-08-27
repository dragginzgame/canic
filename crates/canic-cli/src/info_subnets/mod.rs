//! Module: canic_cli::info_subnets
//!
//! Responsibility: expose the live current-Fleet Subnet inventory command.
//! Does not own: Fleet Registry state, Root counters, or terminal Fleet discovery.
//! Boundary: resolves one terminal ensure authority, collects exact live evidence, and renders only
//! a complete validated report.

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
    CanisterProtocolError, fleet_ensure::CurrentFleetInventoryError, icp_config::IcpConfigError,
    protocol_binding::ProtocolBindingError,
};
use clap::Command as ClapCommand;
use thiserror::Error as ThisError;

const HELP_AFTER: &str = "\
Examples:
  canic info subnets demo-local
  canic --environment staging info subnets demo --json

The command prints nothing unless the terminal current ensure inventory,
Coordinator Registry and every current non-removed Root summary agree.";

/// CLI boundary failure for live current-Fleet Subnet inventory collection.
#[derive(Debug, ThisError)]
pub enum InfoSubnetsCommandError {
    #[error("failed to resolve ICP project root: {0}")]
    IcpRoot(#[source] IcpConfigError),

    #[error(transparent)]
    CurrentFleet(#[from] CurrentFleetInventoryError),

    #[error(transparent)]
    Inventory(#[from] SubnetInventoryError),

    #[error(transparent)]
    Protocol(#[from] CanisterProtocolError),

    #[error(transparent)]
    ProtocolBinding(#[from] ProtocolBindingError),

    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Parsed operator selection for one live current-Fleet Subnet inventory.
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

/// Query and render one complete live current-Fleet Subnet inventory.
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
                .help("Current Fleet name to inspect"),
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
