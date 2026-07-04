use canic::ids::CanisterRole;
use canic_testing_internal::canister;
use canic_tests::root::{
    RootSetupProfile,
    assertions::{
        assert_child_envs_match_registry, assert_child_runtime_introspection_is_controller_gated,
        assert_children_match_registry, assert_registry_parents,
        assert_root_diagnostics_are_controller_gated, assert_state_endpoints_are_root_only,
    },
    harness::setup_root,
};
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
        "setup fresh topology",
    );
    let setup = setup_root(RootSetupProfile::Topology);

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
    assert_child_envs_match_registry(&setup.pic, setup.root_id);

    test_progress(
        "root_reference_topology_is_consistent",
        "assert children match registry",
    );
    assert_children_match_registry(&setup.pic, setup.root_id);

    let app_pid = setup
        .subnet_index
        .get(&canister::APP)
        .copied()
        .expect("app must exist in subnet index");

    test_progress(
        "root_reference_topology_is_consistent",
        "assert state endpoints are root-only",
    );
    assert_state_endpoints_are_root_only(&setup.pic, setup.root_id, app_pid);
    test_progress(
        "root_reference_topology_is_consistent",
        "assert root diagnostics are controller-gated",
    );
    assert_root_diagnostics_are_controller_gated(&setup.pic, setup.root_id);
    test_progress(
        "root_reference_topology_is_consistent",
        "assert child runtime introspection is controller-gated",
    );
    assert_child_runtime_introspection_is_controller_gated(
        &setup.pic,
        app_pid,
        &canister::APP,
        setup.root_id,
        setup.root_id,
    );
    drop(setup);
    test_progress("root_reference_topology_is_consistent", "done");
}
