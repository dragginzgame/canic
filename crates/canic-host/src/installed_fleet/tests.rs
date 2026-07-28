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
        .join(CanonicalNetworkId::public_ic().to_string())
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
    let network = CanonicalNetworkId::public_ic();
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
        icp: "icp".to_string(),
        detect_lost_local_root: false,
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
