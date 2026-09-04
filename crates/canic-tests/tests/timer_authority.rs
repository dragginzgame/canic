// Category C - Artifact / deployment test (embedded config).
// This test exercises direct application-owned native timer custody in PocketIC.

use candid::{CandidType, Deserialize, Principal};
use canic::{
    Error,
    dto::{
        cycles::{CycleTopupEvent, CycleTrackerEntry},
        metrics::{MetricEntry, MetricValue, MetricsKind},
        page::{Page, PageRequest},
        role::{CycleBalanceStatusResponse, MetricsStatusRequest},
        runtime::{
            CanicRuntimeStatus, RuntimeCheckStatus, TimerProcessCondition, TimerRegistrationStatus,
            TimerSchedulingMode,
        },
    },
    protocol,
};
use canic_testing_internal::pic::{CanicPicExt, install_lifecycle_boundary_fixture, upgrade_args};
use ic_testkit::pic::{CandidCallExt, CanisterInstallExt, PocketIc, RetryPolicy};
use std::time::Duration;

const READY_TICK_LIMIT: usize = 120;
const INSTALL_CODE_RETRY_LIMIT: usize = 4;
const INSTALL_CODE_COOLDOWN: Duration = Duration::from_mins(5);

#[derive(CandidType, Clone)]
enum RoleStatusRequest {
    CycleBalance,
    CycleHistory(PageRequest),
    CycleTopups(PageRequest),
    Metrics(MetricsStatusRequest),
    Runtime,
}

#[derive(CandidType, Deserialize)]
enum RoleStatusResponse {
    CycleBalance(CycleBalanceStatusResponse),
    CycleHistory(Page<CycleTrackerEntry>),
    CycleTopups(Page<CycleTopupEvent>),
    Metrics(Page<MetricEntry>),
    Runtime(Box<CanicRuntimeStatus>),
}

#[test]
fn exact_cycles_and_runtime_metrics_require_controller() {
    let fixture = install_lifecycle_boundary_fixture();
    let standalone = fixture.install_runtime_probe_canister();
    let managed = fixture.install_canic_canister();
    let automatic_topup = fixture.install_automatic_topup_canister();

    for (canister_id, supports_topups) in [
        (standalone, false),
        (managed, false),
        (automatic_topup, true),
    ] {
        for request in sensitive_observability_requests(supports_topups) {
            let denied: Result<RoleStatusResponse, Error> = fixture.pic.query_candid_as_or_panic(
                canister_id,
                fixture.root,
                protocol::CANIC_STATUS,
                (request.clone(),),
            );
            assert!(matches!(
                denied,
                Err(error)
                    if error.code()
                        == canic::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
            ));

            let accepted: Result<RoleStatusResponse, Error> = fixture
                .pic
                .query_candid(canister_id, protocol::CANIC_STATUS, (request.clone(),))
                .expect("controller observability transport");
            let accepted = accepted.expect("controller observability application result");
            assert!(matches!(
                (&request, accepted),
                (
                    RoleStatusRequest::CycleBalance,
                    RoleStatusResponse::CycleBalance(_)
                ) | (
                    RoleStatusRequest::CycleHistory(_),
                    RoleStatusResponse::CycleHistory(_)
                ) | (
                    RoleStatusRequest::CycleTopups(_),
                    RoleStatusResponse::CycleTopups(_)
                ) | (
                    RoleStatusRequest::Metrics(_),
                    RoleStatusResponse::Metrics(_)
                )
            ));
        }
    }
}

fn sensitive_observability_requests(supports_topups: bool) -> Vec<RoleStatusRequest> {
    let page = PageRequest {
        limit: 1,
        offset: 0,
    };
    let mut requests = vec![
        RoleStatusRequest::CycleBalance,
        RoleStatusRequest::CycleHistory(page),
        RoleStatusRequest::Metrics(MetricsStatusRequest {
            kind: MetricsKind::Runtime,
            page,
        }),
    ];
    if supports_topups {
        requests.push(RoleStatusRequest::CycleTopups(page));
    }
    requests
}

