use crate::root::{
    RootSetupProfile, assertions::assert_registry_parents, harness::setup_cached_root,
};
use canic::{
    Error, cdk::types::Principal, dto::placement::sharding::ShardingRegistryResponse,
    ids::CanisterRole,
};
use canic_internal::canister;

#[test]
fn user_hub_sharding_profile_prewarms_first_shard_signing_key() {
    let setup = setup_cached_root(RootSetupProfile::Sharding);

    assert!(
        !setup.subnet_index.contains_key(&canister::APP),
        "sharding profile should not boot app",
    );
    assert!(
        !setup.subnet_index.contains_key(&canister::SCALE_HUB),
        "sharding profile should not boot scale_hub",
    );

    let user_hub_pid = setup
        .subnet_index
        .get(&canister::USER_HUB)
        .copied()
        .expect("user_hub must exist in sharding profile");

    let registry: Result<Result<ShardingRegistryResponse, Error>, Error> =
        setup
            .pic
            .query_call_as(user_hub_pid, setup.root_id, "canic_sharding_registry", ());
    let registry = registry
        .expect("registry query transport failed")
        .expect("registry query application failed");
    let startup_shard_pid = registry
        .0
        .into_iter()
        .find(|entry| entry.entry.pool == "user_shards")
        .map(|entry| entry.pid)
        .expect("startup user shard must exist before first account create");

    let shard_public_key: Result<Result<Vec<u8>, Error>, Error> =
        setup
            .pic
            .update_call(startup_shard_pid, "user_shard_local_public_key_test", ());
    assert!(
        !shard_public_key
            .expect("signing key update transport failed")
            .expect("signing key update application failed")
            .is_empty(),
        "startup user shard must have local signer key material before first account create",
    );

    let created: Result<Result<Principal, Error>, Error> = setup.pic.update_call(
        user_hub_pid,
        "create_account",
        (Principal::from_slice(&[7; 29]),),
    );
    let shard_pid = created
        .expect("create_account transport failed")
        .expect("create_account application failed");
    assert_eq!(shard_pid, startup_shard_pid);
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
