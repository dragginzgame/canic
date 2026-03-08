# Layering Integrity Audit — 2026-03-08

## Run Context

- Audit run: `layer-violations`
- Definition: `docs/audits/recurring/layer-violations.md`
- Auditor: `codex`
- Date (UTC): `2026-03-08 00:07:33Z`
- Branch: `eleven`
- Commit: `c968b20d`
- Worktree: `dirty` (active `0.13.x` refactor + docs updates)
- Scope: `crates/canic-core/src/{api,workflow,domain,ops,storage}`

## Checklist

### 1. Dependency Direction (Hard Check)

#### 1.1 No Upward Imports

- [x] No `workflow/ops/storage/domain` imports of `api` detected.
- [x] No `ops/storage/domain` imports of `workflow` detected.
- [x] No `ops/storage` imports of `domain::policy` detected.

Commands run:
- `rg -n 'use crate::api|crate::api::' crates/canic-core/src/{workflow,ops,storage,domain}`
- `rg -n 'use crate::workflow|crate::workflow::' crates/canic-core/src/{ops,storage,domain}`
- `rg -n 'use crate::domain::policy|crate::domain::policy::' crates/canic-core/src/{ops,storage}`

#### 1.2 Policy Purity Imports

- [x] No `policy` imports of `ops/workflow/api/ic_cdk` detected.
- [x] No async functions detected in `domain/policy`.
- [~] One `candid::Principal` import appears in a policy test helper path; no runtime side effect observed.

#### 1.3 DTO Boundary Purity in Domain/Storage

- [x] No `dto` imports in `domain`.
- [x] No `dto` imports in `storage`.

### 2. Layer Responsibility Checks (Behavioral)

#### 2.1 API Boundary Discipline

- [x] No direct `api -> storage` or `api -> infra` imports detected.
- [x] API remains boundary/delegation oriented.

#### 2.2 Workflow Ownership

- [x] No runtime workflow imports of stable storage record types remain.
- [x] Workflow replay commit now delegates replay-record construction to ops (`ops/replay::commit_root_replay`).
- [x] No direct `ic_cdk` side-effect calls detected in workflow modules.

Findings:
- One storage import remains in `workflow/rpc/request/handler/mod.rs:4`, but it is test-gated with `#[cfg(test)]`.

#### 2.3 Ops Boundary Discipline

- [x] No `ops -> workflow` dependency crossings detected.
- [x] IC/system side effects remain in approved ops boundaries (for example `ops/ic/ecdsa.rs`).

#### 2.4 Model/Storage Purity

- [x] No `storage -> workflow/api/policy` behavior leakage detected.
- [x] No auth predicate logic in `storage`.

#### 2.5 Side-Effect Containment

- [x] IC ECDSA calls are contained to `ops/ic/ecdsa.rs`.
- [x] No policy-layer side-effect usage detected.
- [x] No stable-memory direct writes in high-level policy/domain logic.

### 3. Data Boundary Checks

#### 3.1 DTO Leakage

- [x] No DTO persistence directly in storage layer.
- [~] Workflow delegates proof writes via ops (`DelegationStateOps::set_proof_from_dto`), which is valid but should remain ops-owned.

## Violations Summary

- No runtime layering violations found in this pass.

## Recommended Follow-up

1. Optionally remove the test-gated `ReplaySlotKey` import from workflow by moving test helpers behind ops replay test utilities.
2. Continue enforcing replay/auth ownership split when adding new capability families.

## Verdict

- Layering status: **Compliant (runtime)**.
- Blocking violations: **None**.
- Drift risk to track next pass: **test-helper storage type bleed into workflow modules**.
