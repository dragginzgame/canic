# System Audit Suite

This directory contains recurring architecture and drift audits for the core
Canic system.

## Standard Starter Set

Use this set for repeatable "code cleanliness / architecture health" audit
rounds:

1. [layer-violations.md](layer-violations.md)
2. [capability-surface.md](capability-surface.md)
3. [complexity-accretion.md](complexity-accretion.md)
4. [wasm-footprint.md](wasm-footprint.md)
5. [module-structure.md](module-structure.md)
6. [dependency-hygiene.md](dependency-hygiene.md)

These audits cover:

- layering and dependency direction
- public/internal capability surface growth
- branch/enum/concept accretion
- shipped wasm output and retained-size drift
- crate/module topology, visibility hygiene, and facade containment
- crate dependency direction, feature hygiene, and publish-surface discipline

## Additional System Audits

- [instruction-footprint.md](instruction-footprint.md)
- [bootstrap-lifecycle-symmetry.md](bootstrap-lifecycle-symmetry.md)
- [change-friction.md](change-friction.md)

## Usage Guidance

- Use the standard starter set for broad architectural review rounds.
- Add `instruction-footprint` when the goal is runtime optimization or perf
  regression detection.
- Use `bootstrap-lifecycle-symmetry` after lifecycle/bootstrap changes.
- Use `change-friction` during refactor planning or release-hardening windows.
- Use `module-structure` when reviewing public surface drift, crate topology,
  or demo/test/audit boundary cleanliness.
- Use `dependency-hygiene` when reviewing Cargo graph drift, feature-flag sprawl,
  or publish/package surface cleanliness.

## Reporting Discipline

Store outputs under:

- `docs/audits/reports/YYYY-MM/YYYY-MM-DD/<scope>.md`

Follow:

- [../../AUDIT-HOWTO.md](../../AUDIT-HOWTO.md)
- [../../META-AUDIT.md](../../META-AUDIT.md)
