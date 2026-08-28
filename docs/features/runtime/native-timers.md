# Native Timer Adoption

Canic and application code share one canister-local `ic-timers` runtime, but
they do not share timer ownership. Application crates declare, schedule,
cancel, reconstruct and observe their own native registrations. Canic provides
the lifecycle composition point; it does not provide an application timer
facade.

The maintained end-to-end fixture is
[`runtime_probe`](../../../canisters/test/runtime_probe/src/lib.rs). It owns a
one-shot, an after-completion registration, cancellation, native inventory and
synchronous reconstruction without a Canic timer wrapper.

## Pin One Provider

Every timer-owning crate linked into a canister must resolve the same exact
package identity:

```toml
[dependencies]
ic-timers = "=0.7.0"
```

Check the composed graph, not only each direct manifest:

```text
cargo tree -d
cargo tree -i ic-timers@0.7.0
```

Two resolved versions contain two independent sets of library statics and
therefore two inventories. Do not combine a direct `ic-cdk-timers` consumer
with this design without separately inventorying and qualifying that second
provider path.

## Replace the Removed Canic Facade

The pre-0.104 `canic::timer!`, `canic::timer_interval!`, public `TimerHandle`
and `canic::api::timer` application operations have no aliases. Replace them
with native declarations whose identity names the actual owner rather than
`canic`:

```rust
use ic_timers::{
    DeclarationLifetime, OnceRegistration, TimerCompletion, TimerDirective,
    TimerIdentity, TimerRunResult, TimerSchedule, register_once,
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static REFRESH_TIMER: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) };
}

fn declare_refresh_timer() -> Result<(), Box<dyn std::error::Error>> {
    let registration = register_once(
        TimerIdentity::try_new("my-app", "cache", "refresh")?,
        DeclarationLifetime::RemoveWhenStopped,
        |_context| async {
            refresh_cache().await;
            TimerRunResult::new(TimerCompletion::success(1), TimerDirective::Stop)
        },
    )?;
    registration.ensure_scheduled(TimerSchedule::After(Duration::from_secs(30)))?;
    REFRESH_TIMER.with_borrow_mut(|current| *current = Some(registration));
    Ok(())
}
```

`Once` callbacks return `TimerRunResult`. An after-completion callback uses
`TimerDirective::RecurAfterCompletion` when normal completion should schedule
the next cadence:

```rust
use ic_timers::{
    AfterCompletionRegistration, DeclarationLifetime, TimerCadence,
    TimerCompletion, TimerDirective, TimerIdentity, TimerRunResult,
    register_after_completion,
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static SWEEP_TIMER: RefCell<Option<AfterCompletionRegistration>> = const { RefCell::new(None) };
}

fn declare_sweep_timer() -> Result<(), Box<dyn std::error::Error>> {
    let registration = register_after_completion(
        TimerIdentity::try_new("my-app", "maintenance", "sweep")?,
        TimerCadence::new(Duration::from_secs(60))?,
        DeclarationLifetime::Retained,
        |_context| async {
            let processed = sweep_one_batch().await;
            TimerRunResult::new(
                TimerCompletion::success(processed),
                TimerDirective::RecurAfterCompletion,
            )
        },
    )?;
    registration.ensure_scheduled()?;
    SWEEP_TIMER.with_borrow_mut(|current| *current = Some(registration));
    Ok(())
}
```

Use the policy that matches the failure boundary:

- `Once` runs asynchronous work once and schedules a successor only when the
  callback or owner requests one.
- `AfterCompletion` schedules its successor only after normal asynchronous
  completion. It does not catch up missed fixed-rate ticks.
- `Watchdog` pre-arms a successor in a separate scheduler message before
  synchronous work. Use it only when that stronger recovery protocol is
  required.

`ic-timers 0.7` gives Watchdog lifecycle reconstruction its own
`WatchdogReconcileState`. Use `ScheduledImmediately`,
`ensure_scheduled_immediately()` or `WatchdogDecision::ContinueImmediately`
only when durable authority proves actionable work should continue without a
cadence delay. The immediate scheduler and its work still execute as later
replicated messages, and the request replaces the one pre-armed successor
rather than adding another handle. Retain cadence `Scheduled`/`Continue` for
quiescent polling, retry delay or a bounded pass that has already dispatched
all currently actionable work.

## Keep Custody Semantics Explicit

The returned registration is a non-clone owner capability:

- keep it in volatile owner state when later control is required;
- `cancel()` clears scheduled work but does not interrupt work already
  running;
