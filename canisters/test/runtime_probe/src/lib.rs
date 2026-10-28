#![expect(clippy::unused_async)]

mod composition;
mod history_cost;
mod perf_context;
mod process_fixture;
mod timer_wire;

use canic::{
    Error,
    api::intent::{BeginLocalIntentInput, IntentResourceKey, LocalIntentApi},
    prelude::*,
};
use std::{
    cell::{Cell, RefCell},
    time::Duration,
};

use composition::{__icydb_facade, __icydb_host};

canic_icydb::__icydb_start_participant_lifecycle!(canic::memory::runtime::shared_runtime);

thread_local! {
    static REJECT_APPLICATION_SAMPLE: Cell<bool> = const { Cell::new(false) };
    static TIMER_ONCE_EXECUTIONS: Cell<u64> = const { Cell::new(0) };
    static TIMER_INTERVAL_EXECUTIONS: Cell<u64> = const { Cell::new(0) };
    static TIMER_CANCELLED_EXECUTIONS: Cell<u64> = const { Cell::new(0) };
    static LIFECYCLE_INIT_EXECUTIONS: Cell<u64> = const { Cell::new(0) };
    static LIFECYCLE_POST_UPGRADE_EXECUTIONS: Cell<u64> = const { Cell::new(0) };
    static COMPANION_TIMER: RefCell<Option<ic_timers::OnceRegistration>> = const { RefCell::new(None) };
    static APPLICATION_INTERVAL_TIMER: RefCell<Option<ic_timers::AfterCompletionRegistration>> = const { RefCell::new(None) };
    static CAPACITY_TIMERS: RefCell<Vec<ic_timers::OnceRegistration>> = const { RefCell::new(Vec::new()) };
    static COST_WATCHDOG: RefCell<Option<ic_timers::WatchdogRegistration>> = const { RefCell::new(None) };
    static TRAP_COST_WATCHDOG: Cell<bool> = const { Cell::new(false) };
}

canic::start_local!(lifecycle_participant(
    init = __icydb_lifecycle_participant::init,
    post_upgrade = __icydb_lifecycle_participant::post_upgrade,
),);

/// Run no-op setup for the runtime probe.
async fn canic_setup() {
    assert_application_runtime_reconstructed();
}

/// Record the deferred application install hook.
async fn canic_install(_: Option<Vec<u8>>) {
    assert_application_runtime_reconstructed();
}

fn lifecycle_participant_init() {
    assert_eq!(
        (
            LIFECYCLE_INIT_EXECUTIONS.get(),
            LIFECYCLE_POST_UPGRADE_EXECUTIONS.get()
        ),
        (0, 0),
        "the runtime-probe init participant must run exactly once"
    );
    LIFECYCLE_INIT_EXECUTIONS.set(LIFECYCLE_INIT_EXECUTIONS.get().saturating_add(1));
    reconstruct_application_timers();
}

fn lifecycle_participant_post_upgrade() {
    assert_eq!(
        (
            LIFECYCLE_INIT_EXECUTIONS.get(),
            LIFECYCLE_POST_UPGRADE_EXECUTIONS.get()
        ),
        (0, 0),
        "the runtime-probe post-upgrade participant must run exactly once on the fresh heap"
    );
    if option_env!("CANIC_TEST_LIFECYCLE_PARTICIPANT_TRAP").is_some() {
        ic_platform::cdk::api::trap("runtime-probe lifecycle participant requested a test trap");
    }
    LIFECYCLE_POST_UPGRADE_EXECUTIONS
        .set(LIFECYCLE_POST_UPGRADE_EXECUTIONS.get().saturating_add(1));
    reconstruct_application_timers();
}

