use super::*;
use super::{
    data_center::{
        data_center_info_options, data_center_info_usage, data_center_list_options,
        data_center_list_usage, data_center_refresh_options, data_center_refresh_usage,
        data_center_usage,
    },
    node::{
        node_info_options, node_info_usage, node_list_options, node_list_usage,
        node_refresh_options, node_refresh_usage, node_usage,
    },
    node_operator::{
        node_operator_info_options, node_operator_info_usage, node_operator_list_options,
        node_operator_list_usage, node_operator_refresh_options, node_operator_refresh_usage,
        node_operator_usage,
    },
    node_provider::{
        node_provider_info_options, node_provider_info_usage, node_provider_list_options,
        node_provider_list_usage, node_provider_refresh_options, node_provider_refresh_usage,
        node_provider_usage,
    },
    registry::{RegistryVersionOptions, registry_usage, registry_version_usage},
    subnet::{
        CatalogInfoOptions, CatalogListOptions, CatalogRefreshOptions, DEFAULT_RANGE_LIMIT,
        info_usage, list_usage, refresh_usage, resolve_canister_or_role,
        should_retry_info_as_deployment_target, subnet_usage,
    },
    topology::{
        TopologyRefreshOptions, TopologySummaryOptions, topology_refresh_usage,
        topology_summary_usage, topology_usage,
    },
};
use canic_host::{
    installed_deployment::{
        InstalledDeploymentRegistry, InstalledDeploymentResolution, InstalledDeploymentSource,
        ResolvedDeploymentTopology,
    },
    nns_data_center::{
        DEFAULT_DATA_CENTER_REFRESH_LOCK_STALE_SECONDS, DEFAULT_NNS_DATA_CENTER_SOURCE_ENDPOINT,
    },
    nns_node::{DEFAULT_NNS_NODE_SOURCE_ENDPOINT, DEFAULT_NODE_REFRESH_LOCK_STALE_SECONDS},
    nns_node_operator::{
        DEFAULT_NNS_NODE_OPERATOR_SOURCE_ENDPOINT, DEFAULT_NODE_OPERATOR_REFRESH_LOCK_STALE_SECONDS,
    },
    nns_node_provider::{
        DEFAULT_NNS_SOURCE_ENDPOINT, DEFAULT_NODE_PROVIDER_REFRESH_LOCK_STALE_SECONDS,
    },
    nns_registry::DEFAULT_NNS_REGISTRY_SOURCE_ENDPOINT,
    registry::RegistryEntry,
    subnet_catalog::SubnetCatalogHostError,
};
use canic_subnet_catalog::{
    CatalogError, GeographicScope, MAINNET_NETWORK, ResolveAs, SubnetKind, SubnetSpecialization,
};
use std::{collections::BTreeMap, ffi::OsString, path::PathBuf};

#[test]
fn list_defaults_to_mainnet_ic_catalog() {
    let options = CatalogListOptions::parse([]).expect("parse list");

    assert_eq!(options.network, MAINNET_NETWORK);
    assert_eq!(options.format, OutputFormat::Text);
    assert_eq!(options.range_limit, DEFAULT_RANGE_LIMIT);
    assert!(!options.verbose);
}

#[test]
fn list_parses_filters_and_json_format() {
    let options = CatalogListOptions::parse([
        OsString::from("--kind"),
        OsString::from("application"),
        OsString::from("--specialization"),
        OsString::from("fiduciary"),
        OsString::from("--geo"),
        OsString::from("global"),
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--show-ranges"),
        OsString::from("--verbose"),
        OsString::from("--range-limit"),
        OsString::from("12"),
    ])
    .expect("parse list");

    assert_eq!(options.filters.kind, Some(SubnetKind::Application));
    assert_eq!(
        options.filters.specialization,
        Some(SubnetSpecialization::Fiduciary)
    );
    assert_eq!(
        options.filters.geographic_scope,
        Some(GeographicScope::Global)
    );
    assert_eq!(options.format, OutputFormat::Json);
    assert!(options.show_ranges);
    assert!(options.verbose);
    assert_eq!(options.range_limit, 12);
}

