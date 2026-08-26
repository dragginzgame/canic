use super::*;
use crate::test_support::temp_dir;
use canic_core::ids::CanonicalNetworkId;
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
fn singular_root_consumers_reject_zero_or_multiple_current_roots() {
    let topology = |roots: Vec<&str>| ResolvedFleetTopology {
        coordinator_canister_id: "coordinator".to_string(),
        fleet_subnet_root_canister_ids: roots.into_iter().map(str::to_string).collect(),
        children_by_parent: BTreeMap::new(),
        roles_by_canister: BTreeMap::new(),
    };

    assert_eq!(
        topology(vec!["root-a"])
            .unique_fleet_subnet_root("demo")
            .expect("one exact root"),
        "root-a"
    );
    for roots in [Vec::new(), vec!["root-a", "root-b"]] {
        assert!(matches!(
            topology(roots).unique_fleet_subnet_root("demo"),
            Err(InstalledFleetError::AmbiguousFleetSubnetRoot { fleet, .. })
                if fleet == "demo"
        ));
    }
}

#[test]
fn child_projection_retains_exact_parent_role_and_raw_module_hash() {
    let parent = Principal::from_slice(&[20; 29]);
    let child = Principal::from_slice(&[21; 29]);
    let entry = registry_entry_from_child(
        CanisterInfo {
            pid: child,
            role: CanisterRole::from("users"),
            parent_pid: Some(parent),
            module_hash: Some(vec![7; 32]),
            created_at: 81,
        },
        None,
    )
    .expect("project child");

    assert_eq!(entry.pid, child.to_text());
    assert_eq!(entry.role.as_deref(), Some("users"));
    assert_eq!(entry.parent_pid.as_deref(), Some(parent.to_text().as_str()));
    assert_eq!(entry.module_hash.as_deref(), Some("07".repeat(32).as_str()));

    let malformed = registry_entry_from_child(
        CanisterInfo {
            pid: child,
            role: CanisterRole::from("users"),
            parent_pid: Some(parent),
            module_hash: Some(vec![7; 31]),
            created_at: 81,
        },
        None,
    );
    assert!(matches!(
        malformed,
        Err(InstalledFleetError::ChildInventory(_))
    ));
}

#[test]
fn inventory_capacity_includes_every_admitted_component_descendant() {
    assert_eq!(
        maximum_admission_canisters(2, 3).expect("bounded admission"),
        8
    );
    assert_eq!(
        maximum_admission_canisters(u32::MAX, u32::MAX).expect("u32 product fits u64"),
        u64::from(u32::MAX) * (u64::from(u32::MAX) + 1)
    );
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
