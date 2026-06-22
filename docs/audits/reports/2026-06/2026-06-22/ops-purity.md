# Ops Purity Audit - 2026-06-22

## Report Preamble

- Definition path: `docs/audits/recurring/system/ops-purity.md`
- Scope: `canic-core` runtime ops, root proof provisioning split, public-error
  boundary, blob-storage billing ops split, Cashier wrappers/conversions,
  transient funding guard, runtime metrics, and IC/RPC/auth hotspots.
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-19/ops-purity.md`
- Code snapshot identifier: `5bc5a458`
- Method tag/version: `ops-purity/current-blob-storage-billing`
- Comparability status: `partially-comparable`
- Auditor: `codex`
- Run timestamp: `2026-06-22T12:22:56Z`
- Branch: `main`
- Worktree: `dirty`

## Audit Definition Maintenance

The recurring definition was refreshed before execution to cover the current
0.70 blob-storage billing line. The root-proof provisioning method remains
valid, and the definition now also states:

- ops may own blob-storage stable-record/view/DTO mapping, Cashier response
  conversion, typed single Cashier call wrappers, and transient
  single-operation guards;
- ops must not own blob-storage billing workflow orchestration,
  readiness/status construction, funding attachment decisions, or public
  endpoint error mapping;
- the audit must explicitly scan the split between `ops/blob_storage`,
  `ops/cashier`, and `api/blob_storage`.

## Executive Summary

Verdict: **PASS with watchpoints**.

Risk score: **3 / 10**.

Ops remains narrow operational code. No production ops code imports workflow,
and no retry/backoff workflow or cross-domain orchestration moved into ops.
The 0.70 blob-storage billing split is acceptable: lifecycle ops owns stable
record/view/DTO projection and gateway-principal storage mutation, Cashier ops
owns typed single-call wrappers and response normalization, and the funding
ops module owns only the transient single-flight guard.

The larger async billing/status/funding flow remains in `api::blob_storage`.
That is still a layer/API facade pressure area, but it is not an ops-purity
failure because the orchestration has not moved into ops.

## Code Changes Made

- Updated `docs/audits/recurring/system/ops-purity.md` for blob-storage billing
  ops coverage.
- Corrected `ops::cashier` module comments from workflow-specific wording to
  caller-neutral wording. No behavior changed.

## Findings

### PASS - Workflow Dependency Direction

Command:

```bash
rg -n 'crate::workflow|workflow::' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Result: no output.

No production ops imports or calls into workflow were found.

### PASS WITH ACCEPTED HOTSPOTS - Orchestration Drift

Command:

```bash
rg -n 'retry|retries|loop\s*\{|while\s|join_all|spawn\(|sleep|backoff|orchestr|phase|step|transition' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Accepted hits:

- module comments that explicitly deny workflow orchestration ownership;
- `ops/ic/mod.rs` exposing the narrow `cdk::futures::spawn(...)` platform
  primitive;
- bounded storage walks, bootstrap phase state, retry-state helpers, and
  runtime metric labels;
- replay/cost-guard transition terminology for atomic state updates.

No retry loop, backoff loop, sleep, `join_all`, or cross-domain workflow
sequence was found in ops.

### PASS WITH ACCEPTED HOTSPOTS - Policy Ownership

Command:

```bash
rg -n 'struct .*Policy|enum .*Policy|impl .*Policy|mod policy|policy::|/policy/' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Accepted hits:

- `PolicyInputMapper` and placement/topology policy-input mapper names;
- `RootIssuerPolicyRecordMapper` and `root_issuer_policy` boundary mapping;
- domain policy imports used as pure decision inputs or metric labels;
- `replay_policy::CostClass` manifest metadata.

No generic production ops-owned policy module or pure policy decision type was
found.

### PASS - Endpoint/Auth Semantics

Command:

```bash
rg -n 'verify_caller|authenticated_with_scope|requires\(|canic_update|canic_query|endpoint|DelegatedToken|verify_delegated_token|install_delegation_proof_batch|CallOps::|unbounded_wait' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Ops auth still owns token/proof material verification, proof material
prepare/retrieve, key resolution, pending metadata, and bounded metrics. It
does not own generated endpoint authorization, endpoint subject binding, or
root proof install broadcast orchestration.

### PASS - Root Proof Provisioning Split

Command:

```bash
rg -n 'prepare_delegation_proof_batch|get_delegation_proof_batch|install_delegation_proof_batch|install_active_delegation_proof|mark_delegation_proof_batch_installed|CallOps::|unbounded_wait|root_issuer_policy|validate_root_delegation_proof_prepare_policy' crates/canic-core/src/ops/auth crates/canic-core/src/workflow/runtime/auth crates/canic-core/src/api/auth -g '*.rs' --glob '!**/tests.rs'
```

The split remains correct:

- `api/auth` owns endpoint-facing methods, root checks, caller guard checks,
  and public error mapping;
- `ops/auth/delegation` owns bounded batch prepare/get metadata, root proof
  assembly helpers, pending metadata mutation, active proof install
  verification, and root issuer policy mapping;
- `workflow/runtime/auth/provisioning` owns issuer cross-canister install
  broadcast through `CallOps::unbounded_wait(...)`;
- pure issuer policy decisions remain in `domain/policy/auth`.

### PASS WITH ACCEPTED HOTSPOTS - Public Error Boundary

Command:

```bash
rg -n 'crate::dto::error|dto::error::Error|crate::Error|Error::invalid|Error::forbidden|Error::exhausted|InternalError::public|Self::public|root_data_certificate_unavailable' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Accepted hits:

- `ops/rpc/mod.rs` preserves a remote canister's wire-level public error
  through `InternalError::public(...)`;
- root proof provisioning ops return typed `InternalError::invalid_input` and
  `InternalError::forbidden` values for internal protocol failures;
- `ops/auth/error.rs` exposes the typed
  `root_data_certificate_unavailable(...)` protocol error for direct root
  query retrieval failure.

Ops does not construct general public DTO errors directly.

### PASS - Blob-Storage Billing Ops Split

Command:

```bash
rg -n 'set_billing_config|billing_config_dto|billing_config_record_to_view|billing_config_view_to_dto|record_gateway_principal_sync|replace_gateway_principals|BlobStorageFundingOps|CashierClientOps|CashierConversionOps|sync_gateway_principals_from_cashier|sync_gateway_principals_from_configured_cashier|fund_from_project_cycles|status\(' crates/canic-core/src/ops/blob_storage crates/canic-core/src/ops/cashier crates/canic-core/src/api/blob_storage.rs -g '*.rs' --glob '!**/tests.rs'
```

The split is acceptable:

- `ops/blob_storage/lifecycle` constructs stable billing config records,
  returns read-only billing config views, projects DTOs, and performs bounded
  gateway-principal storage mutations;
- `ops/blob_storage/funding` owns only the transient RAII funding guard;
- `ops/cashier/client` owns typed single Cashier bounded-wait calls;
- `ops/cashier/conversion` owns signed balance conversion and gateway-principal
  normalization;
- `api/blob_storage` still owns endpoint-facing config validation, sync/funding
  orchestration, status DTO construction, and public error mapping.

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
| `api::blob_storage` | Medium | Billing sync, funding, and status orchestration remain concentrated in a large API facade. This is a layer/API cleanup watchpoint, not an ops-purity failure. |
| `ops/blob_storage/lifecycle` | Medium | It now owns stable billing config record/view/DTO projection. Keep this as bounded conversion/storage work only. |
| `ops/cashier` | Low | Cashier ops should remain typed single-call wrappers and conversion helpers, with no production defaults or billing workflow decisions. |
| `ops/blob_storage/funding` | Low | The funding guard should stay transient and should not grow Cashier call, cycle math, or stable state ownership. |
| `ops/auth/delegation` | Medium | Root proof provisioning remains recently edited; batch broadcast and retries must stay in workflow/operator code. |
| Runtime metrics ops | Low | Metric stores are large and centralized; avoid moving report orchestration into metrics. |

## Verification Readout

| Check | Result |
| --- | --- |
| Ops workflow dependency scan | PASS |
| Ops orchestration scan | PASS with accepted hotspots |
| Ops policy ownership scan | PASS with accepted hotspots |
| Ops endpoint/auth semantics scan | PASS |
| Root proof provisioning split scan | PASS |
| Ops public error boundary scan | PASS with accepted hotspots |
| Blob-storage billing ops split scan | PASS |
| Ops metrics coordination scan | PASS with accepted hotspots |
| `cargo test --locked -p canic-core cashier --lib --features blob-storage-billing -- --nocapture` | PASS, 9 tests |
| `cargo test --locked -p canic-core blob_storage --lib --features blob-storage-billing -- --nocapture` | PASS, 48 tests |
| `cargo test --locked -p canic-core --lib root_prepare_policy_rejects_audience_or_grant_outside_policy -- --nocapture` | PASS, 1 test |
| `cargo test --locked -p canic-core --lib batch_install_preflight_rejects_proof_mismatch -- --nocapture` | PASS, 1 test |
| `cargo clippy --locked -p canic-core --lib --features blob-storage-billing -- -D warnings` | PASS |
| `bash scripts/ci/run-layering-guards.sh` | PASS |
| `cargo fmt --all -- --check` | PASS |
