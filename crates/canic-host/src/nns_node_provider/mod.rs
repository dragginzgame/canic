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
    DEFAULT_MAINNET_ENDPOINT, MainnetNodeProviderList, MainnetRegistryFetchRequest,
    RegistryFetchError, fetch_mainnet_node_provider_list,
};
use canic_subnet_catalog::{MAINNET_NETWORK, canonical_principal_text};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

pub const DEFAULT_NNS_SOURCE_ENDPOINT: &str = DEFAULT_MAINNET_ENDPOINT;
pub const DEFAULT_NODE_PROVIDER_REFRESH_LOCK_STALE_SECONDS: u64 = 30 * 60;
pub const NNS_NODE_PROVIDER_LIST_REPORT_SCHEMA_VERSION: u32 = 1;
pub const NNS_NODE_PROVIDER_INFO_REPORT_SCHEMA_VERSION: u32 = 1;
pub const NNS_NODE_PROVIDER_REFRESH_REPORT_SCHEMA_VERSION: u32 = 1;
const COMPACT_PRINCIPAL_CHARS: usize = 5;

///
/// NnsNodeProviderCacheRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsNodeProviderCacheRequest {
    pub icp_root: PathBuf,
    pub network: String,
}

///
/// NnsNodeProviderListRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsNodeProviderListRequest {
    pub cache: NnsNodeProviderCacheRequest,
    pub source_endpoint: String,
    pub now_unix_secs: u64,
}

///
/// NnsNodeProviderInfoRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsNodeProviderInfoRequest {
    pub cache: NnsNodeProviderCacheRequest,
    pub source_endpoint: String,
    pub input: String,
    pub now_unix_secs: u64,
}

///
/// NnsNodeProviderRefreshRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsNodeProviderRefreshRequest {
    pub cache: NnsNodeProviderCacheRequest,
    pub source_endpoint: String,
    pub now_unix_secs: u64,
    pub lock_stale_after_seconds: u64,
    pub dry_run: bool,
    pub output_path: Option<PathBuf>,
}

///
/// CachedNnsNodeProviderReport
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachedNnsNodeProviderReport {
    pub path: PathBuf,
    pub report: NnsNodeProviderListReport,
}

///
/// NnsNodeProviderListReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsNodeProviderListReport {
    pub schema_version: u32,
    pub network: String,
    pub governance_canister_id: String,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub source_endpoint: String,
    pub fetched_by: String,
    pub node_provider_count: usize,
    pub node_providers: Vec<NnsNodeProviderRow>,
}

impl JsonCacheReport for NnsNodeProviderListReport {
    fn schema_version(&self) -> u32 {
        self.schema_version
    }

    fn network(&self) -> &str {
        &self.network
    }
}

///
/// NnsNodeProviderRow
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsNodeProviderRow {
    pub node_provider_principal: String,
    pub name: Option<String>,
    pub node_count: Option<u32>,
    pub reward_account_hex: Option<String>,
}

///
/// NnsNodeProviderInfoReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsNodeProviderInfoReport {
    pub schema_version: u32,
    pub input: String,
    pub resolved_from: String,
    pub network: String,
    pub governance_canister_id: String,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub source_endpoint: String,
    pub fetched_by: String,
    pub node_provider_principal: String,
    pub name: Option<String>,
    pub node_count: Option<u32>,
    pub reward_account_hex: Option<String>,
}

///
/// NnsNodeProviderRefreshReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsNodeProviderRefreshReport {
    pub schema_version: u32,
    pub network: String,
    pub cache_path: String,
    pub refresh_lock_path: String,
    pub output_path: Option<String>,
    pub governance_canister_id: String,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub source_endpoint: String,
    pub fetched_by: String,
    pub dry_run: bool,
    pub wrote_cache: bool,
    pub replaced_existing_cache: bool,
    pub node_provider_count: usize,
}

