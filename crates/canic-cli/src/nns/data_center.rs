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
    nns_data_center::{
        DEFAULT_DATA_CENTER_REFRESH_LOCK_STALE_SECONDS, DEFAULT_NNS_DATA_CENTER_SOURCE_ENDPOINT,
        NnsDataCenterCacheRequest, NnsDataCenterInfoRequest, NnsDataCenterListRequest,
        NnsDataCenterRefreshRequest, build_nns_data_center_info_report,
        build_nns_data_center_list_report, nns_data_center_info_report_text,
        nns_data_center_list_report_text, nns_data_center_list_report_verbose_text,
        nns_data_center_refresh_report_text, refresh_nns_data_center_report,
    },
    release_set::icp_root,
};
use canic_subnet_catalog::MAINNET_NETWORK;
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

const DATA_CENTER_LIST_HELP_AFTER: &str = "\
Examples:
  canic nns data-center list
  canic nns data-center list --verbose
  canic --network ic nns data-center list --format json

Force-refresh cached native NNS data:
  canic nns data-center refresh";
const DATA_CENTER_INFO_HELP_AFTER: &str = "\
Examples:
  canic nns data-center info <data-center>
  canic nns data-center info <data-center-prefix>
  canic --network ic nns data-center info <data-center> --format json

Force-refresh cached native NNS data:
  canic nns data-center refresh";
const DATA_CENTER_REFRESH_HELP_AFTER: &str = "\
Examples:
  canic nns data-center refresh
  canic --network ic nns data-center refresh --format json
  canic nns data-center refresh --dry-run --output .canic/data-center/ic/data-centers.preview.json";

///
/// DataCenterListOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DataCenterListOptions {
    pub(super) network: String,
    pub(super) format: OutputFormat,
    pub(super) source_endpoint: String,
    pub(super) verbose: bool,
}

///
/// DataCenterInfoOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DataCenterInfoOptions {
    pub(super) input: String,
    pub(super) network: String,
    pub(super) format: OutputFormat,
    pub(super) source_endpoint: String,
}

///
/// DataCenterRefreshOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DataCenterRefreshOptions {
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
    if print_help_or_version(&args, data_center_usage, version_text()) {
        return Ok(());
    }
    let (command, args) = parse_required_subcommand(data_center_command(), args)
        .map_err(|_| NnsCommandError::Usage(data_center_usage()))?;

    match command.as_str() {
        "list" => run_data_center_list(args),
        "info" => run_data_center_info(args),
        "refresh" => run_data_center_refresh(args),
        _ => unreachable!("nns data-center dispatch command only defines known commands"),
    }
}

fn run_data_center_list<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, data_center_list_usage, version_text()) {
        return Ok(());
    }
    let options = DataCenterListOptions::parse(args)?;
    let icp_root = icp_root().map_err(|err| NnsCommandError::Usage(err.to_string()))?;
    let request = NnsDataCenterListRequest {
        cache: cache_request(&icp_root, &options.network),
        source_endpoint: options.source_endpoint,
        now_unix_secs: now_unix_secs()?,
    };
    let report = build_nns_data_center_list_report(&request)?;
    write_text_or_json(options.format, &report, |report| {
        if options.verbose {
            nns_data_center_list_report_verbose_text(report)
        } else {
            nns_data_center_list_report_text(report)
        }
    })
}

fn run_data_center_info<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, data_center_info_usage, version_text()) {
        return Ok(());
    }
    let options = DataCenterInfoOptions::parse(args)?;
    let icp_root = icp_root().map_err(|err| NnsCommandError::Usage(err.to_string()))?;
    let request = NnsDataCenterInfoRequest {
        cache: cache_request(&icp_root, &options.network),
        source_endpoint: options.source_endpoint,
        input: options.input,
        now_unix_secs: now_unix_secs()?,
    };
    let report = build_nns_data_center_info_report(&request)?;
    write_text_or_json(options.format, &report, nns_data_center_info_report_text)
}

fn run_data_center_refresh<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, data_center_refresh_usage, version_text()) {
        return Ok(());
    }
    let options = DataCenterRefreshOptions::parse(args)?;
    let format = options.format;
    let icp_root = icp_root().map_err(|err| NnsCommandError::Usage(err.to_string()))?;
    let request = NnsDataCenterRefreshRequest {
        cache: cache_request(&icp_root, &options.network),
        source_endpoint: options.source_endpoint,
        now_unix_secs: now_unix_secs()?,
        lock_stale_after_seconds: options.lock_stale_after_seconds,
        dry_run: options.dry_run,
        output_path: options.output_path,
    };
    let report = refresh_nns_data_center_report(&request)?;
    write_text_or_json(format, &report, nns_data_center_refresh_report_text)
}