#[test]
fn clap_rejects_invalid_nns_option_values() {
    std::assert_matches!(
        CatalogListOptions::parse([OsString::from("--kind"), OsString::from("subnet"),]),
        Err(NnsCommandError::Usage(_))
    );
    std::assert_matches!(
        CatalogListOptions::parse([OsString::from("--range-limit"), OsString::from("0"),]),
        Err(NnsCommandError::Usage(_))
    );
    std::assert_matches!(
        CatalogInfoOptions::parse([
            OsString::from("aaaaa-aa"),
            OsString::from("--as"),
            OsString::from("route"),
        ]),
        Err(NnsCommandError::Usage(_))
    );
}

#[test]
fn info_usage_names_deployment_target_input() {
    let text = info_usage();

    assert!(text.contains("subnet|canister|subnet-prefix|deployment-target"));
    assert!(text.contains("unique subnet prefix"));
    assert!(text.contains("canic nns subnet info <subnet-prefix>"));
    assert!(text.contains("--as <subnet|canister>"));
}

#[test]
fn list_and_info_help_hide_stale_policy_knobs() {
    let list = list_usage();
    let info = info_usage();

    assert!(!list.contains("--stale-after"));
    assert!(!list.contains("--allow-stale-subnet-catalog"));
    assert!(!info.contains("--stale-after"));
    assert!(!info.contains("--allow-stale-subnet-catalog"));
}

#[test]
fn deployment_target_fallback_only_follows_prefix_miss() {
    let mut options = CatalogInfoOptions {
        input: "backend".to_string(),
        network: MAINNET_NETWORK.to_string(),
        icp: "icp".to_string(),
        format: OutputFormat::Text,
        forced: None,
    };

    assert!(should_retry_info_as_deployment_target(
        &SubnetCatalogHostError::Catalog(CatalogError::PrincipalPrefixNotFound {
            prefix: "backend".to_string(),
        }),
        &options,
    ));
    assert!(!should_retry_info_as_deployment_target(
        &SubnetCatalogHostError::Catalog(CatalogError::AmbiguousPrincipalPrefix {
            prefix: "b".to_string(),
            matches: vec!["subnet:bbbb".to_string(), "subnet:bbbbb".to_string()],
        }),
        &options,
    ));

    options.forced = Some(ResolveAs::Subnet);
    assert!(!should_retry_info_as_deployment_target(
        &SubnetCatalogHostError::Catalog(CatalogError::PrincipalPrefixNotFound {
            prefix: "backend".to_string(),
        }),
        &options,
    ));
}

#[test]
fn refresh_parses_defaults_and_export_options() {
    let options = CatalogRefreshOptions::parse([
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--source-endpoint"),
        OsString::from("https://icp-api.io"),
        OsString::from("--lock-stale-after"),
        OsString::from("5m"),
        OsString::from("--dry-run"),
        OsString::from("--output"),
        OsString::from("catalog.preview.json"),
    ])
    .expect("parse refresh");

    assert_eq!(options.network, MAINNET_NETWORK);
    assert_eq!(options.format, OutputFormat::Json);
    assert_eq!(options.source_endpoint, "https://icp-api.io");
    assert_eq!(options.lock_stale_after_seconds, 300);
    assert!(options.dry_run);
    assert_eq!(
        options.output_path,
        Some(PathBuf::from("catalog.preview.json"))
    );
}

#[test]
fn node_provider_list_parses_defaults_and_json_format() {
    let defaults = node_provider_list_options([]).expect("parse defaults");

    assert_eq!(defaults.network, MAINNET_NETWORK);
    assert_eq!(defaults.format, OutputFormat::Text);
    assert_eq!(defaults.source_endpoint, DEFAULT_NNS_SOURCE_ENDPOINT);
    assert!(!defaults.verbose);

    let options = node_provider_list_options([
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--source-endpoint"),
        OsString::from("https://icp-api.io"),
        OsString::from("--verbose"),
    ])
    .expect("parse node-provider list");

    assert_eq!(options.format, OutputFormat::Json);
    assert_eq!(options.source_endpoint, "https://icp-api.io");
    assert!(options.verbose);
}

