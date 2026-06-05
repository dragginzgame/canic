use crate::{
    cache_file::{
        CacheFileError, JsonCacheReport, LoadJsonCacheErrorHandlers, LoadJsonCacheRequest,
        RefreshCacheWriteRequest, load_json_cache, write_json_refresh_cache,
    },
    nns_render::{compact_text, optional_node_count_text, text_or_dash, yes_no},
    subnet_catalog::format_utc_timestamp_secs,
    table::{ColumnAlign, render_table},
};
use canic_ic_registry::{
    DEFAULT_MAINNET_ENDPOINT, MainnetNodeOperatorList, MainnetRegistryFetchRequest,
    RegistryFetchError, fetch_mainnet_node_operator_list,
};
use canic_subnet_catalog::{MAINNET_NETWORK, canonical_principal_text};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

pub const DEFAULT_NNS_NODE_OPERATOR_SOURCE_ENDPOINT: &str = DEFAULT_MAINNET_ENDPOINT;
pub const DEFAULT_NODE_OPERATOR_REFRESH_LOCK_STALE_SECONDS: u64 = 30 * 60;
pub const NNS_NODE_OPERATOR_LIST_REPORT_SCHEMA_VERSION: u32 = 1;
pub const NNS_NODE_OPERATOR_INFO_REPORT_SCHEMA_VERSION: u32 = 1;
pub const NNS_NODE_OPERATOR_REFRESH_REPORT_SCHEMA_VERSION: u32 = 1;
const COMPACT_PRINCIPAL_CHARS: usize = 5;

///
/// NnsNodeOperatorCacheRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsNodeOperatorCacheRequest {
    pub icp_root: PathBuf,
    pub network: String,
}

///
/// NnsNodeOperatorListRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsNodeOperatorListRequest {
    pub cache: NnsNodeOperatorCacheRequest,
    pub source_endpoint: String,
    pub now_unix_secs: u64,
}

///
/// NnsNodeOperatorInfoRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsNodeOperatorInfoRequest {
    pub cache: NnsNodeOperatorCacheRequest,
    pub source_endpoint: String,
    pub input: String,
    pub now_unix_secs: u64,
}

///
/// NnsNodeOperatorRefreshRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsNodeOperatorRefreshRequest {
    pub cache: NnsNodeOperatorCacheRequest,
    pub source_endpoint: String,
    pub now_unix_secs: u64,
    pub lock_stale_after_seconds: u64,
    pub dry_run: bool,
    pub output_path: Option<PathBuf>,
}

///
/// CachedNnsNodeOperatorReport
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachedNnsNodeOperatorReport {
    pub path: PathBuf,
    pub report: NnsNodeOperatorListReport,
}

///
/// NnsNodeOperatorListReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsNodeOperatorListReport {
    pub schema_version: u32,
    pub network: String,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub source_endpoint: String,
    pub fetched_by: String,
    pub node_operator_count: usize,
    pub node_operators: Vec<NnsNodeOperatorRow>,
}

impl JsonCacheReport for NnsNodeOperatorListReport {
    fn schema_version(&self) -> u32 {
        self.schema_version
    }

    fn network(&self) -> &str {
        &self.network
    }
}

///
/// NnsNodeOperatorRow
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsNodeOperatorRow {
    pub node_operator_principal: String,
    pub node_provider_principal: String,
    pub node_allowance: u64,
    pub data_center_id: String,
    pub node_count: Option<u32>,
}

///
/// NnsNodeOperatorInfoReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsNodeOperatorInfoReport {
    pub schema_version: u32,
    pub input: String,
    pub resolved_from: String,
    pub network: String,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub source_endpoint: String,
    pub fetched_by: String,
    pub node_operator_principal: String,
    pub node_provider_principal: String,
    pub node_allowance: u64,
    pub data_center_id: String,
    pub node_count: Option<u32>,
}