/// Reconstruct application-owned native registrations from application demand.
fn reconstruct_application_timers() {
    COMPANION_TIMER.with_borrow_mut(|registration| {
        if registration.is_none() {
            let identity =
                ic_timers::TimerIdentity::try_new("companion-framework", "inventory", "visible")
                    .expect("companion timer identity");
            *registration = Some(
                ic_timers::register_once(
                    identity,
                    ic_timers::TimerStart::Retained { initial: None },
                    |_context: ic_timers::OnceContext| async {
                        ic_timers::OnceRunResult::new(
                            ic_timers::TimerCompletion::no_work(),
                            ic_timers::OnceDecision::Stop,
                        )
                    },
                )
                .expect("register companion framework timer"),
            );
        }
    });

    let one_shot = ic_timers::register_once(
        application_timer_identity("timer-once"),
        ic_timers::TimerStart::RemoveWhenStopped {
            initial: ic_timers::TimerSchedule::After(Duration::from_secs(5)),
        },
        |_context: ic_timers::OnceContext| async {
            timer_once().await;
            ic_timers::OnceRunResult::new(
                ic_timers::TimerCompletion::success(1),
                ic_timers::OnceDecision::Stop,
            )
        },
    )
    .expect("register native application one-shot");
    drop(one_shot);

    APPLICATION_INTERVAL_TIMER.with_borrow_mut(|current| {
        let interval = ic_timers::register_after_completion(
            application_timer_identity("timer-interval"),
            ic_timers::TimerCadence::new(Duration::from_secs(10))
                .expect("valid native application cadence"),
            ic_timers::TimerStart::RemoveWhenStopped {
                initial: ic_timers::TimerSchedule::After(Duration::from_secs(10)),
            },
            |_context: ic_timers::AfterCompletionContext| async {
                timer_interval().await;
                ic_timers::AfterCompletionRunResult::new(
                    ic_timers::TimerCompletion::success(1),
                    ic_timers::AfterCompletionDecision::RecurAfterCompletion,
                )
            },
        )
        .expect("register native application interval");
        *current = Some(interval);
    });

    let cancelled = ic_timers::register_once(
        application_timer_identity("timer-cancelled"),
        ic_timers::TimerStart::RemoveWhenStopped {
            initial: ic_timers::TimerSchedule::After(Duration::from_secs(5)),
        },
        |_context: ic_timers::OnceContext| async {
            timer_cancelled().await;
            ic_timers::OnceRunResult::new(
                ic_timers::TimerCompletion::success(1),
                ic_timers::OnceDecision::Stop,
            )
        },
    )
    .expect("register cancellable native application timer");
    cancelled.cancel().expect("cancel native application timer");
}

fn assert_application_runtime_reconstructed() {
    assert_eq!(
        LIFECYCLE_INIT_EXECUTIONS
            .get()
            .saturating_add(LIFECYCLE_POST_UPGRADE_EXECUTIONS.get()),
        1,
        "exactly one matching lifecycle participant must precede deferred hooks"
    );
    let inventory = ic_timers::timer_inventory()
        .expect("the shared timer inventory must exist before deferred hooks");
    assert!(inventory.timers().iter().any(|timer| {
        let identity = timer.identity();
        identity.owner() == "runtime-probe"
            && identity.subsystem() == "application"
            && identity.name() == "timer-interval"
    }));
}

/// Record the deferred application upgrade hook.
async fn canic_upgrade() {
    assert_application_runtime_reconstructed();
}

#[update(public, wire(id = crate::timer_wire::TEST, request = crate::timer_wire::REQUEST_LIMITS, response = crate::timer_wire::RESPONSE_LIMITS))]
async fn test() -> Result<(), Error> {
    Ok(())
}

/// Observe committed memory ownership after standalone cold restoration.
#[query(requires(caller::is_controller()), wire(id = crate::timer_wire::MEMORY, request = crate::timer_wire::REQUEST_LIMITS, response = canic::protocol::protected_memory_allocations::RESPONSE_LIMITS))]
fn memory_allocations_probe() -> Result<canic::dto::memory::MemoryAllocationsResponse, Error> {
    canic::__internal::core::api::memory::MemoryQuery::allocations()
}