#[test]
fn node_provider_info_parses_input_and_json_format() {
    let options = node_provider_info_options([
        OsString::from("ryjl"),
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--source-endpoint"),
        OsString::from("https://icp-api.io"),
    ])
    .expect("parse node-provider info");

    assert_eq!(options.input, "ryjl");
    assert_eq!(options.network, MAINNET_NETWORK);
    assert_eq!(options.format, OutputFormat::Json);
    assert_eq!(options.source_endpoint, "https://icp-api.io");
}

#[test]
fn node_provider_refresh_parses_defaults_and_export_options() {
    let defaults = node_provider_refresh_options([]).expect("parse refresh defaults");

    assert_eq!(defaults.network, MAINNET_NETWORK);
    assert_eq!(defaults.format, OutputFormat::Text);
    assert_eq!(defaults.source_endpoint, DEFAULT_NNS_SOURCE_ENDPOINT);
    assert_eq!(
        defaults.lock_stale_after_seconds,
        DEFAULT_NODE_PROVIDER_REFRESH_LOCK_STALE_SECONDS
    );
    assert!(!defaults.dry_run);
    assert_eq!(defaults.output_path, None);

    let options = node_provider_refresh_options([
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--source-endpoint"),
        OsString::from("https://icp-api.io"),
        OsString::from("--lock-stale-after"),
        OsString::from("5m"),
        OsString::from("--dry-run"),
        OsString::from("--output"),
        OsString::from("providers.preview.json"),
    ])
    .expect("parse node-provider refresh");

    assert_eq!(options.format, OutputFormat::Json);
    assert_eq!(options.source_endpoint, "https://icp-api.io");
    assert_eq!(options.lock_stale_after_seconds, 300);
    assert!(options.dry_run);
    assert_eq!(
        options.output_path,
        Some(PathBuf::from("providers.preview.json"))
    );
}

#[test]
fn node_operator_list_parses_defaults_and_json_format() {
    let defaults = node_operator_list_options([]).expect("parse defaults");

    assert_eq!(defaults.network, MAINNET_NETWORK);
    assert_eq!(defaults.format, OutputFormat::Text);
    assert_eq!(
        defaults.source_endpoint,
        DEFAULT_NNS_NODE_OPERATOR_SOURCE_ENDPOINT
    );
    assert!(!defaults.verbose);

    let options = node_operator_list_options([
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--source-endpoint"),
        OsString::from("https://icp-api.io"),
        OsString::from("--verbose"),
    ])
    .expect("parse node-operator list");

    assert_eq!(options.format, OutputFormat::Json);
    assert_eq!(options.source_endpoint, "https://icp-api.io");
    assert!(options.verbose);
}

#[test]
fn node_operator_info_parses_input_and_json_format() {
    let options = node_operator_info_options([
        OsString::from("ryjl"),
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--source-endpoint"),
        OsString::from("https://icp-api.io"),
    ])
    .expect("parse node-operator info");

    assert_eq!(options.input, "ryjl");
    assert_eq!(options.network, MAINNET_NETWORK);
    assert_eq!(options.format, OutputFormat::Json);
    assert_eq!(options.source_endpoint, "https://icp-api.io");
}

