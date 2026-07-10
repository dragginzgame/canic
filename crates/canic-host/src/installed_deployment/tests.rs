use super::*;
use crate::registry::RegistryEntry;

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
