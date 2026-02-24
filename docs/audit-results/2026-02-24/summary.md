# Audit Summary — 2026-02-24

## Run Contexts

- Audit run: `caller-subject-binding`
  - Definition: `docs/audits/caller-subject-binding.md`
  - Branch: `main`
  - Commit: `a2bdce13`
- Audit run: `layer-violations`
  - Definition: `docs/audits/layer-violations.md`
  - Branch: `main`
  - Commit: `a2bdce13`

## Risk Index Summary

| Risk Index                  | Score | Run Context |
| -------------------------- | ----- | ----------- |
| Auth Binding Integrity     | 2/10  | caller-subject-binding |
| Delegation Chain Integrity | N/A   | not assessed in these runs |
| Root Authority Integrity   | N/A   | not assessed in these runs |
| Access Boundary Integrity  | 3/10  | caller-subject-binding |
| Layering Integrity         | 7/10  | layer-violations |
| Lifecycle Integrity        | N/A   | not assessed in these runs |
| Complexity Pressure        | 5/10  | layer-violations |

## Findings

### Critical

- `ops -> workflow` upward dependency exists and violates canonical layer direction.
  - Evidence: `crates/canic-core/src/ops/runtime/ready.rs:28`

### High

- None.

### Medium

- `ops/rpc/mod.rs` depends on `dto::error::Error`, which couples ops to a boundary DTO contract.
  - Evidence: `crates/canic-core/src/ops/rpc/mod.rs:5`

### Low

- Policy layer imports candid principal types in several modules (no side effects found, but adds coupling pressure).

### Prior Run Carryover

- Threshold-key-dependent integration assertions are conditionally skipped when threshold keys are unavailable.
  - Evidence: `pic_delegation_provision.rs` skip path in `provision_or_skip(...)`.

## Snapshot Notes

- Layering verdict for today: **Fail** (hard violation present).
- Detailed report: `docs/audit-results/2026-02-24/layer-violations.md`.
