use crate::{
    nns_data_center::{
        NnsDataCenterCacheRequest, NnsDataCenterHostError, NnsDataCenterListReport,
        NnsDataCenterListRequest, NnsDataCenterRefreshReport, NnsDataCenterRefreshRequest,
        build_nns_data_center_list_report, refresh_nns_data_center_report,
    },
    nns_node::{
        NNS_NODE_SUBNET_KIND_APPLICATION, NNS_NODE_SUBNET_KIND_CLOUD_ENGINE,
        NNS_NODE_SUBNET_KIND_SYSTEM, NNS_NODE_SUBNET_KIND_UNKNOWN, NnsNodeCacheRequest,
        NnsNodeHostError, NnsNodeListFilters, NnsNodeListReport, NnsNodeListRequest,
        NnsNodeRefreshReport, NnsNodeRefreshRequest, build_nns_node_list_report,
        refresh_nns_node_report,
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
use std::{collections::BTreeSet, path::PathBuf};
use thiserror::Error as ThisError;

pub const NNS_TOPOLOGY_SUMMARY_REPORT_SCHEMA_VERSION: u32 = 3;
pub const NNS_TOPOLOGY_COVERAGE_REPORT_SCHEMA_VERSION: u32 = 1;
pub const NNS_TOPOLOGY_VERSIONS_REPORT_SCHEMA_VERSION: u32 = 1;
pub const NNS_TOPOLOGY_HEALTH_REPORT_SCHEMA_VERSION: u32 = 1;
pub const NNS_TOPOLOGY_GAPS_REPORT_SCHEMA_VERSION: u32 = 1;
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
/// NnsTopologyCoverageRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsTopologyCoverageRequest {
    pub icp_root: PathBuf,
    pub network: String,
    pub source_endpoint: String,
    pub now_unix_secs: u64,
}

///
/// NnsTopologyVersionsRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsTopologyVersionsRequest {
    pub icp_root: PathBuf,
    pub network: String,
    pub source_endpoint: String,
    pub now_unix_secs: u64,
}

///
/// NnsTopologyHealthRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsTopologyHealthRequest {
    pub icp_root: PathBuf,
    pub network: String,
    pub source_endpoint: String,
    pub now_unix_secs: u64,
}

///
/// NnsTopologyGapsRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsTopologyGapsRequest {
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
    pub cloud_engine_subnet_count: usize,
    pub system_subnet_count: usize,
    pub unknown_subnet_count: usize,
    pub routing_range_count: usize,
    pub node_count: usize,
    pub application_node_count: usize,
    pub cloud_engine_node_count: usize,
    pub system_node_count: usize,
    pub unknown_node_count: usize,
    pub node_provider_count: usize,
    pub node_operator_count: usize,
    pub data_center_count: usize,
    pub nodes_with_known_node_provider_count: usize,
    pub nodes_with_unknown_node_provider_count: usize,
    pub nodes_with_known_node_operator_count: usize,
    pub nodes_with_unknown_node_operator_count: usize,
    pub nodes_with_known_data_center_count: usize,
    pub nodes_with_unknown_data_center_count: usize,
    pub node_operators_with_known_node_provider_count: usize,
    pub node_operators_with_unknown_node_provider_count: usize,
    pub node_operators_with_known_data_center_count: usize,
    pub node_operators_with_unknown_data_center_count: usize,
    pub subnet_catalog_stale: bool,
    pub subnet_catalog_stale_reason: String,
    pub registry_versions: Vec<NnsTopologyRegistryVersionRow>,
}

