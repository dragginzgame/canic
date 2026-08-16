# Idea: Synchronous Lifecycle Participation And Combined Shared-Timer Evidence

Date: 2026-08-14
Last amended: 2026-08-16

## Status

- Classification: deferred, unnumbered idea. Its former working number was
  `0.104`; retained batch and approval language is historical design context,
  not current scheduling or implementation authority.
- Former review status: proposed design authority; B1 inventory and evidence
  could begin; mutating implementation remained held.
- Release boundary: reinstall only across Canic releases. Same-release
  interruption recovery and exact retry remain required.
- Scheduling posture: this idea does not gate the numbered 0.103-0.108 Fleet
  path or standalone 0.109 authorization qualification. Promotion requires a
  current release position, accepted B1 contract and explicit maintainer
  authorization.
- Production dependency posture: this is a Canic-owned seam. Production Canic
  imports no IcyDB type, lifecycle registry, timer provider or framework-
  specific runtime authority.
- Qualification posture: when promoted, Canic owns both a generic participant
  proof and one Canic-repository combined Canic+IcyDB test canister. The latter
  is test-only composition evidence, not a production framework dependency or
  authority to modify IcyDB.

## Purpose

`canic::start!` owns the IC `init` and `post_upgrade` exports for a managed
Canic canister. Canic restores its synchronous invariants before scheduling
bootstrap and application work through zero-delay timers.

That ordering is correct for Canic, but the current optional user block is
deferred. It cannot host another runtime whose safety requires synchronous
reconstruction before the lifecycle message returns. A second framework macro
cannot independently export `init` or `post_upgrade` without creating duplicate
lifecycle ownership.

The idea adds one narrow application-owned composition seam:

```text
Canic restores its local invariants synchronously
  -> one application-owned synchronous phase function runs
  -> Canic may schedule bootstrap and deferred application work
  -> the one lifecycle message returns
```

The application function may invoke another framework's private lifecycle
participant and other synchronous reconcilers. Canic neither knows nor names
those participants.

## External Contract Input And Repository Boundary

The current external ownership split is:

| Repository | Required change |
| --- | --- |
| `ic-timers` | None. Atomic inventory, owner attribution and runtime epoch already exist. |
| IcyDB | None. `icydb::start!(participant)` already supplies synchronous `fn() -> ()` lifecycle functions. |
| Canic | Add the framework-neutral lifecycle-participant seam and combined shared-timer evidence. |

The external IcyDB `0.227-framework-neutral-lifecycle-participation` design,
reviewed at its lines 180-201 and 248-258 on 2026-08-16, explicitly assigns
single-owner Canic composition downstream and prohibits a Canic dependency or
Canic-specific type inside IcyDB. Those files and the `ic-timers` repository
remain read-only from Canic work. After Canic's combined proof lands, their
only expected fallout is truthful documentation/status propagation in their
own repositories by their maintainers.

## Current Baseline To Freeze In B1

B1 must re-read the maintained source, macro expansions, tests and generated
artifacts rather than treating this design summary as current-code evidence.

The former 0.101.53 baseline recorded four lifecycle macro families:

| Surface | Current lifecycle owner | Current synchronous/deferred boundary | Proposed disposition |
| --- | --- | --- | --- |
| `canic::start!` managed non-root | Canic `init` and `post_upgrade` | synchronous restore; activation-gated zero-delay bootstrap and application work | required production participant surface |
| root-selected `canic::start!` | Canic control-plane lifecycle | synchronous restore; activation-gated root bootstrap and application work | required production participant surface |
| `canic::start_local!` local non-root | Canic `init` and `post_upgrade` | synchronous restore followed by zero-delay bootstrap and application work | useful development surface after canonical `start!` proof |
| `canic::start_wasm_store!` | Canic Wasm Store lifecycle | synchronous restore; activation-gated infrastructure work | inventory only |
| `canic::start_fleet_coordinator!` | dedicated Coordinator init | no generic application lifecycle participant | inventory only |

The current ordinary managed non-root init restores a Prepared canister and
returns without scheduling bootstrap or application hooks. Activation later
schedules those hooks. Its participant, when configured, still belongs to the
actual IC init message and is not repeated during activation.

