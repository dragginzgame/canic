use crate::{
    cache_file::{
        CacheFileError, JsonCacheReport, LoadJsonCacheErrorHandlers, LoadJsonCacheRequest,
        RefreshCacheWriteRequest, load_json_cache, write_json_refresh_cache,
    },
    nns_render::{compact_text, text_or_dash, yes_no},
    subnet_catalog::format_utc_timestamp_secs,
    table::{ColumnAlign, render_table},
};
use canic_ic_registry::{
    DEFAULT_MAINNET_ENDPOINT, MainnetNodeList, MainnetRegistryFetchRequest, RegistryFetchError,
    fetch_mainnet_node_list,
};
use canic_subnet_catalog::{MAINNET_NETWORK, canonical_principal_text};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

pub const DEFAULT_NNS_NODE_SOURCE_ENDPOINT: &str = DEFAULT_MAINNET_ENDPOINT;
pub const DEFAULT_NODE_REFRESH_LOCK_STALE_SECONDS: u64 = 30 * 60;
pub const NNS_NODE_LIST_REPORT_SCHEMA_VERSION: u32 = 1;
pub const NNS_NODE_INFO_REPORT_SCHEMA_VERSION: u32 = 1;
pub const NNS_NODE_REFRESH_REPORT_SCHEMA_VERSION: u32 = 1;
pub const NNS_NODE_SUBNET_KIND_APPLICATION: &str = "application";
pub const NNS_NODE_SUBNET_KIND_SYSTEM: &str = "system";
pub const NNS_NODE_SUBNET_KIND_UNKNOWN: &str = "unknown";
const COMPACT_PRINCIPAL_CHARS: usize = 5;

///
/// NnsNodeCacheRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsNodeCacheRequest {
    pub icp_root: PathBuf,
    pub network: String,
}

///
/// NnsNodeListRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsNodeListRequest {
    pub cache: NnsNodeCacheRequest,
    pub source_endpoint: String,
    pub now_unix_secs: u64,
    pub filters: NnsNodeListFilters,
}

///
/// NnsNodeInfoRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsNodeInfoRequest {
    pub cache: NnsNodeCacheRequest,
    pub source_endpoint: String,
    pub input: String,
    pub now_unix_secs: u64,
}

///
/// NnsNodeRefreshRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsNodeRefreshRequest {
    pub cache: NnsNodeCacheRequest,
    pub source_endpoint: String,
    pub now_unix_secs: u64,
    pub lock_stale_after_seconds: u64,
    pub dry_run: bool,
    pub output_path: Option<PathBuf>,
}

///
/// NnsNodeListFilters
///
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NnsNodeListFilters {
    pub subnet: Option<String>,
    pub subnet_kind: Option<String>,
    pub data_center: Option<String>,
    pub node_provider: Option<String>,
    pub node_operator: Option<String>,
}

impl NnsNodeListFilters {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.subnet.is_none()
            && self.subnet_kind.is_none()
            && self.data_center.is_none()
            && self.node_provider.is_none()
            && self.node_operator.is_none()
    }
}

///
/// CachedNnsNodeReport
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachedNnsNodeReport {
    pub path: PathBuf,
    pub report: NnsNodeListReport,
}

///
/// NnsNodeListReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsNodeListReport {
    pub schema_version: u32,
    pub network: String,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub source_endpoint: String,
    pub fetched_by: String,
    pub node_count: usize,
    pub nodes: Vec<NnsNodeRow>,
}

impl JsonCacheReport for NnsNodeListReport {
    fn schema_version(&self) -> u32 {
        self.schema_version
    }

    fn network(&self) -> &str {
        &self.network
    }
}

///
/// NnsNodeRow
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsNodeRow {
    pub node_principal: String,
    pub node_operator_principal: String,
    pub node_provider_principal: String,
    pub subnet_principal: String,
    pub subnet_kind: String,
    pub data_center_id: String,
}

///
/// NnsNodeInfoReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsNodeInfoReport {
    pub schema_version: u32,
    pub input: String,
    pub resolved_from: String,
    pub network: String,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub source_endpoint: String,
    pub fetched_by: String,
    pub node_principal: String,
    pub node_operator_principal: String,
    pub node_provider_principal: String,
    pub subnet_principal: String,
    pub subnet_kind: String,
    pub data_center_id: String,
}

