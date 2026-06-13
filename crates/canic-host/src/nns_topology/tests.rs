use super::*;
use crate::{
    nns_data_center::NnsDataCenterRow, nns_node::NnsNodeRow, nns_node_operator::NnsNodeOperatorRow,
    nns_node_provider::NnsNodeProviderRow, subnet_catalog::SubnetCatalogSubnetRow,
};
use canic_subnet_catalog::{
    ClassificationSource, GeographicScope, MAINNET_NETWORK, MAINNET_REGISTRY_CANISTER_ID,
    SubnetKind, SubnetSpecialization,
};

#[test]
fn topology_summary_counts_existing_reports() {
    let report = topology_summary_report_from_reports(
        MAINNET_NETWORK.to_string(),
        "https://icp-api.io".to_string(),
        subnet_report_fixture(),
        node_report_fixture(),
        node_provider_report_fixture(),
        node_operator_report_fixture(),
        data_center_report_fixture(),
    );

    assert_eq!(report.schema_version, 3);
    assert_eq!(report.subnet_count, 2);
    assert_eq!(report.application_subnet_count, 1);
    assert_eq!(report.cloud_engine_subnet_count, 0);
    assert_eq!(report.system_subnet_count, 1);
    assert_eq!(report.routing_range_count, 3);
    assert_eq!(report.node_count, 3);
    assert_eq!(report.application_node_count, 2);
    assert_eq!(report.cloud_engine_node_count, 0);
    assert_eq!(report.system_node_count, 1);
    assert_eq!(report.node_provider_count, 1);
    assert_eq!(report.node_operator_count, 2);
    assert_eq!(report.data_center_count, 1);
    assert_eq!(report.nodes_with_known_node_provider_count, 2);
    assert_eq!(report.nodes_with_unknown_node_provider_count, 1);
    assert_eq!(report.nodes_with_known_node_operator_count, 2);
    assert_eq!(report.nodes_with_unknown_node_operator_count, 1);
    assert_eq!(report.nodes_with_known_data_center_count, 2);
    assert_eq!(report.nodes_with_unknown_data_center_count, 1);
    assert_eq!(report.node_operators_with_known_node_provider_count, 1);
    assert_eq!(report.node_operators_with_unknown_node_provider_count, 1);
    assert_eq!(report.node_operators_with_known_data_center_count, 1);
    assert_eq!(report.node_operators_with_unknown_data_center_count, 1);
    assert_eq!(report.registry_versions.len(), 5);
}

#[test]
fn topology_summary_text_renders_count_and_version_tables() {
    let report = topology_summary_report_from_reports(
        MAINNET_NETWORK.to_string(),
        "https://icp-api.io".to_string(),
        subnet_report_fixture(),
        node_report_fixture(),
        node_provider_report_fixture(),
        node_operator_report_fixture(),
        data_center_report_fixture(),
    );

    let text = nns_topology_summary_report_text(&report);

    assert!(text.contains("topology: ic subnets 2 nodes 3"));
    assert!(text.contains("routing_ranges"));
    assert!(text.contains("KIND"));
    assert!(text.contains("nodes -> node providers"));
    assert!(text.contains("node operators -> data centers"));
    assert!(text.contains("COVERAGE"));
    assert!(text.contains("SOURCE"));
    assert!(text.contains("subnet_catalog"));
    assert!(text.contains("\n\n"));
}

#[test]
fn topology_coverage_report_projects_summary_join_coverage() {
    let summary = topology_summary_report_from_reports(
        MAINNET_NETWORK.to_string(),
        "https://icp-api.io".to_string(),
        subnet_report_fixture(),
        node_report_fixture(),
        node_provider_report_fixture(),
        node_operator_report_fixture(),
        data_center_report_fixture(),
    );

    let report = topology_coverage_report_from_summary(summary);

    assert_eq!(report.schema_version, 1);
    assert_eq!(report.network, MAINNET_NETWORK);
    assert_eq!(report.node_count, 3);
    assert_eq!(report.node_operator_count, 2);
    assert_eq!(report.node_provider_count, 1);
    assert_eq!(report.data_center_count, 1);
    assert_eq!(report.nodes_with_known_node_provider_count, 2);
    assert_eq!(report.nodes_with_unknown_node_provider_count, 1);
    assert_eq!(report.node_operators_with_known_data_center_count, 1);
    assert_eq!(report.node_operators_with_unknown_data_center_count, 1);
}

