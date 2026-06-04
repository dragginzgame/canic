use crate::subnet_catalog::format_utc_timestamp_secs;
use canic_ic_registry::{
    DEFAULT_MAINNET_ENDPOINT, MainnetRegistryFetchRequest, MainnetRegistryVersion,
    RegistryFetchError, fetch_mainnet_registry_version,
};
use canic_subnet_catalog::MAINNET_NETWORK;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

pub const DEFAULT_NNS_REGISTRY_SOURCE_ENDPOINT: &str = DEFAULT_MAINNET_ENDPOINT;
pub const NNS_REGISTRY_VERSION_REPORT_SCHEMA_VERSION: u32 = 1;

///
/// NnsRegistryVersionRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsRegistryVersionRequest {
    pub network: String,
    pub source_endpoint: String,
    pub now_unix_secs: u64,
}

///
/// NnsRegistryVersionReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsRegistryVersionReport {
    pub schema_version: u32,
    pub network: String,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub source_endpoint: String,
    pub fetched_by: String,
}

///
/// NnsRegistryHostError
///
#[derive(Debug, ThisError)]
pub enum NnsRegistryHostError {
    #[error(
        "`canic nns registry` supports only the mainnet `ic` network\n\nThe NNS registry inspected by this command is the public Internet Computer mainnet registry canister.\nLocal replica NNS registry discovery is not implemented yet.\n\nTry:\n  canic --network ic nns registry version"
    )]
    UnsupportedNetwork { network: String },

    #[error("live NNS registry query failed: {0}")]
    NnsQuery(#[from] RegistryFetchError),
}

pub fn build_nns_registry_version_report(
    request: &NnsRegistryVersionRequest,
) -> Result<NnsRegistryVersionReport, NnsRegistryHostError> {
    build_nns_registry_version_report_with_source(request, &LiveNnsRegistrySource)
}

fn build_nns_registry_version_report_with_source(
    request: &NnsRegistryVersionRequest,
    source: &dyn NnsRegistrySource,
) -> Result<NnsRegistryVersionReport, NnsRegistryHostError> {
    enforce_mainnet_network(&request.network)?;
    let fetched_at = format_utc_timestamp_secs(request.now_unix_secs);
    let mut fetch_request = MainnetRegistryFetchRequest::new(fetched_at);
    fetch_request.endpoint.clone_from(&request.source_endpoint);
    let version = source.fetch_registry_version(&fetch_request)?;
    Ok(registry_version_report_from_version(version))
}

#[must_use]
pub fn nns_registry_version_report_text(report: &NnsRegistryVersionReport) -> String {
    [
        format!("network: {}", report.network),
        format!("registry_canister_id: {}", report.registry_canister_id),
        format!("registry_version: {}", report.registry_version),
        format!("fetched_at: {}", report.fetched_at),
        format!("source_endpoint: {}", report.source_endpoint),
        format!("fetched_by: {}", report.fetched_by),
    ]
    .join("\n")
}

fn registry_version_report_from_version(
    version: MainnetRegistryVersion,
) -> NnsRegistryVersionReport {
    NnsRegistryVersionReport {
        schema_version: NNS_REGISTRY_VERSION_REPORT_SCHEMA_VERSION,
        network: version.network,
        registry_canister_id: version.registry_canister_id,
        registry_version: version.registry_version,
        fetched_at: version.fetched_at,
        source_endpoint: version.source_endpoint,
        fetched_by: version.fetched_by,
    }
}

///
/// NnsRegistrySource
///
trait NnsRegistrySource {
    fn fetch_registry_version(
        &self,
        request: &MainnetRegistryFetchRequest,
    ) -> Result<MainnetRegistryVersion, NnsRegistryHostError>;
}

fn enforce_mainnet_network(network: &str) -> Result<(), NnsRegistryHostError> {
    if network == MAINNET_NETWORK {
        return Ok(());
    }
    Err(NnsRegistryHostError::UnsupportedNetwork {
        network: network.to_string(),
    })
}

///
/// LiveNnsRegistrySource
///
struct LiveNnsRegistrySource;

impl NnsRegistrySource for LiveNnsRegistrySource {
    fn fetch_registry_version(
        &self,
        request: &MainnetRegistryFetchRequest,
    ) -> Result<MainnetRegistryVersion, NnsRegistryHostError> {
        Ok(fetch_mainnet_registry_version(request)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_subnet_catalog::MAINNET_REGISTRY_CANISTER_ID;

    #[test]
    fn registry_version_report_uses_live_source_shape() {
        let request = NnsRegistryVersionRequest {
            network: MAINNET_NETWORK.to_string(),
            source_endpoint: "https://icp-api.io".to_string(),
            now_unix_secs: 1_780_531_200,
        };

        let report =
            build_nns_registry_version_report_with_source(&request, &FixtureNnsRegistrySource)
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
}
