use super::*;
use canic_ic_registry::MainnetNode;
use canic_subnet_catalog::MAINNET_REGISTRY_CANISTER_ID;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn node_report_uses_live_registry_source() {
    let request = NnsNodeListRequest {
        cache: test_cache_request(MAINNET_NETWORK, "uses-live-source"),
        source_endpoint: "https://icp-api.io".to_string(),
        now_unix_secs: 1_780_531_200,
        filters: NnsNodeListFilters::default(),
    };
    let report = build_nns_node_list_report_with_source(
        &request,
        &FixtureNodeSource {
            nodes: vec![node_fixture()],
        },
    )
    .expect("node report");

    assert_eq!(report.schema_version, 1);
    assert_eq!(report.network, MAINNET_NETWORK);
    assert_eq!(report.registry_canister_id, MAINNET_REGISTRY_CANISTER_ID);
    assert_eq!(report.registry_version, 42);
    assert_eq!(report.fetched_at, "2026-06-04T00:00:00Z");
    assert_eq!(report.node_count, 1);
    assert_eq!(
        report.nodes[0].node_principal,
        "ryjl3-tyaaa-aaaaa-aaaba-cai"
    );
}

#[test]
fn node_text_keeps_compact_principals() {
    let report = node_report_fixture();

    let text = nns_node_list_report_text(&report);

    assert!(text.contains("nodes: ic count 1"));
    assert!(text.contains("NODE"));
    assert!(text.contains("ryjl3"));
    assert!(!text.contains("ryjl3-tyaaa-aaaaa-aaaba-cai"));
}

#[test]
fn node_info_resolves_unique_prefix() {
    let report = node_report_fixture();

    let (node, resolved_from) = resolve_node(&report, "ryjl").expect("prefix resolves");

    assert_eq!(resolved_from, "node_principal_prefix");
    assert_eq!(node.node_principal, "ryjl3-tyaaa-aaaaa-aaaba-cai");
}

#[test]
fn node_list_filters_by_related_prefixes() {
    let mut report = node_report_fixture();
    report.node_count = 2;
    report.nodes.push(NnsNodeRow {
        node_principal: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
        node_operator_principal: "qaa6y-5yaaa-aaaaa-aaafa-cai".to_string(),
        node_provider_principal: "qaa6y-5yaaa-aaaaa-aaafa-cai".to_string(),
        subnet_principal: "tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe"
            .to_string(),
        subnet_kind: "system".to_string(),
        data_center_id: "dc2".to_string(),
    });

    let filtered = filter_node_list_report(
        report,
        &NnsNodeListFilters {
            subnet: Some("pzp6e".to_string()),
            subnet_kind: Some("application".to_string()),
            data_center: Some("dc".to_string()),
            node_provider: Some("rwlgt".to_string()),
            node_operator: Some("aaaaa-aa".to_string()),
        },
    );

    assert_eq!(filtered.node_count, 1);
    assert_eq!(
        filtered.nodes[0].node_principal,
        "ryjl3-tyaaa-aaaaa-aaaba-cai"
    );
}

fn node_report_fixture() -> NnsNodeListReport {
    NnsNodeListReport {
        schema_version: 1,
        network: MAINNET_NETWORK.to_string(),
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 42,
        fetched_at: "2026-06-04T00:00:00Z".to_string(),
        source_endpoint: "https://icp-api.io".to_string(),
        fetched_by: "test".to_string(),
        node_count: 1,
        nodes: vec![NnsNodeRow {
            node_principal: "ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
            node_operator_principal: "aaaaa-aa".to_string(),
            node_provider_principal: "rwlgt-iiaaa-aaaaa-aaaaa-cai".to_string(),
            subnet_principal: "pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae"
                .to_string(),
            subnet_kind: "application".to_string(),
            data_center_id: "dc1".to_string(),
        }],
    }
}

fn node_fixture() -> MainnetNode {
    MainnetNode {
        principal: "ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
        node_operator_principal: "aaaaa-aa".to_string(),
        node_provider_principal: "rwlgt-iiaaa-aaaaa-aaaaa-cai".to_string(),
        subnet_principal: "pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae"
            .to_string(),
        subnet_kind: "application".to_string(),
        data_center_id: "dc1".to_string(),
    }
}

fn test_cache_request(network: &str, name: &str) -> NnsNodeCacheRequest {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join("canic-host-nns-node-tests")
        .join(format!("{name}-{counter}"));
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove old test root");
    }
    NnsNodeCacheRequest {
        icp_root: root,
        network: network.to_string(),
    }
}

///
/// FixtureNodeSource
///
struct FixtureNodeSource {
    nodes: Vec<MainnetNode>,
}

impl NnsNodeSource for FixtureNodeSource {
    fn fetch_nodes(
        &self,
        request: &MainnetRegistryFetchRequest,
    ) -> Result<MainnetNodeList, NnsNodeHostError> {
        Ok(MainnetNodeList {
            network: MAINNET_NETWORK.to_string(),
            registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
            registry_version: 42,
            fetched_at: request.fetched_at.clone(),
            fetched_by: request.fetched_by.clone(),
            source_endpoint: request.endpoint.clone(),
            nodes: self.nodes.clone(),
        })
    }
}