#[test]
fn topology_coverage_text_renders_join_coverage_table() {
    let summary = topology_summary_report_from_reports(
        MAINNET_NETWORK.to_string(),
        "https://icp-api.io".to_string(),
        subnet_report_fixture(),
        node_report_fixture(),
        node_provider_report_fixture(),
        node_operator_report_fixture(),
        data_center_report_fixture(),
    );
    let report = topology_coverage_report_from_summary(summary);

    let text = nns_topology_coverage_report_text(&report);

    assert!(text.contains("FIELD"));
    assert!(text.contains("RELATION"));
    assert!(text.contains("\n\n"));
    assert!(text.contains("nodes -> node providers"));
    assert!(text.contains("node operators -> data centers"));
    assert!(text.contains("66.7%"));
}

#[test]
fn topology_versions_report_projects_summary_registry_versions() {
    let summary = topology_summary_report_from_reports(
        MAINNET_NETWORK.to_string(),
        "https://icp-api.io".to_string(),
        subnet_report_fixture(),
        node_report_fixture(),
        node_provider_report_fixture(),
        node_operator_report_fixture(),
        data_center_report_fixture(),
    );

    let report = topology_versions_report_from_summary(summary);

    assert_eq!(report.schema_version, 1);
    assert_eq!(report.network, MAINNET_NETWORK);
    assert_eq!(report.source_count, 5);
    assert_eq!(report.registry_versions[0].source, "subnet_catalog");
    assert_eq!(report.registry_versions[1].source, "nodes");
}

#[test]
fn topology_versions_text_renders_registry_version_table() {
    let summary = topology_summary_report_from_reports(
        MAINNET_NETWORK.to_string(),
        "https://icp-api.io".to_string(),
        subnet_report_fixture(),
        node_report_fixture(),
        node_provider_report_fixture(),
        node_operator_report_fixture(),
        data_center_report_fixture(),
    );
    let report = topology_versions_report_from_summary(summary);

    let text = nns_topology_versions_report_text(&report);

    assert!(text.contains("SOURCE"));
    assert!(text.contains("VERSION"));
    assert!(text.contains("subnet_catalog"));
    assert!(text.contains("node_operators"));
    assert!(text.contains("data_centers"));
}

#[test]
fn topology_health_report_flags_mixed_versions_and_unknown_joins() {
    let summary = topology_summary_report_from_reports(
        MAINNET_NETWORK.to_string(),
        "https://icp-api.io".to_string(),
        subnet_report_fixture(),
        node_report_fixture(),
        node_provider_report_fixture(),
        node_operator_report_fixture(),
        data_center_report_fixture(),
    );

    let report = topology_health_report_from_summary(summary);

    assert_eq!(report.schema_version, 1);
    assert_eq!(report.status, "attention");
    assert_eq!(report.registry_source_count, 5);
    assert_eq!(report.registry_version_min, Some(42));
    assert_eq!(report.registry_version_max, Some(46));
    assert!(!report.registry_versions_aligned);
    assert_eq!(report.stale_source_count, 0);
    assert_eq!(report.known_join_count, 8);
    assert_eq!(report.unknown_join_count, 5);
    assert_eq!(report.join_coverage, "61.5%");
}

#[test]
fn topology_health_text_renders_check_table() {
    let summary = topology_summary_report_from_reports(
        MAINNET_NETWORK.to_string(),
        "https://icp-api.io".to_string(),
        subnet_report_fixture(),
        node_report_fixture(),
        node_provider_report_fixture(),
        node_operator_report_fixture(),
        data_center_report_fixture(),
    );
    let report = topology_health_report_from_summary(summary);

    let text = nns_topology_health_report_text(&report);

    assert!(text.contains("CHECK"));
    assert!(text.contains("registry_versions"));
    assert!(text.contains("attention"));
    assert!(text.contains("5 sources span registry versions 42..46"));
    assert!(text.contains("8 known, 5 unknown (61.5%)"));
}

