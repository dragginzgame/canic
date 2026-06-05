use super::*;
use crate::test_support::temp_dir;
use canic_subnet_catalog::{
    CATALOG_SCHEMA_VERSION, ClassificationSource, GeographicScope, MAINNET_REGISTRY_CANISTER_ID,
    SubnetSpecialization,
};

const SUBNET_A: &str = "rwlgt-iiaaa-aaaaa-aaaaa-cai";
const SUBNET_B: &str = "aaaaa-aa";
const CANISTER_A: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";

#[test]
fn catalog_path_lives_outside_deployment_state() {
    let root = PathBuf::from("/tmp/canic-project");

    let path = subnet_catalog_path(&root, MAINNET_NETWORK);

    assert_eq!(
        path,
        PathBuf::from("/tmp/canic-project/.canic/subnet-catalog/ic/catalog.json")
    );
    assert!(!path.display().to_string().contains("/deployments/"));
    assert!(!path.display().to_string().contains("/fleets/"));
}

#[test]
fn load_cached_catalog_rejects_non_mainnet_network() {
    let root = temp_dir("canic-subnet-host-network");
    let request = SubnetCatalogCacheRequest {
        icp_root: root.clone(),
        network: "local".to_string(),
    };

    let err = load_cached_subnet_catalog(&request).expect_err("local rejected");

    let _ = fs::remove_dir_all(root);
    std::assert_matches!(err, SubnetCatalogHostError::UnsupportedNetwork { .. });
}

#[test]
fn missing_catalog_error_explains_cached_only_slice() {
    let root = temp_dir("canic-subnet-host-missing");
    let request = SubnetCatalogCacheRequest {
        icp_root: root.clone(),
        network: MAINNET_NETWORK.to_string(),
    };

    let err = load_cached_subnet_catalog(&request).expect_err("cache missing");
    let message = err.to_string();

    let _ = fs::remove_dir_all(root);
    assert!(message.contains("Run `canic nns subnet refresh`"));
    assert!(message.contains("public Internet Computer mainnet catalog"));
    assert!(message.contains("canic nns subnet refresh"));
}

#[test]
fn list_report_loads_cached_catalog_and_caps_ranges() {
    let root = temp_dir("canic-subnet-host-list");
    write_catalog(&root, fixture_catalog());
    let request = list_request(&root);

    let report = build_subnet_catalog_list_report(&request).expect("list report");
    let text = subnet_catalog_list_report_text(&report);

    let _ = fs::remove_dir_all(root);
    assert_eq!(report.subnets.len(), 2);
    assert_eq!(report.subnets[0].range_count, 2);
    assert_eq!(report.subnets[0].ranges_shown, 1);
    assert!(text.contains("SUBNET"));
    assert!(text.contains("SPEC"));
    assert!(!text.contains("SPECIALIZATION"));
    for subnet in &report.subnets {
        assert!(text.contains(&compact_principal(&subnet.subnet_principal)));
        assert!(!text.contains(&subnet.subnet_principal));
    }
    assert!(!text.contains("FETCHED_AT"));
    assert!(text.contains("showing 1 of 2 ranges"));
}

#[test]
fn list_report_verbose_text_keeps_full_metadata() {
    let root = temp_dir("canic-subnet-host-list-verbose");
    write_catalog(&root, fixture_catalog());
    let request = list_request(&root);

    let report = build_subnet_catalog_list_report(&request).expect("list report");
    let text = subnet_catalog_list_report_verbose_text(&report);

    let _ = fs::remove_dir_all(root);
    assert!(text.contains("catalog_path:"));
    assert!(text.contains("SPECIALIZATION"));
    assert!(text.contains("FETCHED_AT"));
    assert!(text.contains(SUBNET_A));
}

#[test]
fn info_report_resolves_canister_and_marks_application_chargeable() {
    let root = temp_dir("canic-subnet-host-info");
    write_catalog(&root, fixture_catalog());
    let request = info_request(&root, CANISTER_A);

    let report = build_subnet_catalog_info_report(&request).expect("info report");

    let _ = fs::remove_dir_all(root);
    assert_eq!(report.resolved_as, "canister");
    assert_eq!(report.subnet_principal, SUBNET_A);
    assert!(report.charges_apply_to_subject);
    assert_eq!(
        report.charge_applicability_reason,
        "charged_user_canister_subnet"
    );
    assert_eq!(report.cycles_per_billion_instructions, Some(2_615_384_616));
}

