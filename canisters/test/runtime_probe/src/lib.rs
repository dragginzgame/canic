#![expect(clippy::unused_async)]

use canic::{
    Error,
    api::{
        call::Call,
        intent::{BeginLocalIntentInput, IntentResourceKey, LocalIntentApi},
    },
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
    static ASYNC_RECOVERY_PROBE_CONTINUATIONS: Cell<u64> = const { Cell::new(0) };
    static ASYNC_RECOVERY_PROBE_OPERATION_IDS: RefCell<Vec<[u8; 32]>> = const { RefCell::new(Vec::new()) };
    static COMPANION_TIMER: RefCell<Option<ic_timers::OnceRegistration>> = const { RefCell::new(None) };
}

canic::start_local!();

/// Run no-op setup for the runtime probe.
async fn canic_setup() {}

/// Schedule timers used by runtime macro coverage tests.
async fn canic_install(_: Option<Vec<u8>>) {
    canic::__internal::core::api::timer::TimerApi::register_async_recovery_participant(
        dispatch_trapping_async_recovery_probe,
    );
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

/// Start one real trapped continuation for the pre-armed recovery watchdog.
#[canic_update(public)]
async fn begin_trapped_async_recovery_probe() -> Result<(), Error> {
    use canic::__internal::core::control_plane_support::ops::{
        async_recovery::{AsyncRecoveryOwner, AsyncTimerRecoveryOps},
        ic::IcOps,
    };

    ASYNC_RECOVERY_PROBE_CONTINUATIONS.set(0);
    ASYNC_RECOVERY_PROBE_OPERATION_IDS.with_borrow_mut(Vec::clear);
    let now_ns = IcOps::now_nanos();
    AsyncTimerRecoveryOps::activate_recovery(AsyncRecoveryOwner::CanisterPoolMaintenance, now_ns);
    canic::__internal::core::api::timer::TimerApi::ensure_async_recovery_watchdog_required();
    Ok(())
}

/// Return committed continuations and whether the trapped lease was cleared.
#[canic_query(public)]
fn trapped_async_recovery_probe_status() -> Result<(u64, bool, Vec<[u8; 32]>), Error> {
    use canic::__internal::core::control_plane_support::ops::{
        async_recovery::{AsyncRecoveryOwner, AsyncTimerRecoveryOps},
        ic::IcOps,
    };

    Ok((
        ASYNC_RECOVERY_PROBE_CONTINUATIONS.get(),
        AsyncTimerRecoveryOps::expired_deadline(
            AsyncRecoveryOwner::CanisterPoolMaintenance,
            IcOps::now_nanos(),
        )
        .is_none()
            && AsyncTimerRecoveryOps::recovery_due(
                AsyncRecoveryOwner::CanisterPoolMaintenance,
                IcOps::now_nanos(),
            )
            .is_none(),
        ASYNC_RECOVERY_PROBE_OPERATION_IDS.with_borrow(Clone::clone),
    ))
}

/// Commit one independently observable self-call before the first continuation traps.
#[canic_update(public)]
fn advance_async_recovery_probe() -> Result<u64, Error> {
    let count = ASYNC_RECOVERY_PROBE_CONTINUATIONS.get().saturating_add(1);
    ASYNC_RECOVERY_PROBE_CONTINUATIONS.set(count);
    Ok(count)
}

fn dispatch_trapping_async_recovery_probe() -> bool {
    use canic::__internal::core::control_plane_support::ops::{
        async_recovery::{
            AsyncRecoveryClaim, AsyncRecoveryCompletion, AsyncRecoveryOwner, AsyncTimerRecoveryOps,
        },
        ic::IcOps,
    };

    let now_ns = IcOps::now_nanos();
    let Some(lease_expires_at_ns) = now_ns.checked_add(1_000_000_000) else {
        return false;
    };
    let attempt = match AsyncTimerRecoveryOps::claim(
        AsyncRecoveryOwner::CanisterPoolMaintenance,
        now_ns,
        lease_expires_at_ns,
    ) {
        Ok(AsyncRecoveryClaim::Acquired(attempt)) => attempt,
        Ok(AsyncRecoveryClaim::Busy { .. }) | Err(_) => return false,
    };
    ASYNC_RECOVERY_PROBE_OPERATION_IDS
        .with_borrow_mut(|operation_ids| operation_ids.push(attempt.operation_id().into_bytes()));
    ic_cdk::futures::spawn(async move {
        let canister_id = IcOps::canister_self();
        let count = Call::unbounded_wait(canister_id, "advance_async_recovery_probe")
            .with_arg(())
            .expect("encode async recovery probe call")
            .execute_candid::<Result<u64, Error>>()
            .await
            .expect("execute async recovery probe call")
            .expect("advance async recovery probe");
        if count == 1 {
            ic_cdk::trap("intentional async recovery continuation trap");
        }
        let _finished =
            AsyncTimerRecoveryOps::finish(attempt, AsyncRecoveryCompletion::Success, None);
    });
    true
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

canic::finish!();
