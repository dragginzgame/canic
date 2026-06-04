use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, parse_subcommand, passthrough_subcommand, path_option,
            string_option, value_arg,
        },
        defaults::default_icp,
        globals::{internal_icp_arg, internal_network_arg},
        help::print_help_or_version,
    },
    output::write_pretty_json,
    version_text,
};
#[cfg(test)]
use canic_host::registry::RegistryEntry;
use canic_host::{
    installed_deployment::{
        InstalledDeploymentError, InstalledDeploymentRequest, InstalledDeploymentResolution,
        read_installed_deployment_state_from_root, resolve_installed_deployment_from_root,
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
    GeographicScope, MAINNET_NETWORK, ResolveAs, SubnetKind, SubnetSpecialization,
    canonical_principal_text,
};
use clap::Command as ClapCommand;
use std::{
    ffi::OsString,
    io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

const DEFAULT_RANGE_LIMIT: usize = 50;
const LIST_HELP_AFTER: &str = "\
Examples:
  canic subnet catalog list
  canic subnet catalog list --verbose
  canic --network ic subnet catalog list --format json
  canic subnet catalog list --kind application --specialization fiduciary";
const INFO_HELP_AFTER: &str = "\
Examples:
  canic subnet catalog info ryjl3-tyaaa-aaaaa-aaaba-cai
  canic subnet catalog info <deployment>
  canic subnet catalog info <deployment>/<role-or-canister>";
const REFRESH_HELP_AFTER: &str = "\
Examples:
  canic subnet catalog refresh
  canic --network ic subnet catalog refresh --format json
  canic subnet catalog refresh --dry-run --output .canic/subnet-catalog/ic/catalog.preview.json";

///
/// SubnetCommandError
///
#[derive(Debug, ThisError)]
pub enum SubnetCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Host(#[from] SubnetCatalogHostError),

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
    stale_after_seconds: u64,
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
    stale_after_seconds: u64,
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

pub fn run<I>(args: I) -> Result<(), SubnetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }
    let Some((command, args)) =
        parse_subcommand(subnet_command(), args).map_err(|_| SubnetCommandError::Usage(usage()))?
    else {
        return Err(SubnetCommandError::Usage(usage()));
    };

    match command.as_str() {
        "catalog" => run_catalog(args),
        _ => unreachable!("subnet dispatch command only defines known commands"),
    }
}

fn run_catalog<I>(args: I) -> Result<(), SubnetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, catalog_usage, version_text()) {
        return Ok(());
    }
    let Some((command, args)) = parse_subcommand(catalog_command(), args)
        .map_err(|_| SubnetCommandError::Usage(catalog_usage()))?
    else {
        return Err(SubnetCommandError::Usage(catalog_usage()));
    };

    match command.as_str() {
        "list" => run_catalog_list(args),
        "info" => run_catalog_info(args),
        "refresh" => run_catalog_refresh(args),
        _ => unreachable!("subnet catalog dispatch command only defines known commands"),
    }
}

fn run_catalog_list<I>(args: I) -> Result<(), SubnetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, list_usage, version_text()) {
        return Ok(());
    }
    let options = CatalogListOptions::parse(args)?;
    let icp_root = icp_root().map_err(|err| SubnetCommandError::Usage(err.to_string()))?;
    let request = SubnetCatalogListRequest {
        cache: cache_request(&icp_root, &options.network),
        now_unix_secs: now_unix_secs()?,
        stale_after_seconds: options.stale_after_seconds,
        filters: options.filters,
        show_ranges: options.show_ranges,
        range_limit: options.range_limit,
        range_offset: options.range_offset,
    };
    let report = build_subnet_catalog_list_report(&request)?;
    match options.format {
        OutputFormat::Text => {
            let text = if options.verbose {
                subnet_catalog_list_report_verbose_text(&report)
            } else {
                subnet_catalog_list_report_text(&report)
            };
            println!("{text}");
            Ok(())
        }
        OutputFormat::Json => write_pretty_json(None, &report),
    }
}

fn run_catalog_info<I>(args: I) -> Result<(), SubnetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, info_usage, version_text()) {
        return Ok(());
    }
    let options = CatalogInfoOptions::parse(args)?;
    let icp_root = icp_root().map_err(|err| SubnetCommandError::Usage(err.to_string()))?;
    let resolved_target = resolved_target_for_info(&options, &icp_root)?;
    let request = SubnetCatalogInfoRequest {
        cache: cache_request(&icp_root, &options.network),
        input: options.input,
        forced: options.forced,
        resolved_target,
        now_unix_secs: now_unix_secs()?,
        stale_after_seconds: options.stale_after_seconds,
    };
    let report = build_subnet_catalog_info_report(&request)?;
    match options.format {
        OutputFormat::Text => {
            println!("{}", subnet_catalog_info_report_text(&report));
            Ok(())
        }
        OutputFormat::Json => write_pretty_json(None, &report),
    }
}