///
/// NnsNodeProviderHostError
///
#[derive(Debug, ThisError)]
pub enum NnsNodeProviderHostError {
    #[error(
        "`canic nns node-provider` supports only the mainnet `ic` network in 0.60\n\nThe NNS node-provider list is queried from the public Internet Computer mainnet governance canister.\nLocal replica NNS governance discovery is not implemented yet.\n\nTry:\n  canic --network ic nns node-provider list"
    )]
    UnsupportedNetwork { network: String },

    #[error("node-provider cache is missing at {}", path.display())]
    MissingCache { path: PathBuf },

    #[error("failed to read node-provider cache at {}: {source}", path.display())]
    ReadCache { path: PathBuf, source: io::Error },

    #[error("failed to parse node-provider cache at {}: {source}", path.display())]
    ParseCache {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("failed to serialize node-provider cache JSON for {}: {source}", path.display())]
    SerializeCache {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("unsupported node-provider cache schema version {version}; expected {expected}")]
    UnsupportedCacheSchemaVersion { version: u32, expected: u32 },

    #[error(
        "cached node-provider network mismatch: path is for {requested}, report is for {actual}"
    )]
    NetworkMismatch { requested: String, actual: String },

    #[error("node-provider refresh is already in progress; lock exists at {} since unix_ms={started_at_unix_ms}", path.display())]
    RefreshAlreadyInProgress {
        path: PathBuf,
        started_at_unix_ms: u64,
    },

    #[error("failed to create node-provider cache directory at {}: {source}", path.display())]
    CreateCacheDirectory { path: PathBuf, source: io::Error },

    #[error("failed to create node-provider refresh lock at {}: {source}", path.display())]
    CreateRefreshLock { path: PathBuf, source: io::Error },

    #[error("failed to read node-provider refresh lock at {}: {source}", path.display())]
    ReadRefreshLock { path: PathBuf, source: io::Error },

    #[error("failed to parse node-provider refresh lock at {}: {source}", path.display())]
    ParseRefreshLock {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("failed to write node-provider refresh lock at {}: {source}", path.display())]
    WriteRefreshLock { path: PathBuf, source: io::Error },

    #[error("failed to remove node-provider refresh lock at {}: {source}", path.display())]
    RemoveRefreshLock { path: PathBuf, source: io::Error },

    #[error("live NNS node-provider refresh failed: {0}")]
    NnsQuery(#[from] RegistryFetchError),

    #[error("failed to write node-provider cache temp file at {}: {source}", path.display())]
    WriteCacheTemp { path: PathBuf, source: io::Error },

    #[error("failed to sync node-provider cache temp file at {}: {source}", path.display())]
    SyncCacheTemp { path: PathBuf, source: io::Error },

    #[error("failed to replace node-provider cache at {} from {}: {source}", cache_path.display(), temp_path.display())]
    ReplaceCache {
        temp_path: PathBuf,
        cache_path: PathBuf,
        source: io::Error,
    },

    #[error("failed to sync node-provider cache directory at {}: {source}", path.display())]
    SyncCacheDirectory { path: PathBuf, source: io::Error },

    #[error("failed to write refreshed node-provider output at {}: {source}", path.display())]
    WriteRefreshOutput { path: PathBuf, source: io::Error },

    #[error("failed to sync refreshed node-provider output at {}: {source}", path.display())]
    SyncRefreshOutput { path: PathBuf, source: io::Error },

    #[error("node provider {input:?} did not match the mainnet NNS node-provider list")]
    NodeProviderNotFound { input: String },

    #[error("node-provider prefix {prefix:?} is ambiguous; matches: {matches:?}")]
    AmbiguousNodeProviderPrefix {
        prefix: String,
        matches: Vec<String>,
    },
}

#[must_use]
pub fn nns_node_provider_cache_path(icp_root: &Path, network: &str) -> PathBuf {
    icp_root
        .join(".canic")
        .join("node-provider")
        .join(network)
        .join("providers.json")
}