///
/// NnsNodeRefreshReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsNodeRefreshReport {
    pub schema_version: u32,
    pub network: String,
    pub cache_path: String,
    pub refresh_lock_path: String,
    pub output_path: Option<String>,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub source_endpoint: String,
    pub fetched_by: String,
    pub dry_run: bool,
    pub wrote_cache: bool,
    pub replaced_existing_cache: bool,
    pub node_count: usize,
}

///
/// NnsNodeHostError
///
#[derive(Debug, ThisError)]
pub enum NnsNodeHostError {
    #[error(
        "`canic nns node` supports only the mainnet `ic` network\n\nThe NNS node list is derived from public Internet Computer mainnet registry records.\nLocal replica NNS registry discovery is not implemented yet.\n\nTry:\n  canic --network ic nns node list"
    )]
    UnsupportedNetwork { network: String },

    #[error("node cache is missing at {}", path.display())]
    MissingCache { path: PathBuf },

    #[error("failed to read node cache at {}: {source}", path.display())]
    ReadCache { path: PathBuf, source: io::Error },

    #[error("failed to parse node cache at {}: {source}", path.display())]
    ParseCache {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("failed to serialize node cache JSON for {}: {source}", path.display())]
    SerializeCache {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("unsupported node cache schema version {version}; expected {expected}")]
    UnsupportedCacheSchemaVersion { version: u32, expected: u32 },

    #[error("cached node network mismatch: path is for {requested}, report is for {actual}")]
    NetworkMismatch { requested: String, actual: String },

    #[error("node refresh is already in progress; lock exists at {} since unix_ms={started_at_unix_ms}", path.display())]
    RefreshAlreadyInProgress {
        path: PathBuf,
        started_at_unix_ms: u64,
    },

    #[error("failed to create node cache directory at {}: {source}", path.display())]
    CreateCacheDirectory { path: PathBuf, source: io::Error },

    #[error("failed to create node refresh lock at {}: {source}", path.display())]
    CreateRefreshLock { path: PathBuf, source: io::Error },

    #[error("failed to read node refresh lock at {}: {source}", path.display())]
    ReadRefreshLock { path: PathBuf, source: io::Error },

    #[error("failed to parse node refresh lock at {}: {source}", path.display())]
    ParseRefreshLock {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("failed to write node refresh lock at {}: {source}", path.display())]
    WriteRefreshLock { path: PathBuf, source: io::Error },

    #[error("failed to remove node refresh lock at {}: {source}", path.display())]
    RemoveRefreshLock { path: PathBuf, source: io::Error },

    #[error("live NNS node refresh failed: {0}")]
    NnsQuery(#[from] RegistryFetchError),

    #[error("failed to write node cache temp file at {}: {source}", path.display())]
    WriteCacheTemp { path: PathBuf, source: io::Error },

    #[error("failed to sync node cache temp file at {}: {source}", path.display())]
    SyncCacheTemp { path: PathBuf, source: io::Error },

    #[error("failed to replace node cache at {} from {}: {source}", cache_path.display(), temp_path.display())]
    ReplaceCache {
        temp_path: PathBuf,
        cache_path: PathBuf,
        source: io::Error,
    },

    #[error("failed to sync node cache directory at {}: {source}", path.display())]
    SyncCacheDirectory { path: PathBuf, source: io::Error },

    #[error("failed to write refreshed node output at {}: {source}", path.display())]
    WriteRefreshOutput { path: PathBuf, source: io::Error },

    #[error("failed to sync refreshed node output at {}: {source}", path.display())]
    SyncRefreshOutput { path: PathBuf, source: io::Error },

    #[error("node {input:?} did not match the mainnet NNS node list")]
    NodeNotFound { input: String },

    #[error("node prefix {prefix:?} is ambiguous; matches: {matches:?}")]
    AmbiguousNodePrefix {
        prefix: String,
        matches: Vec<String>,
    },
}

#[must_use]
pub fn nns_node_cache_path(icp_root: &Path, network: &str) -> PathBuf {
    icp_root
        .join(".canic")
        .join("node")
        .join(network)
        .join("nodes.json")
}

#[must_use]
pub fn nns_node_refresh_lock_path(icp_root: &Path, network: &str) -> PathBuf {
    icp_root
        .join(".canic")
        .join("node")
        .join(network)
        .join("refresh.lock")
}

pub fn build_nns_node_list_report(
    request: &NnsNodeListRequest,
) -> Result<NnsNodeListReport, NnsNodeHostError> {
    build_nns_node_list_report_with_source(request, &LiveNnsNodeSource)
}