///
/// NnsNodeOperatorRefreshReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsNodeOperatorRefreshReport {
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
    pub node_operator_count: usize,
}

///
/// NnsNodeOperatorHostError
///
#[derive(Debug, ThisError)]
pub enum NnsNodeOperatorHostError {
    #[error(
        "`canic nns node-operator` supports only the mainnet `ic` network\n\nThe NNS node-operator list is derived from public Internet Computer mainnet registry records.\nLocal replica NNS registry discovery is not implemented yet.\n\nTry:\n  canic --network ic nns node-operator list"
    )]
    UnsupportedNetwork { network: String },

    #[error("node-operator cache is missing at {}", path.display())]
    MissingCache { path: PathBuf },

    #[error("failed to read node-operator cache at {}: {source}", path.display())]
    ReadCache { path: PathBuf, source: io::Error },

    #[error("failed to parse node-operator cache at {}: {source}", path.display())]
    ParseCache {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("failed to serialize node-operator cache JSON for {}: {source}", path.display())]
    SerializeCache {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("unsupported node-operator cache schema version {version}; expected {expected}")]
    UnsupportedCacheSchemaVersion { version: u32, expected: u32 },

    #[error(
        "cached node-operator network mismatch: path is for {requested}, report is for {actual}"
    )]
    NetworkMismatch { requested: String, actual: String },

    #[error("node-operator refresh is already in progress; lock exists at {} since unix_ms={started_at_unix_ms}", path.display())]
    RefreshAlreadyInProgress {
        path: PathBuf,
        started_at_unix_ms: u64,
    },

    #[error("failed to create node-operator cache directory at {}: {source}", path.display())]
    CreateCacheDirectory { path: PathBuf, source: io::Error },

    #[error("failed to create node-operator refresh lock at {}: {source}", path.display())]
    CreateRefreshLock { path: PathBuf, source: io::Error },

    #[error("failed to read node-operator refresh lock at {}: {source}", path.display())]
    ReadRefreshLock { path: PathBuf, source: io::Error },

    #[error("failed to parse node-operator refresh lock at {}: {source}", path.display())]
    ParseRefreshLock {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("failed to write node-operator refresh lock at {}: {source}", path.display())]
    WriteRefreshLock { path: PathBuf, source: io::Error },

    #[error("failed to remove node-operator refresh lock at {}: {source}", path.display())]
    RemoveRefreshLock { path: PathBuf, source: io::Error },

    #[error("live NNS node-operator refresh failed: {0}")]
    NnsQuery(#[from] RegistryFetchError),

    #[error("failed to write node-operator cache temp file at {}: {source}", path.display())]
    WriteCacheTemp { path: PathBuf, source: io::Error },

    #[error("failed to sync node-operator cache temp file at {}: {source}", path.display())]
    SyncCacheTemp { path: PathBuf, source: io::Error },

    #[error("failed to replace node-operator cache at {} from {}: {source}", cache_path.display(), temp_path.display())]
    ReplaceCache {
        temp_path: PathBuf,
        cache_path: PathBuf,
        source: io::Error,
    },

    #[error("failed to sync node-operator cache directory at {}: {source}", path.display())]
    SyncCacheDirectory { path: PathBuf, source: io::Error },

    #[error("failed to write refreshed node-operator output at {}: {source}", path.display())]
    WriteRefreshOutput { path: PathBuf, source: io::Error },

    #[error("failed to sync refreshed node-operator output at {}: {source}", path.display())]
    SyncRefreshOutput { path: PathBuf, source: io::Error },

    #[error("node operator {input:?} did not match the mainnet NNS node-operator list")]
    NodeOperatorNotFound { input: String },

    #[error("node-operator prefix {prefix:?} is ambiguous; matches: {matches:?}")]
    AmbiguousNodeOperatorPrefix {
        prefix: String,
        matches: Vec<String>,
    },
}

#[must_use]
pub fn nns_node_operator_cache_path(icp_root: &Path, network: &str) -> PathBuf {
    icp_root
        .join(".canic")
        .join("node-operator")
        .join(network)
        .join("operators.json")
}

