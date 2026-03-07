# Audit Summary — 2026-03-07

## Run Contexts

- Audit run: `complexity-accretion`
  - Definition: `docs/audits/recurring/complexity-accretion.md`
  - Branch: `eleven`
  - Initial commit: `d0b8d415`
  - Latest rerun commit: `bca4da37`
- Audit run: `velocity-preservation`
  - Definition: `docs/audits/recurring/velocity-preservation.md`
  - Branch: `eleven`
  - Initial commit: `d0b8d415`
  - Latest rerun commit: `bca4da37`

## Risk Index Summary

| Risk Index | Score | Run Context |
| ---- | ---- | ---- |
| Complexity Risk Index | 5.36/10 | complexity-accretion (latest rerun) |
| Velocity Risk Index | 4.50/10 | velocity-preservation (latest rerun) |
| Variant Explosion Pressure | 7/10 | complexity-accretion |
| Enum Shock Radius Pressure | 7/10 | velocity-preservation |
| Cross-Layer Leakage Pressure | 3/10 | velocity-preservation |

## Findings

### Critical

- `DelegatedTokenOpsError` remains the dominant branch multiplier (`36 variants x 62 decision sites = 2232`) and is the largest single entropy/shock hotspot.
  - Evidence: `crates/canic-core/src/ops/auth/error.rs`, `crates/canic-core/src/api/auth/mod.rs`

### High

- Capability envelope rollout increased decision surface quickly: `Request` decision sites rose from `32` (`v0.12.0`) to `94` (`v0.13.0/current`).
  - Evidence: `crates/canic-core/src/api/rpc/capability/mod.rs`, `crates/canic-core/src/workflow/rpc/request/handler/mod.rs`
- Control-plane hub pressure was materially reduced in this rerun:
  - `crates/canic-core/src/workflow/rpc/request/handler/mod.rs` (`218` LOC)
  - `crates/canic-core/src/ops/auth/mod.rs` (`76` LOC)
  - `crates/canic-core/src/api/rpc/mod.rs` (`62` LOC)

### Medium

- Workflow still imports stable replay record/key types directly in the request handler path.
  - Evidence: `crates/canic-core/src/workflow/rpc/request/handler/mod.rs`, `crates/canic-core/src/workflow/rpc/request/handler/replay.rs`
- Capability/proof/replay decision density remains high even after module flattening.

### Low

- `InfraError` envelope remains intentionally narrow (`1` variant) and stable.
- `InternalErrorClass` surface is stable (`6` variants, no growth).

## Snapshot Notes

- Complexity trend: control-plane concentration dropped, but auth/capability decision axes still dominate entropy.
- Velocity trend: gravity-well risk improved after decomposition; enum shock and decision density remain the main drag vectors.
- Refactor-noise verdict: rerun reflects structural reduction in hub pressure, not transient code motion.

## Report Files

- `docs/audits/reports/2026-03-07/complexity-accretion.md`
- `docs/audits/reports/2026-03-07/velocity-preservation.md`
