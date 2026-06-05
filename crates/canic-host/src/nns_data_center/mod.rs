use crate::{
    cache_file::{
        CacheFileError, JsonCacheReport, LoadJsonCacheErrorHandlers, LoadJsonCacheRequest,
        RefreshCacheWriteRequest, load_json_cache, write_json_refresh_cache,
    },
    nns_render::{optional_f32_text, text_or_dash, yes_no},
    subnet_catalog::format_utc_timestamp_secs,
    table::{ColumnAlign, render_table},
};
use canic_ic_registry::{
    DEFAULT_MAINNET_ENDPOINT, MainnetDataCenterList, MainnetRegistryFetchRequest,
    RegistryFetchError, fetch_mainnet_data_center_list,
};
use canic_subnet_catalog::MAINNET_NETWORK;
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

pub const DEFAULT_NNS_DATA_CENTER_SOURCE_ENDPOINT: &str = DEFAULT_MAINNET_ENDPOINT;
pub const DEFAULT_DATA_CENTER_REFRESH_LOCK_STALE_SECONDS: u64 = 30 * 60;
pub const NNS_DATA_CENTER_LIST_REPORT_SCHEMA_VERSION: u32 = 1;
pub const NNS_DATA_CENTER_INFO_REPORT_SCHEMA_VERSION: u32 = 1;
pub const NNS_DATA_CENTER_REFRESH_REPORT_SCHEMA_VERSION: u32 = 1;

///
/// NnsDataCenterCacheRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsDataCenterCacheRequest {
    pub icp_root: PathBuf,
    pub network: String,
}

///
/// NnsDataCenterListRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsDataCenterListRequest {
    pub cache: NnsDataCenterCacheRequest,
    pub source_endpoint: String,
    pub now_unix_secs: u64,
}

///
/// NnsDataCenterInfoRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsDataCenterInfoRequest {
    pub cache: NnsDataCenterCacheRequest,
    pub source_endpoint: String,
    pub input: String,
    pub now_unix_secs: u64,
}

///
/// NnsDataCenterRefreshRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsDataCenterRefreshRequest {
    pub cache: NnsDataCenterCacheRequest,
    pub source_endpoint: String,
    pub now_unix_secs: u64,
    pub lock_stale_after_seconds: u64,
    pub dry_run: bool,
    pub output_path: Option<PathBuf>,
}

///
/// CachedNnsDataCenterReport
///
#[derive(Clone, Debug, PartialEq)]
pub struct CachedNnsDataCenterReport {
    pub path: PathBuf,
    pub report: NnsDataCenterListReport,
}

///
/// NnsDataCenterListReport
///
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NnsDataCenterListReport {
    pub schema_version: u32,
    pub network: String,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub source_endpoint: String,
    pub fetched_by: String,
    pub data_center_count: usize,
    pub data_centers: Vec<NnsDataCenterRow>,
}

impl JsonCacheReport for NnsDataCenterListReport {
    fn schema_version(&self) -> u32 {
        self.schema_version
    }

    fn network(&self) -> &str {
        &self.network
    }
}

///
/// NnsDataCenterRow
///
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NnsDataCenterRow {
    pub data_center_id: String,
    pub region: String,
    pub owner: String,
    pub latitude: Option<f32>,
    pub longitude: Option<f32>,
    pub node_operator_count: u32,
    pub node_provider_count: u32,
    pub node_count: u32,
}

///
/// NnsDataCenterInfoReport
///
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NnsDataCenterInfoReport {
    pub schema_version: u32,
    pub input: String,
    pub resolved_from: String,
    pub network: String,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub source_endpoint: String,
    pub fetched_by: String,
    pub data_center_id: String,
    pub region: String,
    pub owner: String,
    pub latitude: Option<f32>,
    pub longitude: Option<f32>,
    pub node_operator_count: u32,
    pub node_provider_count: u32,
    pub node_count: u32,
}

///
/// NnsDataCenterRefreshReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsDataCenterRefreshReport {
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
    pub data_center_count: usize,
}

