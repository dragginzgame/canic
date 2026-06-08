use super::{
    NnsCommandError, OutputFormat,
    leaf::{self, NnsCommonOptions, NnsLeafCommandSpec, NnsLeafInfoOptions, NnsLeafRefreshOptions},
    now_unix_secs, write_text_or_json,
};
use crate::{
    cli::{
        clap::{parse_matches, render_help, typed_option, value_arg},
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::{
    nns_node::{
        DEFAULT_NNS_NODE_SOURCE_ENDPOINT, NNS_NODE_SUBNET_KIND_APPLICATION,
        NNS_NODE_SUBNET_KIND_SYSTEM, NNS_NODE_SUBNET_KIND_UNKNOWN, NnsNodeCacheRequest,
        NnsNodeInfoRequest, NnsNodeListFilters, NnsNodeListRequest, NnsNodeRefreshRequest,
        build_nns_node_info_report, build_nns_node_list_report, nns_node_info_report_text,
        nns_node_list_report_text, nns_node_list_report_verbose_text, nns_node_refresh_report_text,
        refresh_nns_node_report,
    },
    release_set::icp_root,
};
use std::{ffi::OsString, path::PathBuf};

const SUBNET_FILTER_ARG: &str = "subnet";
const SUBNET_KIND_FILTER_ARG: &str = "kind";
const DATA_CENTER_FILTER_ARG: &str = "data-center";
const NODE_PROVIDER_FILTER_ARG: &str = "node-provider";
const NODE_OPERATOR_FILTER_ARG: &str = "node-operator";
const NODE_LIST_HELP_AFTER: &str = "\
Examples:
  canic nns node list
  canic nns node list --verbose
  canic --network ic nns node list --format json
  canic nns node list --data-center zh2
  canic nns node list --node-provider 7at4h
  canic nns node list --subnet tdb26 --kind system

Force-refresh cached native NNS data:
  canic nns node refresh";
const NODE_INFO_HELP_AFTER: &str = "\
Examples:
  canic nns node info <node>
  canic nns node info <node-prefix>
  canic --network ic nns node info <node> --format json

Force-refresh cached native NNS data:
  canic nns node refresh";
const NODE_REFRESH_HELP_AFTER: &str = "\
Examples:
  canic nns node refresh
  canic --network ic nns node refresh --format json
  canic nns node refresh --dry-run --output .canic/node/ic/nodes.preview.json";

const NODE_SPEC: NnsLeafCommandSpec = NnsLeafCommandSpec {
    command_name: "node",
    bin_name: "canic nns node",
    about: "Inspect NNS node metadata",
    list_about: "List cached mainnet NNS nodes",
    info_about: "Show one cached mainnet NNS node",
    refresh_about: "Force-refresh and cache NNS node metadata",
    list_help_after: NODE_LIST_HELP_AFTER,
    info_help_after: NODE_INFO_HELP_AFTER,
    refresh_help_after: NODE_REFRESH_HELP_AFTER,
    input_value_name: "node|node-prefix",
    input_help: "Node principal or unique node principal prefix",
    list_source_help: "IC API endpoint used if the node cache is missing",
    info_source_help: "IC API endpoint used if the node cache is missing",
    refresh_source_help: "IC API endpoint used for native NNS registry queries",
    verbose_help: "Show full node principals and registry metadata in text output",
    dry_run_help: "Fetch and validate without replacing the cached node report",
    output_help: "Also write the fetched node JSON to this path",
};

///
/// NnsNodeListOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct NnsNodeListOptions {
    pub(super) network: String,
    pub(super) format: OutputFormat,
    pub(super) source_endpoint: String,
    pub(super) verbose: bool,
    pub(super) filters: NnsNodeListFilters,
}

pub(super) fn run<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    leaf::run_leaf(
        args,
        &NODE_SPEC,
        run_node_list,
        run_node_info,
        run_node_refresh,
    )
}

fn run_node_list(args: Vec<OsString>) -> Result<(), NnsCommandError> {
    if print_help_or_version(&args, node_list_usage, version_text()) {
        return Ok(());
    }
    let options = node_list_options(args)?;
    let icp_root = icp_root().map_err(|err| NnsCommandError::Usage(err.to_string()))?;
    let request = NnsNodeListRequest {
        cache: cache_request(&icp_root, &options.network),
        source_endpoint: options.source_endpoint,
        now_unix_secs: now_unix_secs()?,
        filters: options.filters,
    };
    let report = build_nns_node_list_report(&request)?;
    write_text_or_json(options.format, &report, |report| {
        if options.verbose {
            nns_node_list_report_verbose_text(report)
        } else {
            nns_node_list_report_text(report)
        }
    })
}

fn run_node_info(args: Vec<OsString>) -> Result<(), NnsCommandError> {
    if print_help_or_version(&args, node_info_usage, version_text()) {
        return Ok(());
    }
    let options = node_info_options(args)?;
    let icp_root = icp_root().map_err(|err| NnsCommandError::Usage(err.to_string()))?;
    let request = NnsNodeInfoRequest {
        cache: cache_request(&icp_root, &options.network),
        source_endpoint: options.source_endpoint,
        input: options.input,
        now_unix_secs: now_unix_secs()?,
    };
    let report = build_nns_node_info_report(&request)?;
    write_text_or_json(options.format, &report, nns_node_info_report_text)
}

