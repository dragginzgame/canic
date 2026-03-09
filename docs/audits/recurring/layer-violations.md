# Audit: Layering Integrity and Cross-Layer Violations

## Purpose

Detect architectural drift against the Canic layering contract.

Security and maintainability invariant:

> Dependencies and behavior must follow the canonical direction:
> `endpoints/macros -> workflow -> policy -> ops -> model`.

This audit verifies:

- dependency direction
- layer responsibilities
- cross-layer data leakage
- macro boundary correctness

Run this audit after:

- architecture refactors
- moving logic across `api/workflow/policy/ops/model`
- macro/runtime dispatch changes
- large feature merges

## Canonical Layer Model

From `docs/contracts/ARCHITECTURE.md`:

```text
endpoints/macros
    ->
workflow
    ->
policy
    ->
ops
    ->
model
```

Supporting rules:

- Lower layers must not depend on higher layers.
- `dto` is transfer format for endpoints/workflow/ops.
- `model` and `policy` must not depend on `dto`.
- Authentication is enforced at endpoint/access boundary.

### Allowed Dependency Matrix (Normative)

| Layer | Allowed dependencies |
| --- | --- |
| `endpoints/macros` | `workflow`, `dto`, `access` |
| `workflow` | `policy`, `ops`, `dto` |
| `policy` | policy inputs/value types (`domain`/`view`), no side effects |
| `ops` | `model`/`storage`, `infra`, `dto` |
| `model`/`storage` | no upward layer dependencies |

Any dependency outside this matrix is a violation.

## Run Context

Record in the audit result file:

- Date
- Auditor
- Branch
- Commit (`git rev-parse --short HEAD`)
- Workspace state (`clean`/`dirty`)
- Scope (paths reviewed)

## Checklist

Mark each item:

- `[x]` Pass
- `[ ]` Fail
- `[~]` Ambiguous / drift risk (needs follow-up)

### 1. Dependency Direction (Hard Check)

#### 1.1 No Upward Imports

Verify lower layers do not import higher layers.

Suggested scans:

```bash
rg -n 'use crate::api|crate::api::' crates/canic-core/src/{workflow,ops,storage,domain} -g '*.rs'
rg -n 'use crate::workflow|crate::workflow::' crates/canic-core/src/{ops,storage,domain} -g '*.rs'
rg -n 'use crate::domain::policy|crate::domain::policy::' crates/canic-core/src/{ops,storage} -g '*.rs'
rg -n 'use crate::ops|crate::ops::' crates/canic-core/src/storage -g '*.rs'
```

- [ ] No upward imports detected
- [ ] Violations listed below

Violations:

- (file, line, import, why invalid)

#### 1.2 Policy Purity Imports

Policy must not import runtime/infra/serialization concerns.

Suggested scans:

```bash
rg -n 'ic_cdk|crate::ops|crate::workflow|crate::api|serde::|candid::' crates/canic-core/src/domain/policy -g '*.rs'
rg -n 'async fn' crates/canic-core/src/domain/policy -g '*.rs'
```

- [ ] Policy remains side-effect free and dependency-clean
- [ ] Policy contains no async behavior
- [ ] Violations listed below

Violations:

- (file, line, symbol, why invalid)

#### 1.3 DTO Boundary Purity in Domain/Storage

`dto` usage must remain out of domain/model/storage layers.

Suggested scans:

```bash
rg -n 'crate::dto::|use crate::dto' crates/canic-core/src/domain -g '*.rs'
rg -n 'crate::dto::|use crate::dto' crates/canic-core/src/storage -g '*.rs'
```

- [ ] `domain` does not depend on DTOs
- [ ] `storage` does not depend on DTOs
- [ ] Violations listed below

Violations:

- (file, line, symbol, why invalid)

## 2. Layer Responsibility Checks (Behavioral)

### 2.1 API Boundary Discipline

API layer should be boundary mapping and delegation only.

Check:

- [ ] API does not embed business policy logic
- [ ] API does not orchestrate multi-step workflows
- [ ] API does not directly mutate model/storage internals

Suggested scan:

```bash
rg -n 'use crate::storage|crate::storage::|use crate::infra|crate::infra::' crates/canic-core/src/api -g '*.rs'
```

Findings:

- (file, line, why)

### 2.2 Workflow Ownership

Workflow should orchestrate and sequence, not become infra/model boundary.

Check:

- [ ] Workflow does not call infra directly (except via approved ops surface)
- [ ] Workflow does not mutate storage internals directly
- [ ] Workflow does not bypass policy where policy is required

Suggested scan:

```bash
rg -n 'ic_cdk::|sign_with_ecdsa|ecdsa_public_key|set_certified_data|data_certificate' crates/canic-core/src/workflow -g '*.rs'
rg -n 'storage::stable::|crate::storage::stable::' crates/canic-core/src/workflow -g '*.rs'
```

Findings:

- (file, line, why)

### 2.3 Ops Boundary Discipline

Ops should not contain domain decisions or multi-step orchestration.

Check:

- [ ] Ops does not encode business policy decisions
- [ ] Ops does not orchestrate retries/cascades/long flows
- [ ] Ops remains deterministic service facade

Manual review required:

- `crates/canic-core/src/ops/**`

Findings:

- (file, function, why)

### 2.4 Model/Storage Purity

Model/storage should hold state and local invariants only.
In the current tree, stable state lives primarily under `storage/**`.

Check:

- [ ] No business policy logic in model/storage
- [ ] No workflow/orchestration imports in model/storage
- [ ] No endpoint/auth concerns in model/storage

Suggested scan:

```bash
rg -n 'crate::workflow|crate::api|authenticated\\(|caller::|policy::' crates/canic-core/src/storage -g '*.rs'
```

Findings:

- (file, line, why)

### 2.5 Side-Effect Containment

Layering includes side-effect placement, not just import shape.

Check:

- [ ] IC call/system side effects are contained to approved ops/infra boundaries
- [ ] Stable-memory write concerns do not leak into higher-layer business logic
- [ ] Time/randomness/external-call decisions do not leak into policy/domain logic

Suggested scans:

```bash
rg -n 'ic_cdk::(call|spawn|api::time|api::call)|sign_with_ecdsa|ecdsa_public_key' crates/canic-core/src -g '*.rs'
rg -n 'stable_(save|read|write)|set_certified_data|data_certificate' crates/canic-core/src -g '*.rs'
```

Findings:

- (file, line, why)

## 3. Data Boundary Checks

### 3.1 DTO Leakage

Verify DTOs are not used as persistent model records.

Check:

- [ ] API DTOs are not persisted directly to stable storage
- [ ] Storage records are not returned directly as public API payloads
- [ ] Workflow-internal models are not exposed as endpoint DTOs

Suggested scan:

```bash
rg -n 'dto::.*(Record|State)|set_.*dto|store_.*dto|export\\(\\).*dto' crates/canic-core/src -g '*.rs'
```

Findings:

- (file, line, why)

### 3.2 Error Boundary Leakage

Check:

- [ ] API maps internal errors to public boundary errors
- [ ] Ops does not return API boundary error types
- [ ] Storage/model does not depend on workflow error enums

Suggested scan:

```bash
rg -n 'dto::error::Error|ErrorCode' crates/canic-core/src/{ops,storage,workflow,api} -g '*.rs'
```

Findings:

- (file, line, why)

## 4. Capability Enforcement Placement (0.11+)

If root capability model is present:

Check:

- [ ] Capability authorization occurs in workflow layer
- [ ] API does not make capability allow/deny decisions directly
- [ ] Ops does not inspect/branch on capability enums
- [ ] Policy does not perform dispatch routing

Suggested scans:

```bash
rg -n 'RootCapability|execute_root_capability|authorize\\(' crates/canic-core/src -g '*.rs'
```

Findings:

- (file, line, why)

## 5. Macro Boundary Check (`canic-dsl-macros`)

Macros should generate boundary wiring, not business behavior.

Check:

- [ ] Macro expansion remains endpoint/access/dispatch wiring only
- [ ] No workflow/policy business logic embedded in macros
- [ ] Access predicates route through access layer

Manual review targets:

- `crates/canic-dsl-macros/src/endpoint/*`
- `crates/canic/src/macros/*`

Findings:

- (file, line, why)

## 6. Cyclic Crate Dependency Check

Run:

```bash
cargo tree -e features
```

Check:

- [ ] No crate-level cyclic dependency patterns introduced via features/re-exports

Findings:

- (crate path, why)

## 7. Drift Pressure Check

Qualitative drift prompts:

- Has any layer grown disproportionately?
- Has ops accumulated domain complexity?
- Has API accumulated policy/orchestration logic?
- Are boundaries less obvious than in previous audit?

Check:

- [ ] No material drift pressure observed
- [ ] Drift pressure recorded below

Notes:

- (short evidence-backed observations)

## Output Requirements for Audit Results

When executing this audit, result file must include:

- exact evidence (file + line)
- pass/fail/ambiguous for each checklist item
- severity classification for violations
- explicit list of ambiguous areas

Severity scale:

- Critical
- High
- Medium
- Low

### Violation Classification Guidance

- `Critical`: upward import across core layer boundary.
- `Critical`: policy importing infra/runtime side-effect surfaces.
- `Critical`: storage/model importing workflow layer.
- `High`: DTO leakage into domain/model/storage.
- `High`: business decision logic embedded in ops.
- `Medium`: policy-like logic embedded in API boundary.
- `Medium`: limited orchestration behavior inside ops.
- `Low`: naming/placement ambiguity with no current behavioral impact.
- `Low`: minor utility placement drift.

## Final Verdict

Choose one:

- Pass — no layering violations
- Pass with drift risk — no hard violations, but trend risk exists
- Fail — one or more concrete layering violations detected

## Confidence

Record:

- Static scan confidence (`high/medium/low`)
- Manual inspection coverage (modules reviewed)
- Areas not deeply inspected
