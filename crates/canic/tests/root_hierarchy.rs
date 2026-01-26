mod root;

use canic::ids::CanisterRole;
use canic_internal::canister;
use root::{
    assertions::{
        assert_child_env, assert_children_match_registry, assert_directories_consistent,
        assert_registry_parents,
    },
    harness::setup_root,
    workers::{count_workers, create_worker},
};

///
/// TESTS
///

#[test]
fn root_builds_hierarchy_and_exposes_env() {
    let setup = setup_root();

    assert_registry_parents(
        &setup.pic,
        setup.root_id,
        &[
            (CanisterRole::ROOT, None),
            (canister::APP, Some(setup.root_id)),
            (canister::USER_HUB, Some(setup.root_id)),
            (canister::SCALE_HUB, Some(setup.root_id)),
            (canister::SHARD_HUB, Some(setup.root_id)),
        ],
    );

    for (role, pid) in &setup.subnet_directory {
        if !role.is_root() {
            assert_child_env(&setup.pic, *pid, role.clone(), setup.root_id);
        }
    }
}

#[test]
fn directories_are_consistent_across_canisters() {
    let setup = setup_root();

    assert_directories_consistent(&setup.pic, setup.root_id, &setup.subnet_directory);
}

#[test]
fn subnet_children_matches_registry_on_root() {
    let setup = setup_root();

    assert_children_match_registry(&setup.pic, setup.root_id);
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

    create_worker(&setup.pic, scale_hub_pid);
    setup.pic.tick_n(10);

    let after = count_workers(&setup.pic, setup.root_id, scale_hub_pid);

    assert_eq!(after, before + 1);
}
