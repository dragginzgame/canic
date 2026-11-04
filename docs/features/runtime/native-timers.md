# Native Timer Adoption

Canic and application code share one canister-local `ic-timers` runtime, but
they do not share timer ownership. Application crates declare, schedule,
cancel, reconstruct and observe their own native registrations. Canic provides
the lifecycle composition point; it does not provide an application timer
facade.

Timers owns callback execution, scheduling transitions and native registration
state. Canic owns durable business demand, paid-operation reconciliation, lifecycle
reconstruction and the authority fences that permit work. Those records remain
necessary even when a callback's execution is entirely delegated to Timers.

The maintained end-to-end fixture is
[`runtime_probe`](../../../canisters/test/runtime_probe/src/lib.rs). It owns a
one-shot, an after-completion registration, cancellation, native inventory and
synchronous reconstruction without a Canic timer wrapper.

## Use One Provider

Every timer-owning crate linked into a canister must resolve the same exact
package identity. In this workspace, inherit the selected provider from the
[root dependency catalog](../../../Cargo.toml):

```toml
[dependencies]
ic-timers.workspace = true
```

An independent application owns its root catalog and must select a requirement
compatible with its chosen Canic release. Canic's locked deployed graph must
contain one provider identity. Compatible
version requirements are supported; an exact manifest pin is not required.
Check the composed graph, not only each direct manifest:

```text
cargo tree --locked --target wasm32-unknown-unknown -d
cargo tree --locked --target wasm32-unknown-unknown -i ic-timers
```

Run these commands with the same package and feature selection as deployment.
If the inverse lookup is ambiguous, use the exact `ic-timers@VERSION` identities
reported by Cargo to inspect both paths and align them before deployment.
Recheck the graph when dependencies change. Two resolved versions contain two independent
sets of library statics and
therefore two inventories. Do not combine a direct `ic-cdk-timers` consumer
with this design without separately inventorying and qualifying that second
provider path.

Applications that compose other timer owners must qualify their complete selected
graph and shared lifecycle in their own repository. Canic's fixture suite qualifies
Canic's timer custody without depending on a database consumer.

## Declare an Application Timer

Use a native registration whose identity names the application owner. Retain
its handle in application state so that the application can observe or cancel
the registration:

```rust
use ic_timers::{
    OnceDecision, OnceRegistration, OnceRunResult, TimerCompletion,
    TimerIdentity, TimerSchedule, TimerStart, register_once,
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static REFRESH_TIMER: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) };
}

fn declare_refresh_timer() -> Result<(), Box<dyn std::error::Error>> {
    let registration = register_once(
        TimerIdentity::try_new("my-app", "cache", "refresh")?,
        TimerStart::RemoveWhenStopped {
            initial: TimerSchedule::After(Duration::from_secs(30)),
        },
        |_context| async {
            refresh_cache().await;
            OnceRunResult::new(TimerCompletion::success(1), OnceDecision::Stop)
        },
    )?;
    REFRESH_TIMER.with_borrow_mut(|current| *current = Some(registration));
    Ok(())
}
```

`Once` callbacks return `OnceRunResult` with a `OnceDecision`. After-completion
callbacks return `AfterCompletionRunResult` with an `AfterCompletionDecision`, using
`AfterCompletionDecision::RecurAfterCompletion` when normal completion should schedule
the next cadence:

