use crate::{
    nns_data_center::{
        NnsDataCenterCacheRequest, NnsDataCenterHostError, NnsDataCenterListReport,
        NnsDataCenterListRequest, NnsDataCenterRefreshReport, NnsDataCenterRefreshRequest,
        build_nns_data_center_list_report, refresh_nns_data_center_report,
    },
    nns_node::{
        NNS_NODE_SUBNET_KIND_APPLICATION, NNS_NODE_SUBNET_KIND_SYSTEM,
        NNS_NODE_SUBNET_KIND_UNKNOWN, NnsNodeCacheRequest, NnsNodeHostError, NnsNodeListFilters,
        NnsNodeListReport, NnsNodeListRequest, NnsNodeRefreshReport, NnsNodeRefreshRequest,
        build_nns_node_list_report, refresh_nns_node_report,
    },
    nns_node_operator::{
        NnsNodeOperatorCacheRequest, NnsNodeOperatorHostError, NnsNodeOperatorListReport,
        NnsNodeOperatorListRequest, NnsNodeOperatorRefreshReport, NnsNodeOperatorRefreshRequest,
        build_nns_node_operator_list_report, refresh_nns_node_operator_report,
    },
    nns_node_provider::{
        NnsNodeProviderCacheRequest, NnsNodeProviderHostError, NnsNodeProviderListReport,
        NnsNodeProviderListRequest, NnsNodeProviderRefreshReport, NnsNodeProviderRefreshRequest,
        build_nns_node_provider_list_report, refresh_nns_node_provider_report,
    },
    nns_render::yes_no,
    subnet_catalog::{
        DEFAULT_STALE_AFTER_SECONDS, SubnetCatalogCacheRequest, SubnetCatalogFilters,
        SubnetCatalogHostError, SubnetCatalogListReport, SubnetCatalogListRequest,
        SubnetCatalogRefreshReport, SubnetCatalogRefreshRequest, build_subnet_catalog_list_report,
        refresh_subnet_catalog,
    },
    table::{ColumnAlign, render_table},
};
use canic_subnet_catalog::{MAINNET_NETWORK, SubnetKind};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error as ThisError;

pub const NNS_TOPOLOGY_SUMMARY_REPORT_SCHEMA_VERSION: u32 = 1;
pub const NNS_TOPOLOGY_REFRESH_REPORT_SCHEMA_VERSION: u32 = 1;

///
/// NnsTopologySummaryRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsTopologySummaryRequest {
    pub icp_root: PathBuf,
    pub network: String,
    pub source_endpoint: String,
    pub now_unix_secs: u64,
}

///
/// NnsTopologyRefreshRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsTopologyRefreshRequest {
    pub icp_root: PathBuf,
    pub network: String,
    pub source_endpoint: String,
    pub now_unix_secs: u64,
    pub lock_stale_after_seconds: u64,
    pub dry_run: bool,
}

///
/// NnsTopologySummaryReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsTopologySummaryReport {
    pub schema_version: u32,
    pub network: String,
    pub source_endpoint: String,
    pub subnet_count: usize,
    pub application_subnet_count: usize,
    pub system_subnet_count: usize,
    pub unknown_subnet_count: usize,
    pub routing_range_count: usize,
    pub node_count: usize,
    pub application_node_count: usize,
    pub system_node_count: usize,
    pub unknown_node_count: usize,
    pub node_provider_count: usize,
    pub node_operator_count: usize,
    pub data_center_count: usize,
    pub subnet_catalog_stale: bool,
    pub subnet_catalog_stale_reason: String,
    pub registry_versions: Vec<NnsTopologyRegistryVersionRow>,
}

///
/// NnsTopologyRegistryVersionRow
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsTopologyRegistryVersionRow {
    pub source: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub source_endpoint: String,
    pub stale: Option<bool>,
}