#[test]
fn info_report_resolves_unique_subnet_prefix() {
    let root = temp_dir("canic-subnet-host-info-subnet-prefix");
    write_catalog(&root, fixture_catalog());
    let request = info_request(&root, "rwl");

    let report = build_subnet_catalog_info_report(&request).expect("info report");

    let _ = fs::remove_dir_all(root);
    assert_eq!(report.input_principal, "rwl");
    assert_eq!(report.resolved_as, "subnet");
    assert_eq!(report.resolved_from, "subnet_principal_prefix");
    assert_eq!(report.subnet_principal, SUBNET_A);
    assert_eq!(report.matched_canister_principal, None);
}

#[test]
fn info_report_rejects_canister_prefix() {
    let root = temp_dir("canic-subnet-host-info-canister-prefix");
    write_catalog(&root, fixture_catalog());
    let request = info_request(&root, "ryj");

    let err = build_subnet_catalog_info_report(&request).expect_err("canister prefix rejected");

    let _ = fs::remove_dir_all(root);
    std::assert_matches!(
        err,
        SubnetCatalogHostError::Catalog(CatalogError::PrincipalPrefixNotFound { prefix })
            if prefix == "ryj"
    );
}

#[test]
fn system_subnet_has_no_catalog_rate() {
    let root = temp_dir("canic-subnet-host-system");
    let mut catalog = fixture_catalog();
    catalog.subnets[0].subnet_kind = SubnetKind::System;
    catalog.subnets[0].charges_apply_by_default = false;
    write_catalog(&root, catalog);
    let request = info_request(&root, CANISTER_A);

    let report = build_subnet_catalog_info_report(&request).expect("info report");

    let _ = fs::remove_dir_all(root);
    assert!(!report.charges_apply_to_subject);
    assert_eq!(
        report.charge_applicability_reason,
        "system_subnet_unknown_subject"
    );
    assert_eq!(report.cycles_per_billion_instructions, None);
}

#[test]
fn stale_status_is_deterministic() {
    let catalog = fixture_catalog();
    let fresh = catalog_stale_status(&catalog, 1_780_531_300, 200);
    let stale = catalog_stale_status(&catalog, 1_780_531_501, 200);

    assert!(!fresh.catalog_stale);
    assert!(stale.catalog_stale);
}

#[test]
fn stale_duration_accepts_units() {
    assert_eq!(parse_stale_after_duration("7d").expect("days"), 604_800);
    assert_eq!(parse_stale_after_duration("2h").expect("hours"), 7_200);
    assert_eq!(parse_stale_after_duration("30m").expect("minutes"), 1_800);
    assert_eq!(parse_stale_after_duration("90s").expect("seconds"), 90);
    assert_eq!(parse_stale_after_duration("42").expect("bare"), 42);
    std::assert_matches!(
        parse_stale_after_duration("0d"),
        Err(SubnetCatalogHostError::InvalidStaleDuration { .. })
    );
}

#[test]
fn refresh_writes_catalog_atomically_and_removes_lock() {
    let root = temp_dir("canic-subnet-host-refresh");
    let mut catalog = fixture_catalog();
    catalog.registry_version = 987_654;
    catalog.fetched_at = "1970-01-01T00:00:00Z".to_string();
    catalog.source_endpoint = DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT.to_string();
    let source = FixtureRefreshSource::ok(catalog);
    let request = refresh_request(&root);

    let report = refresh_subnet_catalog_with_source(&request, &source).expect("refresh catalog");
    let cached = load_cached_subnet_catalog(&cache_request(&root)).expect("cached catalog");
    let lock_path = PathBuf::from(&report.refresh_lock_path);

    let _ = fs::remove_dir_all(root);
    assert!(report.wrote_catalog);
    assert!(!report.replaced_existing_catalog);
    assert_eq!(report.registry_version, 987_654);
    assert_eq!(cached.catalog.registry_version, 987_654);
    assert!(!lock_path.exists());
}

