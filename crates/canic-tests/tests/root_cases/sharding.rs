use crate::root::{
    RootSetupProfile, assertions::assert_registry_parents, harness::setup_cached_root,
};
use canic::{
    Error,
    cdk::types::Principal,
    dto::{
        auth::DelegationAudience, placement::sharding::ShardingRegistryResponse,
        state::SubnetStateResponse,
    },
    ids::{CanisterRole, cap},
    protocol,
};
use canic_reference_support::canister;
use canic_testing_internal::pic::{
    create_user_shard, issue_delegated_token, request_root_delegation_provision,
};

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

#[test]
fn delegated_token_verification_uses_cascaded_subnet_state_root_key() {
    let setup = setup_cached_root(RootSetupProfile::Sharding);

    let root_state: Result<SubnetStateResponse, Error> = setup
        .pic
        .query_call(setup.root_id, protocol::CANIC_SUBNET_STATE, ())
        .expect("root subnet state transport failed");
    let root_key = root_state
        .expect("root subnet state application failed")
        .auth
        .delegated_root_public_key
        .expect("root must publish delegated root key into subnet state");
    assert!(
        !root_key.public_key_sec1.is_empty(),
        "published delegated root key must have SEC1 bytes",
    );

    let user_hub_pid = setup
        .subnet_index
        .get(&canister::USER_HUB)
        .copied()
        .expect("user_hub must exist in sharding profile");
    let verifier_pid = setup
        .subnet_index
        .get(&canister::TEST)
        .copied()
        .expect("test verifier must exist in sharding profile");

    let subject = Principal::from_slice(&[55; 29]);
    let shard_pid = create_user_shard(&setup.pic, user_hub_pid, subject);
    let provision =
        request_root_delegation_provision(&setup.pic, setup.root_id, shard_pid, verifier_pid);
    let token = issue_delegated_token(
        &setup.pic,
        shard_pid,
        subject,
        DelegationAudience::Principals(vec![verifier_pid]),
        vec![cap::VERIFY.to_string()],
        provision.cert.max_token_ttl_secs,
        provision
            .cert
            .expires_at
            .saturating_sub(provision.cert.issued_at),
    );

    let verified: Result<Result<(), Error>, Error> = setup.pic.update_call_as(
        verifier_pid,
        subject,
        "test_verify_delegated_token",
        (token,),
    );
    verified
        .expect("delegated token verifier transport failed")
        .expect("delegated token verifier application failed");
}
