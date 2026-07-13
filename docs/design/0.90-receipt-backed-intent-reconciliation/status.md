# 0.90 Status: Receipt-Backed Intent Reconciliation

Last updated: 2026-07-13

## Current State

`0.90.0` published the hard-cut MVP design without runtime changes. Slice A is
complete locally and ready for review.

The Canic core primitive is implemented locally: receipt-backed operations use
the existing 32-byte `OperationId`, opaque payload bindings, bounded terminal
evidence, exact-key stable storage, idempotent begin-or-load, and non-awaiting
compare-and-set settlement. New state uses one operation map on memory ID 43;
its stable map header is the exact admission count. Existing local intent
records, totals, pending index, metadata, and encodings remain on IDs
39 through 42 unchanged.

The persisted resource aggregate is now the single accounting authority.
Receipt-backed rows contribute to that aggregate but never enter the local TTL
index or change its maintained pending count, so they cannot start or retain
cleanup. This reuses existing metadata and removes the migration proposed by
the design draft.

`CallBuilder::with_intent`, `IntentReservation`, and `IntentKey` are hard-cut.
The maintained intent race fixture now performs explicit local begin, call,
and commit-or-rollback operations. The same focused fixture exercises the
public receipt-backed facade, preserves a pending row through a same-Wasm
upgrade, and settles it idempotently after upgrade.

## Checklist

### Slice A - Minimal Canic primitive

- [x] Reuse `OperationId` with one receipt-backed namespace per canister.
- [x] Add opaque payload binding and bounded terminal evidence.
- [x] Add one receipt-backed operation map on ID 43 and use its maintained
  length for admission.
- [x] Add exact begin-or-load, load, and settlement operations.
- [x] Reject changed binding, resource, or quantity without mutation.
- [x] Keep terminal settlement idempotent and contradictory evidence blocking.
- [x] Share the existing resource aggregate without changing its stable shape.
- [x] Keep receipt-backed rows out of TTL cleanup and timer-idle metadata.
- [x] Enforce a maintained fixed record limit without scanning.
- [x] Hard-cut the call-builder intent shortcut and migrate its maintained
  fixture.
- [x] Expose direct local and receipt-backed operations through the core and
  facade APIs.
- [x] Complete focused stable-upgrade and public-facade behavior proof.
- [x] Record the reviewed Slice A performance and Wasm-size evidence.

### Slice B - Toko mint proof

- [ ] Define Toko-owned mint request, identity, receipt, and outcome types.
- [ ] Add caller-scoped applied and durable no-effect ledger receipts.
- [ ] Validate Toko evidence before constructing Canic terminal evidence.
- [ ] Integrate one-call happy-path settlement and bounded targeted recovery.
- [ ] Retire any co-authoritative mint settlement replay row.
- [ ] Complete focused PocketIC race, recovery, capacity, upgrade, and
  performance proof.

## Validation

- Twelve focused intent-storage tests pass for idempotent begin, conflicts,
  shared capacity, store capacity, no TTL entry, revision CAS, commit,
  rollback, terminal replay, contradictory evidence, unsupported binding and
  evidence schemas, and exact aggregate deltas.
- Two focused stable intent snapshot tests pass for old and new allocations.
- Fourteen role-contract tests pass with ID 43 in the canonical runtime
  intent allocation and no collisions or owner-range violations.
- Focused state-contract descriptor tests pass for every runtime intent domain
  and declared core memory ID.
- The focused PocketIC intent test proves explicit local capacity under an
  overlapping call, receipt begin replay, shared capacity rejection, pending
  state across upgrade, settlement, and terminal replay.
- The receipt-exercising fast intent-authority Wasm is 1,252,273 bytes. The
  local-only Slice A fixture was 1,181,743 bytes, so exercising the new facade
  adds 70,530 bytes (6.0%). The implementation adds no call, scan, timer, or
  cleanup entry; begin and settlement each use one exact-key row and the
  existing exact resource aggregate.
- Warning-denied Clippy passes for core, facade, intent authority, and the
  focused integration test. Layering, metrics ordering, changelog governance,
  retired-surface search, formatting, and diff hygiene pass.
- Full workspace, broad PocketIC, deployment, and release suites were not run.

## Next Action

Review and push Slice A as one hard-cut batch. Slice B then integrates only the
Toko mint proof; do not widen Canic into a generic effect, receipt, resolver,
or recovery framework.
