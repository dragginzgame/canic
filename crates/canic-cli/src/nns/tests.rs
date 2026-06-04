use super::*;
use canic_host::registry::RegistryEntry;
use std::collections::BTreeMap;

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

    assert!(text.contains("Inspect cached NNS registry data"));
    assert!(text.contains("subnet"));
    assert!(!text.contains("Inspect cached IC network subnet metadata"));
}

#[test]
fn role_resolution_reports_ambiguity() {
    let resolution = InstalledDeploymentResolution {
        source: canic_host::installed_deployment::InstalledDeploymentSource::IcpCli,
        state: sample_state(),
        registry: canic_host::installed_deployment::InstalledDeploymentRegistry {
            root_canister_id: "aaaaa-aa".to_string(),
            entries: vec![
                registry_entry("ryjl3-tyaaa-aaaaa-aaaba-cai", "backend"),
                registry_entry("rrkah-fqaaa-aaaaa-aaaaq-cai", "backend"),
            ],
        },
        topology: canic_host::installed_deployment::ResolvedDeploymentTopology {
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