#[test]
fn topology_gaps_report_lists_unknown_join_subjects() {
    let report = topology_gaps_report_from_reports(
        MAINNET_NETWORK.to_string(),
        "https://icp-api.io".to_string(),
        node_report_fixture(),
        node_provider_report_fixture(),
        node_operator_report_fixture(),
        data_center_report_fixture(),
    );

    assert_eq!(report.schema_version, 1);
    assert_eq!(report.status, "attention");
    assert_eq!(report.gap_count, 5);
    assert!(report.gaps.iter().any(|gap| {
        gap.subject_kind == "node"
            && gap.subject == "node-c"
            && gap.missing_relation == "node_provider"
            && gap.referenced_id == "provider-z"
    }));
    assert!(report.gaps.iter().any(|gap| {
        gap.subject_kind == "node"
            && gap.subject == "node-c"
            && gap.missing_relation == "node_operator"
            && gap.referenced_id == "operator-z"
    }));
    assert!(report.gaps.iter().any(|gap| {
        gap.subject_kind == "node_operator"
            && gap.subject == "operator-b"
            && gap.missing_relation == "data_center"
            && gap.referenced_id == "dc-z"
    }));
}

#[test]
fn topology_gaps_text_renders_gap_or_ok_tables() {
    let report = topology_gaps_report_from_reports(
        MAINNET_NETWORK.to_string(),
        "https://icp-api.io".to_string(),
        node_report_fixture(),
        node_provider_report_fixture(),
        node_operator_report_fixture(),
        data_center_report_fixture(),
    );

    let text = nns_topology_gaps_report_text(&report);

    assert!(text.contains("SUBJECT_KIND"));
    assert!(text.contains("MISSING_RELATION"));
    assert!(text.contains("node-c"));
    assert!(text.contains("provider-z"));

    let clean_report = topology_gaps_report_from_reports(
        MAINNET_NETWORK.to_string(),
        "https://icp-api.io".to_string(),
        node_report_fixture(),
        complete_node_provider_report_fixture(),
        complete_node_operator_report_fixture(),
        complete_data_center_report_fixture(),
    );
    let clean_text = nns_topology_gaps_report_text(&clean_report);

    assert_eq!(clean_report.status, "ok");
    assert_eq!(clean_report.gap_count, 0);
    assert!(clean_text.contains("STATUS"));
    assert!(clean_text.contains("no topology join gaps"));
}

#[test]
fn topology_summary_rejects_local_network_with_topology_hint() {
    let request = NnsTopologySummaryRequest {
        icp_root: std::env::temp_dir(),
        network: "local".to_string(),
        source_endpoint: "https://icp-api.io".to_string(),
        now_unix_secs: 1_780_531_200,
    };

    let err = build_nns_topology_summary_report(&request).expect_err("local rejected");
    let message = err.to_string();

    assert!(message.contains("supports only the mainnet `ic` network"));
    assert!(message.contains("canic --network ic nns topology summary"));
    assert!(message.contains("canic --network ic nns topology coverage"));
    assert!(message.contains("canic --network ic nns topology versions"));
    assert!(message.contains("canic --network ic nns topology health"));
    assert!(message.contains("canic --network ic nns topology gaps"));
    assert!(message.contains("canic --network ic nns topology refresh"));
}

#[test]
fn topology_refresh_counts_component_reports() {
    let report = topology_refresh_report_from_reports(
        MAINNET_NETWORK.to_string(),
        "https://icp-api.io".to_string(),
        false,
        NnsTopologyRefreshComponentReports {
            subnet: subnet_refresh_report_fixture(),
            node: node_refresh_report_fixture(),
            node_provider: node_provider_refresh_report_fixture(),
            node_operator: node_operator_refresh_report_fixture(),
            data_center: data_center_refresh_report_fixture(),
        },
    );

    assert_eq!(report.schema_version, 1);
    assert_eq!(report.component_count, 5);
    assert_eq!(report.wrote_cache_count, 5);
    assert_eq!(report.replaced_existing_cache_count, 1);
    assert_eq!(report.components[0].source, "subnet_catalog");
    assert_eq!(report.components[0].item_count, 2);
    assert_eq!(report.components[1].source, "nodes");
    assert_eq!(report.components[1].item_count, 3);
}

