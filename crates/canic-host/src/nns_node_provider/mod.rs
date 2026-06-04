use crate::{
    subnet_catalog::format_utc_timestamp_secs,
    table::{ColumnAlign, render_table},
};
use canic_ic_registry::{
    DEFAULT_MAINNET_ENDPOINT, MainnetNodeProviderList, MainnetRegistryFetchRequest,
    RegistryFetchError, fetch_mainnet_node_provider_list,
};
use canic_subnet_catalog::MAINNET_NETWORK;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

pub const DEFAULT_NNS_GOVERNANCE_SOURCE_ENDPOINT: &str = DEFAULT_MAINNET_ENDPOINT;
pub const NNS_NODE_PROVIDER_LIST_REPORT_SCHEMA_VERSION: u32 = 1;

///
/// NnsNodeProviderListRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsNodeProviderListRequest {
    pub network: String,
    pub source_endpoint: String,
    pub now_unix_secs: u64,
}

///
/// NnsNodeProviderListReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsNodeProviderListReport {
    pub schema_version: u32,
    pub network: String,
    pub governance_canister_id: String,
    pub fetched_at: String,
    pub source_endpoint: String,
    pub fetched_by: String,
    pub node_provider_count: usize,
    pub node_providers: Vec<NnsNodeProviderRow>,
}

///
/// NnsNodeProviderRow
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsNodeProviderRow {
    pub node_provider_principal: String,
    pub reward_account_hex: Option<String>,
}

///
/// NnsNodeProviderHostError
///
#[derive(Debug, ThisError)]
pub enum NnsNodeProviderHostError {
    #[error(
        "`canic nns node-provider` supports only the mainnet `ic` network in 0.60\n\nThe NNS node-provider list is queried from the public Internet Computer mainnet governance canister.\nLocal replica NNS governance discovery is not implemented yet.\n\nTry:\n  canic --network ic nns node-provider list"
    )]
    UnsupportedNetwork { network: String },

    #[error("live NNS governance node-provider query failed: {0}")]
    GovernanceQuery(#[from] RegistryFetchError),
}

pub fn build_nns_node_provider_list_report(
    request: &NnsNodeProviderListRequest,
) -> Result<NnsNodeProviderListReport, NnsNodeProviderHostError> {
    build_nns_node_provider_list_report_with_source(request, &LiveNnsNodeProviderSource)
}

fn build_nns_node_provider_list_report_with_source(
    request: &NnsNodeProviderListRequest,
    source: &dyn NnsNodeProviderSource,
) -> Result<NnsNodeProviderListReport, NnsNodeProviderHostError> {
    enforce_mainnet_network(&request.network)?;
    let fetched_at = format_utc_timestamp_secs(request.now_unix_secs);
    let mut fetch_request = MainnetRegistryFetchRequest::new(fetched_at);
    fetch_request.endpoint.clone_from(&request.source_endpoint);
    let list = source.fetch_node_providers(&fetch_request)?;
    Ok(node_provider_report_from_list(list))
}

