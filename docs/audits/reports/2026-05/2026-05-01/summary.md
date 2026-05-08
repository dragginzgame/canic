# Audit Summary - 2026-05-01

## Run Contexts

| Report | Type | Scope | Snapshot | Worktree | Status |
| --- | --- | --- | --- | --- | --- |
| `wasm-footprint.md` | Recurring system | Canic wasm footprint | `bc920f58` | clean | complete |

## Risk Index Summary

| Report | Risk | Readout |
| --- | ---: | --- |
| `wasm-footprint.md` | N/A | Size capture completed for `app`, `minimal`, `user_hub`, `user_shard`, `scale_hub`, `scale`, and `root`; first run of day so baseline deltas were `N/A`. |

## Method and Comparability Notes

- `wasm-footprint.md` used `Method V1`.
- It was marked comparable by method, but same-day baseline deltas were not
  available because this was the first `wasm-footprint` run for the day.

## Key Findings by Severity

### Warning

- `minimal` remained close to `app`, which means shared runtime cost is still
  the main baseline pressure signal.
- `root` remained a control-plane outlier because it carries root runtime plus
  the bootstrap `wasm_store.wasm.gz` artifact.

## Verification Rollup

| Report | PASS | PARTIAL | FAIL | Notes |
| --- | ---: | ---: | ---: | --- |
| `wasm-footprint.md` | 6 | 1 | 0 | Baseline delta was partial because this was the first run of the day. |

## Follow-up Actions

No immediate action was required by the May 1 wasm footprint report. Continue
tracking the shared-runtime floor in future wasm footprint runs.
