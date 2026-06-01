# Layer Violations Audit - 2026-06-01

## Run Context

- Definition: `docs/audits/recurring/system/layer-violations.md`
- Prior retained report: `docs/audits/reports/2026-05/2026-05-09/layer-violations.md`
- Snapshot: `20887c3c`
- Worktree: clean before audit slice
- Scope: canister runtime layering in `canic-core`, `canic-macros`, and the
  `canic` macro/facade code. Host/operator evidence DTOs and CLI wrappers were
  treated as out of scope unless they feed runtime APIs.

## Executive Summary

Verdict: **PASS**

Initial risk: **2 / 10**.

Post-remediation risk: **1 / 10**.

The core dependency direction still holds:

```text
endpoints/api/macros -> workflow -> policy -> ops -> model/storage
```

The 0.57.1 capability and auth layering cleanup remains intact. Public RPC API
code is a thin boundary that accepts DTOs, delegates to workflow, and maps
workflow/internal errors back to public endpoint errors. Capability proof
verification, replay projection, metrics, and dispatch orchestration remain
under `workflow::rpc::capability` / `workflow::rpc::request`.

The audit found one small cleanup: policy modules imported `Principal` through
`cdk::candid::Principal`. They now import `cdk::types::Principal`, keeping
policy code on the crate's runtime type facade instead of the broad Candid
module path.

## Audit Definition Refresh

The recurring definition was updated for the post-v1 tree:

- it now scopes the audit explicitly to canister runtime layering;
- it names `crates/canic-core/src/api/**` as the current endpoint/API boundary;
- it replaces stale `0.37` focus wording with current focus questions;
- it adds drift checks for public RPC/API proof orchestration, role-attestation
  verification ownership, public DTO error leakage, and host-side evidence DTO
  false positives.

## Findings

### PASS - Upward Imports

No lower layer imports the public API layer, workflow layer, or policy layer in
the checked forbidden directions.

Commands:

```bash
rg -n 'use crate::api|crate::api::' crates/canic-core/src/workflow crates/canic-core/src/ops crates/canic-core/src/storage crates/canic-core/src/domain -g '*.rs'
rg -n 'use crate::workflow|crate::workflow::' crates/canic-core/src/ops crates/canic-core/src/storage crates/canic-core/src/domain -g '*.rs'
rg -n 'use crate::domain::policy|crate::domain::policy::' crates/canic-core/src/ops crates/canic-core/src/storage -g '*.rs'
rg -n 'use crate::ops|crate::ops::' crates/canic-core/src/storage -g '*.rs'
```

All returned no production matches. A broader `workflow::` token scan found one
comment-only storage reference in `ops/storage/children/mod.rs`.

### FIXED - Policy Principal Imports

Policy code had several `cdk::candid::Principal` imports. These were value-type
imports, not Candid serialization use, but policy code should depend on
Canic's runtime type facade.

Changed imports to:

```rust
crate::cdk::types::Principal
```

Files changed:

```text
crates/canic-core/src/domain/policy/pool/mod.rs
crates/canic-core/src/domain/policy/topology/registry.rs
crates/canic-core/src/domain/policy/placement/sharding/backfill.rs
crates/canic-core/src/domain/policy/placement/sharding/hrw.rs
crates/canic-core/src/domain/policy/placement/sharding/metrics.rs
crates/canic-core/src/domain/policy/placement/sharding/mod.rs
```

Post-remediation scan:

```bash
rg -n 'cdk::candid::Principal|ic_cdk|crate::ops|crate::workflow|crate::api|serde::|candid::' crates/canic-core/src/domain/policy -g '*.rs'
```

No matches.

### PASS - Policy Purity

Policy code does not import workflow, ops, API, serde, Candid, or IC side-effect
modules after remediation. It does not contain async functions or `.await`
sites.

Command:

```bash
rg -n 'async fn|\.await|storage::|ic_cdk|crate::ops|crate::workflow|crate::api|serde::|candid::' crates/canic-core/src/domain/policy -g '*.rs'
```

No matches.

### PASS - DTO Leakage

Domain and storage code do not import DTO boundary types.

Command:

```bash
rg -n 'crate::dto::|use crate::dto' crates/canic-core/src/domain crates/canic-core/src/storage -g '*.rs'
```

No matches.

### PASS - API Boundary

API code does not directly import storage or infra. The current `api/rpc`
module receives public DTOs, delegates into workflow, and maps internal errors
at the boundary.

Command:

```bash
rg -n 'use crate::storage|crate::storage::|use crate::infra|crate::infra::' crates/canic-core/src/api -g '*.rs'
```

No matches.

Expected API-to-ops references remain in auth/session and auth-verifier edge
code where the API boundary collects caller, local canister, time, subnet, and
root material before delegating to ops/workflow helpers. No public RPC
capability orchestration was found in API.

### PASS - Capability And Attestation Ownership

Capability envelope verification remains workflow-owned:

```text
crates/canic-core/src/workflow/rpc/capability/
crates/canic-core/src/workflow/rpc/request/
```

The public RPC API only delegates:

```text
crates/canic-core/src/api/rpc/mod.rs
```

Role-attestation verification for capability proofs goes through runtime auth
workflow, not back into public API auth helpers.

### PASS WITH WATCHPOINT - Workflow Serialization

Workflow RPC capability code still performs Candid encoding/decoding for
capability hash binding and proof blob handling. This is currently intentional:
the workflow owns the RPC capability semantics and canonical proof binding.

Watchpoint: if the serialization surface grows beyond capability binding and
IC-call DTO marshaling, consider moving reusable encoding primitives into ops
or a dedicated pure helper module.

### PASS - Workflow Storage Boundary

Production workflow code does not directly use stable-storage APIs. The
workflow storage scan found test-only replay harness imports:

```text
crates/canic-core/src/workflow/rpc/request/handler/replay.rs
crates/canic-core/src/workflow/rpc/request/handler/tests.rs
```

These are behind test-only contexts and do not represent production layering
drift.

### PASS - Macro Boundary

Endpoint macros and public facade macros route through access/auth helpers and
API facade types. No macro code was found reaching directly into policy, ops,
or workflow internals.

Commands:

```bash
rg -n 'eval_access|resolve_authenticated_identity|authenticated_with_scope|requires\(' crates/canic-macros/src/endpoint crates/canic/src/macros -g '*.rs'
rg -n 'crate::domain::policy|crate::ops|crate::workflow|crate::api' crates/canic-macros/src crates/canic/src/macros -g '*.rs'
```

## Verification

| Check | Result |
| --- | --- |
| Upward import scans | PASS |
| Policy purity scans | PASS after import cleanup |
| DTO leakage scan | PASS |
| API storage/infra scan | PASS |
| Capability ownership scan | PASS |
| Workflow storage scan | PASS, test-only matches only |
| Macro boundary scan | PASS |
| `bash scripts/ci/run-layering-guards.sh` | PASS |
| `cargo fmt --all --check` | PASS |
| `cargo test -p canic-core workflow::rpc --lib --locked` | PASS, 77 tests |
| `cargo test -p canic --test changelog_governance --locked` | PASS |
| `cargo test -p canic --test workspace_manifest --locked` | PASS |
| `cargo clippy -p canic-core --all-targets --locked -- -D warnings` | PASS |
| `git diff --check` | PASS |

## Follow-up Actions

1. Keep policy imports on `crate::cdk::types::*` rather than broad Candid module
   paths.
2. Keep public RPC API as a DTO/error boundary only. Capability verification,
   replay projection, metrics, and dispatch orchestration should remain
   workflow-owned.
3. If workflow capability serialization grows, reassess whether a smaller pure
   helper under ops/support would make the boundary clearer.
