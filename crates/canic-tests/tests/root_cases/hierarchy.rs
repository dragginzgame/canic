use crate::root::{
    assertions::{
        assert_child_env, assert_children_match_registry, assert_directories_consistent,
        assert_registry_parents, assert_state_endpoints_are_root_only, registry_pid_for_role,
    },
    harness::{setup_root, setup_root_cached_topology},
    workers::{count_workers, create_worker},
};
use canic::ids::CanisterRole;
use canic_internal::canister;
use std::io::Write;

fn test_progress(test_name: &str, phase: &str) {
    eprintln!("[root_hierarchy] {test_name}: {phase}");
    let _ = std::io::stderr().flush();
}

///
/// TESTS
///

#[test]
fn root_builds_hierarchy_and_exposes_env() {
    test_progress("root_builds_hierarchy_and_exposes_env", "setup root");
    let setup = setup_root_cached_topology();

    test_progress(
        "root_builds_hierarchy_and_exposes_env",
        "assert registry parent relationships",
    );
    assert_registry_parents(
        &setup.pic,
        setup.root_id,
        &[
            (CanisterRole::ROOT, None),
            (canister::WASM_STORE, Some(setup.root_id)),
            (canister::APP, Some(setup.root_id)),
            (canister::USER_HUB, Some(setup.root_id)),
            (canister::SCALE_HUB, Some(setup.root_id)),
        ],
    );

    let wasm_store_pid =
        registry_pid_for_role(&setup.pic, setup.root_id, &CanisterRole::WASM_STORE);
    test_progress(
        "root_builds_hierarchy_and_exposes_env",
        "assert wasm_store child env",
    );
    assert_child_env(
        &setup.pic,
        wasm_store_pid,
        CanisterRole::WASM_STORE,
        setup.root_id,
    );

    test_progress(
        "root_builds_hierarchy_and_exposes_env",
        "assert each child env",
    );
    for (role, pid) in &setup.subnet_directory {
        if !role.is_root() {
            assert_child_env(&setup.pic, *pid, role.clone(), setup.root_id);
        }
    }
    test_progress("root_builds_hierarchy_and_exposes_env", "done");
}

#[test]
fn directories_are_consistent_across_canisters() {
    let setup = setup_root_cached_topology();

    assert_directories_consistent(&setup.pic, setup.root_id, &setup.subnet_directory);
}

#[test]
fn subnet_children_matches_registry_on_root() {
    let setup = setup_root_cached_topology();

    assert_children_match_registry(&setup.pic, setup.root_id);
}

#[test]
fn state_endpoints_are_root_only() {
    let setup = setup_root_cached_topology();

    let app_pid = setup
        .subnet_directory
        .get(&canister::APP)
        .copied()
        .expect("app must exist in subnet directory");

    assert_state_endpoints_are_root_only(&setup.pic, setup.root_id, app_pid);
}

#[test]
fn worker_topology_cascades_through_parent() {
    let setup = setup_root();

    let scale_hub_pid = setup
        .subnet_directory
        .get(&canister::SCALE_HUB)
        .copied()
        .expect("scale_hub must exist in subnet directory");

    let before = count_workers(&setup.pic, setup.root_id, scale_hub_pid);

    match create_worker(&setup.pic, scale_hub_pid) {
        Ok(_) => {}
        Err(err) if is_threshold_key_unavailable(&err) => {
            eprintln!(
                "skipping worker_topology_cascades_through_parent: threshold key unavailable: {err}"
            );
            return;
        }
        Err(err) => panic!("create_worker application failed: {err:?}"),
    }
    setup.pic.tick_n(10);

    let after = count_workers(&setup.pic, setup.root_id, scale_hub_pid);

    assert_eq!(after, before + 1);
}

fn is_threshold_key_unavailable(err: &canic::Error) -> bool {
    err.message.contains("Requested unknown threshold key")
        || err.message.contains("existing keys: []")
}
