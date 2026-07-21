# 0.96 Receipt Replay Horizon And Terminal Reclamation - Status

Last updated: 2026-07-21

## Current State

0.96 is active. Audit-only Slice A is released as `v0.96.0`; the full
Canic-side receipt consumer and authority inventory is frozen. Released
`v0.96.1` measures the existing stable-capacity envelope and fixes two
independent totals-store defects. Released `v0.96.2` hard-cuts application
admission to one immutable absolute replay deadline. Released `v0.96.3`
corrects the resulting lifecycle reconciliation to one ordered pass. Released
`v0.96.4` freezes a 24-hour terminal observation grace, provisions the measured
terminal-index envelope at admission, and persists exact terminal eligibility.
Released `v0.96.5` adds its constant-time capacity and earliest-eligibility
projection. Open `0.96.6` hard-cuts the inherited ceiling to 1,000 records and
enables exact bounded terminal reclamation through the existing intent timer.

The reviewed Toko signed-token ceiling now fixes Canic's maximum remaining
application replay window at 24 hours. Toko still has no receipt consumer,
per-mint action identity, recovery endpoint, ledger replay receipt, explicit
mint batch bound, or operation-rate limit. Those are Toko adoption
requirements, not Canic product gates. Canic owns its generic capacity,
retention, scheduling, reclamation, and diagnostics; consumers conform
afterward.

## Immutable Baseline

- Release anchor: `v0.96.5`.
- Source commit: `ee32d97fa241af20d7c330b43f8659157e072427`.
- Source tree: `50ae33d50dcf9fa003bacb63119ded3c9b858c20`.
- Product-tree hash:
  `ea7a222a1de95af72970ddb2944bf270bc789e9580addd3fd124d5c59d0dafb3`.
- Cargo.lock SHA-256:
  `7de62668fe974e6819ea3b532357e255bc8d7e72735cc6973fc30bcf3b5b45fc`.
- Rust toolchain: `rustc 1.97.0 (2d8144b78 2026-07-07)`.
- Workspace package version at anchor: `0.96.5`.
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
- Nonzero committed totals cannot be reclaimed with a terminal receipt. This
  identified the durable resource-cardinality bound later reclassified in
  open `0.96.6` as a Canic-owned admission responsibility.

## Released Slice C Replay Admission

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

## Released 0.96.3 Reconciliation Correction

The canonical report is
[0.96.3 receipt reconciliation correction](../../audits/reports/2026-07/2026-07-21/0.96-receipt-reconciliation-0.96.3.md).

- The released 0.96.2 lifecycle path scanned both canonical maps but also
  performed one cross-map stable-tree point lookup for every primary and every
  application metadata row. At the 100,000-row all-application limit, that was
  up to 200,000 point lookups in addition to the two scans.
- `reconcile_receipt_indexes` now merge-validates the ordered primary and
  application-adjunct maps in one pass over each. It detects missing, orphaned,
  wrong-schema, wrong-identity, and Canic-owned metadata without lookup
  amplification.
- The placement acknowledgement index is cleared and rebuilt only after the
  complete canonical pass succeeds. Any application contradiction preserves
  the prior derived index and returns the typed ops cause under an accurate
  receipt-index lifecycle diagnostic.
- The old placement-only method name is removed rather than retained as an
  alias. Public APIs, stable data, timers, settlement, accounting, and
  downstream policy remain unchanged.

## Released 0.96.4 Terminal Eligibility And Capacity

The canonical report is
[0.96.4 terminal eligibility and settlement capacity](../../audits/reports/2026-07/2026-07-21/0.96-terminal-eligibility-0.96.4.md).

- Terminal observation grace is exactly 24 hours, matching the maximum
  admitted replay horizon. Eligibility is
  `max(replay_deadline_ns, terminal_timestamp_ns + grace)` with checked
  arithmetic before mutation.
- Allocation 47 is the sole ordered application terminal-eligibility
  authority. Its 40-byte key orders by eligibility then operation ID; its
  exact 229-byte maximum value binds schema, identity, payload, and terminal
  revision.
- Application admission provisions the pinned B-tree's maximum live-node
  envelope before primary or resource mutation. The bound is 726 pages at
  100,000 rows, exactly matching the measured ascending high-water case.
- Application settlement inserts exact eligibility before changing its
  primary or resource aggregate. Overflow, duplicate eligibility, or an
  unavailable reservation fails closed without settlement.
