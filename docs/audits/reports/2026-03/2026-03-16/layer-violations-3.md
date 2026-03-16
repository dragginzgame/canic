# Layer Violations Audit - 2026-03-16 (Rerun 3)

## Report Preamble

- Scope: `crates/canic-core/src/{api,workflow,domain,ops,storage,access,lifecycle}`
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-16/layer-violations.md`
- Code snapshot identifier: `e3a2581d`
- Method tag/version: `Method V4.0`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-03-16T12:25:13Z`
- Branch: `main`
- Worktree: `dirty`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| No upward `workflow/ops/storage/domain -> api` imports | PASS | `rg -n 'use crate::api\|crate::api::' crates/canic-core/src/{workflow,ops,storage,domain}` |
| No upward `ops/storage/domain -> workflow` imports | PASS | `rg -n 'use crate::workflow\|crate::workflow::' crates/canic-core/src/{ops,storage,domain}` |
| No upward `ops/storage -> domain::policy` imports | PASS | `rg -n 'use crate::domain::policy\|crate::domain::policy::' crates/canic-core/src/{ops,storage}` |
| Policy purity (`ops/workflow/api` imports, async) | PASS | no `crate::ops|crate::workflow|crate::api` imports and no `async fn` in `domain/policy` |
| DTO leakage into `domain/policy` | PASS | no matches for `\bdto::` in `crates/canic-core/src/domain/policy` |
| DTO leakage into `storage` | PASS | no matches for `\bdto::` in `crates/canic-core/src/storage` |
| API direct storage/infra coupling | PASS | no matches in `crates/canic-core/src/api` |
| Workflow direct stable-storage coupling (runtime) | PASS | only `#[cfg(test)]` references in replay/delegation handler tests |
| Macro boundary policy leakage | PASS | no layer leaks detected in `crates/canic-dsl-macros/src` |
| Crate dependency cycle signal | PASS | `cargo tree -e features` completed |

## Comparison to Baseline

- Stable: baseline policy DTO leakage remains resolved.
- Stable: no new upward runtime-layer imports introduced while removing legacy compatibility paths.
- Stable: storage remains free of DTO imports.

## Violations Summary

- No concrete runtime layering violations found in this rerun.

## Responsibility Drift Signals

### Policy Layer Drift

| Location | Pattern | Drift Risk | Why |
| --- | --- | --- | --- |
| `domain/policy/*` | `cdk::candid::Principal` usage | Medium | candid principal coupling remains as drift pressure, but no DTO/boundary coupling present |

### Workflow Layer Drift

| Location | Pattern | Drift Risk | Why |
| --- | --- | --- | --- |
| `workflow/rpc/request/handler/{delegation,replay,tests}.rs` | `storage::stable::*` in `#[cfg(test)]` code | Low | runtime path remains clean; test seam coupling persists |

## Risk Score

Risk Score: **3 / 10**

Score contributions:
- `+1` policy principal candid coupling pressure
- `+1` workflow test-only storage coupling signal
- `+1` boundary mapping complexity in API error conversion path

Verdict: **Pass with drift risk - no hard layering violations.**

## Architecture Health Interpretation

| Dimension | Status |
| --- | --- |
| Layer invariants | Good |
| Policy purity | Clean (runtime) |
| Lifecycle boundary | Stable |
| Workflow orchestration | Healthy |
| DTO sharing | Expected |

Interpretation: layer contract remains intact after legacy compatibility removal; remaining drift pressure is non-blocking and structural.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n 'use crate::api\|crate::api::' crates/canic-core/src/{workflow,ops,storage,domain}` | PASS | no matches |
| `rg -n 'use crate::workflow\|crate::workflow::' crates/canic-core/src/{ops,storage,domain}` | PASS | no matches |
| `rg -n '\bdto::' crates/canic-core/src/domain/policy` | PASS | no matches |
| `cargo test -p canic-core api::error::tests --locked` | PASS | boundary mapping tests pass |
| `cargo test -p canic-core registry_policy_seam --locked` | PASS | seam path keeps stable policy mapping behavior |
| `cargo test -p canic-core workflow::rpc::request::handler::tests --locked` | PASS | replay handler remains stable after legacy replay-key cleanup |
| `cargo clippy -p canic-core --all-targets -- -D warnings` | PASS | clean |
| `cargo tree -e features` | PASS | completed |

## Follow-up Actions

No follow-up actions required.