#[test]
fn node_operator_refresh_parses_defaults_and_export_options() {
    let defaults = node_operator_refresh_options([]).expect("parse refresh defaults");

    assert_eq!(defaults.network, MAINNET_NETWORK);
    assert_eq!(defaults.format, OutputFormat::Text);
    assert_eq!(
        defaults.source_endpoint,
        DEFAULT_NNS_NODE_OPERATOR_SOURCE_ENDPOINT
    );
    assert_eq!(
        defaults.lock_stale_after_seconds,
        DEFAULT_NODE_OPERATOR_REFRESH_LOCK_STALE_SECONDS
    );
    assert!(!defaults.dry_run);
    assert_eq!(defaults.output_path, None);

    let options = node_operator_refresh_options([
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--source-endpoint"),
        OsString::from("https://icp-api.io"),
        OsString::from("--lock-stale-after"),
        OsString::from("5m"),
        OsString::from("--dry-run"),
        OsString::from("--output"),
        OsString::from("operators.preview.json"),
    ])
    .expect("parse node-operator refresh");

    assert_eq!(options.format, OutputFormat::Json);
    assert_eq!(options.source_endpoint, "https://icp-api.io");
    assert_eq!(options.lock_stale_after_seconds, 300);
    assert!(options.dry_run);
    assert_eq!(
        options.output_path,
        Some(PathBuf::from("operators.preview.json"))
    );
}

#[test]
fn node_list_parses_defaults_and_json_format() {
    let defaults = node_list_options([]).expect("parse defaults");

    assert_eq!(defaults.network, MAINNET_NETWORK);
    assert_eq!(defaults.format, OutputFormat::Text);
    assert_eq!(defaults.source_endpoint, DEFAULT_NNS_NODE_SOURCE_ENDPOINT);
    assert!(!defaults.verbose);

    let options = node_list_options([
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--source-endpoint"),
        OsString::from("https://icp-api.io"),
        OsString::from("--verbose"),
        OsString::from("--data-center"),
        OsString::from("zh2"),
        OsString::from("--node-provider"),
        OsString::from("7at4h"),
        OsString::from("--node-operator"),
        OsString::from("4lp6i"),
        OsString::from("--subnet"),
        OsString::from("tdb26"),
        OsString::from("--kind"),
        OsString::from("system"),
    ])
    .expect("parse node list");

    assert_eq!(options.format, OutputFormat::Json);
    assert_eq!(options.source_endpoint, "https://icp-api.io");
    assert!(options.verbose);
    assert_eq!(options.filters.data_center.as_deref(), Some("zh2"));
    assert_eq!(options.filters.node_provider.as_deref(), Some("7at4h"));
    assert_eq!(options.filters.node_operator.as_deref(), Some("4lp6i"));
    assert_eq!(options.filters.subnet.as_deref(), Some("tdb26"));
    assert_eq!(options.filters.subnet_kind.as_deref(), Some("system"));
}

#[test]
fn node_info_parses_input_and_json_format() {
    let options = node_info_options([
        OsString::from("ryjl"),
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--source-endpoint"),
        OsString::from("https://icp-api.io"),
    ])
    .expect("parse node info");

    assert_eq!(options.input, "ryjl");
    assert_eq!(options.network, MAINNET_NETWORK);
    assert_eq!(options.format, OutputFormat::Json);
    assert_eq!(options.source_endpoint, "https://icp-api.io");
}

#[test]
fn node_refresh_parses_defaults_and_export_options() {
    let defaults = node_refresh_options([]).expect("parse refresh defaults");

    assert_eq!(defaults.network, MAINNET_NETWORK);
    assert_eq!(defaults.format, OutputFormat::Text);
    assert_eq!(defaults.source_endpoint, DEFAULT_NNS_NODE_SOURCE_ENDPOINT);
    assert_eq!(
        defaults.lock_stale_after_seconds,
        DEFAULT_NODE_REFRESH_LOCK_STALE_SECONDS
    );
    assert!(!defaults.dry_run);
    assert_eq!(defaults.output_path, None);

    let options = node_refresh_options([
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--source-endpoint"),
        OsString::from("https://icp-api.io"),
        OsString::from("--lock-stale-after"),
        OsString::from("5m"),
        OsString::from("--dry-run"),
        OsString::from("--output"),
        OsString::from("nodes.preview.json"),
    ])
    .expect("parse node refresh");

    assert_eq!(options.format, OutputFormat::Json);
    assert_eq!(options.source_endpoint, "https://icp-api.io");
    assert_eq!(options.lock_stale_after_seconds, 300);
    assert!(options.dry_run);
    assert_eq!(
        options.output_path,
        Some(PathBuf::from("nodes.preview.json"))
    );
}

