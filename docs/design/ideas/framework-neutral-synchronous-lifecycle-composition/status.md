# Idea Status: Synchronous Lifecycle Participation And Combined Shared-Timer Evidence

Classification: deferred and unnumbered. The former `0.104` working number is
retired; retained batch notes are historical review context and authorize no
current evidence or implementation work.

Former status: proposed design authority; B1 evidence work was approved before
deferral; mutating implementation was held

Design:
[Synchronous lifecycle participation and combined shared-timer evidence](design.md)

Status cut: 2026-08-16

## Current Boundary

The idea owns one compile-time synchronous application lifecycle seam inside
Canic's existing single IC lifecycle root plus one Canic-owned combined
Canic+IcyDB test canister. Production Canic gains no framework dependency,
participant registry, timer provider, Candid method or persisted callback.

The former B1 evidence approval was retired when this design became an idea.
No current evidence or mutation is authorized. Promotion must assign a current
release position, refresh B1 and explicitly authorize the implementation.

The current 0.102.1 worktree preserves the lifecycle shape:

- `canic::start!` owns managed root/non-root `init` and `post_upgrade`;
- `canic::start_local!` owns local non-root lifecycle;
- synchronous `*_before_bootstrap` adapters restore Canic state;
- managed post-upgrade schedules later work only when Active;
- managed initial install remains Prepared until activation; and
- the optional `init = { ... }` block is deferred through a zero-delay
  `ic-timers` `Once` registration.

That optional block is not a synchronous composition seam.

## Approved Direction

Add one optional `lifecycle_participant(init = ..., post_upgrade = ...)`
declaration to root and non-root expansions of canonical `canic::start!`.
Each path must coerce to `fn() -> ()`. Canic invokes the matching function
exactly once after shared-`ic-timers` initialization and synchronous Canic
restore, but before activation-gated bootstrap or deferred user work.

The application owns any fan-out to independent framework participants. Canic
owns no framework name, participant list, dynamic registry, runtime callback
selection or downstream release gate.

Managed post-upgrade invokes the participant even when the Canister remains
Prepared or inactive. Managed activation does not repeat the init participant.
`start_local!` is useful but cannot substitute for canonical production proof.
Wasm Store and Fleet Coordinator remain excluded.

Participant trap or instruction exhaustion fails and rolls back the complete
lifecycle message. Canic does not catch or translate it and commits no later
bootstrap timer or deferred application hook.

## Scheduling

This idea has no working number and does not gate the scheduled 0.103-0.108
Fleet path or standalone 0.109 authorization qualification. Its former `0.104`
planning references are historical only.

## Release-Batch Tracker

| Batch | Outcome | Owner | Included evidence | Validation | State |
| --- | --- | --- | --- | --- | --- |
| B1 | Current lifecycle, dependency and artifact inventory | lifecycle facade, macros and test inventory | all start variants, shared-timer initialization, ordering, dependency boundary, exclusions, symbols and costs | reproducible dependency, source, expansion, Candid, Wasm and PocketIC baseline | Blocked on promotion |
| B2 | Compile-time participant declaration and thin facade boundary | `canic` macro/facade and `canic-core` lifecycle API | paired safe paths, canonical root/non-root support, specialized exclusions and no dynamic registry | focused compile-pass/fail, macro surface and package checks | Blocked on B1 |
| B3 | Canonical managed synchronous ordering | root/non-root lifecycle APIs and start adapters | timer initialization, Prepared init, Active/Prepared post-upgrade and activation non-repetition | focused source guards and PocketIC root/non-root cases | Blocked on B2 |
| B4 | Local composition and exact failure rollback | local lifecycle adapter and generic fixture | local init/upgrade, trap, exhaustion and corrected-cause retry | focused PocketIC failure/recovery cases | Blocked on B3 |
| B5 | Combined shared-timer evidence and propagation | Canic generic and Canic+IcyDB test canisters, facade tests and docs | one timer package ID, one lifecycle export pair, both owners, progress, upgrade reconstruction and composition guide | dependency tree, raw-Wasm/Candid diff, combined PocketIC proof and measurements | Blocked on B4 |
| B6 | Hard-cut cleanup and Canic-only closeout | cross-layer release batch | residue removal, specialized exclusions and combined-proof reconciliation | targeted residue scan and maintained-doc review | Blocked on B5 |

These are implementation/evidence boundaries, not preassigned patch releases.

## B1 Evidence Questions

B1 must answer:

1. which macro arms emit every current lifecycle and inspect-message export;
2. which exact synchronous invariants each `*_before_bootstrap` adapter
   restores;
3. where Active/Prepared decisions, bootstrap schedulers and deferred user
   hooks currently occur;
4. which compile and artifact guards assume that managed init schedules no
   timer of any kind rather than no Canic bootstrap/application timer;
5. the smallest macro grammar and facade/API owner for paired safe
   `fn() -> ()` paths;
6. exact canonical root/non-root support and compile-time rejection for Wasm
   Store and Fleet Coordinator;
7. how a generic fixture observes ordering without adding production
   `cfg(test)` behavior;
8. how failed install/upgrade and corrected-cause retry are proven from the
   empty or last committed state; and
9. current no-participant Wasm, instruction, timer and symbol baselines plus
   acceptable no-op participant ceilings; and
10. the exact test-only IcyDB revision and dependency graph which resolve one
    `ic-timers` package ID with `ic-cdk-timers` only beneath it.

## Current Integration Statement

No IcyDB or `ic-timers` code change is required. IcyDB already exposes the two
private synchronous participant functions and assigns single-owner composition
downstream; `ic-timers` already supplies atomic owner-labelled inventory and a
runtime epoch. The missing seam and combined proof belong in Canic.

The required combined Canic test canister must prove one resolved `ic-timers`
package ID, `ic-cdk-timers` only below it, one init/post-upgrade export pair,
Canic and IcyDB timer rows, both execution paths, upgrade reconstruction,
Prepared/inactive invocation and trap rollback with safe retry. This test-only
composition adds no IcyDB-specific production branch or authority.

## Next Authorized Action

No implementation or evidence work is currently authorized. On promotion, B1
must re-read and measure the current lifecycle macro, facade/runtime, dependency
graph, generated Candid/Wasm and focused PocketIC fixtures before any mutation.
No downstream-repository mutation is authorized by this idea.