The current managed post-upgrade adapter restores state synchronously, obtains
an `active` decision and schedules bootstrap and application hooks only when
active. A configured synchronous participant must run after restoration and
before that activation branch, including when the canister remains Prepared or
inactive.

The existing `init = { ... }` block is a deferred application hook. It runs
through `TimerApi::defer_lifecycle` and remains semantically separate from the
new synchronous participant. It must never be documented or reused as the
composition seam.

B1 must freeze:

1. every macro arm that emits or selects an IC lifecycle export;
2. every synchronous `*_before_bootstrap` owner and its restored invariants;
3. every activation decision, bootstrap scheduler and zero-delay user hook;
4. the exact canonical root/non-root inclusion and specialized Wasm Store and
   Fleet Coordinator exclusion boundary;
5. all source, compile, Candid and Wasm-symbol guards affected by new grammar;
6. current init/post-upgrade instructions, Wasm bytes and timer effects; and
7. the smallest facade/runtime owner that keeps the public macro thin.

An unresolved owner, ordering edge or generated-symbol collision blocks B2.

## Decision Summary

The preferred API adds one optional compile-time participant declaration to
both root and non-root expansions of canonical `canic::start!`:

```rust
canic::start!(
    lifecycle_participant(
        init = crate::lifecycle::after_canic_init,
        post_upgrade = crate::lifecycle::after_canic_post_upgrade,
    ),
);
```

The concrete IcyDB composition is application-owned and framework-neutral at
the Canic boundary:

```rust
icydb::start!(participant);

canic::start!(
    lifecycle_participant(
        init = crate::__icydb_lifecycle_participant::init,
        post_upgrade = crate::__icydb_lifecycle_participant::post_upgrade,
    ),
);
```

`canic::start_local!` may accept the same declaration for manual development,
but local support cannot substitute for canonical root and non-root
`canic::start!` proof. A promoted B1 may make a mechanical spelling adjustment
if macro expansion proves a smaller unambiguous grammar, but the following
semantic contract is frozen:

- both phase paths are supplied together;
- each path must coerce to the exact safe Rust type `fn() -> ()`;
- the declaration is compile-time and cannot be registered or changed at
  runtime;
- one application-owned phase function is invoked for each phase;
- Canic does not accept a list, registry, trait object, closure, string,
  framework identifier or dynamically selected callback; and
- the application owns any deliberate fan-out and its participant ordering.

The current no-participant forms remain the canonical surface for canisters
that need only Canic lifecycle ownership. The current deferred `init = { ... }`
hook remains a distinct current capability. If both declarations are used,
the synchronous participant completes before the deferred block can be
scheduled. Neither is an alias or fallback for the other.

The participant form is supported by root and non-root expansions of canonical
`canic::start!`. It remains rejected for `canic::start_wasm_store!` and
`canic::start_fleet_coordinator!`. Those specialized infrastructure macros do
not inherit support merely because they also own lifecycle code.

## Participant Contract

The two application functions have the exact Rust ABI:

```rust
fn() -> ()
```

They are required to be:

- synchronous and bounded;
- local to the current Wasm instance;
- safe to re-execute after whole-message rollback and operational retry;
- independent of a remote response or future executor turn; and
- complete before any database-dependent or framework-dependent application
  work is deferred.

A participant may synchronously restore heap indexes, reconstruct volatile
registrations and register a timer required for later recovery. It must not
`await`, perform an inter-Canister or management call, spawn asynchronous work
that is required for lifecycle correctness, or claim success before its
required local reconstruction has completed.

The signature cannot prove boundedness, correct phase selection or absence of
forbidden side effects. The consuming application owns those properties.
Canic proves its invocation count, placement and rollback boundary.

The function returns no policy result. If it cannot restore its required
invariants, it traps. Canic does not catch, translate or downgrade that trap.
The lifecycle message fails atomically before Canic schedules later work.

## Exact Lifecycle Ordering

### Managed Non-Root Init

```text
IC init
  -> decode Canic init payload
  -> initialize the shared ic-timers runtime
  -> initialize compiled configuration and Canic environment
  -> restore Canic synchronous state and timer declarations
  -> establish Prepared state
  -> invoke the application init participant exactly once
  -> return Prepared without Canic bootstrap or deferred application work
```

