#[cfg(test)]
mod tests;

use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, parse_positive_usize, parse_subcommand, parse_usize,
            passthrough_subcommand, path_option, render_help, required_string, string_option,
            string_option_or_else, typed_option, value_arg,
        },
        defaults::default_icp,
        globals::{internal_icp_arg, internal_network_arg},
        help::print_help_or_version,
    },
    output::{write_pretty_json, write_text},
    version_text,
};
use canic_host::{
    installed_deployment::{
        InstalledDeploymentError, InstalledDeploymentRequest, InstalledDeploymentResolution,
        read_installed_deployment_state_from_root, resolve_installed_deployment_from_root,
    },
    nns_node_provider::{
        DEFAULT_NNS_GOVERNANCE_SOURCE_ENDPOINT, NnsNodeProviderHostError,
        NnsNodeProviderListRequest, build_nns_node_provider_list_report,
        nns_node_provider_list_report_text,
    },
    release_set::icp_root,
    subnet_catalog::{
        DEFAULT_REFRESH_LOCK_STALE_SECONDS, DEFAULT_STALE_AFTER_SECONDS,
        DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT, ResolvedDeploymentTarget,
        SubnetCatalogCacheRequest, SubnetCatalogFilters, SubnetCatalogHostError,
        SubnetCatalogInfoRequest, SubnetCatalogListRequest, SubnetCatalogRefreshRequest,
        build_subnet_catalog_info_report, build_subnet_catalog_list_report,
        parse_stale_after_duration, refresh_subnet_catalog, subnet_catalog_info_report_text,
        subnet_catalog_list_report_text, subnet_catalog_list_report_verbose_text,
        subnet_catalog_refresh_report_text,
    },
};
use canic_subnet_catalog::{
    CatalogError, GeographicScope, MAINNET_NETWORK, ResolveAs, SubnetKind, SubnetSpecialization,
    canonical_principal_text,
};
use clap::Command as ClapCommand;
use serde::Serialize;
use std::{
    ffi::OsString,
    io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

const DEFAULT_RANGE_LIMIT: usize = 50;
const INFO_INPUT_VALUE_NAME: &str = "subnet|canister|subnet-prefix|deployment-target";
const INFO_INPUT_HELP: &str = "\
Subnet/canister principal, unique subnet prefix, deployment target, or \
<deployment>/<role-or-canister>";
const LIST_HELP_AFTER: &str = "\
Examples:
  canic nns subnet list
  canic nns subnet list --verbose
  canic --network ic nns subnet list --format json
  canic nns subnet list --kind application --specialization fiduciary

Refresh stale cache:
  canic nns subnet refresh";
const INFO_HELP_AFTER: &str = "\
Examples:
  canic nns subnet info ryjl3-tyaaa-aaaaa-aaaba-cai
  canic nns subnet info <subnet-prefix>
  canic nns subnet info <deployment>
  canic nns subnet info <deployment>/<role-or-canister>

Refresh stale cache:
  canic nns subnet refresh";
const REFRESH_HELP_AFTER: &str = "\
Examples:
  canic nns subnet refresh
  canic --network ic nns subnet refresh --format json
  canic nns subnet refresh --dry-run --output .canic/subnet-catalog/ic/catalog.preview.json";
const NODE_PROVIDER_LIST_HELP_AFTER: &str = "\
Examples:
  canic nns node-provider list
  canic --network ic nns node-provider list --format json";

///
/// NnsCommandError
///
#[derive(Debug, ThisError)]
pub enum NnsCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    SubnetHost(#[from] SubnetCatalogHostError),

    #[error(transparent)]
    NodeProviderHost(#[from] NnsNodeProviderHostError),

    #[error(
        "deployment target {input} did not resolve to exactly one canister principal for network {network}: {reason}"
    )]
    TargetResolutionFailed {
        input: String,
        network: String,
        reason: String,
    },

    #[error("system clock before unix epoch: {0}")]
    Clock(String),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

///
/// OutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    Text,
    Json,
}

///
/// CatalogListOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct CatalogListOptions {
    network: String,
    format: OutputFormat,
    filters: SubnetCatalogFilters,
    show_ranges: bool,
    verbose: bool,
    range_limit: usize,
    range_offset: usize,
}

///
/// CatalogInfoOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct CatalogInfoOptions {
    input: String,
    network: String,
    icp: String,
    format: OutputFormat,
    forced: Option<ResolveAs>,
}

