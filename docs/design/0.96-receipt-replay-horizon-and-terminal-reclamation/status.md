# 0.96 Receipt Replay Horizon And Terminal Reclamation - Status

Last updated: 2026-07-21

## Current State

0.96 is active. Audit-only Slice A is released as `v0.96.0`; the full
Canic-side receipt consumer and authority inventory is frozen. Released
`v0.96.1` measures the existing stable-capacity envelope and fixes two
independent totals-store defects without changing the public receipt contract,
replay behavior, timer ownership, configuration, dependencies, or Cargo
package versions. Open `0.96.2` hard-cuts application admission to one
immutable absolute replay deadline while leaving placement and terminal
cleanup on their existing owners.

The reviewed Toko signed-token ceiling now fixes Canic's maximum remaining
application replay window at 24 hours. Toko still has no receipt consumer,
per-mint action identity, recovery endpoint, ledger replay receipt, explicit
mint batch bound, or operation-rate limit. Terminal eligibility, reclamation,
and scheduling remain gated; Canic does not substitute arbitrary values for
those missing policies.

## Immutable Baseline

- Release anchor: `v0.96.1`.
- Source commit: `ba3368e5b090d72c38cb55b918f4bf3fefee6383`.
- Source tree: `dc2ba444b7670c140b63b8afb58cb0bb59fabd94`.
- Product-tree hash:
  `2ccba78c807cce74e3d281b710d865374c9032b34ee2c97b44696a483a8539ab`.
- Cargo.lock SHA-256:
  `fae17e29869b4828230ec5933bdadb55aa4af22362be9fb6d929f3d5d6781062`.
- Rust toolchain: `rustc 1.97.0 (2d8144b78 2026-07-07)`.
- Workspace package version at anchor: `0.96.1`.
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

## Released Slice B Capacity Evidence

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
  maximum. Released `v0.96.1` corrects the bound without changing stored
  fields or schema meaning.
- Ops now removes an exact totals row when reserved, committed, and pending
  values are all zero. Rollback and abort no longer retain empty stable rows;
  reads still project the same zero totals.
- Nonzero committed totals cannot be reclaimed with a terminal receipt. A
  maximum durable resource cardinality is therefore a required downstream
  capacity input.

## Open Slice C Replay Admission

The canonical report is
[0.96 replay deadline Slice C](../../audits/reports/2026-07/2026-07-21/0.96-replay-deadline-slice-c.md).

- `BeginReceiptBackedIntentInput` requires `replay_deadline_ns`; no optional
  field, default, overload, alias, or fallback reader remains.
- Exact retained lookup precedes time and capacity decisions. An absent
  operation closes at `now >= replay_deadline_ns`; an exact retained pending or
  terminal result remains observable at and after that boundary.
- The inclusive maximum remaining replay window is 24 hours. Longer absent
  operations return the typed `ReplayWindowTooLong` decision before capacity
  mutation.
- Application deadline metadata has one exact adjunct at allocation 46. The
  common primary at 43 remains canonical and unchanged; placement stores no
  application deadline and retains its acknowledgement owner at 45.
- The maximum metadata value encodes to its exact 124-byte bound. At 100,000
  rows its map consumes 442 ascending or 381 permuted Wasm pages. Primary,
  metadata, and maximum distinct totals still occupy 3,969 physical pages in
  the measured ascending high-water case because MemoryManager bucket
  rounding dominates the ten-page logical reduction from placement's index.
- Missing, orphaned, wrong-schema, wrong-identity, or placement-owned metadata
  fails closed. Stable snapshot helpers and the state manifest include the new
  allocation without creating a public export/import API.
- Settlement, terminal observation grace, eligibility, reclamation, timers,
  and resource accounting are unchanged in this slice.

## Finding Index

| Finding | Severity | State | Owner |
| --- | --- | --- | --- |
| `CANIC-096-RECEIPT-001` terminal application rows eventually exhaust new-operation admission | P1 | accepted for 0.96; not fixed | Canic receipt workflow/storage |
| `CANIC-096-RECEIPT-002` current rows contain no proof that deletion is replay-safe | P1 | deadline proof fixed in open 0.96.2; deletion remains disabled pending grace/conformance | Canic contract plus downstream immutable authorization |
| `CANIC-096-GATE-003` downstream mint action identity, recovery, bypass disposition, and numeric envelope are absent | gate | blocked outside Canic | Toko owner |
| `CANIC-096-CAPACITY-004` totals record declares 64 bytes but can validly encode to 69 | P1 | fixed in v0.96.1 | Canic stable intent storage |
| `CANIC-096-CAPACITY-005` abort and rollback retain empty totals rows | P2 | fixed in v0.96.1 | Canic intent storage ops |
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
- Replay-window policy and ops boundaries: 2 passed.
- Complete intent storage ops tests: 23 passed.
- Placement workflow tests: 20 passed.
- Runtime intent workflow tests: 6 passed.
- Replay metadata stable snapshot and state descriptor checks: passed.
- Public `canic` all-feature and Wasm fixture checks: passed.
- Focused PocketIC receipt adapter proof: 1 passed with real Wasm install,
  closed/overlong rejection, immutable deadline conflict, concurrent capacity
  admission, settlement, upgrade, and retained terminal replay.
- Broad workspace, release, general PocketIC, and downstream Toko suites were
  not run for this focused admission batch.

## Accepted Canic-Side Decisions

- Direct retained-row lookup precedes temporal and capacity rejection.
- An absent application operation is closed at `now >= replay_deadline_ns`.
- No minimum execution window is added; a future deadline is admissible up to
  the frozen maximum horizon.
- The maximum accepted remaining application replay window is exactly 24
  hours, inclusive.
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

## Decisions Still Required Before Terminal Reclamation

1. Toko's per-mint immutable action nonce/ID and its authentication vectors.
2. Exact operation-ID and payload-binding vectors covering the signed subject,
   user, project, ledger, action nonce, resolved effect payload, and deadline.
3. Exact terminal observation grace.
4. Explicit maximum mint batch size.
5. Maximum average and burst unique operation rate per canister.
6. Maximum unresolved pending and cleanup backlog.
7. Maximum durable resource-total cardinality per canister.
8. Ledger operation-ID idempotency/receipt design and targeted recovery flow.
9. Hard-cut deletion or integration of the parent-only stack-mint surface and
   unused project helper.
10. Final eligibility encoded size and the smallest physical
    reservation mechanism, using the measured existing-map baseline.
11. Cleanup batch, warning threshold, diagnostic surface, and final focused
    validation matrix.

## Next Action

Freeze terminal observation grace and the remaining downstream
capacity/recovery contract before defining eligibility, settlement
reservation, reclamation, or timer integration. Do not implement guessed
durations or broaden 0.96 into general intent, storage, timer, or downstream
cleanup.