Fleet activation remains a separate same-release transition. It schedules the
existing Canic bootstrap and application install hooks, but does not invoke the
init participant again.

### Managed Non-Root Post-Upgrade

```text
IC post_upgrade
  -> initialize the shared ic-timers runtime
  -> initialize compiled configuration and memory registry
  -> restore Canic environment, authority, runtime state and timer declarations
  -> obtain the current Active/Prepared decision
  -> invoke the application post-upgrade participant exactly once
  -> if Active, schedule Canic bootstrap and deferred application work
  -> if Prepared/inactive, schedule neither
  -> return
```

The participant is not activation-gated. Its job is to restore the composed
Wasm instance, not to decide whether Canic application work may start.

### Canonical Root Init And Post-Upgrade

Root-selected `canic::start!` follows the same partial order:

```text
IC lifecycle message
  -> initialize the shared ic-timers runtime
  -> restore the exact Canic root synchronous state and timer declarations
  -> invoke the matching application participant exactly once
  -> schedule only the Canic root work admitted by current lifecycle state
  -> return
```

The participant does not become root authority and receives no root state. B1
must freeze the actual root adapters and the root Active/Prepared scheduling
edge independently rather than assuming non-root expansion proves it.

### Local Non-Root Init And Post-Upgrade

```text
IC lifecycle message
  -> initialize the shared ic-timers runtime
  -> restore local Canic state and timer declarations synchronously
  -> invoke the matching application participant exactly once
  -> schedule the existing Canic bootstrap and deferred application work
  -> return
```

Local mode does not acquire managed Fleet identity or change the managed
activation contract.

### Independent Synchronous Reconcilers

The application phase function may call more than one independently owned
reconciler:

```rust
fn after_canic_post_upgrade() {
    framework_a_post_upgrade();
    framework_b_post_upgrade();
}
```

That order is application authority. Canic guarantees only that its own
synchronous restoration precedes the application function and that Canic's
later schedulers follow it. Canic does not inspect, reorder or retry individual
calls inside the function.

## Implementation Placement

The macro binds and type-checks the function paths, but lifecycle APIs own
invocation. The macro must not call the participant as free-standing
orchestration. The narrow adapter shape is one optional matching phase function,
for example:

```rust
LifecycleApi::post_upgrade_nonroot_canister_before_bootstrap(
    role,
    config,
    config_source,
    config_path,
    Some(crate::__icydb_lifecycle_participant::post_upgrade),
);
```

The corresponding init, root and optional local adapters receive their matching
`Option<fn() -> ()>` at the same facade boundary. The API invokes it after
shared-timer initialization and Canic synchronous reconstruction but before
the current activation/scheduler branch. In particular, managed post-upgrade
invokes the participant before the existing `if active` branch.

## Ownership And Layering

The maintained dependency direction remains:

```text
start macro -> lifecycle facade/API -> private lifecycle runtime
```

- The public start macros validate and bind the exact function paths, select
  the already canonical role-specific lifecycle adapter and emit the one IC
  lifecycle root.
- The lifecycle facade/API owns the participant invocation boundary and exact
  placement relative to synchronous restoration.
- Private lifecycle runtime continues to own Canic state reconstruction,
  activation decisions, metrics and bootstrap scheduling.
- The application-owned phase function owns downstream composition.

The macro must not gain a participant registry, phase state machine, framework
switch, error mapper or timer implementation. It remains a thin bridge.

DTOs, policy, persisted model and Canic configuration gain no participant
field. There is no serialized function identity, stable callback, dynamic
loader or operator-selected participant.

## Failure, Rollback And Retry

Participant panic, explicit trap and instruction exhaustion fail the enclosing
IC lifecycle message. No later Canic bootstrap timer or deferred user hook may
be committed.

The IC message rollback is the atomicity boundary:

- Canic synchronous writes from the failed lifecycle message roll back;
- participant heap, stable-memory and timer effects roll back;
- a failed install leaves the physical Canister empty and retryable;
- the prior Wasm remains authoritative after a failed same-release upgrade;
  and
- a retry after correcting the application-owned cause starts from the prior
  committed state.