```rust
use ic_timers::{
    AfterCompletionDecision, AfterCompletionRegistration, AfterCompletionRunResult,
    TimerCadence, TimerCompletion, TimerIdentity, TimerSchedule, TimerStart,
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
        TimerStart::Retained {
            initial: Some(TimerSchedule::After(Duration::from_secs(60))),
        },
        |_context| async {
            let processed = sweep_one_batch().await;
            AfterCompletionRunResult::new(
                TimerCompletion::success(processed),
                AfterCompletionDecision::RecurAfterCompletion,
            )
        },
    )?;
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

`ic-timers` gives Watchdog lifecycle reconstruction its own
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

`TimerStart` binds declaration lifetime and the initial schedule in the same
registration. `RemoveWhenStopped` requires a schedule; `Retained` permits
`initial: None` when durable demand is currently absent. Registration succeeds
only after the provider has accepted the requested initial wake-up. Use native
reconciliation for retained declarations that lifecycle work may revisit.

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
    OnceDecision, OnceRegistration, OnceRunResult, TimerCompletion,
    TimerIdentity, TimerSchedule, reconcile_once,
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
            OnceRunResult::new(
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

Canic's retained cleanup, retention, sampling, renewal, funding, acknowledgement
and application-startup owners use `reconcile_once` directly. Native reconciliation
publishes an empty owner slot only after initial admission succeeds. Existing
registrations retain their callback authority. Domain requests that may advance
a deadline without postponing an earlier safety check use native
`ensure_scheduled` on that retained registration.

IC Timers owns registration, wakeups, callback progress and cancellation. Canic
owns its domain demand, durable attempt identities, exact handle custody for
snapshot suspension and thin lifecycle deferral. Every role initializes the same
shared runtime before its domain owners reconstruct demand.

When a callback can trigger a paid or externally mutating effect, persist the
application operation or receipt identity before the effect. A response lost
after the effect must retry that same operation identity; a provider callback
generation or `RetryAfter` directive is not an application idempotency key and
must not be copied into a generic recovery record.

## Recover Abandoned Callbacks

Finite local reservations admit their cleanup deadline before allocating an intent
ID or writing reservation state. Cost permits admit the same shared owner once,
before creating their quota and cycle intents; both use the same validated expiry
deadline. There is no await between schedule admission and reservation, so the
callback cannot run between these steps. Scheduling refusal leaves reservations
unchanged. TTL-free reservations need no timer admission. An existing retained
cleanup declaration can still schedule when the native registry is otherwise full.

Canic retains durable business attempts for cycle top-up, placement
acknowledgements, issuer renewal and pool maintenance. Its recovery watchdog
rearms the exact native owner after confirmed callback abandonment. The native
callback then claims or resumes the business attempt and returns its actual
retry, deadline or stop decision to Timers. Recovery does not spawn a separate
copy of the workflow or discard its completion directive.

Coordinator provisioning retains one native registration for its current journals,
including scale-out. Admission, applied funding rotation and requested Root removal
share `canic/coordinator_transition/advance`, with fair selection between retained
operations. Both owners share Coordinator's native recovery watchdog. Staging a
funding plan or reserving Root removal does not request autonomous execution.
Root-removal acceptance retains execution demand before remote calls; replay shares
that exact operation even while Root is unavailable. Snapshot preparation refuses
pending work, releases idle owner/watchdog claims, then applies the existing empty-
inventory and running-callback fences. The Coordinator post-upgrade hook restores
the durable authority fence before rebuilding owners from accepted journals; sealed
authority schedules no work and retains its history-bound resume checks. Installed
provisioning and admission reconstruction, live snapshot resume, and sealed upgrade
refusal pass. Coordinator retries use the retained business-attempt windows described
below. An unchanged exact publication before Root runtime activation starts is a wait,
not a new failure; the original issued intent remains authoritative.

Root admission and Registry synchronization share `canic/root_transition/advance`.
Exact command replay selects existing durable demand without allocating another
chain. Root upgrade and its shared watchdog reconstruct that owner, while live
callbacks keep native custody. Completion removes the declaration; terminal
synchronization replay does not leave a claim behind.

Root aggregate provisioning holds one native `OnceRegistration` under
`canic/root_component_provisioning/advance`. Its callback reads the current
durable operation and returns its next deadline or stop decision. Exact replay
requests a successor on that claim; terminal replay clears obsolete demand.
The existing Root watchdog also covers prepared provisioning, preserves live
callbacks and re-arms recoverable demand. Normal completion removes this
transient declaration so it does not obstruct a later authority snapshot.

Ordinary and peer allocations retain their continuation request in the existing
allocation record. One `canic/component_allocation/advance` registration rotates
between unfinished operations. Exact replay reuses its claim; synchronous Root
restoration and the shared watchdog reconstruct it without enrolling aggregate
Component-group work. Completed allocations no longer create demand. Exact ordinary
and peer reservation replay validates retained caller and authority evidence before
any Store dependency call, so an unavailable Store cannot obscure accepted work.

Root child allocations retain autonomous continuation demand in their existing
operation records. One native `canic/component_child_allocation/advance` owner
advances one eligible child step per callback, rotating between ready children.
Root reconstruction and the shared watchdog recover this owner from retained
demand. Persisted deadlines, permanent refusals and the consecutive-failure
budget still determine eligibility; reconstruction does not reset them. Completed
operations create no timer demand, and synchronous caller-owned work is not
implicitly enrolled for autonomous continuation.

Standalone Component and subtree removals share the native
`canic/component_retirement/advance` owner. Their existing journals retain whether
an autonomous continuation was requested; admission alone does not enroll nested
work. A draining Component owns its subtree cursor, and Root removal owns its
nested Component work. Reconstruction and watchdog recovery select only requested,
unfinished work. Exact replay reuses the claim and terminal work leaves no demand.
The retry windows below preserve these journals and their reserved storage capacity.

Coordinator admission, applied funding rotation, provisioning and requested Root
removal, plus Root ordinary/peer allocation and standalone Component/subtree removal,
retain a maximum of 16 attempts without observed business progress. Attempts are
charged before dispatch, with exponential delays capped at 60 seconds. Waiting does
not invent a diagnostic; changing errors, command replay, abandonment and upgrade
cannot replenish the budget. Proved authority/data conflicts suspend for review.
Only observed business progress or exact controller resumption renews the window.

Controllers query `canic_operation_retry_status` with the exact operation selector,
then submit `canic_operation_retry_resume` with its observed generation. Replaying
that generation cannot renew twice. Resume refuses a running native owner and changes
neither paid-effect reconciliation nor issued authority. Retry state describes the
attempt window independently of the business operation's phase. Exhausted pending
work still prevents authority snapshot sealing even after its timer stops.

Root removal retains `canic/root_removal/advance` for its current durable journal.
Exact replay reuses that claim, and same-release reconstruction resumes removal
without re-enabling proactive pool maintenance. The shared Root watchdog recovers
missing or abandoned work only after Timers reports no live callback. Removal
journals still own effect reconciliation. This whole-Root driver retains its own
retry policy, independently of the Coordinator and standalone retirement windows.

Child-allocation retry budgets belong to the retained business operation. A new
diagnostic does not reset consecutive failures; only actual work progress or
completion does. Repeated observations of the same remote Root failure preserve
the Coordinator's original observation without another journal write or warning.
A fresh failed attempt and changed remote evidence remain distinct observations.

An expired business lease alone does not authorize replacing a native callback
that is still scheduled or running. A pending IC response can outlive that
lease; recovery leaves the callback and its original paid-operation identity
intact. Explicit cancellation, invariant failure and provider control failure
also remain distinct from recovery demand. Snapshot suspension gates recovery
until the authority owner explicitly resumes.

For `TimerStart::Retained`, normal demand becoming empty leaves the ordinary
registration idle and keeps its declaration. Transient `RemoveWhenStopped`
owners, including Root allocation and retirement continuations, remove their
declaration when they stop; new durable demand recreates it under the same
identity. Clearing normal demand does not count as explicit cancellation.
Prior explicit cancellation remains disabled until the owner rearms it. Runtime
`next_due_at_ns` describes current demand; a metric named for the last armed
delay is historical evidence and may remain after the current deadline clears.

## Keep Snapshot Custody Atomic

Root snapshot preparation observes IC history while its producers retain their
normal demand. After that await, it rechecks quiescence and suspends both Core
and Control Plane timer owners in the same callback that commits the durable
seal. A busy refusal leaves producers intact; a failure after suspension starts
traps so suspension and sealing roll back together.

Resume reconstructs both producer sets while the fence remains sealed, then
opens mutation in that same callback. An exact completed resume replay returns
its stored receipt without observing history or rebuilding timers. Repeated
prepare on the matching seal retains the same receipt and suspended demand.
These are same-release interruption and replay contracts.

## Compose Synchronous Lifecycle Work

Application-owned managed canisters declare one paired participant:

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
development-only declaration. Canonical Root, Store and Coordinator canisters
have no application lifecycle participant; `start_fleet_root!`,
`start_wasm_store!` and `start_fleet_coordinator!` accept no such declaration.

Canic invokes the matching function exactly once after it has initialized the
shared provider and restored its synchronous invariants and native claims, but
before it schedules bootstrap or deferred application hooks. Managed
Prepared/inactive canisters still run the participant during init and
post-upgrade. Store activation/upgrade and standalone-local lifecycle adapters
schedule framework bootstrap independently before the optional asynchronous
`init` block. That block gates only application setup and phase hooks; a held
or trapping user future must not prevent framework bootstrap from running.
Non-root bootstrap retains one native registration throughout its bounded
64-attempt authority retry window. Callbacks return retry decisions to Timers;
they do not create successor registrations. A new same-release lifecycle rebuilds
the bootstrap owner after synchronous restoration. Store GC reports failed
collection as a failed timer completion; Root retains authority to retry the
exact collection operation.
Application fan-out belongs inside the named function:

```rust
pub fn after_canic_post_upgrade() {
    reconstruct_application_timers()
        .unwrap_or_else(|error| ic_platform::cdk::api::trap(format!("timer reconstruction failed: {error}")));
    reconstruct_database_runtime()
        .unwrap_or_else(|error| ic_platform::cdk::api::trap(format!("database reconstruction failed: {error}")));
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

## Observe Through Canic

Timer registration and control use the local Rust API. Canic's controller-only
Wire `canic_runtime` query projects the shared inventory, including application
and companion-library owners, into runtime health and performance records.
Enabled public Operations and Performance families expose bounded cached
measurements through Wire `canic_public_metrics` and `canic_public_history`.
No separately mounted timer endpoint is required.

The installed timer fixture uses bounded native Wire requests for its timer
controls and sampling probes. Its callers exercise initialization and repeated
same-release restoration, detached custody, cancellation, recurrence, capacity,
malformed ingress and public measurement continuity. These fixture controls
are qualification surfaces, not application timer APIs.

## Qualification Checklist

Before shipping a composed canister, prove:

- one resolved `ic-timers` package identity and no unintended raw provider;
- one `ic-platform-runtime` execution owner and its matching exact Runtime guard
  across the selected canister's normal Wasm graph;
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
- the composed Wasm retains one lifecycle export pair, its native Wire methods
  and any required external protocol methods.

## Continue From Here

- [Review runtime observability](public-observability.md)
- [Browse runtime features](README.md)
- [Browse all documentation](../../README.md)
- [Back to the main README](../../../README.md)