#[test]
fn data_center_list_parses_defaults_and_json_format() {
    let defaults = data_center_list_options([]).expect("parse defaults");

    assert_eq!(defaults.network, MAINNET_NETWORK);
    assert_eq!(defaults.format, OutputFormat::Text);
    assert_eq!(
        defaults.source_endpoint,
        DEFAULT_NNS_DATA_CENTER_SOURCE_ENDPOINT
    );
    assert!(!defaults.verbose);

    let options = data_center_list_options([
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--source-endpoint"),
        OsString::from("https://icp-api.io"),
        OsString::from("--verbose"),
    ])
    .expect("parse data-center list");

    assert_eq!(options.format, OutputFormat::Json);
    assert_eq!(options.source_endpoint, "https://icp-api.io");
    assert!(options.verbose);
}

#[test]
fn data_center_info_parses_input_and_json_format() {
    let options = data_center_info_options([
        OsString::from("an1"),
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--source-endpoint"),
        OsString::from("https://icp-api.io"),
    ])
    .expect("parse data-center info");

    assert_eq!(options.input, "an1");
    assert_eq!(options.network, MAINNET_NETWORK);
    assert_eq!(options.format, OutputFormat::Json);
    assert_eq!(options.source_endpoint, "https://icp-api.io");
}

#[test]
fn data_center_refresh_parses_defaults_and_export_options() {
    let defaults = data_center_refresh_options([]).expect("parse refresh defaults");

    assert_eq!(defaults.network, MAINNET_NETWORK);
    assert_eq!(defaults.format, OutputFormat::Text);
    assert_eq!(
        defaults.source_endpoint,
        DEFAULT_NNS_DATA_CENTER_SOURCE_ENDPOINT
    );
    assert_eq!(
        defaults.lock_stale_after_seconds,
        DEFAULT_DATA_CENTER_REFRESH_LOCK_STALE_SECONDS
    );
    assert!(!defaults.dry_run);
    assert_eq!(defaults.output_path, None);

    let options = data_center_refresh_options([
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--source-endpoint"),
        OsString::from("https://icp-api.io"),
        OsString::from("--lock-stale-after"),
        OsString::from("5m"),
        OsString::from("--dry-run"),
        OsString::from("--output"),
        OsString::from("data-centers.preview.json"),
    ])
    .expect("parse data-center refresh");

    assert_eq!(options.format, OutputFormat::Json);
    assert_eq!(options.source_endpoint, "https://icp-api.io");
    assert_eq!(options.lock_stale_after_seconds, 300);
    assert!(options.dry_run);
    assert_eq!(
        options.output_path,
        Some(PathBuf::from("data-centers.preview.json"))
    );
}

#[test]
fn registry_version_parses_defaults_and_json_format() {
    let defaults = RegistryVersionOptions::parse([]).expect("parse defaults");

    assert_eq!(defaults.network, MAINNET_NETWORK);
    assert_eq!(defaults.format, OutputFormat::Text);
    assert_eq!(
        defaults.source_endpoint,
        DEFAULT_NNS_REGISTRY_SOURCE_ENDPOINT
    );

    let options = RegistryVersionOptions::parse([
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--source-endpoint"),
        OsString::from("https://icp-api.io"),
    ])
    .expect("parse registry version");

    assert_eq!(options.format, OutputFormat::Json);
    assert_eq!(options.source_endpoint, "https://icp-api.io");
}

#[test]
fn topology_summary_parses_defaults_and_json_format() {
    let defaults = TopologySummaryOptions::parse([]).expect("parse defaults");

    assert_eq!(defaults.network, MAINNET_NETWORK);
    assert_eq!(defaults.format, OutputFormat::Text);
    assert_eq!(defaults.source_endpoint, DEFAULT_NNS_NODE_SOURCE_ENDPOINT);

    let options = TopologySummaryOptions::parse([
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--source-endpoint"),
        OsString::from("https://icp-api.io"),
    ])
    .expect("parse topology summary");

    assert_eq!(options.format, OutputFormat::Json);
    assert_eq!(options.source_endpoint, "https://icp-api.io");
}