///
/// CatalogRefreshOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct CatalogRefreshOptions {
    network: String,
    format: OutputFormat,
    source_endpoint: String,
    lock_stale_after_seconds: u64,
    dry_run: bool,
    output_path: Option<PathBuf>,
}

///
/// NodeProviderListOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct NodeProviderListOptions {
    network: String,
    format: OutputFormat,
    source_endpoint: String,
}

pub fn run<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }
    let Some((command, args)) =
        parse_subcommand(nns_command(), args).map_err(|_| NnsCommandError::Usage(usage()))?
    else {
        return Err(NnsCommandError::Usage(usage()));
    };

    match command.as_str() {
        "subnet" => run_subnet(args),
        "node-provider" => run_node_provider(args),
        _ => unreachable!("nns dispatch command only defines known commands"),
    }
}

fn run_subnet<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, subnet_usage, version_text()) {
        return Ok(());
    }
    let Some((command, args)) = parse_subcommand(subnet_command(), args)
        .map_err(|_| NnsCommandError::Usage(subnet_usage()))?
    else {
        return Err(NnsCommandError::Usage(subnet_usage()));
    };

    match command.as_str() {
        "list" => run_catalog_list(args),
        "info" => run_catalog_info(args),
        "refresh" => run_catalog_refresh(args),
        _ => unreachable!("nns subnet dispatch command only defines known commands"),
    }
}

fn run_node_provider<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, node_provider_usage, version_text()) {
        return Ok(());
    }
    let Some((command, args)) = parse_subcommand(node_provider_command(), args)
        .map_err(|_| NnsCommandError::Usage(node_provider_usage()))?
    else {
        return Err(NnsCommandError::Usage(node_provider_usage()));
    };

    match command.as_str() {
        "list" => run_node_provider_list(args),
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
    write_text_or_json(options.format, &report, nns_node_provider_list_report_text)
}

fn run_catalog_list<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, list_usage, version_text()) {
        return Ok(());
    }
    let options = CatalogListOptions::parse(args)?;
    let format = options.format;
    let verbose = options.verbose;
    let icp_root = icp_root().map_err(|err| NnsCommandError::Usage(err.to_string()))?;
    let request = SubnetCatalogListRequest {
        cache: cache_request(&icp_root, &options.network),
        now_unix_secs: now_unix_secs()?,
        stale_after_seconds: DEFAULT_STALE_AFTER_SECONDS,
        filters: options.filters,
        show_ranges: options.show_ranges,
        range_limit: options.range_limit,
        range_offset: options.range_offset,
    };
    let report = build_subnet_catalog_list_report(&request)?;
    write_text_or_json(format, &report, |report| {
        if verbose {
            subnet_catalog_list_report_verbose_text(report)
        } else {
            subnet_catalog_list_report_text(report)
        }
    })
}

fn run_catalog_info<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, info_usage, version_text()) {
        return Ok(());
    }
    let options = CatalogInfoOptions::parse(args)?;
    let format = options.format;
    let icp_root = icp_root().map_err(|err| NnsCommandError::Usage(err.to_string()))?;
    let request = SubnetCatalogInfoRequest {
        cache: cache_request(&icp_root, &options.network),
        input: options.input.clone(),
        forced: options.forced,
        resolved_target: None,
        now_unix_secs: now_unix_secs()?,
        stale_after_seconds: DEFAULT_STALE_AFTER_SECONDS,
    };
    let report = match build_subnet_catalog_info_report(&request) {
        Ok(report) => report,
        Err(err) if should_retry_info_as_deployment_target(&err, &options) => {
            let mut retry_request = request;
            retry_request.resolved_target = Some(resolve_deployment_target(
                &options.input,
                &options,
                &icp_root,
            )?);
            build_subnet_catalog_info_report(&retry_request)?
        }
        Err(err) => return Err(NnsCommandError::from(err)),
    };
    write_text_or_json(format, &report, subnet_catalog_info_report_text)
}

fn run_catalog_refresh<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, refresh_usage, version_text()) {
        return Ok(());
    }
    let options = CatalogRefreshOptions::parse(args)?;
    let format = options.format;
    let icp_root = icp_root().map_err(|err| NnsCommandError::Usage(err.to_string()))?;
    let request = SubnetCatalogRefreshRequest {
        cache: cache_request(&icp_root, &options.network),
        source_endpoint: options.source_endpoint,
        now_unix_secs: now_unix_secs()?,
        lock_stale_after_seconds: options.lock_stale_after_seconds,
        dry_run: options.dry_run,
        output_path: options.output_path,
    };
    let report = refresh_subnet_catalog(&request)?;
    write_text_or_json(format, &report, subnet_catalog_refresh_report_text)
}

