# Canic 0.104 Implementation Status

Status: proposed design authority; B1 evidence work approved; mutating
implementation held

Design:
[0.104 framework-neutral synchronous lifecycle composition](0.104-design.md)

Status cut: 2026-08-14

## Current Boundary

0.104 is reserved for one compile-time, synchronous application lifecycle seam
inside Canic's existing single IC lifecycle root. It adds no framework
dependency, participant registry, timer provider, Candid method or persisted
callback.

B1 evidence may run alongside 0.102 and 0.103 evidence. No mutating 0.104 batch
may begin until 0.103 is accepted and complete, the B1 contract is approved
and the maintainer explicitly authorizes mutation.

The current 0.101.53 implementation remains unchanged:

- `canic::start!` owns managed root/non-root `init` and `post_upgrade`;
- `canic::start_local!` owns local non-root lifecycle;
- synchronous `*_before_bootstrap` adapters restore Canic state;
- managed post-upgrade schedules later work only when Active;
- managed initial install remains Prepared until activation; and
- the optional `init = { ... }` block is deferred through a zero-delay timer.

That optional block is not a synchronous composition seam.

## Approved Direction

Add one optional paired declaration to application-capable managed non-root and
local start surfaces. Each phase path must coerce to `fn() -> ()`. Canic invokes
the matching application-owned function exactly once after its synchronous
restore and before any activation-gated bootstrap or deferred user work.

The application owns any fan-out to independent framework participants. Canic
owns no framework name, participant list, dynamic registry, runtime callback
selection or downstream release gate.

Managed post-upgrade invokes the participant even when the Canister remains
Prepared or inactive. Managed activation does not repeat the init participant.
Root-selected `canic::start!`, Wasm Store and Fleet Coordinator remain
inventory-only and gain no participant form in this line.

Participant trap or instruction exhaustion fails and rolls back the complete
lifecycle message. Canic does not catch or translate it and commits no later
bootstrap timer or deferred application hook.

## Renumbered Future Designs

Inserting this line moves the still-provisional former 0.104-0.112 designs to
0.105-0.113 without changing their order or implementation status:

| Current line | Design |
| --- | --- |
| 0.105 | Cross-Subnet data transport groundwork |
| 0.106 | Coordinator Workers |
| 0.107 | Declarative authentication profiles |
| 0.108 | Standalone blob-service extraction |
| 0.109 | Coordinator-backed root funding |
| 0.110 | Optional encrypted Canister snapshot archives |
| 0.111 | Language-neutral managed-guest feasibility |
| 0.112 | Skynet Fleet observatory |
| 0.113 | Fleet Subnet Canister estates |

Published package versions, historical changelogs, audit evidence and archived
handoffs retain their original identities.

## Release-Batch Tracker

| Batch | Outcome | Owner | Included evidence | Validation | State |
| --- | --- | --- | --- | --- | --- |
| B1 | Current lifecycle and artifact inventory; frozen participant grammar and ceilings | lifecycle facade, macros and test inventory | all start variants, restore/scheduler edges, exclusions, symbols and costs; no runtime mutation | reproducible source, expansion, Candid, Wasm and PocketIC baseline | Approved evidence only |
| B2 | Compile-time participant declaration and thin facade boundary | `canic` macro/facade and `canic-core` lifecycle API | exact paired safe function paths and no dynamic registry | focused compile-pass/fail, macro surface and package checks | Blocked on B1 and promotion |
| B3 | Managed non-root synchronous ordering | non-root lifecycle API and start adapter | Prepared init, Active/Prepared post-upgrade and activation non-repetition | focused source guards and PocketIC managed lifecycle cases | Blocked on B2 |
| B4 | Local composition and exact failure rollback | local lifecycle adapter and generic fixture | local init/upgrade, install/upgrade trap, exhaustion and corrected-cause retry | focused PocketIC failure/recovery cases | Blocked on B3 |
| B5 | Artifact, performance and documentation propagation | facade, tests and docs | one lifecycle root, unchanged Candid, overhead ceilings and composition guide | raw-Wasm/Candid diff, measurements and link checks | Blocked on B4 |
| B6 | Hard-cut cleanup and Canic-only closeout | cross-layer release batch | residue removal, infrastructure exclusions and downstream-independent qualification | targeted residue scan and maintained-doc review | Blocked on B5 |

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
6. the exact compile-time rejection for root, Wasm Store and Coordinator;
7. how a generic fixture observes ordering without adding production
   `cfg(test)` behavior;
8. how failed install/upgrade and corrected-cause retry are proven from the
   empty or last committed state; and
9. current no-participant Wasm, instruction, timer and symbol baselines plus
   acceptable no-op participant ceilings.

## Current Integration Statement

Canic 0.103 and a framework-neutral application guard compose at the function
level. Combined runtime qualification is not available from current Canic.
It requires the separately accepted and implemented 0.104 synchronous seam.

0.104 itself remains framework-neutral. A downstream framework may provide
private synchronous init/post-upgrade functions and qualify an application-
owned adapter, but its repository, types and release do not become Canic
authority.

## Next Authorized Action

B1 may inspect and measure the current lifecycle macro, facade/runtime,
generated Candid/Wasm and focused PocketIC fixtures. It may freeze the exact
grammar, ordering, exclusion boundary and numeric overhead ceilings.

B2-B6 remain blocked. No runtime, macro, Candid, stable-state, package-version,
changelog-version or downstream-repository mutation is authorized.