#[must_use]
pub fn nns_node_provider_list_report_text(report: &NnsNodeProviderListReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "node_providers: {} count {} fetched_at {}",
        report.network, report.node_provider_count, report.fetched_at
    ));
    if report.node_providers.is_empty() {
        lines.push("node providers: none".to_string());
        return lines.join("\n");
    }

    let headers = ["#", "NODE_PROVIDER"];
    let rows = report
        .node_providers
        .iter()
        .enumerate()
        .map(|(index, provider)| {
            [
                (index + 1).to_string(),
                provider.node_provider_principal.clone(),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [ColumnAlign::Right, ColumnAlign::Left];
    lines.push(render_table(&headers, &rows, &alignments));
    lines.join("\n")
}

fn node_provider_report_from_list(list: MainnetNodeProviderList) -> NnsNodeProviderListReport {
    let node_providers = list
        .node_providers
        .into_iter()
        .map(|provider| NnsNodeProviderRow {
            node_provider_principal: provider.principal,
            reward_account_hex: provider.reward_account_hex,
        })
        .collect::<Vec<_>>();
    NnsNodeProviderListReport {
        schema_version: NNS_NODE_PROVIDER_LIST_REPORT_SCHEMA_VERSION,
        network: list.network,
        governance_canister_id: list.governance_canister_id,
        fetched_at: list.fetched_at,
        source_endpoint: list.source_endpoint,
        fetched_by: list.fetched_by,
        node_provider_count: node_providers.len(),
        node_providers,
    }
}

///
/// NnsNodeProviderSource
///
trait NnsNodeProviderSource {
    fn fetch_node_providers(
        &self,
        request: &MainnetRegistryFetchRequest,
    ) -> Result<MainnetNodeProviderList, NnsNodeProviderHostError>;
}

fn enforce_mainnet_network(network: &str) -> Result<(), NnsNodeProviderHostError> {
    if network == MAINNET_NETWORK {
        return Ok(());
    }
    Err(NnsNodeProviderHostError::UnsupportedNetwork {
        network: network.to_string(),
    })
}

///
/// LiveNnsNodeProviderSource
///
struct LiveNnsNodeProviderSource;

impl NnsNodeProviderSource for LiveNnsNodeProviderSource {
    fn fetch_node_providers(
        &self,
        request: &MainnetRegistryFetchRequest,
    ) -> Result<MainnetNodeProviderList, NnsNodeProviderHostError> {
        Ok(fetch_mainnet_node_provider_list(request)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_ic_registry::{MAINNET_GOVERNANCE_CANISTER_ID, MainnetNodeProvider};

    #[test]
    fn node_provider_report_uses_live_governance_source() {
        let request = NnsNodeProviderListRequest {
            network: MAINNET_NETWORK.to_string(),
            source_endpoint: "https://icp-api.io".to_string(),
            now_unix_secs: 1_780_531_200,
        };
        let report = build_nns_node_provider_list_report_with_source(
            &request,
            &FixtureNodeProviderSource {
                node_providers: vec![
                    MainnetNodeProvider {
                        principal: "aaaaa-aa".to_string(),
                        reward_account_hex: Some("abcd".to_string()),
                    },
                    MainnetNodeProvider {
                        principal: "ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
                        reward_account_hex: None,
                    },
                ],
            },
        )
        .expect("node provider report");

        assert_eq!(report.schema_version, 1);
        assert_eq!(report.network, MAINNET_NETWORK);
        assert_eq!(
            report.governance_canister_id,
            MAINNET_GOVERNANCE_CANISTER_ID
        );
        assert_eq!(report.fetched_at, "2026-06-04T00:00:00Z");
        assert_eq!(report.node_provider_count, 2);
        assert_eq!(report.node_providers[0].node_provider_principal, "aaaaa-aa");
        assert_eq!(
            report.node_providers[0].reward_account_hex.as_deref(),
            Some("abcd")
        );
    }

    #[test]
    fn node_provider_text_keeps_table_narrow() {
        let report = NnsNodeProviderListReport {
            schema_version: 1,
            network: MAINNET_NETWORK.to_string(),
            governance_canister_id: MAINNET_GOVERNANCE_CANISTER_ID.to_string(),
            fetched_at: "2026-06-04T00:00:00Z".to_string(),
            source_endpoint: "https://icp-api.io".to_string(),
            fetched_by: "test".to_string(),
            node_provider_count: 1,
            node_providers: vec![NnsNodeProviderRow {
                node_provider_principal: "ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
                reward_account_hex: Some("abcd".to_string()),
            }],
        };

        let text = nns_node_provider_list_report_text(&report);

        assert!(text.contains("node_providers: ic count 1"));
        assert!(text.contains("NODE_PROVIDER"));
        assert!(text.contains("ryjl3-tyaaa-aaaaa-aaaba-cai"));
        assert!(!text.contains("abcd"));
    }

    #[test]
    fn node_provider_list_rejects_local_network() {
        let request = NnsNodeProviderListRequest {
            network: "local".to_string(),
            source_endpoint: "https://icp-api.io".to_string(),
            now_unix_secs: 1,
        };

        let err = build_nns_node_provider_list_report_with_source(
            &request,
            &FixtureNodeProviderSource {
                node_providers: Vec::new(),
            },
        )
        .expect_err("local rejected");

        assert!(err.to_string().contains("supports only the mainnet `ic`"));
    }

    ///
    /// FixtureNodeProviderSource
    ///
    struct FixtureNodeProviderSource {
        node_providers: Vec<MainnetNodeProvider>,
    }

    impl NnsNodeProviderSource for FixtureNodeProviderSource {
        fn fetch_node_providers(
            &self,
            request: &MainnetRegistryFetchRequest,
        ) -> Result<MainnetNodeProviderList, NnsNodeProviderHostError> {
            Ok(MainnetNodeProviderList {
                network: MAINNET_NETWORK.to_string(),
                governance_canister_id: MAINNET_GOVERNANCE_CANISTER_ID.to_string(),
                fetched_at: request.fetched_at.clone(),
                fetched_by: "test".to_string(),
                source_endpoint: request.endpoint.clone(),
                node_providers: self.node_providers.clone(),
            })
        }
    }
}