#[test]
fn refresh_dry_run_writes_output_without_replacing_cache() {
    let root = temp_dir("canic-subnet-host-refresh-dry-run");
    let mut catalog = fixture_catalog();
    catalog.fetched_at = "1970-01-01T00:00:00Z".to_string();
    catalog.source_endpoint = DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT.to_string();
    let output_path = root.join("catalog-export.json");
    let source = FixtureRefreshSource::ok(catalog);
    let mut request = refresh_request(&root);
    request.dry_run = true;
    request.output_path = Some(output_path.clone());

    let report = refresh_subnet_catalog_with_source(&request, &source).expect("dry-run");

    assert!(!report.wrote_catalog);
    assert!(!subnet_catalog_path(&request.cache.icp_root, MAINNET_NETWORK).exists());
    assert!(output_path.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn refresh_failure_preserves_existing_catalog_and_removes_lock() {
    let root = temp_dir("canic-subnet-host-refresh-failure");
    write_catalog(&root, fixture_catalog());
    let source = FixtureRefreshSource::err();
    let request = refresh_request(&root);

    let err = refresh_subnet_catalog_with_source(&request, &source).expect_err("refresh fails");
    let cached = load_cached_subnet_catalog(&cache_request(&root)).expect("cached catalog");
    let lock_path = subnet_catalog_refresh_lock_path(&root, MAINNET_NETWORK);

    std::assert_matches!(err, SubnetCatalogHostError::InvalidStaleDuration { .. });
    assert_eq!(cached.catalog.registry_version, 123_456);
    assert!(!lock_path.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn refresh_existing_fresh_lock_fails_fast() {
    let root = temp_dir("canic-subnet-host-refresh-locked");
    let request = refresh_request(&root);
    let lock_path = subnet_catalog_refresh_lock_path(&root, MAINNET_NETWORK);
    write_refresh_lock_for_test(&lock_path, &request, request.now_unix_secs * 1_000);

    let err = refresh_subnet_catalog_with_source(&request, &FixtureRefreshSource::err())
        .expect_err("lock held");

    let _ = fs::remove_dir_all(root);
    std::assert_matches!(err, SubnetCatalogHostError::RefreshAlreadyInProgress { .. });
}

#[test]
fn refresh_removes_stale_lock_and_retries_once() {
    let root = temp_dir("canic-subnet-host-refresh-stale-lock");
    let mut catalog = fixture_catalog();
    catalog.fetched_at = "1970-01-01T00:00:00Z".to_string();
    catalog.source_endpoint = DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT.to_string();
    let source = FixtureRefreshSource::ok(catalog);
    let request = refresh_request(&root);
    let lock_path = subnet_catalog_refresh_lock_path(&root, MAINNET_NETWORK);
    let stale_started_at = (request.now_unix_secs - request.lock_stale_after_seconds - 1) * 1_000;
    write_refresh_lock_for_test(&lock_path, &request, stale_started_at);

    let report = refresh_subnet_catalog_with_source(&request, &source).expect("stale lock removed");

    assert!(report.wrote_catalog);
    assert!(!lock_path.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn utc_timestamp_formatter_is_deterministic() {
    assert_eq!(format_utc_timestamp_secs(0), "1970-01-01T00:00:00Z");
    assert_eq!(
        format_utc_timestamp_secs(1_780_531_200),
        "2026-06-04T00:00:00Z"
    );
}

fn list_request(root: &Path) -> SubnetCatalogListRequest {
    SubnetCatalogListRequest {
        cache: cache_request(root),
        now_unix_secs: 1_780_531_300,
        stale_after_seconds: DEFAULT_STALE_AFTER_SECONDS,
        filters: SubnetCatalogFilters::default(),
        show_ranges: true,
        range_limit: 1,
        range_offset: 0,
    }
}

fn info_request(root: &Path, input: &str) -> SubnetCatalogInfoRequest {
    SubnetCatalogInfoRequest {
        cache: cache_request(root),
        input: input.to_string(),
        forced: None,
        resolved_target: None,
        now_unix_secs: 1_780_531_300,
        stale_after_seconds: DEFAULT_STALE_AFTER_SECONDS,
    }
}

fn cache_request(root: &Path) -> SubnetCatalogCacheRequest {
    SubnetCatalogCacheRequest {
        icp_root: root.to_path_buf(),
        network: MAINNET_NETWORK.to_string(),
    }
}

fn write_catalog(root: &Path, catalog: SubnetCatalog) {
    let path = subnet_catalog_path(root, MAINNET_NETWORK);
    fs::create_dir_all(path.parent().expect("catalog parent")).expect("create parent");
    fs::write(
        path,
        serde_json::to_vec_pretty(&catalog).expect("serialize catalog"),
    )
    .expect("write catalog");
}

fn refresh_request(root: &Path) -> SubnetCatalogRefreshRequest {
    SubnetCatalogRefreshRequest {
        cache: cache_request(root),
        source_endpoint: DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT.to_string(),
        now_unix_secs: 1_780_531_200,
        lock_stale_after_seconds: DEFAULT_REFRESH_LOCK_STALE_SECONDS,
        dry_run: false,
        output_path: None,
    }
}

fn write_refresh_lock_for_test(
    lock_path: &Path,
    request: &SubnetCatalogRefreshRequest,
    started_at_unix_ms: u64,
) {
    fs::create_dir_all(lock_path.parent().expect("lock parent")).expect("create parent");
    let lock = serde_json::json!({
        "schema_version": 1,
        "network": request.cache.network.clone(),
        "pid": 12345,
        "started_at_unix_ms": started_at_unix_ms,
        "target_path": subnet_catalog_path(&request.cache.icp_root, &request.cache.network)
            .display()
            .to_string(),
    });
    fs::write(
        lock_path,
        serde_json::to_vec_pretty(&lock).expect("serialize lock"),
    )
    .expect("write lock");
}

///
/// FixtureRefreshSource
///
struct FixtureRefreshSource {
    catalog: Option<SubnetCatalog>,
    fail: bool,
}

impl FixtureRefreshSource {
    fn ok(catalog: SubnetCatalog) -> Self {
        Self {
            catalog: Some(catalog),
            fail: false,
        }
    }

    fn err() -> Self {
        Self {
            catalog: None,
            fail: true,
        }
    }
}

impl SubnetCatalogRefreshSource for FixtureRefreshSource {
    fn fetch_catalog(
        &self,
        _request: &MainnetRegistryFetchRequest,
    ) -> Result<SubnetCatalog, SubnetCatalogHostError> {
        if self.fail {
            return Err(SubnetCatalogHostError::InvalidStaleDuration {
                value: "fixture".to_string(),
            });
        }
        Ok(self.catalog.clone().expect("fixture catalog"))
    }
}

fn fixture_catalog() -> SubnetCatalog {
    SubnetCatalog {
        catalog_schema_version: CATALOG_SCHEMA_VERSION,
        network: MAINNET_NETWORK.to_string(),
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 123_456,
        fetched_at: "2026-06-04T00:00:00Z".to_string(),
        fetched_by: "fixture".to_string(),
        source_endpoint: "https://icp-api.io".to_string(),
        resolver_backend: "local-nns-subnet-catalog".to_string(),
        subnets: vec![
            SubnetInfo {
                subnet_principal: SUBNET_A.to_string(),
                subnet_kind: SubnetKind::Application,
                subnet_kind_source: ClassificationSource::Registry,
                subnet_specialization: SubnetSpecialization::Fiduciary,
                subnet_specialization_source: ClassificationSource::Curated,
                geographic_scope: GeographicScope::Global,
                geographic_scope_source: ClassificationSource::Curated,
                subnet_label: "fiduciary".to_string(),
                subnet_label_source: ClassificationSource::Curated,
                node_count: Some(34),
                charges_apply_by_default: true,
            },
            SubnetInfo {
                subnet_principal: SUBNET_B.to_string(),
                subnet_kind: SubnetKind::System,
                subnet_kind_source: ClassificationSource::Registry,
                subnet_specialization: SubnetSpecialization::None,
                subnet_specialization_source: ClassificationSource::Curated,
                geographic_scope: GeographicScope::Global,
                geographic_scope_source: ClassificationSource::Curated,
                subnet_label: "system".to_string(),
                subnet_label_source: ClassificationSource::Curated,
                node_count: Some(13),
                charges_apply_by_default: false,
            },
        ],
        routing_ranges: vec![
            RoutingRange {
                start_canister_id: CANISTER_A.to_string(),
                end_canister_id: CANISTER_A.to_string(),
                subnet_principal: SUBNET_A.to_string(),
            },
            RoutingRange {
                start_canister_id: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
                end_canister_id: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
                subnet_principal: SUBNET_A.to_string(),
            },
            RoutingRange {
                start_canister_id: "r7inp-6aaaa-aaaaa-aaabq-cai".to_string(),
                end_canister_id: "r7inp-6aaaa-aaaaa-aaabq-cai".to_string(),
                subnet_principal: SUBNET_B.to_string(),
            },
        ],
    }
}