pub fn build_nns_node_info_report(
    request: &NnsNodeInfoRequest,
) -> Result<NnsNodeInfoReport, NnsNodeHostError> {
    build_nns_node_info_report_with_source(request, &LiveNnsNodeSource)
}

pub fn refresh_nns_node_report(
    request: &NnsNodeRefreshRequest,
) -> Result<NnsNodeRefreshReport, NnsNodeHostError> {
    refresh_nns_node_report_with_source(request, &LiveNnsNodeSource)
}

fn load_cached_nns_node_report(
    request: &NnsNodeCacheRequest,
) -> Result<CachedNnsNodeReport, NnsNodeHostError> {
    enforce_mainnet_network(&request.network)?;
    let path = nns_node_cache_path(&request.icp_root, &request.network);
    let cached = load_json_cache(
        LoadJsonCacheRequest {
            path,
            network: &request.network,
            expected_schema_version: NNS_NODE_LIST_REPORT_SCHEMA_VERSION,
        },
        LoadJsonCacheErrorHandlers {
            missing_cache: |path| NnsNodeHostError::MissingCache { path },
            read_cache: |path, source| NnsNodeHostError::ReadCache { path, source },
            parse_cache: |path, source| NnsNodeHostError::ParseCache { path, source },
            unsupported_schema: |version, expected| {
                NnsNodeHostError::UnsupportedCacheSchemaVersion { version, expected }
            },
            network_mismatch: |requested, actual| NnsNodeHostError::NetworkMismatch {
                requested,
                actual,
            },
        },
    )?;
    Ok(CachedNnsNodeReport {
        path: cached.path,
        report: cached.report,
    })
}

fn build_nns_node_list_report_with_source(
    request: &NnsNodeListRequest,
    source: &dyn NnsNodeSource,
) -> Result<NnsNodeListReport, NnsNodeHostError> {
    let report = match load_cached_nns_node_report(&request.cache) {
        Ok(cached) => cached.report,
        Err(NnsNodeHostError::MissingCache { .. }) => {
            let refresh_request = NnsNodeRefreshRequest {
                cache: request.cache.clone(),
                source_endpoint: request.source_endpoint.clone(),
                now_unix_secs: request.now_unix_secs,
                lock_stale_after_seconds: DEFAULT_NODE_REFRESH_LOCK_STALE_SECONDS,
                dry_run: false,
                output_path: None,
            };
            let (report, _) = refresh_nns_node_cache_with_source(&refresh_request, source)?;
            report
        }
        Err(err) => return Err(err),
    };
    Ok(filter_node_list_report(report, &request.filters))
}

fn build_nns_node_info_report_with_source(
    request: &NnsNodeInfoRequest,
    source: &dyn NnsNodeSource,
) -> Result<NnsNodeInfoReport, NnsNodeHostError> {
    let list_request = NnsNodeListRequest {
        cache: request.cache.clone(),
        source_endpoint: request.source_endpoint.clone(),
        now_unix_secs: request.now_unix_secs,
        filters: NnsNodeListFilters::default(),
    };
    let report = build_nns_node_list_report_with_source(&list_request, source)?;
    let (node, resolved_from) = resolve_node(&report, &request.input)?;
    Ok(NnsNodeInfoReport {
        schema_version: NNS_NODE_INFO_REPORT_SCHEMA_VERSION,
        input: request.input.clone(),
        resolved_from,
        network: report.network,
        registry_canister_id: report.registry_canister_id,
        registry_version: report.registry_version,
        fetched_at: report.fetched_at,
        source_endpoint: report.source_endpoint,
        fetched_by: report.fetched_by,
        node_principal: node.node_principal,
        node_operator_principal: node.node_operator_principal,
        node_provider_principal: node.node_provider_principal,
        subnet_principal: node.subnet_principal,
        subnet_kind: node.subnet_kind,
        data_center_id: node.data_center_id,
    })
}

fn refresh_nns_node_report_with_source(
    request: &NnsNodeRefreshRequest,
    source: &dyn NnsNodeSource,
) -> Result<NnsNodeRefreshReport, NnsNodeHostError> {
    refresh_nns_node_cache_with_source(request, source).map(|(_, report)| report)
}