///
/// NnsTopologyRefreshReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsTopologyRefreshReport {
    pub schema_version: u32,
    pub network: String,
    pub source_endpoint: String,
    pub dry_run: bool,
    pub component_count: usize,
    pub wrote_cache_count: usize,
    pub replaced_existing_cache_count: usize,
    pub components: Vec<NnsTopologyRefreshRow>,
}

///
/// NnsTopologyRefreshRow
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsTopologyRefreshRow {
    pub source: String,
    pub cache_path: String,
    pub refresh_lock_path: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub source_endpoint: String,
    pub fetched_by: String,
    pub dry_run: bool,
    pub wrote_cache: bool,
    pub replaced_existing_cache: bool,
    pub item_count: usize,
}

///
/// NnsTopologyHostError
///
#[derive(Debug, ThisError)]
pub enum NnsTopologyHostError {
    #[error(
        "`canic nns topology` supports only the mainnet `ic` network\n\nThe NNS topology report is derived from public Internet Computer mainnet registry records.\nLocal replica NNS registry discovery is not implemented yet.\n\nTry:\n  canic --network ic nns topology summary\n  canic --network ic nns topology refresh"
    )]
    UnsupportedNetwork { network: String },

    #[error(transparent)]
    Subnet(#[from] SubnetCatalogHostError),

    #[error(transparent)]
    Node(#[from] NnsNodeHostError),

    #[error(transparent)]
    NodeProvider(#[from] NnsNodeProviderHostError),

    #[error(transparent)]
    NodeOperator(#[from] NnsNodeOperatorHostError),

    #[error(transparent)]
    DataCenter(#[from] NnsDataCenterHostError),
}

pub fn build_nns_topology_summary_report(
    request: &NnsTopologySummaryRequest,
) -> Result<NnsTopologySummaryReport, NnsTopologyHostError> {
    enforce_mainnet_network(&request.network)?;

    let subnet_report = build_subnet_catalog_list_report(&SubnetCatalogListRequest {
        cache: SubnetCatalogCacheRequest {
            icp_root: request.icp_root.clone(),
            network: request.network.clone(),
        },
        now_unix_secs: request.now_unix_secs,
        stale_after_seconds: DEFAULT_STALE_AFTER_SECONDS,
        filters: SubnetCatalogFilters::default(),
        show_ranges: false,
        range_limit: 1,
        range_offset: 0,
    })?;
    let node_report = build_nns_node_list_report(&NnsNodeListRequest {
        cache: NnsNodeCacheRequest {
            icp_root: request.icp_root.clone(),
            network: request.network.clone(),
        },
        source_endpoint: request.source_endpoint.clone(),
        now_unix_secs: request.now_unix_secs,
        filters: NnsNodeListFilters::default(),
    })?;
    let node_provider_report = build_nns_node_provider_list_report(&NnsNodeProviderListRequest {
        cache: NnsNodeProviderCacheRequest {
            icp_root: request.icp_root.clone(),
            network: request.network.clone(),
        },
        source_endpoint: request.source_endpoint.clone(),
        now_unix_secs: request.now_unix_secs,
    })?;
    let node_operator_report = build_nns_node_operator_list_report(&NnsNodeOperatorListRequest {
        cache: NnsNodeOperatorCacheRequest {
            icp_root: request.icp_root.clone(),
            network: request.network.clone(),
        },
        source_endpoint: request.source_endpoint.clone(),
        now_unix_secs: request.now_unix_secs,
    })?;
    let data_center_report = build_nns_data_center_list_report(&NnsDataCenterListRequest {
        cache: NnsDataCenterCacheRequest {
            icp_root: request.icp_root.clone(),
            network: request.network.clone(),
        },
        source_endpoint: request.source_endpoint.clone(),
        now_unix_secs: request.now_unix_secs,
    })?;

    Ok(topology_summary_report_from_reports(
        request.network.clone(),
        request.source_endpoint.clone(),
        subnet_report,
        node_report,
        node_provider_report,
        node_operator_report,
        data_center_report,
    ))
}

pub fn refresh_nns_topology_report(
    request: &NnsTopologyRefreshRequest,
) -> Result<NnsTopologyRefreshReport, NnsTopologyHostError> {
    enforce_mainnet_network(&request.network)?;

    let subnet_report = refresh_subnet_catalog(&SubnetCatalogRefreshRequest {
        cache: SubnetCatalogCacheRequest {
            icp_root: request.icp_root.clone(),
            network: request.network.clone(),
        },
        source_endpoint: request.source_endpoint.clone(),
        now_unix_secs: request.now_unix_secs,
        lock_stale_after_seconds: request.lock_stale_after_seconds,
        dry_run: request.dry_run,
        output_path: None,
    })?;
    let node_report = refresh_nns_node_report(&NnsNodeRefreshRequest {
        cache: NnsNodeCacheRequest {
            icp_root: request.icp_root.clone(),
            network: request.network.clone(),
        },
        source_endpoint: request.source_endpoint.clone(),
        now_unix_secs: request.now_unix_secs,
        lock_stale_after_seconds: request.lock_stale_after_seconds,
        dry_run: request.dry_run,
        output_path: None,
    })?;
    let node_provider_report = refresh_nns_node_provider_report(&NnsNodeProviderRefreshRequest {
        cache: NnsNodeProviderCacheRequest {
            icp_root: request.icp_root.clone(),
            network: request.network.clone(),
        },
        source_endpoint: request.source_endpoint.clone(),
        now_unix_secs: request.now_unix_secs,
        lock_stale_after_seconds: request.lock_stale_after_seconds,
        dry_run: request.dry_run,
        output_path: None,
    })?;
    let node_operator_report = refresh_nns_node_operator_report(&NnsNodeOperatorRefreshRequest {
        cache: NnsNodeOperatorCacheRequest {
            icp_root: request.icp_root.clone(),
            network: request.network.clone(),
        },
        source_endpoint: request.source_endpoint.clone(),
        now_unix_secs: request.now_unix_secs,
        lock_stale_after_seconds: request.lock_stale_after_seconds,
        dry_run: request.dry_run,
        output_path: None,
    })?;
    let data_center_report = refresh_nns_data_center_report(&NnsDataCenterRefreshRequest {
        cache: NnsDataCenterCacheRequest {
            icp_root: request.icp_root.clone(),
            network: request.network.clone(),
        },
        source_endpoint: request.source_endpoint.clone(),
        now_unix_secs: request.now_unix_secs,
        lock_stale_after_seconds: request.lock_stale_after_seconds,
        dry_run: request.dry_run,
        output_path: None,
    })?;

    Ok(topology_refresh_report_from_reports(
        request.network.clone(),
        request.source_endpoint.clone(),
        request.dry_run,
        NnsTopologyRefreshComponentReports {
            subnet: subnet_report,
            node: node_report,
            node_provider: node_provider_report,
            node_operator: node_operator_report,
            data_center: data_center_report,
        },
    ))
}

#[must_use]
pub fn nns_topology_summary_report_text(report: &NnsTopologySummaryReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "topology: {} subnets {} nodes {} node_operators {} node_providers {} data_centers {}",
        report.network,
        report.subnet_count,
        report.node_count,
        report.node_operator_count,
        report.node_provider_count,
        report.data_center_count
    ));
    lines.push("counts:".to_string());
    lines.push(render_count_table(report));
    lines.push("subnet_kinds:".to_string());
    lines.push(render_kind_table(report));
    lines.push("registry_versions:".to_string());
    lines.push(render_registry_version_table(report));
    lines.join("\n")
}

#[must_use]
pub fn nns_topology_refresh_report_text(report: &NnsTopologyRefreshReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "topology_refresh: {} components {} wrote {} replaced {} dry_run {}",
        report.network,
        report.component_count,
        report.wrote_cache_count,
        report.replaced_existing_cache_count,
        yes_no(report.dry_run)
    ));
    lines.push(format!("source_endpoint: {}", report.source_endpoint));
    lines.push(render_refresh_table(report));
    lines.join("\n")
}

