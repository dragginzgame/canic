# Layer Violations Audit - 2026-03-16

## Report Preamble

- Scope: `crates/canic-core/src/{api,workflow,domain,ops,storage,access,lifecycle}`
- Compared baseline report path: `N/A` (first run for this scope on 2026-03-16)
- Code snapshot identifier: `e3a2581d`
- Method tag/version: `Method V4.0`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-03-16T11:17:04Z`
- Branch: `main`
- Worktree: `clean`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| No upward `workflow/ops/storage/domain -> api` imports | PASS | `rg -n 'use crate::api\|crate::api::' crates/canic-core/src/{workflow,ops,storage,domain}` |
| No upward `ops/storage/domain -> workflow` imports | PASS | `rg -n 'use crate::workflow\|crate::workflow::' crates/canic-core/src/{ops,storage,domain}` |
| No upward `ops/storage -> domain::policy` imports | PASS | `rg -n 'use crate::domain::policy\|crate::domain::policy::' crates/canic-core/src/{ops,storage}` |
| Policy purity (`ops/workflow/api` imports, async) | PASS | no matches for `crate::ops|crate::workflow|crate::api` and no `async fn` in `crates/canic-core/src/domain/policy` |
| DTO leakage into `domain/policy` | FAIL | `crates/canic-core/src/domain/policy/topology/registry.rs:6` imports `dto::error::{Error as PublicError, ErrorCode}` |
| DTO leakage into `storage` | PASS | no matches for `\bdto::` in `crates/canic-core/src/storage` |
| API direct storage/infra coupling | PASS | no matches for `use crate::storage|crate::storage::|use crate::infra|crate::infra::` in `crates/canic-core/src/api` |
| Workflow direct stable-storage coupling (runtime) | PASS | matches only in test-gated paths (`#[cfg(test)]` modules) |
| Macro boundary policy leakage | PASS | no matches for `domain::policy|crate::domain::policy|policy::` in `crates/canic-dsl-macros/src/endpoint` and `crates/canic/src/macros` |
| Crate dependency cycle signal | PASS | `cargo tree -e features` completed successfully |

## Violations

### High

- Policy layer imports boundary DTO error types in `crates/canic-core/src/domain/policy/topology/registry.rs`.
- Evidence:
  - line 6 imports `dto::error::{Error as PublicError, ErrorCode}`
  - line 52 returns `ErrorCode` from policy error mapping
  - line 73 maps policy errors into `PublicError::policy(...)`
- Why this is invalid: policy must not depend on DTO/boundary serialization contracts.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/domain/policy/topology/registry.rs` | `RegistryPolicyError` | policy-to-DTO error coupling in runtime path | High |
| `crates/canic-core/src/workflow/rpc/request/handler/delegation.rs` | test module imports | workflow test-only storage coupling signal | Low |
| `crates/canic-core/src/workflow/rpc/request/handler/replay.rs` | test helper imports | workflow test-only storage coupling signal | Low |

## Responsibility Drift Signals

### Policy Layer Drift

| Location | Pattern | Drift Risk | Why |
| --- | --- | --- | --- |
| `domain/policy/topology/registry.rs:6` | `dto::error` import | High | boundary DTO contract leaked into policy |
| `domain/policy/*` | `cdk::candid::Principal` usage | Medium | policy depends on candid principal type instead of layer-local ID/value abstraction |

### Workflow Layer Drift

| Location | Pattern | Drift Risk | Why |
| --- | --- | --- | --- |
| `workflow/rpc/request/handler/{delegation,replay,tests}.rs` | `storage::stable::*` in test-gated code | Low | runtime clean, but test seams still touch stable-storage types directly |

## Risk Score

Risk Score: **6 / 10**

Score contributions:
- `+4` confirmed hard layering violation (policy -> DTO)
- `+1` policy candid-type coupling pressure
- `+1` repeated workflow test-only storage coupling signals

Verdict: **Fail - one concrete layering violation detected.**

## Architecture Health Interpretation

| Dimension | Status |
| --- | --- |
| Layer invariants | At risk |
| Policy purity | Drifting |
| Lifecycle boundary | Stable |
| Workflow orchestration | Healthy |
| DTO sharing | Escalating |

Interpretation: runtime architecture is mostly stable, but policy boundary purity is currently broken.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n 'use crate::api\|crate::api::' crates/canic-core/src/{workflow,ops,storage,domain}` | PASS | no matches |
| `rg -n 'use crate::workflow\|crate::workflow::' crates/canic-core/src/{ops,storage,domain}` | PASS | no matches |
| `rg -n 'use crate::domain::policy\|crate::domain::policy::' crates/canic-core/src/{ops,storage}` | PASS | no matches |
| `rg -n '\bdto::' crates/canic-core/src/domain/policy` | FAIL | one match at `domain/policy/topology/registry.rs:6` |
| `rg -n '\bdto::' crates/canic-core/src/storage` | PASS | no matches |
| `rg -n 'use crate::storage\|crate::storage::\|use crate::infra\|crate::infra::' crates/canic-core/src/api` | PASS | no matches |
| `cargo tree -e features` | PASS | command completed |

## Follow-up Actions

1. Owner boundary: `domain/policy` + `workflow/api`.
   - Action: move `ErrorCode`/`PublicError` mapping out of `RegistryPolicyError`; keep policy error types pure.
   - Target report run: `docs/audits/reports/2026-03/2026-03-17/layer-violations.md`.
2. Owner boundary: `domain/policy`.
   - Action: evaluate replacing direct `cdk::candid::Principal` usage with layer-stable ID/value abstractions in policy modules.
   - Target report run: `docs/audits/reports/2026-03/2026-03-17/layer-violations.md`.
3. Owner boundary: `workflow`.
   - Action: keep `storage::stable::*` references confined to `#[cfg(test)]` code and prevent runtime leakage.
   - Target report run: next same-scope rerun.