/// Reserve one test resource so PocketIC can exercise expiry scheduling and recovery.
#[update(public, wire(id = crate::timer_wire::BEGIN_INTENT, request = crate::timer_wire::INTENT_REQUEST_LIMITS, response = crate::timer_wire::RESPONSE_LIMITS))]
async fn begin_timer_probe_intent(request: timer_wire::IntentRequest) -> Result<u64, Error> {
    let timer_wire::IntentRequest {
        resource_seed,
        ttl_secs,
    } = request;
    let resource_key = IntentResourceKey::try_new(format!("timer_probe:{resource_seed}"))
        .map_err(|_| Error::from_registered(canic::diagnostics::codes::REQUEST_INVALID))?;
    LocalIntentApi::begin(BeginLocalIntentInput {
        resource_key,
        quantity: 1,
        ttl_secs,
        reservation_limit: Some(1),
    })
    .map(|intent_id| intent_id.0)
}

/// Exercise distinct scheduler and work measurement envelopes in the existing timer fixture.
#[update(public, wire(id = crate::timer_wire::SCHEDULE_WATCHDOG, request = crate::timer_wire::REQUEST_LIMITS, response = crate::timer_wire::RESPONSE_LIMITS))]
async fn schedule_cost_probe_watchdog() -> Result<(), Error> {
    COST_WATCHDOG.with_borrow_mut(|current| {
        let registration = current.get_or_insert_with(|| {
            ic_timers::register_watchdog(
                application_timer_identity("cost-watchdog"),
                ic_timers::TimerCadence::new(Duration::from_secs(10)).expect("cost probe cadence"),
                ic_timers::TimerStart::Retained { initial: None },
                |_context| {
                    if TRAP_COST_WATCHDOG.get() {
                        ic_platform::cdk::api::trap("cost watchdog interruption probe");
                    }
                    ic_timers::WatchdogRunResult::new(
                        ic_timers::TimerCompletion::success(1),
                        ic_timers::WatchdogDecision::Stop,
                    )
                },
            )
            .expect("register cost probe watchdog")
        });
        registration
            .ensure_scheduled_immediately()
            .expect("schedule cost probe watchdog");
    });
    Ok(())
}

/// Exercise cancellation and replacement without adding a second timer owner.
#[update(public, wire(id = crate::timer_wire::CANCEL_WATCHDOG, request = crate::timer_wire::REQUEST_LIMITS, response = crate::timer_wire::RESPONSE_LIMITS))]
async fn cancel_cost_probe_watchdog(unregister: bool) -> Result<(), Error> {
    COST_WATCHDOG.with_borrow_mut(|current| {
        if unregister {
            current
                .take()
                .expect("cost probe registration")
                .unregister()
                .expect("unregister cost probe");
        } else {
            current
                .as_ref()
                .expect("cost probe registration")
                .cancel()
                .expect("cancel cost probe");
        }
    });
    Ok(())
}

/// Select a real IC callback trap to verify that no completed sample is fabricated.
#[update(public, wire(id = crate::timer_wire::SET_WATCHDOG_TRAP, request = crate::timer_wire::REQUEST_LIMITS, response = crate::timer_wire::RESPONSE_LIMITS))]
async fn set_cost_probe_trap(enabled: bool) -> Result<(), Error> {
    TRAP_COST_WATCHDOG.set(enabled);
    Ok(())
}

/// Fill the shared timer registry until its typed capacity boundary rejects demand.
#[update(public, wire(id = crate::timer_wire::FILL_REGISTRY, request = crate::timer_wire::REQUEST_LIMITS, response = crate::timer_wire::RESPONSE_LIMITS))]
async fn fill_timer_registry() -> Result<(u64, bool), Error> {
    let mut registered = 0u64;
    let mut claims = Vec::new();
    for index in 0..100u64 {
        let identity = ic_timers::TimerIdentity::try_new(
            "runtime-probe",
            "capacity",
            format!("timer-{index}"),
        )
        .expect("valid capacity timer identity");
        if let Ok(registration) = ic_timers::register_once(
            identity,
            ic_timers::TimerStart::RemoveWhenStopped {
                initial: ic_timers::TimerSchedule::After(Duration::from_hours(24)),
            },
            |_context: ic_timers::OnceContext| async {
                ic_timers::OnceRunResult::new(
                    ic_timers::TimerCompletion::no_work(),
                    ic_timers::OnceDecision::Stop,
                )
            },
        ) {
            claims.push(registration);
            registered = registered.saturating_add(1);
        } else {
            CAPACITY_TIMERS.with_borrow_mut(|current| current.extend(claims));
            return Ok((registered, true));
        }
    }
    CAPACITY_TIMERS.with_borrow_mut(|current| current.extend(claims));
    Ok((registered, false))
}