- cancellation leaves a `Retained` declaration inactive, while
  `RemoveWhenStopped` removes the declaration when the transition completes;
- `unregister(self)` consumes custody and removes callback authority; and
- dropping the Rust value only detaches the caller's control capability. It
  does not cancel or unregister the timer.

Provider inventory is volatile.
Provider inventory is not durable business demand.
Provider inventory is not an application recovery record.
`timer_inventory()` and `has_armed_wakeup()` are observations, not durable
authority.
When authoritative application state requires a wake-up, reconcile or ensure
it directly instead of using a check-then-schedule decision.

## Reconstruct From Durable Demand

The provider registry is volatile. Do not persist provider handles,
registrations, generations, epochs, snapshots, retry streaks, scheduling
commands or copied provider deadlines. Persist only the application fact that
creates demand—for example, the next durable job deadline or an unsettled
operation identity—and synchronously derive the desired native schedule from
that fact.

Fixed declarations should use the native reconciliation helpers during both
`init` and `post_upgrade`:

```rust
use ic_timers::{
    OnceRegistration, TimerCompletion, TimerDirective, TimerIdentity,
    TimerRunResult, TimerSchedule, reconcile_once,
};
use std::cell::RefCell;

thread_local! {
    static EXPIRY_TIMER: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) };
}

fn reconstruct_application_timers() -> Result<(), Box<dyn std::error::Error>> {
    let identity = TimerIdentity::try_new("my-app", "jobs", "expiry")?;
    let desired = durable_next_expiry_ns().map(TimerSchedule::At);
    EXPIRY_TIMER.with_borrow_mut(|registration| {
        reconcile_once(registration, &identity, desired, |_context| async {
            let processed = expire_due_jobs().await;
            TimerRunResult::new(
                TimerCompletion::success(processed),
                next_expiry_directive(),
            )
        })
    })?;
    Ok(())
}
```

`reconcile_once`, `reconcile_after_completion` and `reconcile_watchdog`
construct retained declarations on a fresh heap and then reconcile their
desired state. Use direct registration for transient `RemoveWhenStopped`
work.

When a callback can trigger a paid or externally mutating effect, persist the
application operation or receipt identity before the effect. A response lost
after the effect must retry that same operation identity; a provider callback
generation or `RetryAfter` directive is not an application idempotency key and
must not be copied into a generic recovery record.

## Compose Synchronous Lifecycle Work

Canonical managed and Root canisters declare one paired participant:

```rust
canic::start!(
    lifecycle_participant(
        init = crate::lifecycle::after_canic_init,
        post_upgrade = crate::lifecycle::after_canic_post_upgrade,
    ),
);
```

Both paths must be safe functions with the exact type `fn() -> ()`. Closures,
async or unsafe functions, arguments, return values, partial pairs and
duplicate pairs are rejected at compile time. `start_local!` accepts the same
development-only declaration. `start_wasm_store!` and
`start_fleet_coordinator!` do not.

Canic invokes the matching function exactly once after it has initialized the
shared provider and restored its synchronous invariants and native claims, but
before it schedules bootstrap or deferred application hooks. Managed
Prepared/inactive canisters still run the participant during init and
post-upgrade. Application fan-out belongs inside the named function:

```rust
pub fn after_canic_post_upgrade() {
    reconstruct_application_timers()
        .unwrap_or_else(|error| ic_cdk::trap(format!("timer reconstruction failed: {error}")));
    reconstruct_database_runtime()
        .unwrap_or_else(|error| ic_cdk::trap(format!("database reconstruction failed: {error}")));
}
```

The participant is synchronous: it must not await, spawn an untracked future
or defer required reconstruction. A trap aborts the enclosing install or
upgrade message. A failed upgrade retains the previously committed Wasm and
state; retrying after correcting the application-owned cause reruns the whole
lifecycle message.

The declaration adds no lifecycle export, Candid method, stable record or
readiness claim. The canister still has exactly one `init` and one
`post_upgrade` export, both owned by Canic.

## Qualification Checklist

Before shipping a composed canister, prove:

- one resolved `ic-timers` package identity and no unintended raw provider;
- application identities use an application/framework owner label, never
  `canic`;
- native inventory contains each expected owner after init and same-release
  upgrade;
- synchronous reconstruction precedes deferred setup/install/upgrade work;
- cancellation, removal and detached-custody behavior match the selected
  lifetime;
- callback progress and durable retry/idempotency are tested independently;
- participant failure rolls back install/upgrade and a corrected retry
  succeeds; and
- the composed Wasm retains one lifecycle export pair and its intended Candid
  surface.