#[must_use]
pub fn nns_node_operator_refresh_lock_path(icp_root: &Path, network: &str) -> PathBuf {
    icp_root
        .join(".canic")
        .join("node-operator")
        .join(network)
        .join("refresh.lock")
}

pub fn load_cached_nns_node_operator_report(
    request: &NnsNodeOperatorCacheRequest,
) -> Result<CachedNnsNodeOperatorReport, NnsNodeOperatorHostError> {
    enforce_mainnet_network(&request.network)?;
    let path = nns_node_operator_cache_path(&request.icp_root, &request.network);
    let cached = load_json_cache(
        LoadJsonCacheRequest {
            path,
            network: &request.network,
            expected_schema_version: NNS_NODE_OPERATOR_LIST_REPORT_SCHEMA_VERSION,
        },
        LoadJsonCacheErrorHandlers {
            missing_cache: |path| NnsNodeOperatorHostError::MissingCache { path },
            read_cache: |path, source| NnsNodeOperatorHostError::ReadCache { path, source },
            parse_cache: |path, source| NnsNodeOperatorHostError::ParseCache { path, source },
            unsupported_schema: |version, expected| {
                NnsNodeOperatorHostError::UnsupportedCacheSchemaVersion { version, expected }
            },
            network_mismatch: |requested, actual| NnsNodeOperatorHostError::NetworkMismatch {
                requested,
                actual,
            },
        },
    )?;
    Ok(CachedNnsNodeOperatorReport {
        path: cached.path,
        report: cached.report,
    })
}

pub fn build_nns_node_operator_list_report(
    request: &NnsNodeOperatorListRequest,
) -> Result<NnsNodeOperatorListReport, NnsNodeOperatorHostError> {
    build_nns_node_operator_list_report_with_source(request, &LiveNnsNodeOperatorSource)
}

pub fn build_nns_node_operator_info_report(
    request: &NnsNodeOperatorInfoRequest,
) -> Result<NnsNodeOperatorInfoReport, NnsNodeOperatorHostError> {
    build_nns_node_operator_info_report_with_source(request, &LiveNnsNodeOperatorSource)
}

pub fn refresh_nns_node_operator_report(
    request: &NnsNodeOperatorRefreshRequest,
) -> Result<NnsNodeOperatorRefreshReport, NnsNodeOperatorHostError> {
    refresh_nns_node_operator_report_with_source(request, &LiveNnsNodeOperatorSource)
}

fn build_nns_node_operator_list_report_with_source(
    request: &NnsNodeOperatorListRequest,
    source: &dyn NnsNodeOperatorSource,
) -> Result<NnsNodeOperatorListReport, NnsNodeOperatorHostError> {
    match load_cached_nns_node_operator_report(&request.cache) {
        Ok(cached) => Ok(cached.report),
        Err(NnsNodeOperatorHostError::MissingCache { .. }) => {
            let refresh_request = NnsNodeOperatorRefreshRequest {
                cache: request.cache.clone(),
                source_endpoint: request.source_endpoint.clone(),
                now_unix_secs: request.now_unix_secs,
                lock_stale_after_seconds: DEFAULT_NODE_OPERATOR_REFRESH_LOCK_STALE_SECONDS,
                dry_run: false,
                output_path: None,
            };
            let (report, _) =
                refresh_nns_node_operator_cache_with_source(&refresh_request, source)?;
            Ok(report)
        }
        Err(err) => Err(err),
    }
}

