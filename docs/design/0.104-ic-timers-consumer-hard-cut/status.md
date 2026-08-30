# Canic 0.104 Implementation Status

Date: 2026-08-19
Roadmap reconciled: 2026-08-30

## Status

- State: complete. Published `v0.104.0` contains the accepted runtime hard cut,
  `v0.104.1` its first bounded evidence correction and `v0.104.2` the final
  audit correction. The final release freezes native registration actions,
  prunes automatic-top-up callback/workflow reachability from
  capability-disabled artifacts and corrects the release-size evidence
  authority. Its immutable source also contains the accepted 0.105 B1/B2
  application-authorization hard cut; the active changelog records that early
  inclusion rather than misclassifying it as unreleased.
- Runtime impact: B2 removes the public timer facade and transient claim
  machinery. B3 hard-cuts memory ID 60 to minimal domain attempt fences and
  cycle-only exact retry identity. B4 moves auth, cycles and placement to lazy
  owner-native registrations. The seven-to-five result is a historical B3/B4
  development observation; the final inventory is role-specific rather than a
  five-row universal runtime. It changes no Candid. B5 deletes the remaining
  central claim and participant registries, gives pool maintenance and the one
  Root watchdog direct native
  custody, and splits Root/Coordinator snapshot linkage. The original B2-B6
  phase-size and phase-performance deltas are retained only as historical
  observations because their source states were not preserved; they are not
  closeout evidence. B6 publishes the direct-native adoption guide and adds
  one safe paired synchronous participant after Canic restoration and before
  deferred work for ordinary managed, Root and local lifecycles. B7 composes
  that seam with exact published IcyDB, proving one resolved provider, one lifecycle
  export pair and complete phase-specific shared inventories with separately
  reconstructed owner rows. Its artifact is
  test-only and changes no shipped product role. Published B8 parses the
  45-file Rust ownership set and freezes native constructors. Published
  `v0.104.2` freezes exact registration actions in addition to those
  constructors and rejects their aliased, unclassified or duplicate use. The
  46,593-instruction, zero-page
  result is one current two-work-sample observation, not a threshold or causal
  performance claim. The timer correction adds doc-hidden macro plumbing
  for capability-specific internal selection but no supported facade/prelude
  item, lifecycle grammar or Candid method. Separately, the early 0.105 B1/B2
  inclusion changes delegated-token Candid and the public Rust authorization
  model without allocating active session stable state.
- Quantitative boundary: the recorded 19,424,848 raw / 5,030,696 gzip bytes
  for 0.103.0 and 19,124,317 raw / 4,959,729 gzip bytes for 0.104.0 are retained
  as historical release-identity-bearing observations. Their exact
  release-build-ID inputs were not retained, so they are not independently
  reproducible closeout authority. Rerunning the documented no-ID command
  produces 19,424,589 / 5,030,663 for 0.103.0, 19,123,930 / 4,959,656 for
  0.104.0 and 19,123,917 / 4,959,773 for 0.104.1.
  These no-ID results are not canonical release-identity evidence.
  No controlled causal percentage is claimed. Intermediate phase tables are
  historical only.
- Baseline finding: Canic resolves one `ic-timers 0.6.1` runtime and has no raw
  production `ic-cdk-timers` path, but duplicate provider vocabulary, a public
  application facade and generic stable shadow scheduling remain in published
  `v0.103.0`.
- Release boundary: reinstall only; no timer or async-recovery state migration
  or compatibility surface is permitted.
- Predecessor: `v0.103.0` publishes the completed role-owned Candid and
  autonomous-operation hard cut. B1 inventories that exact tagged result.
- Successors: 0.105 B1/B2 are already present in the exact published
  `v0.104.2` predecessor, so B3 may proceed. 0.106 B1 evidence may remain
  preserved, but its final timer inventory must use `v0.104.2` rather than
  `v0.104.1` as current authority.

## Release-Batch Tracker

| Batch | Outcome | Direct evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| B1 | Frozen timer/recovery ownership | Complete call-site, type, claim and stable-field dispositions; exact provider graph; domain-demand map | Source/dependency guards and [reviewable inventory](../../audits/working/0.104-timer-ownership/README.md) | Accepted 2026-08-18 |
| B2 | Native provider surface hard cut | Public timer API removal, native results/directives, typed registration custody and compile fallout | Core/facade tests, current-surface guard, targeted Clippy, PocketIC and [Wasm/performance evidence](../../audits/working/0.104-timer-ownership/b2-native-provider-surface.md) | Accepted 2026-08-18 |
| B3 | Domain-owned async-job recovery | Minimal stable state, exact retry, leases, takeover and deletion of shadow scheduling fields | State/property, interruption and [Wasm/performance evidence](../../audits/working/0.104-timer-ownership/b3-domain-async-job-recovery.md) | Accepted 2026-08-18 |
| B4 | Core fixed-owner reconstruction | Auth, capability-pruned automatic top-up and placement native schedules plus owner-specific retry/stop behavior | Targeted unit, snapshot, [Wasm/performance evidence](../../audits/working/0.104-timer-ownership/b4-domain-native-custody.md) and PocketIC recovery journeys | Accepted 2026-08-18 |
| B5 | Pool/lifecycle/snapshot propagation | Native pool custody, one watchdog, zero-delay lifecycle work, suspension/resume and [exact evidence](../../audits/working/0.104-timer-ownership/README.md#b5-pool-lifecycle-and-snapshot-completion) | Control-plane, lifecycle, snapshot, runtime-probe and role-pruned Wasm checks | Accepted 2026-08-18 |
| B6 | Native downstream adoption and lifecycle participant | Maintained migration guide, direct-provider fixture, paired compile-time participant, exact ordering and [rollback evidence](../../audits/working/0.104-timer-ownership/README.md#b6-native-adoption-and-lifecycle-participant) | Compile pass/fail, exact Candid/exports and PocketIC lifecycle checks | Accepted 2026-08-18 |
| B7 | Combined Canic/IcyDB qualification | One provider, one lifecycle export pair, complete shared inventories, progress, inactive reconstruction and [corrected-cause retry](../../audits/working/0.104-timer-ownership/README.md#b7-combined-canic-and-icydb-qualification) | Dependency, artifact and focused PocketIC composition proof | Accepted 2026-08-19 |
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
5. 0.107 closes fresh-Fleet preflight and runtime-admission gaps.
6. 0.108 closes replay-safe Coordinator-backed root operating funding.
7. 0.109 establishes Coordinator-owned Fleet-wide ingress admission with
   complete local enforcement projections.
8. 0.110 contracts Fleet runtime/build/operator/validation surfaces and adds
   stateful-retirement safety.
9. 0.111 qualifies one explicit stateful predecessor-to-successor adoption.
10. 0.112 adds bounded indexed estates and one same-Subnet cross-Fleet
    transfer.

Unnumbered ideas remain outside this path. Historical releases, archived
designs and deferred drafts retain their truthful old identities.

## Next Authorized Action

No 0.104 implementation work remains. Preserve every published tag and resume
0.105 B3 from exact predecessor `v0.104.2`; do not reopen timer provider,
lifecycle-export, watchdog or stable provider-state ownership.
