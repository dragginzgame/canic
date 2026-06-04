use crate::{
    subnet_catalog::format_utc_timestamp_secs,
    table::{ColumnAlign, render_table},
};
use canic_ic_registry::{
    DEFAULT_MAINNET_ENDPOINT, MainnetNodeProviderList, MainnetRegistryFetchRequest,
    RegistryFetchError, fetch_mainnet_node_provider_list,
};
use canic_subnet_catalog::{MAINNET_NETWORK, canonical_principal_text};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

pub const DEFAULT_NNS_GOVERNANCE_SOURCE_ENDPOINT: &str = DEFAULT_MAINNET_ENDPOINT;
pub const NNS_NODE_PROVIDER_LIST_REPORT_SCHEMA_VERSION: u32 = 1;
pub const NNS_NODE_PROVIDER_INFO_REPORT_SCHEMA_VERSION: u32 = 1;
const COMPACT_PRINCIPAL_CHARS: usize = 5;

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
/// NnsNodeProviderInfoRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NnsNodeProviderInfoRequest {
    pub network: String,
    pub source_endpoint: String,
    pub input: String,
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
    pub name: Option<String>,
    pub node_count: Option<u32>,
    pub reward_account_hex: Option<String>,
}

///
/// NnsNodeProviderInfoReport
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NnsNodeProviderInfoReport {
    pub schema_version: u32,
    pub input: String,
    pub resolved_from: String,
    pub network: String,
    pub governance_canister_id: String,
    pub fetched_at: String,
    pub source_endpoint: String,
    pub fetched_by: String,
    pub node_provider_principal: String,
    pub name: Option<String>,
    pub node_count: Option<u32>,
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

    #[error("node provider {input:?} did not match the mainnet NNS node-provider list")]
    NodeProviderNotFound { input: String },

    #[error("node-provider prefix {prefix:?} is ambiguous; matches: {matches:?}")]
    AmbiguousNodeProviderPrefix {
        prefix: String,
        matches: Vec<String>,
    },
}

pub fn build_nns_node_provider_list_report(
    request: &NnsNodeProviderListRequest,
) -> Result<NnsNodeProviderListReport, NnsNodeProviderHostError> {
    build_nns_node_provider_list_report_with_source(request, &LiveNnsNodeProviderSource)
}

