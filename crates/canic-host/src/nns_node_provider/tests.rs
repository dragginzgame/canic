use super::*;
use canic_ic_registry::{MAINNET_GOVERNANCE_CANISTER_ID, MainnetNodeProvider};
use canic_subnet_catalog::MAINNET_REGISTRY_CANISTER_ID;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn node_provider_report_uses_live_governance_source() {
    let request = NnsNodeProviderListRequest {
        cache: test_cache_request(MAINNET_NETWORK, "uses-live-source"),
        source_endpoint: "https://icp-api.io".to_string(),
        now_unix_secs: 1_780_531_200,
    };
    let report = build_nns_node_provider_list_report_with_source(
        &request,
        &FixtureNodeProviderSource {
            node_providers: vec![
                MainnetNodeProvider {
                    principal: "aaaaa-aa".to_string(),
                    node_count: Some(3),
                    reward_account_hex: Some("abcd".to_string()),
                },
                MainnetNodeProvider {
                    principal: "ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
                    node_count: None,
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
    assert_eq!(report.registry_canister_id, MAINNET_REGISTRY_CANISTER_ID);
    assert_eq!(report.registry_version, 42);
    assert_eq!(report.fetched_at, "2026-06-04T00:00:00Z");
    assert_eq!(report.node_provider_count, 2);
    assert_eq!(report.node_providers[0].node_provider_principal, "aaaaa-aa");
    assert_eq!(report.node_providers[0].name, None);
    assert_eq!(report.node_providers[0].node_count, Some(3));
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
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 42,
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
    assert!(text.contains("13"));
    assert!(!text.contains("NAME"));
    assert!(!text.contains("DFINITY"));
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
    assert!(text.contains("42"));
    assert!(text.contains("FETCHED_AT"));
    assert!(!text.contains("NAME"));
    assert!(!text.contains("DFINITY"));
}

#[test]
fn node_provider_info_resolves_exact_principal() {
    let request = NnsNodeProviderInfoRequest {
        cache: test_cache_request(MAINNET_NETWORK, "info-exact"),
        source_endpoint: "https://icp-api.io".to_string(),
        input: "ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
        now_unix_secs: 1_780_531_200,
    };
    let report = build_nns_node_provider_info_report_with_source(
        &request,
        &FixtureNodeProviderSource {
            node_providers: vec![MainnetNodeProvider {
                principal: "ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
                node_count: Some(13),
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
    assert_eq!(report.node_count, Some(13));
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
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 42,
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
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 42,
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
    assert!(!text.contains("name:"));
    assert!(text.contains("node_count: unknown"));
    assert!(text.contains("reward_account_hex: abcd"));
    assert!(text.contains("registry_version: 42"));
}

#[test]
fn node_provider_list_rejects_local_network() {
    let request = NnsNodeProviderListRequest {
        cache: test_cache_request("local", "local-rejected"),
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

#[test]
fn node_provider_refresh_writes_cache_and_list_reads_it() {
    let cache = test_cache_request(MAINNET_NETWORK, "refresh-cache");
    let refresh_request = NnsNodeProviderRefreshRequest {
        cache: cache.clone(),
        source_endpoint: "https://icp-api.io".to_string(),
        now_unix_secs: 1_780_531_200,
        lock_stale_after_seconds: DEFAULT_NODE_PROVIDER_REFRESH_LOCK_STALE_SECONDS,
        dry_run: false,
        output_path: None,
    };
    let refresh_report = refresh_nns_node_provider_report_with_source(
        &refresh_request,
        &FixtureNodeProviderSource {
            node_providers: vec![MainnetNodeProvider {
                principal: "ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
                node_count: Some(13),
                reward_account_hex: Some("abcd".to_string()),
            }],
        },
    )
    .expect("refresh report");

    assert!(nns_node_provider_cache_path(&cache.icp_root, &cache.network).is_file());
    assert!(refresh_report.wrote_cache);
    assert_eq!(refresh_report.node_provider_count, 1);

    let list_request = NnsNodeProviderListRequest {
        cache,
        source_endpoint: "https://unused.example".to_string(),
        now_unix_secs: 1_780_531_300,
    };
    let report =
        build_nns_node_provider_list_report_with_source(&list_request, &FailingNodeProviderSource)
            .expect("cached report");

    assert_eq!(report.source_endpoint, "https://icp-api.io");
    assert_eq!(report.node_providers.len(), 1);
    assert_eq!(report.node_providers[0].node_count, Some(13));
}

fn node_provider_report_fixture() -> NnsNodeProviderListReport {
    NnsNodeProviderListReport {
        schema_version: 1,
        network: MAINNET_NETWORK.to_string(),
        governance_canister_id: MAINNET_GOVERNANCE_CANISTER_ID.to_string(),
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 42,
        fetched_at: "2026-06-04T00:00:00Z".to_string(),
        source_endpoint: "https://icp-api.io".to_string(),
        fetched_by: "test".to_string(),
        node_provider_count: 2,
        node_providers: vec![
            NnsNodeProviderRow {
                node_provider_principal: "aaaaa-aa".to_string(),
                name: None,
                node_count: Some(3),
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
            registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
            registry_version: 42,
            fetched_at: request.fetched_at.clone(),
            fetched_by: "test".to_string(),
            source_endpoint: request.endpoint.clone(),
            node_providers: self.node_providers.clone(),
        })
    }
}

///
/// FailingNodeProviderSource
///
struct FailingNodeProviderSource;

impl NnsNodeProviderSource for FailingNodeProviderSource {
    fn fetch_node_providers(
        &self,
        _request: &MainnetRegistryFetchRequest,
    ) -> Result<MainnetNodeProviderList, NnsNodeProviderHostError> {
        Err(NnsNodeProviderHostError::NodeProviderNotFound {
            input: "unexpected-live-fetch".to_string(),
        })
    }
}

fn test_cache_request(network: &str, name: &str) -> NnsNodeProviderCacheRequest {
    let count = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let icp_root = std::env::temp_dir().join(format!(
        "canic-node-provider-{name}-{}-{count}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&icp_root);
    NnsNodeProviderCacheRequest {
        icp_root,
        network: network.to_string(),
    }
}
