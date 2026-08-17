# Canic 0.103 Implementation Status

Date: 2026-08-16

## Status

- State: accepted and scheduled as reserve-Fleet critical-path step 1. B1
  repository evidence/inventory is review-ready; B2 and runtime mutation are held.
- Outcome: one bounded command/status control plane per Canic role. Compiled
  capabilities add variants, command/status authority is variant-specific
  and workflows retain every private phase. Only genuinely asynchronous or
  durable commands acquire operation identities.
- Baseline evidence: the immutable [`v0.102.2` role-surface capture](../../audits/working/0.103-role-owned-candid-surface/README.md)
  freezes all 207 methods across representative Root and managed profiles plus
  canonical Coordinator and Store interfaces. It separates 188 Canic-owned,
  three external-standard and 16 fixture-owned methods.
- Runtime impact: none from this design and renumbering cut.
- Release boundary: reinstall only; no old method alias, fallback caller,
  dual protocol, migration or mixed-version release set is permitted.
- Implementation approval: B1 evidence only. The accepted B1 contract must
  reconcile against completed 0.102 diagnostics before a separate B2
  promotion; no endpoint, runtime, Candid, stable-state, CLI or version
  mutation is authorized.
- Successor: 0.104 timer ownership begins from the completed autonomous-
  operation surface. No later production protocol may add phase methods while
  0.103 is incomplete.

Design:
[Role-owned Candid surface and autonomous operations hard cut](0.103-design.md)

## Quantitative Contract

- managed application canister: status plus an optional command, maximum two;
- Fleet Subnet Root: command plus status, maximum two;
- Fleet Coordinator: command plus status, maximum two;
- those roles: maximum three only with one proven composite-status exception;
- Wasm Store: starts with command/status; every additional method needs
  independent byte-transport/resource admission evidence, and six is an
  absolute emergency ceiling rather than spare capacity; and
- capabilities, callers, operations and phases add zero methods.

Externally mandated standards and application-owned methods are counted and
classified separately from the Canic-owned architectural ceilings.

B1 may propose smaller counts. Raising a ceiling requires an explicit design
amendment and cannot be hidden in implementation.

## Release-Batch Tracker

| Batch | Outcome | Direct evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| B1 | Exact endpoint/caller inventory and role manifest | six-way disposition, command/status authority, capability discovery, reserved-name rule, old-to-new variant map and immutable counts | reproducible Candid/source/caller inventory and review | Review-ready: complete register, DTO/pruning manifest, operation ownership and 0.104 handoff; awaiting explicit acceptance |
| B2 | Manifest-exact role status surfaces and standards | only accepted Root/Coordinator/Store/managed status variants, standards and bounded observations | DTO/policy, Candid, command/status auth and first-excess tests | Blocked on accepted B1 |
| B3 | Compile-time variant pruning | pruned variants, DTOs, handlers and types; reserved-name failures; thin `start!` | build pass/fail, Candid absence, collision, reachability and role guards | Blocked on B2; B2/B3 unreleased until complete |
| B4 | Root and Coordinator autonomous intent | command variants, async/durable receipts, typed atomic responses, variant auth/replay, self-advance and private phases | property and PocketIC replay/interruption journeys | Blocked on B3 |
| B5 | Managed and Store role surfaces | optional managed command, inert pruning, Store control variants and justified byte lanes | authority, absence, payload and cross-canister PocketIC tests | Blocked on B4 |
| B6 | Caller and generated propagation | host, CLI, protocol constants, replay manifests, bindings, fixtures and representative configured profiles | targeted host/CLI/build/PocketIC checks | Blocked on B5 |
| B7 | Removal and measured closeout | old surface deletion, residue guards, count/size report and documentation | generated count, forbidden-method and residue checks | Blocked on B6 |

B2 may not emit a temporary all-capabilities status superset. B2 and B3 remain
one unreleased sequence until representative builds prove exact variant/type/
handler/reachability pruning.

## B1 Completion Contract

B1 must deliver together:

1. an immutable source, toolchain and generated-Candid baseline;
2. every emitted method by canonical role and capability;
3. exact execution mode, visibility, authority, payload and replay policy;
4. every workflow owner and in-repository caller;
5. exactly one disposition per method: role command variant, role status
   variant, Store data plane, external standard, application-owned or
   private/delete;
6. one accepted old-to-new variant and DTO mapping;
7. the exact static capability declaration, invalid-combination rules,
   authoritative capability-discovery source and reserved-name collision rule;
8. a final role/capability manifest satisfying the quantitative ceilings and
   proving capabilities prune variants and reachability without adding methods;
9. complete host, CLI, binding, test and documentation propagation;
10. proof that atomic commands avoid operation machinery while asynchronous or
    durable commands receive exact operation ownership; and
11. the exact 0.104 timer-consumer handoff.

## Critical-Path Position

1. 0.103 hard-cuts the Candid surface and internalizes orchestration phases.
2. 0.104 hard-cuts timer mechanics and domain async-job recovery ownership.
3. 0.105 qualifies platform behavior, costs, balances and bounded lanes.
4. 0.106 closes replay-safe Coordinator-backed root operating funding.
5. 0.107 implements reusable Fleet Subnet Canister estates and proves the
   10/100/1,000 progression.
6. 0.108 serves the T2 Fleet observatory from every installed Canister.

0.109 local application authorization remains the scheduled successor, not a
reserve-Fleet prerequisite. Unnumbered ideas remain outside the path.

## Next Authorized Action

B1 is review-ready; no runtime or protocol mutation is authorized. Review and
explicitly accept the complete method register and capability/DTO/operation
manifest before promoting B2 or replacing any Candid method or generated
caller. `private/delete` remains the default and unused method-ceiling capacity
is not evidence.
