# 0.86 Status: Structural Maintainability

Last updated: 2026-07-12

## Purpose

This file tracks the bounded mechanical splits defined by
[0.86-design.md](0.86-design.md) so the work is not reconstructed from chat
history.

## Current State

The three Medic slices are published as `v0.86.0` through `v0.86.2`.
Auth-renewal, blob-storage, project configuration, state audit, role-package,
resolved role-contract, and deployment diagnostics have focused owners. The
Medic structural pass is complete.

The first deploy-plan slice is published as `v0.86.3`. Report rendering, JSON
output persistence, and exit classification have one focused owner; command
options, root discovery, parsing, and usage have another.

The verified-evidence slice is published as `v0.86.4`. Context, identity,
artifact, inventory, authority, trust-domain, and verifier-readiness evidence
have one focused owner.

The next slice is changelog-finalized for `0.86.5`. Target-resolution blockers,
unsupported and blocking assumptions, local-state warnings, unresolved
assumptions, stable diagnostic codes, and diagnostic-specific next actions have
one focused owner. The parent retains comparison, aggregate status, global next
actions, proposed operations, and report assembly. Classification, diagnostic
ordering, CLI behavior, JSON field order, text output, and exit behavior are
unchanged. Package versions remain `0.86.4` until the human-owned release flow
runs.

## Checklist

### Slice A - Medic

- [x] Extract auth-renewal checks.
- [x] Extract blob-storage checks and passive endpoint detection.
- [x] Extract role-package and resolved role-contract checks.
- [x] Extract project configuration and state-audit checks.
- [x] Extract deployment context, state, receipt, registry, and readiness checks.
- [x] Complete the Medic structural pass and record its final module boundary.

### Slice B - Deploy plan

- [x] Extract rendering, output persistence, and exit classification.
- [x] Extract command inputs, root discovery, parsing, and usage.
- [x] Extract verified evidence construction.
- [x] Extract blocker, warning, and assumption diagnostics.
- [ ] Extract comparison, status aggregation, and proposed operations.
- [ ] Preserve command, exit, and report contracts exactly.

### Slice C - State manifest

- [ ] Split resolution, descriptor joining, audit categories, and aggregation
      by existing responsibility.
- [ ] Preserve state-contract, report, and serialized contracts exactly.

## Validation

- `cargo test -p canic-cli medic:: --lib`: 51 passed.
- `cargo test -p canic-cli deploy::tests::plan --lib`: 18 passed.
- `cargo test -p canic-cli deploy::plan::tests --lib`: 12 passed.
- Focused cached subnet-catalog host test against `ic-query 0.10.4`: passed.
- `cargo clippy -p canic-cli --lib -- -D warnings`: passed.

## Next Action

Run the human-owned `0.86.5` release flow after reviewing the finalized patch.
After publication, continue Slice B with comparison, aggregate status, global
next actions, and proposed operations as a separately reviewable
responsibility. Do not introduce a generic planning framework or retain
parallel implementations.
