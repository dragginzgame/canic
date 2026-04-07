use crate::root::{
    RootSetupProfile,
    assertions::{
        assert_child_env, assert_children_match_registry, assert_directories_consistent,
        assert_registry_parents, assert_state_endpoints_are_root_only,
    },
    harness::setup_cached_root,
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
fn root_reference_topology_is_consistent() {
    test_progress(
        "root_reference_topology_is_consistent",
        "setup cached topology",
    );
    let setup = setup_cached_root(RootSetupProfile::Topology);

    test_progress(
        "root_reference_topology_is_consistent",
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

    test_progress(
        "root_reference_topology_is_consistent",
        "assert each child env",
    );
    for (role, pid) in &setup.subnet_directory {
        if !role.is_root() && *role != CanisterRole::WASM_STORE {
            assert_child_env(&setup.pic, *pid, role.clone(), setup.root_id);
        }
    }

    test_progress(
        "root_reference_topology_is_consistent",
        "assert directories consistent",
    );
    assert_directories_consistent(&setup.pic, setup.root_id, &setup.subnet_directory);

    test_progress(
        "root_reference_topology_is_consistent",
        "assert children match registry",
    );
    assert_children_match_registry(&setup.pic, setup.root_id);

    let app_pid = setup
        .subnet_directory
        .get(&canister::APP)
        .copied()
        .expect("app must exist in subnet directory");

    test_progress(
        "root_reference_topology_is_consistent",
        "assert state endpoints are root-only",
    );
    assert_state_endpoints_are_root_only(&setup.pic, setup.root_id, app_pid);
    test_progress("root_reference_topology_is_consistent", "done");
}
