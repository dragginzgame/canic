// Category C - Artifact / deployment test (embedded config).
// This test exercises the maintained application timer surface in PocketIC.

use candid::Principal;
use canic::{
    dto::runtime::{
        CanicRuntimeStatus, TimerProcessCondition, TimerRegistrationStatus, TimerSchedulingMode,
    },
    protocol,
};
use canic_testing_internal::pic::{CanicPicExt, install_lifecycle_boundary_fixture, upgrade_args};
use std::time::Duration;

const READY_TICK_LIMIT: usize = 120;
const INSTALL_CODE_RETRY_LIMIT: usize = 4;
const INSTALL_CODE_COOLDOWN: Duration = Duration::from_mins(5);

#[test]
fn application_timers_cancel_and_recur_only_after_completion() {
    let fixture = install_lifecycle_boundary_fixture();
    let canister_id = fixture.install_canic_canister();
    fixture
        .pic
        .wait_for_ready(canister_id, READY_TICK_LIMIT, "install");

    fixture.pic.advance_time(Duration::from_secs(6));
    tick(&fixture.pic, 4);
    let first = counts(&fixture.pic, canister_id);
    assert_eq!(first.0, 1, "one-shot should execute exactly once");
    assert_eq!(first.2, 0, "cancelled one-shot must not execute");

    fixture.pic.advance_time(Duration::from_secs(10));
    tick(&fixture.pic, 4);
    let second = counts(&fixture.pic, canister_id);
    assert_eq!(second.1, first.1.saturating_add(1));

    fixture.pic.advance_time(Duration::from_secs(30));
    tick(&fixture.pic, 4);
    let third = counts(&fixture.pic, canister_id);
    assert_eq!(
        third.1,
        second.1.saturating_add(1),
        "after-completion recurrence must not replay missed fixed-rate ticks"
    );
    assert_eq!(third.0, 1);
    assert_eq!(third.2, 0);

    let status = runtime_status(&fixture.pic, canister_id);
    let interval = status
        .timers
        .iter()
        .find(|timer| timer.subsystem == "runtime_probe" && timer.name == "timer_interval")
        .expect("live interval registration");
    assert_eq!(interval.registration, TimerRegistrationStatus::Scheduled);
    assert_eq!(interval.condition, TimerProcessCondition::Active);
    assert_eq!(
        interval.scheduling_mode,
        TimerSchedulingMode::AfterCompletion
    );
    assert!(
        status
            .timers
            .iter()
            .all(|timer| { timer.name != "timer_once" && timer.name != "timer_cancelled" })
    );
    let log_retention = status
        .timers
        .iter()
        .find(|timer| timer.subsystem == "log_retention" && timer.name == "run")
        .expect("log retention runtime status");
    assert_eq!(
        log_retention.registration,
        TimerRegistrationStatus::Unregistered
    );
    assert_eq!(log_retention.condition, TimerProcessCondition::Idle);
    assert_eq!(log_retention.next_due_at_ns, None);
    assert_eq!(log_retention.executions_since_runtime_start, 0);
}

#[test]
fn finite_intent_expiry_is_rebuilt_after_upgrade_without_arming_ttl_free_work() {
    let fixture = install_lifecycle_boundary_fixture();
    let canister_id = fixture.install_canic_canister();
    fixture
        .pic
        .wait_for_ready(canister_id, READY_TICK_LIMIT, "install");

    let idle = intent_cleanup_status(&fixture.pic, canister_id);
    assert_eq!(idle.registration, TimerRegistrationStatus::Unregistered);
    assert_eq!(idle.condition, TimerProcessCondition::Idle);
    assert_eq!(idle.next_due_at_ns, None);

    begin_intent(&fixture.pic, canister_id, 1, Some(600))
        .expect("finite intent reservation should succeed");
    assert!(
        begin_intent(&fixture.pic, canister_id, 1, Some(600)).is_err(),
        "an unexpired reservation must retain its capacity"
    );

    let scheduled = intent_cleanup_status(&fixture.pic, canister_id);
    assert_eq!(scheduled.registration, TimerRegistrationStatus::Scheduled);
    assert_eq!(scheduled.condition, TimerProcessCondition::Active);
    assert_eq!(scheduled.scheduling_mode, TimerSchedulingMode::Deadline);
    assert!(scheduled.next_due_at_ns.is_some());

    fixture
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    fixture
        .pic
        .retry_install_code_ok(INSTALL_CODE_RETRY_LIMIT, INSTALL_CODE_COOLDOWN, || {
            fixture
                .pic
                .upgrade_canister(
                    canister_id,
                    fixture.canic_wasm.clone(),
                    upgrade_args(),
                    None,
                )
                .map_err(|err| err.to_string())
        })
        .expect("upgrade should succeed");
    fixture
        .pic
        .wait_for_ready(canister_id, READY_TICK_LIMIT, "post_upgrade");

    let rebuilt = intent_cleanup_status(&fixture.pic, canister_id);
    assert_eq!(rebuilt.registration, TimerRegistrationStatus::Scheduled);
    assert_eq!(rebuilt.condition, TimerProcessCondition::Active);
    assert_eq!(rebuilt.scheduling_mode, TimerSchedulingMode::Deadline);

    fixture.pic.advance_time(Duration::from_secs(302));
    tick(&fixture.pic, 8);
    begin_intent(&fixture.pic, canister_id, 1, Some(600))
        .expect("expired reservation should release capacity after lifecycle rebuild");

    begin_intent(&fixture.pic, canister_id, 2, None).expect("TTL-free reservation should succeed");
    fixture.pic.advance_time(Duration::from_hours(24));
    tick(&fixture.pic, 8);
    assert!(
        begin_intent(&fixture.pic, canister_id, 2, None).is_err(),
        "TTL-free reservation must not be treated as expirable work"
    );
    let idle = intent_cleanup_status(&fixture.pic, canister_id);
    assert_eq!(idle.registration, TimerRegistrationStatus::Unregistered);
    assert_eq!(idle.condition, TimerProcessCondition::Idle);
    assert_eq!(idle.next_due_at_ns, None);
}

fn runtime_status(pic: &ic_testkit::pic::Pic, canister_id: Principal) -> CanicRuntimeStatus {
    let result: Result<CanicRuntimeStatus, canic::Error> = pic
        .query_call(canister_id, protocol::CANIC_RUNTIME_STATUS, ())
        .expect("query runtime status");
    result.expect("runtime status application result")
}

fn intent_cleanup_status(
    pic: &ic_testkit::pic::Pic,
    canister_id: Principal,
) -> canic::dto::runtime::CanicTimerStatus {
    runtime_status(pic, canister_id)
        .timers
        .into_iter()
        .find(|timer| timer.subsystem == "intent_cleanup" && timer.name == "run")
        .expect("intent cleanup runtime status")
}

fn begin_intent(
    pic: &ic_testkit::pic::Pic,
    canister_id: Principal,
    resource_seed: u8,
    ttl_secs: Option<u64>,
) -> Result<u64, canic::Error> {
    pic.update_call(
        canister_id,
        "begin_timer_probe_intent",
        (resource_seed, ttl_secs),
    )
    .expect("call intent reservation endpoint")
}

fn counts(pic: &ic_testkit::pic::Pic, canister_id: Principal) -> (u64, u64, u64) {
    let result: Result<(u64, u64, u64), canic::Error> = pic
        .query_call(canister_id, "timer_probe_counts", ())
        .expect("query timer probe counts");
    result.expect("timer probe counts application result")
}

fn tick(pic: &ic_testkit::pic::Pic, count: usize) {
    for _ in 0..count {
        pic.tick();
    }
}
