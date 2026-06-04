use crate::table::{ColumnAlign, render_table};
use canic_ic_registry::{
    DEFAULT_MAINNET_ENDPOINT, MainnetRegistryFetchRequest, RegistryFetchError,
    fetch_mainnet_subnet_catalog,
};
use canic_subnet_catalog::{
    CatalogError, ClassificationSource, GeographicScope, MAINNET_NETWORK, ResolveAs,
    ResolvedSubnetSubject, RoutingRange, SubnetCatalog, SubnetInfo, SubnetKind,
    SubnetSpecialization, catalog_to_pretty_json, parse_catalog_json,
};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

pub const DEFAULT_STALE_AFTER_SECONDS: u64 = 7 * 24 * 60 * 60;
pub const DEFAULT_REFRESH_LOCK_STALE_SECONDS: u64 = 30 * 60;
pub const DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT: &str = DEFAULT_MAINNET_ENDPOINT;
pub const SUBNET_CATALOG_LIST_REPORT_SCHEMA_VERSION: u32 = 1;
pub const SUBNET_CATALOG_INFO_REPORT_SCHEMA_VERSION: u32 = 1;
pub const SUBNET_CATALOG_REFRESH_REPORT_SCHEMA_VERSION: u32 = 1;
const BASE_13_NODE_CYCLES_PER_BILLION_INSTRUCTIONS: u128 = 1_000_000_000;
const FORMULA_VERSION: &str = "base_13_node_linear_v1";

///
/// SubnetCatalogCacheRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubnetCatalogCacheRequest {
    pub icp_root: PathBuf,
    pub network: String,
}

///
/// CachedSubnetCatalog
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachedSubnetCatalog {
    pub path: PathBuf,
    pub catalog: SubnetCatalog,
}

///
/// SubnetCatalogFilters
///
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SubnetCatalogFilters {
    pub kind: Option<SubnetKind>,
    pub specialization: Option<SubnetSpecialization>,
    pub geographic_scope: Option<GeographicScope>,
}

///
/// SubnetCatalogListRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubnetCatalogListRequest {
    pub cache: SubnetCatalogCacheRequest,
    pub now_unix_secs: u64,
    pub stale_after_seconds: u64,
    pub filters: SubnetCatalogFilters,
    pub show_ranges: bool,
    pub range_limit: usize,
    pub range_offset: usize,
}

///
/// SubnetCatalogInfoRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubnetCatalogInfoRequest {
    pub cache: SubnetCatalogCacheRequest,
    pub input: String,
    pub forced: Option<ResolveAs>,
    pub resolved_target: Option<ResolvedDeploymentTarget>,
    pub now_unix_secs: u64,
    pub stale_after_seconds: u64,
}

///
/// SubnetCatalogRefreshRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubnetCatalogRefreshRequest {
    pub cache: SubnetCatalogCacheRequest,
    pub source_endpoint: String,
    pub now_unix_secs: u64,
    pub lock_stale_after_seconds: u64,
    pub dry_run: bool,
    pub output_path: Option<PathBuf>,
}

///
/// ResolvedDeploymentTarget
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedDeploymentTarget {
    pub canister_principal: String,
    pub resolved_from: String,
}

///
/// CatalogStaleStatus
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CatalogStaleStatus {
    pub catalog_stale: bool,
    pub stale_reason: String,
    pub stale_after_seconds: u64,
    pub fetched_at_unix_secs: Option<u64>,
    pub age_seconds: Option<u64>,
}

///
/// SubnetCatalogListReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetCatalogListReport {
    pub schema_version: u32,
    pub network: String,
    pub catalog_path: String,
    pub catalog_schema_version: u32,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub catalog_stale: bool,
    pub stale_reason: String,
    pub resolver_backend: String,
    pub subnets: Vec<SubnetCatalogSubnetRow>,
}

///
/// SubnetCatalogSubnetRow
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetCatalogSubnetRow {
    pub subnet_principal: String,
    pub subnet_kind: SubnetKind,
    pub subnet_kind_source: ClassificationSource,
    pub subnet_specialization: SubnetSpecialization,
    pub subnet_specialization_source: ClassificationSource,
    pub geographic_scope: GeographicScope,
    pub geographic_scope_source: ClassificationSource,
    pub subnet_label: String,
    pub subnet_label_source: ClassificationSource,
    pub node_count: Option<u32>,
    pub charges_apply_by_default: bool,
    pub range_count: usize,
    pub ranges_shown: usize,
    pub range_offset: usize,
    pub range_limit: usize,
    pub ranges: Vec<RoutingRange>,
}

///
/// SubnetCatalogInfoReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetCatalogInfoReport {
    pub schema_version: u32,
    pub input_principal: String,
    pub resolved_as: String,
    pub resolved_from: String,
    pub subnet_principal: String,
    pub subnet_kind: SubnetKind,
    pub subnet_kind_source: ClassificationSource,
    pub subnet_specialization: SubnetSpecialization,
    pub subnet_specialization_source: ClassificationSource,
    pub geographic_scope: GeographicScope,
    pub geographic_scope_source: ClassificationSource,
    pub subnet_label: String,
    pub subnet_label_source: ClassificationSource,
    pub node_count: Option<u32>,
    pub charges_apply_to_subject: bool,
    pub charge_applicability_reason: String,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub catalog_schema_version: u32,
    pub catalog_path: String,
    pub fetched_at: String,
    pub catalog_stale: bool,
    pub stale_reason: String,
    pub resolver_backend: String,
    pub matched_canister_principal: Option<String>,
    pub matched_routing_range: Option<RoutingRange>,
    pub cycles_per_billion_instructions: Option<u128>,
    pub rate_source: Option<String>,
    pub formula_version: Option<String>,
}

///
/// SubnetCatalogRefreshReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetCatalogRefreshReport {
    pub schema_version: u32,
    pub network: String,
    pub catalog_path: String,
    pub refresh_lock_path: String,
    pub output_path: Option<String>,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub source_endpoint: String,
    pub fetched_by: String,
    pub dry_run: bool,
    pub wrote_catalog: bool,
    pub replaced_existing_catalog: bool,
    pub subnet_count: usize,
    pub routing_range_count: usize,
}