///
/// NnsTopologyCoverageReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsTopologyCoverageReport {
    pub schema_version: u32,
    pub network: String,
    pub source_endpoint: String,
    pub node_count: usize,
    pub node_provider_count: usize,
    pub node_operator_count: usize,
    pub data_center_count: usize,
    pub nodes_with_known_node_provider_count: usize,
    pub nodes_with_unknown_node_provider_count: usize,
    pub nodes_with_known_node_operator_count: usize,
    pub nodes_with_unknown_node_operator_count: usize,
    pub nodes_with_known_data_center_count: usize,
    pub nodes_with_unknown_data_center_count: usize,
    pub node_operators_with_known_node_provider_count: usize,
    pub node_operators_with_unknown_node_provider_count: usize,
    pub node_operators_with_known_data_center_count: usize,
    pub node_operators_with_unknown_data_center_count: usize,
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
/// NnsTopologyVersionsReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsTopologyVersionsReport {
    pub schema_version: u32,
    pub network: String,
    pub source_endpoint: String,
    pub source_count: usize,
    pub registry_versions: Vec<NnsTopologyRegistryVersionRow>,
}

///
/// NnsTopologyHealthReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsTopologyHealthReport {
    pub schema_version: u32,
    pub network: String,
    pub source_endpoint: String,
    pub status: String,
    pub registry_source_count: usize,
    pub registry_version_min: Option<u64>,
    pub registry_version_max: Option<u64>,
    pub registry_versions_aligned: bool,
    pub stale_source_count: usize,
    pub subnet_catalog_stale: bool,
    pub subnet_catalog_stale_reason: String,
    pub known_join_count: usize,
    pub unknown_join_count: usize,
    pub join_coverage: String,
    pub checks: Vec<NnsTopologyHealthCheckRow>,
}

///
/// NnsTopologyHealthCheckRow
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsTopologyHealthCheckRow {
    pub check: String,
    pub status: String,
    pub detail: String,
}

///
/// NnsTopologyGapsReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsTopologyGapsReport {
    pub schema_version: u32,
    pub network: String,
    pub source_endpoint: String,
    pub status: String,
    pub gap_count: usize,
    pub gaps: Vec<NnsTopologyGapRow>,
}