fn run_catalog_refresh<I>(args: I) -> Result<(), SubnetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, refresh_usage, version_text()) {
        return Ok(());
    }
    let options = CatalogRefreshOptions::parse(args)?;
    let icp_root = icp_root().map_err(|err| SubnetCommandError::Usage(err.to_string()))?;
    let request = SubnetCatalogRefreshRequest {
        cache: cache_request(&icp_root, &options.network),
        source_endpoint: options.source_endpoint,
        now_unix_secs: now_unix_secs()?,
        lock_stale_after_seconds: options.lock_stale_after_seconds,
        dry_run: options.dry_run,
        output_path: options.output_path,
    };
    let report = refresh_subnet_catalog(&request)?;
    match options.format {
        OutputFormat::Text => {
            println!("{}", subnet_catalog_refresh_report_text(&report));
            Ok(())
        }
        OutputFormat::Json => write_pretty_json(None, &report),
    }
}

impl CatalogListOptions {
    fn parse<I>(args: I) -> Result<Self, SubnetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(list_command(), args)
            .map_err(|_| SubnetCommandError::Usage(list_usage()))?;
        let range_limit = parse_usize_option(&matches, "range-limit", DEFAULT_RANGE_LIMIT)?;
        let range_offset = parse_usize_option(&matches, "range-offset", 0)?;
        if range_limit == 0 {
            return Err(SubnetCommandError::Usage(
                "--range-limit must be greater than zero".to_string(),
            ));
        }
        Ok(Self {
            network: string_option(&matches, "network")
                .unwrap_or_else(|| MAINNET_NETWORK.to_string()),
            format: parse_format(string_option(&matches, "format").as_deref())?,
            filters: SubnetCatalogFilters {
                kind: string_option(&matches, "kind")
                    .map(|value| parse_kind(&value))
                    .transpose()?,
                specialization: string_option(&matches, "specialization")
                    .map(|value| parse_specialization(&value))
                    .transpose()?,
                geographic_scope: string_option(&matches, "geo")
                    .map(|value| parse_geo(&value))
                    .transpose()?,
            },
            show_ranges: matches.get_flag("show-ranges"),
            verbose: matches.get_flag("verbose"),
            range_limit,
            range_offset,
            stale_after_seconds: parse_stale_after(string_option(&matches, "stale-after"))?,
        })
    }
}

impl CatalogInfoOptions {
    fn parse<I>(args: I) -> Result<Self, SubnetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(info_command(), args)
            .map_err(|_| SubnetCommandError::Usage(info_usage()))?;
        Ok(Self {
            input: string_option(&matches, "input").expect("clap requires input"),
            network: string_option(&matches, "network")
                .unwrap_or_else(|| MAINNET_NETWORK.to_string()),
            icp: string_option(&matches, "icp").unwrap_or_else(default_icp),
            format: parse_format(string_option(&matches, "format").as_deref())?,
            forced: string_option(&matches, "as")
                .map(|value| parse_resolve_as(&value))
                .transpose()?,
            stale_after_seconds: parse_stale_after(string_option(&matches, "stale-after"))?,
        })
    }
}

impl CatalogRefreshOptions {
    fn parse<I>(args: I) -> Result<Self, SubnetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(refresh_command(), args)
            .map_err(|_| SubnetCommandError::Usage(refresh_usage()))?;
        Ok(Self {
            network: string_option(&matches, "network")
                .unwrap_or_else(|| MAINNET_NETWORK.to_string()),
            format: parse_format(string_option(&matches, "format").as_deref())?,
            source_endpoint: string_option(&matches, "source-endpoint")
                .unwrap_or_else(|| DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT.to_string()),
            lock_stale_after_seconds: parse_refresh_lock_stale_after(string_option(
                &matches,
                "lock-stale-after",
            ))?,
            dry_run: matches.get_flag("dry-run"),
            output_path: path_option(&matches, "output"),
        })
    }
}

fn resolved_target_for_info(
    options: &CatalogInfoOptions,
    icp_root: &Path,
) -> Result<Option<ResolvedDeploymentTarget>, SubnetCommandError> {
    if canonical_principal_text(&options.input).is_ok() {
        return Ok(None);
    }
    if options.forced == Some(ResolveAs::Subnet) {
        return Err(SubnetCommandError::Usage(
            "--as subnet requires a subnet principal".to_string(),
        ));
    }
    resolve_deployment_target(&options.input, options, icp_root).map(Some)
}