///
/// NnsDataCenterHostError
///
#[derive(Debug, ThisError)]
pub enum NnsDataCenterHostError {
    #[error(
        "`canic nns data-center` supports only the mainnet `ic` network\n\nThe NNS data-center list is derived from public Internet Computer mainnet registry records.\nLocal replica NNS registry discovery is not implemented yet.\n\nTry:\n  canic --network ic nns data-center list"
    )]
    UnsupportedNetwork { network: String },

    #[error("data-center cache is missing at {}", path.display())]
    MissingCache { path: PathBuf },

    #[error("failed to read data-center cache at {}: {source}", path.display())]
    ReadCache { path: PathBuf, source: io::Error },

    #[error("failed to parse data-center cache at {}: {source}", path.display())]
    ParseCache {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("failed to serialize data-center cache JSON for {}: {source}", path.display())]
    SerializeCache {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("unsupported data-center cache schema version {version}; expected {expected}")]
    UnsupportedCacheSchemaVersion { version: u32, expected: u32 },

    #[error("cached data-center network mismatch: path is for {requested}, report is for {actual}")]
    NetworkMismatch { requested: String, actual: String },

    #[error("data-center refresh is already in progress; lock exists at {} since unix_ms={started_at_unix_ms}", path.display())]
    RefreshAlreadyInProgress {
        path: PathBuf,
        started_at_unix_ms: u64,
    },

    #[error("failed to create data-center cache directory at {}: {source}", path.display())]
    CreateCacheDirectory { path: PathBuf, source: io::Error },

    #[error("failed to create data-center refresh lock at {}: {source}", path.display())]
    CreateRefreshLock { path: PathBuf, source: io::Error },

    #[error("failed to read data-center refresh lock at {}: {source}", path.display())]
    ReadRefreshLock { path: PathBuf, source: io::Error },

    #[error("failed to parse data-center refresh lock at {}: {source}", path.display())]
    ParseRefreshLock {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("failed to write data-center refresh lock at {}: {source}", path.display())]
    WriteRefreshLock { path: PathBuf, source: io::Error },

    #[error("failed to remove data-center refresh lock at {}: {source}", path.display())]
    RemoveRefreshLock { path: PathBuf, source: io::Error },

    #[error("live NNS data-center refresh failed: {0}")]
    NnsQuery(#[from] RegistryFetchError),

    #[error("failed to write data-center cache temp file at {}: {source}", path.display())]
    WriteCacheTemp { path: PathBuf, source: io::Error },

    #[error("failed to sync data-center cache temp file at {}: {source}", path.display())]
    SyncCacheTemp { path: PathBuf, source: io::Error },

    #[error("failed to replace data-center cache at {} from {}: {source}", cache_path.display(), temp_path.display())]
    ReplaceCache {
        temp_path: PathBuf,
        cache_path: PathBuf,
        source: io::Error,
    },

    #[error("failed to sync data-center cache directory at {}: {source}", path.display())]
    SyncCacheDirectory { path: PathBuf, source: io::Error },

    #[error("failed to write refreshed data-center output at {}: {source}", path.display())]
    WriteRefreshOutput { path: PathBuf, source: io::Error },

    #[error("failed to sync refreshed data-center output at {}: {source}", path.display())]
    SyncRefreshOutput { path: PathBuf, source: io::Error },

    #[error("data center {input:?} did not match the mainnet NNS data-center list")]
    DataCenterNotFound { input: String },

    #[error("data-center prefix {prefix:?} is ambiguous; matches: {matches:?}")]
    AmbiguousDataCenterPrefix {
        prefix: String,
        matches: Vec<String>,
    },
}

#[must_use]
pub fn nns_data_center_cache_path(icp_root: &Path, network: &str) -> PathBuf {
    icp_root
        .join(".canic")
        .join("data-center")
        .join(network)
        .join("data-centers.json")
}

#[must_use]
pub fn nns_data_center_refresh_lock_path(icp_root: &Path, network: &str) -> PathBuf {
    icp_root
        .join(".canic")
        .join("data-center")
        .join(network)
        .join("refresh.lock")
}

pub fn build_nns_data_center_list_report(
    request: &NnsDataCenterListRequest,
) -> Result<NnsDataCenterListReport, NnsDataCenterHostError> {
    build_nns_data_center_list_report_with_source(request, &LiveNnsDataCenterSource)
}

pub fn build_nns_data_center_info_report(
    request: &NnsDataCenterInfoRequest,
) -> Result<NnsDataCenterInfoReport, NnsDataCenterHostError> {
    build_nns_data_center_info_report_with_source(request, &LiveNnsDataCenterSource)
}

pub fn refresh_nns_data_center_report(
    request: &NnsDataCenterRefreshRequest,
) -> Result<NnsDataCenterRefreshReport, NnsDataCenterHostError> {
    refresh_nns_data_center_report_with_source(request, &LiveNnsDataCenterSource)
}

