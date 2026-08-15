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
    static COMPANION_TIMER: RefCell<Option<ic_timers::OnceRegistration>> = const { RefCell::new(None) };
}

canic::start_local!();

/// Run no-op setup for the runtime probe.
async fn canic_setup() {}

/// Schedule timers used by runtime macro coverage tests.
async fn canic_install(_: Option<Vec<u8>>) {
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

    // Schedule perf-instrumented timers to ensure timer macros are covered.
    timer!(Duration::from_secs(5), timer_once)
        .expect("schedule one-shot timer")
        .detach();
    timer_interval!(Duration::from_secs(10), timer_interval)
        .expect("schedule interval timer")
        .detach();
    let cancelled =
        timer!(Duration::from_secs(5), timer_cancelled).expect("schedule cancellable timer");
    canic::api::timer::cancel(cancelled).expect("cancel timer");
}

/// Run no-op upgrade handling for the runtime probe.
async fn canic_upgrade() {}

#[canic_update(public)]
async fn test() -> Result<(), Error> {
    Ok(())
}

/// Reserve one test resource so PocketIC can exercise expiry scheduling and recovery.
#[canic_update(public)]
async fn begin_timer_probe_intent(resource_seed: u8, ttl_secs: Option<u64>) -> Result<u64, Error> {
    let resource_key = IntentResourceKey::try_new(format!("timer_probe:{resource_seed}"))
        .map_err(|err| Error::invalid(err.to_string()))?;
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
    for index in 0..100u64 {
        match canic::__internal::core::api::timer::TimerApi::set(
            Duration::from_hours(24),
            format!("capacity-{index}"),
            async {},
        ) {
            Ok(handle) => {
                handle.detach();
                registered = registered.saturating_add(1);
            }
            Err(_) => return Ok((registered, true)),
        }
    }
    Ok((registered, false))
}

/// Attempt one invalid identity so tests can prove registration is leak-free.
#[canic_update(public)]
async fn reject_invalid_timer_identity() -> Result<bool, Error> {
    Ok(canic::__internal::core::api::timer::TimerApi::set(
        Duration::from_hours(24),
        "x".repeat(ic_timers::MAX_TIMER_IDENTITY_COMPONENT_BYTES + 1),
        async {},
    )
    .is_err())
}

#[canic_update(requires(auth::authenticated(cap::VERIFY)))]
async fn test_verify_delegated_token(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;
    if let Err(err) = canic::access::env::build_network_local() {
        return Err(Error::forbidden(err.to_string()));
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

canic::finish!();
