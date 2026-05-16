# Layer Boundary Audit - 2026-05-16

## Run Context

- Definition: `docs/audits/recurring/system/layer-violations.md`
- Prior baseline: `docs/audits/reports/2026-05/2026-05-09/layer-violations.md`
- Snapshot: `92ec102b`
- Branch: `main`
- Worktree: dirty
- Method: V6.0, focused on workflow/policy/ops drift
- Scope: `crates/canic-core/src/workflow`, `crates/canic-core/src/domain/policy`,
  `crates/canic-core/src/ops`, `crates/canic-core/src/access`,
  `crates/canic-macros/src/endpoint`, and `crates/canic/src/macros`
- Comparability: partially comparable. This run tightens the old
  layer-violations audit around grouped imports, storage-record references from
  workflow, current `canic-macros` paths, and the current
  `domain/policy` policy location.

## Executive Summary

Initial risk: **4 / 10**.

Post-remediation risk: **2 / 10**.

The hard dependency direction is now clean after two local fixes. The audit
found one workflow-to-API import and one workflow-to-storage-record reference.
Both were layering issues rather than behavior bugs, and both were removed
without changing public behavior.

No policy purity violation, ops-to-workflow call, DTO leakage into policy or
storage, or macro auth-guard bypass was found.

## Findings

### FIXED - Workflow Imported API Runtime Install Types

Severity: **Critical** before remediation, because workflow depended upward on
the API layer.

Evidence before remediation:

```text
crates/canic-core/src/workflow/runtime/install/mod.rs
crates/canic-core/src/workflow/canister_lifecycle/mod.rs
crates/canic-core/src/workflow/ic/provision/mod.rs
crates/canic-core/src/workflow/ic/provision/install.rs
```

Those workflow modules imported `api::runtime::install` types through grouped
imports. The older scan missed this because it only looked for
`use crate::api`.

Remediation:

- Moved `ApprovedModulePayload`, `ApprovedModuleSource`,
  `ModuleSourceResolver`, and `ModuleSourceRuntimeApi` to
  `ops::runtime::install_source`.
- Kept `api::runtime::install` as a compatibility re-export.
- Updated workflow modules to import the install-source surface from ops.
- Added a layering guard that fails if workflow imports the API layer again.

Post-remediation scan:

```bash
rg -n '(^|[^A-Za-z0-9_])api::|crate::api::' crates/canic-core/src/workflow -g '*.rs'
```

Result: no matches.

### FIXED - Workflow Referenced Storage Record Shape

Severity: **Medium** before remediation, because workflow did not construct or
mutate the record but still named a storage-owned persistence type.

Evidence before remediation:

```text
crates/canic-core/src/workflow/rpc/request/handler/nonroot_cycles.rs
```

The cycles authorization path accepted resolver functions returning
`storage::canister::CanisterRecord` and then read only `role` and
`parent_pid`.

Remediation:

- Added `role_parent` lookup helpers on `CanisterChildrenOps` and
  `SubnetRegistryOps`.
- Changed the workflow resolver boundary to use a local
  `ResolvedCyclesChild` with only `role` and `parent_pid`.
- Tightened the layering guard from `storage::stable::.*Record` to
  `storage::.*Record` under workflow, excluding tests.

Post-remediation scan:

```bash
rg -n 'storage::.*Record' crates/canic-core/src/workflow --glob '!**/tests.rs'
```

Result: no matches.

### PASS - Upward Imports

Hard direction scans found no remaining upward imports after remediation.

Commands:

```bash
rg -n '(^|[^A-Za-z0-9_])api::|crate::api::' crates/canic-core/src/workflow crates/canic-core/src/ops crates/canic-core/src/storage crates/canic-core/src/domain -g '*.rs'
rg -n '(^|[^A-Za-z0-9_])workflow::|crate::workflow::' crates/canic-core/src/ops crates/canic-core/src/storage crates/canic-core/src/domain -g '*.rs'
rg -n 'use crate::domain::policy|crate::domain::policy::' crates/canic-core/src/ops crates/canic-core/src/storage -g '*.rs'
rg -n 'use crate::ops|crate::ops::' crates/canic-core/src/storage -g '*.rs'
```

Notes:

- `cdk::api::*` matches from ops are platform API calls, not Canic API-layer
  imports.
- One `workflow::cascade::*` match in ops is a documentation comment, not a
  dependency.

### PASS - Policy Purity

There is no top-level `crates/canic-core/src/policy` directory in the current
tree. The active policy path is `crates/canic-core/src/domain/policy`.

