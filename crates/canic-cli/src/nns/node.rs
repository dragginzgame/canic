use super::{
    NnsCommandError,
    leaf::{
        self, NnsLeafCommandSpec, NnsLeafInfoOptions, NnsLeafListOptions, NnsLeafRefreshOptions,
    },
    now_unix_secs, write_text_or_json,
};
use crate::{cli::help::print_help_or_version, version_text};
use canic_host::{
    nns_node::{
        DEFAULT_NNS_NODE_SOURCE_ENDPOINT, NnsNodeCacheRequest, NnsNodeInfoRequest,
        NnsNodeListRequest, NnsNodeRefreshRequest, build_nns_node_info_report,
        build_nns_node_list_report, nns_node_info_report_text, nns_node_list_report_text,
        nns_node_list_report_verbose_text, nns_node_refresh_report_text, refresh_nns_node_report,
    },
    release_set::icp_root,
};
use std::{ffi::OsString, path::PathBuf};

const NODE_LIST_HELP_AFTER: &str = "\
Examples:
  canic nns node list
  canic nns node list --verbose
  canic --network ic nns node list --format json

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

pub(super) fn node_list_options<I>(args: I) -> Result<NnsLeafListOptions, NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    NnsLeafListOptions::parse(args, &NODE_SPEC, DEFAULT_NNS_NODE_SOURCE_ENDPOINT)
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
    leaf::list_usage(&NODE_SPEC, DEFAULT_NNS_NODE_SOURCE_ENDPOINT)
}

pub(super) fn node_info_usage() -> String {
    leaf::info_usage(&NODE_SPEC, DEFAULT_NNS_NODE_SOURCE_ENDPOINT)
}

pub(super) fn node_refresh_usage() -> String {
    leaf::refresh_usage(&NODE_SPEC, DEFAULT_NNS_NODE_SOURCE_ENDPOINT)
}