fn topology_summary_report_from_reports(
    network: String,
    source_endpoint: String,
    subnet_report: SubnetCatalogListReport,
    node_report: NnsNodeListReport,
    node_provider_report: NnsNodeProviderListReport,
    node_operator_report: NnsNodeOperatorListReport,
    data_center_report: NnsDataCenterListReport,
) -> NnsTopologySummaryReport {
    let application_subnet_count = subnet_count_by_kind(&subnet_report, SubnetKind::Application);
    let system_subnet_count = subnet_count_by_kind(&subnet_report, SubnetKind::System);
    let unknown_subnet_count = subnet_count_by_kind(&subnet_report, SubnetKind::Unknown);
    let application_node_count =
        node_count_by_subnet_kind(&node_report, NNS_NODE_SUBNET_KIND_APPLICATION);
    let system_node_count = node_count_by_subnet_kind(&node_report, NNS_NODE_SUBNET_KIND_SYSTEM);
    let unknown_node_count = node_count_by_subnet_kind(&node_report, NNS_NODE_SUBNET_KIND_UNKNOWN);
    let registry_versions = vec![
        registry_version_row(
            "subnet_catalog",
            subnet_report.registry_version,
            subnet_report.fetched_at.clone(),
            None,
            Some(subnet_report.catalog_stale),
        ),
        registry_version_row(
            "nodes",
            node_report.registry_version,
            node_report.fetched_at.clone(),
            Some(node_report.source_endpoint.clone()),
            None,
        ),
        registry_version_row(
            "node_providers",
            node_provider_report.registry_version,
            node_provider_report.fetched_at.clone(),
            Some(node_provider_report.source_endpoint.clone()),
            None,
        ),
        registry_version_row(
            "node_operators",
            node_operator_report.registry_version,
            node_operator_report.fetched_at.clone(),
            Some(node_operator_report.source_endpoint.clone()),
            None,
        ),
        registry_version_row(
            "data_centers",
            data_center_report.registry_version,
            data_center_report.fetched_at.clone(),
            Some(data_center_report.source_endpoint.clone()),
            None,
        ),
    ];

    NnsTopologySummaryReport {
        schema_version: NNS_TOPOLOGY_SUMMARY_REPORT_SCHEMA_VERSION,
        network,
        source_endpoint,
        subnet_count: subnet_report.subnets.len(),
        application_subnet_count,
        system_subnet_count,
        unknown_subnet_count,
        routing_range_count: subnet_report
            .subnets
            .iter()
            .map(|subnet| subnet.range_count)
            .sum(),
        node_count: node_report.node_count,
        application_node_count,
        system_node_count,
        unknown_node_count,
        node_provider_count: node_provider_report.node_provider_count,
        node_operator_count: node_operator_report.node_operator_count,
        data_center_count: data_center_report.data_center_count,
        subnet_catalog_stale: subnet_report.catalog_stale,
        subnet_catalog_stale_reason: subnet_report.stale_reason,
        registry_versions,
    }
}