#[test]
fn topology_refresh_parses_defaults_and_dry_run() {
    let defaults = TopologyRefreshOptions::parse([]).expect("parse defaults");

    assert_eq!(defaults.network, MAINNET_NETWORK);
    assert_eq!(defaults.format, OutputFormat::Text);
    assert_eq!(defaults.source_endpoint, DEFAULT_NNS_NODE_SOURCE_ENDPOINT);
    assert_eq!(defaults.lock_stale_after_seconds, 30 * 60);
    assert!(!defaults.dry_run);

    let options = TopologyRefreshOptions::parse([
        OsString::from("--format"),
        OsString::from("json"),
        OsString::from("--source-endpoint"),
        OsString::from("https://icp-api.io"),
        OsString::from("--lock-stale-after"),
        OsString::from("5m"),
        OsString::from("--dry-run"),
    ])
    .expect("parse topology refresh");

    assert_eq!(options.format, OutputFormat::Json);
    assert_eq!(options.source_endpoint, "https://icp-api.io");
    assert_eq!(options.lock_stale_after_seconds, 300);
    assert!(options.dry_run);
}

#[test]
fn data_center_help_is_advertised_under_nns() {
    let nns = usage();
    let data_center = data_center_usage();
    let list = data_center_list_usage();
    let info = data_center_info_usage();
    let refresh = data_center_refresh_usage();

    assert!(nns.contains("data-center"));
    assert!(data_center.contains("List cached mainnet NNS data centers"));
    assert!(data_center.contains("Show one cached mainnet NNS data center"));
    assert!(data_center.contains("Force-refresh and cache NNS data-center metadata"));
    assert!(list.contains("canic nns data-center list"));
    assert!(list.contains("--verbose"));
    assert!(list.contains("--format json"));
    assert!(info.contains("canic nns data-center info"));
    assert!(info.contains("data-center|data-center-prefix"));
    assert!(refresh.contains("canic nns data-center refresh"));
    assert!(refresh.contains("--dry-run"));
}

#[test]
fn node_help_is_advertised_under_nns() {
    let nns = usage();
    let node = node_usage();
    let list = node_list_usage();
    let info = node_info_usage();
    let refresh = node_refresh_usage();

    assert!(nns.contains("node"));
    assert!(node.contains("List cached mainnet NNS nodes"));
    assert!(node.contains("Show one cached mainnet NNS node"));
    assert!(node.contains("Force-refresh and cache NNS node metadata"));
    assert!(list.contains("canic nns node list"));
    assert!(list.contains("--verbose"));
    assert!(list.contains("--format json"));
    assert!(list.contains("--data-center"));
    assert!(list.contains("--node-provider"));
    assert!(list.contains("--node-operator"));
    assert!(list.contains("--subnet"));
    assert!(list.contains("--kind"));
    assert!(info.contains("canic nns node info"));
    assert!(info.contains("node|node-prefix"));
    assert!(refresh.contains("canic nns node refresh"));
    assert!(refresh.contains("--dry-run"));
}

#[test]
fn node_provider_help_is_advertised_under_nns() {
    let nns = usage();
    let node_provider = node_provider_usage();
    let list = node_provider_list_usage();
    let info = node_provider_info_usage();
    let refresh = node_provider_refresh_usage();

    assert!(nns.contains("node-provider"));
    assert!(node_provider.contains("List cached mainnet NNS node providers"));
    assert!(node_provider.contains("Show one cached mainnet NNS node provider"));
    assert!(node_provider.contains("Force-refresh and cache NNS node-provider metadata"));
    assert!(list.contains("canic nns node-provider list"));
    assert!(list.contains("--verbose"));
    assert!(list.contains("--format json"));
    assert!(info.contains("canic nns node-provider info"));
    assert!(info.contains("node-provider|node-provider-prefix"));
    assert!(refresh.contains("canic nns node-provider refresh"));
    assert!(refresh.contains("--dry-run"));
}