- Lifecycle reconciliation validates the complete eligibility projection
  before clearing or rebuilding placement acknowledgement state. Pending
  application rows require no terminal entry; terminal rows require exactly
  one.
- The eligibility map measures 726/626 pages for ascending/permuted order.
  Primary, replay metadata, eligibility, and maximum distinct totals consume
  4,737 physical pages, or 296.0625 MiB, through the pinned base
  `MemoryManager`.
- No timer or removal path consumes the index in this patch. Terminal records
  remain retained until bounded reclamation and downstream conformance are
  proven.

## Released 0.96.5 Receipt Capacity Projection

The canonical report is
[0.96.5 receipt capacity projection](../../audits/reports/2026-07/2026-07-21/0.96-receipt-capacity-projection-0.96.5.md).

- The application replay map's maintained B-tree length is the total record
  count and reserved terminal-slot count. The eligibility map's maintained
  length is the terminal count; their checked difference is the pending count.
- No primary scan, stable counter cell, extra allocation, duplicated metadata,
  or repair flow is added. Avoiding another `MemoryManager` bucket preserves the
  measured 4,737-page, 296.0625-MiB high-water subtotal.
- One internal view reports the fixed record limit, remaining logical
  headroom, actual reserved eligibility pages, and the exact first-key
  eligibility deadline.
- The first eligibility row is validated against its schema, identity,
  application primary, replay metadata, binding, revision, and recomputed
  deadline before projection.
- Lifecycle invokes the projection only after complete index reconciliation.
  No timer, public diagnostic surface, or removal path is enabled.

## Open 0.96.6 Bounded Terminal Reclamation

The canonical report is
[0.96.6 bounded terminal reclamation](../../audits/reports/2026-07/2026-07-21/0.96-terminal-reclamation-0.96.6.md).

- The shared primary ceiling is hard-cut from 100,000 to 1,000 records. Exact
  admission accepts 1,000 and rejects row 1,001; no configuration override or
  old ceiling remains.
- The pinned four-allocation application envelope at 1,000 rows is 513
  physical Wasm pages, or 32.0625 MiB, in the ascending high-water case. The
  terminal eligibility reservation is eight pages.
- Capacity projection counts application and Canic-owned rows against the
  shared primary ceiling while deriving application pending and terminal
  counts from the replay and eligibility indexes.
- The existing `intent_cleanup:run` timer owns finite local expiry and terminal
  application reclamation under one shared 32-item batch. Application due work
  consumes the prefix first; remaining capacity serves local expiry.
- Ops validates the complete due application prefix before mutation and then
  removes only exact eligibility, replay-metadata, and terminal-primary rows.
  Resource totals remain unchanged.
- A contradictory earliest entry removes nothing, skips nothing, and returns
  an invariant result that stops the 0.95 timer. Guarded recent-failure
  diagnostics retain the exact typed cause.
- The real-Wasm PocketIC path advances beyond grace, observes timer-owned
  committed and rolled-back removal, retains pending work, rejects the expired
  authorization after removal, and preserves committed resource accounting.

## Finding Index

| Finding | Severity | State | Owner |
| --- | --- | --- | --- |
| `CANIC-096-RECEIPT-001` terminal application rows eventually exhaust new-operation admission | P1 | fixed in open 0.96.6 by bounded exact reclamation | Canic receipt workflow/storage |
| `CANIC-096-RECEIPT-002` current rows contain no proof that deletion is replay-safe | P1 | fixed in open 0.96.6 by deadline/grace eligibility and closed post-removal admission | Canic receipt workflow/storage |
| `CANIC-096-GATE-003` downstream mint action identity, recovery, bypass disposition, and numeric envelope are absent | adoption | does not block Canic; required before Toko consumes the completed contract | Toko owner |
| `CANIC-096-CAPACITY-004` totals record declares 64 bytes but can validly encode to 69 | P1 | fixed in v0.96.1 | Canic stable intent storage |
| `CANIC-096-CAPACITY-005` abort and rollback retain empty totals rows | P2 | fixed in v0.96.1 | Canic intent storage ops |
| `CANIC-096-GATE-006` committed resource-total cardinality has no finite Canic bound | P1 | open; receipt cleanup must not move exhaustion into the totals map | Canic intent admission |
| `CANIC-096-RECONCILE-007` lifecycle receipt reconciliation amplifies ordered scans with per-row cross-map point lookups | P2 | fixed in v0.96.3 | Canic receipt storage ops/lifecycle |
| `CANIC-096-CAPACITY-008` admitted pending application work has no reserved terminal-index capacity | P1 | fixed in v0.96.4 | Canic receipt storage ops |

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
- Open 0.96.3 reconciliation matrix: 24 receipt storage-op tests and 68 runtime
  workflow tests passed; five application-adjunct contradiction classes
  preserve the placement index on failure.