fn topology_refresh_report_from_reports(
    network: String,
    source_endpoint: String,
    dry_run: bool,
    reports: NnsTopologyRefreshComponentReports,
) -> NnsTopologyRefreshReport {
    let components = vec![
        refresh_row_from_subnet_report(reports.subnet),
        refresh_row_from_node_report(reports.node),
        refresh_row_from_node_provider_report(reports.node_provider),
        refresh_row_from_node_operator_report(reports.node_operator),
        refresh_row_from_data_center_report(reports.data_center),
    ];
    let wrote_cache_count = components
        .iter()
        .filter(|component| component.wrote_cache)
        .count();
    let replaced_existing_cache_count = components
        .iter()
        .filter(|component| component.replaced_existing_cache)
        .count();

    NnsTopologyRefreshReport {
        schema_version: NNS_TOPOLOGY_REFRESH_REPORT_SCHEMA_VERSION,
        network,
        source_endpoint,
        dry_run,
        component_count: components.len(),
        wrote_cache_count,
        replaced_existing_cache_count,
        components,
    }
}

///
/// NnsTopologyRefreshComponentReports
///
struct NnsTopologyRefreshComponentReports {
    subnet: SubnetCatalogRefreshReport,
    node: NnsNodeRefreshReport,
    node_provider: NnsNodeProviderRefreshReport,
    node_operator: NnsNodeOperatorRefreshReport,
    data_center: NnsDataCenterRefreshReport,
}