#[test]
fn lifecycle_participant_reconstructs_native_timers_before_deferred_hooks() {
    let fixture = install_lifecycle_boundary_fixture();
    let canister_id = fixture.install_runtime_probe_canister();

    assert_application_timer_rows(&fixture.pic, canister_id);

    wait_for_fixture_ready(&fixture.pic, canister_id, "install");

    fixture
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    fixture
        .pic
        .retry_install_code(install_retry_policy(), || {
            fixture.pic.upgrade_canister(
                canister_id,
                fixture.runtime_probe_wasm.clone(),
                upgrade_args(),
                None,
            )
        })
        .expect("runtime probe upgrade should succeed");

    assert_application_timer_rows(&fixture.pic, canister_id);

    wait_for_fixture_ready(&fixture.pic, canister_id, "post_upgrade");
}

#[test]
fn application_timers_cancel_and_recur_only_after_completion() {
    let fixture = install_lifecycle_boundary_fixture();
    let canister_id = fixture.install_runtime_probe_canister();
    wait_for_fixture_ready(&fixture.pic, canister_id, "install");

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
    assert_eq!(status.timer_inventory.status, RuntimeCheckStatus::Pass);
    report_timer_inventory(&status);
    let interval = status
        .timers
        .iter()
        .find(|timer| {
            timer.owner == "runtime-probe"
                && timer.subsystem == "application"
                && timer.name == "timer-interval"
        })
        .expect("live interval registration");
    assert_eq!(interval.registration, TimerRegistrationStatus::Scheduled);
    assert_eq!(interval.condition, TimerProcessCondition::Active);
    assert_eq!(
        interval.scheduling_mode,
        TimerSchedulingMode::AfterCompletion
    );
    assert_interval_performance(interval);
    assert!(
        status
            .timers
            .iter()
            .all(|timer| { timer.name != "timer-once" && timer.name != "timer-cancelled" }),
        "terminal RemoveWhenStopped timers and their observations must leave inventory"
    );
    assert!(
        status
            .timers
            .iter()
            .all(|timer| timer.subsystem != "auth_renewal"),
        "non-root inventory must not reserve the root-only issuer-renewal job"
    );
    assert!(
        status
            .timers
            .iter()
            .all(|timer| { timer.subsystem != "placement" || timer.name != "receipt_ack" }),
        "an empty receipt index must not declare placement acknowledgement"
    );
    assert!(status.timers.iter().any(|timer| {
        timer.owner == "companion-framework"
            && timer.subsystem == "inventory"
            && timer.name == "visible"
    }));
    let receipt_capacity = status
        .receipt_capacity
        .as_ref()
        .expect("guarded receipt capacity status");
    assert_eq!(receipt_capacity.status, RuntimeCheckStatus::Pass);
    assert_eq!(receipt_capacity.receipt_records, 0);
    assert_eq!(receipt_capacity.receipt_record_limit, 1_000);
    assert_eq!(receipt_capacity.resource_total_records, 0);
    assert_eq!(receipt_capacity.resource_total_record_limit, 1_000);
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
    assert!(
        status
            .timers
            .iter()
            .all(|timer| timer.subsystem != "cycles" || timer.name != "topup"),
        "a profile without AutomaticTopup must not declare the top-up registration"
    );
}

fn report_timer_inventory(status: &CanicRuntimeStatus) {
    let scheduled = status
        .timers
        .iter()
        .filter(|timer| timer.registration == TimerRegistrationStatus::Scheduled)
        .count();
    eprintln!(
        "runtime-probe timer inventory: declared={} scheduled={scheduled}",
        status.timers.len()
    );
}

