# Audit Summary - 2026-03-25

## Run Contexts

- Audit run: `wasm-footprint`
  - Definition: `docs/audits/recurring/system/wasm-footprint.md`
  - Baseline: `N/A` (first run for this scope on 2026-03-25)
  - Branch: `main`
  - Commit: `5b27de05`
  - Worktree: `dirty`
  - Method: `Method V1`
  - Comparability: `comparable`

Audits generated in this run:

- `wasm-footprint`

## Risk Index Summary

| Audit | Risk Score |
| --- | ---: |
| `wasm-footprint` | 6 / 10 |

Overall day posture: **first `0.17` wasm baseline established; the shared runtime floor is heavy across leaf canisters, and `root` is a clear bundle-canister outlier that now needs explicit payload decomposition and cap-headroom modeling**.

## Key Findings by Severity

### High

- No correctness failure was found; this is a build-artifact audit. The main high-severity architectural signal is that `root` remains structurally oversized relative to leaf canisters and must be decomposed into runtime bytes versus embedded payload bytes in the next `0.17` step.

### Medium

- `blank` and `app` landed in the same size class (`3231312` vs `3214731` shrunk bytes), which points to shared-runtime baseline pressure rather than role-specific feature cost.
- `root` shrunk wasm measured `11267613` bytes in the first baseline, making it the dominant artifact and the main focus for the remaining `0.17` work.
- The first real run exposed a concrete runner bug: `dfx build` was being called with multiple positional canister names, and the runner had to be corrected to build shrunk artifacts one canister at a time.

### Low

- `ic-wasm` and `twiggy` were both available, so the first run produced structure snapshots and hotspot artifacts without tooling gaps.
- Built and shrunk wasm bytes were identical for most leaf canisters in this baseline, which is a signal worth watching rather than a defect by itself.

## Verification Readout Rollup

| Command | Status | Notes |
| --- | --- | --- |
| `bash -n scripts/ci/wasm-audit-report.sh` | PASS | runner parses after the `dfx build` fix |
| `bash scripts/ci/wasm-audit-report.sh` | PASS | first full `wasm-footprint` baseline recorded under `docs/audits/reports/2026-03/2026-03-25/` |
| `cargo build --target wasm32-unknown-unknown ... && dfx build ...` | PASS | raw and shrunk artifacts recorded for the full default canister set |
| `ic-wasm <artifact> info` | PASS | built and shrunk structure snapshots captured |
| `twiggy top|dominators|monos <analysis.wasm>` | PASS | hotspot attribution captured for every canister in scope |
| `baseline size-metrics.tsv comparison` | BLOCKED | first run of day; no same-day baseline exists yet |

## Follow-up Actions

1. Produce the root-specific decomposition that `0.17` requires: runtime bytes, embedded payload bytes, metadata bytes, largest embedded roles, and projected growth slope.
2. Add IC ceiling headroom math and warning bands to the next wasm-footprint run so `root` risk is measured against the install limit instead of only against sibling artifacts.
3. Compare `blank` retained hotspots against one feature canister and treat the overlap as shared-runtime reduction scope.
4. Identify the minimum bootstrap/recovery artifact set and the ordered extraction list needed for the `0.18` cutover handoff.

## Report Files

- `docs/audits/reports/2026-03/2026-03-25/wasm-footprint.md`
- `docs/audits/reports/2026-03/2026-03-25/artifacts/wasm-footprint/size-summary.md`
- `docs/audits/reports/2026-03/2026-03-25/artifacts/wasm-footprint/size-report.json`
- `docs/audits/reports/2026-03/2026-03-25/artifacts/wasm-footprint/root.md`
- `docs/audits/reports/2026-03/2026-03-25/summary.md`
