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

#[derive(CandidType)]
enum PublicStatusRequest {
    Health,
    Metrics(canic::dto::public_status::PublicMetricsRequest),
    History(canic::dto::public_status::PublicHistoryRequest),
}

#[derive(CandidType, Deserialize)]
enum PublicStatusResponse {
    Health(canic::dto::public_status::PublicHealth),
    Metrics(canic::dto::public_status::PublicMetricsSnapshot),
    History(canic::dto::public_status::PublicHistorySnapshot),
}

#[test]
fn public_snapshots_preserve_observer_authority() {
    use canic::dto::public_status::{
        PublicMetricFamily, PublicMetricsRequest, PublicSnapshotState,
    };
    let fixture = install_lifecycle_boundary_fixture();
    let published = fixture.install_runtime_probe_canister();
    let restricted = fixture.install_canic_canister();
    let outsider = fixture.root;
    let query = |canister_id, family| {
        let response: Result<PublicStatusResponse, Error> = fixture.pic.query_candid_as_or_panic(
            canister_id,
            outsider,
            protocol::CANIC_PUBLIC_STATUS,
            (PublicStatusRequest::Metrics(PublicMetricsRequest {
                family,
                page: PageRequest {
                    limit: 256,
                    offset: 0,
                },
            }),),
        );
        let PublicStatusResponse::Metrics(snapshot) = response.expect("public snapshot read")
        else {
            panic!("expected metrics snapshot");
        };
        snapshot
    };
    let health: Result<PublicStatusResponse, Error> = fixture.pic.query_candid_as_or_panic(
        published,
        outsider,
        protocol::CANIC_PUBLIC_STATUS,
        (PublicStatusRequest::Health,),
    );
    let PublicStatusResponse::Health(health) = health.expect("public health") else {
        panic!("expected summary health");
    };
    assert_eq!(health.canister_id, published);
    let disabled = query(restricted, PublicMetricFamily::Cycles);
    assert_eq!(disabled.state, PublicSnapshotState::Disabled);
    assert!(disabled.metrics.entries.is_empty());
    assert_eq!(
        query(published, PublicMetricFamily::Application).state,
        PublicSnapshotState::Unavailable
    );
    let sampled: Result<(), Error> = fixture
        .pic
        .update_candid(published, "sample_public_metrics", ())
        .expect("trusted sampling transport");
    sampled.expect("trusted sampling");
    let snapshot = query(published, PublicMetricFamily::Application);
    assert_eq!(snapshot.state, PublicSnapshotState::Fresh);
    assert_eq!(snapshot.metrics.entries[0].value, 7);
    assert_eq!(
        query(published, PublicMetricFamily::Operations).state,
        PublicSnapshotState::Fresh
    );
    assert_anonymous_public_boundary(&fixture.pic, published, fixture.root);
    let cycles = query(published, PublicMetricFamily::Cycles);
    assert_eq!(cycles.state, PublicSnapshotState::Fresh);
    assert_eq!(cycles.metrics.entries[0].unit, "cycles");
    for request in sensitive_observability_requests(false) {
        let denied: Result<RoleStatusResponse, Error> = fixture.pic.query_candid_as_or_panic(
            published,
            outsider,
            protocol::CANIC_OBSERVABILITY,
            (request,),
        );
        assert!(
            matches!(denied, Err(error) if error.code() == canic::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code())
        );
    }
    fixture.pic.advance_time(Duration::from_secs(361));
    fixture.pic.tick();
    let stale = query(published, PublicMetricFamily::Application);
    assert_eq!(stale.state, PublicSnapshotState::Stale);
    assert_eq!(stale.sampled_at_ns, snapshot.sampled_at_ns);
    assert_eq!(stale.metrics.entries, snapshot.metrics.entries);
}

#[derive(CandidType, Deserialize)]
struct PublicSamplingProbe {
    sample_instructions: u64,
    sample: Result<(), Error>,
    cycle_tracking: Result<(), Error>,
}

