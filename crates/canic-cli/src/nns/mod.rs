mod data_center;
mod node;
mod node_operator;
mod node_provider;
mod registry;
mod subnet;

#[cfg(test)]
mod tests;

use crate::{
    cli::{
        clap::{parse_required_subcommand, passthrough_subcommand, render_help},
        help::print_help_or_version,
    },
    output::{write_pretty_json, write_text},
    version_text,
};
use canic_host::{
    nns_data_center::NnsDataCenterHostError, nns_node::NnsNodeHostError,
    nns_node_operator::NnsNodeOperatorHostError, nns_node_provider::NnsNodeProviderHostError,
    nns_registry::NnsRegistryHostError, subnet_catalog::SubnetCatalogHostError,
};
use clap::Command as ClapCommand;
use serde::Serialize;
use std::{
    ffi::OsString,
    io,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

///
/// NnsCommandError
///
#[derive(Debug, ThisError)]
pub enum NnsCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    SubnetHost(#[from] SubnetCatalogHostError),

    #[error(transparent)]
    DataCenterHost(#[from] NnsDataCenterHostError),

    #[error(transparent)]
    NodeHost(#[from] NnsNodeHostError),

    #[error(transparent)]
    NodeProviderHost(#[from] NnsNodeProviderHostError),

    #[error(transparent)]
    NodeOperatorHost(#[from] NnsNodeOperatorHostError),

    #[error(transparent)]
    RegistryHost(#[from] NnsRegistryHostError),

    #[error(
        "deployment target {input} did not resolve to exactly one canister principal for network {network}: {reason}"
    )]
    TargetResolutionFailed {
        input: String,
        network: String,
        reason: String,
    },

    #[error("system clock before unix epoch: {0}")]
    Clock(String),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

///
/// OutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    Text,
    Json,
}

pub fn run<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }
    let (command, args) = parse_required_subcommand(nns_command(), args)
        .map_err(|_| NnsCommandError::Usage(usage()))?;

    match command.as_str() {
        "subnet" => subnet::run(args),
        "data-center" => data_center::run(args),
        "node" => node::run(args),
        "node-provider" => node_provider::run(args),
        "node-operator" => node_operator::run(args),
        "registry" => registry::run(args),
        _ => unreachable!("nns dispatch command only defines known commands"),
    }
}

fn write_text_or_json<T>(
    format: OutputFormat,
    report: &T,
    render_text: impl FnOnce(&T) -> String,
) -> Result<(), NnsCommandError>
where
    T: Serialize,
{
    match format {
        OutputFormat::Text => {
            let text = render_text(report);
            write_text::<NnsCommandError>(None, &text)
        }
        OutputFormat::Json => write_pretty_json(None, report),
    }
}

fn parse_format(value: &str) -> Result<OutputFormat, String> {
    match value {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        other => Err(format!("invalid value {other}; use text or json")),
    }
}

fn now_unix_secs() -> Result<u64, NnsCommandError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|err| NnsCommandError::Clock(err.to_string()))
}

fn nns_command() -> ClapCommand {
    ClapCommand::new("nns")
        .bin_name("canic nns")
        .about("Inspect NNS metadata")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("subnet").about("Inspect and refresh NNS subnet metadata"),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("data-center").about("Inspect NNS data-center metadata"),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("node").about("Inspect NNS node metadata"),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("node-provider").about("Inspect NNS node-provider metadata"),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("node-operator").about("Inspect NNS node-operator metadata"),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("registry").about("Inspect NNS registry metadata"),
        ))
}

fn usage() -> String {
    render_help(nns_command())
}
