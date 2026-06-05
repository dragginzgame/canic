use super::{
    NnsCommandError,
    leaf::{
        self, NnsLeafCommandSpec, NnsLeafInfoOptions, NnsLeafListOptions, NnsLeafRefreshOptions,
    },
    now_unix_secs, write_text_or_json,
};
use crate::{cli::help::print_help_or_version, version_text};
use canic_host::{
    nns_data_center::{
        DEFAULT_NNS_DATA_CENTER_SOURCE_ENDPOINT, NnsDataCenterCacheRequest,
        NnsDataCenterInfoRequest, NnsDataCenterListRequest, NnsDataCenterRefreshRequest,
        build_nns_data_center_info_report, build_nns_data_center_list_report,
        nns_data_center_info_report_text, nns_data_center_list_report_text,
        nns_data_center_list_report_verbose_text, nns_data_center_refresh_report_text,
        refresh_nns_data_center_report,
    },
    release_set::icp_root,
};
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

const DATA_CENTER_SPEC: NnsLeafCommandSpec = NnsLeafCommandSpec {
    command_name: "data-center",
    bin_name: "canic nns data-center",
    about: "Inspect NNS data-center metadata",
    list_about: "List cached mainnet NNS data centers",
    info_about: "Show one cached mainnet NNS data center",
    refresh_about: "Force-refresh and cache NNS data-center metadata",
    list_help_after: DATA_CENTER_LIST_HELP_AFTER,
    info_help_after: DATA_CENTER_INFO_HELP_AFTER,
    refresh_help_after: DATA_CENTER_REFRESH_HELP_AFTER,
    input_value_name: "data-center|data-center-prefix",
    input_help: "Data-center id or unique data-center id prefix",
    list_source_help: "IC API endpoint used if the data-center cache is missing",
    info_source_help: "IC API endpoint used if the data-center cache is missing",
    refresh_source_help: "IC API endpoint used for native NNS registry queries",
    verbose_help: "Show GPS coordinates and registry metadata in text output",
    dry_run_help: "Fetch and validate without replacing the cached data-center report",
    output_help: "Also write the fetched data-center JSON to this path",
};

pub(super) fn run<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    leaf::run_leaf(
        args,
        &DATA_CENTER_SPEC,
        run_data_center_list,
        run_data_center_info,
        run_data_center_refresh,
    )
}

fn run_data_center_list(args: Vec<OsString>) -> Result<(), NnsCommandError> {
    if print_help_or_version(&args, data_center_list_usage, version_text()) {
        return Ok(());
    }
    let options = data_center_list_options(args)?;
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

fn run_data_center_info(args: Vec<OsString>) -> Result<(), NnsCommandError> {
    if print_help_or_version(&args, data_center_info_usage, version_text()) {
        return Ok(());
    }
    let options = data_center_info_options(args)?;
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

fn run_data_center_refresh(args: Vec<OsString>) -> Result<(), NnsCommandError> {
    if print_help_or_version(&args, data_center_refresh_usage, version_text()) {
        return Ok(());
    }
    let options = data_center_refresh_options(args)?;
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

pub(super) fn data_center_list_options<I>(args: I) -> Result<NnsLeafListOptions, NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    NnsLeafListOptions::parse(
        args,
        &DATA_CENTER_SPEC,
        DEFAULT_NNS_DATA_CENTER_SOURCE_ENDPOINT,
    )
}

pub(super) fn data_center_info_options<I>(args: I) -> Result<NnsLeafInfoOptions, NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    NnsLeafInfoOptions::parse(
        args,
        &DATA_CENTER_SPEC,
        DEFAULT_NNS_DATA_CENTER_SOURCE_ENDPOINT,
    )
}

pub(super) fn data_center_refresh_options<I>(
    args: I,
) -> Result<NnsLeafRefreshOptions, NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    NnsLeafRefreshOptions::parse(
        args,
        &DATA_CENTER_SPEC,
        DEFAULT_NNS_DATA_CENTER_SOURCE_ENDPOINT,
    )
}

fn cache_request(icp_root: &std::path::Path, network: &str) -> NnsDataCenterCacheRequest {
    NnsDataCenterCacheRequest {
        icp_root: PathBuf::from(icp_root),
        network: network.to_string(),
    }
}

#[cfg(test)]
pub(super) fn data_center_usage() -> String {
    leaf::usage(&DATA_CENTER_SPEC)
}

pub(super) fn data_center_list_usage() -> String {
    leaf::list_usage(&DATA_CENTER_SPEC, DEFAULT_NNS_DATA_CENTER_SOURCE_ENDPOINT)
}

pub(super) fn data_center_info_usage() -> String {
    leaf::info_usage(&DATA_CENTER_SPEC, DEFAULT_NNS_DATA_CENTER_SOURCE_ENDPOINT)
}

pub(super) fn data_center_refresh_usage() -> String {
    leaf::refresh_usage(&DATA_CENTER_SPEC, DEFAULT_NNS_DATA_CENTER_SOURCE_ENDPOINT)
}
