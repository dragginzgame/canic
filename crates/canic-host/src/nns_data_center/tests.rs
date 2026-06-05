use super::*;
use canic_ic_registry::MainnetDataCenter;
use canic_subnet_catalog::MAINNET_REGISTRY_CANISTER_ID;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn data_center_report_uses_live_registry_source() {
    let request = NnsDataCenterListRequest {
        cache: test_cache_request(MAINNET_NETWORK, "uses-live-source"),
        source_endpoint: "https://icp-api.io".to_string(),
        now_unix_secs: 1_780_531_200,
    };
    let report = build_nns_data_center_list_report_with_source(
        &request,
        &FixtureDataCenterSource {
            data_centers: vec![data_center_fixture()],
        },
    )
    .expect("data-center report");

    assert_eq!(report.schema_version, 1);
    assert_eq!(report.network, MAINNET_NETWORK);
    assert_eq!(report.registry_canister_id, MAINNET_REGISTRY_CANISTER_ID);
    assert_eq!(report.registry_version, 42);
    assert_eq!(report.fetched_at, "2026-06-04T00:00:00Z");
    assert_eq!(report.data_center_count, 1);
    assert_eq!(report.data_centers[0].data_center_id, "dc1");
}

#[test]
fn data_center_text_renders_relation_counts() {
    let report = data_center_report_fixture();

    let text = nns_data_center_list_report_text(&report);

    assert!(text.contains("data_centers: ic count 1"));
    assert!(text.contains("OPS"));
    assert!(text.contains("PROVIDERS"));
    assert!(text.contains("NODES"));
    assert!(text.contains("dc1"));
}

#[test]
fn data_center_info_resolves_unique_prefix() {
    let report = data_center_report_fixture();

    let (data_center, resolved_from) = resolve_data_center(&report, "dc").expect("prefix resolves");

    assert_eq!(resolved_from, "data_center_id_prefix");
    assert_eq!(data_center.data_center_id, "dc1");
}

fn data_center_report_fixture() -> NnsDataCenterListReport {
    NnsDataCenterListReport {
        schema_version: 1,
        network: MAINNET_NETWORK.to_string(),
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 42,
        fetched_at: "2026-06-04T00:00:00Z".to_string(),
        source_endpoint: "https://icp-api.io".to_string(),
        fetched_by: "test".to_string(),
        data_center_count: 1,
        data_centers: vec![NnsDataCenterRow {
            data_center_id: "dc1".to_string(),
            region: "eu-west".to_string(),
            owner: "example owner".to_string(),
            latitude: Some(48.8566),
            longitude: Some(2.3522),
            node_operator_count: 2,
            node_provider_count: 1,
            node_count: 13,
        }],
    }
}

fn data_center_fixture() -> MainnetDataCenter {
    MainnetDataCenter {
        id: "dc1".to_string(),
        region: "eu-west".to_string(),
        owner: "example owner".to_string(),
        latitude: Some(48.8566),
        longitude: Some(2.3522),
        node_operator_count: 2,
        node_provider_count: 1,
        node_count: 13,
    }
}

fn test_cache_request(network: &str, name: &str) -> NnsDataCenterCacheRequest {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join("canic-host-nns-data-center-tests")
        .join(format!("{name}-{counter}"));
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove old test root");
    }
    NnsDataCenterCacheRequest {
        icp_root: root,
        network: network.to_string(),
    }
}

///
/// FixtureDataCenterSource
///
struct FixtureDataCenterSource {
    data_centers: Vec<MainnetDataCenter>,
}

impl NnsDataCenterSource for FixtureDataCenterSource {
    fn fetch_data_centers(
        &self,
        request: &MainnetRegistryFetchRequest,
    ) -> Result<MainnetDataCenterList, NnsDataCenterHostError> {
        Ok(MainnetDataCenterList {
            network: MAINNET_NETWORK.to_string(),
            registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
            registry_version: 42,
            fetched_at: request.fetched_at.clone(),
            fetched_by: request.fetched_by.clone(),
            source_endpoint: request.endpoint.clone(),
            data_centers: self.data_centers.clone(),
        })
    }
}