///
/// SubnetCatalogHostError
///
#[derive(Debug, ThisError)]
pub enum SubnetCatalogHostError {
    #[error(
        "`canic subnet catalog` supports only the mainnet `ic` network in 0.60\n\nThe cached subnet catalog describes the public Internet Computer mainnet.\nLocal replica subnet discovery is not implemented yet.\n\nTry:\n  canic --network ic subnet catalog list"
    )]
    UnsupportedNetwork { network: String },

    #[error(
        "subnet catalog cache is missing at {}\n\nRun `canic subnet catalog refresh` to fetch the public Internet Computer mainnet catalog, or populate this path with a valid Canic subnet catalog JSON.",
        path.display()
    )]
    MissingCatalog { path: PathBuf },

    #[error("failed to read subnet catalog at {}: {source}", path.display())]
    ReadCatalog { path: PathBuf, source: io::Error },

    #[error(
        "cached subnet catalog network mismatch: path is for {requested}, catalog is for {actual}"
    )]
    NetworkMismatch { requested: String, actual: String },

    #[error(
        "invalid stale duration {value:?}; use positive seconds or a value ending in s, m, h, or d"
    )]
    InvalidStaleDuration { value: String },

    #[error("subnet catalog refresh is already in progress; lock exists at {} since unix_ms={started_at_unix_ms}", path.display())]
    RefreshAlreadyInProgress {
        path: PathBuf,
        started_at_unix_ms: u64,
    },

    #[error("failed to create subnet catalog directory at {}: {source}", path.display())]
    CreateCatalogDirectory { path: PathBuf, source: io::Error },

    #[error("failed to create refresh lock at {}: {source}", path.display())]
    CreateRefreshLock { path: PathBuf, source: io::Error },

    #[error("failed to read refresh lock at {}: {source}", path.display())]
    ReadRefreshLock { path: PathBuf, source: io::Error },

    #[error("failed to parse refresh lock at {}: {source}", path.display())]
    ParseRefreshLock {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("failed to write refresh lock at {}: {source}", path.display())]
    WriteRefreshLock { path: PathBuf, source: io::Error },

    #[error("failed to remove refresh lock at {}: {source}", path.display())]
    RemoveRefreshLock { path: PathBuf, source: io::Error },

    #[error("live NNS registry refresh failed: {0}")]
    RegistryRefresh(#[from] RegistryFetchError),

    #[error("refreshed subnet catalog network mismatch: requested {requested}, fetched {actual}")]
    RefreshNetworkMismatch { requested: String, actual: String },

    #[error("failed to write subnet catalog temp file at {}: {source}", path.display())]
    WriteCatalogTemp { path: PathBuf, source: io::Error },

    #[error("failed to sync subnet catalog temp file at {}: {source}", path.display())]
    SyncCatalogTemp { path: PathBuf, source: io::Error },

    #[error("failed to replace subnet catalog at {} from {}: {source}", catalog_path.display(), temp_path.display())]
    ReplaceCatalog {
        temp_path: PathBuf,
        catalog_path: PathBuf,
        source: io::Error,
    },

    #[error("failed to sync subnet catalog directory at {}: {source}", path.display())]
    SyncCatalogDirectory { path: PathBuf, source: io::Error },

    #[error("failed to write refreshed subnet catalog output at {}: {source}", path.display())]
    WriteRefreshOutput { path: PathBuf, source: io::Error },

    #[error("failed to sync refreshed subnet catalog output at {}: {source}", path.display())]
    SyncRefreshOutput { path: PathBuf, source: io::Error },

    #[error(transparent)]
    Catalog(#[from] CatalogError),
}

#[must_use]
pub fn subnet_catalog_path(icp_root: &Path, network: &str) -> PathBuf {
    icp_root
        .join(".canic")
        .join("subnet-catalog")
        .join(network)
        .join("catalog.json")
}

#[must_use]
pub fn subnet_catalog_refresh_lock_path(icp_root: &Path, network: &str) -> PathBuf {
    icp_root
        .join(".canic")
        .join("subnet-catalog")
        .join(network)
        .join("refresh.lock")
}

pub fn load_cached_subnet_catalog(
    request: &SubnetCatalogCacheRequest,
) -> Result<CachedSubnetCatalog, SubnetCatalogHostError> {
    enforce_mainnet_network(&request.network)?;
    let path = subnet_catalog_path(&request.icp_root, &request.network);
    if !path.is_file() {
        return Err(SubnetCatalogHostError::MissingCatalog { path });
    }
    let data = fs::read_to_string(&path).map_err(|source| SubnetCatalogHostError::ReadCatalog {
        path: path.clone(),
        source,
    })?;
    let catalog = parse_catalog_json(&data)?;
    if catalog.network != request.network {
        return Err(SubnetCatalogHostError::NetworkMismatch {
            requested: request.network.clone(),
            actual: catalog.network,
        });
    }
    Ok(CachedSubnetCatalog { path, catalog })
}

pub fn refresh_subnet_catalog(
    request: &SubnetCatalogRefreshRequest,
) -> Result<SubnetCatalogRefreshReport, SubnetCatalogHostError> {
    refresh_subnet_catalog_with_source(request, &LiveNnsRegistryRefreshSource)
}

fn refresh_subnet_catalog_with_source(
    request: &SubnetCatalogRefreshRequest,
    source: &dyn SubnetCatalogRefreshSource,
) -> Result<SubnetCatalogRefreshReport, SubnetCatalogHostError> {
    enforce_mainnet_network(&request.cache.network)?;
    let catalog_path = subnet_catalog_path(&request.cache.icp_root, &request.cache.network);
    let lock_path =
        subnet_catalog_refresh_lock_path(&request.cache.icp_root, &request.cache.network);
    let catalog_dir = catalog_path
        .parent()
        .expect("subnet catalog path always has parent")
        .to_path_buf();
    fs::create_dir_all(&catalog_dir).map_err(|source| {
        SubnetCatalogHostError::CreateCatalogDirectory {
            path: catalog_dir.clone(),
            source,
        }
    })?;
    let lock = acquire_refresh_lock(
        &lock_path,
        &catalog_path,
        &request.cache.network,
        request.now_unix_secs,
        request.lock_stale_after_seconds,
    )?;
    let replaced_existing_catalog = catalog_path.is_file();
    let fetched_at = format_utc_timestamp_secs(request.now_unix_secs);
    let mut fetch_request = MainnetRegistryFetchRequest::new(fetched_at);
    fetch_request.endpoint.clone_from(&request.source_endpoint);
    let catalog = source.fetch_catalog(&fetch_request)?;
    if catalog.network != request.cache.network {
        return Err(SubnetCatalogHostError::RefreshNetworkMismatch {
            requested: request.cache.network.clone(),
            actual: catalog.network,
        });
    }
    catalog.validate()?;
    let catalog_json = catalog_to_pretty_json(&catalog)?;
    if let Some(output_path) = &request.output_path {
        write_refresh_output(output_path, &catalog_json)?;
    }
    if !request.dry_run {
        write_catalog_atomically(&catalog_path, &catalog_json)?;
    }
    lock.release()?;
    Ok(SubnetCatalogRefreshReport {
        schema_version: SUBNET_CATALOG_REFRESH_REPORT_SCHEMA_VERSION,
        network: catalog.network,
        catalog_path: catalog_path.display().to_string(),
        refresh_lock_path: lock_path.display().to_string(),
        output_path: request
            .output_path
            .as_ref()
            .map(|path| path.display().to_string()),
        registry_canister_id: catalog.registry_canister_id,
        registry_version: catalog.registry_version,
        fetched_at: catalog.fetched_at,
        source_endpoint: catalog.source_endpoint,
        fetched_by: catalog.fetched_by,
        dry_run: request.dry_run,
        wrote_catalog: !request.dry_run,
        replaced_existing_catalog,
        subnet_count: catalog.subnets.len(),
        routing_range_count: catalog.routing_ranges.len(),
    })
}

pub fn build_subnet_catalog_list_report(
    request: &SubnetCatalogListRequest,
) -> Result<SubnetCatalogListReport, SubnetCatalogHostError> {
    let cached = load_cached_subnet_catalog(&request.cache)?;
    let stale = catalog_stale_status(
        &cached.catalog,
        request.now_unix_secs,
        request.stale_after_seconds,
    );
    let subnets = cached
        .catalog
        .subnets
        .iter()
        .filter(|subnet| subnet_matches_filters(subnet, request.filters))
        .map(|subnet| subnet_row(&cached.catalog, subnet, request))
        .collect::<Vec<_>>();

    Ok(SubnetCatalogListReport {
        schema_version: SUBNET_CATALOG_LIST_REPORT_SCHEMA_VERSION,
        network: cached.catalog.network,
        catalog_path: cached.path.display().to_string(),
        catalog_schema_version: cached.catalog.catalog_schema_version,
        registry_canister_id: cached.catalog.registry_canister_id,
        registry_version: cached.catalog.registry_version,
        fetched_at: cached.catalog.fetched_at,
        catalog_stale: stale.catalog_stale,
        stale_reason: stale.stale_reason,
        resolver_backend: cached.catalog.resolver_backend,
        subnets,
    })
}

pub fn build_subnet_catalog_info_report(
    request: &SubnetCatalogInfoRequest,
) -> Result<SubnetCatalogInfoReport, SubnetCatalogHostError> {
    let cached = load_cached_subnet_catalog(&request.cache)?;
    let stale = catalog_stale_status(
        &cached.catalog,
        request.now_unix_secs,
        request.stale_after_seconds,
    );
    let resolved = if let Some(target) = &request.resolved_target {
        let mut resolved = cached
            .catalog
            .resolve_canister(&target.canister_principal)?;
        resolved.input_principal.clone_from(&request.input);
        resolved.resolved_from.clone_from(&target.resolved_from);
        resolved
    } else {
        cached
            .catalog
            .resolve_principal(&request.input, request.forced)?
    };
    let (charges_apply_to_subject, charge_applicability_reason) =
        charge_applicability(resolved.resolved_as, resolved.subnet.subnet_kind);
    let cycles_per_billion_instructions = catalog_cycles_per_billion(&resolved.subnet);
    let rate_source = cycles_per_billion_instructions
        .is_some()
        .then(|| "nns-registry-cache".to_string());
    let formula_version = cycles_per_billion_instructions
        .is_some()
        .then(|| FORMULA_VERSION.to_string());

    Ok(SubnetCatalogInfoReport {
        schema_version: SUBNET_CATALOG_INFO_REPORT_SCHEMA_VERSION,
        input_principal: resolved.input_principal,
        resolved_as: resolved.resolved_as.as_str().to_string(),
        resolved_from: resolved.resolved_from,
        subnet_principal: resolved.subnet.subnet_principal,
        subnet_kind: resolved.subnet.subnet_kind,
        subnet_kind_source: resolved.subnet.subnet_kind_source,
        subnet_specialization: resolved.subnet.subnet_specialization,
        subnet_specialization_source: resolved.subnet.subnet_specialization_source,
        geographic_scope: resolved.subnet.geographic_scope,
        geographic_scope_source: resolved.subnet.geographic_scope_source,
        subnet_label: resolved.subnet.subnet_label,
        subnet_label_source: resolved.subnet.subnet_label_source,
        node_count: resolved.subnet.node_count,
        charges_apply_to_subject,
        charge_applicability_reason,
        registry_canister_id: cached.catalog.registry_canister_id,
        registry_version: cached.catalog.registry_version,
        catalog_schema_version: cached.catalog.catalog_schema_version,
        catalog_path: cached.path.display().to_string(),
        fetched_at: cached.catalog.fetched_at,
        catalog_stale: stale.catalog_stale,
        stale_reason: stale.stale_reason,
        resolver_backend: cached.catalog.resolver_backend,
        matched_canister_principal: resolved.matched_canister_principal,
        matched_routing_range: resolved.matched_routing_range,
        cycles_per_billion_instructions,
        rate_source,
        formula_version,
    })
}

#[must_use]
pub fn catalog_stale_status(
    catalog: &SubnetCatalog,
    now_unix_secs: u64,
    stale_after_seconds: u64,
) -> CatalogStaleStatus {
    let Some(fetched_at_unix_secs) = parse_utc_timestamp_secs(&catalog.fetched_at) else {
        return CatalogStaleStatus {
            catalog_stale: true,
            stale_reason: "fetched_at_unparseable".to_string(),
            stale_after_seconds,
            fetched_at_unix_secs: None,
            age_seconds: None,
        };
    };
    let Some(age_seconds) = now_unix_secs.checked_sub(fetched_at_unix_secs) else {
        return CatalogStaleStatus {
            catalog_stale: false,
            stale_reason: "fetched_at_in_future".to_string(),
            stale_after_seconds,
            fetched_at_unix_secs: Some(fetched_at_unix_secs),
            age_seconds: None,
        };
    };
    let catalog_stale = age_seconds > stale_after_seconds;
    CatalogStaleStatus {
        catalog_stale,
        stale_reason: if catalog_stale { "expired" } else { "fresh" }.to_string(),
        stale_after_seconds,
        fetched_at_unix_secs: Some(fetched_at_unix_secs),
        age_seconds: Some(age_seconds),
    }
}

pub fn parse_stale_after_duration(value: &str) -> Result<u64, SubnetCatalogHostError> {
    let (number, multiplier) = match value.as_bytes().last().copied() {
        Some(b's') => (&value[..value.len() - 1], 1),
        Some(b'm') => (&value[..value.len() - 1], 60),
        Some(b'h') => (&value[..value.len() - 1], 60 * 60),
        Some(b'd') => (&value[..value.len() - 1], 24 * 60 * 60),
        Some(b'0'..=b'9') => (value, 1),
        _ => return invalid_stale_duration(value),
    };
    let seconds = number
        .parse::<u64>()
        .ok()
        .and_then(|amount| amount.checked_mul(multiplier))
        .filter(|seconds| *seconds > 0)
        .ok_or_else(|| SubnetCatalogHostError::InvalidStaleDuration {
            value: value.to_string(),
        })?;
    Ok(seconds)
}

#[must_use]
pub fn subnet_catalog_list_report_text(report: &SubnetCatalogListReport) -> String {
    let headers = [
        "SUBNET", "KIND", "SPEC", "GEO", "NODES", "CHG", "RANGES", "STALE",
    ];
    let rows = report
        .subnets
        .iter()
        .map(|subnet| {
            [
                compact_principal(&subnet.subnet_principal),
                subnet.subnet_kind.as_str().to_string(),
                subnet.subnet_specialization.as_str().to_string(),
                subnet.geographic_scope.as_str().to_string(),
                subnet
                    .node_count
                    .map_or_else(|| "unknown".to_string(), |count| count.to_string()),
                yes_no(subnet.charges_apply_by_default).to_string(),
                subnet.range_count.to_string(),
                yes_no(report.catalog_stale).to_string(),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Left,
    ];
    let mut lines = Vec::new();
    lines.push(format!(
        "catalog: {} version {} stale {}",
        report.network,
        report.registry_version,
        yes_no(report.catalog_stale)
    ));
    if rows.is_empty() {
        lines.push("subnets: none".to_string());
        return lines.join("\n");
    }
    lines.push(render_table(&headers, &rows, &alignments));
    append_compact_range_lines(report, &mut lines);
    lines.join("\n")
}

#[must_use]
pub fn subnet_catalog_list_report_verbose_text(report: &SubnetCatalogListReport) -> String {
    let headers = [
        "SUBNET",
        "KIND",
        "SPECIALIZATION",
        "GEO",
        "NODES",
        "CHARGES",
        "RANGES",
        "VERSION",
        "FETCHED_AT",
        "STALE",
    ];
    let rows = report
        .subnets
        .iter()
        .map(|subnet| {
            [
                subnet.subnet_principal.clone(),
                subnet.subnet_kind.as_str().to_string(),
                subnet.subnet_specialization.as_str().to_string(),
                subnet.geographic_scope.as_str().to_string(),
                subnet
                    .node_count
                    .map_or_else(|| "unknown".to_string(), |count| count.to_string()),
                yes_no(subnet.charges_apply_by_default).to_string(),
                subnet.range_count.to_string(),
                report.registry_version.to_string(),
                report.fetched_at.clone(),
                yes_no(report.catalog_stale).to_string(),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Left,
        ColumnAlign::Left,
    ];
    let mut lines = Vec::new();
    lines.push(format!("catalog_path: {}", report.catalog_path));
    lines.push(format!("stale_reason: {}", report.stale_reason));
    if rows.is_empty() {
        lines.push("subnets: none".to_string());
        return lines.join("\n");
    }
    lines.push(render_table(&headers, &rows, &alignments));
    append_range_lines(report, &mut lines);
    lines.join("\n")
}

#[must_use]
pub fn subnet_catalog_info_report_text(report: &SubnetCatalogInfoReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!("input_principal: {}", report.input_principal));
    lines.push(format!("resolved_as: {}", report.resolved_as));
    lines.push(format!("resolved_from: {}", report.resolved_from));
    lines.push(format!("subnet_principal: {}", report.subnet_principal));
    lines.push(format!("subnet_kind: {}", report.subnet_kind.as_str()));
    lines.push(format!(
        "subnet_kind_source: {}",
        report.subnet_kind_source.as_str()
    ));
    lines.push(format!(
        "subnet_specialization: {}",
        report.subnet_specialization.as_str()
    ));
    lines.push(format!(
        "subnet_specialization_source: {}",
        report.subnet_specialization_source.as_str()
    ));
    lines.push(format!(
        "geographic_scope: {}",
        report.geographic_scope.as_str()
    ));
    lines.push(format!(
        "geographic_scope_source: {}",
        report.geographic_scope_source.as_str()
    ));
    lines.push(format!("subnet_label: {}", report.subnet_label));
    lines.push(format!(
        "subnet_label_source: {}",
        report.subnet_label_source.as_str()
    ));
    lines.push(format!(
        "node_count: {}",
        report
            .node_count
            .map_or_else(|| "unknown".to_string(), |count| count.to_string())
    ));
    lines.push(format!(
        "charges_apply_to_subject: {}",
        yes_no(report.charges_apply_to_subject)
    ));
    lines.push(format!(
        "charge_applicability_reason: {}",
        report.charge_applicability_reason
    ));
    lines.push(format!(
        "registry_canister_id: {}",
        report.registry_canister_id
    ));
    lines.push(format!("registry_version: {}", report.registry_version));
    lines.push(format!(
        "catalog_schema_version: {}",
        report.catalog_schema_version
    ));
    lines.push(format!("catalog_path: {}", report.catalog_path));
    lines.push(format!("fetched_at: {}", report.fetched_at));
    lines.push(format!("catalog_stale: {}", yes_no(report.catalog_stale)));
    lines.push(format!("stale_reason: {}", report.stale_reason));
    lines.push(format!("resolver_backend: {}", report.resolver_backend));
    if let Some(canister) = &report.matched_canister_principal {
        lines.push(format!("matched_canister_principal: {canister}"));
    }
    if let Some(range) = &report.matched_routing_range {
        lines.push(format!(
            "matched_routing_range: {}..{}",
            range.start_canister_id, range.end_canister_id
        ));
    }
    lines.push(format!(
        "cycles_per_billion_instructions: {}",
        report
            .cycles_per_billion_instructions
            .map_or_else(|| "not_applicable".to_string(), |cycles| cycles.to_string())
    ));
    if let Some(rate_source) = &report.rate_source {
        lines.push(format!("rate_source: {rate_source}"));
    }
    if let Some(formula_version) = &report.formula_version {
        lines.push(format!("formula_version: {formula_version}"));
    }
    lines.join("\n")
}

#[must_use]
pub fn subnet_catalog_refresh_report_text(report: &SubnetCatalogRefreshReport) -> String {
    [
        format!("network: {}", report.network),
        format!("catalog_path: {}", report.catalog_path),
        format!("refresh_lock_path: {}", report.refresh_lock_path),
        format!("registry_canister_id: {}", report.registry_canister_id),
        format!("registry_version: {}", report.registry_version),
        format!("fetched_at: {}", report.fetched_at),
        format!("source_endpoint: {}", report.source_endpoint),
        format!("fetched_by: {}", report.fetched_by),
        format!("dry_run: {}", yes_no(report.dry_run)),
        format!("wrote_catalog: {}", yes_no(report.wrote_catalog)),
        format!(
            "replaced_existing_catalog: {}",
            yes_no(report.replaced_existing_catalog)
        ),
        format!("subnet_count: {}", report.subnet_count),
        format!("routing_range_count: {}", report.routing_range_count),
    ]
    .join("\n")
}

fn enforce_mainnet_network(network: &str) -> Result<(), SubnetCatalogHostError> {
    if network == MAINNET_NETWORK {
        return Ok(());
    }
    Err(SubnetCatalogHostError::UnsupportedNetwork {
        network: network.to_string(),
    })
}

trait SubnetCatalogRefreshSource {
    fn fetch_catalog(
        &self,
        request: &MainnetRegistryFetchRequest,
    ) -> Result<SubnetCatalog, SubnetCatalogHostError>;
}

///
/// LiveNnsRegistryRefreshSource
///
struct LiveNnsRegistryRefreshSource;

impl SubnetCatalogRefreshSource for LiveNnsRegistryRefreshSource {
    fn fetch_catalog(
        &self,
        request: &MainnetRegistryFetchRequest,
    ) -> Result<SubnetCatalog, SubnetCatalogHostError> {
        Ok(fetch_mainnet_subnet_catalog(request)?)
    }
}

///
/// RefreshLockFile
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct RefreshLockFile {
    schema_version: u32,
    network: String,
    pid: u32,
    started_at_unix_ms: u64,
    catalog_path: String,
}

///
/// RefreshLockGuard
///
#[derive(Debug)]
struct RefreshLockGuard {
    path: PathBuf,
    active: bool,
}

impl RefreshLockGuard {
    fn release(mut self) -> Result<(), SubnetCatalogHostError> {
        fs::remove_file(&self.path).map_err(|source| {
            SubnetCatalogHostError::RemoveRefreshLock {
                path: self.path.clone(),
                source,
            }
        })?;
        self.active = false;
        Ok(())
    }
}

impl Drop for RefreshLockGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn acquire_refresh_lock(
    lock_path: &Path,
    catalog_path: &Path,
    network: &str,
    now_unix_secs: u64,
    lock_stale_after_seconds: u64,
) -> Result<RefreshLockGuard, SubnetCatalogHostError> {
    let now_unix_ms = now_unix_secs.saturating_mul(1_000);
    for attempt in 0..2 {
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(lock_path)
        {
            Ok(mut file) => {
                let lock = RefreshLockFile {
                    schema_version: 1,
                    network: network.to_string(),
                    pid: std::process::id(),
                    started_at_unix_ms: now_unix_ms,
                    catalog_path: catalog_path.display().to_string(),
                };
                let data = serde_json::to_vec_pretty(&lock).map_err(|source| {
                    SubnetCatalogHostError::ParseRefreshLock {
                        path: lock_path.to_path_buf(),
                        source,
                    }
                })?;
                file.write_all(&data).map_err(|source| {
                    SubnetCatalogHostError::WriteRefreshLock {
                        path: lock_path.to_path_buf(),
                        source,
                    }
                })?;
                file.sync_all()
                    .map_err(|source| SubnetCatalogHostError::WriteRefreshLock {
                        path: lock_path.to_path_buf(),
                        source,
                    })?;
                return Ok(RefreshLockGuard {
                    path: lock_path.to_path_buf(),
                    active: true,
                });
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                let existing = read_refresh_lock(lock_path)?;
                if lock_is_stale(
                    existing.started_at_unix_ms,
                    now_unix_ms,
                    lock_stale_after_seconds,
                ) && attempt == 0
                {
                    fs::remove_file(lock_path).map_err(|source| {
                        SubnetCatalogHostError::RemoveRefreshLock {
                            path: lock_path.to_path_buf(),
                            source,
                        }
                    })?;
                    continue;
                }
                return Err(SubnetCatalogHostError::RefreshAlreadyInProgress {
                    path: lock_path.to_path_buf(),
                    started_at_unix_ms: existing.started_at_unix_ms,
                });
            }
            Err(source) => {
                return Err(SubnetCatalogHostError::CreateRefreshLock {
                    path: lock_path.to_path_buf(),
                    source,
                });
            }
        }
    }
    Err(SubnetCatalogHostError::CreateRefreshLock {
        path: lock_path.to_path_buf(),
        source: io::Error::new(io::ErrorKind::AlreadyExists, "refresh lock retry exhausted"),
    })
}

fn read_refresh_lock(lock_path: &Path) -> Result<RefreshLockFile, SubnetCatalogHostError> {
    let data = fs::read(lock_path).map_err(|source| SubnetCatalogHostError::ReadRefreshLock {
        path: lock_path.to_path_buf(),
        source,
    })?;
    serde_json::from_slice(&data).map_err(|source| SubnetCatalogHostError::ParseRefreshLock {
        path: lock_path.to_path_buf(),
        source,
    })
}

fn lock_is_stale(started_at_unix_ms: u64, now_unix_ms: u64, stale_after_seconds: u64) -> bool {
    now_unix_ms
        .saturating_sub(started_at_unix_ms)
        .gt(&stale_after_seconds.saturating_mul(1_000))
}

fn write_catalog_atomically(
    catalog_path: &Path,
    catalog_json: &str,
) -> Result<(), SubnetCatalogHostError> {
    let catalog_dir = catalog_path
        .parent()
        .expect("subnet catalog path always has parent");
    let temp_path = catalog_dir.join(format!("catalog.json.tmp.{}", std::process::id()));
    {
        let mut temp = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|source| SubnetCatalogHostError::WriteCatalogTemp {
                path: temp_path.clone(),
                source,
            })?;
        temp.write_all(catalog_json.as_bytes()).map_err(|source| {
            SubnetCatalogHostError::WriteCatalogTemp {
                path: temp_path.clone(),
                source,
            }
        })?;
        temp.sync_all()
            .map_err(|source| SubnetCatalogHostError::SyncCatalogTemp {
                path: temp_path.clone(),
                source,
            })?;
    }
    fs::rename(&temp_path, catalog_path).map_err(|source| {
        SubnetCatalogHostError::ReplaceCatalog {
            temp_path: temp_path.clone(),
            catalog_path: catalog_path.to_path_buf(),
            source,
        }
    })?;
    sync_directory(catalog_dir)
}

fn write_refresh_output(
    output_path: &Path,
    catalog_json: &str,
) -> Result<(), SubnetCatalogHostError> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|source| {
            SubnetCatalogHostError::CreateCatalogDirectory {
                path: parent.to_path_buf(),
                source,
            }
        })?;
    }
    let mut output = fs::File::create(output_path).map_err(|source| {
        SubnetCatalogHostError::WriteRefreshOutput {
            path: output_path.to_path_buf(),
            source,
        }
    })?;
    output
        .write_all(catalog_json.as_bytes())
        .map_err(|source| SubnetCatalogHostError::WriteRefreshOutput {
            path: output_path.to_path_buf(),
            source,
        })?;
    output
        .sync_all()
        .map_err(|source| SubnetCatalogHostError::SyncRefreshOutput {
            path: output_path.to_path_buf(),
            source,
        })
}

