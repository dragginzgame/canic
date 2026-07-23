use super::*;
use crate::test_support::temp_dir;
use std::fs;

#[test]
fn catalog_reads_network_scoped_fleet_rows_in_canonical_order() {
    let root = fixture("list");
    let network = CanonicalNetworkId::public_ic();
    write_catalog(
        &root,
        network,
        vec![
            entry(network, 1, "alpha", "shop", "staging", "aaaaa-aa"),
            entry(network, 2, "zeta", "shop", "production", "2vxsx-fae"),
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
    let network = CanonicalNetworkId::public_ic();
    write_catalog(
        &root,
        network,
        vec![entry(
            network,
            1,
            "shop-production",
            "shop",
            "production",
            "aaaaa-aa",
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
fn missing_catalog_is_empty_and_never_reads_legacy_deployment_state() {
    let root = fixture("no-legacy-fallback");
    let legacy = root.join(".canic/staging/deployments/shop.json");
    fs::create_dir_all(legacy.parent().expect("legacy parent")).expect("legacy directory");
    fs::write(&legacy, b"{\"deployment_name\":\"shop\"}").expect("legacy state");

    let report = build_fleet_catalog_report(&request(&root, "staging")).expect("empty catalog");

    assert!(report.entries.is_empty());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn catalog_rejects_wrong_network_unsorted_names_and_duplicate_ids() {
    let root = fixture("invalid");
    let network = CanonicalNetworkId::public_ic();
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
            entry(network, 1, "zeta", "shop", "staging", "aaaaa-aa"),
            entry(network, 2, "alpha", "shop", "staging", "2vxsx-fae"),
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
            entry(network, 1, "alpha", "shop", "staging", "aaaaa-aa"),
            entry(network, 1, "zeta", "shop", "staging", "2vxsx-fae"),
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
    let network = CanonicalNetworkId::public_ic();
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
            "aaaaa-aa",
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
            "aaaaa-aa",
        )],
    );
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
    let network = CanonicalNetworkId::public_ic();
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
    use rustix::fs::{CWD, Mode, mkfifoat};

    let root = fixture("special-file");
    let network = CanonicalNetworkId::public_ic();
    let path = fleet_catalog_path(&root, network);
    fs::create_dir_all(path.parent().expect("catalog parent")).expect("catalog directory");
    mkfifoat(CWD, &path, Mode::from_raw_mode(0o600)).expect("catalog FIFO");

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
    let network = CanonicalNetworkId::public_ic();
    write_catalog(
        &root,
        network,
        vec![entry(
            network,
            9,
            "shop-production",
            "shop",
            "production",
            "aaaaa-aa",
        )],
    );

    let report = inspect_fleet_catalog_report(&request(&root, "staging"), "shop-production")
        .expect("inspect Fleet");
    let text = fleet_catalog_report_text(&report);

    assert_eq!(report.entries.len(), 1);
    assert!(text.contains("Fleet catalog:"));
    assert!(text.contains("fleet_id:"));
    assert!(text.contains("app: shop"));
    assert!(!text.contains("deployment target"));
    assert!(matches!(
        inspect_fleet_catalog_report(&request(&root, "staging"), "unknown"),
        Err(FleetCatalogError::UnknownFleet { .. })
    ));
    fs::remove_dir_all(root).expect("remove fixture");
}

fn fixture(name: &str) -> PathBuf {
    let root = temp_dir(&format!("canic-fleet-catalog-{name}"));
    fs::create_dir_all(&root).expect("create project root");
    fs::write(
        root.join("icp.yaml"),
        "environments:\n  - name: staging\n    network: ic\n  - name: production\n    network: ic\n",
    )
    .expect("write ICP config");
    root
}

fn request(root: &Path, environment: &str) -> FleetCatalogRequest {
    FleetCatalogRequest {
        project_root: root.to_path_buf(),
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
    fs::create_dir_all(path.parent().expect("catalog parent")).expect("catalog directory");
    fs::write(
        path,
        serde_json::to_vec_pretty(&catalog).expect("catalog JSON"),
    )
    .expect("write catalog");
}

fn entry(
    network: CanonicalNetworkId,
    id_byte: u8,
    fleet_name: &str,
    app: &str,
    environment: &str,
    root_principal: &str,
) -> FleetCatalogEntryV1 {
    FleetCatalogEntryV1 {
        canonical_network_id: network,
        fleet_id: FleetId::from_generated_bytes([id_byte; 32]),
        fleet_name: fleet_name.parse().expect("Fleet name"),
        app: AppId::from(app),
        environment: environment.to_string(),
        deployed_at_unix_secs: 54,
        root_principal: root_principal.to_string(),
        root_verification: FleetCatalogRootVerificationV1::Verified,
    }
}