fn write_text_or_json<T>(
    format: OutputFormat,
    report: &T,
    render_text: impl FnOnce(&T) -> String,
) -> Result<(), NnsCommandError>
where
    T: Serialize,
{
    match format {
        OutputFormat::Text => {
            let text = render_text(report);
            write_text::<NnsCommandError>(None, &text)
        }
        OutputFormat::Json => write_pretty_json(None, report),
    }
}

impl CatalogListOptions {
    fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(list_command(), args)
            .map_err(|_| NnsCommandError::Usage(list_usage()))?;
        let range_limit = typed_option(&matches, "range-limit").unwrap_or(DEFAULT_RANGE_LIMIT);
        let range_offset = typed_option(&matches, "range-offset").unwrap_or(0);
        Ok(Self {
            network: string_option(&matches, "network")
                .unwrap_or_else(|| MAINNET_NETWORK.to_string()),
            format: typed_option(&matches, "format").unwrap_or(OutputFormat::Text),
            filters: SubnetCatalogFilters {
                kind: typed_option(&matches, "kind"),
                specialization: typed_option(&matches, "specialization"),
                geographic_scope: typed_option(&matches, "geo"),
            },
            show_ranges: matches.get_flag("show-ranges"),
            verbose: matches.get_flag("verbose"),
            range_limit,
            range_offset,
        })
    }
}

impl CatalogInfoOptions {
    fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(info_command(), args)
            .map_err(|_| NnsCommandError::Usage(info_usage()))?;
        Ok(Self {
            input: required_string(&matches, "input"),
            network: string_option(&matches, "network")
                .unwrap_or_else(|| MAINNET_NETWORK.to_string()),
            icp: string_option_or_else(&matches, "icp", default_icp),
            format: typed_option(&matches, "format").unwrap_or(OutputFormat::Text),
            forced: typed_option(&matches, "as"),
        })
    }
}

impl CatalogRefreshOptions {
    fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(refresh_command(), args)
            .map_err(|_| NnsCommandError::Usage(refresh_usage()))?;
        Ok(Self {
            network: string_option(&matches, "network")
                .unwrap_or_else(|| MAINNET_NETWORK.to_string()),
            format: typed_option(&matches, "format").unwrap_or(OutputFormat::Text),
            source_endpoint: string_option(&matches, "source-endpoint")
                .unwrap_or_else(|| DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT.to_string()),
            lock_stale_after_seconds: typed_option(&matches, "lock-stale-after")
                .unwrap_or(DEFAULT_REFRESH_LOCK_STALE_SECONDS),
            dry_run: matches.get_flag("dry-run"),
            output_path: path_option(&matches, "output"),
        })
    }
}

impl NodeProviderListOptions {
    fn parse<I>(args: I) -> Result<Self, NnsCommandError>
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
        })
    }
}

fn should_retry_info_as_deployment_target(
    err: &SubnetCatalogHostError,
    options: &CatalogInfoOptions,
) -> bool {
    if options.forced.is_some() || canonical_principal_text(&options.input).is_ok() {
        return false;
    }
    matches!(
        err,
        SubnetCatalogHostError::Catalog(CatalogError::PrincipalPrefixNotFound { .. })
    )
}

fn resolve_deployment_target(
    input: &str,
    options: &CatalogInfoOptions,
    icp_root: &Path,
) -> Result<ResolvedDeploymentTarget, NnsCommandError> {
    if let Some((deployment, target)) = split_deployment_selector(input) {
        let resolution = resolve_installed_deployment_from_root(
            &InstalledDeploymentRequest {
                deployment: deployment.to_string(),
                network: options.network.clone(),
                icp: options.icp.clone(),
                detect_lost_local_root: false,
            },
            icp_root,
        )
        .map_err(|err| target_resolution_error(input, &options.network, err))?;
        let canister_principal = resolve_canister_or_role(&resolution, deployment, target)
            .map_err(|reason| NnsCommandError::TargetResolutionFailed {
                input: input.to_string(),
                network: options.network.clone(),
                reason,
            })?;
        return Ok(ResolvedDeploymentTarget {
            canister_principal,
            resolved_from: format!("deployment_target:{deployment}/{target}"),
        });
    }

    let state = read_installed_deployment_state_from_root(&options.network, input, icp_root)
        .map_err(|err| target_resolution_error(input, &options.network, err))?;
    Ok(ResolvedDeploymentTarget {
        canister_principal: state.root_canister_id,
        resolved_from: format!("deployment_target:{input}"),
    })
}

