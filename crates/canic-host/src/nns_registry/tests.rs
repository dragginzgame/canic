use super::*;
use canic_subnet_catalog::MAINNET_REGISTRY_CANISTER_ID;

#[test]
fn registry_version_report_uses_live_source_shape() {
    let request = NnsRegistryVersionRequest {
        network: MAINNET_NETWORK.to_string(),
        source_endpoint: "https://icp-api.io".to_string(),
        now_unix_secs: 1_780_531_200,
    };

    let report = build_nns_registry_version_report_with_source(&request, &FixtureNnsRegistrySource)
        .expect("registry version report");

    assert_eq!(report.schema_version, 1);
    assert_eq!(report.network, MAINNET_NETWORK);
    assert_eq!(report.registry_canister_id, MAINNET_REGISTRY_CANISTER_ID);
    assert_eq!(report.registry_version, 42);
    assert_eq!(report.fetched_at, "2026-06-04T00:00:00Z");
    assert_eq!(report.source_endpoint, "https://icp-api.io");
    assert_eq!(report.fetched_by, "canic-ic-registry");
}

#[test]
fn registry_version_text_is_key_value_output() {
    let report = NnsRegistryVersionReport {
        schema_version: 1,
        network: MAINNET_NETWORK.to_string(),
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 42,
        fetched_at: "2026-06-04T00:00:00Z".to_string(),
        source_endpoint: "https://icp-api.io".to_string(),
        fetched_by: "test".to_string(),
    };

    let text = nns_registry_version_report_text(&report);

    assert!(text.contains("network: ic"));
    assert!(text.contains("registry_canister_id: rwlgt-iiaaa-aaaaa-aaaaa-cai"));
    assert!(text.contains("registry_version: 42"));
    assert!(text.contains("fetched_at: 2026-06-04T00:00:00Z"));
}

///
/// FixtureNnsRegistrySource
///
struct FixtureNnsRegistrySource;

impl NnsRegistrySource for FixtureNnsRegistrySource {
    fn fetch_registry_version(
        &self,
        request: &MainnetRegistryFetchRequest,
    ) -> Result<MainnetRegistryVersion, NnsRegistryHostError> {
        Ok(MainnetRegistryVersion {
            network: MAINNET_NETWORK.to_string(),
            registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
            registry_version: 42,
            fetched_at: request.fetched_at.clone(),
            fetched_by: request.fetched_by.clone(),
            source_endpoint: request.endpoint.clone(),
        })
    }
}
