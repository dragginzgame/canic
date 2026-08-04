use super::*;
#[cfg(unix)]
use crate::test_support::create_fifo;
use crate::test_support::temp_dir;
use std::fs;

#[test]
fn catalog_reads_network_scoped_fleet_rows_in_canonical_order() {
    let root = fixture("list");
    let network = CanonicalNetworkId::ic_mainnet();
    write_catalog(
        &root,
        network,
        vec![
            entry(
                network,
                1,
                "alpha",
                "shop",
                "staging",
                "rrkah-fqaaa-aaaaa-aaaaq-cai",
            ),
            entry(
                network,
                2,
                "zeta",
                "shop",
                "production",
                "ryjl3-tyaaa-aaaaa-aaaba-cai",
            ),
        ],
    );

    let report = build_fleet_catalog_report(&request(&root, "staging")).expect("Fleet catalog");

    assert_eq!(report.canonical_network_id, network);
    assert_eq!(report.environment, "staging");
    assert_eq!(
        report
            .entries
            .iter()
            .map(|entry| entry.fleet_name.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha", "zeta"]
    );
    assert_eq!(report.entries[0].app.as_str(), "shop");
    assert_eq!(report.entries[0].environment, "staging");
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn environment_aliases_read_the_same_canonical_network_catalog() {
    let root = fixture("aliases");
    let network = CanonicalNetworkId::ic_mainnet();
    write_catalog(
        &root,
        network,
        vec![entry(
            network,
            1,
            "shop-production",
            "shop",
            "production",
            "rrkah-fqaaa-aaaaa-aaaaq-cai",
        )],
    );

    let staging = build_fleet_catalog_report(&request(&root, "staging")).expect("staging catalog");
    let production =
        build_fleet_catalog_report(&request(&root, "production")).expect("production catalog");

    assert_eq!(
        staging.canonical_network_id,
        production.canonical_network_id
    );
    assert_eq!(staging.entries, production.entries);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn missing_catalog_has_no_fleet_entries() {
    let root = fixture("missing");

    let report = build_fleet_catalog_report(&request(&root, "staging")).expect("empty catalog");

    assert!(report.entries.is_empty());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn catalog_rejects_wrong_network_unsorted_names_and_duplicate_ids() {
    let root = fixture("invalid");
    let network = CanonicalNetworkId::ic_mainnet();
    let path = fleet_catalog_path(&root, network);

    write_catalog_record(
        &path,
        FleetCatalogRecord {
            schema_version: FLEET_CATALOG_SCHEMA_VERSION,
            canonical_network_id: FleetId::from_generated_bytes([8; 32])
                .to_string()
                .parse()
                .expect("network-shaped text"),
            entries: Vec::new(),
        },
    );
    assert!(matches!(
        build_fleet_catalog_report(&request(&root, "staging")),
        Err(FleetCatalogError::Invalid { .. })
    ));

    write_catalog(
        &root,
        network,
        vec![
            entry(
                network,
                1,
                "zeta",
                "shop",
                "staging",
                "rrkah-fqaaa-aaaaa-aaaaq-cai",
            ),
            entry(
                network,
                2,
                "alpha",
                "shop",
                "staging",
                "ryjl3-tyaaa-aaaaa-aaaba-cai",
            ),
        ],
    );
    assert!(matches!(
        build_fleet_catalog_report(&request(&root, "staging")),
        Err(FleetCatalogError::Invalid { .. })
    ));

    write_catalog(
        &root,
        network,
        vec![
            entry(
                network,
                1,
                "alpha",
                "shop",
                "staging",
                "rrkah-fqaaa-aaaaa-aaaaq-cai",
            ),
            entry(
                network,
                1,
                "zeta",
                "shop",
                "staging",
                "ryjl3-tyaaa-aaaaa-aaaba-cai",
            ),
        ],
    );
    assert!(matches!(
        build_fleet_catalog_report(&request(&root, "staging")),
        Err(FleetCatalogError::Invalid { .. })
    ));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn catalog_rejects_malformed_unknown_field_and_invalid_identity_rows() {
    let root = fixture("malformed");
    let network = CanonicalNetworkId::ic_mainnet();
    let path = fleet_catalog_path(&root, network);
    fs::create_dir_all(path.parent().expect("catalog parent")).expect("catalog directory");
    fs::write(&path, b"{not-json").expect("malformed catalog");
    assert!(matches!(
        build_fleet_catalog_report(&request(&root, "staging")),
        Err(FleetCatalogError::Decode { .. })
    ));

    let mut value = serde_json::to_value(FleetCatalogRecord {
        schema_version: FLEET_CATALOG_SCHEMA_VERSION,
        canonical_network_id: network,
        entries: vec![entry(
            network,
            1,
            "shop-production",
            "bad/app",
            "staging",
            "rrkah-fqaaa-aaaaa-aaaaq-cai",
        )],
    })
    .expect("catalog value");
    value
        .as_object_mut()
        .expect("catalog object")
        .insert("legacy".to_string(), serde_json::Value::Bool(true));
    fs::write(&path, serde_json::to_vec(&value).expect("catalog JSON")).expect("unknown field");
    assert!(matches!(
        build_fleet_catalog_report(&request(&root, "staging")),
        Err(FleetCatalogError::Decode { .. })
    ));

    write_catalog(
        &root,
        network,
        vec![entry(
            network,
            1,
            "shop-production",
            "bad/app",
            "staging",
            "rrkah-fqaaa-aaaaa-aaaaq-cai",
        )],
    );
    assert!(matches!(
        build_fleet_catalog_report(&request(&root, "staging")),
        Err(FleetCatalogError::Invalid { .. })
    ));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn catalog_rejects_non_canister_coordinator_and_zero_deployment_time() {
    let root = fixture("invalid-coordinator");
    let network = CanonicalNetworkId::ic_mainnet();

    write_catalog(
        &root,
        network,
        vec![entry(
            network,
            1,
            "shop-production",
            "shop",
            "staging",
            "2vxsx-fae",
        )],
    );
    assert!(matches!(
        build_fleet_catalog_report(&request(&root, "staging")),
        Err(FleetCatalogError::Invalid { .. })
    ));

    let mut invalid_time = entry(
        network,
        1,
        "shop-production",
        "shop",
        "staging",
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
    );
    invalid_time.deployed_at_unix_secs = 0;
    write_catalog(&root, network, vec![invalid_time]);
    assert!(matches!(
        build_fleet_catalog_report(&request(&root, "staging")),
        Err(FleetCatalogError::Invalid { .. })
    ));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(unix)]
#[test]
fn catalog_rejects_symlinked_authority() {
    use std::os::unix::fs::symlink;

    let root = fixture("symlink");
    let network = CanonicalNetworkId::ic_mainnet();
    let path = fleet_catalog_path(&root, network);
    fs::create_dir_all(path.parent().expect("catalog parent")).expect("catalog directory");
    let target = root.join("catalog-target.json");
    write_catalog_record(
        &target,
        FleetCatalogRecord {
            schema_version: FLEET_CATALOG_SCHEMA_VERSION,
            canonical_network_id: network,
            entries: Vec::new(),
        },
    );
    symlink(&target, &path).expect("catalog symlink");

    assert!(matches!(
        build_fleet_catalog_report(&request(&root, "staging")),
        Err(FleetCatalogError::NotRegular { .. })
    ));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(unix)]
#[test]
fn catalog_rejects_special_file_authority() {
    let root = fixture("special-file");
    let network = CanonicalNetworkId::ic_mainnet();
    let path = fleet_catalog_path(&root, network);
    fs::create_dir_all(path.parent().expect("catalog parent")).expect("catalog directory");
    create_fifo(&path);

    assert!(matches!(
        build_fleet_catalog_report(&request(&root, "staging")),
        Err(FleetCatalogError::NotRegular { .. })
    ));
    fs::remove_file(path).expect("remove FIFO");
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn catalog_inspect_and_text_use_fleet_identity_terms() {
    let root = fixture("inspect");
    let network = CanonicalNetworkId::ic_mainnet();
    write_catalog(
        &root,
        network,
        vec![entry(
            network,
            9,
            "shop-production",
            "shop",
            "production",
            "rrkah-fqaaa-aaaaa-aaaaq-cai",
        )],
    );

    let report = inspect_fleet_catalog_report(&request(&root, "staging"), "shop-production")
        .expect("inspect Fleet");
    let text = fleet_catalog_report_text(&report);

    assert_eq!(report.entries.len(), 1);
    assert!(text.contains("Fleet catalog:"));
    assert!(text.contains("fleet_id:"));
    assert!(text.contains("app: shop"));
    assert!(text.contains("coordinator_principal: rrkah-fqaaa-aaaaa-aaaaq-cai"));
    assert!(text.contains("workspace_root: ."));
    assert!(!text.contains("project_root"));
    let value = serde_json::to_value(&report).expect("serialize Fleet catalog report");
    assert_eq!(value["workspace_root"], ".");
    assert!(value.get("project_root").is_none());
    assert!(!text.contains("root_principal"));
    assert!(!text.contains("deployment target"));
    assert!(matches!(
        inspect_fleet_catalog_report(&request(&root, "staging"), "unknown"),
        Err(FleetCatalogError::UnknownFleet { .. })
    ));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn catalog_commit_is_canonical_exact_retry_and_conflict_closed() {
    let root = fixture("commit");
    let network = CanonicalNetworkId::ic_mainnet();
    let alpha = entry(
        network,
        1,
        "alpha",
        "shop",
        "staging",
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
    );
    let zeta = entry(
        network,
        2,
        "zeta",
        "shop",
        "staging",
        "ryjl3-tyaaa-aaaaa-aaaba-cai",
    );

    let committed_zeta =
        commit_fleet_catalog_entry(&root, zeta.clone()).expect("commit zeta Fleet");
    assert!(committed_zeta.advanced);
    let committed_alpha =
        commit_fleet_catalog_entry(&root, alpha.clone()).expect("commit alpha Fleet");
    assert!(committed_alpha.advanced);
    let repeated_alpha =
        commit_fleet_catalog_entry(&root, alpha.clone()).expect("repeat alpha Fleet");

    assert!(!repeated_alpha.advanced);
    assert_eq!(repeated_alpha.entry, alpha);
    assert_eq!(repeated_alpha.catalog_hash, committed_alpha.catalog_hash);
    let report = build_fleet_catalog_report(&request(&root, "staging")).expect("Fleet catalog");
    assert_eq!(report.entries, vec![alpha.clone(), zeta]);

    let mut conflicting = alpha;
    conflicting.coordinator_principal = "r7inp-6aaaa-aaaaa-aaabq-cai".to_string();
    assert!(matches!(
        commit_fleet_catalog_entry(&root, conflicting),
        Err(FleetCatalogError::Conflict {
            field: "fleet_name",
            ..
        })
    ));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn catalog_hard_rejects_the_removed_single_root_shape() {
    let root = fixture("removed-root-shape");
    let network = CanonicalNetworkId::ic_mainnet();
    let path = fleet_catalog_path(&root, network);
    let current = FleetCatalogRecord {
        schema_version: FLEET_CATALOG_SCHEMA_VERSION,
        canonical_network_id: network,
        entries: vec![entry(
            network,
            1,
            "shop-production",
            "shop",
            "staging",
            "rrkah-fqaaa-aaaaa-aaaaq-cai",
        )],
    };
    let mut value = serde_json::to_value(current).expect("catalog JSON value");
    let entry = &mut value["entries"][0];
    entry["root_principal"] = entry["coordinator_principal"].take();
    write_catalog_value(&path, &value);

    assert!(matches!(
        build_fleet_catalog_report(&request(&root, "staging")),
        Err(FleetCatalogError::Decode { .. })
    ));
    fs::remove_dir_all(root).expect("remove fixture");
}

fn fixture(name: &str) -> PathBuf {
    let root = temp_dir(&format!("canic-fleet-catalog-{name}"));
    fs::create_dir_all(&root).expect("create workspace root");
    fs::write(
        root.join("icp.yaml"),
        "environments:\n  - name: staging\n    network: ic\n  - name: production\n    network: ic\n",
    )
    .expect("write ICP config");
    root
}

fn request(root: &Path, environment: &str) -> FleetCatalogRequest {
    FleetCatalogRequest {
        workspace_root: root.to_path_buf(),
        environment: environment.to_string(),
        generated_at: "unix:54".to_string(),
    }
}

fn write_catalog(root: &Path, network: CanonicalNetworkId, entries: Vec<FleetCatalogEntryV1>) {
    write_catalog_record(
        &fleet_catalog_path(root, network),
        FleetCatalogRecord {
            schema_version: FLEET_CATALOG_SCHEMA_VERSION,
            canonical_network_id: network,
            entries,
        },
    );
}

fn write_catalog_record(path: &Path, catalog: FleetCatalogRecord) {
    write_catalog_value(
        path,
        &serde_json::to_value(catalog).expect("catalog JSON value"),
    );
}

fn write_catalog_value(path: &Path, value: &serde_json::Value) {
    fs::create_dir_all(path.parent().expect("catalog parent")).expect("catalog directory");
    fs::write(
        path,
        serde_json::to_vec_pretty(value).expect("catalog JSON"),
    )
    .expect("write catalog");
}

fn entry(
    network: CanonicalNetworkId,
    id_byte: u8,
    fleet_name: &str,
    app: &str,
    environment: &str,
    coordinator_principal: &str,
) -> FleetCatalogEntryV1 {
    FleetCatalogEntryV1 {
        canonical_network_id: network,
        fleet_id: FleetId::from_generated_bytes([id_byte; 32]),
        fleet_name: fleet_name.parse().expect("Fleet name"),
        app: AppId::from(app),
        environment: environment.to_string(),
        deployed_at_unix_secs: 54,
        coordinator_principal: coordinator_principal.to_string(),
    }
}
