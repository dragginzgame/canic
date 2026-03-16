# Audit Summary - 2026-03-16

## Run Contexts

- Audit run: `layer-violations`
  - Definition: `docs/audits/recurring/system/layer-violations.md`
  - Branch: `main`
  - Commit: `e3a2581d`
  - Worktree: `clean`
  - Method: `Method V4.0`
  - Comparability: `comparable`
- Audit run: `layer-violations-2`
  - Definition: `docs/audits/recurring/system/layer-violations.md`
  - Baseline: `docs/audits/reports/2026-03/2026-03-16/layer-violations.md`
  - Branch: `main`
  - Commit: `e3a2581d`
  - Worktree: `dirty`
  - Method: `Method V4.0`
  - Comparability: `comparable`
- Audit run: `layer-violations-3`
  - Definition: `docs/audits/recurring/system/layer-violations.md`
  - Baseline: `docs/audits/reports/2026-03/2026-03-16/layer-violations.md`
  - Branch: `main`
  - Commit: `e3a2581d`
  - Worktree: `dirty`
  - Method: `Method V4.0`
  - Comparability: `comparable`

## Risk Index Summary

| Audit | Risk Score |
| --- | ---: |
| `layer-violations` | 6 / 10 |
| `layer-violations-2` | 3 / 10 |
| `layer-violations-3` | 3 / 10 |

Overall day posture: **pass with stable drift profile after reruns**.

## Key Findings by Severity

### High

- Initial run found policy DTO leakage in `crates/canic-core/src/domain/policy/topology/registry.rs`; rerun confirms it is resolved.
- Third rerun confirms no new layer violations after legacy compatibility path removal.

### Medium

- Policy modules still rely on `cdk::candid::Principal`, which increases policy coupling to candid type surfaces.

### Low

- Workflow `storage::stable::*` references are present only in test-gated modules.

## Verification Readout Rollup

| Command | Status | Notes |
| --- | --- | --- |
| Upward import scans (`api/workflow/policy/ops`) | PASS | no runtime upward imports detected across both runs |
| Policy purity scans (`ops/workflow/api/async`) | PASS | no async and no side-effect imports in `domain/policy` |
| DTO boundary scan in policy | PASS | rerun has no matches |
| DTO boundary scan in storage | PASS | no matches |
| `cargo test -p canic-core api::error::tests --locked` | PASS | boundary mapping test added |
| `cargo test -p canic-core registry_kind_policy_blocks_but_ops_allows --locked` | PASS | seam keeps stable policy error code |
| `cargo test -p canic-core workflow::rpc::request::handler::tests --locked` | PASS | replay handler stable after legacy replay-key cleanup |
| `cargo clippy -p canic-core --all-targets -- -D warnings` | PASS | clean |
| `cargo tree -e features` | PASS | completed successfully |

## Follow-up Actions

1. Re-audit policy principal typing (`cdk::candid::Principal` pressure) in the next recurring layer-violations run.

## Report Files

- `docs/audits/reports/2026-03/2026-03-16/layer-violations.md`
- `docs/audits/reports/2026-03/2026-03-16/layer-violations-2.md`
- `docs/audits/reports/2026-03/2026-03-16/layer-violations-3.md`
- `docs/audits/reports/2026-03/2026-03-16/summary.md`
