# Audit Summary - 2026-07-01

## Run Contexts

| Reports | Type | Scope | Status |
| --- | --- | --- | --- |
| `wasm-footprint.md` through `wasm-footprint-4.md` | Recurring Wasm footprint baseline and same-day reruns | release and wasm-debug artifacts for the retained test fleet | PASS with the first-run baseline comparison unavailable by definition |

## Risk Index Summary

| Report set | Risk | Notes |
| --- | ---: | --- |
| Wasm footprint runs | 3 / 10 | Shared leaf-runtime pressure and the expected root bundle outlier remain attributable. |

## Method / Comparability Notes

- All four reports use `Method V2` and are comparable.
- Same-day reruns compare directly with `wasm-footprint.md`, not with each
  other.

## Key Findings By Severity

- No high or medium correctness finding was reported.
- The root artifact remains a separate bundle-canister outlier.
- No dedicated minimal role is attached to the audited fleet, so repeated leaf
  hotspots remain the shared-runtime signal.

## Verification Readout Rollup

- PASS: canonical release and wasm-debug artifact capture, `ic-wasm` snapshots,
  Twiggy reports, and same-day baseline comparisons for reruns.
- BLOCKED: only the first report's prior same-day baseline, because it
  establishes that baseline.

## Follow-up Actions

- Keep root separate from leaf comparisons.
- Decide during a future Wasm audit whether to attach a dedicated minimal
  baseline role.