#[test]
fn topology_refresh_text_renders_component_table() {
    let report = topology_refresh_report_from_reports(
        MAINNET_NETWORK.to_string(),
        "https://icp-api.io".to_string(),
        true,
        NnsTopologyRefreshComponentReports {
            subnet: dry_run_subnet_refresh_report_fixture(),
            node: dry_run_node_refresh_report_fixture(),
            node_provider: dry_run_node_provider_refresh_report_fixture(),
            node_operator: dry_run_node_operator_refresh_report_fixture(),
            data_center: dry_run_data_center_refresh_report_fixture(),
        },
    );

    let text = nns_topology_refresh_report_text(&report);

    assert!(text.contains("topology_refresh: ic components 5 wrote 0 replaced 1 dry_run yes"));
    assert!(text.contains("subnet_catalog"));
    assert!(text.contains("node_operators"));
    assert!(text.contains("data_centers"));
}

fn subnet_report_fixture() -> SubnetCatalogListReport {
    SubnetCatalogListReport {
        schema_version: 1,
        network: MAINNET_NETWORK.to_string(),
        catalog_path: "catalog.json".to_string(),
        catalog_schema_version: 1,
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 42,
        fetched_at: "2026-06-04T00:00:00Z".to_string(),
        catalog_stale: false,
        stale_reason: "fresh".to_string(),
        resolver_backend: "local-nns-subnet-catalog".to_string(),
        subnets: vec![
            subnet_row("pzp6e", SubnetKind::Application, 2, 2),
            subnet_row("tdb26", SubnetKind::System, 1, 1),
        ],
    }
}

fn subnet_refresh_report_fixture() -> SubnetCatalogRefreshReport {
    SubnetCatalogRefreshReport {
        schema_version: 1,
        network: MAINNET_NETWORK.to_string(),
        catalog_path: ".canic/subnet-catalog/ic/catalog.json".to_string(),
        refresh_lock_path: ".canic/subnet-catalog/ic/refresh.lock".to_string(),
        output_path: None,
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 42,
        fetched_at: "2026-06-04T00:00:00Z".to_string(),
        source_endpoint: "https://icp-api.io".to_string(),
        fetched_by: "test".to_string(),
        dry_run: false,
        wrote_catalog: true,
        replaced_existing_catalog: true,
        subnet_count: 2,
        routing_range_count: 3,
    }
}

fn dry_run_subnet_refresh_report_fixture() -> SubnetCatalogRefreshReport {
    SubnetCatalogRefreshReport {
        dry_run: true,
        wrote_catalog: false,
        ..subnet_refresh_report_fixture()
    }
}

fn node_refresh_report_fixture() -> NnsNodeRefreshReport {
    NnsNodeRefreshReport {
        schema_version: 1,
        network: MAINNET_NETWORK.to_string(),
        cache_path: ".canic/node/ic/nodes.json".to_string(),
        refresh_lock_path: ".canic/node/ic/refresh.lock".to_string(),
        output_path: None,
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 43,
        fetched_at: "2026-06-04T00:01:00Z".to_string(),
        source_endpoint: "https://icp-api.io".to_string(),
        fetched_by: "test".to_string(),
        dry_run: false,
        wrote_cache: true,
        replaced_existing_cache: false,
        node_count: 3,
    }
}

fn dry_run_node_refresh_report_fixture() -> NnsNodeRefreshReport {
    NnsNodeRefreshReport {
        dry_run: true,
        wrote_cache: false,
        ..node_refresh_report_fixture()
    }
}

fn node_provider_refresh_report_fixture() -> NnsNodeProviderRefreshReport {
    NnsNodeProviderRefreshReport {
        schema_version: 1,
        network: MAINNET_NETWORK.to_string(),
        cache_path: ".canic/node-provider/ic/providers.json".to_string(),
        refresh_lock_path: ".canic/node-provider/ic/refresh.lock".to_string(),
        output_path: None,
        governance_canister_id: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 44,
        fetched_at: "2026-06-04T00:02:00Z".to_string(),
        source_endpoint: "https://icp-api.io".to_string(),
        fetched_by: "test".to_string(),
        dry_run: false,
        wrote_cache: true,
        replaced_existing_cache: false,
        node_provider_count: 1,
    }
}