fn load_cached_nns_data_center_report(
    request: &NnsDataCenterCacheRequest,
) -> Result<CachedNnsDataCenterReport, NnsDataCenterHostError> {
    enforce_mainnet_network(&request.network)?;
    let path = nns_data_center_cache_path(&request.icp_root, &request.network);
    let cached = load_json_cache(
        LoadJsonCacheRequest {
            path,
            network: &request.network,
            expected_schema_version: NNS_DATA_CENTER_LIST_REPORT_SCHEMA_VERSION,
        },
        LoadJsonCacheErrorHandlers {
            missing_cache: |path| NnsDataCenterHostError::MissingCache { path },
            read_cache: |path, source| NnsDataCenterHostError::ReadCache { path, source },
            parse_cache: |path, source| NnsDataCenterHostError::ParseCache { path, source },
            unsupported_schema: |version, expected| {
                NnsDataCenterHostError::UnsupportedCacheSchemaVersion { version, expected }
            },
            network_mismatch: |requested, actual| NnsDataCenterHostError::NetworkMismatch {
                requested,
                actual,
            },
        },
    )?;
    Ok(CachedNnsDataCenterReport {
        path: cached.path,
        report: cached.report,
    })
}

fn build_nns_data_center_list_report_with_source(
    request: &NnsDataCenterListRequest,
    source: &dyn NnsDataCenterSource,
) -> Result<NnsDataCenterListReport, NnsDataCenterHostError> {
    match load_cached_nns_data_center_report(&request.cache) {
        Ok(cached) => Ok(cached.report),
        Err(NnsDataCenterHostError::MissingCache { .. }) => {
            let refresh_request = NnsDataCenterRefreshRequest {
                cache: request.cache.clone(),
                source_endpoint: request.source_endpoint.clone(),
                now_unix_secs: request.now_unix_secs,
                lock_stale_after_seconds: DEFAULT_DATA_CENTER_REFRESH_LOCK_STALE_SECONDS,
                dry_run: false,
                output_path: None,
            };
            let (report, _) = refresh_nns_data_center_cache_with_source(&refresh_request, source)?;
            Ok(report)
        }
        Err(err) => Err(err),
    }
}

fn build_nns_data_center_info_report_with_source(
    request: &NnsDataCenterInfoRequest,
    source: &dyn NnsDataCenterSource,
) -> Result<NnsDataCenterInfoReport, NnsDataCenterHostError> {
    let list_request = NnsDataCenterListRequest {
        cache: request.cache.clone(),
        source_endpoint: request.source_endpoint.clone(),
        now_unix_secs: request.now_unix_secs,
    };
    let report = build_nns_data_center_list_report_with_source(&list_request, source)?;
    let (data_center, resolved_from) = resolve_data_center(&report, &request.input)?;
    Ok(NnsDataCenterInfoReport {
        schema_version: NNS_DATA_CENTER_INFO_REPORT_SCHEMA_VERSION,
        input: request.input.clone(),
        resolved_from,
        network: report.network,
        registry_canister_id: report.registry_canister_id,
        registry_version: report.registry_version,
        fetched_at: report.fetched_at,
        source_endpoint: report.source_endpoint,
        fetched_by: report.fetched_by,
        data_center_id: data_center.data_center_id,
        region: data_center.region,
        owner: data_center.owner,
        latitude: data_center.latitude,
        longitude: data_center.longitude,
        node_operator_count: data_center.node_operator_count,
        node_provider_count: data_center.node_provider_count,
        node_count: data_center.node_count,
    })
}

fn refresh_nns_data_center_report_with_source(
    request: &NnsDataCenterRefreshRequest,
    source: &dyn NnsDataCenterSource,
) -> Result<NnsDataCenterRefreshReport, NnsDataCenterHostError> {
    refresh_nns_data_center_cache_with_source(request, source).map(|(_, report)| report)
}

fn refresh_nns_data_center_cache_with_source(
    request: &NnsDataCenterRefreshRequest,
    source: &dyn NnsDataCenterSource,
) -> Result<(NnsDataCenterListReport, NnsDataCenterRefreshReport), NnsDataCenterHostError> {
    enforce_mainnet_network(&request.cache.network)?;
    let cache_path = nns_data_center_cache_path(&request.cache.icp_root, &request.cache.network);
    let lock_path =
        nns_data_center_refresh_lock_path(&request.cache.icp_root, &request.cache.network);
    let report = fetch_nns_data_center_list_report_with_source(
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
        data_center_cache_error,
        |path, source| NnsDataCenterHostError::SerializeCache { path, source },
    )?;
    let refresh_report = NnsDataCenterRefreshReport {
        schema_version: NNS_DATA_CENTER_REFRESH_REPORT_SCHEMA_VERSION,
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
        data_center_count: report.data_center_count,
    };
    Ok((report, refresh_report))
}