#[test]
fn node_operator_help_is_advertised_under_nns() {
    let nns = usage();
    let node_operator = node_operator_usage();
    let list = node_operator_list_usage();
    let info = node_operator_info_usage();
    let refresh = node_operator_refresh_usage();

    assert!(nns.contains("node-operator"));
    assert!(node_operator.contains("List cached mainnet NNS node operators"));
    assert!(node_operator.contains("Show one cached mainnet NNS node operator"));
    assert!(node_operator.contains("Force-refresh and cache NNS node-operator metadata"));
    assert!(list.contains("canic nns node-operator list"));
    assert!(list.contains("--verbose"));
    assert!(list.contains("--format json"));
    assert!(info.contains("canic nns node-operator info"));
    assert!(info.contains("node-operator|node-operator-prefix"));
    assert!(refresh.contains("canic nns node-operator refresh"));
    assert!(refresh.contains("--dry-run"));
}

#[test]
fn registry_help_is_advertised_under_nns() {
    let nns = usage();
    let registry = registry_usage();
    let version = registry_version_usage();

    assert!(nns.contains("registry"));
    assert!(registry.contains("Show the latest mainnet NNS registry version"));
    assert!(version.contains("canic nns registry version"));
    assert!(version.contains("--format json"));
}

#[test]
fn topology_help_is_advertised_under_nns() {
    let nns = usage();
    let topology = topology_usage();
    let summary = topology_summary_usage();
    let refresh = topology_refresh_usage();

    assert!(nns.contains("topology"));
    assert!(topology.contains("Summarize cached mainnet NNS topology reports"));
    assert!(topology.contains("Refresh cached mainnet NNS topology component reports"));
    assert!(summary.contains("canic nns topology summary"));
    assert!(summary.contains("--format json"));
    assert!(summary.contains("--source-endpoint"));
    assert!(refresh.contains("canic nns topology refresh"));
    assert!(refresh.contains("--format json"));
    assert!(refresh.contains("--source-endpoint"));
    assert!(refresh.contains("--lock-stale-after"));
    assert!(refresh.contains("--dry-run"));
}

#[test]
fn data_center_local_is_rejected_with_pinned_message() {
    let err = run([
        OsString::from("data-center"),
        OsString::from("list"),
        OsString::from("--__canic-network"),
        OsString::from("local"),
    ])
    .expect_err("local rejected");

    let message = err.to_string();
    assert!(message.contains("supports only the mainnet `ic` network"));
    assert!(message.contains("canic --network ic nns data-center list"));
}

#[test]
fn node_local_is_rejected_with_pinned_message() {
    let err = run([
        OsString::from("node"),
        OsString::from("list"),
        OsString::from("--__canic-network"),
        OsString::from("local"),
    ])
    .expect_err("local rejected");

    let message = err.to_string();
    assert!(message.contains("supports only the mainnet `ic` network"));
    assert!(message.contains("canic --network ic nns node list"));
}

#[test]
fn node_provider_local_is_rejected_with_pinned_message() {
    let err = run([
        OsString::from("node-provider"),
        OsString::from("list"),
        OsString::from("--__canic-network"),
        OsString::from("local"),
    ])
    .expect_err("local rejected");

    let message = err.to_string();
    assert!(message.contains("supports only the mainnet `ic` network in 0.60"));
    assert!(message.contains("canic --network ic nns node-provider list"));
}

#[test]
fn node_operator_local_is_rejected_with_pinned_message() {
    let err = run([
        OsString::from("node-operator"),
        OsString::from("list"),
        OsString::from("--__canic-network"),
        OsString::from("local"),
    ])
    .expect_err("local rejected");

    let message = err.to_string();
    assert!(message.contains("supports only the mainnet `ic` network"));
    assert!(message.contains("canic --network ic nns node-operator list"));
}