/// Attempt one invalid identity so tests can prove registration is leak-free.
#[update(public, wire(id = crate::timer_wire::REJECT_INVALID_IDENTITY, request = crate::timer_wire::REQUEST_LIMITS, response = crate::timer_wire::RESPONSE_LIMITS))]
async fn reject_invalid_timer_identity() -> Result<bool, Error> {
    Ok(ic_timers::TimerIdentity::try_new(
        "runtime-probe",
        "capacity",
        "x".repeat(ic_timers::MAX_TIMER_IDENTITY_COMPONENT_BYTES + 1),
    )
    .is_err())
}

#[query(public, wire(id = crate::timer_wire::COUNTS, request = crate::timer_wire::REQUEST_LIMITS, response = crate::timer_wire::RESPONSE_LIMITS))]
fn timer_probe_counts() -> Result<(u64, u64, u64), Error> {
    Ok((
        TIMER_ONCE_EXECUTIONS.get(),
        TIMER_INTERVAL_EXECUTIONS.get(),
        TIMER_CANCELLED_EXECUTIONS.get(),
    ))
}

async fn timer_once() {
    TIMER_ONCE_EXECUTIONS.set(TIMER_ONCE_EXECUTIONS.get().saturating_add(1));
}

async fn timer_interval() {
    TIMER_INTERVAL_EXECUTIONS.set(TIMER_INTERVAL_EXECUTIONS.get().saturating_add(1));
}

async fn timer_cancelled() {
    TIMER_CANCELLED_EXECUTIONS.set(TIMER_CANCELLED_EXECUTIONS.get().saturating_add(1));
}

fn application_timer_identity(name: &str) -> ic_timers::TimerIdentity {
    ic_timers::TimerIdentity::try_new("runtime-probe", "application", name)
        .expect("valid native application timer identity")
}

/// Publish bounded synthetic aggregates through the trusted local API.
#[update(requires(caller::is_controller()), wire(id = crate::timer_wire::SAMPLE_METRICS, request = crate::timer_wire::REQUEST_LIMITS, response = crate::timer_wire::RESPONSE_LIMITS))]
async fn sample_public_metrics() -> Result<(), canic::Error> {
    canic::api::public_status::PublicStatusApi::sample_metrics()?;
    canic::api::public_status::PublicStatusApi::record_application_metrics(vec![
        canic::dto::public_status::PublicMetric {
            name: "assigned_items".to_string(),
            canister_id: Some(ic_platform::cdk::api::canister_self()),
            value: 7,
            unit: "assignments".to_string(),
            observed_at_ns: ic_platform::cdk::api::time(),
            kind: canic::dto::public_status::PublicMetricKind::Gauge,
        },
    ])
}

/// Bounded fixture evidence for optional sampling and ordinary cycle tracking.
#[derive(serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
struct PublicSamplingProbe {
    sample_instructions: u64,
    sample: Result<(), canic::Error>,
    cycle_tracking: Result<(), canic::Error>,
    allocation_unchanged: bool,
    allocation_ids_measured: u64,
}