pub fn build_nns_node_provider_info_report(
    request: &NnsNodeProviderInfoRequest,
) -> Result<NnsNodeProviderInfoReport, NnsNodeProviderHostError> {
    build_nns_node_provider_info_report_with_source(request, &LiveNnsNodeProviderSource)
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

fn build_nns_node_provider_info_report_with_source(
    request: &NnsNodeProviderInfoRequest,
    source: &dyn NnsNodeProviderSource,
) -> Result<NnsNodeProviderInfoReport, NnsNodeProviderHostError> {
    let list_request = NnsNodeProviderListRequest {
        network: request.network.clone(),
        source_endpoint: request.source_endpoint.clone(),
        now_unix_secs: request.now_unix_secs,
    };
    let report = build_nns_node_provider_list_report_with_source(&list_request, source)?;
    let (provider, resolved_from) = resolve_node_provider(&report, &request.input)?;
    Ok(NnsNodeProviderInfoReport {
        schema_version: NNS_NODE_PROVIDER_INFO_REPORT_SCHEMA_VERSION,
        input: request.input.clone(),
        resolved_from,
        network: report.network,
        governance_canister_id: report.governance_canister_id,
        fetched_at: report.fetched_at,
        source_endpoint: report.source_endpoint,
        fetched_by: report.fetched_by,
        node_provider_principal: provider.node_provider_principal,
        name: provider.name,
        node_count: provider.node_count,
        reward_account_hex: provider.reward_account_hex,
    })
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

    let headers = ["NODE_PROVIDER", "NAME", "NODES"];
    let rows = report
        .node_providers
        .iter()
        .map(|provider| {
            [
                compact_principal(&provider.node_provider_principal),
                text_or_dash(provider.name.as_deref()).to_string(),
                node_count_text(provider.node_count),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [ColumnAlign::Left, ColumnAlign::Left, ColumnAlign::Right];
    lines.push(render_table(&headers, &rows, &alignments));
    lines.join("\n")
}

#[must_use]
pub fn nns_node_provider_list_report_verbose_text(report: &NnsNodeProviderListReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!("source_endpoint: {}", report.source_endpoint));
    lines.push(format!("fetched_by: {}", report.fetched_by));
    if report.node_providers.is_empty() {
        lines.push("node providers: none".to_string());
        return lines.join("\n");
    }

    let headers = [
        "NODE_PROVIDER",
        "NAME",
        "NODES",
        "REWARD_ACCOUNT",
        "FETCHED_AT",
    ];
    let rows = report
        .node_providers
        .iter()
        .map(|provider| {
            [
                provider.node_provider_principal.clone(),
                text_or_dash(provider.name.as_deref()).to_string(),
                node_count_text(provider.node_count),
                text_or_dash(provider.reward_account_hex.as_deref()).to_string(),
                report.fetched_at.clone(),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Left,
        ColumnAlign::Left,
    ];
    lines.push(render_table(&headers, &rows, &alignments));
    lines.join("\n")
}

#[must_use]
pub fn nns_node_provider_info_report_text(report: &NnsNodeProviderInfoReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!("input: {}", report.input));
    lines.push(format!("resolved_from: {}", report.resolved_from));
    lines.push(format!(
        "node_provider_principal: {}",
        report.node_provider_principal
    ));
    lines.push(format!("name: {}", text_or_dash(report.name.as_deref())));
    lines.push(format!(
        "node_count: {}",
        node_count_text(report.node_count)
    ));
    lines.push(format!(
        "reward_account_hex: {}",
        text_or_dash(report.reward_account_hex.as_deref())
    ));
    lines.push(format!(
        "governance_canister_id: {}",
        report.governance_canister_id
    ));
    lines.push(format!("network: {}", report.network));
    lines.push(format!("fetched_at: {}", report.fetched_at));
    lines.push(format!("source_endpoint: {}", report.source_endpoint));
    lines.push(format!("fetched_by: {}", report.fetched_by));
    lines.join("\n")
}

fn node_provider_report_from_list(list: MainnetNodeProviderList) -> NnsNodeProviderListReport {
    let node_providers = list
        .node_providers
        .into_iter()
        .map(|provider| NnsNodeProviderRow {
            node_provider_principal: provider.principal,
            name: None,
            node_count: None,
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

fn resolve_node_provider(
    report: &NnsNodeProviderListReport,
    input: &str,
) -> Result<(NnsNodeProviderRow, String), NnsNodeProviderHostError> {
    if let Ok(principal) = canonical_principal_text(input)
        && let Some(provider) = report
            .node_providers
            .iter()
            .find(|provider| provider.node_provider_principal == principal)
    {
        return Ok((provider.clone(), "node_provider_principal".to_string()));
    }

    let prefix = input.trim().to_ascii_lowercase();
    if prefix.is_empty() {
        return Err(NnsNodeProviderHostError::NodeProviderNotFound {
            input: input.to_string(),
        });
    }
    let matches = report
        .node_providers
        .iter()
        .filter(|provider| provider.node_provider_principal.starts_with(&prefix))
        .cloned()
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [provider] => Ok((
            provider.clone(),
            "node_provider_principal_prefix".to_string(),
        )),
        [] => Err(NnsNodeProviderHostError::NodeProviderNotFound {
            input: input.to_string(),
        }),
        _ => Err(NnsNodeProviderHostError::AmbiguousNodeProviderPrefix {
            prefix,
            matches: matches
                .into_iter()
                .map(|provider| provider.node_provider_principal)
                .collect(),
        }),
    }
}

fn compact_principal(value: &str) -> String {
    value.chars().take(COMPACT_PRINCIPAL_CHARS).collect()
}

fn node_count_text(value: Option<u32>) -> String {
    value.map_or_else(|| "unknown".to_string(), |count| count.to_string())
}

fn text_or_dash(value: Option<&str>) -> &str {
    value.filter(|text| !text.is_empty()).unwrap_or("-")
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
        assert_eq!(report.node_providers[0].name, None);
        assert_eq!(report.node_providers[0].node_count, None);
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
                name: Some("DFINITY".to_string()),
                node_count: Some(13),
                reward_account_hex: Some("abcd".to_string()),
            }],
        };

        let text = nns_node_provider_list_report_text(&report);

        assert!(text.contains("node_providers: ic count 1"));
        assert!(text.contains("NODE_PROVIDER"));
        assert!(text.contains("ryjl3"));
        assert!(text.contains("DFINITY"));
        assert!(text.contains("13"));
        assert!(!text.contains("ryjl3-tyaaa-aaaaa-aaaba-cai"));
        assert!(!text.contains("abcd"));
    }

    #[test]
    fn node_provider_verbose_text_keeps_full_metadata() {
        let report = node_provider_report_fixture();

        let text = nns_node_provider_list_report_verbose_text(&report);

        assert!(text.contains("source_endpoint: https://icp-api.io"));
        assert!(text.contains("ryjl3-tyaaa-aaaaa-aaaba-cai"));
        assert!(text.contains("abcd"));
        assert!(text.contains("FETCHED_AT"));
    }

    #[test]
    fn node_provider_info_resolves_exact_principal() {
        let request = NnsNodeProviderInfoRequest {
            network: MAINNET_NETWORK.to_string(),
            source_endpoint: "https://icp-api.io".to_string(),
            input: "ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
            now_unix_secs: 1_780_531_200,
        };
        let report = build_nns_node_provider_info_report_with_source(
            &request,
            &FixtureNodeProviderSource {
                node_providers: vec![MainnetNodeProvider {
                    principal: "ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
                    reward_account_hex: Some("abcd".to_string()),
                }],
            },
        )
        .expect("node provider info");

        assert_eq!(report.input, "ryjl3-tyaaa-aaaaa-aaaba-cai");
        assert_eq!(report.resolved_from, "node_provider_principal");
        assert_eq!(
            report.node_provider_principal,
            "ryjl3-tyaaa-aaaaa-aaaba-cai"
        );
        assert_eq!(report.reward_account_hex.as_deref(), Some("abcd"));
    }

    #[test]
    fn node_provider_info_resolves_unique_prefix() {
        let report = node_provider_report_fixture();

        let (provider, resolved_from) =
            resolve_node_provider(&report, "ryjl").expect("prefix resolves");

        assert_eq!(resolved_from, "node_provider_principal_prefix");
        assert_eq!(
            provider.node_provider_principal,
            "ryjl3-tyaaa-aaaaa-aaaba-cai"
        );
    }

    #[test]
    fn node_provider_info_rejects_ambiguous_prefix() {
        let report = NnsNodeProviderListReport {
            schema_version: 1,
            network: MAINNET_NETWORK.to_string(),
            governance_canister_id: MAINNET_GOVERNANCE_CANISTER_ID.to_string(),
            fetched_at: "2026-06-04T00:00:00Z".to_string(),
            source_endpoint: "https://icp-api.io".to_string(),
            fetched_by: "test".to_string(),
            node_provider_count: 2,
            node_providers: vec![
                NnsNodeProviderRow {
                    node_provider_principal: "ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
                    name: None,
                    node_count: None,
                    reward_account_hex: None,
                },
                NnsNodeProviderRow {
                    node_provider_principal: "rwlgt-iiaaa-aaaaa-aaaaa-cai".to_string(),
                    name: None,
                    node_count: None,
                    reward_account_hex: None,
                },
            ],
        };

        let err = resolve_node_provider(&report, "r").expect_err("ambiguous");

        assert!(matches!(
            err,
            NnsNodeProviderHostError::AmbiguousNodeProviderPrefix { prefix, matches }
                if prefix == "r" && matches.len() == 2
        ));
    }

    #[test]
    fn node_provider_info_text_renders_detail_lines() {
        let report = NnsNodeProviderInfoReport {
            schema_version: 1,
            input: "ryjl".to_string(),
            resolved_from: "node_provider_principal_prefix".to_string(),
            network: MAINNET_NETWORK.to_string(),
            governance_canister_id: MAINNET_GOVERNANCE_CANISTER_ID.to_string(),
            fetched_at: "2026-06-04T00:00:00Z".to_string(),
            source_endpoint: "https://icp-api.io".to_string(),
            fetched_by: "test".to_string(),
            node_provider_principal: "ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
            name: None,
            node_count: None,
            reward_account_hex: Some("abcd".to_string()),
        };

        let text = nns_node_provider_info_report_text(&report);

        assert!(text.contains("resolved_from: node_provider_principal_prefix"));
        assert!(text.contains("node_provider_principal: ryjl3-tyaaa-aaaaa-aaaba-cai"));
        assert!(text.contains("name: -"));
        assert!(text.contains("node_count: unknown"));
        assert!(text.contains("reward_account_hex: abcd"));
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

    fn node_provider_report_fixture() -> NnsNodeProviderListReport {
        NnsNodeProviderListReport {
            schema_version: 1,
            network: MAINNET_NETWORK.to_string(),
            governance_canister_id: MAINNET_GOVERNANCE_CANISTER_ID.to_string(),
            fetched_at: "2026-06-04T00:00:00Z".to_string(),
            source_endpoint: "https://icp-api.io".to_string(),
            fetched_by: "test".to_string(),
            node_provider_count: 2,
            node_providers: vec![
                NnsNodeProviderRow {
                    node_provider_principal: "aaaaa-aa".to_string(),
                    name: None,
                    node_count: None,
                    reward_account_hex: None,
                },
                NnsNodeProviderRow {
                    node_provider_principal: "ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
                    name: Some("DFINITY".to_string()),
                    node_count: Some(13),
                    reward_account_hex: Some("abcd".to_string()),
                },
            ],
        }
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
