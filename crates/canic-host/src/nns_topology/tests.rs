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

    assert_eq!(report.schema_version, 1);
    assert_eq!(report.subnet_count, 2);
    assert_eq!(report.application_subnet_count, 1);
    assert_eq!(report.system_subnet_count, 1);
    assert_eq!(report.routing_range_count, 3);
    assert_eq!(report.node_count, 3);
    assert_eq!(report.application_node_count, 2);
    assert_eq!(report.system_node_count, 1);
    assert_eq!(report.node_provider_count, 1);
    assert_eq!(report.node_operator_count, 2);
    assert_eq!(report.data_center_count, 1);
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
    assert!(text.contains("subnet_kinds:"));
    assert!(text.contains("registry_versions:"));
    assert!(text.contains("subnet_catalog"));
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
        charges_apply_by_default: subnet_kind == SubnetKind::Application,
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
            node_row("node-a", "application"),
            node_row("node-b", "application"),
            node_row("node-c", "system"),
        ],
    }
}

fn node_row(node_principal: &str, subnet_kind: &str) -> NnsNodeRow {
    NnsNodeRow {
        node_principal: node_principal.to_string(),
        node_operator_principal: "operator-a".to_string(),
        node_provider_principal: "provider-a".to_string(),
        subnet_principal: "subnet-a".to_string(),
        subnet_kind: subnet_kind.to_string(),
        data_center_id: "dc1".to_string(),
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
                node_provider_principal: "provider-a".to_string(),
                node_allowance: 1,
                data_center_id: "dc1".to_string(),
                node_count: Some(1),
            },
        ],
    }
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
