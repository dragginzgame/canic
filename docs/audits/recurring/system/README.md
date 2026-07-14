# System Audit Suite

This directory contains the active recurring system definitions named by the
canonical [method catalog](../../METHODS.md).

## Architecture And Structure

- [Layering and responsibility](layer-violations.md) is the single owner for
  endpoint, access, workflow, policy, ops, model, DTO, record, view,
  conversion, and side-effect placement.
- [Module structure](module-structure.md) owns topology and visibility.
- [Capability surface](capability-surface.md) owns endpoint/Candid capability
  surface and growth.
- [DRY consolidation](dry-consolidation.md) owns duplicated behavior and
  competing implementation authority.

The former access-, ops-, and workflow-purity definitions are merged into the
layering method. They remain recoverable through
[retired-methods.md](../../retired-methods.md), not through compatibility audit
paths.

## Maintainability And Package Surface

- [Change friction](change-friction.md)
- [Complexity accretion](complexity-accretion.md)
- [Dependency hygiene](dependency-hygiene.md)
- [Publish surface](publish-surface.md)

## Security And Lifecycle

- [Security boundary ordering](security-boundary-ordering.md) owns cross-stage
  order; individual auth properties remain under `recurring/invariants/`.
- [Bootstrap lifecycle symmetry](bootstrap-lifecycle-symmetry.md)

## Build, Release, And Measurement

- [Build integrity](build-integrity.md)
- [Release integrity](release-integrity.md)
- [Instruction footprint](instruction-footprint.md)
- [Wasm footprint](wasm-footprint.md)

## Selection And Reporting

Use [METHODS.md](../../METHODS.md) for triggers, owners, dispositions, and
holistic coverage. Use [AUDIT-HOWTO.md](../../AUDIT-HOWTO.md) for run identity,
safety, results, comparison, evidence, retention, and closeout rules.
