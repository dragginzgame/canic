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
    nns_node_provider::{
        DEFAULT_NNS_SOURCE_ENDPOINT, DEFAULT_NODE_PROVIDER_REFRESH_LOCK_STALE_SECONDS,
        NnsNodeProviderCacheRequest, NnsNodeProviderInfoRequest, NnsNodeProviderListRequest,
        NnsNodeProviderRefreshRequest, build_nns_node_provider_info_report,
        build_nns_node_provider_list_report, nns_node_provider_info_report_text,
        nns_node_provider_list_report_text, nns_node_provider_list_report_verbose_text,
        nns_node_provider_refresh_report_text, refresh_nns_node_provider_report,
    },
    release_set::icp_root,
};
use canic_subnet_catalog::MAINNET_NETWORK;
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

const NODE_PROVIDER_LIST_HELP_AFTER: &str = "\
Examples:
  canic nns node-provider list
  canic nns node-provider list --verbose
  canic --network ic nns node-provider list --format json

Force-refresh cached native NNS data:
  canic nns node-provider refresh";
const NODE_PROVIDER_INFO_HELP_AFTER: &str = "\
Examples:
  canic nns node-provider info <node-provider>
  canic nns node-provider info <node-provider-prefix>
  canic --network ic nns node-provider info <node-provider> --format json

Force-refresh cached native NNS data:
  canic nns node-provider refresh";
const NODE_PROVIDER_REFRESH_HELP_AFTER: &str = "\
Examples:
  canic nns node-provider refresh
  canic --network ic nns node-provider refresh --format json
  canic nns node-provider refresh --dry-run --output .canic/node-provider/ic/providers.preview.json";

///
/// NodeProviderListOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct NodeProviderListOptions {
    pub(super) network: String,
    pub(super) format: OutputFormat,
    pub(super) source_endpoint: String,
    pub(super) verbose: bool,
}

///
/// NodeProviderInfoOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct NodeProviderInfoOptions {
    pub(super) input: String,
    pub(super) network: String,
    pub(super) format: OutputFormat,
    pub(super) source_endpoint: String,
}

///
/// NodeProviderRefreshOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct NodeProviderRefreshOptions {
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
    if print_help_or_version(&args, node_provider_usage, version_text()) {
        return Ok(());
    }
    let (command, args) = parse_required_subcommand(node_provider_command(), args)
        .map_err(|_| NnsCommandError::Usage(node_provider_usage()))?;

    match command.as_str() {
        "list" => run_node_provider_list(args),
        "info" => run_node_provider_info(args),
        "refresh" => run_node_provider_refresh(args),
        _ => unreachable!("nns node-provider dispatch command only defines known commands"),
    }
}

fn run_node_provider_list<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, node_provider_list_usage, version_text()) {
        return Ok(());
    }
    let options = NodeProviderListOptions::parse(args)?;
    let icp_root = icp_root().map_err(|err| NnsCommandError::Usage(err.to_string()))?;
    let request = NnsNodeProviderListRequest {
        cache: cache_request(&icp_root, &options.network),
        source_endpoint: options.source_endpoint,
        now_unix_secs: now_unix_secs()?,
    };
    let report = build_nns_node_provider_list_report(&request)?;
    write_text_or_json(options.format, &report, |report| {
        if options.verbose {
            nns_node_provider_list_report_verbose_text(report)
        } else {
            nns_node_provider_list_report_text(report)
        }
    })
}

fn run_node_provider_info<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, node_provider_info_usage, version_text()) {
        return Ok(());
    }
    let options = NodeProviderInfoOptions::parse(args)?;
    let icp_root = icp_root().map_err(|err| NnsCommandError::Usage(err.to_string()))?;
    let request = NnsNodeProviderInfoRequest {
        cache: cache_request(&icp_root, &options.network),
        source_endpoint: options.source_endpoint,
        input: options.input,
        now_unix_secs: now_unix_secs()?,
    };
    let report = build_nns_node_provider_info_report(&request)?;
    write_text_or_json(options.format, &report, nns_node_provider_info_report_text)
}

fn run_node_provider_refresh<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, node_provider_refresh_usage, version_text()) {
        return Ok(());
    }
    let options = NodeProviderRefreshOptions::parse(args)?;
    let format = options.format;
    let icp_root = icp_root().map_err(|err| NnsCommandError::Usage(err.to_string()))?;
    let request = NnsNodeProviderRefreshRequest {
        cache: cache_request(&icp_root, &options.network),
        source_endpoint: options.source_endpoint,
        now_unix_secs: now_unix_secs()?,
        lock_stale_after_seconds: options.lock_stale_after_seconds,
        dry_run: options.dry_run,
        output_path: options.output_path,
    };
    let report = refresh_nns_node_provider_report(&request)?;
    write_text_or_json(format, &report, nns_node_provider_refresh_report_text)
}

