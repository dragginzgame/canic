# Layer Violations Audit - 2026-06-22

## Run Context

- Definition: `docs/audits/recurring/system/layer-violations.md`
- Compared baseline report:
  `docs/audits/reports/2026-06/2026-06-17/layer-violations.md`
- Snapshot: `4bcad983`
- Branch: `main`
- Worktree: dirty
- Scope: canister runtime layering in `canic-core`, `canic-macros`, and the
  `canic` macro/facade code.
- Method tag/version: recurring layer-boundary audit, 2026-06 refreshed
  guard-parity run.
- Comparability: partial. The audit definition was tightened before this run,
  and the workspace already contained unrelated host-side audit/report work.

## Executive Summary

Verdict: **PASS with drift risk after remediation**.

Initial risk: **5 / 10**.

Post-remediation risk: **3 / 10**.

The refreshed audit found one concrete production layer leak:
`api::blob_storage` imported the stable billing config record and constructed it
directly. That put persisted record conversion in the API boundary instead of
ops.

That leak was remediated during the run:

- added `view::blob_storage::BlobStorageBillingConfigView`;
- moved billing config record-to-view and view-to-DTO mapping into
  `ops::blob_storage::lifecycle`;
- changed `BlobStorageApi` to delegate billing config record construction and
  DTO projection through ops.

After remediation, the hard layering guard passes and cfg-test-aware scans show
no production API direct dependency on blob-storage stable records.

The residual risk is architectural pressure, not a hard violation:
`api::blob_storage` remains a large facade with async billing/status helper
logic and direct platform/cashier ops calls. That is acceptable for the current
slice but should remain the next layer cleanup watchpoint.

## Findings

### Fixed - Blob Storage API Constructed Stable Billing Record

Initial evidence:

```text
crates/canic-core/src/api/blob_storage.rs:47: storage::stable::blob_storage::BlobStorageBillingConfigRecord
crates/canic-core/src/api/blob_storage.rs:105: BlobStorageBillingConfigRecord::new(...)
```

Why this was invalid:

- `api/**` is a boundary facade.
- Persisted `*Record` construction belongs in `ops::*`/storage ownership.
- API may validate public DTO inputs and delegate, but must not own stable
  record conversion.

Remediation:

- `crates/canic-core/src/view/blob_storage.rs` now defines
  `BlobStorageBillingConfigView`.
- `BlobStorageLifecycleOps::set_billing_config` now constructs the stable
  `BlobStorageBillingConfigRecord`.
- `BlobStorageLifecycleOps::billing_config` maps stable records into the view.
- `BlobStorageLifecycleOps::billing_config_dto` maps the view to the public
  billing config DTO.
- `BlobStorageApi` no longer imports `BlobStorageBillingConfigRecord`.

Post-remediation production scan:

```bash
awk 'BEGIN { in_test = 0 } /^#\[cfg\(test\)\]/ { in_test = 1 } !in_test && /BlobStorageBillingConfigRecord|storage::stable::blob_storage/ { print FILENAME ":" FNR ":" $0 }' crates/canic-core/src/api/blob_storage.rs
```

Result: no output.

Plain `rg` still finds inline test reset helpers under `#[cfg(test)]`, which is
not a production boundary violation.

### Pass - Executable Layer Guard

Command:

```bash
bash scripts/ci/run-layering-guards.sh
```

Result: PASS.

The guard covers workflow storage-record access, workflow API imports, API
shared replay orchestration, API root issuer policy upsert handling,
workflow-defined policy types, DTO usage in domain/storage/model, public error
DTO leakage in auth ops, access direct stable/record usage, public record
re-exports, and view naming misuse.

### Pass - Lower-Layer Dependency Direction

Scans:

```bash
rg -n 'use crate::api|crate::api::' crates/canic-core/src/workflow crates/canic-core/src/ops crates/canic-core/src/storage crates/canic-core/src/domain -g '*.rs'
rg -n 'use crate::workflow|crate::workflow::' crates/canic-core/src/ops crates/canic-core/src/storage crates/canic-core/src/domain -g '*.rs'
rg -n 'use crate::domain::policy|crate::domain::policy::' crates/canic-core/src/ops crates/canic-core/src/storage -g '*.rs'
rg -n 'use crate::ops|crate::ops::' crates/canic-core/src/storage -g '*.rs'
```

Result: PASS.

The broad `api::` token scan found only `cdk::api` platform calls under `ops`,
which is an approved side-effect location.

### Pass - Policy, Access, Record, And View Guards

Scans:

```bash
rg -n 'ic_cdk|crate::ops|crate::workflow|crate::api|storage::|serde::|candid::' crates/canic-core/src/domain/policy -g '*.rs'
find crates/canic-core/src/access -name '*.rs' -print0 | xargs -0 awk 'FNR == 1 { in_test = 0 } /^#\[cfg\(test\)\]/ { in_test = 1 } !in_test { print FILENAME ":" FNR ":" $0 }' | rg 'stable::|storage::.*Record|AppMode|EnvRecord|AppStateRecord'
rg 'pub use .*Record' crates/canic-core/src | rg -v 'pub\(crate\)'
rg '(to_view|from_view)' crates/canic-core/src | rg -v 'record_to_view|view::'
```

Result: PASS.

The policy drift scan also matched `domain/policy/env.rs` and `EnvPolicy` by
name, not `std::env` or side-effect usage; that is a false-positive token hit.

### Pass - DTO And Error Boundaries

Scans:

```bash
rg -n 'crate::dto::|use crate::dto|\bdto::' crates/canic-core/src/domain crates/canic-core/src/storage crates/canic-core/src/model -g '*.rs'
rg -n 'dto::error::Error|crate::dto::error|ErrorCode|InternalError::public\(' crates/canic-core/src/ops/auth -g '*.rs' --glob '!**/tests.rs'
```

Result: PASS.

No DTO imports were found in domain/storage/model, and auth ops do not expose
public error DTOs in production code.

### Low - Workflow Adapter Watchpoints Remain

Cfg-test-aware workflow scan still finds established adapter seams:

```text
crates/canic-core/src/workflow/mod.rs:33: candid::CandidType
crates/canic-core/src/workflow/canister_lifecycle/mod.rs:30: RegistryPolicyInputMapper
crates/canic-core/src/workflow/canister_lifecycle/mod.rs:413: RegistryPolicyInputMapper::record_to_policy_input(...)
crates/canic-core/src/workflow/canister_lifecycle/mod.rs:457: RegistryPolicyInputMapper::record_to_policy_input(...)
```

Classification:

- Not a hard violation in this run.
- The mapper is owned under ops topology input mapping; workflow invokes it but
  does not define persisted records or conversion helpers.
- Keep watching workflow Candid/type-bound seams so reusable codecs do not move
  upward.

### Medium - Blob Storage API Facade Pressure

Evidence:

```text
crates/canic-core/src/api/blob_storage.rs: 1198 lines
crates/canic-core/src/api/blob_storage.rs:338: pub async fn sync_gateway_principals_from_cashier(...)
crates/canic-core/src/api/blob_storage.rs:373: pub async fn fund_from_project_cycles(...)
crates/canic-core/src/api/blob_storage.rs:438: pub async fn status(...)
crates/canic-core/src/api/blob_storage.rs:385: MgmtOps::canister_cycle_balance()
crates/canic-core/src/api/blob_storage.rs:475: cashier_account_total_balance(...)
```

Classification:

- No hard violation after the stable-record leak was fixed.
- This API module still owns endpoint-facing billing orchestration, status
  projection, and direct platform/cashier ops calls.
- Next cleanup should consider moving blob-storage billing orchestration into
  workflow or a narrower ops-owned service surface, leaving API as a DTO facade.

### Pass - Lifecycle Adapter Bounds

Lifecycle files remain under the current line threshold:

```text
98 crates/canic-core/src/lifecycle/init/nonroot.rs
51 crates/canic-core/src/lifecycle/init/root.rs
79 crates/canic-core/src/lifecycle/mod.rs
121 crates/canic-core/src/lifecycle/upgrade/nonroot.rs
73 crates/canic-core/src/lifecycle/upgrade/root.rs
```

Lifecycle adapters call ops/workflow and schedule async bootstrap through timer
closures; no direct storage/model mutation drift was found in this run.

### Pass - Crate Feature Tree

Command:

```bash
cargo tree -e features
```

Result: PASS.

No crate-level dependency cycle signal was observed.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/api/blob_storage.rs` | `BlobStorageApi` | large endpoint facade with billing/status orchestration | Medium |
| `crates/canic-core/src/workflow/rpc/request/handler/*` | root capability authorize/replay/execute paths | central workflow-policy/ops boundary pressure | Medium |
| `crates/canic-core/src/workflow/ic/icp_refill/*` | `IcpRefillWorkflow` | replay, cost guard, transfer/notify orchestration | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `api/blob_storage.rs` | `BlobStorageLifecycleOps`, `CashierClientOps`, `MgmtOps`, DTOs | 4 | 3 | 6 |
| `workflow/rpc/request/handler/*` | capability, replay, metrics, request execution | 4 | 3 | 6 |
| `workflow/ic/icp_refill/*` | DTOs, ledger transfer, replay, cost guard | 4 | 3 | 5 |

## Architecture Watchpoint

Primary watchpoint: `crates/canic-core/src/api/blob_storage.rs`.

Reason: after this run, it is the remaining place where endpoint facade size
and async billing/status orchestration create layer-pressure, even though the
hard storage-record leak is fixed.

## Responsibility Drift Signals

- Workflow side-effect drift: none confirmed.
- Policy side-effect drift: none confirmed.
- Access stable-record drift: none.
- DTO behavior drift: none.
- Lifecycle drift: none confirmed.
- API facade pressure: `api::blob_storage` remains medium watchpoint.

## Risk Score

Risk Score: **3 / 10**.

Derivation:

- `+2` for a medium API facade hotspot after remediation.
- `+1` for established workflow adapter/hub pressure.
- `+0` for hard layering violations after remediation.
- `+0` for policy/access/DTO/record/view guard failures.

## Architecture Health Interpretation

| Dimension | Status |
| --- | --- |
| Layer invariants | Good |
| Policy purity | Clean |
| Lifecycle boundary | Stable |
| Workflow orchestration | Healthy with watchpoints |
| DTO sharing | Expected |

Interpretation: layer invariants are clean after remediation, with blob-storage
API facade pressure as the next practical cleanup target.

## Verification Readout

| Check | Result |
| --- | --- |
| `bash scripts/ci/run-layering-guards.sh` | PASS |
| lower-layer upward import scans | PASS |
| cfg-test-aware API stable-record scan | PASS |
| DTO leakage scan | PASS |
| auth ops public-error scan | PASS |
| access stable-record scan | PASS |
| public record re-export scan | PASS |
| view naming scan | PASS |
| lifecycle line-count check | PASS |
| `cargo tree -e features` | PASS |
| `cargo fmt --all` | PASS |
| `cargo check --locked -p canic-core --features blob-storage-billing` | PASS |
| `cargo test --locked -p canic-core blob_storage --lib --features blob-storage-billing -- --nocapture` | PASS, 48 tests |
| `cargo clippy --locked -p canic-core --lib --features blob-storage-billing -- -D warnings` | PASS |
| `git diff --check` | PASS |

## Follow-Up Actions

1. Keep the new `view::blob_storage::BlobStorageBillingConfigView` and ops-owned
   billing config mapping; do not reintroduce stable records into API.
2. Consider a follow-up cleanup moving blob-storage billing sync/funding/status
   orchestration out of `api::blob_storage` once the current 70 slice is ready
   for another low-risk boundary pass.