#[test]
fn optional_sampling_is_bounded_and_rejected_performance_preserves_cycle_tracking() {
    use canic::dto::public_status::{
        PublicMetricFamily, PublicMetricsRequest, PublicSnapshotState,
    };
    let fixture = install_lifecycle_boundary_fixture();
    let canister = fixture.install_runtime_probe_canister();
    let query = |family| {
        let response: Result<PublicStatusResponse, Error> = fixture.pic.query_candid_as_or_panic(
            canister,
            fixture.root,
            protocol::CANIC_PUBLIC_STATUS,
            (PublicStatusRequest::Metrics(PublicMetricsRequest {
                family,
                page: PageRequest {
                    limit: 1000,
                    offset: 0,
                },
            }),),
        );
        let PublicStatusResponse::Metrics(snapshot) = response.expect("public cache") else {
            panic!("metrics response")
        };
        snapshot
    };
    let baseline: Result<PublicSamplingProbe, Error> = fixture
        .pic
        .update_candid(
            canister,
            "qualify_public_metrics_sampling",
            (256_u16, false),
        )
        .unwrap();
    let baseline = baseline.expect("authorized fixture sampling");
    baseline.sample.unwrap();
    baseline.cycle_tracking.unwrap();
    let full: Result<PublicSamplingProbe, Error> = fixture
        .pic
        .update_candid(
            canister,
            "qualify_public_metrics_sampling",
            (4096_u16, false),
        )
        .unwrap();
    let full = full.expect("authorized fixture sampling");
    full.sample.unwrap();
    full.cycle_tracking.unwrap();
    assert_explicit_sampling_cost(baseline.sample_instructions, full.sample_instructions);
    assert_periodic_sampling_cost(&fixture.pic, canister);
    let operations = query(PublicMetricFamily::Operations);
    let performance = query(PublicMetricFamily::Performance);
    assert_public_process_projection(&operations, &performance);
    assert!(performance.truncated);
    assert_eq!(performance.metrics.entries.len(), 256);
    fixture.pic.advance_time(Duration::from_secs(361));
    let rejected: Result<PublicSamplingProbe, Error> = fixture
        .pic
        .update_candid(canister, "qualify_public_metrics_sampling", (0_u16, true))
        .unwrap();
    let rejected = rejected.expect("authorized fixture sampling");
    assert!(
        matches!(rejected.sample, Err(error) if error.code() == canic::diagnostics::codes::REQUEST_INVALID.raw_code())
    );
    rejected
        .cycle_tracking
        .expect("optional family rejection must preserve cycle tracking");
    let operations_after_failure = query(PublicMetricFamily::Operations);
    assert_eq!(operations_after_failure.state, PublicSnapshotState::Fresh);
    assert!(operations_after_failure.sampled_at_ns > operations.sampled_at_ns);
    let retained = query(PublicMetricFamily::Performance);
    assert_eq!(retained.sampled_at_ns, performance.sampled_at_ns);
    assert_eq!(retained.metrics.entries, performance.metrics.entries);
    assert_eq!(retained.state, PublicSnapshotState::Stale);
    let occupancy = query(PublicMetricFamily::ShardOccupancy);
    assert_eq!(occupancy.state, PublicSnapshotState::Fresh);
    assert!(occupancy.sampled_at_ns > performance.sampled_at_ns);
    let history: Result<RoleStatusResponse, Error> = fixture
        .pic
        .query_candid(
            canister,
            protocol::CANIC_OBSERVABILITY,
            (RoleStatusRequest::CycleHistory(PageRequest {
                limit: 100,
                offset: 0,
            }),),
        )
        .unwrap();
    let RoleStatusResponse::CycleHistory(history) = history.unwrap() else {
        panic!("cycle history response")
    };
    assert!(
        history
            .entries
            .iter()
            .any(|entry| entry.timestamp_secs > performance.sampled_at_ns.unwrap() / 1_000_000_000)
    );
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
                protocol::CANIC_OBSERVABILITY,
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
                .query_candid(
                    canister_id,
                    protocol::CANIC_OBSERVABILITY,
                    (request.clone(),),
                )
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
            protocol::CANIC_OBSERVABILITY,
            (RoleStatusRequest::Runtime,),
        )
        .expect("query runtime status");
    let RoleStatusResponse::Runtime(status) = result.expect("runtime status application result")
    else {
        panic!("canic_observability returned a non-Runtime response")
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
            protocol::CANIC_OBSERVABILITY,
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
        panic!("canic_observability returned a non-Metrics response")
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

#[test]
fn public_history_samples_without_readers_and_resets_after_restoration() {
    use canic::dto::public_status::PublicSnapshotState;
    let fixture = install_lifecycle_boundary_fixture();
    let canister = fixture.install_runtime_probe_canister();
    wait_for_fixture_ready(&fixture.pic, canister, "install");
    let history = || cached_cycle_history(&fixture.pic, canister, fixture.root);
    let sampler_count = |canister| {
        runtime_status(&fixture.pic, canister)
            .timers
            .iter()
            .filter(|timer| {
                timer.owner == "canic"
                    && timer.subsystem == "public_metrics"
                    && timer.name == "sample"
            })
            .count()
    };
    assert_eq!(sampler_count(canister), 1);
    let restricted = fixture.install_canic_canister();
    assert_eq!(sampler_count(restricted), 0);
    fixture
        .pic
        .update_candid::<Result<(), Error>, _>(canister, "configure_public_sampler", (false,))
        .unwrap()
        .unwrap();
    let application = || cached_application_metrics(&fixture.pic, canister, fixture.root);
    let before = history();
    fixture.pic.advance_time(Duration::from_secs(301));
    tick(&fixture.pic, 8);
    let first = history();
    assert_eq!(first.state, PublicSnapshotState::Fresh);
    assert!(!first.points.entries.is_empty());
    assert!(first.points.entries.len() <= 288);
    let supplied = application();
    assert_eq!(supplied.state, PublicSnapshotState::Fresh);
    assert_eq!(supplied.metrics.entries[0].name, "timer_completions");
    fixture
        .pic
        .update_candid::<Result<(), Error>, _>(canister, "configure_public_sampler", (true,))
        .unwrap()
        .unwrap();
    let repeated = history();
    assert_eq!(first.points.entries, repeated.points.entries);
    assert_eq!(first.reserved_bytes, repeated.reserved_bytes);
    // Several missed slots produce one new observation, without a catch-up backlog.
    fixture.pic.advance_time(Duration::from_mins(25));
    tick(&fixture.pic, 8);
    let delayed = history();
    assert_eq!(delayed.points.entries.len(), first.points.entries.len() + 1);
    let last = delayed.points.entries.last().unwrap();
    let prior = &delayed.points.entries[delayed.points.entries.len() - 2];
    assert!(last.slot_start_ns - prior.slot_start_ns >= 5 * delayed.cadence_ns);
    assert!(delayed.reserved_bytes <= delayed.byte_limit);
    let rejected = application();
    assert_eq!(rejected.sampled_at_ns, supplied.sampled_at_ns);
    assert_eq!(rejected.metrics.entries, supplied.metrics.entries);
    assert_eq!(rejected.state, PublicSnapshotState::Stale);
    fixture
        .pic
        .update_candid::<Result<(), Error>, _>(canister, "suspend_public_sampler_fixture", ())
        .unwrap()
        .unwrap();
    fixture.pic.advance_time(Duration::from_secs(601));
    tick(&fixture.pic, 8);
    let suspended = history();
    assert_eq!(suspended.points.entries, delayed.points.entries);
    assert_eq!(suspended.state, PublicSnapshotState::Stale);
    assert!(delayed.canister_version >= before.canister_version);
    fixture
        .pic
        .retry_install_code(install_retry_policy(), || {
            fixture.pic.upgrade_canister(
                canister,
                fixture.runtime_probe_wasm.clone(),
                upgrade_args(),
                None,
            )
        })
        .expect("same-release restoration");
    let restored = history();
    assert!(restored.canister_version > delayed.canister_version);
    assert_eq!(restored.state, PublicSnapshotState::Unavailable);
    assert!(restored.points.entries.is_empty());
    wait_for_fixture_ready(&fixture.pic, canister, "post_upgrade");
    fixture.pic.advance_time(Duration::from_secs(301));
    tick(&fixture.pic, 8);
    let resumed = history();
    assert_eq!(resumed.points.entries.len(), 1);
    assert_eq!(resumed.state, PublicSnapshotState::Fresh);
    assert_eq!(sampler_count(canister), 1);
}

fn cached_cycle_history(
    pic: &PocketIc,
    canister: Principal,
    caller: Principal,
) -> canic::dto::public_status::PublicHistorySnapshot {
    use canic::dto::public_status::{PublicHistoryRequest, PublicMetricFamily};

    let response: Result<PublicStatusResponse, Error> = pic.query_candid_as_or_panic(
        canister,
        caller,
        protocol::CANIC_PUBLIC_STATUS,
        (PublicStatusRequest::History(PublicHistoryRequest {
            family: PublicMetricFamily::Cycles,
            name: "balance".into(),
            canister_id: Some(canister),
            page: PageRequest {
                offset: 0,
                limit: u64::MAX,
            },
        }),),
    );
    let PublicStatusResponse::History(history) = response.unwrap() else {
        panic!("history response")
    };
    history
}

fn cached_application_metrics(
    pic: &PocketIc,
    canister: Principal,
    caller: Principal,
) -> canic::dto::public_status::PublicMetricsSnapshot {
    use canic::dto::public_status::PublicMetricFamily;

    let response: Result<PublicStatusResponse, Error> = pic.query_candid_as_or_panic(
        canister,
        caller,
        protocol::CANIC_PUBLIC_STATUS,
        (PublicStatusRequest::Metrics(
            canic::dto::public_status::PublicMetricsRequest {
                family: PublicMetricFamily::Application,
                page: PageRequest {
                    offset: 0,
                    limit: 256,
                },
            },
        ),),
    );
    let PublicStatusResponse::Metrics(snapshot) = response.unwrap() else {
        panic!("application snapshot")
    };
    snapshot
}

fn assert_periodic_sampling_cost(pic: &PocketIc, canister: Principal) {
    pic.advance_time(Duration::from_secs(301));
    tick(pic, 8);
    let status = runtime_status(pic, canister);
    let sampler = status
        .timers
        .iter()
        .find(|timer| timer.subsystem == "public_metrics" && timer.name == "sample")
        .expect("public sampling timer");
    let instructions = sampler
        .work_performance
        .instructions_maximum
        .expect("scheduled sample completed");
    println!("periodic public sampling maximum instructions: {instructions}");
    assert!(
        instructions < 20_000_000,
        "bounded scheduled sampling instruction budget"
    );
}

fn assert_explicit_sampling_cost(baseline: u64, full: u64) {
    println!("public sampling instructions: 256 checkpoints={baseline}, 4096 checkpoints={full}");
    assert!(
        baseline < 20_000_000,
        "bounded first-sample allocation instruction budget"
    );
    assert!(
        full < 20_000_000,
        "bounded public sampling instruction budget"
    );
    assert!(
        full <= baseline.saturating_mul(2),
        "sampling cost must remain bounded beyond the retained-series ceiling"
    );
}

fn assert_anonymous_public_boundary(pic: &PocketIc, published: Principal, controller: Principal) {
    pic.set_controllers(published, None, vec![controller])
        .unwrap();
    let anonymous_public: Result<PublicStatusResponse, Error> = pic.query_candid_as_or_panic(
        published,
        Principal::anonymous(),
        protocol::CANIC_PUBLIC_STATUS,
        (PublicStatusRequest::Health,),
    );
    assert!(anonymous_public.is_ok());
    for request in sensitive_observability_requests(false) {
        let denied: Result<RoleStatusResponse, Error> = pic.query_candid_as_or_panic(
            published,
            Principal::anonymous(),
            protocol::CANIC_OBSERVABILITY,
            (request,),
        );
        assert!(
            matches!(denied, Err(error) if error.code() == canic::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code())
        );
    }
    pic.set_controllers(published, Some(controller), vec![Principal::anonymous()])
        .unwrap();
}

fn assert_public_process_projection(
    operations: &canic::dto::public_status::PublicMetricsSnapshot,
    performance: &canic::dto::public_status::PublicMetricsSnapshot,
) {
    for name in [
        "process.inter_canister_call.started",
        "process.wasm_store.completed",
        "timer.events.schedule_requests",
        "timer.events.retryable_failure",
        "timer.events.invariant_failure",
    ] {
        let row = operations
            .metrics
            .entries
            .iter()
            .find(|row| row.name == name)
            .expect("bounded public process aggregate");
        assert_eq!(row.unit, "count");
        assert_eq!(row.canister_id, None);
        assert!(row.observed_at_ns > 0);
    }
    for name in ["memory.wasm_extent", "memory.stable_extent"] {
        let row = performance
            .metrics
            .entries
            .iter()
            .find(|row| row.name == name)
            .expect("public memory extent");
        assert_eq!(row.unit, "bytes");
        assert_eq!(row.kind, canic::dto::public_status::PublicMetricKind::Gauge);
    }
}
