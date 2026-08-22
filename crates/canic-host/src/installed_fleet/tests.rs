use super::*;
use crate::test_support::temp_dir;
use canic_core::ids::{CanonicalNetworkId, FleetId};
use std::fs;

// Ensure installed-Fleet lookup retains the Fleet-catalog path and JSON source.
#[test]
fn retains_fleet_catalog_decode_error() {
    let root = temp_dir("canic-installed-fleet-decode");
    fs::create_dir_all(&root).expect("create project root");
    let path = root
        .join(".canic")
        .join("networks")
        .join(CanonicalNetworkId::ic_mainnet().to_string())
        .join("fleets/catalog.json");
    fs::create_dir_all(path.parent().expect("Fleet catalog parent"))
        .expect("create Fleet catalog parent");
    fs::write(&path, b"{").expect("write malformed Fleet catalog");

    let error = read_installed_fleet_from_root("ic", "demo", &root)
        .expect_err("malformed Fleet catalog must fail");

    match error {
        InstalledFleetError::FleetCatalog(FleetCatalogError::Decode {
            path: error_path,
            source,
        }) => {
            assert_eq!(error_path, path);
            assert!(source.is_eof());
        }
        other => panic!("unexpected installed-Fleet error: {other:?}"),
    }

    fs::remove_dir_all(root).expect("remove test directory");
}

#[test]
fn coordinator_catalog_rejects_the_removed_single_root_topology_resolver() {
    let root = temp_dir("canic-installed-fleet-coordinator");
    fs::create_dir_all(&root).expect("create project root");
    fs::write(
        root.join("icp.yaml"),
        "environments:\n  - name: staging\n    network: ic\n",
    )
    .expect("write ICP config");
    let network = CanonicalNetworkId::ic_mainnet();
    let path = root
        .join(".canic")
        .join("networks")
        .join(network.to_string())
        .join("fleets/catalog.json");
    fs::create_dir_all(path.parent().expect("Fleet catalog parent"))
        .expect("create Fleet catalog parent");
    let catalog = serde_json::json!({
        "schema_version": 1,
        "canonical_network_id": network,
        "entries": [{
            "canonical_network_id": network,
            "fleet_id": FleetId::from_generated_bytes([1; 32]),
            "fleet_name": "demo",
            "app": "shop",
            "environment": "staging",
            "deployed_at_unix_secs": 54,
            "release_build_id": "01".repeat(32),
            "coordinator_principal": "rrkah-fqaaa-aaaaa-aaaaq-cai"
        }]
    });
    fs::write(
        &path,
        serde_json::to_vec_pretty(&catalog).expect("catalog JSON"),
    )
    .expect("write Fleet catalog");
    let request = InstalledFleetRequest {
        fleet: "demo".to_string(),
        environment: "staging".to_string(),
    };

    let error = resolve_installed_fleet_from_root(&request, &root)
        .expect_err("Coordinator catalog must not be treated as one root");

    assert!(matches!(
        error,
        InstalledFleetError::CoordinatorAnchoredTopologyUnavailable {
            fleet,
            coordinator,
        } if fleet == "demo" && coordinator == "rrkah-fqaaa-aaaaa-aaaaq-cai"
    ));
    fs::remove_dir_all(root).expect("remove test directory");
}

#[test]
fn explicit_root_selection_rejects_foreign_and_removed_principals() {
    let active = Principal::from_slice(&[10; 29]);
    let draining = Principal::from_slice(&[11; 29]);
    let removed = Principal::from_slice(&[12; 29]);
    let roots = [
        (active, FleetSubnetRootStatus::Active),
        (draining, FleetSubnetRootStatus::Draining),
        (removed, FleetSubnetRootStatus::Removed),
    ];

    assert_eq!(select_current_root(roots, active), Some(active));
    assert_eq!(select_current_root(roots, draining), Some(draining));
    assert_eq!(select_current_root(roots, removed), None);
    assert_eq!(
        select_current_root(roots, Principal::from_slice(&[13; 29])),
        None
    );
}
