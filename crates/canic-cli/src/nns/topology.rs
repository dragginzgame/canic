use super::{
    NnsCommandError,
    leaf::{self, NnsCommonOptions},
    now_unix_secs, write_text_or_json,
};
use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, parse_required_subcommand, passthrough_subcommand,
            render_help, required_typed,
        },
        help::{first_arg_is_help, print_help_or_version},
    },
    version_text,
};
use canic_host::{
    nns_topology::{
        NnsTopologyRefreshRequest, NnsTopologySummaryRequest, build_nns_topology_summary_report,
        nns_topology_refresh_report_text, nns_topology_summary_report_text,
        refresh_nns_topology_report,
    },
    release_set::icp_root,
};
use std::ffi::OsString;

const TOPOLOGY_SUMMARY_HELP_AFTER: &str = "\
Examples:
  canic nns topology summary
  canic --network ic nns topology summary --format json
  canic nns topology summary --source-endpoint https://icp-api.io";
const TOPOLOGY_REFRESH_HELP_AFTER: &str = "\
Examples:
  canic nns topology refresh
  canic nns topology refresh --dry-run
  canic --network ic nns topology refresh --format json
  canic nns topology refresh --source-endpoint https://icp-api.io";
const DRY_RUN_ARG: &str = "dry-run";
const LOCK_STALE_AFTER_ARG: &str = "lock-stale-after";

///
/// TopologySummaryOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TopologySummaryOptions {
    pub(super) network: String,
    pub(super) format: super::OutputFormat,
    pub(super) source_endpoint: String,
}

///
/// TopologyRefreshOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TopologyRefreshOptions {
    pub(super) network: String,
    pub(super) format: super::OutputFormat,
    pub(super) source_endpoint: String,
    pub(super) lock_stale_after_seconds: u64,
    pub(super) dry_run: bool,
}

pub(super) fn run<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_topology_help_or_version(&args) {
        return Ok(());
    }
    let (command, args) = parse_required_subcommand(topology_command(), args)
        .map_err(|_| NnsCommandError::Usage(topology_usage()))?;

    match command.as_str() {
        "summary" => run_topology_summary(args),
        "refresh" => run_topology_refresh(args),
        _ => unreachable!("nns topology dispatch command only defines known commands"),
    }
}

fn print_topology_help_or_version(args: &[OsString]) -> bool {
    if first_arg_is_help(args) {
        println!("{}", topology_usage());
        return true;
    }
    if args.first().is_some_and(is_version_flag) {
        println!("{}", version_text());
        return true;
    }
    false
}

fn is_version_flag(arg: &OsString) -> bool {
    arg.to_str()
        .is_some_and(|arg| matches!(arg, "--version" | "-V"))
}

fn run_topology_summary<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, topology_summary_usage, version_text()) {
        return Ok(());
    }
    let options = TopologySummaryOptions::parse(args)?;
    let format = options.format;
    let icp_root = icp_root().map_err(|err| NnsCommandError::Usage(err.to_string()))?;
    let request = NnsTopologySummaryRequest {
        icp_root,
        network: options.network,
        source_endpoint: options.source_endpoint,
        now_unix_secs: now_unix_secs()?,
    };
    let report = build_nns_topology_summary_report(&request)?;
    write_text_or_json(format, &report, nns_topology_summary_report_text)
}

fn run_topology_refresh<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, topology_refresh_usage, version_text()) {
        return Ok(());
    }
    let options = TopologyRefreshOptions::parse(args)?;
    let format = options.format;
    let icp_root = icp_root().map_err(|err| NnsCommandError::Usage(err.to_string()))?;
    let request = NnsTopologyRefreshRequest {
        icp_root,
        network: options.network,
        source_endpoint: options.source_endpoint,
        now_unix_secs: now_unix_secs()?,
        lock_stale_after_seconds: options.lock_stale_after_seconds,
        dry_run: options.dry_run,
    };
    let report = refresh_nns_topology_report(&request)?;
    write_text_or_json(format, &report, nns_topology_refresh_report_text)
}

impl TopologySummaryOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(topology_summary_command(), args)
            .map_err(|_| NnsCommandError::Usage(topology_summary_usage()))?;
        let common = NnsCommonOptions::from_matches(&matches);
        Ok(Self {
            network: common.network,
            format: common.format,
            source_endpoint: common.source_endpoint,
        })
    }
}

impl TopologyRefreshOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(topology_refresh_command(), args)
            .map_err(|_| NnsCommandError::Usage(topology_refresh_usage()))?;
        let common = NnsCommonOptions::from_matches(&matches);
        Ok(Self {
            network: common.network,
            format: common.format,
            source_endpoint: common.source_endpoint,
            lock_stale_after_seconds: required_typed(&matches, LOCK_STALE_AFTER_ARG),
            dry_run: matches.get_flag(DRY_RUN_ARG),
        })
    }
}

pub(super) fn topology_command() -> clap::Command {
    clap::Command::new("topology")
        .bin_name("canic nns topology")
        .about("Inspect joined NNS topology metadata")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            clap::Command::new("summary").about("Summarize cached mainnet NNS topology reports"),
        ))
        .subcommand(passthrough_subcommand(
            clap::Command::new("refresh")
                .about("Refresh cached mainnet NNS topology component reports"),
        ))
}

fn topology_summary_command() -> clap::Command {
    clap::Command::new("summary")
        .bin_name("canic nns topology summary")
        .about("Summarize cached mainnet NNS topology reports")
        .disable_help_flag(true)
        .arg(leaf::format_arg())
        .arg(
            leaf::source_endpoint_arg(canic_host::nns_node::DEFAULT_NNS_NODE_SOURCE_ENDPOINT)
                .help("IC API endpoint used if a topology component cache is missing"),
        )
        .arg(leaf::network_arg())
        .after_help(TOPOLOGY_SUMMARY_HELP_AFTER)
}

fn topology_refresh_command() -> clap::Command {
    clap::Command::new("refresh")
        .bin_name("canic nns topology refresh")
        .about("Refresh cached mainnet NNS topology component reports")
        .disable_help_flag(true)
        .arg(leaf::format_arg())
        .arg(
            leaf::source_endpoint_arg(canic_host::nns_node::DEFAULT_NNS_NODE_SOURCE_ENDPOINT)
                .help("IC API endpoint used for NNS topology component refreshes"),
        )
        .arg(leaf::refresh_lock_stale_after_arg())
        .arg(
            flag_arg(DRY_RUN_ARG)
                .long(DRY_RUN_ARG)
                .help("Fetch and validate without replacing topology component caches"),
        )
        .arg(leaf::network_arg())
        .after_help(TOPOLOGY_REFRESH_HELP_AFTER)
}

pub(super) fn topology_usage() -> String {
    render_help(topology_command())
}

pub(super) fn topology_summary_usage() -> String {
    render_help(topology_summary_command())
}

pub(super) fn topology_refresh_usage() -> String {
    render_help(topology_refresh_command())
}