Canic adds no persistent attempt counter or partial-completion receipt for an
uncommitted lifecycle message. Existing install/upgrade execution evidence and
the platform rejection remain the operator-visible failure boundary.

The participant must not catch its own failed mandatory reconstruction and
return success. Advisory work that may fail without invalidating the lifecycle
does not belong in this synchronous seam; it belongs in an existing deferred
application path.

## Public And Artifact Surface

The core seam adds no Candid method, payload, response, diagnostic protocol or
stable state. The participant functions are ordinary Rust items owned by the
application. Canic emits no public executor endpoint for them.

The composed Wasm must retain exactly one IC `init` export and, for surfaces
that support upgrade, exactly one `post_upgrade` export. Participant form must
not add query/update methods, Candid entries, function-table lookup by name or
custom discovery metadata.

Compile and artifact guards must cover:

- missing one of the two phase functions;
- unsafe, async, argument-taking, result-returning and ABI-mismatched paths;
- duplicate participant declarations;
- use on an excluded infrastructure start surface;
- coexistence with the maintained deferred application hook;
- exactly one lifecycle export of each supported kind;
- unchanged Candid apart from unrelated accepted work; and
- absence of participant executor methods and string registries.

## Security Boundary

Canic restores its local ingress and authority invariants before invoking the
application participant. The participant receives no caller, controller,
Fleet, role, session, scope, token or request payload from Canic.

The seam does not:

- authorize application ingress;
- add controller fallback or framework-specific trust;
- permit a remote authorization or lifecycle call;
- expose Canic private state to a participant;
- duplicate the separate local-application-authorization idea's session or
  scope authority; or
- make participant completion evidence that an application is Active or its
  database is Ready.

Function-level authorization and lifecycle composition remain separate. A
consumer may use the separate authorization idea's synchronous local decision
from an endpoint guard only after all participating runtimes have restored
their own ingress-critical state.

## Timer Boundary

This is not a timer-provider design. The independent `ic-timers` adoption
already provides one shared canister-local registry when every linked owner
resolves the same exact package ID. A participant may reconstruct its own
claims through that shared substrate, but continues to own their durable
authority and scheduling policy.

Canic runtime status and metrics may project the complete shared inventory;
neither observation nor lifecycle ordering grants Canic scheduling control
over another owner's claims. The idea adds no provider API, timer policy, naming
scheme or cross-framework dependency.

### Optional Runtime-Epoch Projection

Canic currently consumes the atomic `TimerInventorySnapshot` rows while
dropping its runtime epoch. A useful optional projection is:

```rust
pub struct TimerRuntimeEpochStatus {
    pub canister_version: u64,
    pub started_at_ns: u64,
}
```

This would expose the counter-reset boundary to operators. It is not required
for complete timer/owner inventory or for lifecycle composition. If promoted,
its runtime-status DTO/Candid impact requires explicit ownership and should be
included only when that schema is already changing or approved in the same
slice; omitting it does not block the core seam or combined proof.

## Performance And Measurement

B1 records current no-participant baselines for canonical managed root/non-root
and local fixtures:

- raw and compressed Wasm bytes;
- init and post-upgrade instructions;
- lifecycle wall time in the maintained PocketIC harness;
- Canic timers committed before lifecycle return; and
- generated lifecycle/Candid symbol counts.

Before B2, B1 freezes numeric ceilings for:

1. no-participant wrapper overhead;
2. a no-op participant's incremental instructions;
3. a no-op participant's incremental Wasm bytes; and
4. participant dispatch count.

Canic reports wrapper overhead separately from application participant cost.
It makes no performance claim for arbitrary application code or another
framework. The maintained path uses compile-time function paths and must not
allocate, scan a list or perform string lookup merely to dispatch a
participant.

## Qualification Matrix

The idea requires focused compile, source, artifact and PocketIC evidence:

