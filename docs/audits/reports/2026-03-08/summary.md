# Audit Summary — 2026-03-08

## Run Contexts

- Audit run: `complexity-accretion`
  - Definition: `docs/audits/recurring/complexity-accretion.md`
  - Branch: `eleven`
  - Commit: `c98bb574`
- Audit run: `velocity-preservation`
  - Definition: `docs/audits/recurring/velocity-preservation.md`
  - Branch: `eleven`
  - Commit: `c98bb574`
- Audit run: `layer-violations`
  - Definition: `docs/audits/recurring/layer-violations.md`
  - Branch: `eleven`
  - Commit: `c98bb574`
- Audit run: `caller-subject-binding`
  - Definition: `docs/audits/recurring/caller-subject-binding.md`
  - Branch: `eleven`
  - Commit: `c98bb574`
- Audit run: `capability-pipeline-conformance`
  - Definition: `docs/audits/recurring/capability-pipeline-conformance.md`
  - Branch: `eleven`
  - Commit: `c98bb574`
- Audit run: `replay-integrity-and-ttl`
  - Definition: `docs/audits/recurring/replay-integrity-and-ttl.md`
  - Branch: `eleven`
  - Commit: `c98bb574`
- Audit run: `lifecycle-symmetry-and-bootstrap`
  - Definition: `docs/audits/recurring/lifecycle-symmetry-and-bootstrap.md`
  - Branch: `eleven`
  - Commit: `c98bb574`

## Risk Index Summary

| Risk Index | Score | Trend vs 2026-03-07 |
| ---- | ---- | ---- |
| Complexity Risk Index | 3.64/10 | Improved (from 5.36) |
| Velocity Risk Index | 2.80/10 | Improved (from 4.50) |
| Layering Integrity | Compliant (runtime) | Improved |
| Caller-Subject Binding | Enforced | Stable |
| Capability Pipeline Conformance | Pass | New recurring audit |
| Replay Integrity and TTL | Pass | New recurring audit |
| Lifecycle Symmetry and Bootstrap | Pass | New recurring audit |

## Findings

### Critical

- No critical security regressions found in this pass.

### High

- No active high-severity findings remain after follow-up decomposition of `BuiltinPredicate` and root-capability outcome handling.

### Medium

- Caller-subject-binding invariant is now covered by both unit checks and end-to-end delegated-token mismatch PocketIC endpoint flow.
- `MintCycles` insufficient-root-cycles authorization branch is now covered by focused PocketIC replay integration.
- Targeted root replay integration reruns now pass after local environment cleanup.

### Low

- Top-level delegated-auth error shock radius was reduced materially after layered error split.
- Control-plane hub pressure remains below previous hotspot thresholds.
- Lifecycle audit found no contract drift; repeated non-root post-upgrade readiness stress coverage is now in place.

## What Changed Since 2026-03-07

- Replay semantics are now extracted behind `ops/replay/*` pure guard decisions.
- Capability proof verification dispatch moved to verifier implementations.
- Monolithic delegated auth error taxonomy pressure dropped significantly at top-level decision sites.
- `Request` decision surface dropped from `94` to `18` sites, with runtime non-dto variant branching reduced to `0`.
- `BuiltinPredicate` decision/evaluation surface dropped from `30` to `16` sites after evaluator extraction.
- Root capability observability moved from monolithic `RootCapabilityMetricEvent` variants to dimensional metrics (`event_type`, `outcome`, `proof_mode`).
- Workflow replay no longer constructs stable replay records directly; runtime layering crossing count is now `0`.
- Recurring audit set now includes explicit checks for capability pipeline ordering, replay/TTL semantics, and lifecycle symmetry/bootstrap safety.
- Focused replay coverage now includes the insufficient-root-cycles denial branch.
- `BuiltinPredicate` now uses grouped sub-enums (`App`, `Caller`, `Environment`, `Authenticated`) rather than a flat top-level variant surface.
- Root capability pipeline metrics now use stage-typed outcomes (`envelope`, `proof`, `authorization`, `replay`, `execution`) at call sites.
- Root capability metrics storage now uses an internal stage-specific dimension key while preserving snapshot contract shape.
- Lifecycle coverage now includes both repeated non-root post-upgrade readiness and focused non-root post-upgrade failure-phase integration checks.
- Complexity/velocity reruns now reflect reduced `BuiltinPredicate` multiplier after grouped-sub-enum decomposition.

## Recommended Next Work

1. Keep replay-focused PocketIC integration reruns on the default workspace target dir in local environments with `/tmp` filesystem constraints.
2. Continue reducing delegated validation/expiry enum shock surfaces if velocity-pressure trends regress in later slices.

## Report Files

- `docs/audits/reports/2026-03-08/complexity-accretion.md`
- `docs/audits/reports/2026-03-08/velocity-preservation.md`
- `docs/audits/reports/2026-03-08/layer-violations.md`
- `docs/audits/reports/2026-03-08/caller-subject-binding.md`
- `docs/audits/reports/2026-03-08/capability-pipeline-conformance.md`
- `docs/audits/reports/2026-03-08/replay-integrity-and-ttl.md`
- `docs/audits/reports/2026-03-08/lifecycle-symmetry-and-bootstrap.md`
- `docs/audits/reports/2026-03-08/summary.md`