fn assert_interval_performance(interval: &canic::dto::runtime::CanisterTimerStatus) {
    eprintln!(
        "runtime-probe interval performance: scheduler_samples={} work_samples={} latest={:?} maximum={:?} total={} max_wasm_growth={:?} max_stable_growth={:?}",
        interval
            .scheduler_performance
            .instruction_samples_since_runtime_start,
        interval
            .work_performance
            .instruction_samples_since_runtime_start,
        interval.work_performance.instructions_latest,
        interval.work_performance.instructions_maximum,
        interval
            .work_performance
            .instructions_total_since_runtime_start,
        interval.work_performance.maximum_wasm_memory_growth_pages,
        interval.work_performance.maximum_stable_memory_growth_pages,
    );
    assert_eq!(
        interval
            .scheduler_performance
            .instruction_samples_since_runtime_start,
        0,
        "ordinary after-completion timers have no separate scheduler callback"
    );
    assert!(
        interval
            .work_performance
            .instruction_samples_since_runtime_start
            > 0,
        "completed timer work must retain instruction observations"
    );
    assert_eq!(
        interval
            .work_performance
            .memory_page_samples_since_runtime_start,
        interval
            .work_performance
            .instruction_samples_since_runtime_start,
        "normal work completion must retain paired instruction and memory samples"
    );
    let latest_memory = interval
        .work_performance
        .memory_pages_latest
        .as_ref()
        .expect("completed timer work memory-page sample");
    assert!(latest_memory.end.wasm_pages >= latest_memory.start.wasm_pages);
    assert!(latest_memory.end.stable_pages >= latest_memory.start.stable_pages);
}

#[test]
fn timer_registration_capacity_and_invalid_identity_are_leak_free() {
    let fixture = install_lifecycle_boundary_fixture();
    let canister_id = fixture.install_runtime_probe_canister();
    wait_for_fixture_ready(&fixture.pic, canister_id, "install");

    let before_invalid = runtime_status(&fixture.pic, canister_id).timers.len();
    let invalid: Result<bool, Error> = fixture
        .pic
        .update_candid(canister_id, "reject_invalid_timer_identity", ())
        .expect("call invalid timer identity probe");
    assert!(invalid.expect("invalid identity probe result"));
    assert_eq!(
        runtime_status(&fixture.pic, canister_id).timers.len(),
        before_invalid,
        "rejected identity must not consume shared registry capacity"
    );

    let fill: Result<(u64, bool), Error> = fixture
        .pic
        .update_candid(canister_id, "fill_timer_registry", ())
        .expect("fill shared timer registry");
    let (registered, rejected) = fill.expect("capacity probe result");
    assert!(registered > 0);
    assert!(
        rejected,
        "registry demand beyond the bound must be rejected"
    );

    let status = runtime_status(&fixture.pic, canister_id);
    assert_eq!(status.timer_inventory.status, RuntimeCheckStatus::Pass);
    assert_eq!(status.timers.len(), ic_timers::MAX_TIMER_REGISTRATIONS);
}

#[test]
fn finite_intent_expiry_is_rebuilt_after_upgrade_without_arming_ttl_free_work() {
    let fixture = install_lifecycle_boundary_fixture();
    let canister_id = fixture.install_runtime_probe_canister();
    wait_for_fixture_ready(&fixture.pic, canister_id, "install");

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
        .retry_install_code(install_retry_policy(), || {
            fixture.pic.upgrade_canister(
                canister_id,
                fixture.runtime_probe_wasm.clone(),
                upgrade_args(),
                None,
            )
        })
        .expect("upgrade should succeed");
    wait_for_fixture_ready(&fixture.pic, canister_id, "post_upgrade");

    let rebuilt = intent_cleanup_status(&fixture.pic, canister_id);
    assert_eq!(rebuilt.registration, TimerRegistrationStatus::Scheduled);
    assert_eq!(rebuilt.condition, TimerProcessCondition::Active);
    assert_eq!(rebuilt.scheduling_mode, TimerSchedulingMode::Deadline);

    fixture.pic.advance_time(Duration::from_secs(302));
    tick(&fixture.pic, 8);
    begin_intent(&fixture.pic, canister_id, 1, Some(600))
        .expect("expired reservation should release capacity after lifecycle rebuild");
    fixture.pic.advance_time(Duration::from_secs(602));
    tick(&fixture.pic, 8);
    let idle = intent_cleanup_status(&fixture.pic, canister_id);
    assert_eq!(idle.registration, TimerRegistrationStatus::Unregistered);
    assert_eq!(idle.condition, TimerProcessCondition::Idle);

    begin_intent(&fixture.pic, canister_id, 2, None).expect("TTL-free reservation should succeed");
    let idle_timer_metrics = timer_metrics(&fixture.pic, canister_id);
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
    let after_timer_metrics = timer_metrics(&fixture.pic, canister_id);
    for label in ["cycles:topup", "intent_cleanup:run", "log_retention:run"] {
        assert_eq!(
            timer_metric(&after_timer_metrics, label),
            timer_metric(&idle_timer_metrics, label),
            "idle timer owner {label} must execute no callback during 24 hours"
        );
    }
}