#[must_use]
pub fn nns_node_provider_refresh_lock_path(icp_root: &Path, network: &str) -> PathBuf {
    icp_root
        .join(".canic")
        .join("node-provider")
        .join(network)
        .join("refresh.lock")
}

pub fn load_cached_nns_node_provider_report(
    request: &NnsNodeProviderCacheRequest,
) -> Result<CachedNnsNodeProviderReport, NnsNodeProviderHostError> {
    enforce_mainnet_network(&request.network)?;
    let path = nns_node_provider_cache_path(&request.icp_root, &request.network);
    let cached = load_json_cache(
        LoadJsonCacheRequest {
            path,
            network: &request.network,
            expected_schema_version: NNS_NODE_PROVIDER_LIST_REPORT_SCHEMA_VERSION,
        },
        LoadJsonCacheErrorHandlers {
            missing_cache: |path| NnsNodeProviderHostError::MissingCache { path },
            read_cache: |path, source| NnsNodeProviderHostError::ReadCache { path, source },
            parse_cache: |path, source| NnsNodeProviderHostError::ParseCache { path, source },
            unsupported_schema: |version, expected| {
                NnsNodeProviderHostError::UnsupportedCacheSchemaVersion { version, expected }
            },
            network_mismatch: |requested, actual| NnsNodeProviderHostError::NetworkMismatch {
                requested,
                actual,
            },
        },
    )?;
    Ok(CachedNnsNodeProviderReport {
        path: cached.path,
        report: cached.report,
    })
}

pub fn build_nns_node_provider_list_report(
    request: &NnsNodeProviderListRequest,
) -> Result<NnsNodeProviderListReport, NnsNodeProviderHostError> {
    build_nns_node_provider_list_report_with_source(request, &LiveNnsNodeProviderSource)
}

pub fn build_nns_node_provider_info_report(
    request: &NnsNodeProviderInfoRequest,
) -> Result<NnsNodeProviderInfoReport, NnsNodeProviderHostError> {
    build_nns_node_provider_info_report_with_source(request, &LiveNnsNodeProviderSource)
}

pub fn refresh_nns_node_provider_report(
    request: &NnsNodeProviderRefreshRequest,
) -> Result<NnsNodeProviderRefreshReport, NnsNodeProviderHostError> {
    refresh_nns_node_provider_report_with_source(request, &LiveNnsNodeProviderSource)
}

fn build_nns_node_provider_list_report_with_source(
    request: &NnsNodeProviderListRequest,
    source: &dyn NnsNodeProviderSource,
) -> Result<NnsNodeProviderListReport, NnsNodeProviderHostError> {
    match load_cached_nns_node_provider_report(&request.cache) {
        Ok(cached) => Ok(cached.report),
        Err(NnsNodeProviderHostError::MissingCache { .. }) => {
            let refresh_request = NnsNodeProviderRefreshRequest {
                cache: request.cache.clone(),
                source_endpoint: request.source_endpoint.clone(),
                now_unix_secs: request.now_unix_secs,
                lock_stale_after_seconds: DEFAULT_NODE_PROVIDER_REFRESH_LOCK_STALE_SECONDS,
                dry_run: false,
                output_path: None,
            };
            let (report, _) =
                refresh_nns_node_provider_cache_with_source(&refresh_request, source)?;
            Ok(report)
        }
        Err(err) => Err(err),
    }
}

fn build_nns_node_provider_info_report_with_source(
    request: &NnsNodeProviderInfoRequest,
    source: &dyn NnsNodeProviderSource,
) -> Result<NnsNodeProviderInfoReport, NnsNodeProviderHostError> {
    let list_request = NnsNodeProviderListRequest {
        cache: request.cache.clone(),
        source_endpoint: request.source_endpoint.clone(),
        now_unix_secs: request.now_unix_secs,
    };
    let report = build_nns_node_provider_list_report_with_source(&list_request, source)?;
    let (provider, resolved_from) = resolve_node_provider(&report, &request.input)?;
    Ok(NnsNodeProviderInfoReport {
        schema_version: NNS_NODE_PROVIDER_INFO_REPORT_SCHEMA_VERSION,
        input: request.input.clone(),
        resolved_from,
        network: report.network,
        governance_canister_id: report.governance_canister_id,
        registry_canister_id: report.registry_canister_id,
        registry_version: report.registry_version,
        fetched_at: report.fetched_at,
        source_endpoint: report.source_endpoint,
        fetched_by: report.fetched_by,
        node_provider_principal: provider.node_provider_principal,
        name: provider.name,
        node_count: provider.node_count,
        reward_account_hex: provider.reward_account_hex,
    })
}

