use super::{NnsCommandError, OutputFormat, now_unix_secs, parse_format, write_text_or_json};
use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, parse_required_subcommand, passthrough_subcommand,
            render_help, required_string, string_option, typed_option, value_arg,
        },
        globals::internal_network_arg,
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::nns_node_provider::{
    DEFAULT_NNS_GOVERNANCE_SOURCE_ENDPOINT, NnsNodeProviderInfoRequest, NnsNodeProviderListRequest,
    build_nns_node_provider_info_report, build_nns_node_provider_list_report,
    nns_node_provider_info_report_text, nns_node_provider_list_report_text,
    nns_node_provider_list_report_verbose_text,
};
use canic_subnet_catalog::MAINNET_NETWORK;
use clap::Command as ClapCommand;
use std::ffi::OsString;

const NODE_PROVIDER_LIST_HELP_AFTER: &str = "\
Examples:
  canic nns node-provider list
  canic nns node-provider list --verbose
  canic --network ic nns node-provider list --format json";
const NODE_PROVIDER_INFO_HELP_AFTER: &str = "\
Examples:
  canic nns node-provider info <node-provider>
  canic nns node-provider info <node-provider-prefix>
  canic --network ic nns node-provider info <node-provider> --format json";

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
    let request = NnsNodeProviderListRequest {
        network: options.network,
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
    let request = NnsNodeProviderInfoRequest {
        network: options.network,
        source_endpoint: options.source_endpoint,
        input: options.input,
        now_unix_secs: now_unix_secs()?,
    };
    let report = build_nns_node_provider_info_report(&request)?;
    write_text_or_json(options.format, &report, nns_node_provider_info_report_text)
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
                .unwrap_or_else(|| DEFAULT_NNS_GOVERNANCE_SOURCE_ENDPOINT.to_string()),
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
                .unwrap_or_else(|| DEFAULT_NNS_GOVERNANCE_SOURCE_ENDPOINT.to_string()),
        })
    }
}

pub(super) fn node_provider_command() -> ClapCommand {
    ClapCommand::new("node-provider")
        .bin_name("canic nns node-provider")
        .about("Inspect NNS node-provider metadata")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list").about("List mainnet NNS node providers"),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("info").about("Show one mainnet NNS node provider"),
        ))
}

fn node_provider_list_command() -> ClapCommand {
    ClapCommand::new("list")
        .bin_name("canic nns node-provider list")
        .about("List mainnet NNS node providers")
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
                .help("IC API endpoint used for the NNS governance query"),
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
        .about("Show one mainnet NNS node provider")
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
                .help("IC API endpoint used for the NNS governance query"),
        )
        .arg(internal_network_arg())
        .after_help(NODE_PROVIDER_INFO_HELP_AFTER)
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