#[test]
fn registry_local_is_rejected_with_pinned_message() {
    let err = run([
        OsString::from("registry"),
        OsString::from("version"),
        OsString::from("--__canic-network"),
        OsString::from("local"),
    ])
    .expect_err("local rejected");

    let message = err.to_string();
    assert!(message.contains("supports only the mainnet `ic` network"));
    assert!(message.contains("canic --network ic nns registry version"));
}

#[test]
fn topology_local_is_rejected_with_pinned_message() {
    let err = run([
        OsString::from("topology"),
        OsString::from("summary"),
        OsString::from("--__canic-network"),
        OsString::from("local"),
    ])
    .expect_err("local rejected");

    let message = err.to_string();
    assert!(message.contains("supports only the mainnet `ic` network"));
    assert!(message.contains("canic --network ic nns topology summary"));
}

#[test]
fn topology_refresh_local_is_rejected_with_pinned_message() {
    let err = run([
        OsString::from("topology"),
        OsString::from("refresh"),
        OsString::from("--__canic-network"),
        OsString::from("local"),
    ])
    .expect_err("local rejected");

    let message = err.to_string();
    assert!(message.contains("supports only the mainnet `ic` network"));
    assert!(message.contains("canic --network ic nns topology refresh"));
}

#[test]
fn catalog_local_is_rejected_with_pinned_message() {
    let err = run([
        OsString::from("subnet"),
        OsString::from("list"),
        OsString::from("--__canic-network"),
        OsString::from("local"),
    ])
    .expect_err("local rejected");

    let message = err.to_string();
    assert!(message.contains("supports only the mainnet `ic` network in 0.60"));
    assert!(message.contains("canic --network ic nns subnet list"));
}

#[test]
fn refresh_is_advertised_as_subnet_command() {
    let text = subnet_usage();

    assert!(text.contains("refresh"));
    assert!(refresh_usage().contains("canic nns subnet refresh"));
}

#[test]
fn nns_namespace_help_mentions_subnet() {
    let text = usage();

    assert!(text.contains("Inspect NNS metadata"));
    assert!(text.contains("subnet"));
    assert!(!text.contains("Inspect cached IC network subnet metadata"));
}

#[test]
fn role_resolution_reports_ambiguity() {
    let resolution = InstalledDeploymentResolution {
        source: InstalledDeploymentSource::IcpCli,
        state: sample_state(),
        registry: InstalledDeploymentRegistry {
            root_canister_id: "aaaaa-aa".to_string(),
            entries: vec![
                registry_entry("ryjl3-tyaaa-aaaaa-aaaba-cai", "backend"),
                registry_entry("rrkah-fqaaa-aaaaa-aaaaq-cai", "backend"),
            ],
        },
        topology: ResolvedDeploymentTopology {
            root_canister_id: "aaaaa-aa".to_string(),
            children_by_parent: BTreeMap::default(),
            roles_by_canister: BTreeMap::default(),
        },
    };

    let err = resolve_canister_or_role(&resolution, "demo", "backend").expect_err("ambiguous role");

    assert!(err.contains("role backend is ambiguous"));
}

fn registry_entry(pid: &str, role: &str) -> RegistryEntry {
    RegistryEntry {
        pid: pid.to_string(),
        role: Some(role.to_string()),
        kind: Some("canister".to_string()),
        parent_pid: None,
        module_hash: None,
    }
}

fn sample_state() -> canic_host::install_root::InstallState {
    canic_host::install_root::InstallState {
        schema_version: 2,
        deployment_name: "demo".to_string(),
        fleet_template: "demo".to_string(),
        created_at_unix_secs: 1,
        updated_at_unix_secs: 1,
        network: MAINNET_NETWORK.to_string(),
        root_target: "root".to_string(),
        root_canister_id: "aaaaa-aa".to_string(),
        root_verification: canic_host::install_root::RootVerificationStatus::Verified,
        root_build_target: "root".to_string(),
        workspace_root: ".".to_string(),
        icp_root: ".".to_string(),
        config_path: "fleets/demo/canic.toml".to_string(),
        release_set_manifest_path: ".canic/ic/release-set.json".to_string(),
    }
}