fn run_node_refresh(args: Vec<OsString>) -> Result<(), NnsCommandError> {
    if print_help_or_version(&args, node_refresh_usage, version_text()) {
        return Ok(());
    }
    let options = node_refresh_options(args)?;
    let format = options.format;
    let icp_root = icp_root().map_err(|err| NnsCommandError::Usage(err.to_string()))?;
    let request = NnsNodeRefreshRequest {
        cache: cache_request(&icp_root, &options.network),
        source_endpoint: options.source_endpoint,
        now_unix_secs: now_unix_secs()?,
        lock_stale_after_seconds: options.lock_stale_after_seconds,
        dry_run: options.dry_run,
        output_path: options.output_path,
    };
    let report = refresh_nns_node_report(&request)?;
    write_text_or_json(format, &report, nns_node_refresh_report_text)
}

pub(super) fn node_list_options<I>(args: I) -> Result<NnsNodeListOptions, NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let matches = parse_matches(node_list_command(), args)
        .map_err(|_| NnsCommandError::Usage(node_list_usage()))?;
    let common = NnsCommonOptions::from_matches(&matches);
    Ok(NnsNodeListOptions {
        network: common.network,
        format: common.format,
        source_endpoint: common.source_endpoint,
        verbose: matches.get_flag("verbose"),
        filters: NnsNodeListFilters {
            subnet: typed_option(&matches, SUBNET_FILTER_ARG),
            subnet_kind: typed_option(&matches, SUBNET_KIND_FILTER_ARG),
            data_center: typed_option(&matches, DATA_CENTER_FILTER_ARG),
            node_provider: typed_option(&matches, NODE_PROVIDER_FILTER_ARG),
            node_operator: typed_option(&matches, NODE_OPERATOR_FILTER_ARG),
        },
    })
}

pub(super) fn node_info_options<I>(args: I) -> Result<NnsLeafInfoOptions, NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    NnsLeafInfoOptions::parse(args, &NODE_SPEC, DEFAULT_NNS_NODE_SOURCE_ENDPOINT)
}

pub(super) fn node_refresh_options<I>(args: I) -> Result<NnsLeafRefreshOptions, NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    NnsLeafRefreshOptions::parse(args, &NODE_SPEC, DEFAULT_NNS_NODE_SOURCE_ENDPOINT)
}

fn cache_request(icp_root: &std::path::Path, network: &str) -> NnsNodeCacheRequest {
    NnsNodeCacheRequest {
        icp_root: PathBuf::from(icp_root),
        network: network.to_string(),
    }
}

#[cfg(test)]
pub(super) fn node_usage() -> String {
    leaf::usage(&NODE_SPEC)
}

pub(super) fn node_list_usage() -> String {
    render_help(node_list_command())
}

pub(super) fn node_info_usage() -> String {
    leaf::info_usage(&NODE_SPEC, DEFAULT_NNS_NODE_SOURCE_ENDPOINT)
}

pub(super) fn node_refresh_usage() -> String {
    leaf::refresh_usage(&NODE_SPEC, DEFAULT_NNS_NODE_SOURCE_ENDPOINT)
}

fn node_list_command() -> clap::Command {
    leaf::list_command(&NODE_SPEC, DEFAULT_NNS_NODE_SOURCE_ENDPOINT)
        .arg(
            value_arg(SUBNET_FILTER_ARG)
                .long(SUBNET_FILTER_ARG)
                .value_name("subnet|subnet-prefix")
                .help("Show only nodes assigned to a subnet principal or prefix"),
        )
        .arg(
            value_arg(SUBNET_KIND_FILTER_ARG)
                .long(SUBNET_KIND_FILTER_ARG)
                .value_name("application|system|unknown")
                .value_parser([
                    NNS_NODE_SUBNET_KIND_APPLICATION,
                    NNS_NODE_SUBNET_KIND_SYSTEM,
                    NNS_NODE_SUBNET_KIND_UNKNOWN,
                ])
                .help("Show only nodes assigned to subnets of this kind"),
        )
        .arg(
            value_arg(DATA_CENTER_FILTER_ARG)
                .long(DATA_CENTER_FILTER_ARG)
                .value_name("data-center|data-center-prefix")
                .help("Show only nodes in a data center id or prefix"),
        )
        .arg(
            value_arg(NODE_PROVIDER_FILTER_ARG)
                .long(NODE_PROVIDER_FILTER_ARG)
                .value_name("node-provider|node-provider-prefix")
                .help("Show only nodes owned by a node-provider principal or prefix"),
        )
        .arg(
            value_arg(NODE_OPERATOR_FILTER_ARG)
                .long(NODE_OPERATOR_FILTER_ARG)
                .value_name("node-operator|node-operator-prefix")
                .help("Show only nodes owned by a node-operator principal or prefix"),
        )
}