fn build_nns_node_operator_info_report_with_source(
    request: &NnsNodeOperatorInfoRequest,
    source: &dyn NnsNodeOperatorSource,
) -> Result<NnsNodeOperatorInfoReport, NnsNodeOperatorHostError> {
    let list_request = NnsNodeOperatorListRequest {
        cache: request.cache.clone(),
        source_endpoint: request.source_endpoint.clone(),
        now_unix_secs: request.now_unix_secs,
    };
    let report = build_nns_node_operator_list_report_with_source(&list_request, source)?;
    let (operator, resolved_from) = resolve_node_operator(&report, &request.input)?;
    Ok(NnsNodeOperatorInfoReport {
        schema_version: NNS_NODE_OPERATOR_INFO_REPORT_SCHEMA_VERSION,
        input: request.input.clone(),
        resolved_from,
        network: report.network,
        registry_canister_id: report.registry_canister_id,
        registry_version: report.registry_version,
        fetched_at: report.fetched_at,
        source_endpoint: report.source_endpoint,
        fetched_by: report.fetched_by,
        node_operator_principal: operator.node_operator_principal,
        node_provider_principal: operator.node_provider_principal,
        node_allowance: operator.node_allowance,
        data_center_id: operator.data_center_id,
        node_count: operator.node_count,
    })
}

fn refresh_nns_node_operator_report_with_source(
    request: &NnsNodeOperatorRefreshRequest,
    source: &dyn NnsNodeOperatorSource,
) -> Result<NnsNodeOperatorRefreshReport, NnsNodeOperatorHostError> {
    refresh_nns_node_operator_cache_with_source(request, source).map(|(_, report)| report)
}

fn refresh_nns_node_operator_cache_with_source(
    request: &NnsNodeOperatorRefreshRequest,
    source: &dyn NnsNodeOperatorSource,
) -> Result<(NnsNodeOperatorListReport, NnsNodeOperatorRefreshReport), NnsNodeOperatorHostError> {
    enforce_mainnet_network(&request.cache.network)?;
    let cache_path = nns_node_operator_cache_path(&request.cache.icp_root, &request.cache.network);
    let lock_path =
        nns_node_operator_refresh_lock_path(&request.cache.icp_root, &request.cache.network);
    let report = fetch_nns_node_operator_list_report_with_source(
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
        node_operator_cache_error,
        |path, source| NnsNodeOperatorHostError::SerializeCache { path, source },
    )?;
    let refresh_report = NnsNodeOperatorRefreshReport {
        schema_version: NNS_NODE_OPERATOR_REFRESH_REPORT_SCHEMA_VERSION,
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
        node_operator_count: report.node_operator_count,
    };
    Ok((report, refresh_report))
}

fn fetch_nns_node_operator_list_report_with_source(
    network: &str,
    source_endpoint: &str,
    now_unix_secs: u64,
    source: &dyn NnsNodeOperatorSource,
) -> Result<NnsNodeOperatorListReport, NnsNodeOperatorHostError> {
    enforce_mainnet_network(network)?;
    let fetched_at = format_utc_timestamp_secs(now_unix_secs);
    let mut fetch_request = MainnetRegistryFetchRequest::new(fetched_at);
    fetch_request.endpoint = source_endpoint.to_string();
    let list = source.fetch_node_operators(&fetch_request)?;
    Ok(node_operator_report_from_list(list))
}

