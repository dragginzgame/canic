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