fn fetch_nns_data_center_list_report_with_source(
    network: &str,
    source_endpoint: &str,
    now_unix_secs: u64,
    source: &dyn NnsDataCenterSource,
) -> Result<NnsDataCenterListReport, NnsDataCenterHostError> {
    enforce_mainnet_network(network)?;
    let fetched_at = format_utc_timestamp_secs(now_unix_secs);
    let mut fetch_request = MainnetRegistryFetchRequest::new(fetched_at);
    fetch_request.endpoint = source_endpoint.to_string();
    let list = source.fetch_data_centers(&fetch_request)?;
    Ok(data_center_report_from_list(list))
}

fn data_center_cache_error(err: CacheFileError) -> NnsDataCenterHostError {
    match err {
        CacheFileError::CreateDirectory { path, source } => {
            NnsDataCenterHostError::CreateCacheDirectory { path, source }
        }
        CacheFileError::CreateRefreshLock { path, source } => {
            NnsDataCenterHostError::CreateRefreshLock { path, source }
        }
        CacheFileError::ReadRefreshLock { path, source } => {
            NnsDataCenterHostError::ReadRefreshLock { path, source }
        }
        CacheFileError::ParseRefreshLock { path, source } => {
            NnsDataCenterHostError::ParseRefreshLock { path, source }
        }
        CacheFileError::WriteRefreshLock { path, source } => {
            NnsDataCenterHostError::WriteRefreshLock { path, source }
        }
        CacheFileError::RemoveRefreshLock { path, source } => {
            NnsDataCenterHostError::RemoveRefreshLock { path, source }
        }
        CacheFileError::RefreshAlreadyInProgress {
            path,
            started_at_unix_ms,
        } => NnsDataCenterHostError::RefreshAlreadyInProgress {
            path,
            started_at_unix_ms,
        },
        CacheFileError::WriteTemp { path, source } => {
            NnsDataCenterHostError::WriteCacheTemp { path, source }
        }
        CacheFileError::SyncTemp { path, source } => {
            NnsDataCenterHostError::SyncCacheTemp { path, source }
        }
        CacheFileError::Replace {
            temp_path,
            target_path,
            source,
        } => NnsDataCenterHostError::ReplaceCache {
            temp_path,
            cache_path: target_path,
            source,
        },
        CacheFileError::SyncDirectory { path, source } => {
            NnsDataCenterHostError::SyncCacheDirectory { path, source }
        }
        CacheFileError::WriteOutput { path, source } => {
            NnsDataCenterHostError::WriteRefreshOutput { path, source }
        }
        CacheFileError::SyncOutput { path, source } => {
            NnsDataCenterHostError::SyncRefreshOutput { path, source }
        }
    }
}