- Focused 0.96.3 PocketIC lifecycle proof: 1 passed through real Wasm install,
  application receipt persistence, post-upgrade reconciliation, and retained
  terminal replay.
- Released 0.96.4 intent storage ops: 25 passed, including exact eligibility,
  overflow non-mutation, logical-saturation settlement, and lifecycle
  fail-closed behavior.
- Released 0.96.4 stable intent snapshots and malformed-key/capacity checks: 8
  passed.
- Released 0.96.4 state contract, allocation inventory, and authority guard: 16
  passed.
- Released 0.96.4 complete role-contract allocation projections: 14 passed.
- Released 0.96.4 host state-manifest projections: 21 passed; strict all-target
  `canic-host` Clippy passed.
- Explicit updated 100,000-row capacity probe: 1 passed with 726/626
  eligibility pages and a 4,737-page managed high-water subtotal.
- Strict targeted `canic-core` all-feature lib/tests Clippy: passed.
- Targeted `canic-core` all-feature Wasm check: passed.
- Focused 0.96.4 PocketIC receipt lifecycle proof: 1 passed through real Wasm
  install, concurrent capacity admission, settlement, post-upgrade
  reconstruction, and retained replay; its strict targeted Clippy also passed.
- Final layering guard, changelog governance, formatting, and diff hygiene:
  passed.
- Released 0.96.5 intent storage ops: 26 passed, including maintained map-length
  counts, checked pending derivation, physical reservation, logical headroom,
  and exact first eligibility.
- Released 0.96.5 runtime workflow tests: 68 passed; strict targeted `canic-core`
  all-feature lib/tests Clippy passed.
- Released 0.96.5 receipt authority inventory: 2 passed; all-feature Wasm check
  passed; focused PocketIC admission, settlement, upgrade projection, and
  retained replay: 1 passed with strict targeted Clippy.
- Released 0.96.5 layering, changelog governance, formatting, and diff hygiene:
  passed.
- Open 0.96.6 intent storage ops: 30 passed, including the exact 1,000-row
  boundary, shared-primary headroom, bounded reclamation, preserved totals,
  post-removal rejection, and poisoned-prefix non-mutation.
- Open 0.96.6 runtime intent workflow: 7 passed, including the shared 32-item
  cleanup budget and protected invariant diagnostics.
- Open 0.96.6 stable intent storage: 8 passed; explicit 1,000-row capacity
  probe: 1 passed at 513 managed physical pages.
- Open 0.96.6 PocketIC receipt lifecycle: 1 passed through real Wasm timer
  reclamation, pending retention, closed post-removal replay, and preserved
  committed accounting.

## Accepted Canic-Side Decisions

- Direct retained-row lookup precedes temporal and capacity rejection.
- An absent application operation is closed at `now >= replay_deadline_ns`.
- No minimum execution window is added; a future deadline is admissible up to
  the frozen maximum horizon.
- The maximum accepted remaining application replay window is exactly 24
  hours, inclusive.
- Terminal observation grace is exactly 24 hours and is not caller input.
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
- The shared receipt ceiling is 1,000 records. The existing intent cleanup
  owner processes one shared 32-item application/local batch and stops on a
  contradictory earliest row.

## Decisions Still Required Before Canic Closeout

1. Bound durable resource-total cardinality in Canic so receipt reclamation
   cannot move exhaustion into the aggregate map.
2. Freeze capacity warning thresholds and whether the maintained internal
   projection joins the existing guarded runtime-status contract.
3. Run the final focused and cumulative validation matrix.

Toko's action identity, binding vectors, batch/rate envelope, ledger
idempotency, recovery, and entrypoint cleanup remain required before Toko
adopts the API. They do not gate Canic's generic reclamation implementation.

## Next Action

Finish the Canic-owned Slice E boundary by capping durable resource-total
cardinality and surfacing the maintained capacity projection through the
existing guarded runtime-status authority. Then run the final 0.96 validation.
Toko adoption evidence follows the completed Canic contract and does not gate
product reclamation. Do not broaden 0.96 into unrelated intent, storage,
timer, ICP-refill, or downstream cleanup.
