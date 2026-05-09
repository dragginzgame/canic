# Layer Violations Audit - 2026-05-09

## Run Context

- Definition: `docs/audits/recurring/system/layer-violations.md`
- Prior baseline: `docs/audits/reports/2026-04/2026-04-05/layer-violations.md`
- Snapshot: `53476764`
- Worktree: dirty
- Method: V5.0, with current crate paths
- Scope: `crates/canic-core/src/{api,workflow,domain,ops,storage,ids}`,
  `crates/canic-macros/src`, `crates/canic/src/macros`
- Comparability: partially comparable. The macro crate path is now
  `crates/canic-macros`, and `ids` is included because remediation moved a
  pure boundary identifier there.

## Executive Summary

Initial risk: **3 / 10**.

Post-remediation risk: **1 / 10**.

The dependency direction still holds: `endpoints -> workflow -> policy -> ops
-> model/storage`. The audit found one production coupling worth fixing:
workflow signatures referenced `storage::stable::intent::IntentId` directly.
`IntentId` is a pure identifier, so it now lives in `ids`; storage keeps the
`Storable` implementation and persistence details.

## Findings

### PASS - Upward Imports

No lower layer imports an upper layer.

Commands:

```bash
rg -n 'use crate::api|crate::api::' crates/canic-core/src/{workflow,ops,storage,domain} -g '*.rs'
rg -n 'use crate::workflow|crate::workflow::' crates/canic-core/src/{ops,storage,domain} -g '*.rs'
rg -n 'use crate::domain::policy|crate::domain::policy::' crates/canic-core/src/{ops,storage} -g '*.rs'
rg -n 'use crate::ops|crate::ops::' crates/canic-core/src/storage -g '*.rs'
```

All returned no matches.

### PASS - Policy Purity

Policy code has no async functions and no timer, storage, or IC side-effect
imports. The only `crate::cdk` policy references are `Principal` value-type
imports.

Commands:

```bash
rg -n 'async fn' crates/canic-core/src/domain/policy -g '*.rs'
rg -n 'crate::cdk::|crate::storage::|crate::infra::|set_timer|spawn|call\\(' crates/canic-core/src/domain/policy -g '*.rs'
```

### PASS - DTO Leakage

Domain and storage code do not import DTO boundary types.

Command:

```bash
rg -n 'crate::dto::|use crate::dto' crates/canic-core/src/domain crates/canic-core/src/storage -g '*.rs'
```

No matches.

### PASS - API Delegation

API code does not directly import storage or infra. Endpoint code remains a
marshal/authenticate/delegate boundary.

Command:

```bash
rg -n 'use crate::storage|crate::storage::|use crate::infra|crate::infra::' crates/canic-core/src/api -g '*.rs'
```

No matches.

### FIXED - Workflow Stable-Storage Type Coupling

Production workflow code previously referenced
`crate::storage::stable::intent::IntentId` in pool and IC-call signatures. That
made a pure identifier look storage-owned at the workflow boundary.

Remediation:

- Moved `IntentId` to `crates/canic-core/src/ids/intent.rs`.
- Exported it from `ids`.
- Updated workflow and ops code to import `ids::IntentId`.
- Kept `impl Storable for IntentId` in `storage/stable/intent.rs`, so storage
  still owns stable-memory encoding.

Post-remediation scan:

```bash
rg -n 'crate::storage::stable::intent::IntentId|storage::stable::intent::IntentId' crates/canic-core/src -g '*.rs'
```

No matches.

The broader workflow storage scan now only reports test-only replay harness
imports.

### PASS - Macro Boundary

The macro/facade scan found only
`$crate::api::canister::CanisterRole::WASM_STORE` in `start_wasm_store!`. That
is an exported facade type, not generated business logic reaching down through
core layers.

Command:

```bash
rg -n 'crate::domain::policy|crate::ops|crate::workflow|crate::api' crates/canic-macros/src crates/canic/src/macros -g '*.rs'
```

## Verification

| Check | Result |
| --- | --- |
| Upward import scans | PASS |
| Policy purity scans | PASS |
| DTO leakage scan | PASS |
| API storage/infra scan | PASS |
| Workflow storage scan after remediation | PASS, test-only matches remain |
| Macro boundary scan | PASS |
| `bash scripts/ci/run-layering-guards.sh` | PASS |
| `cargo fmt --all` | PASS |
| `cargo test -p canic-core --lib workflow::rpc::request::handler -- --nocapture` | PASS, 34 tests |
| `cargo test -p canic-core --lib intent -- --nocapture` | PASS, 8 tests |
| `cargo clippy -p canic-core --all-targets -- -D warnings` | PASS |

## Follow-up Actions

1. Keep pure cross-layer identifiers in `ids`, with storage-specific encoding
   implementations kept in storage modules.
2. Keep the test-only replay harness storage imports from expanding into
   production workflow code.
3. Re-run this audit after any broad endpoint/workflow/ops/storage refactor or
   macro expansion change.
