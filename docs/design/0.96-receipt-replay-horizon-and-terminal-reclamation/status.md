# 0.96 Receipt Replay Horizon And Terminal Reclamation - Status

Last updated: 2026-07-21

## Current State

0.96 is active. Audit-only Slice A is released as `v0.96.0`; the full
Canic-side receipt consumer and authority inventory is frozen. Open `0.96.1`
measures the existing stable-capacity envelope and fixes two independent
totals-store defects without changing the public receipt contract, replay
behavior, timer ownership, configuration, dependencies, or Cargo package
versions.

Receipt deadline, eligibility, reclamation, and timer implementation is not yet
accepted. A read-only Toko trace establishes the current complete mint call
graph, signed 24-hour token-expiry ceiling, and absence of any receipt API
consumer or old receipt row. It also proves that Toko has no per-mint action
identity, recovery endpoint, ledger replay receipt, explicit mint batch bound,
or operation-rate limit. Canic will not substitute arbitrary values for those
missing policies.

## Immutable Baseline

- Release anchor: `v0.96.0`.
- Source commit: `ea80087951835d7f808847d9c6b9f37e92c2e7a1`.
- Source tree: `d025bd9e58444447ba21eac162d35c38fb2bb78d`.
- Product-tree hash:
  `83b19bc26ca5f20454fbe00b5520cd0d06eca81970d9bbacb767c315acd17b5a`.
- Cargo.lock SHA-256:
  `a0aeda74ecafd0d936989dab832f715306f647df5546a61f299198dee66bb4c4`.
- Rust toolchain: `rustc 1.97.0 (2d8144b78 2026-07-07)`.
- Workspace package version at anchor: `0.96.0`.
- Read-only downstream snapshot: Toko
  `485f586184651d67739b8e9c0ec489fea6a16b3a`, tree
  `e3c5a9317b6d2017119ab88ee7009663b3f1a1c8`, clean worktree.

## Released Slice A Evidence

The canonical report is
[0.96 receipt contract Slice A](../../audits/reports/2026-07/2026-07-21/0.96-receipt-contract-slice-a.md).

- One common stable primary receipt map exists at allocation 43.
- One placement-only acknowledgement index exists at allocation 45.
- Storage mutation is owned by `ops::storage::intent`; endpoint and public API
  layers do not bypass it.
- The public application facade has exactly three operations: begin/load,
  direct load, and compare-and-set settlement. It has no removal operation.
- The only direct public-facade consumer in this repository is the
  `intent_authority` PocketIC fixture.
- Toko currently consumes Canic `0.71.3` and contains no receipt API reference,
  so its current deployment state cannot contain Canic application receipt
  rows. No compatibility reader or migration is required for its adoption.
- Toko has one maintained authenticated project mint endpoint and one
  parent-only batch ledger effect. A second parent-only stack-mint Candid
  surface survives without a maintained caller and must be removed or covered
  downstream before conformance.
- Production placement uses private Canic-owned admission, settlement, and
  acknowledgement-backed removal and remains outside application
  reclamation.
- Application rows have no deadline and no cleanup owner. Pending and terminal
  application rows are retained until the 100,000-row ceiling refuses new
  identities.
- A permanent source inventory guard rejects unreviewed consumer, ops, store,
  or stable-allocation ownership changes.

## Open Slice B Capacity Evidence

The canonical report is
[0.96 receipt capacity Slice B](../../audits/reports/2026-07/2026-07-21/0.96-receipt-capacity-slice-b.md).

- Maximum current pending and terminal primary values encode to 441 and 617
  bytes respectively; the primary map retains its 1,024-byte bound.
- At 100,000 rows, allocation 43 consumes 2,707 ascending or 1,899
  deterministic-permuted Wasm pages.
- The placement acknowledgement allocation consumes 452/317 pages and
  100,000 distinct maximum totals rows consume 545/469 pages.
- The three-allocation receipt subtotal consumes 3,969 physical Wasm pages,
  or 248.0625 MiB, in the measured ascending high-water case after base
  MemoryManager bucket rounding; unrelated canister allocations are excluded.
- The totals record's previous 64-byte bound did not cover its valid 69-byte
  maximum. Open `.1` corrects the bound without changing stored fields or
  schema meaning.
- Ops now removes an exact totals row when reserved, committed, and pending
  values are all zero. Rollback and abort no longer retain empty stable rows;
  reads still project the same zero totals.
- Nonzero committed totals cannot be reclaimed with a terminal receipt. A
  maximum durable resource cardinality is therefore a required downstream
  capacity input.