fn sync_directory(path: &Path) -> Result<(), SubnetCatalogHostError> {
    fs::File::open(path)
        .and_then(|dir| dir.sync_all())
        .map_err(|source| SubnetCatalogHostError::SyncCatalogDirectory {
            path: path.to_path_buf(),
            source,
        })
}

fn subnet_matches_filters(subnet: &SubnetInfo, filters: SubnetCatalogFilters) -> bool {
    filters.kind.is_none_or(|kind| subnet.subnet_kind == kind)
        && filters
            .specialization
            .is_none_or(|specialization| subnet.subnet_specialization == specialization)
        && filters
            .geographic_scope
            .is_none_or(|scope| subnet.geographic_scope == scope)
}

fn subnet_row(
    catalog: &SubnetCatalog,
    subnet: &SubnetInfo,
    request: &SubnetCatalogListRequest,
) -> SubnetCatalogSubnetRow {
    let ranges = catalog.routing_ranges_for_subnet(&subnet.subnet_principal);
    let range_count = ranges.len();
    let shown_ranges = if request.show_ranges {
        ranges
            .into_iter()
            .skip(request.range_offset)
            .take(request.range_limit)
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    SubnetCatalogSubnetRow {
        subnet_principal: subnet.subnet_principal.clone(),
        subnet_kind: subnet.subnet_kind,
        subnet_kind_source: subnet.subnet_kind_source,
        subnet_specialization: subnet.subnet_specialization,
        subnet_specialization_source: subnet.subnet_specialization_source,
        geographic_scope: subnet.geographic_scope,
        geographic_scope_source: subnet.geographic_scope_source,
        subnet_label: subnet.subnet_label.clone(),
        subnet_label_source: subnet.subnet_label_source,
        node_count: subnet.node_count,
        charges_apply_by_default: subnet.charges_apply_by_default,
        range_count,
        ranges_shown: shown_ranges.len(),
        range_offset: request.range_offset,
        range_limit: request.range_limit,
        ranges: shown_ranges,
    }
}

fn charge_applicability(subject: ResolvedSubnetSubject, kind: SubnetKind) -> (bool, String) {
    match kind {
        SubnetKind::Application => (true, "charged_user_canister_subnet".to_string()),
        SubnetKind::System if subject == ResolvedSubnetSubject::Subnet => {
            (false, "system_subnet_core_canister".to_string())
        }
        SubnetKind::System => (false, "system_subnet_unknown_subject".to_string()),
        SubnetKind::Unknown => (false, "unknown_subnet_type".to_string()),
    }
}

fn catalog_cycles_per_billion(subnet: &SubnetInfo) -> Option<u128> {
    if subnet.subnet_kind != SubnetKind::Application {
        return None;
    }
    let node_count = u128::from(subnet.node_count?);
    if node_count == 0 {
        return None;
    }
    Some(ceil_div(
        BASE_13_NODE_CYCLES_PER_BILLION_INSTRUCTIONS * node_count,
        13,
    ))
}

const fn ceil_div(numerator: u128, denominator: u128) -> u128 {
    numerator.div_ceil(denominator)
}

fn append_range_lines(report: &SubnetCatalogListReport, lines: &mut Vec<String>) {
    for subnet in &report.subnets {
        if subnet.ranges.is_empty() {
            continue;
        }
        lines.push(format!("ranges for {}:", subnet.subnet_principal));
        for range in &subnet.ranges {
            lines.push(format!(
                "  {}..{}",
                range.start_canister_id, range.end_canister_id
            ));
        }
        if subnet.ranges_shown < subnet.range_count {
            lines.push(format!(
                "  showing {} of {} ranges; use --range-limit or --format json",
                subnet.ranges_shown, subnet.range_count
            ));
        }
    }
}

fn append_compact_range_lines(report: &SubnetCatalogListReport, lines: &mut Vec<String>) {
    for subnet in &report.subnets {
        if subnet.ranges.is_empty() {
            continue;
        }
        lines.push(format!(
            "ranges for {}:",
            compact_principal(&subnet.subnet_principal)
        ));
        for range in &subnet.ranges {
            lines.push(format!(
                "  {}..{}",
                compact_principal(&range.start_canister_id),
                compact_principal(&range.end_canister_id)
            ));
        }
        if subnet.ranges_shown < subnet.range_count {
            lines.push(format!(
                "  showing {} of {} ranges; use --range-limit or --format json",
                subnet.ranges_shown, subnet.range_count
            ));
        }
    }
}

fn compact_principal(value: &str) -> String {
    value.chars().take(5).collect()
}

fn parse_utc_timestamp_secs(value: &str) -> Option<u64> {
    let value = value.strip_suffix('Z')?;
    let (date, time) = value.split_once('T')?;
    let mut date_parts = date.split('-');
    let year = date_parts.next()?.parse::<i64>().ok()?;
    let month = date_parts.next()?.parse::<u32>().ok()?;
    let day = date_parts.next()?.parse::<u32>().ok()?;
    if date_parts.next().is_some() {
        return None;
    }
    let mut time_parts = time.split(':');
    let hour = time_parts.next()?.parse::<u32>().ok()?;
    let minute = time_parts.next()?.parse::<u32>().ok()?;
    let second = time_parts.next()?.parse::<u32>().ok()?;
    if time_parts.next().is_some()
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    let days = days_from_civil(year, month, day)?;
    let seconds = days
        .checked_mul(86_400)?
        .checked_add(i64::from(hour) * 3_600)?
        .checked_add(i64::from(minute) * 60)?
        .checked_add(i64::from(second))?;
    u64::try_from(seconds).ok()
}

fn format_utc_timestamp_secs(value: u64) -> String {
    let days = i64::try_from(value / 86_400).unwrap_or(i64::MAX);
    let seconds_of_day = value % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (
        year,
        u32::try_from(month).expect("civil month is in u32 range"),
        u32::try_from(day).expect("civil day is in u32 range"),
    )
}

fn days_from_civil(year: i64, month: u32, day: u32) -> Option<i64> {
    let month = i64::from(month);
    let day = i64::from(day);
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era.checked_mul(146_097)?
        .checked_add(day_of_era)?
        .checked_sub(719_468)
}

fn invalid_stale_duration<T>(value: &str) -> Result<T, SubnetCatalogHostError> {
    Err(SubnetCatalogHostError::InvalidStaleDuration {
        value: value.to_string(),
    })
}

const fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use canic_subnet_catalog::{
        CATALOG_SCHEMA_VERSION, ClassificationSource, GeographicScope,
        MAINNET_REGISTRY_CANISTER_ID, SubnetSpecialization,
    };

    const SUBNET_A: &str = "rwlgt-iiaaa-aaaaa-aaaaa-cai";
    const SUBNET_B: &str = "aaaaa-aa";
    const CANISTER_A: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";

    #[test]
    fn catalog_path_lives_outside_deployment_state() {
        let root = PathBuf::from("/tmp/canic-project");

        let path = subnet_catalog_path(&root, MAINNET_NETWORK);

        assert_eq!(
            path,
            PathBuf::from("/tmp/canic-project/.canic/subnet-catalog/ic/catalog.json")
        );
        assert!(!path.display().to_string().contains("/deployments/"));
        assert!(!path.display().to_string().contains("/fleets/"));
    }

    #[test]
    fn load_cached_catalog_rejects_non_mainnet_network() {
        let root = temp_dir("canic-subnet-host-network");
        let request = SubnetCatalogCacheRequest {
            icp_root: root.clone(),
            network: "local".to_string(),
        };

        let err = load_cached_subnet_catalog(&request).expect_err("local rejected");

        let _ = fs::remove_dir_all(root);
        std::assert_matches!(err, SubnetCatalogHostError::UnsupportedNetwork { .. });
    }

    #[test]
    fn missing_catalog_error_explains_cached_only_slice() {
        let root = temp_dir("canic-subnet-host-missing");
        let request = SubnetCatalogCacheRequest {
            icp_root: root.clone(),
            network: MAINNET_NETWORK.to_string(),
        };

        let err = load_cached_subnet_catalog(&request).expect_err("cache missing");
        let message = err.to_string();

        let _ = fs::remove_dir_all(root);
        assert!(message.contains("Run `canic subnet catalog refresh`"));
        assert!(message.contains("public Internet Computer mainnet catalog"));
        assert!(message.contains("canic subnet catalog refresh"));
    }

    #[test]
    fn list_report_loads_cached_catalog_and_caps_ranges() {
        let root = temp_dir("canic-subnet-host-list");
        write_catalog(&root, fixture_catalog());
        let request = list_request(&root);

        let report = build_subnet_catalog_list_report(&request).expect("list report");
        let text = subnet_catalog_list_report_text(&report);

        let _ = fs::remove_dir_all(root);
        assert_eq!(report.subnets.len(), 2);
        assert_eq!(report.subnets[0].range_count, 2);
        assert_eq!(report.subnets[0].ranges_shown, 1);
        assert!(text.contains("SUBNET"));
        assert!(text.contains("SPEC"));
        assert!(!text.contains("SPECIALIZATION"));
        for subnet in &report.subnets {
            assert!(text.contains(&compact_principal(&subnet.subnet_principal)));
            assert!(!text.contains(&subnet.subnet_principal));
        }
        assert!(!text.contains("FETCHED_AT"));
        assert!(text.contains("showing 1 of 2 ranges"));
    }

    #[test]
    fn list_report_verbose_text_keeps_full_metadata() {
        let root = temp_dir("canic-subnet-host-list-verbose");
        write_catalog(&root, fixture_catalog());
        let request = list_request(&root);

        let report = build_subnet_catalog_list_report(&request).expect("list report");
        let text = subnet_catalog_list_report_verbose_text(&report);

        let _ = fs::remove_dir_all(root);
        assert!(text.contains("catalog_path:"));
        assert!(text.contains("SPECIALIZATION"));
        assert!(text.contains("FETCHED_AT"));
        assert!(text.contains(SUBNET_A));
    }

    #[test]
    fn info_report_resolves_canister_and_marks_application_chargeable() {
        let root = temp_dir("canic-subnet-host-info");
        write_catalog(&root, fixture_catalog());
        let request = info_request(&root, CANISTER_A);

        let report = build_subnet_catalog_info_report(&request).expect("info report");

        let _ = fs::remove_dir_all(root);
        assert_eq!(report.resolved_as, "canister");
        assert_eq!(report.subnet_principal, SUBNET_A);
        assert!(report.charges_apply_to_subject);
        assert_eq!(
            report.charge_applicability_reason,
            "charged_user_canister_subnet"
        );
        assert_eq!(report.cycles_per_billion_instructions, Some(2_615_384_616));
    }

    #[test]
    fn system_subnet_has_no_catalog_rate() {
        let root = temp_dir("canic-subnet-host-system");
        let mut catalog = fixture_catalog();
        catalog.subnets[0].subnet_kind = SubnetKind::System;
        catalog.subnets[0].charges_apply_by_default = false;
        write_catalog(&root, catalog);
        let request = info_request(&root, CANISTER_A);

        let report = build_subnet_catalog_info_report(&request).expect("info report");

        let _ = fs::remove_dir_all(root);
        assert!(!report.charges_apply_to_subject);
        assert_eq!(
            report.charge_applicability_reason,
            "system_subnet_unknown_subject"
        );
        assert_eq!(report.cycles_per_billion_instructions, None);
    }

    #[test]
    fn stale_status_is_deterministic() {
        let catalog = fixture_catalog();
        let fresh = catalog_stale_status(&catalog, 1_780_531_300, 200);
        let stale = catalog_stale_status(&catalog, 1_780_531_501, 200);

        assert!(!fresh.catalog_stale);
        assert!(stale.catalog_stale);
    }

    #[test]
    fn stale_duration_accepts_units() {
        assert_eq!(parse_stale_after_duration("7d").expect("days"), 604_800);
        assert_eq!(parse_stale_after_duration("2h").expect("hours"), 7_200);
        assert_eq!(parse_stale_after_duration("30m").expect("minutes"), 1_800);
        assert_eq!(parse_stale_after_duration("90s").expect("seconds"), 90);
        assert_eq!(parse_stale_after_duration("42").expect("bare"), 42);
        std::assert_matches!(
            parse_stale_after_duration("0d"),
            Err(SubnetCatalogHostError::InvalidStaleDuration { .. })
        );
    }

    #[test]
    fn refresh_writes_catalog_atomically_and_removes_lock() {
        let root = temp_dir("canic-subnet-host-refresh");
        let mut catalog = fixture_catalog();
        catalog.registry_version = 987_654;
        catalog.fetched_at = "1970-01-01T00:00:00Z".to_string();
        catalog.source_endpoint = DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT.to_string();
        let source = FixtureRefreshSource::ok(catalog);
        let request = refresh_request(&root);

        let report =
            refresh_subnet_catalog_with_source(&request, &source).expect("refresh catalog");
        let cached = load_cached_subnet_catalog(&cache_request(&root)).expect("cached catalog");
        let lock_path = PathBuf::from(&report.refresh_lock_path);

        let _ = fs::remove_dir_all(root);
        assert!(report.wrote_catalog);
        assert!(!report.replaced_existing_catalog);
        assert_eq!(report.registry_version, 987_654);
        assert_eq!(cached.catalog.registry_version, 987_654);
        assert!(!lock_path.exists());
    }

    #[test]
    fn refresh_dry_run_writes_output_without_replacing_cache() {
        let root = temp_dir("canic-subnet-host-refresh-dry-run");
        let mut catalog = fixture_catalog();
        catalog.fetched_at = "1970-01-01T00:00:00Z".to_string();
        catalog.source_endpoint = DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT.to_string();
        let output_path = root.join("catalog-export.json");
        let source = FixtureRefreshSource::ok(catalog);
        let mut request = refresh_request(&root);
        request.dry_run = true;
        request.output_path = Some(output_path.clone());

        let report = refresh_subnet_catalog_with_source(&request, &source).expect("dry-run");

        assert!(!report.wrote_catalog);
        assert!(!subnet_catalog_path(&request.cache.icp_root, MAINNET_NETWORK).exists());
        assert!(output_path.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn refresh_failure_preserves_existing_catalog_and_removes_lock() {
        let root = temp_dir("canic-subnet-host-refresh-failure");
        write_catalog(&root, fixture_catalog());
        let source = FixtureRefreshSource::err();
        let request = refresh_request(&root);

        let err = refresh_subnet_catalog_with_source(&request, &source).expect_err("refresh fails");
        let cached = load_cached_subnet_catalog(&cache_request(&root)).expect("cached catalog");
        let lock_path = subnet_catalog_refresh_lock_path(&root, MAINNET_NETWORK);

        std::assert_matches!(err, SubnetCatalogHostError::InvalidStaleDuration { .. });
        assert_eq!(cached.catalog.registry_version, 123_456);
        assert!(!lock_path.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn refresh_existing_fresh_lock_fails_fast() {
        let root = temp_dir("canic-subnet-host-refresh-locked");
        let request = refresh_request(&root);
        let lock_path = subnet_catalog_refresh_lock_path(&root, MAINNET_NETWORK);
        write_refresh_lock_for_test(&lock_path, &request, request.now_unix_secs * 1_000);

        let err = refresh_subnet_catalog_with_source(&request, &FixtureRefreshSource::err())
            .expect_err("lock held");

        let _ = fs::remove_dir_all(root);
        std::assert_matches!(err, SubnetCatalogHostError::RefreshAlreadyInProgress { .. });
    }

    #[test]
    fn refresh_removes_stale_lock_and_retries_once() {
        let root = temp_dir("canic-subnet-host-refresh-stale-lock");
        let mut catalog = fixture_catalog();
        catalog.fetched_at = "1970-01-01T00:00:00Z".to_string();
        catalog.source_endpoint = DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT.to_string();
        let source = FixtureRefreshSource::ok(catalog);
        let request = refresh_request(&root);
        let lock_path = subnet_catalog_refresh_lock_path(&root, MAINNET_NETWORK);
        let stale_started_at =
            (request.now_unix_secs - request.lock_stale_after_seconds - 1) * 1_000;
        write_refresh_lock_for_test(&lock_path, &request, stale_started_at);

        let report =
            refresh_subnet_catalog_with_source(&request, &source).expect("stale lock removed");

        assert!(report.wrote_catalog);
        assert!(!lock_path.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn utc_timestamp_formatter_is_deterministic() {
        assert_eq!(format_utc_timestamp_secs(0), "1970-01-01T00:00:00Z");
        assert_eq!(
            format_utc_timestamp_secs(1_780_531_200),
            "2026-06-04T00:00:00Z"
        );
    }

    fn list_request(root: &Path) -> SubnetCatalogListRequest {
        SubnetCatalogListRequest {
            cache: cache_request(root),
            now_unix_secs: 1_780_531_300,
            stale_after_seconds: DEFAULT_STALE_AFTER_SECONDS,
            filters: SubnetCatalogFilters::default(),
            show_ranges: true,
            range_limit: 1,
            range_offset: 0,
        }
    }

    fn info_request(root: &Path, input: &str) -> SubnetCatalogInfoRequest {
        SubnetCatalogInfoRequest {
            cache: cache_request(root),
            input: input.to_string(),
            forced: None,
            resolved_target: None,
            now_unix_secs: 1_780_531_300,
            stale_after_seconds: DEFAULT_STALE_AFTER_SECONDS,
        }
    }

    fn cache_request(root: &Path) -> SubnetCatalogCacheRequest {
        SubnetCatalogCacheRequest {
            icp_root: root.to_path_buf(),
            network: MAINNET_NETWORK.to_string(),
        }
    }

    fn write_catalog(root: &Path, catalog: SubnetCatalog) {
        let path = subnet_catalog_path(root, MAINNET_NETWORK);
        fs::create_dir_all(path.parent().expect("catalog parent")).expect("create parent");
        fs::write(
            path,
            serde_json::to_vec_pretty(&catalog).expect("serialize catalog"),
        )
        .expect("write catalog");
    }

    fn refresh_request(root: &Path) -> SubnetCatalogRefreshRequest {
        SubnetCatalogRefreshRequest {
            cache: cache_request(root),
            source_endpoint: DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT.to_string(),
            now_unix_secs: 1_780_531_200,
            lock_stale_after_seconds: DEFAULT_REFRESH_LOCK_STALE_SECONDS,
            dry_run: false,
            output_path: None,
        }
    }

    fn write_refresh_lock_for_test(
        lock_path: &Path,
        request: &SubnetCatalogRefreshRequest,
        started_at_unix_ms: u64,
    ) {
        fs::create_dir_all(lock_path.parent().expect("lock parent")).expect("create parent");
        let lock = RefreshLockFile {
            schema_version: 1,
            network: request.cache.network.clone(),
            pid: 12345,
            started_at_unix_ms,
            catalog_path: subnet_catalog_path(&request.cache.icp_root, &request.cache.network)
                .display()
                .to_string(),
        };
        fs::write(
            lock_path,
            serde_json::to_vec_pretty(&lock).expect("serialize lock"),
        )
        .expect("write lock");
    }

    ///
    /// FixtureRefreshSource
    ///
    struct FixtureRefreshSource {
        catalog: Option<SubnetCatalog>,
        fail: bool,
    }

    impl FixtureRefreshSource {
        fn ok(catalog: SubnetCatalog) -> Self {
            Self {
                catalog: Some(catalog),
                fail: false,
            }
        }

        fn err() -> Self {
            Self {
                catalog: None,
                fail: true,
            }
        }
    }

    impl SubnetCatalogRefreshSource for FixtureRefreshSource {
        fn fetch_catalog(
            &self,
            _request: &MainnetRegistryFetchRequest,
        ) -> Result<SubnetCatalog, SubnetCatalogHostError> {
            if self.fail {
                return Err(SubnetCatalogHostError::InvalidStaleDuration {
                    value: "fixture".to_string(),
                });
            }
            Ok(self.catalog.clone().expect("fixture catalog"))
        }
    }

    fn fixture_catalog() -> SubnetCatalog {
        SubnetCatalog {
            catalog_schema_version: CATALOG_SCHEMA_VERSION,
            network: MAINNET_NETWORK.to_string(),
            registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
            registry_version: 123_456,
            fetched_at: "2026-06-04T00:00:00Z".to_string(),
            fetched_by: "fixture".to_string(),
            source_endpoint: "https://icp-api.io".to_string(),
            resolver_backend: "local-nns-subnet-catalog".to_string(),
            subnets: vec![
                SubnetInfo {
                    subnet_principal: SUBNET_A.to_string(),
                    subnet_kind: SubnetKind::Application,
                    subnet_kind_source: ClassificationSource::Registry,
                    subnet_specialization: SubnetSpecialization::Fiduciary,
                    subnet_specialization_source: ClassificationSource::Curated,
                    geographic_scope: GeographicScope::Global,
                    geographic_scope_source: ClassificationSource::Curated,
                    subnet_label: "fiduciary".to_string(),
                    subnet_label_source: ClassificationSource::Curated,
                    node_count: Some(34),
                    charges_apply_by_default: true,
                },
                SubnetInfo {
                    subnet_principal: SUBNET_B.to_string(),
                    subnet_kind: SubnetKind::System,
                    subnet_kind_source: ClassificationSource::Registry,
                    subnet_specialization: SubnetSpecialization::None,
                    subnet_specialization_source: ClassificationSource::Curated,
                    geographic_scope: GeographicScope::Global,
                    geographic_scope_source: ClassificationSource::Curated,
                    subnet_label: "system".to_string(),
                    subnet_label_source: ClassificationSource::Curated,
                    node_count: Some(13),
                    charges_apply_by_default: false,
                },
            ],
            routing_ranges: vec![
                RoutingRange {
                    start_canister_id: CANISTER_A.to_string(),
                    end_canister_id: CANISTER_A.to_string(),
                    subnet_principal: SUBNET_A.to_string(),
                },
                RoutingRange {
                    start_canister_id: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
                    end_canister_id: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
                    subnet_principal: SUBNET_A.to_string(),
                },
                RoutingRange {
                    start_canister_id: "r7inp-6aaaa-aaaaa-aaabq-cai".to_string(),
                    end_canister_id: "r7inp-6aaaa-aaaaa-aaabq-cai".to_string(),
                    subnet_principal: SUBNET_B.to_string(),
                },
            ],
        }
    }
}