fn node_operator_cache_error(err: CacheFileError) -> NnsNodeOperatorHostError {
    match err {
        CacheFileError::CreateDirectory { path, source } => {
            NnsNodeOperatorHostError::CreateCacheDirectory { path, source }
        }
        CacheFileError::CreateRefreshLock { path, source } => {
            NnsNodeOperatorHostError::CreateRefreshLock { path, source }
        }
        CacheFileError::ReadRefreshLock { path, source } => {
            NnsNodeOperatorHostError::ReadRefreshLock { path, source }
        }
        CacheFileError::ParseRefreshLock { path, source } => {
            NnsNodeOperatorHostError::ParseRefreshLock { path, source }
        }
        CacheFileError::WriteRefreshLock { path, source } => {
            NnsNodeOperatorHostError::WriteRefreshLock { path, source }
        }
        CacheFileError::RemoveRefreshLock { path, source } => {
            NnsNodeOperatorHostError::RemoveRefreshLock { path, source }
        }
        CacheFileError::RefreshAlreadyInProgress {
            path,
            started_at_unix_ms,
        } => NnsNodeOperatorHostError::RefreshAlreadyInProgress {
            path,
            started_at_unix_ms,
        },
        CacheFileError::WriteTemp { path, source } => {
            NnsNodeOperatorHostError::WriteCacheTemp { path, source }
        }
        CacheFileError::SyncTemp { path, source } => {
            NnsNodeOperatorHostError::SyncCacheTemp { path, source }
        }
        CacheFileError::Replace {
            temp_path,
            target_path,
            source,
        } => NnsNodeOperatorHostError::ReplaceCache {
            temp_path,
            cache_path: target_path,
            source,
        },
        CacheFileError::SyncDirectory { path, source } => {
            NnsNodeOperatorHostError::SyncCacheDirectory { path, source }
        }
        CacheFileError::WriteOutput { path, source } => {
            NnsNodeOperatorHostError::WriteRefreshOutput { path, source }
        }
        CacheFileError::SyncOutput { path, source } => {
            NnsNodeOperatorHostError::SyncRefreshOutput { path, source }
        }
    }
}

