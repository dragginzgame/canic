use canic::{
    Error,
    dto::state::{FleetCommand, FleetCommandResponse, FleetStatus},
    ids::CanisterRole,
    protocol,
};
use canic_testing_internal::canister;
use canic_tests::root::{
    RootSetupProfile,
    assertions::{
        assert_child_envs_match_registry, assert_child_runtime_introspection_is_controller_gated,
        assert_children_match_registry, assert_fleet_state_endpoint_is_root_only,
        assert_registry_parents, assert_root_diagnostics_are_controller_gated,
    },
    harness::{setup_cached_root, setup_root},
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
        .subnet_directory
        .get(&canister::APP)
        .copied()
        .expect("app must exist in Subnet Directory");

    test_progress(
        "root_reference_topology_is_consistent",
        "assert Fleet state endpoint is root-only",
    );
    assert_fleet_state_endpoint_is_root_only(&setup.pic, setup.root_id, app_pid);
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

#[test]
fn exact_fleet_command_retry_repairs_a_failed_child_cascade() {
    let setup = setup_cached_root(RootSetupProfile::Topology);
    let user_hub = setup
        .subnet_directory
        .get(&canister::USER_HUB)
        .copied()
        .expect("user hub must exist");
    let command = FleetCommand::SetStatus(FleetStatus::Readonly);

    let stopped: Result<(), Error> = setup.pic.update_call_or_panic(
        setup.root_id,
        "test_set_canister_running",
        (user_hub, false),
    );
    stopped.expect("root controller must stop user hub");
    let first: Result<FleetCommandResponse, Error> =
        setup
            .pic
            .update_call_or_panic(setup.root_id, protocol::CANIC_FLEET_ADMIN, (command,));
    assert!(
        first.is_err(),
        "failed child fanout must reject the root command"
    );

    let started: Result<(), Error> = setup.pic.update_call_or_panic(
        setup.root_id,
        "test_set_canister_running",
        (user_hub, true),
    );
    started.expect("root controller must restart user hub");
    let retried: Result<FleetCommandResponse, Error> =
        setup
            .pic
            .update_call_or_panic(setup.root_id, protocol::CANIC_FLEET_ADMIN, (command,));
    assert!(matches!(
        retried,
        Ok(FleetCommandResponse::Status(response)) if !response.changed
    ));

    let child_update: Result<(), Error> = setup.pic.update_call_or_panic(
        user_hub,
        "test_set_recovery_generation",
        ("must-not-run".to_string(),),
    );
    assert!(
        child_update.is_err(),
        "the restarted child must receive the retried Readonly state"
    );
    drop(setup);
}
