use super::{NnsCommandError, OutputFormat, now_unix_secs, parse_format, write_text_or_json};
use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, parse_required_subcommand, passthrough_subcommand,
            path_option, render_help, required_string, string_option, typed_option, value_arg,
        },
        globals::internal_network_arg,
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::{
    duration::parse_duration_seconds,
    nns_node_operator::{
        DEFAULT_NNS_NODE_OPERATOR_SOURCE_ENDPOINT,
        DEFAULT_NODE_OPERATOR_REFRESH_LOCK_STALE_SECONDS, NnsNodeOperatorCacheRequest,
        NnsNodeOperatorInfoRequest, NnsNodeOperatorListRequest, NnsNodeOperatorRefreshRequest,
        build_nns_node_operator_info_report, build_nns_node_operator_list_report,
        nns_node_operator_info_report_text, nns_node_operator_list_report_text,
        nns_node_operator_list_report_verbose_text, nns_node_operator_refresh_report_text,
        refresh_nns_node_operator_report,
    },
    release_set::icp_root,
};
use canic_subnet_catalog::MAINNET_NETWORK;
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

const NODE_OPERATOR_LIST_HELP_AFTER: &str = "\
Examples:
  canic nns node-operator list
  canic nns node-operator list --verbose
  canic --network ic nns node-operator list --format json

Force-refresh cached native NNS data:
  canic nns node-operator refresh";
const NODE_OPERATOR_INFO_HELP_AFTER: &str = "\
Examples:
  canic nns node-operator info <node-operator>
  canic nns node-operator info <node-operator-prefix>
  canic --network ic nns node-operator info <node-operator> --format json

Force-refresh cached native NNS data:
  canic nns node-operator refresh";
const NODE_OPERATOR_REFRESH_HELP_AFTER: &str = "\
Examples:
  canic nns node-operator refresh
  canic --network ic nns node-operator refresh --format json
  canic nns node-operator refresh --dry-run --output .canic/node-operator/ic/operators.preview.json";

///
/// NodeOperatorListOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct NodeOperatorListOptions {
    pub(super) network: String,
    pub(super) format: OutputFormat,
    pub(super) source_endpoint: String,
    pub(super) verbose: bool,
}

///
/// NodeOperatorInfoOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct NodeOperatorInfoOptions {
    pub(super) input: String,
    pub(super) network: String,
    pub(super) format: OutputFormat,
    pub(super) source_endpoint: String,
}

///
/// NodeOperatorRefreshOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct NodeOperatorRefreshOptions {
    pub(super) network: String,
    pub(super) format: OutputFormat,
    pub(super) source_endpoint: String,
    pub(super) lock_stale_after_seconds: u64,
    pub(super) dry_run: bool,
    pub(super) output_path: Option<PathBuf>,
}

pub(super) fn run<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, node_operator_usage, version_text()) {
        return Ok(());
    }
    let (command, args) = parse_required_subcommand(node_operator_command(), args)
        .map_err(|_| NnsCommandError::Usage(node_operator_usage()))?;

    match command.as_str() {
        "list" => run_node_operator_list(args),
        "info" => run_node_operator_info(args),
        "refresh" => run_node_operator_refresh(args),
        _ => unreachable!("nns node-operator dispatch command only defines known commands"),
    }
}

fn run_node_operator_list<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, node_operator_list_usage, version_text()) {
        return Ok(());
    }
    let options = NodeOperatorListOptions::parse(args)?;
    let icp_root = icp_root().map_err(|err| NnsCommandError::Usage(err.to_string()))?;
    let request = NnsNodeOperatorListRequest {
        cache: cache_request(&icp_root, &options.network),
        source_endpoint: options.source_endpoint,
        now_unix_secs: now_unix_secs()?,
    };
    let report = build_nns_node_operator_list_report(&request)?;
    write_text_or_json(options.format, &report, |report| {
        if options.verbose {
            nns_node_operator_list_report_verbose_text(report)
        } else {
            nns_node_operator_list_report_text(report)
        }
    })
}