fn runtime_status(pic: &PocketIc, canister_id: Principal) -> CanicRuntimeStatus {
    let result: Result<RoleStatusResponse, canic::Error> = pic
        .query_candid(
            canister_id,
            protocol::CANIC_STATUS,
            (RoleStatusRequest::Runtime,),
        )
        .expect("query runtime status");
    let RoleStatusResponse::Runtime(status) = result.expect("runtime status application result")
    else {
        panic!("canic_status returned a non-Runtime response")
    };
    *status
}

fn intent_cleanup_status(
    pic: &PocketIc,
    canister_id: Principal,
) -> canic::dto::runtime::CanisterTimerStatus {
    runtime_status(pic, canister_id)
        .timers
        .into_iter()
        .find(|timer| timer.subsystem == "intent_cleanup" && timer.name == "run")
        .expect("intent cleanup runtime status")
}

fn begin_intent(
    pic: &PocketIc,
    canister_id: Principal,
    resource_seed: u8,
    ttl_secs: Option<u64>,
) -> Result<u64, canic::Error> {
    pic.update_candid(
        canister_id,
        "begin_timer_probe_intent",
        (resource_seed, ttl_secs),
    )
    .expect("call intent reservation endpoint")
}

fn counts(pic: &PocketIc, canister_id: Principal) -> (u64, u64, u64) {
    let result: Result<(u64, u64, u64), canic::Error> = pic
        .query_candid(canister_id, "timer_probe_counts", ())
        .expect("query timer probe counts");
    result.expect("timer probe counts application result")
}

fn assert_application_timer_rows(pic: &PocketIc, canister_id: Principal) {
    let status = runtime_status(pic, canister_id);
    for (owner, subsystem, name) in [
        ("companion-framework", "inventory", "visible"),
        ("runtime-probe", "application", "timer-interval"),
    ] {
        assert!(
            status.timers.iter().any(|timer| {
                timer.owner == owner && timer.subsystem == subsystem && timer.name == name
            }),
            "missing synchronously reconstructed native timer {owner}:{subsystem}:{name}"
        );
    }
}

fn timer_metrics(pic: &PocketIc, canister_id: Principal) -> Vec<MetricEntry> {
    let response: Result<RoleStatusResponse, Error> = pic
        .query_candid(
            canister_id,
            protocol::CANIC_STATUS,
            (RoleStatusRequest::Metrics(MetricsStatusRequest {
                kind: MetricsKind::Runtime,
                page: PageRequest {
                    limit: 256,
                    offset: 0,
                },
            }),),
        )
        .expect("query runtime metrics");

    let RoleStatusResponse::Metrics(page) = response.expect("runtime metrics application result")
    else {
        panic!("canic_status returned a non-Metrics response")
    };
    page.entries
}

fn timer_metric(entries: &[MetricEntry], label: &str) -> (u64, u64) {
    let (subsystem, name) = label.split_once(':').expect("fixed Canic timer label");
    entries
        .iter()
        .find_map(|entry| {
            (entry.labels == ["perf", "timer", "canic", subsystem, name]).then(|| {
                match entry.value {
                    MetricValue::CountAndU64 { count, value_u64 } => (count, value_u64),
                    MetricValue::Count(_) | MetricValue::U128(_) => {
                        panic!("timer performance metric must carry count and instructions")
                    }
                }
            })
        })
        .unwrap_or_default()
}

fn tick(pic: &PocketIc, count: usize) {
    for _ in 0..count {
        pic.tick();
    }
}

fn wait_for_fixture_ready(pic: &PocketIc, canister_id: Principal, context: &str) {
    pic.wait_for_ready(
        canister_id,
        Principal::anonymous(),
        READY_TICK_LIMIT,
        context,
    );
}

fn install_retry_policy() -> RetryPolicy {
    RetryPolicy::try_new(INSTALL_CODE_RETRY_LIMIT, INSTALL_CODE_COOLDOWN)
        .expect("install retry policy")
}
