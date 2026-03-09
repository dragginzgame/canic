# Layer Violations Audit — 2026-02-24

## Run Context

- Audit run: `layer-violations`
- Definition: `docs/audits/layer-violations.md`
- Auditor: `codex`
- Date (UTC): `2026-02-24 21:44:55Z`
- Branch: `main`
- Commit: `a2bdce13`
- Worktree: `dirty`
- Scope: `crates/canic-core/src/**`, `crates/canic-dsl-macros/src/endpoint/**`, `crates/canic/src/macros/**`

## Checklist Results

### 1. Dependency Direction (Hard Check)

#### 1.1 No Upward Imports

- [ ] No upward imports detected
- [x] Violations listed below

Violations:
- **Critical**: `ops -> workflow` upward dependency.
  - Evidence: `crates/canic-core/src/ops/runtime/ready.rs:28`
  - Code: `pub(crate) fn mark_ready(_token: crate::workflow::bootstrap::ReadyToken)`
  - Why invalid: `ops` directly depends on a `workflow` type, violating canonical dependency direction.

#### 1.2 Policy Purity Imports

- [~] Policy remains side-effect free and dependency-clean
- [x] Policy contains no async behavior
- [x] Violations listed below

Findings:
- **Low / Ambiguous drift risk**: policy modules import `Principal` via candid paths.
  - Evidence:
    - `crates/canic-core/src/domain/policy/topology/registry.rs:4`
    - `crates/canic-core/src/domain/policy/pool/mod.rs:4`
    - `crates/canic-core/src/domain/policy/placement/sharding/mod.rs:15`
  - Why ambiguous: no side-effect APIs were found, but candid-type imports in policy increase serialization-coupling pressure.

#### 1.3 DTO Boundary Purity in Domain/Storage

- [x] `domain` does not depend on DTOs
- [x] `storage` does not depend on DTOs
- [x] Violations listed below

Violations:
- None.

### 2. Layer Responsibility Checks (Behavioral)

#### 2.1 API Boundary Discipline

- [x] API does not embed storage/infra dependency leaks from scan
- [~] Manual business-logic review only partially covered

Findings:
- No direct `api -> storage` or `api -> infra` imports found from scan.

#### 2.2 Workflow Ownership

- [x] No direct workflow calls to `ic_cdk`/management signature APIs from scan
- [x] No direct workflow `storage::stable` imports from scan

Findings:
- None.

#### 2.3 Ops Boundary Discipline

- [~] Ops remains mostly deterministic service facade
- [x] Findings listed below

Findings:
- **Medium (contract drift risk)**: ops layer consumes boundary DTO error type.
  - Evidence: `crates/canic-core/src/ops/rpc/mod.rs:5`
  - Code: `use crate::{ ..., dto::error::Error, ... }`
  - Why risk: ops is coupled to API-facing error DTO semantics; this weakens layer isolation.

#### 2.4 Model/Storage Purity

- [x] No workflow/api/auth concern imports detected in storage scan

Findings:
- None.

#### 2.5 Side-Effect Containment

- [x] IC signature and key APIs are confined to approved ops boundary
- [x] No certified-data API usage found

Findings:
- Expected containment only:
  - `crates/canic-core/src/ops/ic/ecdsa.rs:5-6,60,81`

### 3. Data Boundary Checks

#### 3.1 DTO Leakage

- [x] No direct DTO persistence in domain/storage layers
- [~] Ops includes DTO mapping helpers by design

Findings:
- Scan matches are primarily in `ops` and `api` adapters/mappers (expected in current contract).

#### 3.2 Error Boundary Leakage

- [~] Ops does not fully isolate from boundary error DTOs

Findings:
- Same medium drift-risk item as §2.3 (`ops/rpc/mod.rs` using `dto::error::Error`).

### 4. Capability Enforcement Placement (0.11+)

- [~] Not assessable from current code scan (`RootCapability` pipeline not present yet)

Findings:
- No `RootCapability`/`execute_root_capability` symbols found.

### 5. Macro Boundary Check

- [x] Macros are primarily endpoint/access wiring
- [~] Minor policy-like guard in endpoint macro remains boundary-level auth check

Findings:
- Manual review targets:
  - `crates/canic/src/macros/start.rs`
  - `crates/canic/src/macros/endpoints.rs`
  - `crates/canic-dsl-macros/src/endpoint/expand.rs`
- No orchestration/policy engine embedded in macro expansion logic.

### 6. Cyclic Crate Dependency Check

- [x] `cargo tree -e features` completed successfully
- [x] No crate cycle evidence detected

### 7. Drift Pressure Check

- [~] Drift pressure observed

Notes:
- `ops -> workflow` type dependency indicates boundary erosion risk.
- `ops` dependence on `dto::error::Error` indicates outward contract coupling pressure.
- Capability placement checks are not yet testable from symbols because the 0.11 capability pipeline has not landed.

## Severity Summary

- Critical: 1
- High: 0
- Medium: 1
- Low: 1
- Ambiguous: 3

## Final Verdict

**Fail — one concrete layering violation detected.**

Hard blocker:
- `crates/canic-core/src/ops/runtime/ready.rs:28` (`ops` importing `workflow` token type).

## Confidence

- Static scan confidence: `high`
- Manual inspection coverage: `medium`
- Not deeply inspected: full function-level business logic review across all `ops/**` and `api/**`
