# Canic 0.104 Implementation Status

Date: 2026-08-18

## Status

- State: accepted and scheduled as application-safety step 2. The 2026-08-18
  Prequel Wars review promotes downstream native-timer adoption and the
  synchronous lifecycle-participant seam into this line. No implementation
  batch is promoted by this planning cut.
- Runtime impact: none from this design/resequencing change.
- Baseline finding: Canic resolves one `ic-timers 0.6.1` runtime and has no raw
  production `ic-cdk-timers` path, but duplicate provider vocabulary, a public
  application facade and generic stable shadow scheduling remain in the
  current untagged candidate.
- Release boundary: reinstall only; no timer or async-recovery state migration
  or compatibility surface is permitted.
- Predecessor: completed 0.103 role-owned Candid and autonomous-
  operation hard cut. B1 must inventory its exact resulting timer consumers.
- Successors: 0.105 local application authorization follows directly. 0.106
  platform B1 may continue as evidence-only work but must reconcile its final
  timer inventory against 0.104. No 0.107 root-funding or 0.108 estate
  mutation begins before 0.104 closeout.

## Release-Batch Tracker

| Batch | Outcome | Direct evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| B1 | Frozen timer/recovery ownership | Complete call-site, type, claim and stable-field dispositions; exact provider graph; domain-demand map | Source/dependency guards and reviewable inventory | Pending promotion |
| B2 | Native provider surface hard cut | Public timer API removal, native results/directives, typed registration custody and compile fallout | Core/facade tests, compile-fail guard and targeted Clippy | Blocked on accepted B1 |
| B3 | Domain-owned async-job recovery | Minimal stable state, exact retry, leases, takeover and deletion of shadow scheduling fields | State/property and interruption tests | Blocked on B2 |
| B4 | Core fixed-owner reconstruction | Auth, capability-pruned automatic top-up and placement native schedules plus owner-specific retry/stop behavior | Targeted unit and PocketIC recovery journeys | Blocked on B3 |
| B5 | Pool/lifecycle/snapshot propagation | Native pool custody, one watchdog, zero-delay lifecycle work, suspension/resume and fixtures | Control-plane, lifecycle, snapshot and runtime-probe checks | Blocked on B4 |
| B6 | Native downstream adoption and lifecycle participant | Maintained migration guide, direct-provider fixture, paired compile-time participant, exact ordering and rollback | Compile pass/fail, symbols and PocketIC lifecycle checks | Blocked on B5 |
| B7 | Combined Canic/IcyDB qualification | One provider, one lifecycle export pair, both owner rows, progress, inactive reconstruction and corrected-cause retry | Dependency, artifact and focused PocketIC composition proof | Blocked on B6 |
| B8 | Semantic ownership proof and closeout | One-provider graph, shared inventory, ownership guards and complete docs/changelog cleanup | Targeted repository guards and PocketIC timer/lifecycle suites | Blocked on B7 |

## B1 Completion Contract

B1 must deliver together:

1. immutable source, Cargo-lock and exact provider baseline;
2. every production registration, schedule, reconcile, cancellation and
   lifecycle path;
3. every public/internal timer type, macro, facade and handle disposition;
4. every native registration claim and its required custody owner;
5. every async-recovery field classified as business authority, derived view
   or prohibited provider mirror;
6. owner-specific authoritative demand and retry reconstruction for auth,
   cycles, placement acknowledgement and pool maintenance;
7. exact memory-ID-60 retain/reshape-or-remove decision; and
8. the exact native downstream guide/fixture contract and every lifecycle
   participant grammar, ordering, exclusion and artifact edge; and
9. a complete propagation and validation map for B2-B8.

## Critical-Path Position

1. 0.103 hard-cuts the Candid surface and internalizes orchestration phases.
2. 0.104 completes timer ownership, async-job recovery, downstream adoption
   and synchronous lifecycle composition.
3. 0.105 adds framework-neutral local application authorization.
4. 0.106 qualifies platform behavior, costs, balances and bounded lanes.
5. 0.107 closes replay-safe Coordinator-backed root operating funding.
6. 0.108 implements reusable Fleet Subnet Canister estates and proves the
   10/100/1,000 progression.
7. 0.109 qualifies one explicit stateful predecessor-to-successor adoption.
8. 0.110 serves the generic Fleet observatory from every installed Canister.

Unnumbered ideas remain outside this path. Historical releases, archived
designs and deferred drafts retain their truthful old identities.

## Next Authorized Action

No source mutation is authorized. Review and explicitly promote B1, then
freeze the exact ownership, durable-field, downstream-adoption and lifecycle
inventory before removing or reshaping any timer or recovery surface.