#[must_use]
pub fn nns_data_center_list_report_text(report: &NnsDataCenterListReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "data_centers: {} count {} fetched_at {}",
        report.network, report.data_center_count, report.fetched_at
    ));
    if report.data_centers.is_empty() {
        lines.push("data_centers: none".to_string());
        return lines.join("\n");
    }
    let headers = ["DC", "REGION", "OWNER", "OPS", "PROVIDERS", "NODES"];
    let rows = report
        .data_centers
        .iter()
        .map(|data_center| {
            [
                data_center.data_center_id.clone(),
                text_or_dash(Some(&data_center.region)).to_string(),
                text_or_dash(Some(&data_center.owner)).to_string(),
                data_center.node_operator_count.to_string(),
                data_center.node_provider_count.to_string(),
                data_center.node_count.to_string(),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
    ];
    lines.push(render_table(&headers, &rows, &alignments));
    lines.join("\n")
}

#[must_use]
pub fn nns_data_center_list_report_verbose_text(report: &NnsDataCenterListReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!("source_endpoint: {}", report.source_endpoint));
    lines.push(format!("fetched_by: {}", report.fetched_by));
    if report.data_centers.is_empty() {
        lines.push("data_centers: none".to_string());
        return lines.join("\n");
    }
    let headers = [
        "DC",
        "REGION",
        "OWNER",
        "LATITUDE",
        "LONGITUDE",
        "OPS",
        "PROVIDERS",
        "NODES",
        "REGISTRY_VERSION",
        "FETCHED_AT",
    ];
    let rows = report
        .data_centers
        .iter()
        .map(|data_center| {
            [
                data_center.data_center_id.clone(),
                text_or_dash(Some(&data_center.region)).to_string(),
                text_or_dash(Some(&data_center.owner)).to_string(),
                optional_f32_text(data_center.latitude),
                optional_f32_text(data_center.longitude),
                data_center.node_operator_count.to_string(),
                data_center.node_provider_count.to_string(),
                data_center.node_count.to_string(),
                report.registry_version.to_string(),
                report.fetched_at.clone(),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Left,
    ];
    lines.push(render_table(&headers, &rows, &alignments));
    lines.join("\n")
}

#[must_use]
pub fn nns_data_center_info_report_text(report: &NnsDataCenterInfoReport) -> String {
    [
        format!("input: {}", report.input),
        format!("resolved_from: {}", report.resolved_from),
        format!("data_center_id: {}", report.data_center_id),
        format!("region: {}", text_or_dash(Some(&report.region))),
        format!("owner: {}", text_or_dash(Some(&report.owner))),
        format!("latitude: {}", optional_f32_text(report.latitude)),
        format!("longitude: {}", optional_f32_text(report.longitude)),
        format!("node_operator_count: {}", report.node_operator_count),
        format!("node_provider_count: {}", report.node_provider_count),
        format!("node_count: {}", report.node_count),
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
pub fn nns_data_center_refresh_report_text(report: &NnsDataCenterRefreshReport) -> String {
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
        format!("data_center_count: {}", report.data_center_count),
    ]
    .join("\n")
}

fn data_center_report_from_list(list: MainnetDataCenterList) -> NnsDataCenterListReport {
    let data_centers = list
        .data_centers
        .into_iter()
        .map(|data_center| NnsDataCenterRow {
            data_center_id: data_center.id,
            region: data_center.region,
            owner: data_center.owner,
            latitude: data_center.latitude,
            longitude: data_center.longitude,
            node_operator_count: data_center.node_operator_count,
            node_provider_count: data_center.node_provider_count,
            node_count: data_center.node_count,
        })
        .collect::<Vec<_>>();
    NnsDataCenterListReport {
        schema_version: NNS_DATA_CENTER_LIST_REPORT_SCHEMA_VERSION,
        network: list.network,
        registry_canister_id: list.registry_canister_id,
        registry_version: list.registry_version,
        fetched_at: list.fetched_at,
        source_endpoint: list.source_endpoint,
        fetched_by: list.fetched_by,
        data_center_count: data_centers.len(),
        data_centers,
    }
}

///
/// NnsDataCenterSource
///
trait NnsDataCenterSource {
    fn fetch_data_centers(
        &self,
        request: &MainnetRegistryFetchRequest,
    ) -> Result<MainnetDataCenterList, NnsDataCenterHostError>;
}

///
/// LiveNnsDataCenterSource
///
struct LiveNnsDataCenterSource;

impl NnsDataCenterSource for LiveNnsDataCenterSource {
    fn fetch_data_centers(
        &self,
        request: &MainnetRegistryFetchRequest,
    ) -> Result<MainnetDataCenterList, NnsDataCenterHostError> {
        Ok(fetch_mainnet_data_center_list(request)?)
    }
}

fn enforce_mainnet_network(network: &str) -> Result<(), NnsDataCenterHostError> {
    if network == MAINNET_NETWORK {
        return Ok(());
    }
    Err(NnsDataCenterHostError::UnsupportedNetwork {
        network: network.to_string(),
    })
}

fn resolve_data_center(
    report: &NnsDataCenterListReport,
    input: &str,
) -> Result<(NnsDataCenterRow, String), NnsDataCenterHostError> {
    let normalized = input.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(NnsDataCenterHostError::DataCenterNotFound {
            input: input.to_string(),
        });
    }
    if let Some(data_center) = report
        .data_centers
        .iter()
        .find(|data_center| data_center.data_center_id == normalized)
    {
        return Ok((data_center.clone(), "data_center_id".to_string()));
    }
    let matches = report
        .data_centers
        .iter()
        .filter(|data_center| data_center.data_center_id.starts_with(&normalized))
        .cloned()
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [data_center] => Ok((data_center.clone(), "data_center_id_prefix".to_string())),
        [] => Err(NnsDataCenterHostError::DataCenterNotFound {
            input: input.to_string(),
        }),
        _ => Err(NnsDataCenterHostError::AmbiguousDataCenterPrefix {
            prefix: normalized,
            matches: matches
                .into_iter()
                .map(|data_center| data_center.data_center_id)
                .collect(),
        }),
    }
}

#[cfg(test)]
mod tests;