fn dry_run_node_provider_refresh_report_fixture() -> NnsNodeProviderRefreshReport {
    NnsNodeProviderRefreshReport {
        dry_run: true,
        wrote_cache: false,
        ..node_provider_refresh_report_fixture()
    }
}

fn node_operator_refresh_report_fixture() -> NnsNodeOperatorRefreshReport {
    NnsNodeOperatorRefreshReport {
        schema_version: 1,
        network: MAINNET_NETWORK.to_string(),
        cache_path: ".canic/node-operator/ic/operators.json".to_string(),
        refresh_lock_path: ".canic/node-operator/ic/refresh.lock".to_string(),
        output_path: None,
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 45,
        fetched_at: "2026-06-04T00:03:00Z".to_string(),
        source_endpoint: "https://icp-api.io".to_string(),
        fetched_by: "test".to_string(),
        dry_run: false,
        wrote_cache: true,
        replaced_existing_cache: false,
        node_operator_count: 2,
    }
}

fn dry_run_node_operator_refresh_report_fixture() -> NnsNodeOperatorRefreshReport {
    NnsNodeOperatorRefreshReport {
        dry_run: true,
        wrote_cache: false,
        ..node_operator_refresh_report_fixture()
    }
}

fn data_center_refresh_report_fixture() -> NnsDataCenterRefreshReport {
    NnsDataCenterRefreshReport {
        schema_version: 1,
        network: MAINNET_NETWORK.to_string(),
        cache_path: ".canic/data-center/ic/data-centers.json".to_string(),
        refresh_lock_path: ".canic/data-center/ic/refresh.lock".to_string(),
        output_path: None,
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 46,
        fetched_at: "2026-06-04T00:04:00Z".to_string(),
        source_endpoint: "https://icp-api.io".to_string(),
        fetched_by: "test".to_string(),
        dry_run: false,
        wrote_cache: true,
        replaced_existing_cache: false,
        data_center_count: 1,
    }
}

fn dry_run_data_center_refresh_report_fixture() -> NnsDataCenterRefreshReport {
    NnsDataCenterRefreshReport {
        dry_run: true,
        wrote_cache: false,
        ..data_center_refresh_report_fixture()
    }
}

fn subnet_row(
    subnet_principal: &str,
    subnet_kind: SubnetKind,
    node_count: u32,
    range_count: usize,
) -> SubnetCatalogSubnetRow {
    SubnetCatalogSubnetRow {
        subnet_principal: subnet_principal.to_string(),
        subnet_kind,
        subnet_kind_source: ClassificationSource::Registry,
        subnet_specialization: SubnetSpecialization::None,
        subnet_specialization_source: ClassificationSource::Computed,
        geographic_scope: GeographicScope::Global,
        geographic_scope_source: ClassificationSource::Computed,
        subnet_label: subnet_kind.as_str().to_string(),
        subnet_label_source: ClassificationSource::Computed,
        node_count: Some(node_count),
        charges_apply_by_default: subnet_kind.charges_apply_by_default(),
        range_count,
        ranges_shown: 0,
        range_offset: 0,
        range_limit: 1,
        ranges: Vec::new(),
    }
}

fn node_report_fixture() -> NnsNodeListReport {
    NnsNodeListReport {
        schema_version: 1,
        network: MAINNET_NETWORK.to_string(),
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 43,
        fetched_at: "2026-06-04T00:01:00Z".to_string(),
        source_endpoint: "https://icp-api.io".to_string(),
        fetched_by: "test".to_string(),
        node_count: 3,
        nodes: vec![
            node_row("node-a", "operator-a", "provider-a", "dc1", "application"),
            node_row("node-b", "operator-a", "provider-a", "dc1", "application"),
            node_row("node-c", "operator-z", "provider-z", "dc-z", "system"),
        ],
    }
}