impl DataCenterListOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(data_center_list_command(), args)
            .map_err(|_| NnsCommandError::Usage(data_center_list_usage()))?;
        Ok(Self {
            network: string_option(&matches, "network")
                .unwrap_or_else(|| MAINNET_NETWORK.to_string()),
            format: typed_option(&matches, "format").unwrap_or(OutputFormat::Text),
            source_endpoint: string_option(&matches, "source-endpoint")
                .unwrap_or_else(|| DEFAULT_NNS_DATA_CENTER_SOURCE_ENDPOINT.to_string()),
            verbose: matches.get_flag("verbose"),
        })
    }
}

impl DataCenterInfoOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(data_center_info_command(), args)
            .map_err(|_| NnsCommandError::Usage(data_center_info_usage()))?;
        Ok(Self {
            input: required_string(&matches, "input"),
            network: string_option(&matches, "network")
                .unwrap_or_else(|| MAINNET_NETWORK.to_string()),
            format: typed_option(&matches, "format").unwrap_or(OutputFormat::Text),
            source_endpoint: string_option(&matches, "source-endpoint")
                .unwrap_or_else(|| DEFAULT_NNS_DATA_CENTER_SOURCE_ENDPOINT.to_string()),
        })
    }
}

impl DataCenterRefreshOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(data_center_refresh_command(), args)
            .map_err(|_| NnsCommandError::Usage(data_center_refresh_usage()))?;
        Ok(Self {
            network: string_option(&matches, "network")
                .unwrap_or_else(|| MAINNET_NETWORK.to_string()),
            format: typed_option(&matches, "format").unwrap_or(OutputFormat::Text),
            source_endpoint: string_option(&matches, "source-endpoint")
                .unwrap_or_else(|| DEFAULT_NNS_DATA_CENTER_SOURCE_ENDPOINT.to_string()),
            lock_stale_after_seconds: typed_option(&matches, "lock-stale-after")
                .unwrap_or(DEFAULT_DATA_CENTER_REFRESH_LOCK_STALE_SECONDS),
            dry_run: matches.get_flag("dry-run"),
            output_path: path_option(&matches, "output"),
        })
    }
}

fn parse_refresh_lock_stale_after(value: &str) -> Result<u64, String> {
    parse_duration_seconds(value).map_err(|err| err.to_string())
}

fn cache_request(icp_root: &std::path::Path, network: &str) -> NnsDataCenterCacheRequest {
    NnsDataCenterCacheRequest {
        icp_root: PathBuf::from(icp_root),
        network: network.to_string(),
    }
}

pub(super) fn data_center_command() -> ClapCommand {
    ClapCommand::new("data-center")
        .bin_name("canic nns data-center")
        .about("Inspect NNS data-center metadata")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list").about("List cached mainnet NNS data centers"),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("info").about("Show one cached mainnet NNS data center"),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("refresh").about("Force-refresh and cache NNS data-center metadata"),
        ))
}

fn data_center_list_command() -> ClapCommand {
    ClapCommand::new("list")
        .bin_name("canic nns data-center list")
        .about("List cached mainnet NNS data centers")
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
                .help("IC API endpoint used if the data-center cache is missing"),
        )
        .arg(
            flag_arg("verbose")
                .long("verbose")
                .help("Show GPS coordinates and registry metadata in text output"),
        )
        .arg(internal_network_arg())
        .after_help(DATA_CENTER_LIST_HELP_AFTER)
}

fn data_center_info_command() -> ClapCommand {
    ClapCommand::new("info")
        .bin_name("canic nns data-center info")
        .about("Show one cached mainnet NNS data center")
        .disable_help_flag(true)
        .arg(
            value_arg("input")
                .value_name("data-center|data-center-prefix")
                .required(true)
                .help("Data-center id or unique data-center id prefix"),
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
                .help("IC API endpoint used if the data-center cache is missing"),
        )
        .arg(internal_network_arg())
        .after_help(DATA_CENTER_INFO_HELP_AFTER)
}

fn data_center_refresh_command() -> ClapCommand {
    ClapCommand::new("refresh")
        .bin_name("canic nns data-center refresh")
        .about("Force-refresh and cache NNS data-center metadata")
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
                .help("Fetch and validate without replacing the cached data-center report"),
        )
        .arg(
            value_arg("output")
                .long("output")
                .value_name("path")
                .help("Also write the fetched data-center JSON to this path"),
        )
        .arg(internal_network_arg())
        .after_help(DATA_CENTER_REFRESH_HELP_AFTER)
}

pub(super) fn data_center_usage() -> String {
    render_help(data_center_command())
}

pub(super) fn data_center_list_usage() -> String {
    render_help(data_center_list_command())
}

pub(super) fn data_center_info_usage() -> String {
    render_help(data_center_info_command())
}

pub(super) fn data_center_refresh_usage() -> String {
    render_help(data_center_refresh_command())
}