#[must_use]
pub fn nns_node_operator_list_report_text(report: &NnsNodeOperatorListReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "node_operators: {} count {} fetched_at {}",
        report.network, report.node_operator_count, report.fetched_at
    ));
    if report.node_operators.is_empty() {
        lines.push("node operators: none".to_string());
        return lines.join("\n");
    }

    let headers = ["NODE_OPERATOR", "PROVIDER", "NODES", "ALLOWANCE", "DC"];
    let rows = report
        .node_operators
        .iter()
        .map(|operator| {
            [
                compact_text(&operator.node_operator_principal, COMPACT_PRINCIPAL_CHARS),
                compact_text(&operator.node_provider_principal, COMPACT_PRINCIPAL_CHARS),
                optional_node_count_text(operator.node_count),
                operator.node_allowance.to_string(),
                text_or_dash(Some(&operator.data_center_id)).to_string(),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Left,
    ];
    lines.push(render_table(&headers, &rows, &alignments));
    lines.join("\n")
}

#[must_use]
pub fn nns_node_operator_list_report_verbose_text(report: &NnsNodeOperatorListReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!("source_endpoint: {}", report.source_endpoint));
    lines.push(format!("fetched_by: {}", report.fetched_by));
    if report.node_operators.is_empty() {
        lines.push("node operators: none".to_string());
        return lines.join("\n");
    }

    let headers = [
        "NODE_OPERATOR",
        "PROVIDER",
        "NODES",
        "ALLOWANCE",
        "DC",
        "REGISTRY_VERSION",
        "FETCHED_AT",
    ];
    let rows = report
        .node_operators
        .iter()
        .map(|operator| {
            [
                operator.node_operator_principal.clone(),
                operator.node_provider_principal.clone(),
                optional_node_count_text(operator.node_count),
                operator.node_allowance.to_string(),
                text_or_dash(Some(&operator.data_center_id)).to_string(),
                report.registry_version.to_string(),
                report.fetched_at.clone(),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Left,
    ];
    lines.push(render_table(&headers, &rows, &alignments));
    lines.join("\n")
}

#[must_use]
pub fn nns_node_operator_info_report_text(report: &NnsNodeOperatorInfoReport) -> String {
    [
        format!("input: {}", report.input),
        format!("resolved_from: {}", report.resolved_from),
        format!(
            "node_operator_principal: {}",
            report.node_operator_principal
        ),
        format!(
            "node_provider_principal: {}",
            report.node_provider_principal
        ),
        format!(
            "node_count: {}",
            optional_node_count_text(report.node_count)
        ),
        format!("node_allowance: {}", report.node_allowance),
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
pub fn nns_node_operator_refresh_report_text(report: &NnsNodeOperatorRefreshReport) -> String {
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
        format!("node_operator_count: {}", report.node_operator_count),
    ]
    .join("\n")
}

fn node_operator_report_from_list(list: MainnetNodeOperatorList) -> NnsNodeOperatorListReport {
    let node_operators = list
        .node_operators
        .into_iter()
        .map(|operator| NnsNodeOperatorRow {
            node_operator_principal: operator.principal,
            node_provider_principal: operator.node_provider_principal,
            node_allowance: operator.node_allowance,
            data_center_id: operator.data_center_id,
            node_count: operator.node_count,
        })
        .collect::<Vec<_>>();
    NnsNodeOperatorListReport {
        schema_version: NNS_NODE_OPERATOR_LIST_REPORT_SCHEMA_VERSION,
        network: list.network,
        registry_canister_id: list.registry_canister_id,
        registry_version: list.registry_version,
        fetched_at: list.fetched_at,
        source_endpoint: list.source_endpoint,
        fetched_by: list.fetched_by,
        node_operator_count: node_operators.len(),
        node_operators,
    }
}

///
/// NnsNodeOperatorSource
///
trait NnsNodeOperatorSource {
    fn fetch_node_operators(
        &self,
        request: &MainnetRegistryFetchRequest,
    ) -> Result<MainnetNodeOperatorList, NnsNodeOperatorHostError>;
}

fn enforce_mainnet_network(network: &str) -> Result<(), NnsNodeOperatorHostError> {
    if network == MAINNET_NETWORK {
        return Ok(());
    }
    Err(NnsNodeOperatorHostError::UnsupportedNetwork {
        network: network.to_string(),
    })
}

///
/// LiveNnsNodeOperatorSource
///
struct LiveNnsNodeOperatorSource;

impl NnsNodeOperatorSource for LiveNnsNodeOperatorSource {
    fn fetch_node_operators(
        &self,
        request: &MainnetRegistryFetchRequest,
    ) -> Result<MainnetNodeOperatorList, NnsNodeOperatorHostError> {
        Ok(fetch_mainnet_node_operator_list(request)?)
    }
}

fn resolve_node_operator(
    report: &NnsNodeOperatorListReport,
    input: &str,
) -> Result<(NnsNodeOperatorRow, String), NnsNodeOperatorHostError> {
    if let Ok(principal) = canonical_principal_text(input)
        && let Some(operator) = report
            .node_operators
            .iter()
            .find(|operator| operator.node_operator_principal == principal)
    {
        return Ok((operator.clone(), "node_operator_principal".to_string()));
    }

    let prefix = input.trim().to_ascii_lowercase();
    if prefix.is_empty() {
        return Err(NnsNodeOperatorHostError::NodeOperatorNotFound {
            input: input.to_string(),
        });
    }
    let matches = report
        .node_operators
        .iter()
        .filter(|operator| operator.node_operator_principal.starts_with(&prefix))
        .cloned()
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [operator] => Ok((
            operator.clone(),
            "node_operator_principal_prefix".to_string(),
        )),
        [] => Err(NnsNodeOperatorHostError::NodeOperatorNotFound {
            input: input.to_string(),
        }),
        _ => Err(NnsNodeOperatorHostError::AmbiguousNodeOperatorPrefix {
            prefix,
            matches: matches
                .into_iter()
                .map(|operator| operator.node_operator_principal)
                .collect(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_ic_registry::MainnetNodeOperator;
    use canic_subnet_catalog::MAINNET_REGISTRY_CANISTER_ID;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn node_operator_report_uses_live_registry_source() {
        let request = NnsNodeOperatorListRequest {
            cache: test_cache_request(MAINNET_NETWORK, "uses-live-source"),
            source_endpoint: "https://icp-api.io".to_string(),
            now_unix_secs: 1_780_531_200,
        };
        let report = build_nns_node_operator_list_report_with_source(
            &request,
            &FixtureNodeOperatorSource {
                node_operators: vec![MainnetNodeOperator {
                    principal: "aaaaa-aa".to_string(),
                    node_provider_principal: "ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
                    node_allowance: 4,
                    data_center_id: "dc1".to_string(),
                    node_count: Some(3),
                }],
            },
        )
        .expect("node operator report");

        assert_eq!(report.schema_version, 1);
        assert_eq!(report.network, MAINNET_NETWORK);
        assert_eq!(report.registry_canister_id, MAINNET_REGISTRY_CANISTER_ID);
        assert_eq!(report.registry_version, 42);
        assert_eq!(report.fetched_at, "2026-06-04T00:00:00Z");
        assert_eq!(report.node_operator_count, 1);
        assert_eq!(report.node_operators[0].node_operator_principal, "aaaaa-aa");
        assert_eq!(
            report.node_operators[0].node_provider_principal,
            "ryjl3-tyaaa-aaaaa-aaaba-cai"
        );
        assert_eq!(report.node_operators[0].node_allowance, 4);
        assert_eq!(report.node_operators[0].data_center_id, "dc1");
        assert_eq!(report.node_operators[0].node_count, Some(3));
    }

    #[test]
    fn node_operator_text_keeps_compact_principals() {
        let report = node_operator_report_fixture();

        let text = nns_node_operator_list_report_text(&report);

        assert!(text.contains("node_operators: ic count 1"));
        assert!(text.contains("NODE_OPERATOR"));
        assert!(text.contains("ryjl3"));
        assert!(text.contains("aaaaa"));
        assert!(text.contains("13"));
        assert!(!text.contains("ryjl3-tyaaa-aaaaa-aaaba-cai"));
    }

    #[test]
    fn node_operator_info_resolves_unique_prefix() {
        let report = node_operator_report_fixture();

        let (operator, resolved_from) =
            resolve_node_operator(&report, "ryjl").expect("prefix resolves");

        assert_eq!(resolved_from, "node_operator_principal_prefix");
        assert_eq!(
            operator.node_operator_principal,
            "ryjl3-tyaaa-aaaaa-aaaba-cai"
        );
    }

    fn node_operator_report_fixture() -> NnsNodeOperatorListReport {
        NnsNodeOperatorListReport {
            schema_version: 1,
            network: MAINNET_NETWORK.to_string(),
            registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
            registry_version: 42,
            fetched_at: "2026-06-04T00:00:00Z".to_string(),
            source_endpoint: "https://icp-api.io".to_string(),
            fetched_by: "test".to_string(),
            node_operator_count: 1,
            node_operators: vec![NnsNodeOperatorRow {
                node_operator_principal: "ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
                node_provider_principal: "aaaaa-aa".to_string(),
                node_allowance: 7,
                data_center_id: "dc1".to_string(),
                node_count: Some(13),
            }],
        }
    }

    fn test_cache_request(network: &str, name: &str) -> NnsNodeOperatorCacheRequest {
        let counter = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir()
            .join("canic-host-nns-node-operator-tests")
            .join(format!("{name}-{counter}"));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove old test root");
        }
        NnsNodeOperatorCacheRequest {
            icp_root: root,
            network: network.to_string(),
        }
    }

    ///
    /// FixtureNodeOperatorSource
    ///
    struct FixtureNodeOperatorSource {
        node_operators: Vec<MainnetNodeOperator>,
    }

    impl NnsNodeOperatorSource for FixtureNodeOperatorSource {
        fn fetch_node_operators(
            &self,
            request: &MainnetRegistryFetchRequest,
        ) -> Result<MainnetNodeOperatorList, NnsNodeOperatorHostError> {
            Ok(MainnetNodeOperatorList {
                network: MAINNET_NETWORK.to_string(),
                registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
                registry_version: 42,
                fetched_at: request.fetched_at.clone(),
                fetched_by: request.fetched_by.clone(),
                source_endpoint: request.endpoint.clone(),
                node_operators: self.node_operators.clone(),
            })
        }
    }
}
