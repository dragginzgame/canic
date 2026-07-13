# 0.90 Status: Receipt-Backed Intent Reconciliation

Last updated: 2026-07-13

## Current State

`0.90.2` publishes the complete hard-cut Canic primitive, focused
downstream-adapter conformance proof, and handoff. The Canic-owned line is
closed; Toko developers can now adopt the API from the published release.

The published Canic core primitive uses
the existing 32-byte `OperationId`, opaque payload bindings, bounded terminal
evidence, exact-key stable storage, idempotent begin-or-load, and non-awaiting
compare-and-set settlement. New state uses one operation map on memory ID 43;
its stable map header is the exact admission count. Existing local intent
records, totals, pending index, metadata, and encodings remain on IDs
39 through 42 unchanged.

The persisted resource aggregate is the single accounting authority.
Receipt-backed rows contribute to its `pending_count`, but never enter the
local TTL index or change the metadata's expirable-pending count, so they cannot
start or retain cleanup. This reuses existing metadata and avoids another
aggregate migration.

`CallBuilder::with_intent`, `IntentReservation`, and `IntentKey` remain
hard-cut. The focused intent fixture now reports exact begin and settlement
decisions instead of collapsing conflicts to an optional record. It proves
creation, replay, changed binding, shared capacity, stale revision, missing
operation, commit, rollback, contradictory evidence, released rollback
capacity, and pending-state upgrade recovery through the public facade.

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

### Slice B - Canic adapter conformance and handoff

- [x] Expose exact test-only begin decisions through the public facade.
- [x] Expose exact test-only settlement decisions and evidence source.
- [x] Prove changed payload/resource, capacity, missing operation, stale
  revision, and settlement binding conflicts without mutation.
- [x] Prove committed and rolled-back settlement plus exact terminal replay.
- [x] Prove contradictory evidence preserves the first terminal state.
- [x] Prove durable rollback releases reservation capacity.
- [x] Preserve a pending operation through a same-Wasm upgrade.
- [x] Document the downstream adapter and evidence-validation contract.

### Downstream Toko adoption

- [ ] Define Toko-owned mint request, identity, receipt, and outcome types.
- [ ] Add caller-scoped applied and durable no-effect ledger receipts.
- [ ] Validate Toko evidence before constructing Canic terminal evidence.
- [ ] Integrate one-call happy-path settlement and bounded targeted recovery.
- [ ] Retire any co-authoritative mint settlement replay row.
- [ ] Complete focused PocketIC race, recovery, capacity, upgrade, and
  performance proof.

This checklist is owned by the Toko repository after the compatible Canic
release is published. It is not implemented by Canic agents and does not block
publication of the Canic conformance slice.

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
  overlapping call plus the complete generic adapter decisions listed in
  Slice B.
- The exact-outcome fast intent-authority Wasm is 1,268,888 bytes. It is 87,145
  bytes (7.4%) above the 1,181,743-byte local-only fixture and 16,615 bytes
  above the published receipt fixture. That increase is confined to test
  Candid outcome types and branches. Production Canic code adds no call, scan,
  timer, or cleanup entry; begin and settlement each use one exact-key row and
  the existing exact resource aggregate.
- Warning-denied Clippy passes for core, facade, intent authority, and the
  focused integration test. Layering, metrics ordering, changelog governance,
  retired-surface search, formatting, and diff hygiene pass.
- Full workspace, broad PocketIC, deployment, and release suites were not run.

## Next Action

The Canic line is closed at published `v0.90.2`; the focused closeout audit
passes without a correction. Toko developers now own the downstream mint
adapter and its domain tests. Do not widen Canic into a generic effect,
receipt, resolver, or recovery framework.

Multi-step claim orchestration is deferred to a separately accepted future
design. No numbered release line is reserved for it.
