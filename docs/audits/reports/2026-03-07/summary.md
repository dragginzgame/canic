# Audit Summary — 2026-03-07

## Run Contexts

- Audit run: `complexity-accretion`
  - Definition: `docs/audits/recurring/complexity-accretion.md`
  - Branch: `eleven`
  - Commit: `d0b8d415`
- Audit run: `velocity-preservation`
  - Definition: `docs/audits/recurring/velocity-preservation.md`
  - Branch: `eleven`
  - Commit: `d0b8d415`

## Risk Index Summary

| Risk Index | Score | Run Context |
| ---- | ---- | ---- |
| Complexity Risk Index | 6.55/10 | complexity-accretion |
| Velocity Risk Index | 5.80/10 | velocity-preservation |
| Variant Explosion Pressure | 7/10 | complexity-accretion |
| Enum Shock Radius Pressure | 7/10 | velocity-preservation |
| Cross-Layer Leakage Pressure | 4/10 | velocity-preservation |

## Findings

### Critical

- `DelegatedTokenOpsError` remains the dominant branch multiplier (`36 variants x 62 decision sites = 2232`) and is the largest single entropy/shock hotspot.
  - Evidence: `crates/canic-core/src/ops/auth.rs`, `crates/canic-core/src/api/auth.rs`

### High

- Capability envelope rollout increased decision surface quickly: `Request` decision sites rose from `32` (`v0.12.0`) to `94` (`v0.13.0/current`).
  - Evidence: `crates/canic-core/src/api/rpc.rs`, `crates/canic-core/src/workflow/rpc/request/handler.rs`
- Hub pressure is concentrated in three modules with both high LOC and multi-domain branching:
  - `crates/canic-core/src/workflow/rpc/request/handler.rs` (1581 LOC)
  - `crates/canic-core/src/ops/auth.rs` (1253 LOC)
  - `crates/canic-core/src/api/rpc.rs` (900 LOC)
- Velocity drag hotspot formed in one slice: `api/rpc.rs` grew by `+825` LOC from `v0.12.0 -> v0.13.0`.

### Medium

- One direct workflow-to-storage stable import persists.
  - Evidence: `crates/canic-core/src/workflow/rpc/request/handler.rs:28`
- Policy layer still has one DTO coupling point (`dto::error`) from mechanical scan.
  - Evidence: `crates/canic-core/src/domain/policy/topology/registry.rs:6`

### Low

- `InfraError` envelope remains intentionally narrow (`1` variant) and stable.
- `InternalErrorClass` surface is stable (`6` variants, no growth).

## Snapshot Notes

- Complexity trend: entropy is growing in auth/capability/replay intersections, not uniformly across the codebase.
- Velocity trend: change-surface size is shrinking (`CAF 45 -> 35 -> 30`), but decision density is concentrating in fewer hubs.
- Refactor-noise verdict: this run is mostly true structural pressure, with partial containment improvement in slice blast radius.

## Report Files

- `docs/audits/reports/2026-03-07/complexity-accretion.md`
- `docs/audits/reports/2026-03-07/velocity-preservation.md`
