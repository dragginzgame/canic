use crate::{
    cli::clap::{
        flag_arg, parse_matches, parse_positive_u64, path_option, render_usage, required_string,
        required_typed, string_option, string_option_or_else, value_arg,
    },
    cli::defaults::{default_icp, local_network},
    cli::globals::{internal_icp_arg, internal_network_arg},
    metrics::{MetricsCommandError, model::MetricsKind},
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

const DEFAULT_KIND: &str = "core";
const DEFAULT_LIMIT: &str = "1000";

///
/// MetricsOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct MetricsOptions {
    pub(super) deployment: String,
    pub(super) kind: MetricsKind,
    pub(super) role: Option<String>,
    pub(super) canister: Option<String>,
    pub(super) nonzero: bool,
    pub(super) limit: u64,
    pub(super) json: bool,
    pub(super) out: Option<PathBuf>,
    pub(super) network: String,
    pub(super) icp: String,
}

impl MetricsOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, MetricsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(metrics_command(), args)
            .map_err(|_| MetricsCommandError::Usage(usage()))?;
        Ok(Self {
            deployment: required_string(&matches, "deployment"),
            kind: required_typed(&matches, "kind"),
            role: string_option(&matches, "role"),
            canister: string_option(&matches, "canister"),
            nonzero: matches.get_flag("nonzero"),
            limit: required_typed(&matches, "limit"),
            json: matches.get_flag("json"),
            out: path_option(&matches, "out"),
            network: string_option_or_else(&matches, "network", local_network),
            icp: string_option_or_else(&matches, "icp", default_icp),
        })
    }
}

pub(super) fn usage() -> String {
    render_usage(metrics_command)
}

fn metrics_command() -> ClapCommand {
    ClapCommand::new("metrics")
        .bin_name("canic metrics")
        .about("Query Canic runtime telemetry")
        .disable_help_flag(true)
        .arg(
            value_arg("deployment")
                .value_name("deployment")
                .required(true)
                .help("Installed deployment target name to inspect"),
        )
        .arg(
            value_arg("kind")
                .long("kind")
                .value_name("kind")
                .default_value(DEFAULT_KIND)
                .value_parser(clap::value_parser!(MetricsKind))
                .help("Metrics tier to query; defaults to core"),
        )
        .arg(
            value_arg("role")
                .long("role")
                .value_name("role")
                .help("Only query one registry role"),
        )
        .arg(
            value_arg("canister")
                .long("canister")
                .value_name("id")
                .help("Only query one canister principal"),
        )
        .arg(
            value_arg("limit")
                .long("limit")
                .value_name("entries")
                .default_value(DEFAULT_LIMIT)
                .value_parser(clap::builder::ValueParser::new(parse_positive_u64))
                .help("Maximum metric rows to fetch per canister; defaults to 1000"),
        )
        .arg(flag_arg("nonzero").long("nonzero"))
        .arg(flag_arg("json").long("json"))
        .arg(value_arg("out").long("out").value_name("file"))
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}
