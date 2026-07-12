# 0.86 Status: Structural Maintainability

Last updated: 2026-07-12

## Purpose

This file tracks the bounded mechanical splits defined by
[0.86-design.md](0.86-design.md) so the work is not reconstructed from chat
history.

## Current State

The first Medic slice is published as `v0.86.0`. Auth-renewal and blob-storage
check construction and passive Candid detection have focused child owners.

The next slice is changelog-finalized for `0.86.1`. Project configuration and
state-audit checks have one focused owner; role-package metadata,
runtime-feature requirements, resolved role-contract findings, and
state-descriptor admission checks have another. The parent still owns check
selection and ordering. CLI behavior, finding codes, report shapes, and
rendering are unchanged. Package versions remain `0.86.0` until the
human-owned release flow runs.

## Checklist

### Slice A - Medic

- [x] Extract auth-renewal checks.
- [x] Extract blob-storage checks and passive endpoint detection.
- [x] Extract role-package and resolved role-contract checks.
- [x] Extract project configuration and state-audit checks.
- [ ] Continue splitting only coherent existing Medic responsibilities.
- [ ] Complete the Medic structural pass and record its final module boundary.

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

Run the human-owned `0.86.1` release flow after reviewing the finalized patch.
After publication, continue Slice A with deployment diagnostics as a separately
reviewable responsibility. Do not introduce a generic check framework or
retain wrappers in the parent module.