fn split_deployment_selector(input: &str) -> Option<(&str, &str)> {
    let (deployment, target) = input.split_once('/')?;
    if deployment.is_empty() || target.is_empty() || target.contains('/') {
        return None;
    }
    Some((deployment, target))
}

fn resolve_canister_or_role(
    resolution: &InstalledDeploymentResolution,
    deployment: &str,
    target: &str,
) -> Result<String, String> {
    if target == "root" || target == resolution.registry.root_canister_id {
        return Ok(resolution.registry.root_canister_id.clone());
    }
    if resolution
        .registry
        .entries
        .iter()
        .any(|entry| entry.pid == target)
    {
        return Ok(target.to_string());
    }

    let matches = resolution
        .registry
        .entries
        .iter()
        .filter(|entry| entry.role.as_deref() == Some(target))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [entry] => Ok(entry.pid.clone()),
        [] => Err(format!(
            "deployment target {deployment} has no role or canister principal {target}"
        )),
        _ => Err(format!(
            "role {target} is ambiguous in deployment target {deployment}; use one canister principal"
        )),
    }
}

fn target_resolution_error(
    input: &str,
    network: &str,
    err: InstalledDeploymentError,
) -> NnsCommandError {
    NnsCommandError::TargetResolutionFailed {
        input: input.to_string(),
        network: network.to_string(),
        reason: err.to_string(),
    }
}

fn parse_format(value: &str) -> Result<OutputFormat, String> {
    match value {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        other => Err(format!("invalid value {other}; use text or json")),
    }
}

fn parse_kind(value: &str) -> Result<SubnetKind, String> {
    match value {
        "application" => Ok(SubnetKind::Application),
        "system" => Ok(SubnetKind::System),
        "unknown" => Ok(SubnetKind::Unknown),
        other => Err(format!(
            "invalid value {other}; use application, system, or unknown"
        )),
    }
}

fn parse_specialization(value: &str) -> Result<SubnetSpecialization, String> {
    match value {
        "none" => Ok(SubnetSpecialization::None),
        "fiduciary" => Ok(SubnetSpecialization::Fiduciary),
        "european" => Ok(SubnetSpecialization::European),
        "unknown" => Ok(SubnetSpecialization::Unknown),
        other => Err(format!(
            "invalid value {other}; use none, fiduciary, european, or unknown"
        )),
    }
}

fn parse_geo(value: &str) -> Result<GeographicScope, String> {
    match value {
        "global" => Ok(GeographicScope::Global),
        "europe" => Ok(GeographicScope::Europe),
        "unknown" => Ok(GeographicScope::Unknown),
        other => Err(format!(
            "invalid value {other}; use global, europe, or unknown"
        )),
    }
}

fn parse_resolve_as(value: &str) -> Result<ResolveAs, String> {
    match value {
        "subnet" => Ok(ResolveAs::Subnet),
        "canister" => Ok(ResolveAs::Canister),
        other => Err(format!("invalid value {other}; use subnet or canister")),
    }
}

fn parse_refresh_lock_stale_after(value: &str) -> Result<u64, String> {
    parse_stale_after_duration(value).map_err(|err| err.to_string())
}

fn cache_request(icp_root: &Path, network: &str) -> SubnetCatalogCacheRequest {
    SubnetCatalogCacheRequest {
        icp_root: PathBuf::from(icp_root),
        network: network.to_string(),
    }
}

fn now_unix_secs() -> Result<u64, NnsCommandError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|err| NnsCommandError::Clock(err.to_string()))
}

fn nns_command() -> ClapCommand {
    ClapCommand::new("nns")
        .bin_name("canic nns")
        .about("Inspect NNS metadata")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("subnet").about("Inspect and refresh NNS subnet metadata"),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("node-provider").about("Inspect NNS node-provider metadata"),
        ))
}

fn subnet_command() -> ClapCommand {
    ClapCommand::new("subnet")
        .bin_name("canic nns subnet")
        .about("Inspect and refresh NNS subnet metadata")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list").about("List cached mainnet IC subnets"),
        ))
        .subcommand(passthrough_subcommand(ClapCommand::new("info").about(
            "Resolve a subnet, canister, or deployment target to cached subnet info",
        )))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("refresh").about("Force-refresh and cache NNS subnet metadata"),
        ))
}

