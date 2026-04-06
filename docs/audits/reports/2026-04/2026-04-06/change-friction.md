# Change Friction Audit - 2026-04-06

## Report Preamble

- Scope: `crates/canic-core/src/**`, `crates/canic-memory/src/**`, `crates/canic-testkit/src/**`, `crates/canic-testing-internal/src/**`, `crates/canic-tests/tests/**`, `canisters/**`, and recent `0.25.x` feature slices
- Compared baseline report path: `docs/audits/reports/2026-04/2026-04-05/change-friction.md`
- Code snapshot identifier: `7e514afb`
- Method tag/version: `Method V4.1`
- Comparability status: `partially comparable` (same CAF method, but the current sample is dominated by `0.25.x` structure/testkit cleanup and memory-facade work rather than the earlier `0.24.x` demo/test split line)
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-06T09:43:25Z`
- Branch: `main`
- Worktree: `dirty`

## Executive Summary

- Risk Score: `2 / 10`
- Delta summary: routine change friction is still low. Average sampled slice size is flat versus the 2026-04-05 run, but the p95 blast radius is lower and the hottest repeat-touch seam has narrowed from the broad `pic/mod.rs` hubs to a smaller cached-baseline seam around `baseline.rs`, `startup.rs`, and attestation/root harness adapters.
- Largest remaining pressure: shared PocketIC baseline lifecycle changes still coordinate between [baseline.rs](/home/adam/projects/canic/crates/canic-testkit/src/pic/baseline.rs), [startup.rs](/home/adam/projects/canic/crates/canic-testkit/src/pic/startup.rs), and [attestation.rs](/home/adam/projects/canic/crates/canic-testing-internal/src/pic/attestation.rs).
- Cross-layer leakage status: `none observed`.
- Follow-up required: `no` for immediate architecture health; `yes` only if the cached-baseline seam starts broadening back into `mod.rs`-style gravity wells.

## Baseline Delta Summary

| Metric | Previous | Current | Delta |
| --- | ---: | ---: | ---: |
| Velocity Risk Index | 3 | 2 | -1 |
| Cross-layer leakage crossings | 0 | 0 | 0 |
| Avg files touched per feature slice | 19.25 | 19.25 | 0.00 |
| p95 files touched | 35 | 27 | -8 |
| Top gravity-well fan-in | 4 | 2 | -2 |

Notes:
- Current averages use the sampled `0.25.x` slice set below with generated audit artifacts excluded.
- Current top gravity-well fan-in excludes `CHANGELOG.md` and `docs/changelog/0.25.md`, which are expected per-release bookkeeping rather than architectural friction.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Feature slices sampled from recent commits | PASS | sampled `0.25.3`, `0.25.4`, `0.25.6`, and `0.25.7` line commits |
| Revised CAF computed per slice | PASS | routine slices land in the CAF `5-12` range; the auth/runtime trim slice is the broadest routine slice at `12` |
| Release sweeps separated from routine slices | PASS | `34ecc226` is treated as a `release_sweep` and excluded from routine-friction interpretation |
| Slice density computed | PASS | density measured as `files/subsystems` per sampled slice |
| Blast radius measured | PASS | sampled slices range from `14` to `27` files after excluding generated audit artifacts |
| Boundary leakage reviewed | PASS | no new hard layering bypass was surfaced in the current sampled slice set |
| Friction amplification drivers identified | PASS | repeat slice touches now concentrate in the cached-baseline seam and a small set of support files rather than broad `pic` root hubs |

## Amplification Drivers

| Commit | Feature Slice | Slice Type | Files Touched | Subsystems | Layers | Flow Axes | Revised CAF | Density | ELS | Feature Locality Index | Containment Score | Risk |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `6382a3f1` | delegation/replay runtime trim and borrowed-proof storage cleanup | `feature_slice` | 19 | 7 | 3 | 2 | 14 | 2.71 | 0.47 | 0.11 | 0.88 | Medium |
| `34ecc226` | audit-canister split and placement-audit cleanup | `release_sweep` | 27 | 7 | 3 | 2 | 14 | 3.86 | 0.44 | 0.15 | 0.88 | Medium |
| `a151d606` | module-structure narrowing and early `canic-testkit::pic` split | `feature_slice` | 17 | 5 | 2 | 1 | 5 | 3.40 | 0.35 | 0.24 | 0.62 | Low |
| `bd6e8d95` | `canic-memory::api` addition and shared cached-baseline hardening | `feature_slice` | 14 | 5 | 1 | 2 | 10 | 2.80 | 0.50 | 0.50 | 0.62 | Low |

Interpretation:

- The broadest sampled slice is a structural release sweep, not a routine feature signal.
- Routine `0.25.x` slices are still mostly confined to one seam at a time: auth/runtime trim, module visibility cleanup, or testkit/baseline hardening.
- The newer `canic-memory::api` slice is noticeably more localized than the older `0.24.x` test-boundary cleanup line because it adds a small facade instead of reopening hidden backend state.

## Comparison to Previous Relevant Run

- Improved: p95 blast radius is lower (`27` vs `35`) even though the average slice size is flat.
- Improved: the previous broad hotspots at [mod.rs](/home/adam/projects/canic/crates/canic-testing-internal/src/pic/mod.rs) and [mod.rs](/home/adam/projects/canic/crates/canic-testkit/src/pic/mod.rs) no longer dominate repeat-touch counts.
- Stable: no cross-layer leakage was detected in sampled work.
- Changed: the main remaining pressure has shifted to a smaller cached-baseline lifecycle seam rather than general-purpose PocketIC/testkit hub growth.

## Structural Hotspots

1. [attestation.rs](/home/adam/projects/canic/crates/canic-testing-internal/src/pic/attestation.rs) is the clearest current feature-friction hotspot.
   Evidence: it appears in two sampled routine slices and is where Canic-specific cached-baseline restore behavior meets the shared `canic-testkit` baseline path.

2. [baseline.rs](/home/adam/projects/canic/crates/canic-testkit/src/pic/baseline.rs) and [startup.rs](/home/adam/projects/canic/crates/canic-testkit/src/pic/startup.rs) are the current public testkit friction seam.
   Evidence: both recur across the `0.25.6` and `0.25.7` sampled slices, and both own the shared dead-instance classification/rebuild behavior that multiple harnesses now consume.

3. [wasm_store.did](/home/adam/projects/canic/crates/canic-wasm-store/wasm_store.did) is still noisy support surface.
   Evidence: it recurs in sampled slices even when the substantive work is audit/structure cleanup, so it remains incidental blast radius rather than primary feature ownership.

## Hub Module Pressure

Recent-slice repeat-touch scan across the sampled `0.25.x` window, excluding changelog files:

| File / Module | Sampled Slice Touches | Pressure |
| --- | ---: | --- |
| [attestation.rs](/home/adam/projects/canic/crates/canic-testing-internal/src/pic/attestation.rs) | 2 | medium |
| [startup.rs](/home/adam/projects/canic/crates/canic-testkit/src/pic/startup.rs) | 2 | medium |
| [mod.rs](/home/adam/projects/canic/crates/canic-testkit/src/pic/mod.rs) | 2 | medium |
| [baseline.rs](/home/adam/projects/canic/crates/canic-testkit/src/pic/baseline.rs) | 2 | medium |
| [lib.rs](/home/adam/projects/canic/crates/canic-memory/src/lib.rs) | 2 | medium/noisy |
| [mod.rs](/home/adam/projects/canic/crates/canic-core/src/ops/replay/mod.rs) | 2 | medium |
| [mod.rs](/home/adam/projects/canic/crates/canic-core/src/workflow/placement/sharding/mod.rs) | 2 | medium |

Interpretation: the current pressure is spread across smaller seams. No single runtime or testkit root file dominates the sampled window the way `pic/mod.rs` did in the previous run.

## Responsibility Drift Signals

- `PASS`: no sampled routine slice required edits across all five runtime layers.
- `PASS`: `dto` remained absent from the sampled routine `0.25.x` feature slices.
- `WARN`: support artifacts like [wasm_store.did](/home/adam/projects/canic/crates/canic-wasm-store/wasm_store.did) still enlarge otherwise-contained slices.
- `WARN`: the cached-baseline seam still crosses public testkit plus internal harness code, so future lifecycle hardening there should stay narrow and avoid reconcentrating logic in [mod.rs](/home/adam/projects/canic/crates/canic-testkit/src/pic/mod.rs).

## Risk Score

Risk Score: **2 / 10**

Score contributions:
- `+1` shared cached-baseline lifecycle still crosses `canic-testkit` and `canic-testing-internal`
- `+1` support-file churn (`wasm_store.did`, changelogs, audit docs) still inflates some otherwise-contained slices

Verdict: **Low friction risk, slightly improved from 2026-04-05.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `git show --stat --oneline --name-only 6382a3f1` | PASS | sampled auth/runtime trim slice |
| `git show --stat --oneline --name-only 34ecc226` | PASS | sampled release-sweep split of audit vs test canisters |
| `git show --stat --oneline --name-only a151d606` | PASS | sampled module-structure cleanup slice |
| `git show --stat --oneline --name-only bd6e8d95` | PASS | sampled `canic-memory::api` + baseline-hardening slice |
| `cargo clippy -p canic-memory --lib -- -D warnings` | PASS | current `canic-memory` facade slice is clean |
| `cargo clippy -p canic --lib --all-features -- -D warnings` | PASS | narrow facade cleanup remains warning-free |
| `cargo test -p canic --lib` | PASS | facade crate still builds/tests after the narrow public-surface cleanup |

## Follow-up Actions

1. Keep cached-baseline recovery logic concentrated in [baseline.rs](/home/adam/projects/canic/crates/canic-testkit/src/pic/baseline.rs) and [startup.rs](/home/adam/projects/canic/crates/canic-testkit/src/pic/startup.rs) instead of letting local harness files grow custom variants.
2. Continue treating [wasm_store.did](/home/adam/projects/canic/crates/canic-wasm-store/wasm_store.did) as a noisy blast-radius file in future friction reads so it does not distort routine feature-locality conclusions.