fn run_node_operator_info<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, node_operator_info_usage, version_text()) {
        return Ok(());
    }
    let options = NodeOperatorInfoOptions::parse(args)?;
    let icp_root = icp_root().map_err(|err| NnsCommandError::Usage(err.to_string()))?;
    let request = NnsNodeOperatorInfoRequest {
        cache: cache_request(&icp_root, &options.network),
        source_endpoint: options.source_endpoint,
        input: options.input,
        now_unix_secs: now_unix_secs()?,
    };
    let report = build_nns_node_operator_info_report(&request)?;
    write_text_or_json(options.format, &report, nns_node_operator_info_report_text)
}

fn run_node_operator_refresh<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, node_operator_refresh_usage, version_text()) {
        return Ok(());
    }
    let options = NodeOperatorRefreshOptions::parse(args)?;
    let format = options.format;
    let icp_root = icp_root().map_err(|err| NnsCommandError::Usage(err.to_string()))?;
    let request = NnsNodeOperatorRefreshRequest {
        cache: cache_request(&icp_root, &options.network),
        source_endpoint: options.source_endpoint,
        now_unix_secs: now_unix_secs()?,
        lock_stale_after_seconds: options.lock_stale_after_seconds,
        dry_run: options.dry_run,
        output_path: options.output_path,
    };
    let report = refresh_nns_node_operator_report(&request)?;
    write_text_or_json(format, &report, nns_node_operator_refresh_report_text)
}

impl NodeOperatorListOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(node_operator_list_command(), args)
            .map_err(|_| NnsCommandError::Usage(node_operator_list_usage()))?;
        Ok(Self {
            network: string_option(&matches, "network")
                .unwrap_or_else(|| MAINNET_NETWORK.to_string()),
            format: typed_option(&matches, "format").unwrap_or(OutputFormat::Text),
            source_endpoint: string_option(&matches, "source-endpoint")
                .unwrap_or_else(|| DEFAULT_NNS_NODE_OPERATOR_SOURCE_ENDPOINT.to_string()),
            verbose: matches.get_flag("verbose"),
        })
    }
}

impl NodeOperatorInfoOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(node_operator_info_command(), args)
            .map_err(|_| NnsCommandError::Usage(node_operator_info_usage()))?;
        Ok(Self {
            input: required_string(&matches, "input"),
            network: string_option(&matches, "network")
                .unwrap_or_else(|| MAINNET_NETWORK.to_string()),
            format: typed_option(&matches, "format").unwrap_or(OutputFormat::Text),
            source_endpoint: string_option(&matches, "source-endpoint")
                .unwrap_or_else(|| DEFAULT_NNS_NODE_OPERATOR_SOURCE_ENDPOINT.to_string()),
        })
    }
}

impl NodeOperatorRefreshOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(node_operator_refresh_command(), args)
            .map_err(|_| NnsCommandError::Usage(node_operator_refresh_usage()))?;
        Ok(Self {
            network: string_option(&matches, "network")
                .unwrap_or_else(|| MAINNET_NETWORK.to_string()),
            format: typed_option(&matches, "format").unwrap_or(OutputFormat::Text),
            source_endpoint: string_option(&matches, "source-endpoint")
                .unwrap_or_else(|| DEFAULT_NNS_NODE_OPERATOR_SOURCE_ENDPOINT.to_string()),
            lock_stale_after_seconds: typed_option(&matches, "lock-stale-after")
                .unwrap_or(DEFAULT_NODE_OPERATOR_REFRESH_LOCK_STALE_SECONDS),
            dry_run: matches.get_flag("dry-run"),
            output_path: path_option(&matches, "output"),
        })
    }
}

fn parse_refresh_lock_stale_after(value: &str) -> Result<u64, String> {
    parse_duration_seconds(value).map_err(|err| err.to_string())
}

fn cache_request(icp_root: &std::path::Path, network: &str) -> NnsNodeOperatorCacheRequest {
    NnsNodeOperatorCacheRequest {
        icp_root: PathBuf::from(icp_root),
        network: network.to_string(),
    }
}