///
/// NnsTopologyGapRow
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsTopologyGapRow {
    pub subject_kind: String,
    pub subject: String,
    pub missing_relation: String,
    pub referenced_id: String,
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
        "`canic nns topology` supports only the mainnet `ic` network\n\nThe NNS topology report is derived from public Internet Computer mainnet registry records.\nLocal replica NNS registry discovery is not implemented yet.\n\nTry:\n  canic --network ic nns topology summary\n  canic --network ic nns topology coverage\n  canic --network ic nns topology versions\n  canic --network ic nns topology health\n  canic --network ic nns topology gaps\n  canic --network ic nns topology refresh"
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

pub fn build_nns_topology_versions_report(
    request: &NnsTopologyVersionsRequest,
) -> Result<NnsTopologyVersionsReport, NnsTopologyHostError> {
    let summary = build_nns_topology_summary_report(&NnsTopologySummaryRequest {
        icp_root: request.icp_root.clone(),
        network: request.network.clone(),
        source_endpoint: request.source_endpoint.clone(),
        now_unix_secs: request.now_unix_secs,
    })?;

    Ok(topology_versions_report_from_summary(summary))
}

pub fn build_nns_topology_coverage_report(
    request: &NnsTopologyCoverageRequest,
) -> Result<NnsTopologyCoverageReport, NnsTopologyHostError> {
    let summary = build_nns_topology_summary_report(&NnsTopologySummaryRequest {
        icp_root: request.icp_root.clone(),
        network: request.network.clone(),
        source_endpoint: request.source_endpoint.clone(),
        now_unix_secs: request.now_unix_secs,
    })?;

    Ok(topology_coverage_report_from_summary(summary))
}

pub fn build_nns_topology_health_report(
    request: &NnsTopologyHealthRequest,
) -> Result<NnsTopologyHealthReport, NnsTopologyHostError> {
    let summary = build_nns_topology_summary_report(&NnsTopologySummaryRequest {
        icp_root: request.icp_root.clone(),
        network: request.network.clone(),
        source_endpoint: request.source_endpoint.clone(),
        now_unix_secs: request.now_unix_secs,
    })?;

    Ok(topology_health_report_from_summary(summary))
}

pub fn build_nns_topology_gaps_report(
    request: &NnsTopologyGapsRequest,
) -> Result<NnsTopologyGapsReport, NnsTopologyHostError> {
    enforce_mainnet_network(&request.network)?;

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

    Ok(topology_gaps_report_from_reports(
        request.network.clone(),
        request.source_endpoint.clone(),
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
    lines.push(String::new());
    lines.push(render_count_table(report));
    lines.push(String::new());
    lines.push(render_kind_table(report));
    lines.push(String::new());
    lines.push(render_summary_join_coverage_table(report));
    lines.push(String::new());
    lines.push(render_registry_version_table(&report.registry_versions));
    lines.join("\n")
}

#[must_use]
pub fn nns_topology_coverage_report_text(report: &NnsTopologyCoverageReport) -> String {
    let lines = [
        render_coverage_count_table(report),
        String::new(),
        render_coverage_join_coverage_table(report),
    ];
    lines.join("\n")
}

#[must_use]
pub fn nns_topology_versions_report_text(report: &NnsTopologyVersionsReport) -> String {
    render_registry_version_table(&report.registry_versions)
}

#[must_use]
pub fn nns_topology_health_report_text(report: &NnsTopologyHealthReport) -> String {
    render_health_check_table(&report.checks)
}

#[must_use]
pub fn nns_topology_gaps_report_text(report: &NnsTopologyGapsReport) -> String {
    if report.gaps.is_empty() {
        return render_gaps_status_table(report);
    }
    render_gaps_table(&report.gaps)
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
    let cloud_engine_subnet_count = subnet_count_by_kind(&subnet_report, SubnetKind::CloudEngine);
    let system_subnet_count = subnet_count_by_kind(&subnet_report, SubnetKind::System);
    let unknown_subnet_count = subnet_count_by_kind(&subnet_report, SubnetKind::Unknown);
    let application_node_count =
        node_count_by_subnet_kind(&node_report, NNS_NODE_SUBNET_KIND_APPLICATION);
    let cloud_engine_node_count =
        node_count_by_subnet_kind(&node_report, NNS_NODE_SUBNET_KIND_CLOUD_ENGINE);
    let system_node_count = node_count_by_subnet_kind(&node_report, NNS_NODE_SUBNET_KIND_SYSTEM);
    let unknown_node_count = node_count_by_subnet_kind(&node_report, NNS_NODE_SUBNET_KIND_UNKNOWN);
    let join_coverage = topology_summary_join_coverage_counts(
        &node_report,
        &node_provider_report,
        &node_operator_report,
        &data_center_report,
    );
    let registry_versions = topology_summary_registry_versions(
        &subnet_report,
        &node_report,
        &node_provider_report,
        &node_operator_report,
        &data_center_report,
    );

    NnsTopologySummaryReport {
        schema_version: NNS_TOPOLOGY_SUMMARY_REPORT_SCHEMA_VERSION,
        network,
        source_endpoint,
        subnet_count: subnet_report.subnets.len(),
        application_subnet_count,
        cloud_engine_subnet_count,
        system_subnet_count,
        unknown_subnet_count,
        routing_range_count: subnet_report
            .subnets
            .iter()
            .map(|subnet| subnet.range_count)
            .sum(),
        node_count: node_report.node_count,
        application_node_count,
        cloud_engine_node_count,
        system_node_count,
        unknown_node_count,
        node_provider_count: node_provider_report.node_provider_count,
        node_operator_count: node_operator_report.node_operator_count,
        data_center_count: data_center_report.data_center_count,
        nodes_with_known_node_provider_count: join_coverage.nodes_with_known_node_provider_count,
        nodes_with_unknown_node_provider_count: node_report
            .node_count
            .saturating_sub(join_coverage.nodes_with_known_node_provider_count),
        nodes_with_known_node_operator_count: join_coverage.nodes_with_known_node_operator_count,
        nodes_with_unknown_node_operator_count: node_report
            .node_count
            .saturating_sub(join_coverage.nodes_with_known_node_operator_count),
        nodes_with_known_data_center_count: join_coverage.nodes_with_known_data_center_count,
        nodes_with_unknown_data_center_count: node_report
            .node_count
            .saturating_sub(join_coverage.nodes_with_known_data_center_count),
        node_operators_with_known_node_provider_count: join_coverage
            .node_operators_with_known_node_provider_count,
        node_operators_with_unknown_node_provider_count: node_operator_report
            .node_operator_count
            .saturating_sub(join_coverage.node_operators_with_known_node_provider_count),
        node_operators_with_known_data_center_count: join_coverage
            .node_operators_with_known_data_center_count,
        node_operators_with_unknown_data_center_count: node_operator_report
            .node_operator_count
            .saturating_sub(join_coverage.node_operators_with_known_data_center_count),
        subnet_catalog_stale: subnet_report.catalog_stale,
        subnet_catalog_stale_reason: subnet_report.stale_reason,
        registry_versions,
    }
}

///
/// NnsTopologyJoinCoverageCounts
///
struct NnsTopologyJoinCoverageCounts {
    nodes_with_known_node_provider_count: usize,
    nodes_with_known_node_operator_count: usize,
    nodes_with_known_data_center_count: usize,
    node_operators_with_known_node_provider_count: usize,
    node_operators_with_known_data_center_count: usize,
}

fn topology_summary_join_coverage_counts(
    node_report: &NnsNodeListReport,
    node_provider_report: &NnsNodeProviderListReport,
    node_operator_report: &NnsNodeOperatorListReport,
    data_center_report: &NnsDataCenterListReport,
) -> NnsTopologyJoinCoverageCounts {
    let node_provider_principals = node_provider_report
        .node_providers
        .iter()
        .map(|provider| provider.node_provider_principal.as_str())
        .collect::<BTreeSet<_>>();
    let node_operator_principals = node_operator_report
        .node_operators
        .iter()
        .map(|operator| operator.node_operator_principal.as_str())
        .collect::<BTreeSet<_>>();
    let data_center_ids = data_center_report
        .data_centers
        .iter()
        .map(|data_center| data_center.data_center_id.as_str())
        .collect::<BTreeSet<_>>();

    NnsTopologyJoinCoverageCounts {
        nodes_with_known_node_provider_count: node_count_with_known_node_provider(
            node_report,
            &node_provider_principals,
        ),
        nodes_with_known_node_operator_count: node_count_with_known_node_operator(
            node_report,
            &node_operator_principals,
        ),
        nodes_with_known_data_center_count: node_count_with_known_data_center(
            node_report,
            &data_center_ids,
        ),
        node_operators_with_known_node_provider_count: operator_count_with_known_node_provider(
            node_operator_report,
            &node_provider_principals,
        ),
        node_operators_with_known_data_center_count: operator_count_with_known_data_center(
            node_operator_report,
            &data_center_ids,
        ),
    }
}

fn topology_summary_registry_versions(
    subnet_report: &SubnetCatalogListReport,
    node_report: &NnsNodeListReport,
    node_provider_report: &NnsNodeProviderListReport,
    node_operator_report: &NnsNodeOperatorListReport,
    data_center_report: &NnsDataCenterListReport,
) -> Vec<NnsTopologyRegistryVersionRow> {
    vec![
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
    ]
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

fn topology_coverage_report_from_summary(
    summary: NnsTopologySummaryReport,
) -> NnsTopologyCoverageReport {
    NnsTopologyCoverageReport {
        schema_version: NNS_TOPOLOGY_COVERAGE_REPORT_SCHEMA_VERSION,
        network: summary.network,
        source_endpoint: summary.source_endpoint,
        node_count: summary.node_count,
        node_provider_count: summary.node_provider_count,
        node_operator_count: summary.node_operator_count,
        data_center_count: summary.data_center_count,
        nodes_with_known_node_provider_count: summary.nodes_with_known_node_provider_count,
        nodes_with_unknown_node_provider_count: summary.nodes_with_unknown_node_provider_count,
        nodes_with_known_node_operator_count: summary.nodes_with_known_node_operator_count,
        nodes_with_unknown_node_operator_count: summary.nodes_with_unknown_node_operator_count,
        nodes_with_known_data_center_count: summary.nodes_with_known_data_center_count,
        nodes_with_unknown_data_center_count: summary.nodes_with_unknown_data_center_count,
        node_operators_with_known_node_provider_count: summary
            .node_operators_with_known_node_provider_count,
        node_operators_with_unknown_node_provider_count: summary
            .node_operators_with_unknown_node_provider_count,
        node_operators_with_known_data_center_count: summary
            .node_operators_with_known_data_center_count,
        node_operators_with_unknown_data_center_count: summary
            .node_operators_with_unknown_data_center_count,
    }
}

fn topology_versions_report_from_summary(
    summary: NnsTopologySummaryReport,
) -> NnsTopologyVersionsReport {
    NnsTopologyVersionsReport {
        schema_version: NNS_TOPOLOGY_VERSIONS_REPORT_SCHEMA_VERSION,
        network: summary.network,
        source_endpoint: summary.source_endpoint,
        source_count: summary.registry_versions.len(),
        registry_versions: summary.registry_versions,
    }
}

fn topology_health_report_from_summary(
    summary: NnsTopologySummaryReport,
) -> NnsTopologyHealthReport {
    let health = topology_health_derived_metrics(&summary);
    let status = if health.registry_versions_aligned
        && health.stale_source_count == 0
        && health.unknown_join_count == 0
    {
        "ok"
    } else {
        "attention"
    }
    .to_string();
    let checks = topology_health_checks(&summary, &health);

    NnsTopologyHealthReport {
        schema_version: NNS_TOPOLOGY_HEALTH_REPORT_SCHEMA_VERSION,
        network: summary.network,
        source_endpoint: summary.source_endpoint,
        status,
        registry_source_count: health.registry_source_count,
        registry_version_min: health.registry_version_min,
        registry_version_max: health.registry_version_max,
        registry_versions_aligned: health.registry_versions_aligned,
        stale_source_count: health.stale_source_count,
        subnet_catalog_stale: summary.subnet_catalog_stale,
        subnet_catalog_stale_reason: summary.subnet_catalog_stale_reason,
        known_join_count: health.known_join_count,
        unknown_join_count: health.unknown_join_count,
        join_coverage: health.join_coverage,
        checks,
    }
}

fn topology_gaps_report_from_reports(
    network: String,
    source_endpoint: String,
    node_report: NnsNodeListReport,
    node_provider_report: NnsNodeProviderListReport,
    node_operator_report: NnsNodeOperatorListReport,
    data_center_report: NnsDataCenterListReport,
) -> NnsTopologyGapsReport {
    let node_provider_principals = node_provider_report
        .node_providers
        .iter()
        .map(|provider| provider.node_provider_principal.as_str())
        .collect::<BTreeSet<_>>();
    let node_operator_principals = node_operator_report
        .node_operators
        .iter()
        .map(|operator| operator.node_operator_principal.as_str())
        .collect::<BTreeSet<_>>();
    let data_center_ids = data_center_report
        .data_centers
        .iter()
        .map(|data_center| data_center.data_center_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut gaps = Vec::new();

    for node in &node_report.nodes {
        if !node_provider_principals.contains(node.node_provider_principal.as_str()) {
            gaps.push(topology_gap_row(
                "node",
                &node.node_principal,
                "node_provider",
                &node.node_provider_principal,
            ));
        }
        if !node_operator_principals.contains(node.node_operator_principal.as_str()) {
            gaps.push(topology_gap_row(
                "node",
                &node.node_principal,
                "node_operator",
                &node.node_operator_principal,
            ));
        }
        if !data_center_ids.contains(node.data_center_id.as_str()) {
            gaps.push(topology_gap_row(
                "node",
                &node.node_principal,
                "data_center",
                &node.data_center_id,
            ));
        }
    }

    for operator in &node_operator_report.node_operators {
        if !node_provider_principals.contains(operator.node_provider_principal.as_str()) {
            gaps.push(topology_gap_row(
                "node_operator",
                &operator.node_operator_principal,
                "node_provider",
                &operator.node_provider_principal,
            ));
        }
        if !data_center_ids.contains(operator.data_center_id.as_str()) {
            gaps.push(topology_gap_row(
                "node_operator",
                &operator.node_operator_principal,
                "data_center",
                &operator.data_center_id,
            ));
        }
    }

    gaps.sort_by(|left, right| {
        (
            left.subject_kind.as_str(),
            left.subject.as_str(),
            left.missing_relation.as_str(),
            left.referenced_id.as_str(),
        )
            .cmp(&(
                right.subject_kind.as_str(),
                right.subject.as_str(),
                right.missing_relation.as_str(),
                right.referenced_id.as_str(),
            ))
    });
    let gap_count = gaps.len();
    let status = if gap_count == 0 { "ok" } else { "attention" }.to_string();

    NnsTopologyGapsReport {
        schema_version: NNS_TOPOLOGY_GAPS_REPORT_SCHEMA_VERSION,
        network,
        source_endpoint,
        status,
        gap_count,
        gaps,
    }
}

fn topology_gap_row(
    subject_kind: &str,
    subject: &str,
    missing_relation: &str,
    referenced_id: &str,
) -> NnsTopologyGapRow {
    NnsTopologyGapRow {
        subject_kind: subject_kind.to_string(),
        subject: subject.to_string(),
        missing_relation: missing_relation.to_string(),
        referenced_id: referenced_id.to_string(),
    }
}

///
/// NnsTopologyHealthDerivedMetrics
///
struct NnsTopologyHealthDerivedMetrics {
    registry_source_count: usize,
    registry_version_min: Option<u64>,
    registry_version_max: Option<u64>,
    registry_versions_aligned: bool,
    stale_source_count: usize,
    known_join_count: usize,
    unknown_join_count: usize,
    join_coverage: String,
}

fn topology_health_derived_metrics(
    summary: &NnsTopologySummaryReport,
) -> NnsTopologyHealthDerivedMetrics {
    let registry_version_min = summary
        .registry_versions
        .iter()
        .map(|row| row.registry_version)
        .min();
    let registry_version_max = summary
        .registry_versions
        .iter()
        .map(|row| row.registry_version)
        .max();
    let known_join_count = known_join_count(summary);
    let unknown_join_count = unknown_join_count(summary);

    NnsTopologyHealthDerivedMetrics {
        registry_source_count: summary.registry_versions.len(),
        registry_version_min,
        registry_version_max,
        registry_versions_aligned: registry_version_min == registry_version_max,
        stale_source_count: summary
            .registry_versions
            .iter()
            .filter(|row| row.stale == Some(true))
            .count(),
        known_join_count,
        unknown_join_count,
        join_coverage: coverage_percent_text(known_join_count, unknown_join_count),
    }
}

fn topology_health_checks(
    summary: &NnsTopologySummaryReport,
    health: &NnsTopologyHealthDerivedMetrics,
) -> Vec<NnsTopologyHealthCheckRow> {
    vec![
        health_check_row(
            "registry_versions",
            health.registry_versions_aligned,
            registry_version_detail(
                health.registry_source_count,
                health.registry_version_min,
                health.registry_version_max,
                health.registry_versions_aligned,
            ),
        ),
        health_check_row(
            "cache_freshness",
            health.stale_source_count == 0,
            cache_freshness_detail(health.stale_source_count, summary),
        ),
        health_check_row(
            "join_coverage",
            health.unknown_join_count == 0,
            format!(
                "{} known, {} unknown ({})",
                health.known_join_count, health.unknown_join_count, health.join_coverage
            ),
        ),
    ]
}

fn health_check_row(check: &str, is_ok: bool, detail: String) -> NnsTopologyHealthCheckRow {
    NnsTopologyHealthCheckRow {
        check: check.to_string(),
        status: if is_ok { "ok" } else { "attention" }.to_string(),
        detail,
    }
}

fn registry_version_detail(
    source_count: usize,
    min: Option<u64>,
    max: Option<u64>,
    aligned: bool,
) -> String {
    match (min, max, aligned) {
        (Some(version), Some(_), true) => {
            format!("{source_count} sources at registry version {version}")
        }
        (Some(min), Some(max), false) => {
            format!("{source_count} sources span registry versions {min}..{max}")
        }
        _ => "no registry versions recorded".to_string(),
    }
}

fn cache_freshness_detail(stale_source_count: usize, summary: &NnsTopologySummaryReport) -> String {
    if stale_source_count == 0 {
        return "no stale topology sources".to_string();
    }
    if summary.subnet_catalog_stale {
        return format!(
            "{stale_source_count} stale source; subnet catalog {}",
            summary.subnet_catalog_stale_reason
        );
    }
    format!("{stale_source_count} stale source")
}

const fn known_join_count(report: &NnsTopologySummaryReport) -> usize {
    report
        .nodes_with_known_node_provider_count
        .saturating_add(report.nodes_with_known_node_operator_count)
        .saturating_add(report.nodes_with_known_data_center_count)
        .saturating_add(report.node_operators_with_known_node_provider_count)
        .saturating_add(report.node_operators_with_known_data_center_count)
}

const fn unknown_join_count(report: &NnsTopologySummaryReport) -> usize {
    report
        .nodes_with_unknown_node_provider_count
        .saturating_add(report.nodes_with_unknown_node_operator_count)
        .saturating_add(report.nodes_with_unknown_data_center_count)
        .saturating_add(report.node_operators_with_unknown_node_provider_count)
        .saturating_add(report.node_operators_with_unknown_data_center_count)
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

fn node_count_with_known_node_provider(
    report: &NnsNodeListReport,
    providers: &BTreeSet<&str>,
) -> usize {
    report
        .nodes
        .iter()
        .filter(|node| providers.contains(node.node_provider_principal.as_str()))
        .count()
}

fn node_count_with_known_node_operator(
    report: &NnsNodeListReport,
    operators: &BTreeSet<&str>,
) -> usize {
    report
        .nodes
        .iter()
        .filter(|node| operators.contains(node.node_operator_principal.as_str()))
        .count()
}

fn node_count_with_known_data_center(
    report: &NnsNodeListReport,
    data_centers: &BTreeSet<&str>,
) -> usize {
    report
        .nodes
        .iter()
        .filter(|node| data_centers.contains(node.data_center_id.as_str()))
        .count()
}

fn operator_count_with_known_node_provider(
    report: &NnsNodeOperatorListReport,
    providers: &BTreeSet<&str>,
) -> usize {
    report
        .node_operators
        .iter()
        .filter(|operator| providers.contains(operator.node_provider_principal.as_str()))
        .count()
}

fn operator_count_with_known_data_center(
    report: &NnsNodeOperatorListReport,
    data_centers: &BTreeSet<&str>,
) -> usize {
    report
        .node_operators
        .iter()
        .filter(|operator| data_centers.contains(operator.data_center_id.as_str()))
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

fn render_coverage_count_table(report: &NnsTopologyCoverageReport) -> String {
    let headers = ["FIELD", "VALUE"];
    let rows = [
        ["network".to_string(), report.network.clone()],
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
            NNS_NODE_SUBNET_KIND_CLOUD_ENGINE.to_string(),
            report.cloud_engine_subnet_count.to_string(),
            report.cloud_engine_node_count.to_string(),
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

fn render_summary_join_coverage_table(report: &NnsTopologySummaryReport) -> String {
    render_join_coverage_table(&[
        (
            "nodes -> node providers",
            report.nodes_with_known_node_provider_count,
            report.nodes_with_unknown_node_provider_count,
        ),
        (
            "nodes -> node operators",
            report.nodes_with_known_node_operator_count,
            report.nodes_with_unknown_node_operator_count,
        ),
        (
            "nodes -> data centers",
            report.nodes_with_known_data_center_count,
            report.nodes_with_unknown_data_center_count,
        ),
        (
            "node operators -> node providers",
            report.node_operators_with_known_node_provider_count,
            report.node_operators_with_unknown_node_provider_count,
        ),
        (
            "node operators -> data centers",
            report.node_operators_with_known_data_center_count,
            report.node_operators_with_unknown_data_center_count,
        ),
    ])
}

fn render_coverage_join_coverage_table(report: &NnsTopologyCoverageReport) -> String {
    render_join_coverage_table(&[
        (
            "nodes -> node providers",
            report.nodes_with_known_node_provider_count,
            report.nodes_with_unknown_node_provider_count,
        ),
        (
            "nodes -> node operators",
            report.nodes_with_known_node_operator_count,
            report.nodes_with_unknown_node_operator_count,
        ),
        (
            "nodes -> data centers",
            report.nodes_with_known_data_center_count,
            report.nodes_with_unknown_data_center_count,
        ),
        (
            "node operators -> node providers",
            report.node_operators_with_known_node_provider_count,
            report.node_operators_with_unknown_node_provider_count,
        ),
        (
            "node operators -> data centers",
            report.node_operators_with_known_data_center_count,
            report.node_operators_with_unknown_data_center_count,
        ),
    ])
}

fn render_join_coverage_table(rows: &[(&str, usize, usize)]) -> String {
    let headers = ["RELATION", "KNOWN", "UNKNOWN", "COVERAGE"];
    let rows = rows
        .iter()
        .map(|(link, known, unknown)| {
            [
                (*link).to_string(),
                known.to_string(),
                unknown.to_string(),
                coverage_percent_text(*known, *unknown),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Right,
        ColumnAlign::Right,
    ];
    render_table(&headers, &rows, &alignments)
}

fn render_health_check_table(rows: &[NnsTopologyHealthCheckRow]) -> String {
    let headers = ["CHECK", "STATUS", "DETAIL"];
    let rows = rows
        .iter()
        .map(|row| [row.check.clone(), row.status.clone(), row.detail.clone()])
        .collect::<Vec<_>>();
    let alignments = [ColumnAlign::Left, ColumnAlign::Left, ColumnAlign::Left];
    render_table(&headers, &rows, &alignments)
}

fn render_gaps_status_table(report: &NnsTopologyGapsReport) -> String {
    let headers = ["STATUS", "DETAIL"];
    let rows = [[report.status.clone(), "no topology join gaps".to_string()]];
    let alignments = [ColumnAlign::Left, ColumnAlign::Left];
    render_table(&headers, &rows, &alignments)
}

fn render_gaps_table(rows: &[NnsTopologyGapRow]) -> String {
    let headers = [
        "SUBJECT_KIND",
        "SUBJECT",
        "MISSING_RELATION",
        "REFERENCED_ID",
    ];
    let rows = rows
        .iter()
        .map(|row| {
            [
                row.subject_kind.clone(),
                row.subject.clone(),
                row.missing_relation.clone(),
                row.referenced_id.clone(),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Left,
    ];
    render_table(&headers, &rows, &alignments)
}

fn coverage_percent_text(known: usize, unknown: usize) -> String {
    let total = known.saturating_add(unknown);
    if total == 0 {
        return "-".to_string();
    }
    let tenths = known.saturating_mul(1000).saturating_add(total / 2) / total;
    format!("{}.{:01}%", tenths / 10, tenths % 10)
}

fn render_registry_version_table(rows: &[NnsTopologyRegistryVersionRow]) -> String {
    let headers = ["SOURCE", "VERSION", "FETCHED_AT", "STALE", "ENDPOINT"];
    let rows = rows
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
