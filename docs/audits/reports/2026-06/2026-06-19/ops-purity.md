# Ops Purity Audit - 2026-06-19

## Report Preamble

- Definition path: `docs/audits/recurring/system/ops-purity.md`
- Scope: `canic-core` runtime ops, root proof provisioning split, public-error
  boundary, policy mapper naming, runtime metrics, IC/RPC/auth hotspots, and
  topology index mappers.
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-01/ops-purity.md`
- Code snapshot identifier: `ef55e53c`
- Method tag/version: `ops-purity/current-root-proof-provisioning`
- Comparability status: `partially-comparable`
- Auditor: `codex`
- Run timestamp: `2026-06-19`
- Branch: `main`
- Worktree: `dirty`

## Audit Selection

This audit was selected as one of the oldest recurring system audits. The prior
report was still focused on generic ops purity and earlier policy-mapper
cleanup. The current implementation has added root proof provisioning, active
proof install/status, direct root proof retrieval, and issuer-local delegated
token issuance, so the audit definition needed to be refreshed before running.

## Audit Definition Maintenance

The recurring definition was updated before execution. The live audit now
recognizes that ops may own bounded proof-material operations while still
rejecting workflow ownership:

- root/issuer canister-signature proof prepare, retrieval, and verification are
  allowed in ops when they are a single bounded step;
- root proof batch broadcast and external provisioning loops remain workflow or
  operator responsibilities;
- root issuer policy DTO/record mapping helpers are allowed when they delegate
  pure decisions to `domain/policy/auth`;
- public-error boundary scans now distinguish typed protocol errors from
  general API public error mapping;
- root proof prepare/get/install scans cover `api/auth`,
  `workflow/runtime/auth`, and `ops/auth`.

## Executive Summary

Verdict: **PASS with watchpoints**.

Initial risk: **4 / 10**.

Post-remediation risk: **3 / 10**.

Ops remains narrow operational code. No production ops code imports workflow,
and root proof installation broadcast remains in workflow. Ops owns bounded
root proof metadata/proof operations, pending metadata mutation, issuer-local
active proof verification/storage, runtime metrics stores, platform call
helpers, and record/DTO/view conversion.

Two low-risk cleanups were made:

- `ops/auth/delegation/policy.rs` was renamed to
  `ops/auth/delegation/root_issuer_policy.rs` so the path no longer suggests
  ops owns domain policy decisions;
- `ops/topology/index/builder.rs` now imports `Principal` through
  `cdk::types::Principal` instead of the broader Candid path.

## Findings

### FIXED - Root Issuer Mapping Looked Policy-Owned

Severity: **Low**.

The root issuer policy mapping helpers lived in:

```text
crates/canic-core/src/ops/auth/delegation/policy.rs
```

The code did not define pure policy decisions. It validated DTO shape, mapped
boundary request material into root issuer policy records, mapped records back
to views, and delegated prepare decisions to `domain/policy/auth`. The generic
`policy` module name still made the ops layer look like it owned policy.

Remediation:

```text
crates/canic-core/src/ops/auth/delegation/root_issuer_policy.rs
```

The facade and batch modules now import the boundary-specific module name. A
post-remediation scan found no production `ops::auth::delegation::policy` or
generic `mod policy` path in the delegation ops cluster.

### FIXED - Ops Topology Builder Used Broad Candid Principal Path

Severity: **Low**.

`ops/topology/index/builder.rs` imported `Principal` through
`crate::cdk::candid::Principal`. The runtime ops convention is to use the local
runtime facade type path.

Remediation:

```rust
use crate::cdk::types::Principal;
```

Post-remediation scan:

```bash
rg -n 'cdk::candid::Principal|candid::Principal' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

No production matches remain.

### PASS - Workflow Dependency Direction

Command:

```bash
rg -n 'crate::workflow|workflow::' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

No production ops imports or calls into workflow were found.

### PASS WITH ACCEPTED HOTSPOTS - Orchestration Drift

Command:

```bash
rg -n 'retry|retries|loop\s*\{|while\s|join_all|spawn\(|sleep|backoff|orchestr|phase|step|transition' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Accepted hits:

- `ops/ic/mod.rs` exposes a narrow `cdk::futures::spawn(...)` platform
  primitive; workflow owns when to call it.
- `ops/storage/registry/subnet.rs` walks a bounded parent chain within one
  registry operation.
- `ops/runtime/bootstrap.rs` stores bootstrap phase/status facts.
- runtime metrics modules use phase labels and transition wording for bounded
  observability.
- `ops/storage/icp_refill.rs` keeps retry-state helpers, not retry loops.
- replay modules use transition terminology for receipt state.

No retry loop, backoff loop, sleep, `join_all`, or cross-domain workflow
sequence was found in ops.

### PASS WITH ACCEPTED HOTSPOTS - Policy Ownership

Command:

```bash
rg -n 'struct .*Policy|enum .*Policy|impl .*Policy|mod policy|policy::|/policy/' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Accepted hits:

- `PolicyInputMapper` remains a conversion helper name.
- `RootIssuerPolicyRecordMapper` and `root_issuer_policy` map boundary/storage
  shapes and delegate pure decisions to `domain/policy/auth`.
- ops metric labels reference policy error/reason types for bounded reporting.
- `replay_policy::CostClass` is replay-policy manifest metadata, not an ops
  decision module.

No generic production ops `policy` module remains in the inspected auth
delegation cluster.

### PASS - Endpoint/Auth Semantics

Command:

```bash
rg -n 'verify_caller|authenticated_with_scope|requires\(|canic_update|canic_query|endpoint|DelegatedToken|verify_delegated_token|install_delegation_proof_batch|CallOps::|unbounded_wait' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Ops auth still owns token/proof material verification, proof material
prepare/retrieve, key resolution, pending metadata, and bounded metrics. It
does not own generated endpoint authorization, endpoint subject binding, or
root proof install broadcast orchestration.

Accepted hits include endpoint words in comments/metric labels and
`ops/rpc`/`ops/ic` single-operation platform call helpers.

### PASS - Root Proof Provisioning Split

Command:

```bash
rg -n 'prepare_delegation_proof_batch|get_delegation_proof_batch|install_delegation_proof_batch|install_active_delegation_proof|mark_delegation_proof_batch_installed|CallOps::|unbounded_wait|root_issuer_policy|validate_root_delegation_proof_prepare_policy' crates/canic-core/src/ops/auth crates/canic-core/src/workflow/runtime/auth crates/canic-core/src/api/auth -g '*.rs' --glob '!**/tests.rs'
```

The split is correct:

- `api/auth` owns endpoint-facing methods, root checks, caller guard checks,
  and public error mapping.
- `ops/auth/delegation` owns batch prepare/get metadata, root proof assembly
  helpers, pending metadata mutation, active proof install verification, and
  root issuer policy mapping.
- `workflow/runtime/auth/provisioning` owns issuer cross-canister install
  broadcast via `CallOps::unbounded_wait(...)` and records installed outcomes
  only after issuer success.
- pure issuer policy decisions remain in `domain/policy/auth`.

### PASS WITH ACCEPTED HOTSPOTS - Public Error Boundary

Command:

```bash
rg -n 'crate::dto::error|dto::error::Error|InternalError::public|Self::public|root_data_certificate_unavailable' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Accepted hits:

- `ops/rpc/mod.rs` preserves a remote canister's wire-level public error
  through `InternalError::public(...)`;
- `ops/auth/error.rs` exposes the typed
  `root_data_certificate_unavailable(...)` protocol error for direct root
  query retrieval failure;
- inline test names in `ops/auth/error.rs` appeared in the scan despite the
  production glob and are not production boundary code.

Ops does not invent general endpoint/API public error DTOs.

### PASS WITH ACCEPTED HOTSPOTS - Metrics Coordination

Command:

```bash
rg -n 'Metric|Metrics::record|record_.*metric|metrics::' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Runtime metrics remain metric stores and single-operation recording helpers.
No metrics module was found running a multi-domain workflow to produce a
report.

## Residual Watchpoints

| Area | Risk | Note |
| --- | --- | --- |
| `ops/auth/delegation/` | Medium | Recently split root proof provisioning code should keep batch broadcast and retry ownership in workflow/operator code. |
| `workflow/runtime/auth/provisioning/` | Medium | This remains the intended cross-canister install orchestration owner. |
| `ops/auth/token.rs` and `ops/auth/delegated/**` | Medium | Token/proof material verification is ops-owned; endpoint subject binding must stay outside ops. |
| `ops/rpc/mod.rs` and `ops/ic/call.rs` | Medium | Single protocol calls are acceptable; retry/recovery must stay out of ops. |
| Runtime metrics ops | Low | Metric stores are large and centralized; avoid moving report orchestration into metrics. |
| Policy input and root issuer mappers | Low | Mapper names may mention policy input or root issuer policy shape, but pure decisions must stay in `domain/policy`. |

## Verification Readout

| Check | Result |
| --- | --- |
| Ops workflow dependency scan | PASS |
| Ops orchestration scan | PASS with accepted hotspots |
| Ops policy ownership scan | PASS after root issuer module rename |
| Ops endpoint/auth semantics scan | PASS |
| Root proof provisioning split scan | PASS |
| Ops public error boundary scan | PASS with accepted hotspots |
| Ops metrics coordination scan | PASS with accepted hotspots |
| Ops Candid Principal scan | PASS after import cleanup |
| `cargo fmt --all` | PASS |
| `cargo test --locked -p canic-core ops::topology::index --lib -- --nocapture` | PASS, 5 tests |
| `cargo test --locked -p canic-core ops::auth::delegation --lib -- --nocapture` | PASS, 26 tests |
| `cargo test --locked -p canic-core ops --lib -- --nocapture` | PASS, 237 tests |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --locked -p canic-core --lib -- -D warnings` | PASS |

## Final Verdict

Pass with watchpoints.

Ops remains narrow operational code. The main residual risk is expected pressure
around recently changed auth provisioning, RPC/platform call helpers, and
metrics storage, not a current layering violation.
