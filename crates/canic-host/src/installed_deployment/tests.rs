use super::*;
use crate::{registry::RegistryEntry, test_support::temp_dir};
use std::fs;

// Ensure installed-deployment lookup retains the install-state path and JSON source.
#[test]
fn retains_install_state_decode_error() {
    let root = temp_dir("canic-installed-deployment-decode");
    let path = root
        .join(".canic")
        .join("local")
        .join("deployments")
        .join("demo.json");
    fs::create_dir_all(path.parent().expect("deployment state parent"))
        .expect("create deployment state parent");
    fs::write(&path, b"{").expect("write malformed deployment state");

    let error = read_installed_deployment_state_from_root("local", "demo", &root)
        .expect_err("malformed deployment state must fail");

    match error {
        InstalledDeploymentError::InstallState(InstallStateError::Decode {
            path: error_path,
            source,
        }) => {
            assert_eq!(error_path, path);
            assert!(source.is_eof());
        }
        other => panic!("unexpected installed-deployment error: {other:?}"),
    }

    fs::remove_dir_all(root).expect("remove test directory");
}

// Ensure ordinary local-registry rejection remains concrete through classification.
#[test]
fn retains_replica_query_error() {
    let request = InstalledDeploymentRequest {
        deployment: "demo".to_string(),
        network: "local".to_string(),
        icp: "icp".to_string(),
        detect_lost_local_root: false,
    };

    let error = local_registry_error(
        &request,
        "root-id",
        ReplicaQueryError::Rejected {
            code: 5,
            message: "query failed".to_string(),
        },
    );

    assert!(matches!(
        error,
        InstalledDeploymentError::ReplicaQuery(ReplicaQueryError::Rejected {
            code: 5,
            message,
        }) if message == "query failed"
    ));
}

// Ensure the resolved topology gives command code parent/role projections without reparsing.
#[test]
fn topology_indexes_registry_entries() {
    let registry = InstalledDeploymentRegistry {
        root_canister_id: "root-id".to_string(),
        entries: vec![
            RegistryEntry {
                pid: "child-b".to_string(),
                role: Some("worker".to_string()),
                kind: None,
                parent_pid: Some("root-id".to_string()),
                module_hash: None,
            },
            RegistryEntry {
                pid: "root-id".to_string(),
                role: Some("root".to_string()),
                kind: None,
                parent_pid: None,
                module_hash: None,
            },
            RegistryEntry {
                pid: "child-a".to_string(),
                role: Some("app".to_string()),
                kind: None,
                parent_pid: Some("root-id".to_string()),
                module_hash: None,
            },
        ],
    };

    let topology = ResolvedDeploymentTopology::from_registry(&registry);

    assert_eq!(
        topology
            .children_by_parent
            .get(&Some("root-id".to_string())),
        Some(&vec!["child-a".to_string(), "child-b".to_string()])
    );
    assert_eq!(topology.roles_by_canister["child-a"], "app");
    assert_eq!(topology.root_canister_id, "root-id");
}

// Ensure the structured destination-invalid reject is recognized for lost fleet guidance.
#[test]
fn detects_local_canister_not_found_error() {
    assert!(is_missing_destination_error(&ReplicaQueryError::Rejected {
        code: IC_REJECT_CODE_DESTINATION_INVALID,
        message: "canister is unavailable".to_string(),
    }));
    assert!(!is_missing_destination_error(
        &ReplicaQueryError::Rejected {
            code: 5,
            message: "some other failure".to_string(),
        }
    ));
}
