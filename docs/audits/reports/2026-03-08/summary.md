# Audit Summary — 2026-03-08

## Run Contexts

- Audit run: `complexity-accretion`
  - Definition: `docs/audits/recurring/complexity-accretion.md`
  - Branch: `eleven`
  - Commit: `c968b20d`
- Audit run: `velocity-preservation`
  - Definition: `docs/audits/recurring/velocity-preservation.md`
  - Branch: `eleven`
  - Commit: `c968b20d`
- Audit run: `layer-violations`
  - Definition: `docs/audits/recurring/layer-violations.md`
  - Branch: `eleven`
  - Commit: `c968b20d`
- Audit run: `caller-subject-binding`
  - Definition: `docs/audits/recurring/caller-subject-binding.md`
  - Branch: `eleven`
  - Commit: `c968b20d`

## Risk Index Summary

| Risk Index | Score | Trend vs 2026-03-07 |
| ---- | ---- | ---- |
| Complexity Risk Index | 3.82/10 | Improved (from 5.36) |
| Velocity Risk Index | 2.80/10 | Improved (from 4.50) |
| Layering Integrity | Compliant (runtime) | Improved |
| Caller-Subject Binding | Enforced | Stable |

## Findings

### Critical

- No critical security regressions found in this pass.

### High

- `BuiltinPredicate` and `RootCapabilityMetricOutcome` remain the highest current shock multipliers.

### Medium

- Caller-subject-binding invariant is now covered by both unit checks and end-to-end delegated-token mismatch PocketIC endpoint flow.

### Low

- Top-level delegated-auth error shock radius was reduced materially after layered error split.
- Control-plane hub pressure remains below previous hotspot thresholds.

## What Changed Since 2026-03-07

- Replay semantics are now extracted behind `ops/replay/*` pure guard decisions.
- Capability proof verification dispatch moved to verifier implementations.
- Monolithic delegated auth error taxonomy pressure dropped significantly at top-level decision sites.
- `Request` decision surface dropped from `94` to `18` sites, with runtime non-dto variant branching reduced to `0`.
- `BuiltinPredicate` decision/evaluation surface dropped from `30` to `16` sites after evaluator extraction.
- Root capability observability moved from monolithic `RootCapabilityMetricEvent` variants to dimensional metrics (`event_type`, `outcome`, `proof_mode`).
- Workflow replay no longer constructs stable replay records directly; runtime layering crossing count is now `0`.

## Recommended Next Work

1. Address remaining high-shock enums (`BuiltinPredicate`, `RootCapabilityMetricOutcome`) with the same extraction pattern used for request/proof surfaces.
2. Optionally clean up the remaining test-gated storage import in workflow test helpers.
3. Add one PocketIC delegated-token A/B mismatch endpoint test to lock caller-subject invariant end-to-end.

## Report Files

- `docs/audits/reports/2026-03-08/complexity-accretion.md`
- `docs/audits/reports/2026-03-08/velocity-preservation.md`
- `docs/audits/reports/2026-03-08/layer-violations.md`
- `docs/audits/reports/2026-03-08/caller-subject-binding.md`
- `docs/audits/reports/2026-03-08/summary.md`