fn resolve_deployment_target(
    input: &str,
    options: &CatalogInfoOptions,
    icp_root: &Path,
) -> Result<ResolvedDeploymentTarget, SubnetCommandError> {
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
            .map_err(|reason| SubnetCommandError::TargetResolutionFailed {
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
) -> SubnetCommandError {
    SubnetCommandError::TargetResolutionFailed {
        input: input.to_string(),
        network: network.to_string(),
        reason: err.to_string(),
    }
}

fn parse_format(value: Option<&str>) -> Result<OutputFormat, SubnetCommandError> {
    match value.unwrap_or("text") {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        other => Err(SubnetCommandError::Usage(format!(
            "invalid --format {other}; use text or json"
        ))),
    }
}

fn parse_kind(value: &str) -> Result<SubnetKind, SubnetCommandError> {
    match value {
        "application" => Ok(SubnetKind::Application),
        "system" => Ok(SubnetKind::System),
        "unknown" => Ok(SubnetKind::Unknown),
        other => Err(SubnetCommandError::Usage(format!(
            "invalid --kind {other}; use application, system, or unknown"
        ))),
    }
}

fn parse_specialization(value: &str) -> Result<SubnetSpecialization, SubnetCommandError> {
    match value {
        "none" => Ok(SubnetSpecialization::None),
        "fiduciary" => Ok(SubnetSpecialization::Fiduciary),
        "european" => Ok(SubnetSpecialization::European),
        "unknown" => Ok(SubnetSpecialization::Unknown),
        other => Err(SubnetCommandError::Usage(format!(
            "invalid --specialization {other}; use none, fiduciary, european, or unknown"
        ))),
    }
}

fn parse_geo(value: &str) -> Result<GeographicScope, SubnetCommandError> {
    match value {
        "global" => Ok(GeographicScope::Global),
        "europe" => Ok(GeographicScope::Europe),
        "unknown" => Ok(GeographicScope::Unknown),
        other => Err(SubnetCommandError::Usage(format!(
            "invalid --geo {other}; use global, europe, or unknown"
        ))),
    }
}

fn parse_resolve_as(value: &str) -> Result<ResolveAs, SubnetCommandError> {
    match value {
        "subnet" => Ok(ResolveAs::Subnet),
        "canister" => Ok(ResolveAs::Canister),
        other => Err(SubnetCommandError::Usage(format!(
            "invalid --as {other}; use subnet or canister"
        ))),
    }
}

fn parse_stale_after(value: Option<String>) -> Result<u64, SubnetCommandError> {
    value.map_or(Ok(DEFAULT_STALE_AFTER_SECONDS), |value| {
        parse_stale_after_duration(&value).map_err(SubnetCommandError::from)
    })
}

fn parse_refresh_lock_stale_after(value: Option<String>) -> Result<u64, SubnetCommandError> {
    value.map_or(Ok(DEFAULT_REFRESH_LOCK_STALE_SECONDS), |value| {
        parse_stale_after_duration(&value).map_err(SubnetCommandError::from)
    })
}

fn parse_usize_option(
    matches: &clap::ArgMatches,
    id: &str,
    default: usize,
) -> Result<usize, SubnetCommandError> {
    string_option(matches, id).map_or(Ok(default), |value| {
        value.parse::<usize>().map_err(|_| {
            SubnetCommandError::Usage(format!("--{id} must be a non-negative integer"))
        })
    })
}

fn cache_request(icp_root: &Path, network: &str) -> SubnetCatalogCacheRequest {
    SubnetCatalogCacheRequest {
        icp_root: PathBuf::from(icp_root),
        network: network.to_string(),
    }
}

fn now_unix_secs() -> Result<u64, SubnetCommandError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|err| SubnetCommandError::Clock(err.to_string()))
}

fn subnet_command() -> ClapCommand {
    ClapCommand::new("subnet")
        .bin_name("canic subnet")
        .about("Inspect and refresh IC subnet catalog metadata")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("catalog").about("Inspect and refresh IC subnet catalog metadata"),
        ))
}

fn catalog_command() -> ClapCommand {
    ClapCommand::new("catalog")
        .bin_name("canic subnet catalog")
        .about("Inspect and refresh IC subnet catalog metadata")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list").about("List cached mainnet IC subnets"),
        ))
        .subcommand(passthrough_subcommand(ClapCommand::new("info").about(
            "Resolve a subnet, canister, or deployment target to cached subnet info",
        )))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("refresh").about("Fetch and cache the mainnet IC subnet catalog"),
        ))
}