## Finding Index

| Finding | Severity | State | Owner |
| --- | --- | --- | --- |
| `CANIC-096-RECEIPT-001` terminal application rows eventually exhaust new-operation admission | P1 | accepted for 0.96; not fixed | Canic receipt workflow/storage |
| `CANIC-096-RECEIPT-002` current rows contain no proof that deletion is replay-safe | P1 | accepted for 0.96; not fixed | Canic contract plus downstream immutable authorization |
| `CANIC-096-GATE-003` downstream mint action identity, recovery, bypass disposition, and numeric envelope are absent | gate | blocked outside Canic | Toko owner |
| `CANIC-096-CAPACITY-004` totals record declares 64 bytes but can validly encode to 69 | P1 | fixed in open 0.96.1 | Canic stable intent storage |
| `CANIC-096-CAPACITY-005` abort and rollback retain empty totals rows | P2 | fixed in open 0.96.1 | Canic intent storage ops |
| `CANIC-096-GATE-006` committed resource-total cardinality has no finite downstream bound | gate | blocked outside Canic | Toko owner plus Canic admission |

No other product finding is admitted without a design amendment and direct
receipt-path evidence.

## Focused Validation

- Receipt source/stable inventory guard: 2 passed.
- Placement directory workflow tests: 12 passed.
- Receipt-backed intent unit/snapshot tests: 7 passed.
- Changelog governance: 1 passed.
- Strict targeted `canic-core` lib/inventory Clippy: passed.
- Layering guards: passed.
- Formatting and diff hygiene: passed.
- Exact totals-bound regression: 1 passed.
- Intent storage ops tests, including zero-total compaction: 21 passed.
- Intent stable-storage and capacity tests: 8 passed, 1 explicit capacity
  probe ignored by ordinary runs.
- Explicit 100,000-row capacity probe: 1 passed with the measurements above.
- Strict `canic-core` lib/tests Clippy: passed.
- Layering guards and changelog governance: passed.
- Broad workspace, release, PocketIC, and downstream Toko suites were not run
  for this focused stable-capacity batch.

## Accepted Canic-Side Decisions

- Direct retained-row lookup precedes temporal and capacity rejection.
- An absent application operation is closed at `now >= replay_deadline_ns`.
- No minimum execution window is added; a future deadline is admissible up to
  the frozen maximum horizon.
- Existing pending and terminal rows remain observable while retained even
  after their deadline.
- Pending state is never deleted because of age.
- Terminal eligibility is exact and ordered; reclamation changes no resource
  aggregate.
- Placement retains its separate acknowledgement-owned cleanup.
- The hard cut has no optional deadline, default, alias, overload, fallback
  reader, or compatibility shim.
- The common primary receipt map remains canonical. Any application replay
  metadata is an exact adjunct, not a second primary authority.
- Capacity work must use the smallest measured dedicated mechanism. It does
  not authorize a generic allocator or transaction framework.
- No public export/import API is added; existing stable snapshot helpers are
  test-only.
- The exact maximum totals encoding is 69 bytes and its stable declaration
  must cover that value.
- An all-zero resource total is represented by absence; nonzero committed
  totals remain canonical state.

## Decisions Still Required Before Product Work

1. Toko's per-mint immutable action nonce/ID and its authentication vectors.
2. Exact operation-ID and payload-binding vectors covering the signed subject,
   user, project, ledger, action nonce, resolved effect payload, and deadline.
3. Confirmation that signed delegated-token expiry is the replay deadline;
   current source proves it is at most 24 hours.
4. Exact terminal observation grace.
5. Explicit maximum mint batch size.
6. Maximum average and burst unique operation rate per canister.
7. Maximum unresolved pending and cleanup backlog.
8. Maximum durable resource-total cardinality per canister.
9. Ledger operation-ID idempotency/receipt design and targeted recovery flow.
10. Hard-cut deletion or integration of the parent-only stack-mint surface and
   unused project helper.
11. Final eligibility/metadata encoded sizes and the smallest physical
    reservation mechanism, using the measured existing-map baseline.
12. Cleanup batch, warning threshold, diagnostic surface, and final focused
    validation matrix.

## Next Action

Complete open `0.96.1` validation. Then obtain and freeze the remaining
downstream contract and capacity decisions before defining the eligibility
record or editing a public receipt input or timer. Do not implement guessed
durations or broaden 0.96 into general intent, storage, timer, or downstream
cleanup.