| Boundary | Required evidence |
| --- | --- |
| Grammar | accepted canonical root/non-root and optional local forms; rejected partial, duplicate, async, unsafe, argument/result and specialized infrastructure forms |
| Managed init | Canic restore precedes participant; participant runs once; no activation/bootstrap/application hook is scheduled |
| Managed post-upgrade Active | restore, participant, bootstrap and deferred application ordering |
| Managed post-upgrade Prepared | participant still runs once; bootstrap and deferred application work remain absent |
| Managed root | exact root restore, participant and admitted scheduler ordering for init and post-upgrade |
| Local lifecycle | restore, participant and scheduler ordering for init and post-upgrade |
| Failure | init/post-upgrade panic, trap and instruction exhaustion commit no later timer or partial Canic/participant state |
| Exact retry | failed install remains empty; failed same-release upgrade leaves prior Wasm/state authoritative; each succeeds after its external/application-owned cause is corrected |
| Symbols | one lifecycle root, unchanged Candid and no participant executor export |
| Performance | frozen no-op overhead ceilings and separated application cost |
| Residue | no framework names in production, registries, serialized callbacks, compatibility grammar or duplicate lifecycle owner |

### Combined Canic And IcyDB Proof Canister

Canic qualification uses both a generic repository-owned participant fixture
and one Canic-owned test canister that composes:

```rust
icydb::start!(participant);

canic::start!(
    lifecycle_participant(
        init = crate::__icydb_lifecycle_participant::init,
        post_upgrade = crate::__icydb_lifecycle_participant::post_upgrade,
    ),
);
```

The canister also schedules one obvious Canic interval timer. Its focused proof
must establish:

1. the resolved dependency graph contains exactly one `ic-timers` package ID;
2. `ic-cdk-timers` occurs only beneath `ic-timers`;
3. the installed Wasm exports exactly one `init` and one `post_upgrade`;
4. `canic_runtime_status().timers` contains at least one `owner = "canic"`
   row and the IcyDB `owner = "icydb"`, `subsystem = "startup"`,
   `name = "recovery"` row;
5. Canic timer execution counters advance;
6. IcyDB recovery progresses through its watchdog;
7. same-release upgrade reconstructs both owners;
8. the IcyDB participant runs while Canic is Prepared or inactive; and
9. a participant trap aborts lifecycle composition and a corrected-cause retry
   safely reconstructs both runtimes.

This proof may use an IcyDB test/dev dependency, but no IcyDB-specific type or
branch enters shipped Canic runtime code. It authorizes no mutation of IcyDB or
`ic-timers` and does not make their releases Canic-owned.

## Release-Batch Plan

| Batch | Outcome | Owner | Included evidence | Validation | State |
| --- | --- | --- | --- | --- | --- |
| B1 | Current lifecycle, dependency and artifact inventory; frozen participant grammar and ceilings | lifecycle facade, macros and test inventory | all start variants, shared-timer initialization, restore/scheduler edges, production/test dependency boundary, exclusions, symbols and costs; no runtime mutation | reproducible source, dependency, expansion, Candid, Wasm and PocketIC baseline | Historical evidence approval only; idea now deferred |
| B2 | Compile-time participant declaration and thin facade boundary | `canic` macro/facade and `canic-core` lifecycle API | exact safe `fn() -> ()` coercion, paired phases, canonical root/non-root support, specialized infrastructure exclusions and no dynamic registry | focused compile-pass/fail, macro surface and package checks | Blocked on promotion and refreshed B1 |
| B3 | Canonical managed synchronous ordering | root/non-root lifecycle APIs and start adapters | shared timer initialization, Prepared init, Active/Prepared post-upgrade, activation non-repetition and deferred-hook ordering | focused source guards and PocketIC root/non-root lifecycle cases | Blocked on B2 |
| B4 | Local composition and exact failure rollback | local lifecycle adapter and generic participant fixture | local init/upgrade, install/upgrade trap, instruction exhaustion and corrected-cause retry | focused PocketIC failure/recovery matrix | Blocked on B3 |
| B5 | Combined shared-timer evidence and artifact propagation | Canic-owned generic and Canic+IcyDB test canisters, facade tests and docs | one `ic-timers` package ID, one lifecycle export pair, both timer owners, execution/recovery, upgrade reconstruction, no-op overhead and composition guide | dependency tree, raw-Wasm/Candid diff, PocketIC combined proof, measurements and link checks | Blocked on B4 |
| B6 | Hard-cut cleanup and Canic-only closeout | cross-layer release batch | stale wording/duplicate path removal, specialized exclusions and combined-proof reconciliation | targeted residue scan and maintained-doc review | Blocked on B5 |