fn refresh_nns_node_cache_with_source(
    request: &NnsNodeRefreshRequest,
    source: &dyn NnsNodeSource,
) -> Result<(NnsNodeListReport, NnsNodeRefreshReport), NnsNodeHostError> {
    enforce_mainnet_network(&request.cache.network)?;
    let cache_path = nns_node_cache_path(&request.cache.icp_root, &request.cache.network);
    let lock_path = nns_node_refresh_lock_path(&request.cache.icp_root, &request.cache.network);
    let report = fetch_nns_node_list_report_with_source(
        &request.cache.network,
        &request.source_endpoint,
        request.now_unix_secs,
        source,
    )?;
    let write_result = write_json_refresh_cache(
        RefreshCacheWriteRequest {
            cache_path: &cache_path,
            lock_path: &lock_path,
            network: &request.cache.network,
            now_unix_secs: request.now_unix_secs,
            lock_stale_after_seconds: request.lock_stale_after_seconds,
            dry_run: request.dry_run,
            output_path: request.output_path.as_deref(),
            report: &report,
        },
        node_cache_error,
        |path, source| NnsNodeHostError::SerializeCache { path, source },
    )?;
    let refresh_report = NnsNodeRefreshReport {
        schema_version: NNS_NODE_REFRESH_REPORT_SCHEMA_VERSION,
        network: report.network.clone(),
        cache_path: write_result.cache_path,
        refresh_lock_path: write_result.refresh_lock_path,
        output_path: write_result.output_path,
        registry_canister_id: report.registry_canister_id.clone(),
        registry_version: report.registry_version,
        fetched_at: report.fetched_at.clone(),
        source_endpoint: report.source_endpoint.clone(),
        fetched_by: report.fetched_by.clone(),
        dry_run: request.dry_run,
        wrote_cache: write_result.wrote_cache,
        replaced_existing_cache: write_result.replaced_existing_cache,
        node_count: report.node_count,
    };
    Ok((report, refresh_report))
}

fn fetch_nns_node_list_report_with_source(
    network: &str,
    source_endpoint: &str,
    now_unix_secs: u64,
    source: &dyn NnsNodeSource,
) -> Result<NnsNodeListReport, NnsNodeHostError> {
    enforce_mainnet_network(network)?;
    let fetched_at = format_utc_timestamp_secs(now_unix_secs);
    let mut fetch_request = MainnetRegistryFetchRequest::new(fetched_at);
    fetch_request.endpoint = source_endpoint.to_string();
    let list = source.fetch_nodes(&fetch_request)?;
    Ok(node_report_from_list(list))
}

fn node_cache_error(err: CacheFileError) -> NnsNodeHostError {
    match err {
        CacheFileError::CreateDirectory { path, source } => {
            NnsNodeHostError::CreateCacheDirectory { path, source }
        }
        CacheFileError::CreateRefreshLock { path, source } => {
            NnsNodeHostError::CreateRefreshLock { path, source }
        }
        CacheFileError::ReadRefreshLock { path, source } => {
            NnsNodeHostError::ReadRefreshLock { path, source }
        }
        CacheFileError::ParseRefreshLock { path, source } => {
            NnsNodeHostError::ParseRefreshLock { path, source }
        }
        CacheFileError::WriteRefreshLock { path, source } => {
            NnsNodeHostError::WriteRefreshLock { path, source }
        }
        CacheFileError::RemoveRefreshLock { path, source } => {
            NnsNodeHostError::RemoveRefreshLock { path, source }
        }
        CacheFileError::RefreshAlreadyInProgress {
            path,
            started_at_unix_ms,
        } => NnsNodeHostError::RefreshAlreadyInProgress {
            path,
            started_at_unix_ms,
        },
        CacheFileError::WriteTemp { path, source } => {
            NnsNodeHostError::WriteCacheTemp { path, source }
        }
        CacheFileError::SyncTemp { path, source } => {
            NnsNodeHostError::SyncCacheTemp { path, source }
        }
        CacheFileError::Replace {
            temp_path,
            target_path,
            source,
        } => NnsNodeHostError::ReplaceCache {
            temp_path,
            cache_path: target_path,
            source,
        },
        CacheFileError::SyncDirectory { path, source } => {
            NnsNodeHostError::SyncCacheDirectory { path, source }
        }
        CacheFileError::WriteOutput { path, source } => {
            NnsNodeHostError::WriteRefreshOutput { path, source }
        }
        CacheFileError::SyncOutput { path, source } => {
            NnsNodeHostError::SyncRefreshOutput { path, source }
        }
    }
}