fn refresh_row_from_subnet_report(report: SubnetCatalogRefreshReport) -> NnsTopologyRefreshRow {
    NnsTopologyRefreshRow {
        source: "subnet_catalog".to_string(),
        cache_path: report.catalog_path,
        refresh_lock_path: report.refresh_lock_path,
        registry_version: report.registry_version,
        fetched_at: report.fetched_at,
        source_endpoint: report.source_endpoint,
        fetched_by: report.fetched_by,
        dry_run: report.dry_run,
        wrote_cache: report.wrote_catalog,
        replaced_existing_cache: report.replaced_existing_catalog,
        item_count: report.subnet_count,
    }
}

fn refresh_row_from_node_report(report: NnsNodeRefreshReport) -> NnsTopologyRefreshRow {
    NnsTopologyRefreshRow {
        source: "nodes".to_string(),
        cache_path: report.cache_path,
        refresh_lock_path: report.refresh_lock_path,
        registry_version: report.registry_version,
        fetched_at: report.fetched_at,
        source_endpoint: report.source_endpoint,
        fetched_by: report.fetched_by,
        dry_run: report.dry_run,
        wrote_cache: report.wrote_cache,
        replaced_existing_cache: report.replaced_existing_cache,
        item_count: report.node_count,
    }
}

fn refresh_row_from_node_provider_report(
    report: NnsNodeProviderRefreshReport,
) -> NnsTopologyRefreshRow {
    NnsTopologyRefreshRow {
        source: "node_providers".to_string(),
        cache_path: report.cache_path,
        refresh_lock_path: report.refresh_lock_path,
        registry_version: report.registry_version,
        fetched_at: report.fetched_at,
        source_endpoint: report.source_endpoint,
        fetched_by: report.fetched_by,
        dry_run: report.dry_run,
        wrote_cache: report.wrote_cache,
        replaced_existing_cache: report.replaced_existing_cache,
        item_count: report.node_provider_count,
    }
}

fn refresh_row_from_node_operator_report(
    report: NnsNodeOperatorRefreshReport,
) -> NnsTopologyRefreshRow {
    NnsTopologyRefreshRow {
        source: "node_operators".to_string(),
        cache_path: report.cache_path,
        refresh_lock_path: report.refresh_lock_path,
        registry_version: report.registry_version,
        fetched_at: report.fetched_at,
        source_endpoint: report.source_endpoint,
        fetched_by: report.fetched_by,
        dry_run: report.dry_run,
        wrote_cache: report.wrote_cache,
        replaced_existing_cache: report.replaced_existing_cache,
        item_count: report.node_operator_count,
    }
}

fn refresh_row_from_data_center_report(
    report: NnsDataCenterRefreshReport,
) -> NnsTopologyRefreshRow {
    NnsTopologyRefreshRow {
        source: "data_centers".to_string(),
        cache_path: report.cache_path,
        refresh_lock_path: report.refresh_lock_path,
        registry_version: report.registry_version,
        fetched_at: report.fetched_at,
        source_endpoint: report.source_endpoint,
        fetched_by: report.fetched_by,
        dry_run: report.dry_run,
        wrote_cache: report.wrote_cache,
        replaced_existing_cache: report.replaced_existing_cache,
        item_count: report.data_center_count,
    }
}

fn enforce_mainnet_network(network: &str) -> Result<(), NnsTopologyHostError> {
    if network == MAINNET_NETWORK {
        return Ok(());
    }
    Err(NnsTopologyHostError::UnsupportedNetwork {
        network: network.to_string(),
    })
}