fn node_row(
    node_principal: &str,
    node_operator_principal: &str,
    node_provider_principal: &str,
    data_center_id: &str,
    subnet_kind: &str,
) -> NnsNodeRow {
    NnsNodeRow {
        node_principal: node_principal.to_string(),
        node_operator_principal: node_operator_principal.to_string(),
        node_provider_principal: node_provider_principal.to_string(),
        subnet_principal: "subnet-a".to_string(),
        subnet_kind: subnet_kind.to_string(),
        data_center_id: data_center_id.to_string(),
    }
}

fn node_provider_report_fixture() -> NnsNodeProviderListReport {
    NnsNodeProviderListReport {
        schema_version: 1,
        network: MAINNET_NETWORK.to_string(),
        governance_canister_id: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 44,
        fetched_at: "2026-06-04T00:02:00Z".to_string(),
        source_endpoint: "https://icp-api.io".to_string(),
        fetched_by: "test".to_string(),
        node_provider_count: 1,
        node_providers: vec![NnsNodeProviderRow {
            node_provider_principal: "provider-a".to_string(),
            name: None,
            node_count: Some(3),
            reward_account_hex: None,
        }],
    }
}

fn complete_node_provider_report_fixture() -> NnsNodeProviderListReport {
    let mut report = node_provider_report_fixture();
    report.node_provider_count = 2;
    report.node_providers.push(NnsNodeProviderRow {
        node_provider_principal: "provider-z".to_string(),
        name: None,
        node_count: Some(1),
        reward_account_hex: None,
    });
    report
}

fn node_operator_report_fixture() -> NnsNodeOperatorListReport {
    NnsNodeOperatorListReport {
        schema_version: 1,
        network: MAINNET_NETWORK.to_string(),
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 45,
        fetched_at: "2026-06-04T00:03:00Z".to_string(),
        source_endpoint: "https://icp-api.io".to_string(),
        fetched_by: "test".to_string(),
        node_operator_count: 2,
        node_operators: vec![
            NnsNodeOperatorRow {
                node_operator_principal: "operator-a".to_string(),
                node_provider_principal: "provider-a".to_string(),
                node_allowance: 1,
                data_center_id: "dc1".to_string(),
                node_count: Some(2),
            },
            NnsNodeOperatorRow {
                node_operator_principal: "operator-b".to_string(),
                node_provider_principal: "provider-z".to_string(),
                node_allowance: 1,
                data_center_id: "dc-z".to_string(),
                node_count: Some(1),
            },
        ],
    }
}

fn complete_node_operator_report_fixture() -> NnsNodeOperatorListReport {
    let mut report = node_operator_report_fixture();
    report.node_operator_count = 3;
    report.node_operators.push(NnsNodeOperatorRow {
        node_operator_principal: "operator-z".to_string(),
        node_provider_principal: "provider-z".to_string(),
        node_allowance: 1,
        data_center_id: "dc-z".to_string(),
        node_count: Some(1),
    });
    report
}

fn data_center_report_fixture() -> NnsDataCenterListReport {
    NnsDataCenterListReport {
        schema_version: 1,
        network: MAINNET_NETWORK.to_string(),
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 46,
        fetched_at: "2026-06-04T00:04:00Z".to_string(),
        source_endpoint: "https://icp-api.io".to_string(),
        fetched_by: "test".to_string(),
        data_center_count: 1,
        data_centers: vec![NnsDataCenterRow {
            data_center_id: "dc1".to_string(),
            region: "eu-west".to_string(),
            owner: "example".to_string(),
            latitude: None,
            longitude: None,
            node_operator_count: 2,
            node_provider_count: 1,
            node_count: 3,
        }],
    }
}

fn complete_data_center_report_fixture() -> NnsDataCenterListReport {
    let mut report = data_center_report_fixture();
    report.data_center_count = 2;
    report.data_centers.push(NnsDataCenterRow {
        data_center_id: "dc-z".to_string(),
        region: "eu-west".to_string(),
        owner: "example".to_string(),
        latitude: None,
        longitude: None,
        node_operator_count: 1,
        node_provider_count: 1,
        node_count: 1,
    });
    report
}
