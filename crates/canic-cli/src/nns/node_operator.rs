use super::{
    NnsCommandError,
    leaf::{
        self, NnsLeafCommandSpec, NnsLeafInfoOptions, NnsLeafListOptions, NnsLeafRefreshOptions,
    },
    now_unix_secs, write_text_or_json,
};
use crate::{cli::help::print_help_or_version, version_text};
use canic_host::{
    nns_node_operator::{
        DEFAULT_NNS_NODE_OPERATOR_SOURCE_ENDPOINT, NnsNodeOperatorCacheRequest,
        NnsNodeOperatorInfoRequest, NnsNodeOperatorListRequest, NnsNodeOperatorRefreshRequest,
        build_nns_node_operator_info_report, build_nns_node_operator_list_report,
        nns_node_operator_info_report_text, nns_node_operator_list_report_text,
        nns_node_operator_list_report_verbose_text, nns_node_operator_refresh_report_text,
        refresh_nns_node_operator_report,
    },
    release_set::icp_root,
};
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

const NODE_OPERATOR_SPEC: NnsLeafCommandSpec = NnsLeafCommandSpec {
    command_name: "node-operator",
    bin_name: "canic nns node-operator",
    about: "Inspect NNS node-operator metadata",
    list_about: "List cached mainnet NNS node operators",
    info_about: "Show one cached mainnet NNS node operator",
    refresh_about: "Force-refresh and cache NNS node-operator metadata",
    list_help_after: NODE_OPERATOR_LIST_HELP_AFTER,
    info_help_after: NODE_OPERATOR_INFO_HELP_AFTER,
    refresh_help_after: NODE_OPERATOR_REFRESH_HELP_AFTER,
    input_value_name: "node-operator|node-operator-prefix",
    input_help: "Node-operator principal or unique node-operator principal prefix",
    list_source_help: "IC API endpoint used if the node-operator cache is missing",
    info_source_help: "IC API endpoint used if the node-operator cache is missing",
    refresh_source_help: "IC API endpoint used for native NNS registry queries",
    verbose_help: "Show full node-operator principals and registry metadata in text output",
    dry_run_help: "Fetch and validate without replacing the cached node-operator report",
    output_help: "Also write the fetched node-operator JSON to this path",
};

pub(super) fn run<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    leaf::run_leaf(
        args,
        &NODE_OPERATOR_SPEC,
        run_node_operator_list,
        run_node_operator_info,
        run_node_operator_refresh,
    )
}

fn run_node_operator_list(args: Vec<OsString>) -> Result<(), NnsCommandError> {
    if print_help_or_version(&args, node_operator_list_usage, version_text()) {
        return Ok(());
    }
    let options = node_operator_list_options(args)?;
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

fn run_node_operator_info(args: Vec<OsString>) -> Result<(), NnsCommandError> {
    if print_help_or_version(&args, node_operator_info_usage, version_text()) {
        return Ok(());
    }
    let options = node_operator_info_options(args)?;
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

fn run_node_operator_refresh(args: Vec<OsString>) -> Result<(), NnsCommandError> {
    if print_help_or_version(&args, node_operator_refresh_usage, version_text()) {
        return Ok(());
    }
    let options = node_operator_refresh_options(args)?;
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

pub(super) fn node_operator_list_options<I>(args: I) -> Result<NnsLeafListOptions, NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    NnsLeafListOptions::parse(
        args,
        &NODE_OPERATOR_SPEC,
        DEFAULT_NNS_NODE_OPERATOR_SOURCE_ENDPOINT,
    )
}

pub(super) fn node_operator_info_options<I>(args: I) -> Result<NnsLeafInfoOptions, NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    NnsLeafInfoOptions::parse(
        args,
        &NODE_OPERATOR_SPEC,
        DEFAULT_NNS_NODE_OPERATOR_SOURCE_ENDPOINT,
    )
}

pub(super) fn node_operator_refresh_options<I>(
    args: I,
) -> Result<NnsLeafRefreshOptions, NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    NnsLeafRefreshOptions::parse(
        args,
        &NODE_OPERATOR_SPEC,
        DEFAULT_NNS_NODE_OPERATOR_SOURCE_ENDPOINT,
    )
}

fn cache_request(icp_root: &std::path::Path, network: &str) -> NnsNodeOperatorCacheRequest {
    NnsNodeOperatorCacheRequest {
        icp_root: PathBuf::from(icp_root),
        network: network.to_string(),
    }
}

#[cfg(test)]
pub(super) fn node_operator_usage() -> String {
    leaf::usage(&NODE_OPERATOR_SPEC)
}

pub(super) fn node_operator_list_usage() -> String {
    leaf::list_usage(
        &NODE_OPERATOR_SPEC,
        DEFAULT_NNS_NODE_OPERATOR_SOURCE_ENDPOINT,
    )
}

pub(super) fn node_operator_info_usage() -> String {
    leaf::info_usage(
        &NODE_OPERATOR_SPEC,
        DEFAULT_NNS_NODE_OPERATOR_SOURCE_ENDPOINT,
    )
}

pub(super) fn node_operator_refresh_usage() -> String {
    leaf::refresh_usage(
        &NODE_OPERATOR_SPEC,
        DEFAULT_NNS_NODE_OPERATOR_SOURCE_ENDPOINT,
    )
}