fn refresh_nns_node_provider_report_with_source(
    request: &NnsNodeProviderRefreshRequest,
    source: &dyn NnsNodeProviderSource,
) -> Result<NnsNodeProviderRefreshReport, NnsNodeProviderHostError> {
    refresh_nns_node_provider_cache_with_source(request, source).map(|(_, report)| report)
}

fn refresh_nns_node_provider_cache_with_source(
    request: &NnsNodeProviderRefreshRequest,
    source: &dyn NnsNodeProviderSource,
) -> Result<(NnsNodeProviderListReport, NnsNodeProviderRefreshReport), NnsNodeProviderHostError> {
    enforce_mainnet_network(&request.cache.network)?;
    let cache_path = nns_node_provider_cache_path(&request.cache.icp_root, &request.cache.network);
    let lock_path =
        nns_node_provider_refresh_lock_path(&request.cache.icp_root, &request.cache.network);
    let report = fetch_nns_node_provider_list_report_with_source(
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
        node_provider_cache_error,
        |path, source| NnsNodeProviderHostError::SerializeCache { path, source },
    )?;
    let refresh_report = NnsNodeProviderRefreshReport {
        schema_version: NNS_NODE_PROVIDER_REFRESH_REPORT_SCHEMA_VERSION,
        network: report.network.clone(),
        cache_path: write_result.cache_path,
        refresh_lock_path: write_result.refresh_lock_path,
        output_path: write_result.output_path,
        governance_canister_id: report.governance_canister_id.clone(),
        registry_canister_id: report.registry_canister_id.clone(),
        registry_version: report.registry_version,
        fetched_at: report.fetched_at.clone(),
        source_endpoint: report.source_endpoint.clone(),
        fetched_by: report.fetched_by.clone(),
        dry_run: request.dry_run,
        wrote_cache: write_result.wrote_cache,
        replaced_existing_cache: write_result.replaced_existing_cache,
        node_provider_count: report.node_provider_count,
    };
    Ok((report, refresh_report))
}

fn fetch_nns_node_provider_list_report_with_source(
    network: &str,
    source_endpoint: &str,
    now_unix_secs: u64,
    source: &dyn NnsNodeProviderSource,
) -> Result<NnsNodeProviderListReport, NnsNodeProviderHostError> {
    enforce_mainnet_network(network)?;
    let fetched_at = format_utc_timestamp_secs(now_unix_secs);
    let mut fetch_request = MainnetRegistryFetchRequest::new(fetched_at);
    fetch_request.endpoint = source_endpoint.to_string();
    let list = source.fetch_node_providers(&fetch_request)?;
    Ok(node_provider_report_from_list(list))
}