fn node_provider_command() -> ClapCommand {
    ClapCommand::new("node-provider")
        .bin_name("canic nns node-provider")
        .about("Inspect NNS node-provider metadata")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list").about("List mainnet NNS node providers"),
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
        .arg(internal_network_arg())
        .after_help(NODE_PROVIDER_LIST_HELP_AFTER)
}

fn list_command() -> ClapCommand {
    ClapCommand::new("list")
        .bin_name("canic nns subnet list")
        .about("List cached mainnet IC subnets")
        .disable_help_flag(true)
        .arg(
            value_arg("kind")
                .long("kind")
                .value_name("kind")
                .value_parser(clap::builder::ValueParser::new(parse_kind))
                .help("Filter by subnet kind: application, system, or unknown"),
        )
        .arg(
            value_arg("specialization")
                .long("specialization")
                .value_name("specialization")
                .value_parser(clap::builder::ValueParser::new(parse_specialization))
                .help("Filter by specialization: none, fiduciary, european, or unknown"),
        )
        .arg(
            value_arg("geo")
                .long("geo")
                .value_name("scope")
                .value_parser(clap::builder::ValueParser::new(parse_geo))
                .help("Filter by geographic scope: global, europe, or unknown"),
        )
        .arg(
            value_arg("format")
                .long("format")
                .value_name("text|json")
                .value_parser(clap::builder::ValueParser::new(parse_format))
                .help("Output format; defaults to text"),
        )
        .arg(
            flag_arg("show-ranges")
                .long("show-ranges")
                .help("Show cached routing ranges after the subnet table"),
        )
        .arg(
            flag_arg("verbose")
                .long("verbose")
                .help("Show full subnet principals and catalog metadata in text output"),
        )
        .arg(
            value_arg("range-limit")
                .long("range-limit")
                .value_name("n")
                .value_parser(clap::builder::ValueParser::new(parse_positive_usize))
                .help("Maximum routing ranges to show per subnet in text output"),
        )
        .arg(
            value_arg("range-offset")
                .long("range-offset")
                .value_name("n")
                .value_parser(clap::builder::ValueParser::new(parse_usize))
                .help("Routing range offset for text output"),
        )
        .arg(internal_network_arg())
        .after_help(LIST_HELP_AFTER)
}

fn info_command() -> ClapCommand {
    ClapCommand::new("info")
        .bin_name("canic nns subnet info")
        .about("Resolve a subnet, canister, or deployment target to cached subnet info")
        .disable_help_flag(true)
        .arg(
            value_arg("input")
                .value_name(INFO_INPUT_VALUE_NAME)
                .required(true)
                .help(INFO_INPUT_HELP),
        )
        .arg(
            value_arg("as")
                .long("as")
                .value_name("subnet|canister")
                .value_parser(clap::builder::ValueParser::new(parse_resolve_as))
                .help("Force principal interpretation"),
        )
        .arg(
            value_arg("format")
                .long("format")
                .value_name("text|json")
                .value_parser(clap::builder::ValueParser::new(parse_format))
                .help("Output format; defaults to text"),
        )
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
        .after_help(INFO_HELP_AFTER)
}

fn refresh_command() -> ClapCommand {
    ClapCommand::new("refresh")
        .bin_name("canic nns subnet refresh")
        .about("Force-refresh and cache NNS subnet metadata")
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
                .help("IC API endpoint used for the NNS registry query"),
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
                .help("Fetch and validate without replacing the cached catalog"),
        )
        .arg(
            value_arg("output")
                .long("output")
                .value_name("path")
                .help("Also write the fetched catalog JSON to this path"),
        )
        .arg(internal_network_arg())
        .after_help(REFRESH_HELP_AFTER)
}

fn usage() -> String {
    render_help(nns_command())
}

fn subnet_usage() -> String {
    render_help(subnet_command())
}

fn node_provider_usage() -> String {
    render_help(node_provider_command())
}

fn node_provider_list_usage() -> String {
    render_help(node_provider_list_command())
}

fn list_usage() -> String {
    render_help(list_command())
}

fn info_usage() -> String {
    render_help(info_command())
}

fn refresh_usage() -> String {
    render_help(refresh_command())
}