fn list_command() -> ClapCommand {
    ClapCommand::new("list")
        .bin_name("canic subnet catalog list")
        .about("List cached mainnet IC subnets")
        .disable_help_flag(true)
        .arg(
            value_arg("kind")
                .long("kind")
                .value_name("kind")
                .help("Filter by subnet kind: application, system, or unknown"),
        )
        .arg(
            value_arg("specialization")
                .long("specialization")
                .value_name("specialization")
                .help("Filter by specialization: none, fiduciary, european, or unknown"),
        )
        .arg(
            value_arg("geo")
                .long("geo")
                .value_name("scope")
                .help("Filter by geographic scope: global, europe, or unknown"),
        )
        .arg(
            value_arg("format")
                .long("format")
                .value_name("text|json")
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
                .help("Maximum routing ranges to show per subnet in text output"),
        )
        .arg(
            value_arg("range-offset")
                .long("range-offset")
                .value_name("n")
                .help("Routing range offset for text output"),
        )
        .arg(
            value_arg("stale-after")
                .long("stale-after")
                .value_name("duration")
                .help("Mark the cached catalog stale after this duration; defaults to 7d"),
        )
        .arg(
            flag_arg("allow-stale-subnet-catalog")
                .long("allow-stale-subnet-catalog")
                .help("Allow stale cached catalog output; list output always records stale status"),
        )
        .arg(internal_network_arg())
        .after_help(LIST_HELP_AFTER)
}

fn info_command() -> ClapCommand {
    ClapCommand::new("info")
        .bin_name("canic subnet catalog info")
        .about("Resolve a subnet, canister, or deployment target to cached subnet info")
        .disable_help_flag(true)
        .arg(
            value_arg("input")
                .value_name("subnet-principal|canister-principal|deployment-target")
                .required(true)
                .help("Subnet principal, canister principal, deployment target, or <deployment>/<role-or-canister>"),
        )
        .arg(
            value_arg("as")
                .long("as")
                .value_name("subnet|canister")
                .help("Force principal interpretation"),
        )
        .arg(
            value_arg("format")
                .long("format")
                .value_name("text|json")
                .help("Output format; defaults to text"),
        )
        .arg(
            value_arg("stale-after")
                .long("stale-after")
                .value_name("duration")
                .help("Mark the cached catalog stale after this duration; defaults to 7d"),
        )
        .arg(
            flag_arg("allow-stale-subnet-catalog")
                .long("allow-stale-subnet-catalog")
                .help("Allow stale cached catalog output; info output always records stale status"),
        )
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
        .after_help(INFO_HELP_AFTER)
}

