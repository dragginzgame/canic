#![expect(clippy::unused_async)]

use canic::{
    Error,
    api::intent::{BeginLocalIntentInput, IntentResourceKey, LocalIntentApi},
    dto::auth::DelegatedToken,
    ids::cap,
    prelude::*,
};
use std::{
    cell::{Cell, RefCell},
    time::Duration,
};

thread_local! {
    static TIMER_ONCE_EXECUTIONS: Cell<u64> = const { Cell::new(0) };
    static TIMER_INTERVAL_EXECUTIONS: Cell<u64> = const { Cell::new(0) };
    static TIMER_CANCELLED_EXECUTIONS: Cell<u64> = const { Cell::new(0) };
    static LIFECYCLE_INIT_EXECUTIONS: Cell<u64> = const { Cell::new(0) };
    static LIFECYCLE_POST_UPGRADE_EXECUTIONS: Cell<u64> = const { Cell::new(0) };
    static COMPANION_TIMER: RefCell<Option<ic_timers::OnceRegistration>> = const { RefCell::new(None) };
    static APPLICATION_INTERVAL_TIMER: RefCell<Option<ic_timers::AfterCompletionRegistration>> = const { RefCell::new(None) };
    static CAPACITY_TIMERS: RefCell<Vec<ic_timers::OnceRegistration>> = const { RefCell::new(Vec::new()) };
}

canic::start_local!(lifecycle_participant(
    init = lifecycle_participant_init,
    post_upgrade = lifecycle_participant_post_upgrade,
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
        ic_cdk::trap("runtime-probe lifecycle participant requested a test trap");
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
                    ic_timers::DeclarationLifetime::Retained,
                    |_context: ic_timers::OnceContext| async {
                        ic_timers::TimerRunResult::new(
                            ic_timers::TimerCompletion::no_work(),
                            ic_timers::TimerDirective::Stop,
                        )
                    },
                )
                .expect("register companion framework timer"),
            );
        }
    });

    let one_shot = ic_timers::register_once(
        application_timer_identity("timer-once"),
        ic_timers::DeclarationLifetime::RemoveWhenStopped,
        |_context: ic_timers::OnceContext| async {
            timer_once().await;
            ic_timers::TimerRunResult::new(
                ic_timers::TimerCompletion::success(1),
                ic_timers::TimerDirective::Stop,
            )
        },
    )
    .expect("register native application one-shot");
    one_shot
        .ensure_scheduled(ic_timers::TimerSchedule::After(Duration::from_secs(5)))
        .expect("schedule native application one-shot");
    drop(one_shot);

    APPLICATION_INTERVAL_TIMER.with_borrow_mut(|current| {
        let interval = ic_timers::register_after_completion(
            application_timer_identity("timer-interval"),
            ic_timers::TimerCadence::new(Duration::from_secs(10))
                .expect("valid native application cadence"),
            ic_timers::DeclarationLifetime::RemoveWhenStopped,
            |_context: ic_timers::AfterCompletionContext| async {
                timer_interval().await;
                ic_timers::TimerRunResult::new(
                    ic_timers::TimerCompletion::success(1),
                    ic_timers::TimerDirective::RecurAfterCompletion,
                )
            },
        )
        .expect("register native application interval");
        interval
            .ensure_scheduled()
            .expect("schedule native application interval");
        *current = Some(interval);
    });

    let cancelled = ic_timers::register_once(
        application_timer_identity("timer-cancelled"),
        ic_timers::DeclarationLifetime::RemoveWhenStopped,
        |_context: ic_timers::OnceContext| async {
            timer_cancelled().await;
            ic_timers::TimerRunResult::new(
                ic_timers::TimerCompletion::success(1),
                ic_timers::TimerDirective::Stop,
            )
        },
    )
    .expect("register cancellable native application timer");
    cancelled
        .ensure_scheduled(ic_timers::TimerSchedule::After(Duration::from_secs(5)))
        .expect("schedule cancellable native application timer");
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

#[canic_update(public)]
async fn test() -> Result<(), Error> {
    Ok(())
}

/// Reserve one test resource so PocketIC can exercise expiry scheduling and recovery.
#[canic_update(public)]
async fn begin_timer_probe_intent(resource_seed: u8, ttl_secs: Option<u64>) -> Result<u64, Error> {
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

/// Fill the shared timer registry until its typed capacity boundary rejects demand.
#[canic_update(public)]
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
            ic_timers::DeclarationLifetime::RemoveWhenStopped,
            |_context: ic_timers::OnceContext| async {
                ic_timers::TimerRunResult::new(
                    ic_timers::TimerCompletion::no_work(),
                    ic_timers::TimerDirective::Stop,
                )
            },
        ) {
            registration
                .ensure_scheduled(ic_timers::TimerSchedule::After(Duration::from_hours(24)))
                .expect("schedule capacity timer");
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
#[canic_update(public)]
async fn reject_invalid_timer_identity() -> Result<bool, Error> {
    Ok(ic_timers::TimerIdentity::try_new(
        "runtime-probe",
        "capacity",
        "x".repeat(ic_timers::MAX_TIMER_IDENTITY_COMPONENT_BYTES + 1),
    )
    .is_err())
}

#[canic_update(requires(auth::authenticated(cap::VERIFY)))]
async fn test_verify_delegated_token(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;
    if canic::access::env::build_network_local().is_err() {
        return Err(Error::from_registered(
            canic::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }

    Ok(())
}

#[canic_query(public)]
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

canic::finish!();
