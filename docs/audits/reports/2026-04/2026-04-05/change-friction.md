# Change Friction Audit - 2026-04-05

## Report Preamble

- Scope: `crates/canic-core/src/**`, `crates/canic-testkit/src/**`, `crates/canic-testing-internal/src/**`, `crates/canic-tests/tests/**`, and recent `0.24.x` feature slices
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-10/change-friction.md`
- Code snapshot identifier: `c027b8df`
- Method tag/version: `Method V4.1`
- Comparability status: `partially comparable` (the CAF method is stable, but the current slice sample is drawn from the `0.24.x` demo/test cleanup line rather than the older replay/capability-heavy March sample)
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-05T16:34:13Z`
- Branch: `main`
- Worktree: `dirty`

## Executive Summary

- Risk Score: `3 / 10`
- Delta summary: routine feature friction is lower than the March baseline. The current `0.24.x` sample is dominated by testkit/testing-boundary cleanup, and the routine feature slices are smaller and more contained than the older replay/auth/control-plane slices.
- Largest remaining pressure: the internal/public test harness seam around `canic-testing-internal::pic` and `canic-testkit::pic`.
- Cross-layer leakage status: `none observed`.
- Follow-up required: `no` for immediate architecture health; `yes` for keeping generated/test noise from inflating future friction signals.

## Baseline Delta Summary

| Metric | Previous | Current | Delta |
| --- | ---: | ---: | ---: |
| Velocity Risk Index | 5 | 3 | -2 |
| Cross-layer leakage crossings | 0 | 0 | 0 |
| Avg files touched per feature slice | 27.50 | 19.25 | -8.25 |
| p95 files touched | 49 | 35 | -14 |
| Top gravity-well fan-in | 11 | 4 | -7 |

Notes:
- Current average/p95 use the sampled `0.24.x` slice set below.
- Current top gravity-well fan-in is measured as repeat slice-touch recurrence in the sampled window; the March run used a broader module import-fan-in scan, so this delta is directional.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Feature slices sampled from recent commits | PASS | sampled `0.24.4`, `0.24.5`, `0.24.6`, and `0.24.8` line commits |
| Revised CAF computed per slice | PASS | routine slices now land at CAF `2`, `3`, and `6` |
| Release sweeps separated from routine slices | PASS | `ede4e597` is treated as `release_sweep` and excluded from routine-friction interpretation |
| Slice density computed | PASS | density measured as `files/subsystems` per sampled slice |
| Blast radius measured | PASS | sampled slices range from `9` to `35` files |
| Boundary leakage reviewed | PASS | no new hard layering bypass was surfaced in the current sampled slice set |
| Friction amplification drivers identified | PASS | repeated slice touches concentrate in `canic-testing-internal::pic`, `canic-testkit::pic`, and the audit/test harness seam |

## Amplification Drivers

| Commit | Feature Slice | Slice Type | Files Touched | Subsystems | Layers | Flow Axes | Revised CAF | Density | ELS | Feature Locality Index | Containment Score | Risk |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `ede4e597` | demo/test surface split, local `dfx` recovery, summary-only audit cleanup | `release_sweep` | 35 | 4 | 4 | 3 | 12 | 8.75 | 0.17 | 0.31 | 0.50 | Medium |
| `ed028418` | named root harness profiles and hierarchy narrowing | `feature_slice` | 23 | 3 | 3 | 2 | 6 | 7.67 | 0.57 | 0.39 | 0.38 | Medium |
| `30807142` | public `canic-testkit` standalone fixture + retry promotion | `feature_slice` | 9 | 3 | 3 | 1 | 3 | 3.00 | 0.33 | 0.33 | 0.38 | Low |
| `413d0c53` | generic prebuilt install path for downstream `canic-testkit` users | `feature_slice` | 10 | 2 | 2 | 1 | 2 | 5.00 | 0.40 | 0.30 | 0.25 | Low |

Interpretation:

- The broadest current slice is a release sweep, not a routine feature signal.
- Routine `0.24.x` feature slices now mostly stay in the `workflow + ops` testkit/testing boundary, instead of cutting through `policy`, `dto`, `ops`, and `workflow` at once.
- The March routine CAF range (`16-24`) has collapsed to `2-6` in the sampled `0.24.x` routine slices.

## Comparison to Previous Relevant Run

- Improved: routine feature slices are materially smaller and more localized than the March replay/capability/auth slices.
- Improved: the current sampled routine slices do not reach into `policy` or `dto` at all.
- Stable: no cross-layer leakage was detected in sampled work.
- Changed: the main architectural pressure has shifted away from replay/auth/control-plane coordination and toward the internal/public testing seam.