fn node_provider_cache_error(err: CacheFileError) -> NnsNodeProviderHostError {
    match err {
        CacheFileError::CreateDirectory { path, source } => {
            NnsNodeProviderHostError::CreateCacheDirectory { path, source }
        }
        CacheFileError::CreateRefreshLock { path, source } => {
            NnsNodeProviderHostError::CreateRefreshLock { path, source }
        }
        CacheFileError::ReadRefreshLock { path, source } => {
            NnsNodeProviderHostError::ReadRefreshLock { path, source }
        }
        CacheFileError::ParseRefreshLock { path, source } => {
            NnsNodeProviderHostError::ParseRefreshLock { path, source }
        }
        CacheFileError::WriteRefreshLock { path, source } => {
            NnsNodeProviderHostError::WriteRefreshLock { path, source }
        }
        CacheFileError::RemoveRefreshLock { path, source } => {
            NnsNodeProviderHostError::RemoveRefreshLock { path, source }
        }
        CacheFileError::RefreshAlreadyInProgress {
            path,
            started_at_unix_ms,
        } => NnsNodeProviderHostError::RefreshAlreadyInProgress {
            path,
            started_at_unix_ms,
        },
        CacheFileError::WriteTemp { path, source } => {
            NnsNodeProviderHostError::WriteCacheTemp { path, source }
        }
        CacheFileError::SyncTemp { path, source } => {
            NnsNodeProviderHostError::SyncCacheTemp { path, source }
        }
        CacheFileError::Replace {
            temp_path,
            target_path,
            source,
        } => NnsNodeProviderHostError::ReplaceCache {
            temp_path,
            cache_path: target_path,
            source,
        },
        CacheFileError::SyncDirectory { path, source } => {
            NnsNodeProviderHostError::SyncCacheDirectory { path, source }
        }
        CacheFileError::WriteOutput { path, source } => {
            NnsNodeProviderHostError::WriteRefreshOutput { path, source }
        }
        CacheFileError::SyncOutput { path, source } => {
            NnsNodeProviderHostError::SyncRefreshOutput { path, source }
        }
    }
}