fn subnet_count_by_kind(report: &SubnetCatalogListReport, kind: SubnetKind) -> usize {
    report
        .subnets
        .iter()
        .filter(|subnet| subnet.subnet_kind == kind)
        .count()
}

fn node_count_by_subnet_kind(report: &NnsNodeListReport, kind: &str) -> usize {
    report
        .nodes
        .iter()
        .filter(|node| node.subnet_kind.eq_ignore_ascii_case(kind))
        .count()
}

fn registry_version_row(
    source: &str,
    registry_version: u64,
    fetched_at: String,
    source_endpoint: Option<String>,
    stale: Option<bool>,
) -> NnsTopologyRegistryVersionRow {
    NnsTopologyRegistryVersionRow {
        source: source.to_string(),
        registry_version,
        fetched_at,
        source_endpoint: source_endpoint.unwrap_or_else(|| "-".to_string()),
        stale,
    }
}

fn render_count_table(report: &NnsTopologySummaryReport) -> String {
    let headers = ["METRIC", "COUNT"];
    let rows = [
        ["subnets".to_string(), report.subnet_count.to_string()],
        [
            "routing_ranges".to_string(),
            report.routing_range_count.to_string(),
        ],
        ["nodes".to_string(), report.node_count.to_string()],
        [
            "node_operators".to_string(),
            report.node_operator_count.to_string(),
        ],
        [
            "node_providers".to_string(),
            report.node_provider_count.to_string(),
        ],
        [
            "data_centers".to_string(),
            report.data_center_count.to_string(),
        ],
    ];
    let alignments = [ColumnAlign::Left, ColumnAlign::Right];
    render_table(&headers, &rows, &alignments)
}

fn render_kind_table(report: &NnsTopologySummaryReport) -> String {
    let headers = ["KIND", "SUBNETS", "NODES"];
    let rows = [
        [
            NNS_NODE_SUBNET_KIND_APPLICATION.to_string(),
            report.application_subnet_count.to_string(),
            report.application_node_count.to_string(),
        ],
        [
            NNS_NODE_SUBNET_KIND_SYSTEM.to_string(),
            report.system_subnet_count.to_string(),
            report.system_node_count.to_string(),
        ],
        [
            NNS_NODE_SUBNET_KIND_UNKNOWN.to_string(),
            report.unknown_subnet_count.to_string(),
            report.unknown_node_count.to_string(),
        ],
    ];
    let alignments = [ColumnAlign::Left, ColumnAlign::Right, ColumnAlign::Right];
    render_table(&headers, &rows, &alignments)
}

fn render_registry_version_table(report: &NnsTopologySummaryReport) -> String {
    let headers = ["SOURCE", "VERSION", "FETCHED_AT", "STALE", "ENDPOINT"];
    let rows = report
        .registry_versions
        .iter()
        .map(|row| {
            [
                row.source.clone(),
                row.registry_version.to_string(),
                row.fetched_at.clone(),
                row.stale
                    .map_or_else(|| "-".to_string(), |stale| yes_no(stale).to_string()),
                row.source_endpoint.clone(),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
    ];
    render_table(&headers, &rows, &alignments)
}

fn render_refresh_table(report: &NnsTopologyRefreshReport) -> String {
    let headers = [
        "SOURCE",
        "COUNT",
        "VERSION",
        "FETCHED_AT",
        "WROTE",
        "REPLACED",
        "CACHE",
    ];
    let rows = report
        .components
        .iter()
        .map(|row| {
            [
                row.source.clone(),
                row.item_count.to_string(),
                row.registry_version.to_string(),
                row.fetched_at.clone(),
                yes_no(row.wrote_cache).to_string(),
                yes_no(row.replaced_existing_cache).to_string(),
                row.cache_path.clone(),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
    ];
    render_table(&headers, &rows, &alignments)
}

#[cfg(test)]
mod tests;
