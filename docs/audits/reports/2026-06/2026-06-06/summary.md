# Audit Summary - 2026-06-06

## Run Contexts

| Report | Type | Scope | Status |
| ---- | ---- | ---- | ---- |
| `workflow-purity.md` | Recurring system | workflow orchestration ownership, replay/cost/intent boundaries, recovery, and module pressure | FAIL |

## Risk Index Summary

| Report | Risk | Notes |
| ---- | ----: | ---- |
| `workflow-purity.md` | 6 / 10 | Bounded lower-layer ownership leaks found in workflow record carriers, codecs, and pool module pressure. |

## Method / Comparability Notes

- `workflow-purity.md` uses `workflow-purity-v2`.
- The run is non-comparable with `2026-05-16` because the method now covers
  replay receipts, cost guards, durable intents, management-effect recovery,
  and module pressure.

## Key Findings

### Critical

- None.

### High

- None.

### Medium

- `workflow/ic/icp_refill/mod.rs` carries persisted `IcpRefillRecord` values
  through the workflow state machine.
- `workflow/pool/mod.rs` and `workflow/rpc/capability/*` own Candid
  encode/decode and hash helpers that should move to ops or a lower codec
  boundary.

### Low

- `workflow/pool/mod.rs` reads `CanisterRecord` directly for recycle metadata.
- `workflow/pool/mod.rs` is a high-pressure workflow hub after recent replay
  and pending-reset slices.

## Verification Readout Rollup

| Report | PASS | FAIL | BLOCKED |
| ---- | ----: | ----: | ----: |
| `workflow-purity.md` | 9 | 3 | 0 |

## Follow-up Actions

- Move ICP refill record transition carriers behind `ops::storage::icp_refill`.
- Replace pool recycle's direct `CanisterRecord` dependency with an ops-owned
  projection/helper.
- Move pool replay response codecs and capability proof/grant codecs out of
  workflow.
