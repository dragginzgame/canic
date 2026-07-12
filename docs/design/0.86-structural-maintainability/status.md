# 0.86 Status: Structural Maintainability

Last updated: 2026-07-12

## Purpose

This file tracks the bounded mechanical splits defined by
[0.86-design.md](0.86-design.md) so the work is not reconstructed from chat
history.

## Current State

Implementation has started without changing package versions. The first Medic
slice moves auth-renewal and blob-storage check construction and passive Candid
detection into focused child modules. The parent still owns check selection and
ordering. CLI behavior, finding codes, report shapes, and rendering are
unchanged. This slice is changelog-finalized for `0.86.0`; package versions
remain `0.85.5` until the human-owned release flow runs.

## Checklist

### Slice A - Medic

- [x] Extract auth-renewal checks.
- [x] Extract blob-storage checks and passive endpoint detection.
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

Run the human-owned `0.86.0` release flow after reviewing the finalized patch.
After publication, continue Slice A with the next cohesive Medic
responsibility. Do not introduce a generic check framework or retain wrappers
in the parent module.