Policy imports stayed pure. Matches were limited to `cdk::candid::Principal`
as a value type and `view::placement::sharding::ShardPlacement` as a read-only
projection. No `ops`, `workflow`, API, DTO, async, IC side-effect, or storage
dependency was found in `domain/policy`.

Command:

```bash
rg -n 'ic_cdk|crate::ops|crate::workflow|crate::api|serde::|candid::|async fn|\.await|storage::' crates/canic-core/src/domain/policy -g '*.rs'
```

### PASS - DTO Leakage

No DTO imports were found in policy or storage.

Command:

```bash
rg -n 'use crate::dto|crate::dto::' crates/canic-core/src/domain/policy crates/canic-core/src/storage -g '*.rs'
```

Result: no matches.

### PASS - Ops Does Not Call Workflow

No production ops dependency on workflow was found. The only match was a
documentation comment naming the topology cascade workflow that owns a cache
update invariant.

### PASS - Endpoint Macro Guard Boundary

The macro surface continues to route access checks through the access layer.
Authenticated endpoints still resolve identity through
`access::auth::resolve_authenticated_identity`, evaluate generated access
expressions through `access::expr::eval_access`, and route scoped auth through
`access::expr::auth::authenticated_with_scope`.

Command:

```bash
rg -n 'eval_access|resolve_authenticated_identity|authenticated_with_scope|requires\(' crates/canic-macros/src/endpoint crates/canic/src/macros -g '*.rs'
```

No macro bypass of access/auth guards was found.

## Ambiguous Areas

### Workflow Candid Boundary Adapters

`workflow/ic/call.rs`, `workflow/runtime/install/mod.rs`, and
`workflow/rpc/request/mod.rs` still contain Candid bounds or call-argument
adapter logic. This is a known boundary-adapter role for inter-canister calls
and install payloads, not persistence serialization.

Risk: **Low**.

Recommendation: leave as-is for now. If this grows, split the encoding adapter
behind an ops-owned call/install helper rather than moving policy or storage
logic into workflow.

### Workflow RPC Handler Size

The largest current workflow files are:

```text
633 crates/canic-core/src/workflow/placement/sharding/mod.rs
589 crates/canic-core/src/workflow/placement/directory/mod.rs
578 crates/canic-core/src/workflow/rpc/request/handler/nonroot_cycles.rs
521 crates/canic-core/src/workflow/canister_lifecycle/mod.rs
484 crates/canic-core/src/workflow/pool/mod.rs
```

Risk: **Low to Medium**.

These modules are orchestration-heavy but not currently violating direction.
Treat `workflow/rpc/request/handler/nonroot_cycles.rs` as the main watchpoint
because it combines replay, funding policy application, metrics, and execution
sequencing.

## Structural Hotspots

| Module | Reason | Risk |
| --- | --- | --- |
| `workflow/rpc/request/handler/nonroot_cycles.rs` | replay, funding authorization, metrics, and execution in one workflow module | Medium |
| `workflow/ic/call.rs` | workflow-owned Candid call adapter plus intent orchestration | Low |
| `ops/runtime/metrics/mod.rs` | large aggregation module and recent churn hotspot | Low |
| `crates/canic-macros/src/endpoint/expand.rs` | endpoint auth/access lowering chokepoint | Medium |

## Architecture Health

| Dimension | Status |
| --- | --- |
| Layer invariants | Good after remediation |
| Policy purity | Clean |
| Lifecycle boundary | Stable |
| Workflow orchestration | Healthy with one RPC watchpoint |
| DTO sharing | Expected |

Interpretation: healthy after two small boundary fixes; workflow remains the
right audit focus for future drift.

## Verification Readout

| Check | Result |
| --- | --- |
| Upward import scans | PASS |
| Policy purity scan | PASS |
| DTO leakage scan | PASS |
| Macro access/auth scan | PASS |
| `bash scripts/ci/run-layering-guards.sh` | PASS |
| `cargo fmt --all` | PASS |
| `cargo check -p canic-core -p canic-control-plane -p canic` | PASS |
| `cargo test -p canic-core --lib request_cycles -- --nocapture` | PASS |
| `cargo test -p canic-macros authenticated -- --nocapture` | PASS |
| `cargo clippy -p canic-core --all-targets -- -D warnings` | PASS |

## Final Verdict

Pass with drift risk.

No hard layering violations remain after remediation. The next runs should keep
watching workflow RPC/cycles code, endpoint macro auth lowering, and broad
runtime metrics aggregation.