#[must_use]
pub fn nns_node_list_report_text(report: &NnsNodeListReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "nodes: {} count {} fetched_at {}",
        report.network, report.node_count, report.fetched_at
    ));
    if report.nodes.is_empty() {
        lines.push("nodes: none".to_string());
        return lines.join("\n");
    }
    let headers = ["NODE", "OPERATOR", "PROVIDER", "SUBNET", "KIND", "DC"];
    let rows = report
        .nodes
        .iter()
        .map(|node| {
            [
                compact_text(&node.node_principal, COMPACT_PRINCIPAL_CHARS),
                compact_text(&node.node_operator_principal, COMPACT_PRINCIPAL_CHARS),
                compact_text(&node.node_provider_principal, COMPACT_PRINCIPAL_CHARS),
                compact_text(&node.subnet_principal, COMPACT_PRINCIPAL_CHARS),
                node.subnet_kind.clone(),
                text_or_dash(Some(&node.data_center_id)).to_string(),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
    ];
    lines.push(render_table(&headers, &rows, &alignments));
    lines.join("\n")
}

#[must_use]
pub fn nns_node_list_report_verbose_text(report: &NnsNodeListReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!("source_endpoint: {}", report.source_endpoint));
    lines.push(format!("fetched_by: {}", report.fetched_by));
    if report.nodes.is_empty() {
        lines.push("nodes: none".to_string());
        return lines.join("\n");
    }
    let headers = [
        "NODE",
        "OPERATOR",
        "PROVIDER",
        "SUBNET",
        "KIND",
        "DC",
        "REGISTRY_VERSION",
        "FETCHED_AT",
    ];
    let rows = report
        .nodes
        .iter()
        .map(|node| {
            [
                node.node_principal.clone(),
                node.node_operator_principal.clone(),
                node.node_provider_principal.clone(),
                node.subnet_principal.clone(),
                node.subnet_kind.clone(),
                text_or_dash(Some(&node.data_center_id)).to_string(),
                report.registry_version.to_string(),
                report.fetched_at.clone(),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Left,
    ];
    lines.push(render_table(&headers, &rows, &alignments));
    lines.join("\n")
}

#[must_use]
pub fn nns_node_info_report_text(report: &NnsNodeInfoReport) -> String {
    [
        format!("input: {}", report.input),
        format!("resolved_from: {}", report.resolved_from),
        format!("node_principal: {}", report.node_principal),
        format!(
            "node_operator_principal: {}",
            report.node_operator_principal
        ),
        format!(
            "node_provider_principal: {}",
            report.node_provider_principal
        ),
        format!("subnet_principal: {}", report.subnet_principal),
        format!("subnet_kind: {}", report.subnet_kind),
        format!(
            "data_center_id: {}",
            text_or_dash(Some(&report.data_center_id))
        ),
        format!("registry_canister_id: {}", report.registry_canister_id),
        format!("registry_version: {}", report.registry_version),
        format!("network: {}", report.network),
        format!("fetched_at: {}", report.fetched_at),
        format!("source_endpoint: {}", report.source_endpoint),
        format!("fetched_by: {}", report.fetched_by),
    ]
    .join("\n")
}

#[must_use]
pub fn nns_node_refresh_report_text(report: &NnsNodeRefreshReport) -> String {
    [
        format!("network: {}", report.network),
        format!("cache_path: {}", report.cache_path),
        format!("refresh_lock_path: {}", report.refresh_lock_path),
        format!("registry_canister_id: {}", report.registry_canister_id),
        format!("registry_version: {}", report.registry_version),
        format!("fetched_at: {}", report.fetched_at),
        format!("source_endpoint: {}", report.source_endpoint),
        format!("fetched_by: {}", report.fetched_by),
        format!("dry_run: {}", yes_no(report.dry_run)),
        format!("wrote_cache: {}", yes_no(report.wrote_cache)),
        format!(
            "replaced_existing_cache: {}",
            yes_no(report.replaced_existing_cache)
        ),
        format!("node_count: {}", report.node_count),
    ]
    .join("\n")
}

fn node_report_from_list(list: MainnetNodeList) -> NnsNodeListReport {
    let nodes = list
        .nodes
        .into_iter()
        .map(|node| NnsNodeRow {
            node_principal: node.principal,
            node_operator_principal: node.node_operator_principal,
            node_provider_principal: node.node_provider_principal,
            subnet_principal: node.subnet_principal,
            subnet_kind: node.subnet_kind,
            data_center_id: node.data_center_id,
        })
        .collect::<Vec<_>>();
    NnsNodeListReport {
        schema_version: NNS_NODE_LIST_REPORT_SCHEMA_VERSION,
        network: list.network,
        registry_canister_id: list.registry_canister_id,
        registry_version: list.registry_version,
        fetched_at: list.fetched_at,
        source_endpoint: list.source_endpoint,
        fetched_by: list.fetched_by,
        node_count: nodes.len(),
        nodes,
    }
}

fn filter_node_list_report(
    mut report: NnsNodeListReport,
    filters: &NnsNodeListFilters,
) -> NnsNodeListReport {
    if filters.is_empty() {
        return report;
    }
    report
        .nodes
        .retain(|node| node_matches_filters(node, filters));
    report.node_count = report.nodes.len();
    report
}

fn node_matches_filters(node: &NnsNodeRow, filters: &NnsNodeListFilters) -> bool {
    filters
        .subnet
        .as_deref()
        .is_none_or(|filter| principal_filter_matches(&node.subnet_principal, filter))
        && filters
            .subnet_kind
            .as_deref()
            .is_none_or(|filter| text_filter_equals(&node.subnet_kind, filter))
        && filters
            .data_center
            .as_deref()
            .is_none_or(|filter| text_filter_starts_with(&node.data_center_id, filter))
        && filters
            .node_provider
            .as_deref()
            .is_none_or(|filter| principal_filter_matches(&node.node_provider_principal, filter))
        && filters
            .node_operator
            .as_deref()
            .is_none_or(|filter| principal_filter_matches(&node.node_operator_principal, filter))
}

fn principal_filter_matches(value: &str, filter: &str) -> bool {
    let Some(filter) = non_empty_filter(filter) else {
        return false;
    };
    if let Ok(principal) = canonical_principal_text(filter) {
        value == principal
    } else {
        value.starts_with(&filter.to_ascii_lowercase())
    }
}

fn text_filter_starts_with(value: &str, filter: &str) -> bool {
    let Some(filter) = non_empty_filter(filter) else {
        return false;
    };
    value
        .to_ascii_lowercase()
        .starts_with(&filter.to_ascii_lowercase())
}

fn text_filter_equals(value: &str, filter: &str) -> bool {
    let Some(filter) = non_empty_filter(filter) else {
        return false;
    };
    value.eq_ignore_ascii_case(filter)
}

fn non_empty_filter(filter: &str) -> Option<&str> {
    let filter = filter.trim();
    (!filter.is_empty()).then_some(filter)
}

///
/// NnsNodeSource
///
trait NnsNodeSource {
    fn fetch_nodes(
        &self,
        request: &MainnetRegistryFetchRequest,
    ) -> Result<MainnetNodeList, NnsNodeHostError>;
}

///
/// LiveNnsNodeSource
///
struct LiveNnsNodeSource;

impl NnsNodeSource for LiveNnsNodeSource {
    fn fetch_nodes(
        &self,
        request: &MainnetRegistryFetchRequest,
    ) -> Result<MainnetNodeList, NnsNodeHostError> {
        Ok(fetch_mainnet_node_list(request)?)
    }
}

fn enforce_mainnet_network(network: &str) -> Result<(), NnsNodeHostError> {
    if network == MAINNET_NETWORK {
        return Ok(());
    }
    Err(NnsNodeHostError::UnsupportedNetwork {
        network: network.to_string(),
    })
}

fn resolve_node(
    report: &NnsNodeListReport,
    input: &str,
) -> Result<(NnsNodeRow, String), NnsNodeHostError> {
    if let Ok(principal) = canonical_principal_text(input)
        && let Some(node) = report
            .nodes
            .iter()
            .find(|node| node.node_principal == principal)
    {
        return Ok((node.clone(), "node_principal".to_string()));
    }

    let prefix = input.trim().to_ascii_lowercase();
    if prefix.is_empty() {
        return Err(NnsNodeHostError::NodeNotFound {
            input: input.to_string(),
        });
    }
    let matches = report
        .nodes
        .iter()
        .filter(|node| node.node_principal.starts_with(&prefix))
        .cloned()
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [node] => Ok((node.clone(), "node_principal_prefix".to_string())),
        [] => Err(NnsNodeHostError::NodeNotFound {
            input: input.to_string(),
        }),
        _ => Err(NnsNodeHostError::AmbiguousNodePrefix {
            prefix,
            matches: matches
                .into_iter()
                .map(|node| node.node_principal)
                .collect(),
        }),
    }
}

#[cfg(test)]
mod tests;
