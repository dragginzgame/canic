use std::time::Duration;

use canic::{
    Error,
    cdk::types::Principal,
    dto::{
        placement::sharding::ShardingRegistryResponse,
        runtime::{
            CanicRuntimeStatus, CanicTimerStatus, TimerExecutionOutcome, TimerProcessCondition,
            TimerRegistrationStatus,
        },
    },
    ids::CanisterRole,
    protocol,
};
use canic_testing_internal::canister;
use canic_testing_internal::pic::CanicPicExt;
use canic_tests::root::{
    RootSetupProfile,
    assertions::assert_registry_parents,
    harness::{RootSetup, setup_cached_root},
};

#[test]
fn user_hub_sharding_profile_prewarms_first_user_shard() {
    let setup = setup_cached_root(RootSetupProfile::Sharding);

    assert!(
        !setup.subnet_index.contains_key(&canister::APP),
        "sharding profile should not boot app",
    );
    assert!(
        !setup.subnet_index.contains_key(&canister::SCALE_HUB),
        "sharding profile should not boot scale_hub",
    );

    let user_hub_pid = user_hub_pid(&setup);

    let startup_shard_pid = startup_user_shard_pid(&setup, user_hub_pid);

    let created: Result<Result<Principal, Error>, _> = setup.pic.update_call(
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
fn nested_shard_funding_recovers_after_parent_same_round_refill() {
    let setup = setup_cached_root(RootSetupProfile::Sharding);
    let user_hub_pid = user_hub_pid(&setup);
    let shard_pid = startup_user_shard_pid(&setup, user_hub_pid);
    let before = cycle_topup_timer(&setup, user_hub_pid, shard_pid);

    setup.pic.advance_time(Duration::from_hours(1));
    tick(&setup, 16);
    let exhausted = cycle_topup_timer(&setup, user_hub_pid, shard_pid);
    assert_eq!(
        exhausted.last_outcome,
        Some(TimerExecutionOutcome::RetryableFailure)
    );
    assert_eq!(exhausted.registration, TimerRegistrationStatus::Scheduled);
    assert_eq!(exhausted.condition, TimerProcessCondition::Retrying);
    assert_eq!(
        exhausted.expected_failures_since_runtime_start,
        before.expected_failures_since_runtime_start + 1
    );

    setup.pic.advance_time(Duration::from_mins(1));
    tick(&setup, 16);
    let recovered = cycle_topup_timer(&setup, user_hub_pid, shard_pid);
    drop(setup);
    assert_eq!(recovered.registration, TimerRegistrationStatus::Scheduled);
    assert_eq!(recovered.condition, TimerProcessCondition::Active);
    assert!(
        recovered.successes_since_runtime_start > before.successes_since_runtime_start,
        "nested shard should obtain funding after its parent refills"
    );
    assert_eq!(recovered.consecutive_expected_failures, 0);
}

fn startup_user_shard_pid(setup: &RootSetup, user_hub_pid: Principal) -> Principal {
    let registry: Result<Result<ShardingRegistryResponse, Error>, _> =
        setup
            .pic
            .query_call_as(user_hub_pid, setup.root_id, "canic_sharding_registry", ());
    registry
        .expect("registry query transport failed")
        .expect("registry query application failed")
        .0
        .into_iter()
        .find(|entry| entry.entry.pool == "user_shards")
        .map(|entry| entry.pid)
        .expect("startup user shard must exist before first account create")
}

fn cycle_topup_timer(
    setup: &RootSetup,
    caller: Principal,
    canister_id: Principal,
) -> CanicTimerStatus {
    let status: Result<Result<CanicRuntimeStatus, Error>, _> =
        setup
            .pic
            .query_call_as(canister_id, caller, protocol::CANIC_RUNTIME_STATUS, ());
    status
        .expect("runtime status transport failed")
        .expect("runtime status application failed")
        .timers
        .into_iter()
        .find(|timer| timer.subsystem == "cycles" && timer.name == "topup")
        .expect("nested shard should expose cycle top-up ownership")
}

fn tick(setup: &RootSetup, count: usize) {
    for _ in 0..count {
        setup.pic.tick();
    }
}

fn user_hub_pid(setup: &RootSetup) -> Principal {
    sharding_profile_pid(setup, &canister::USER_HUB, "user_hub")
}

fn sharding_profile_pid(setup: &RootSetup, role: &CanisterRole, label: &str) -> Principal {
    setup
        .subnet_index
        .get(role)
        .copied()
        .unwrap_or_else(|| panic!("{label} must exist in sharding profile"))
}
