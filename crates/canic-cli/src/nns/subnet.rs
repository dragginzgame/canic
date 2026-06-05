use super::{NnsCommandError, OutputFormat, leaf, now_unix_secs, write_text_or_json};
use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, parse_positive_usize, parse_required_subcommand, parse_usize,
            passthrough_subcommand, render_help, required_string, required_typed,
            string_option_or_else, typed_option, value_arg,
        },
        defaults::default_icp,
        globals::internal_icp_arg,
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::{
    installed_deployment::{
        InstalledDeploymentError, InstalledDeploymentRequest, InstalledDeploymentResolution,
        read_installed_deployment_state_from_root, resolve_installed_deployment_from_root,
    },
    release_set::icp_root,
    subnet_catalog::{
        DEFAULT_STALE_AFTER_SECONDS, DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT,
        ResolvedDeploymentTarget, SubnetCatalogCacheRequest, SubnetCatalogFilters,
        SubnetCatalogHostError, SubnetCatalogInfoRequest, SubnetCatalogListRequest,
        SubnetCatalogRefreshRequest, build_subnet_catalog_info_report,
        build_subnet_catalog_list_report, refresh_subnet_catalog, subnet_catalog_info_report_text,
        subnet_catalog_list_report_text, subnet_catalog_list_report_verbose_text,
        subnet_catalog_refresh_report_text,
    },
};
use canic_subnet_catalog::{
    CatalogError, GeographicScope, ResolveAs, SubnetKind, SubnetSpecialization,
    canonical_principal_text,
};
use clap::Command as ClapCommand;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

#[cfg(test)]
pub(super) const DEFAULT_RANGE_LIMIT: usize = 50;
const DEFAULT_RANGE_LIMIT_ARG: &str = "50";
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

///
/// CatalogListOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CatalogListOptions {
    pub(super) network: String,
    pub(super) format: OutputFormat,
    pub(super) filters: SubnetCatalogFilters,
    pub(super) show_ranges: bool,
    pub(super) verbose: bool,
    pub(super) range_limit: usize,
    pub(super) range_offset: usize,
}

///
/// CatalogInfoOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CatalogInfoOptions {
    pub(super) input: String,
    pub(super) network: String,
    pub(super) icp: String,
    pub(super) format: OutputFormat,
    pub(super) forced: Option<ResolveAs>,
}

///
/// CatalogRefreshOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CatalogRefreshOptions {
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
    if print_help_or_version(&args, subnet_usage, version_text()) {
        return Ok(());
    }
    let (command, args) = parse_required_subcommand(subnet_command(), args)
        .map_err(|_| NnsCommandError::Usage(subnet_usage()))?;

    match command.as_str() {
        "list" => run_catalog_list(args),
        "info" => run_catalog_info(args),
        "refresh" => run_catalog_refresh(args),
        _ => unreachable!("nns subnet dispatch command only defines known commands"),
    }
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

impl CatalogListOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(list_command(), args)
            .map_err(|_| NnsCommandError::Usage(list_usage()))?;
        Ok(Self {
            network: required_string(&matches, "network"),
            format: required_typed(&matches, "format"),
            filters: SubnetCatalogFilters {
                kind: typed_option(&matches, "kind"),
                specialization: typed_option(&matches, "specialization"),
                geographic_scope: typed_option(&matches, "geo"),
            },
            show_ranges: matches.get_flag("show-ranges"),
            verbose: matches.get_flag("verbose"),
            range_limit: required_typed(&matches, "range-limit"),
            range_offset: required_typed(&matches, "range-offset"),
        })
    }
}

impl CatalogInfoOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(info_command(), args)
            .map_err(|_| NnsCommandError::Usage(info_usage()))?;
        Ok(Self {
            input: required_string(&matches, "input"),
            network: required_string(&matches, "network"),
            icp: string_option_or_else(&matches, "icp", default_icp),
            format: required_typed(&matches, "format"),
            forced: typed_option(&matches, "as"),
        })
    }
}

impl CatalogRefreshOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(refresh_command(), args)
            .map_err(|_| NnsCommandError::Usage(refresh_usage()))?;
        Ok(Self {
            network: required_string(&matches, "network"),
            format: required_typed(&matches, "format"),
            source_endpoint: required_string(&matches, "source-endpoint"),
            lock_stale_after_seconds: required_typed(&matches, "lock-stale-after"),
            dry_run: matches.get_flag("dry-run"),
            output_path: typed_option(&matches, "output"),
        })
    }
}

pub(super) fn should_retry_info_as_deployment_target(
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

pub(super) fn resolve_canister_or_role(
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

fn cache_request(icp_root: &Path, network: &str) -> SubnetCatalogCacheRequest {
    SubnetCatalogCacheRequest {
        icp_root: PathBuf::from(icp_root),
        network: network.to_string(),
    }
}

pub(super) fn subnet_command() -> ClapCommand {
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
        .arg(leaf::format_arg())
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
                .default_value(DEFAULT_RANGE_LIMIT_ARG)
                .value_parser(clap::builder::ValueParser::new(parse_positive_usize))
                .help("Maximum routing ranges to show per subnet in text output"),
        )
        .arg(
            value_arg("range-offset")
                .long("range-offset")
                .value_name("n")
                .default_value("0")
                .value_parser(clap::builder::ValueParser::new(parse_usize))
                .help("Routing range offset for text output"),
        )
        .arg(leaf::network_arg())
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
        .arg(leaf::format_arg())
        .arg(leaf::network_arg())
        .arg(internal_icp_arg())
        .after_help(INFO_HELP_AFTER)
}

fn refresh_command() -> ClapCommand {
    ClapCommand::new("refresh")
        .bin_name("canic nns subnet refresh")
        .about("Force-refresh and cache NNS subnet metadata")
        .disable_help_flag(true)
        .arg(leaf::format_arg())
        .arg(
            leaf::source_endpoint_arg(DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT)
                .help("IC API endpoint used for the NNS registry query"),
        )
        .arg(leaf::refresh_lock_stale_after_arg())
        .arg(
            flag_arg("dry-run")
                .long("dry-run")
                .help("Fetch and validate without replacing the cached catalog"),
        )
        .arg(leaf::output_path_arg().help("Also write the fetched catalog JSON to this path"))
        .arg(leaf::network_arg())
        .after_help(REFRESH_HELP_AFTER)
}

pub(super) fn subnet_usage() -> String {
    render_help(subnet_command())
}

pub(super) fn list_usage() -> String {
    render_help(list_command())
}

pub(super) fn info_usage() -> String {
    render_help(info_command())
}

pub(super) fn refresh_usage() -> String {
    render_help(refresh_command())
}
