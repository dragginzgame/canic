// Category C - Artifact / deployment test (embedded config).
// This test exercises the maintained application timer surface in PocketIC.

use candid::Principal;
use canic::{
    dto::runtime::{
        CanicRuntimeStatus, TimerProcessCondition, TimerRegistrationStatus, TimerSchedulingMode,
    },
    protocol,
};
use canic_testing_internal::pic::{CanicPicExt, install_lifecycle_boundary_fixture};
use std::time::Duration;

const READY_TICK_LIMIT: usize = 120;

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
}

fn runtime_status(pic: &ic_testkit::pic::Pic, canister_id: Principal) -> CanicRuntimeStatus {
    let result: Result<CanicRuntimeStatus, canic::Error> = pic
        .query_call(canister_id, protocol::CANIC_RUNTIME_STATUS, ())
        .expect("query runtime status");
    result.expect("runtime status application result")
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