pub(super) fn node_operator_command() -> ClapCommand {
    ClapCommand::new("node-operator")
        .bin_name("canic nns node-operator")
        .about("Inspect NNS node-operator metadata")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list").about("List cached mainnet NNS node operators"),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("info").about("Show one cached mainnet NNS node operator"),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("refresh").about("Force-refresh and cache NNS node-operator metadata"),
        ))
}

fn node_operator_list_command() -> ClapCommand {
    ClapCommand::new("list")
        .bin_name("canic nns node-operator list")
        .about("List cached mainnet NNS node operators")
        .disable_help_flag(true)
        .arg(
            value_arg("format")
                .long("format")
                .value_name("text|json")
                .value_parser(clap::builder::ValueParser::new(parse_format))
                .help("Output format; defaults to text"),
        )
        .arg(
            value_arg("source-endpoint")
                .long("source-endpoint")
                .value_name("url")
                .help("IC API endpoint used if the node-operator cache is missing"),
        )
        .arg(
            flag_arg("verbose")
                .long("verbose")
                .help("Show full node-operator principals and registry metadata in text output"),
        )
        .arg(internal_network_arg())
        .after_help(NODE_OPERATOR_LIST_HELP_AFTER)
}

fn node_operator_info_command() -> ClapCommand {
    ClapCommand::new("info")
        .bin_name("canic nns node-operator info")
        .about("Show one cached mainnet NNS node operator")
        .disable_help_flag(true)
        .arg(
            value_arg("input")
                .value_name("node-operator|node-operator-prefix")
                .required(true)
                .help("Node-operator principal or unique node-operator principal prefix"),
        )
        .arg(
            value_arg("format")
                .long("format")
                .value_name("text|json")
                .value_parser(clap::builder::ValueParser::new(parse_format))
                .help("Output format; defaults to text"),
        )
        .arg(
            value_arg("source-endpoint")
                .long("source-endpoint")
                .value_name("url")
                .help("IC API endpoint used if the node-operator cache is missing"),
        )
        .arg(internal_network_arg())
        .after_help(NODE_OPERATOR_INFO_HELP_AFTER)
}

fn node_operator_refresh_command() -> ClapCommand {
    ClapCommand::new("refresh")
        .bin_name("canic nns node-operator refresh")
        .about("Force-refresh and cache NNS node-operator metadata")
        .disable_help_flag(true)
        .arg(
            value_arg("format")
                .long("format")
                .value_name("text|json")
                .value_parser(clap::builder::ValueParser::new(parse_format))
                .help("Output format; defaults to text"),
        )
        .arg(
            value_arg("source-endpoint")
                .long("source-endpoint")
                .value_name("url")
                .help("IC API endpoint used for native NNS registry queries"),
        )
        .arg(
            value_arg("lock-stale-after")
                .long("lock-stale-after")
                .value_name("duration")
                .value_parser(clap::builder::ValueParser::new(
                    parse_refresh_lock_stale_after,
                ))
                .help(
                    "Treat an existing refresh lock as stale after this duration; defaults to 30m",
                ),
        )
        .arg(
            flag_arg("dry-run")
                .long("dry-run")
                .help("Fetch and validate without replacing the cached node-operator report"),
        )
        .arg(
            value_arg("output")
                .long("output")
                .value_name("path")
                .help("Also write the fetched node-operator JSON to this path"),
        )
        .arg(internal_network_arg())
        .after_help(NODE_OPERATOR_REFRESH_HELP_AFTER)
}

pub(super) fn node_operator_usage() -> String {
    render_help(node_operator_command())
}

pub(super) fn node_operator_list_usage() -> String {
    render_help(node_operator_list_command())
}

pub(super) fn node_operator_info_usage() -> String {
    render_help(node_operator_info_command())
}

pub(super) fn node_operator_refresh_usage() -> String {
    render_help(node_operator_refresh_command())
}