impl NodeProviderListOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(node_provider_list_command(), args)
            .map_err(|_| NnsCommandError::Usage(node_provider_list_usage()))?;
        Ok(Self {
            network: string_option(&matches, "network")
                .unwrap_or_else(|| MAINNET_NETWORK.to_string()),
            format: typed_option(&matches, "format").unwrap_or(OutputFormat::Text),
            source_endpoint: string_option(&matches, "source-endpoint")
                .unwrap_or_else(|| DEFAULT_NNS_SOURCE_ENDPOINT.to_string()),
            verbose: matches.get_flag("verbose"),
        })
    }
}

impl NodeProviderInfoOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(node_provider_info_command(), args)
            .map_err(|_| NnsCommandError::Usage(node_provider_info_usage()))?;
        Ok(Self {
            input: required_string(&matches, "input"),
            network: string_option(&matches, "network")
                .unwrap_or_else(|| MAINNET_NETWORK.to_string()),
            format: typed_option(&matches, "format").unwrap_or(OutputFormat::Text),
            source_endpoint: string_option(&matches, "source-endpoint")
                .unwrap_or_else(|| DEFAULT_NNS_SOURCE_ENDPOINT.to_string()),
        })
    }
}

impl NodeProviderRefreshOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(node_provider_refresh_command(), args)
            .map_err(|_| NnsCommandError::Usage(node_provider_refresh_usage()))?;
        Ok(Self {
            network: string_option(&matches, "network")
                .unwrap_or_else(|| MAINNET_NETWORK.to_string()),
            format: typed_option(&matches, "format").unwrap_or(OutputFormat::Text),
            source_endpoint: string_option(&matches, "source-endpoint")
                .unwrap_or_else(|| DEFAULT_NNS_SOURCE_ENDPOINT.to_string()),
            lock_stale_after_seconds: typed_option(&matches, "lock-stale-after")
                .unwrap_or(DEFAULT_NODE_PROVIDER_REFRESH_LOCK_STALE_SECONDS),
            dry_run: matches.get_flag("dry-run"),
            output_path: path_option(&matches, "output"),
        })
    }
}

fn parse_refresh_lock_stale_after(value: &str) -> Result<u64, String> {
    parse_duration_seconds(value).map_err(|err| err.to_string())
}

fn cache_request(icp_root: &std::path::Path, network: &str) -> NnsNodeProviderCacheRequest {
    NnsNodeProviderCacheRequest {
        icp_root: PathBuf::from(icp_root),
        network: network.to_string(),
    }
}

pub(super) fn node_provider_command() -> ClapCommand {
    ClapCommand::new("node-provider")
        .bin_name("canic nns node-provider")
        .about("Inspect NNS node-provider metadata")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list").about("List cached mainnet NNS node providers"),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("info").about("Show one cached mainnet NNS node provider"),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("refresh").about("Force-refresh and cache NNS node-provider metadata"),
        ))
}

fn node_provider_list_command() -> ClapCommand {
    ClapCommand::new("list")
        .bin_name("canic nns node-provider list")
        .about("List cached mainnet NNS node providers")
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
                .help("IC API endpoint used if the node-provider cache is missing"),
        )
        .arg(
            flag_arg("verbose").long("verbose").help(
                "Show full node-provider principals and reward-account metadata in text output",
            ),
        )
        .arg(internal_network_arg())
        .after_help(NODE_PROVIDER_LIST_HELP_AFTER)
}

fn node_provider_info_command() -> ClapCommand {
    ClapCommand::new("info")
        .bin_name("canic nns node-provider info")
        .about("Show one cached mainnet NNS node provider")
        .disable_help_flag(true)
        .arg(
            value_arg("input")
                .value_name("node-provider|node-provider-prefix")
                .required(true)
                .help("Node-provider principal or unique node-provider principal prefix"),
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
                .help("IC API endpoint used if the node-provider cache is missing"),
        )
        .arg(internal_network_arg())
        .after_help(NODE_PROVIDER_INFO_HELP_AFTER)
}

fn node_provider_refresh_command() -> ClapCommand {
    ClapCommand::new("refresh")
        .bin_name("canic nns node-provider refresh")
        .about("Force-refresh and cache NNS node-provider metadata")
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
                .help("IC API endpoint used for native NNS governance and registry queries"),
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
                .help("Fetch and validate without replacing the cached node-provider report"),
        )
        .arg(
            value_arg("output")
                .long("output")
                .value_name("path")
                .help("Also write the fetched node-provider JSON to this path"),
        )
        .arg(internal_network_arg())
        .after_help(NODE_PROVIDER_REFRESH_HELP_AFTER)
}

pub(super) fn node_provider_usage() -> String {
    render_help(node_provider_command())
}

pub(super) fn node_provider_list_usage() -> String {
    render_help(node_provider_list_command())
}

pub(super) fn node_provider_info_usage() -> String {
    render_help(node_provider_info_command())
}

pub(super) fn node_provider_refresh_usage() -> String {
    render_help(node_provider_refresh_command())
}