## Structural Hotspots

1. [mod.rs](/home/adam/projects/canic/crates/canic-testing-internal/src/pic/mod.rs) is the clearest current gravity well.
   Evidence: it was touched in all four sampled `0.24.x` slices, and it sits at the seam between public `canic-testkit` helpers and Canic-only fixtures.

2. [mod.rs](/home/adam/projects/canic/crates/canic-testkit/src/pic/mod.rs) is the public friction hub.
   Evidence: it was touched in three of four sampled slices, and it owns the generic PocketIC lifecycle/retry surface that downstreams now depend on.

3. [wasm_store.did](/home/adam/projects/canic/crates/canic-wasm-store/wasm_store.did) is still noisy change surface.
   Evidence: it was touched in three of four sampled slices even when the substantive work was testkit/testing cleanup, which makes it a recurring incidental blast-radius contributor.

4. [instruction_audit.rs](/home/adam/projects/canic/crates/canic-tests/tests/instruction_audit.rs) remains a broad verification touchpoint.
   Evidence: it recurs in the sampled cleanup line and still has to adapt whenever audit probes or root harness profiles move.

## Hub Module Pressure

Recent-slice repeat-touch scan across the sampled `0.24.x` window:

| File / Module | Sampled Slice Touches | Pressure |
| --- | ---: | --- |
| [mod.rs](/home/adam/projects/canic/crates/canic-testing-internal/src/pic/mod.rs) | 4 | high |
| [mod.rs](/home/adam/projects/canic/crates/canic-testkit/src/pic/mod.rs) | 3 | medium |
| [lifecycle.rs](/home/adam/projects/canic/crates/canic-testing-internal/src/pic/lifecycle.rs) | 3 | medium |
| [standalone_canisters.rs](/home/adam/projects/canic/crates/canic-tests/tests/standalone_canisters.rs) | 3 | medium |
| [wasm_store.did](/home/adam/projects/canic/crates/canic-wasm-store/wasm_store.did) | 3 | medium/noisy |

Interpretation: the current pressure is in test helper boundary files, not in core policy or DTO hubs.

## Responsibility Drift Signals

- `PASS`: no sampled routine slice required edits across all five runtime layers.
- `PASS`: `policy` was absent from the sampled routine `0.24.x` feature slices.
- `WARN`: generated/support artifacts like [wasm_store.did](/home/adam/projects/canic/crates/canic-wasm-store/wasm_store.did) still enlarge otherwise-contained slices.
- `WARN`: [instruction_audit.rs](/home/adam/projects/canic/crates/canic-tests/tests/instruction_audit.rs) continues to absorb topology/test-surface movement, which makes it a recurring coordination hotspot even though it is test-only.

## Risk Score

Risk Score: **3 / 10**

Score contributions:
- `+1` internal/public test helper seam is now the main recurring hotspot
- `+1` generated/support artifact churn still inflates some otherwise-contained slices
- `+1` one broad `release_sweep` in the sampled window

Verdict: **Low-to-moderate friction risk, improved from March.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `git show --stat --oneline --name-only ede4e597` | PASS | broad `0.24.4` release-sweep sample |
| `git show --stat --oneline --name-only ed028418` | PASS | root harness/profile cleanup sample |
| `git show --stat --oneline --name-only 30807142` | PASS | public testkit promotion sample |
| `git show --stat --oneline --name-only 413d0c53` | PASS | downstream-generic install helper sample |
| `cargo clippy -p canic-testkit -p canic-testing-internal -p canic-tests --test standalone_canisters --test lifecycle_boundary -- -D warnings` | PASS | current testkit/testing seam is clean under clippy |
| `cargo test -p canic-tests --test lifecycle_boundary -- --nocapture` | PASS | non-root lifecycle boundary remains green |
| `cargo test -p canic-tests --test root_wasm_store_reconcile root_post_upgrade_preserves_multi_store_current_release_binding -- --test-threads=1 --nocapture` | PASS | root post-upgrade reconcile path remains green |

## Follow-up Actions

1. Keep shrinking incidental support-file churn inside routine feature slices, especially [wasm_store.did](/home/adam/projects/canic/crates/canic-wasm-store/wasm_store.did).
2. Watch [mod.rs](/home/adam/projects/canic/crates/canic-testing-internal/src/pic/mod.rs) and [mod.rs](/home/adam/projects/canic/crates/canic-testkit/src/pic/mod.rs) for continued gravity-well growth as `0.25` audits continue.