#[must_use]
pub fn nns_node_provider_list_report_text(report: &NnsNodeProviderListReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "node_providers: {} count {} fetched_at {}",
        report.network, report.node_provider_count, report.fetched_at
    ));
    if report.node_providers.is_empty() {
        lines.push("node providers: none".to_string());
        return lines.join("\n");
    }

    let headers = ["NODE_PROVIDER", "NODES"];
    let rows = report
        .node_providers
        .iter()
        .map(|provider| {
            [
                compact_text(&provider.node_provider_principal, COMPACT_PRINCIPAL_CHARS),
                optional_node_count_text(provider.node_count),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [ColumnAlign::Left, ColumnAlign::Right];
    lines.push(render_table(&headers, &rows, &alignments));
    lines.join("\n")
}

#[must_use]
pub fn nns_node_provider_list_report_verbose_text(report: &NnsNodeProviderListReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!("source_endpoint: {}", report.source_endpoint));
    lines.push(format!("fetched_by: {}", report.fetched_by));
    if report.node_providers.is_empty() {
        lines.push("node providers: none".to_string());
        return lines.join("\n");
    }

    let headers = [
        "NODE_PROVIDER",
        "NODES",
        "REWARD_ACCOUNT",
        "REGISTRY_VERSION",
        "FETCHED_AT",
    ];
    let rows = report
        .node_providers
        .iter()
        .map(|provider| {
            [
                provider.node_provider_principal.clone(),
                optional_node_count_text(provider.node_count),
                text_or_dash(provider.reward_account_hex.as_deref()).to_string(),
                report.registry_version.to_string(),
                report.fetched_at.clone(),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Left,
    ];
    lines.push(render_table(&headers, &rows, &alignments));
    lines.join("\n")
}

#[must_use]
pub fn nns_node_provider_info_report_text(report: &NnsNodeProviderInfoReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!("input: {}", report.input));
    lines.push(format!("resolved_from: {}", report.resolved_from));
    lines.push(format!(
        "node_provider_principal: {}",
        report.node_provider_principal
    ));
    lines.push(format!(
        "node_count: {}",
        optional_node_count_text(report.node_count)
    ));
    lines.push(format!(
        "reward_account_hex: {}",
        text_or_dash(report.reward_account_hex.as_deref())
    ));
    lines.push(format!(
        "governance_canister_id: {}",
        report.governance_canister_id
    ));
    lines.push(format!(
        "registry_canister_id: {}",
        report.registry_canister_id
    ));
    lines.push(format!("registry_version: {}", report.registry_version));
    lines.push(format!("network: {}", report.network));
    lines.push(format!("fetched_at: {}", report.fetched_at));
    lines.push(format!("source_endpoint: {}", report.source_endpoint));
    lines.push(format!("fetched_by: {}", report.fetched_by));
    lines.join("\n")
}

#[must_use]
pub fn nns_node_provider_refresh_report_text(report: &NnsNodeProviderRefreshReport) -> String {
    [
        format!("network: {}", report.network),
        format!("cache_path: {}", report.cache_path),
        format!("refresh_lock_path: {}", report.refresh_lock_path),
        format!("governance_canister_id: {}", report.governance_canister_id),
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
        format!("node_provider_count: {}", report.node_provider_count),
    ]
    .join("\n")
}

fn node_provider_report_from_list(list: MainnetNodeProviderList) -> NnsNodeProviderListReport {
    let node_providers = list
        .node_providers
        .into_iter()
        .map(|provider| NnsNodeProviderRow {
            node_provider_principal: provider.principal,
            name: None,
            node_count: provider.node_count,
            reward_account_hex: provider.reward_account_hex,
        })
        .collect::<Vec<_>>();
    NnsNodeProviderListReport {
        schema_version: NNS_NODE_PROVIDER_LIST_REPORT_SCHEMA_VERSION,
        network: list.network,
        governance_canister_id: list.governance_canister_id,
        registry_canister_id: list.registry_canister_id,
        registry_version: list.registry_version,
        fetched_at: list.fetched_at,
        source_endpoint: list.source_endpoint,
        fetched_by: list.fetched_by,
        node_provider_count: node_providers.len(),
        node_providers,
    }
}

///
/// NnsNodeProviderSource
///
trait NnsNodeProviderSource {
    fn fetch_node_providers(
        &self,
        request: &MainnetRegistryFetchRequest,
    ) -> Result<MainnetNodeProviderList, NnsNodeProviderHostError>;
}

fn enforce_mainnet_network(network: &str) -> Result<(), NnsNodeProviderHostError> {
    if network == MAINNET_NETWORK {
        return Ok(());
    }
    Err(NnsNodeProviderHostError::UnsupportedNetwork {
        network: network.to_string(),
    })
}

///
/// LiveNnsNodeProviderSource
///
struct LiveNnsNodeProviderSource;

impl NnsNodeProviderSource for LiveNnsNodeProviderSource {
    fn fetch_node_providers(
        &self,
        request: &MainnetRegistryFetchRequest,
    ) -> Result<MainnetNodeProviderList, NnsNodeProviderHostError> {
        Ok(fetch_mainnet_node_provider_list(request)?)
    }
}

fn resolve_node_provider(
    report: &NnsNodeProviderListReport,
    input: &str,
) -> Result<(NnsNodeProviderRow, String), NnsNodeProviderHostError> {
    if let Ok(principal) = canonical_principal_text(input)
        && let Some(provider) = report
            .node_providers
            .iter()
            .find(|provider| provider.node_provider_principal == principal)
    {
        return Ok((provider.clone(), "node_provider_principal".to_string()));
    }

    let prefix = input.trim().to_ascii_lowercase();
    if prefix.is_empty() {
        return Err(NnsNodeProviderHostError::NodeProviderNotFound {
            input: input.to_string(),
        });
    }
    let matches = report
        .node_providers
        .iter()
        .filter(|provider| provider.node_provider_principal.starts_with(&prefix))
        .cloned()
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [provider] => Ok((
            provider.clone(),
            "node_provider_principal_prefix".to_string(),
        )),
        [] => Err(NnsNodeProviderHostError::NodeProviderNotFound {
            input: input.to_string(),
        }),
        _ => Err(NnsNodeProviderHostError::AmbiguousNodeProviderPrefix {
            prefix,
            matches: matches
                .into_iter()
                .map(|provider| provider.node_provider_principal)
                .collect(),
        }),
    }
}

#[cfg(test)]
mod tests;