fn refresh_command() -> ClapCommand {
    ClapCommand::new("refresh")
        .bin_name("canic subnet catalog refresh")
        .about("Fetch and cache the mainnet IC subnet catalog")
        .disable_help_flag(true)
        .arg(
            value_arg("format")
                .long("format")
                .value_name("text|json")
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
    let mut command = subnet_command();
    command.render_help().to_string()
}

fn catalog_usage() -> String {
    let mut command = catalog_command();
    command.render_help().to_string()
}

fn list_usage() -> String {
    let mut command = list_command();
    command.render_help().to_string()
}

fn info_usage() -> String {
    let mut command = info_command();
    command.render_help().to_string()
}

fn refresh_usage() -> String {
    let mut command = refresh_command();
    command.render_help().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn list_defaults_to_mainnet_ic_catalog() {
        let options = CatalogListOptions::parse([]).expect("parse list");

        assert_eq!(options.network, MAINNET_NETWORK);
        assert_eq!(options.format, OutputFormat::Text);
        assert_eq!(options.range_limit, DEFAULT_RANGE_LIMIT);
        assert!(!options.verbose);
    }

    #[test]
    fn list_parses_filters_and_json_format() {
        let options = CatalogListOptions::parse([
            OsString::from("--kind"),
            OsString::from("application"),
            OsString::from("--specialization"),
            OsString::from("fiduciary"),
            OsString::from("--geo"),
            OsString::from("global"),
            OsString::from("--format"),
            OsString::from("json"),
            OsString::from("--show-ranges"),
            OsString::from("--verbose"),
            OsString::from("--range-limit"),
            OsString::from("12"),
        ])
        .expect("parse list");

        assert_eq!(options.filters.kind, Some(SubnetKind::Application));
        assert_eq!(
            options.filters.specialization,
            Some(SubnetSpecialization::Fiduciary)
        );
        assert_eq!(
            options.filters.geographic_scope,
            Some(GeographicScope::Global)
        );
        assert_eq!(options.format, OutputFormat::Json);
        assert!(options.show_ranges);
        assert!(options.verbose);
        assert_eq!(options.range_limit, 12);
    }

    #[test]
    fn info_usage_names_deployment_target_input() {
        let text = info_usage();

        assert!(text.contains("subnet-principal|canister-principal|deployment-target"));
        assert!(text.contains("--as <subnet|canister>"));
    }

    #[test]
    fn refresh_parses_defaults_and_export_options() {
        let options = CatalogRefreshOptions::parse([
            OsString::from("--format"),
            OsString::from("json"),
            OsString::from("--source-endpoint"),
            OsString::from("https://icp-api.io"),
            OsString::from("--lock-stale-after"),
            OsString::from("5m"),
            OsString::from("--dry-run"),
            OsString::from("--output"),
            OsString::from("catalog.preview.json"),
        ])
        .expect("parse refresh");

        assert_eq!(options.network, MAINNET_NETWORK);
        assert_eq!(options.format, OutputFormat::Json);
        assert_eq!(options.source_endpoint, "https://icp-api.io");
        assert_eq!(options.lock_stale_after_seconds, 300);
        assert!(options.dry_run);
        assert_eq!(
            options.output_path,
            Some(PathBuf::from("catalog.preview.json"))
        );
    }

    #[test]
    fn catalog_local_is_rejected_with_pinned_message() {
        let err = run([
            OsString::from("catalog"),
            OsString::from("list"),
            OsString::from("--__canic-network"),
            OsString::from("local"),
        ])
        .expect_err("local rejected");

        let message = err.to_string();
        assert!(message.contains("supports only the mainnet `ic` network in 0.60"));
        assert!(message.contains("canic --network ic subnet catalog list"));
    }

    #[test]
    fn refresh_is_advertised_as_catalog_command() {
        let text = catalog_usage();

        assert!(text.contains("refresh"));
        assert!(refresh_usage().contains("canic subnet catalog refresh"));
    }

    #[test]
    fn subnet_namespace_help_mentions_refresh() {
        let text = usage();

        assert!(text.contains("Inspect and refresh IC subnet catalog metadata"));
        assert!(!text.contains("Inspect cached IC network subnet metadata"));
    }

    #[test]
    fn role_resolution_reports_ambiguity() {
        let resolution = InstalledDeploymentResolution {
            source: canic_host::installed_deployment::InstalledDeploymentSource::IcpCli,
            state: sample_state(),
            registry: canic_host::installed_deployment::InstalledDeploymentRegistry {
                root_canister_id: "aaaaa-aa".to_string(),
                entries: vec![
                    registry_entry("ryjl3-tyaaa-aaaaa-aaaba-cai", "backend"),
                    registry_entry("rrkah-fqaaa-aaaaa-aaaaq-cai", "backend"),
                ],
            },
            topology: canic_host::installed_deployment::ResolvedDeploymentTopology {
                root_canister_id: "aaaaa-aa".to_string(),
                children_by_parent: BTreeMap::default(),
                roles_by_canister: BTreeMap::default(),
            },
        };

        let err =
            resolve_canister_or_role(&resolution, "demo", "backend").expect_err("ambiguous role");

        assert!(err.contains("role backend is ambiguous"));
    }

    fn registry_entry(pid: &str, role: &str) -> RegistryEntry {
        RegistryEntry {
            pid: pid.to_string(),
            role: Some(role.to_string()),
            kind: Some("canister".to_string()),
            parent_pid: None,
            module_hash: None,
        }
    }

    fn sample_state() -> canic_host::install_root::InstallState {
        canic_host::install_root::InstallState {
            schema_version: 2,
            deployment_name: "demo".to_string(),
            fleet_template: "demo".to_string(),
            created_at_unix_secs: 1,
            updated_at_unix_secs: 1,
            network: MAINNET_NETWORK.to_string(),
            root_target: "root".to_string(),
            root_canister_id: "aaaaa-aa".to_string(),
            root_verification: canic_host::install_root::RootVerificationStatus::Verified,
            root_build_target: "root".to_string(),
            workspace_root: ".".to_string(),
            icp_root: ".".to_string(),
            config_path: "fleets/demo/canic.toml".to_string(),
            release_set_manifest_path: ".canic/ic/release-set.json".to_string(),
        }
    }
}
