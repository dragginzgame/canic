# 0.96 Receipt Replay Horizon And Terminal Reclamation - Status

Last updated: 2026-07-21

## Current State

0.96 is active for audit-only Slice A against immutable `v0.95.10`. The full
Canic-side receipt consumer and authority inventory is frozen. Product
behavior, public APIs, stable state, timers, configuration, dependencies, and
Cargo package versions are unchanged.

Product implementation is not yet accepted. A read-only Toko trace establishes
the current complete mint call graph, signed 24-hour token-expiry ceiling, and
absence of any receipt API consumer or old receipt row. It also proves that
Toko has no per-mint action identity, recovery endpoint, ledger replay receipt,
explicit mint batch bound, or operation-rate limit. Canic will not substitute
arbitrary values for those missing policies.

## Immutable Baseline

- Release anchor: `v0.95.10`.
- Source commit: `a3ad7ff37996ceba2860a7b3fd56ca78d529199b`.
- Source tree: `4957308bb5baa32f0fa87af9d20d4d70ad6693e3`.
- Product-tree hash:
  `efd454797e24935434c4f7725494284efa495256fb2c8c1b89ce0be8c39a73a1`.
- Cargo.lock SHA-256:
  `4cec6ab4e0295e690d00c27f62f7507d58d224bde76d489a7ff1327389c38b29`.
- Rust toolchain: `rustc 1.97.0 (2d8144b78 2026-07-07)`.
- Workspace package version at anchor: `0.95.10`.
- Read-only downstream snapshot: Toko
  `485f586184651d67739b8e9c0ec489fea6a16b3a`, tree
  `e3c5a9317b6d2017119ab88ee7009663b3f1a1c8`, clean worktree.

## Slice A Evidence

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

## Finding Index

| Finding | Severity | State | Owner |
| --- | --- | --- | --- |
| `CANIC-096-RECEIPT-001` terminal application rows eventually exhaust new-operation admission | P1 | accepted for 0.96; not fixed | Canic receipt workflow/storage |
| `CANIC-096-RECEIPT-002` current rows contain no proof that deletion is replay-safe | P1 | accepted for 0.96; not fixed | Canic contract plus downstream immutable authorization |
| `CANIC-096-GATE-003` downstream mint action identity, recovery, bypass disposition, and numeric envelope are absent | gate | blocked outside Canic | Toko owner |

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
- Broad workspace, release, PocketIC, and downstream Toko suites were not run
  for this audit-only, behavior-neutral batch.

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
8. Ledger operation-ID idempotency/receipt design and targeted recovery flow.
9. Hard-cut deletion or integration of the parent-only stack-mint surface and
   unused project helper.
10. Measured stable encoded/allocator/growth envelope and the smallest
    reservation mechanism.
11. Cleanup batch, warning threshold, diagnostic surface, and final focused
    validation matrix.

## Next Action

Obtain and freeze the eleven downstream/capacity decisions above. Then begin
Slice B measurement before editing a public input, stable record, or timer. Do
not implement guessed durations or broaden 0.96 into general intent, storage,
timer, or downstream cleanup.
