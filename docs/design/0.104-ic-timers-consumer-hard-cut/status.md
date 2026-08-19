# Canic 0.104 Implementation Status

Date: 2026-08-19

## Status

- State: implementation-complete as application-safety step 2. The 2026-08-18
  Prequel Wars review promotes downstream native-timer adoption and the
  synchronous lifecycle-participant seam into this line. The maintainer
  accepted the complete B1 evidence on 2026-08-18 and accepted B2 on the same
  date with its intermediate Wasm observation retained. Continuation accepted
  B3-B8 after their behavioral evidence and current-graph PocketIC checks.
  Published `v0.104.0` contains that runtime boundary. A bounded 0.104.1
  closeout correction replaces the lexical B8 census, completes failed-install
  participant rollback evidence and corrects the quantitative record.
- Runtime impact: B2 removes the public timer facade and transient claim
  machinery. B3 hard-cuts memory ID 60 to minimal domain attempt fences and
  cycle-only exact retry identity. B4 moves auth, cycles and placement to lazy
  owner-native registrations, reducing the representative provider inventory
  from seven rows to five. It changes no Candid. B5 deletes the remaining
  central claim and participant registries, gives pool maintenance and the one
  Root watchdog direct native
  custody, and splits Root/Coordinator snapshot linkage. The original B2-B6
  phase-size and phase-performance deltas are retained only as historical
  observations because their source states were not preserved; they are not
  closeout evidence. B6 publishes the direct-native adoption guide and adds
  one safe paired synchronous participant after Canic restoration and before
  deferred work for ordinary managed, Root and local lifecycles. B7 composes
  that seam with exact published IcyDB, proving one resolved provider, one lifecycle
  export pair and separately reconstructed owner rows. Its artifact is
  test-only and changes no shipped product role. B8 parses the 45-file Rust
  ownership set, freezes exact native-capability callsites and rejects renamed,
  re-exported, unclassified or duplicate scheduling authority. The current
  interval observation is 46,593 instructions with no memory-page growth.
- Quantitative boundary: exact locked release-tree builds report 19,424,848
  raw / 5,030,696 gzip bytes for 0.103.0 and 19,124,317 raw / 4,959,729 gzip
  bytes for 0.104.0. Each release uses its own version-enforcing canonical
  builder, so no controlled causal percentage is claimed. Intermediate phase
  tables are historical only.
- Baseline finding: Canic resolves one `ic-timers 0.6.1` runtime and has no raw
  production `ic-cdk-timers` path, but duplicate provider vocabulary, a public
  application facade and generic stable shadow scheduling remain in published
  `v0.103.0`.
- Release boundary: reinstall only; no timer or async-recovery state migration
  or compatibility surface is permitted.
- Predecessor: `v0.103.0` publishes the completed role-owned Candid and
  autonomous-operation hard cut. B1 inventories that exact tagged result.
- Successors: 0.105 local application authorization follows directly. 0.106
  platform B1 may continue as evidence-only work but must reconcile its final
  timer inventory against 0.104. No 0.107 root-funding or 0.108 estate
  mutation begins before 0.104 closeout.

## Release-Batch Tracker

| Batch | Outcome | Direct evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| B1 | Frozen timer/recovery ownership | Complete call-site, type, claim and stable-field dispositions; exact provider graph; domain-demand map | Source/dependency guards and [reviewable inventory](../../audits/working/0.104-timer-ownership/README.md) | Accepted 2026-08-18 |
| B2 | Native provider surface hard cut | Public timer API removal, native results/directives, typed registration custody and compile fallout | Core/facade tests, current-surface guard, targeted Clippy, PocketIC and [Wasm/performance evidence](../../audits/working/0.104-timer-ownership/b2-native-provider-surface.md) | Accepted 2026-08-18 |
| B3 | Domain-owned async-job recovery | Minimal stable state, exact retry, leases, takeover and deletion of shadow scheduling fields | State/property, interruption and [Wasm/performance evidence](../../audits/working/0.104-timer-ownership/b3-domain-async-job-recovery.md) | Accepted 2026-08-18 |
| B4 | Core fixed-owner reconstruction | Auth, capability-pruned automatic top-up and placement native schedules plus owner-specific retry/stop behavior | Targeted unit, snapshot, [Wasm/performance evidence](../../audits/working/0.104-timer-ownership/b4-domain-native-custody.md) and PocketIC recovery journeys | Accepted 2026-08-18 |
| B5 | Pool/lifecycle/snapshot propagation | Native pool custody, one watchdog, zero-delay lifecycle work, suspension/resume and [exact evidence](../../audits/working/0.104-timer-ownership/README.md#b5-pool-lifecycle-and-snapshot-completion) | Control-plane, lifecycle, snapshot, runtime-probe and role-pruned Wasm checks | Accepted 2026-08-18 |
| B6 | Native downstream adoption and lifecycle participant | Maintained migration guide, direct-provider fixture, paired compile-time participant, exact ordering and [rollback evidence](../../audits/working/0.104-timer-ownership/README.md#b6-native-adoption-and-lifecycle-participant) | Compile pass/fail, exact Candid/exports and PocketIC lifecycle checks | Accepted 2026-08-18 |
| B7 | Combined Canic/IcyDB qualification | One provider, one lifecycle export pair, both owner rows, progress, inactive reconstruction and [corrected-cause retry](../../audits/working/0.104-timer-ownership/README.md#b7-combined-canic-and-icydb-qualification) | Dependency, artifact and focused PocketIC composition proof | Accepted 2026-08-19 |
| B8 | Semantic ownership proof and closeout | One-provider graph, [shared inventory](../../audits/working/0.104-timer-ownership/consumer-inventory.tsv), ownership guards and [complete measured closeout](../../audits/working/0.104-timer-ownership/README.md#b8-semantic-ownership-and-closeout) | Targeted repository guards and PocketIC timer/lifecycle suites | Accepted 2026-08-19 |

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

Published `v0.104.0` remains the runtime release. The bounded closeout
correction is recorded in the open 0.104.1 changelog. Versioning, tagging and
publication remain part of the separately owned maintainer release flow. After
its targeted checks, the only remaining action is maintainer closeout review.
Scheduled 0.105 B1 still requires explicit maintainer promotion and is not
authorized by this completion.
