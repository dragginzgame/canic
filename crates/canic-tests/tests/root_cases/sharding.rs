use crate::root::{assertions::assert_registry_parents, harness::setup_root_cached_sharding};
use canic::{Error, cdk::types::Principal, ids::CanisterRole};
use canic_internal::canister;

#[test]
fn user_hub_sharding_profile_stays_minimal_and_can_create_a_shard() {
    let setup = setup_root_cached_sharding();

    assert!(
        !setup.subnet_directory.contains_key(&canister::APP),
        "sharding profile should not boot app",
    );
    assert!(
        !setup.subnet_directory.contains_key(&canister::SCALE_HUB),
        "sharding profile should not boot scale_hub",
    );

    let user_hub_pid = setup
        .subnet_directory
        .get(&canister::USER_HUB)
        .copied()
        .expect("user_hub must exist in sharding profile");

    let created: Result<Result<Principal, Error>, Error> = setup.pic.update_call(
        user_hub_pid,
        "create_account",
        (Principal::from_slice(&[7; 29]),),
    );
    let shard_pid = created
        .expect("create_account transport failed")
        .expect("create_account application failed");
    setup
        .pic
        .wait_for_ready(shard_pid, 50, "user shard bootstrap");

    assert_registry_parents(
        &setup.pic,
        setup.root_id,
        &[
            (CanisterRole::ROOT, None),
            (canister::USER_HUB, Some(setup.root_id)),
            (canister::TEST, Some(setup.root_id)),
            (canister::USER_SHARD, Some(user_hub_pid)),
        ],
    );
}
