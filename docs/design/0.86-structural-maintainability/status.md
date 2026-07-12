# 0.86 Status: Structural Maintainability

Last updated: 2026-07-12

## Purpose

This file tracks the bounded mechanical splits defined by
[0.86-design.md](0.86-design.md) so the work is not reconstructed from chat
history.

## Current State

The first two Medic slices are published as `v0.86.0` and `v0.86.1`.
Auth-renewal, blob-storage, project configuration, state audit, role-package,
and resolved role-contract diagnostics have focused owners.

The next slice is changelog-finalized for `0.86.2` and completes the Medic
structural pass. Deployment
context, installed-state, receipt, registry, and root-readiness checks move to
one focused deployment module. The 268-line parent retains project and
deployment check ordering plus shared ICP CLI checks. CLI behavior, finding
codes, report shapes, and rendering are unchanged. The workspace also adopts
`ic-query 0.10.2` after its focused cached-catalog integration passes. Package
versions remain `0.86.1` until the human-owned release flow runs.

## Checklist

### Slice A - Medic

- [x] Extract auth-renewal checks.
- [x] Extract blob-storage checks and passive endpoint detection.
- [x] Extract role-package and resolved role-contract checks.
- [x] Extract project configuration and state-audit checks.
- [x] Extract deployment context, state, receipt, registry, and readiness checks.
- [x] Complete the Medic structural pass and record its final module boundary.

### Slice B - Deploy plan

- [ ] Split evidence collection, comparison, diagnostics, and rendering by
      existing responsibility.
- [ ] Preserve command, exit, and report contracts exactly.

### Slice C - State manifest

- [ ] Split resolution, descriptor joining, audit categories, and aggregation
      by existing responsibility.
- [ ] Preserve state-contract, report, and serialized contracts exactly.

## Validation

- `cargo test -p canic-cli medic:: --lib`: 51 passed.
- `cargo clippy -p canic-cli --lib -- -D warnings`: passed.

## Next Action

Run the human-owned `0.86.2` release flow after reviewing the finalized patch.
After publication, begin Slice B with one mechanical deploy-plan
responsibility. Do not introduce a generic planning framework or retain
wrappers in the parent module.