The six batches cover one narrow end-to-end capability without creating a
participant framework or timer redesign. They are implementation and evidence
boundaries, not preassigned patch releases.

## Non-Goals

The idea does not add:

- an IcyDB or other framework production dependency;
- a generic lifecycle framework, registry or plugin system;
- more than one application-owned function per phase;
- runtime participant registration or ordering configuration;
- async participant completion;
- Wasm Store or Fleet Coordinator participant support;
- a new shared timer provider, watchdog API or timer-metrics authority;
- application readiness, authorization or health policy;
- a new lifecycle Candid endpoint or diagnostic protocol;
- cross-release upgrade, state migration, adoption or rollback compatibility;
  or
- compatibility aliases for rejected macro grammar.

## Hard-Cut And Recovery Posture

The promoted line has one current participant grammar when released. Any
provisional or rejected spelling is deleted rather than retained as an alias.
Earlier-release installations are recreated; no prior lifecycle mode or
callback identity is decoded.

Same-release lifecycle failure remains recoverable. An interrupted or rejected
same-release install/upgrade may be retried from the last committed state after
the cause is corrected. This operational safety does not create cross-release
compatibility.

## Acceptance Criteria

The promoted line is complete only when:

1. the exact maintained lifecycle owners and all macro variants are inventoried;
2. the participant declaration binds exactly two safe `fn() -> ()` paths at
   compile time;
3. one application-owned phase function, not a list or registry, owns any
   downstream fan-out;
4. Canic synchronous restoration precedes the participant in every supported
   phase;
5. the participant precedes every Canic bootstrap scheduler and deferred user
   hook;
6. managed post-upgrade invokes the participant even when Canic remains
   Prepared or inactive;
7. managed activation never repeats the init participant;
8. canonical root and non-root `canic::start!` expansions support the same
   participant contract, while Wasm Store and Fleet Coordinator omit it;
9. participant trap and instruction exhaustion commit no later scheduler,
   timer or partial state;
10. corrected-cause install and same-release upgrade retries preserve the empty
    or last committed state respectively and succeed exactly once;
11. the composed artifact retains one lifecycle root, unchanged Candid and no
    participant executor endpoint;
12. the macro remains thin and participant invocation follows the canonical
    lifecycle facade/API boundary;
13. dispatch uses no allocation, scan, string lookup, dynamic callback or
    persisted function identity;
14. measured wrapper overhead remains within the frozen B1 ceilings;
15. Canic documentation distinguishes the synchronous participant from the
    existing deferred application hook;
16. shipped Canic owns no downstream framework type or release gate;
17. one Canic-owned combined test canister proves a single `ic-timers` package
    ID, `ic-cdk-timers` only beneath it, one lifecycle export pair, exact Canic
    and IcyDB inventory owners, Canic counter progress, IcyDB watchdog progress,
    upgrade reconstruction and inactive invocation;
18. participant-trap rollback and corrected-cause retry are proven in the
    combined canister;
19. timer-provider work remains outside this line and runtime-epoch projection
    remains optional;
20. no compatibility alias, old participant spelling or second lifecycle owner
    remains; and
21. status, design relationships and the open release batch are current.

## Promotion Gate

Mutating B2-B6 work may begin only when:

1. the idea has an accepted numbered release position and refreshed B1 is
   explicitly authorized;
2. B1 freezes the exact current lifecycle owners, macro expansions, artifact
   symbols and performance baselines;
3. the maintainer approves the exact participant grammar, canonical root/non-
   root inclusion and Wasm Store/Fleet Coordinator exclusion boundary;
4. the root/non-root managed Prepared-init, Active/Prepared post-upgrade and
   optional local ordering tables are reconciled with current source;
5. the failure, rollback and exact-retry harness is feasible without production
   `cfg(test)` behavior;
6. the facade/runtime owner keeps `canic::start!` thin; and
7. the combined proof can consume an exact test-only IcyDB revision without
   adding a shipped Canic dependency; and
8. the maintainer explicitly authorizes mutation.

As an unnumbered idea, this document authorizes no B1 work, runtime, macro,
Candid, stable-state, package-version, changelog-version or downstream-
repository mutation.