/// Exercise production sampling after a bounded amount of synthetic instrumentation.
#[update(requires(caller::is_controller()), wire(id = crate::timer_wire::QUALIFY_SAMPLING, request = crate::timer_wire::REQUEST_LIMITS, response = crate::timer_wire::RESPONSE_LIMITS))]
async fn qualify_public_metrics_sampling(
    request: timer_wire::SamplingRequest,
) -> Result<PublicSamplingProbe, canic::Error> {
    let timer_wire::SamplingRequest {
        checkpoints,
        reject_family,
    } = request;
    for index in 0..checkpoints.min(4096) {
        canic::__internal::core::perf::record_checkpoint(
            "zz_sample",
            &format!("checkpoint_{index:04}"),
            7,
        );
    }
    if reject_family {
        // Checkpoint names are admitted into overflow; exercise endpoint projection rejection.
        canic::__internal::core::perf::record(
            canic::__internal::core::perf::PerfKey::Endpoint {
                kind: canic::ids::EndpointCallKind::Update,
                name: "a".repeat(128),
            },
            7,
        );
    }
    process_fixture::record();
    let before = canic::__internal::core::api::memory::MemoryQuery::allocations()?;
    let start = ic_platform::cdk::api::performance_counter(0);
    let sample = canic::api::public_status::PublicStatusApi::sample_metrics();
    let sample_instructions = ic_platform::cdk::api::performance_counter(0).saturating_sub(start);
    let after = canic::__internal::core::api::memory::MemoryQuery::allocations()?;
    let allocation_unchanged = before == after;
    let allocation_ids_measured = after.memories.len() as u64;
    // This internal fixture qualifies the existing cycle owner without exposing a new App API.
    let cycle_tracking =
        canic::__internal::core::api::runtime::root_funding::RootFundingTimerApi::reconcile();
    Ok(PublicSamplingProbe {
        sample_instructions,
        sample,
        cycle_tracking,
        allocation_unchanged,
        allocation_ids_measured,
    })
}

/// Select a bounded local measurement participant for the sampling fixture.
#[update(requires(caller::is_controller()), wire(id = crate::timer_wire::CONFIGURE_SAMPLER, request = crate::timer_wire::REQUEST_LIMITS, response = crate::timer_wire::RESPONSE_LIMITS))]
async fn configure_public_sampler(reject: bool) -> Result<(), Error> {
    REJECT_APPLICATION_SAMPLE.set(reject);
    canic::api::public_status::PublicStatusApi::set_application_sampler(Some(
        canic::api::public_status::ApplicationMetricsSampler::new(sample_application_counters),
    ));
    Ok(())
}

fn sample_application_counters() -> Result<Vec<canic::dto::public_status::PublicMetric>, Error> {
    if REJECT_APPLICATION_SAMPLE.get() {
        return Err(Error::from_registered(
            canic::diagnostics::codes::REQUEST_INVALID,
        ));
    }
    Ok(vec![canic::dto::public_status::PublicMetric {
        name: "timer_completions".into(),
        canister_id: Some(ic_platform::cdk::api::canister_self()),
        value: TIMER_INTERVAL_EXECUTIONS.get().into(),
        unit: "count".into(),
        observed_at_ns: ic_platform::cdk::api::time(),
        kind: canic::dto::public_status::PublicMetricKind::Counter {
            window_id: 0,
            saturated: TIMER_INTERVAL_EXECUTIONS.get() == u64::MAX,
        },
    }])
}

/// Exercise the existing volatile suspension fence without creating a timer owner.
#[update(requires(caller::is_controller()), wire(id = crate::timer_wire::SUSPEND_SAMPLER, request = crate::timer_wire::REQUEST_LIMITS, response = crate::timer_wire::RESPONSE_LIMITS))]
async fn suspend_public_sampler_fixture() -> Result<(), Error> {
    canic::__internal::core::api::timer::TimerApi::restore_snapshot_suspension(true);
    Ok(())
}

/// Controlled input for matched history-layout cost measurements.
#[update(requires(caller::is_controller()), wire(id = crate::timer_wire::HISTORY_SAMPLING_COST, request = crate::timer_wire::REQUEST_LIMITS, response = crate::timer_wire::RESPONSE_LIMITS))]
async fn qualify_history_sampling_cost(rows: u16) -> Result<u64, Error> {
    history_cost::sample(rows)
}

/// Read costs are measured in their own query message, without sampling first.
#[query(requires(caller::is_controller()), wire(id = crate::timer_wire::HISTORY_QUERY_COST, request = crate::timer_wire::REQUEST_LIMITS, response = crate::timer_wire::RESPONSE_LIMITS))]
async fn qualify_history_query_cost(limit: u64) -> Result<history_cost::HistoryCost, Error> {
    Ok(history_cost::history(limit))
}

canic::finish!();
