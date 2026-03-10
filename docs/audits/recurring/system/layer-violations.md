# Audit: Layer Violations

## Purpose

Detect architectural drift against the Canic layering contract.

## Risk Model / Invariant

This is a drift audit, not a security invariant audit.

Risk model:

> Dependencies and behavior must follow the canonical direction:
> `endpoints/macros -> workflow -> policy -> ops -> model`.

## Why This Matters

Layer drift increases defect rate and weakens ownership boundaries, which slows safe change.

This audit verifies:

- dependency direction
- layer responsibilities
- cross-layer data leakage
- macro boundary correctness

## Run This Audit After

- architecture refactors
- moving logic across `api/workflow/policy/ops/model`
- macro/runtime dispatch changes
- large feature merges

## Canonical Layer Model

From `docs/contracts/ARCHITECTURE.md`:

```text
endpoints/macros
    ↓
workflow
    ↓
policy
    ↓
ops
    ↓
model
```

### Layer -> Directory Mapping

Canonical path ownership for this audit:

- `endpoints/macros`:
  - `crates/canic/src/macros/**`
  - `crates/canic-core/src/endpoints/**`
- `workflow`:
  - `crates/canic-core/src/workflow/**`
- `policy`:
  - `crates/canic-core/src/domain/policy/**`
  - `crates/canic-core/src/access/**`
- `ops`:
  - `crates/canic-core/src/ops/**`
- `model/storage`:
  - `crates/canic-core/src/model/**`
  - `crates/canic-core/src/storage/**`

### DTO Usage Rule

DTO types may appear in:

- `endpoints`
- `workflow`
- `ops`

DTO types must not appear in:

- `policy`
- `model`
- `storage`

### Policy Layer Naming

For this audit, `policy layer` means `crates/canic-core/src/domain/policy/**`.

### Model vs Storage Scope

- `model`: pure state structures and local invariants
- `storage`: persistence and stable-memory projection

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

Policy may depend on:

- domain value types
- topology information
- capability expressions
- pure helper utilities

Policy must not depend on:

- `workflow`
- `ops`
- `infra`
- serialization (`serde`, `candid`)
- async behavior

Side effects are allowed only in:

- `ops`
- infra adapters

## Run Context

Record in the audit result file:

- Date
- Auditor
- Branch
- Commit (`git rev-parse --short HEAD`)
- Workspace state (`clean`/`dirty`)
- Scope (paths reviewed)

Report preamble (required):

- Scope
- Compared baseline report path
- Code snapshot identifier
- Method tag/version
- Comparability status

## Audit Checklist

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

#### 1.4 Module Dependency Drift Signals

Detect cross-layer coupling signals even when direct violations are not yet obvious.

Suggested scans:

```bash
rg -n 'use crate::.*workflow.*policy' crates/canic-core/src -g '*.rs'
rg -n 'use crate::.*policy.*workflow' crates/canic-core/src -g '*.rs'
```

Check:

- [ ] No recurring workflow<->policy cross-import pattern
- [ ] Drift signals recorded below when frequent

Findings:

- (file, line, pattern, why risky)

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

- [ ] Side effects are implemented only in `ops` or infra adapters
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
- [ ] DTO <-> model mapping occurs in API/workflow adapters, never inside `model`/`storage`

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

Expansion check example:

```bash
cargo expand --lib endpoint_macro | rg -n 'policy'
```

Findings:

- (file, line, why)

## 6. Cyclic Crate Dependency Check

Run:

```bash
cargo tree -e features
```

Check:

- [ ] No crate-level cyclic dependency patterns introduced via features/re-exports
- [ ] No recurring module-level cycle signal (`workflow` <-> `policy`) detected

Findings:

- (crate path, why)

## 7. Drift Pressure Check

Qualitative drift prompts:

- Has any layer grown disproportionately?
- Has ops accumulated domain complexity?
- Has API accumulated policy/orchestration logic?
- Are boundaries less obvious than in previous audit?

Drift Pressure Indicators:

- `ops`: `LOC > 600` and `domain_count >= 3`
- `workflow`: imports `>= 3` domain concepts
- `api`: branching depth `> 2` layers

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

## Structural Hotspots

List concrete files/modules/structs that contribute to layer drift pressure.

Detection commands (run and record output references):

```bash
rg '^use ' crates/ -g '*.rs'
rg 'crate::workflow|crate::ops|crate::api|crate::policy' crates/ -g '*.rs'
rg 'pub struct|impl ' crates/ -g '*.rs'
git log --name-only -n 20 -- crates/
```

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `workflow/rpc/request/handler/*` | replay/authorize execution paths | workflow-policy/ops boundary pressure | Medium |
| `api/rpc/*` | endpoint boundary modules | API orchestration drift risk | Medium |
| `ops/*` | service facade modules | policy leakage risk into ops | Medium |

If none are detected in a given run, state: No structural hotspots detected in this run.

## Hub Module Pressure

Detect modules trending toward gravity-well behavior from import fan-in, cross-layer coupling, and edit frequency.

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `<module>` | `<top import tokens>` | `<n>` | `<n>` | `<1-10>` |

Pressure score guidance:

- 1-3 = low
- 4-6 = moderate
- 7-10 = high

## Architecture Watchpoint

Flag one primary module that should be treated as the near-term architectural watchpoint.

Default expected watchpoint in this codebase:

`crates/canic-core/src/workflow/runtime/mod.rs`

Record:

- why it is central
- current pressure score
- why it must remain thin orchestration only

## Responsibility Drift Signals

Detect behavioral layer drift even when imports still satisfy dependency rules.

If no drift signals are found, state exactly:

`No behavioral layer drift detected.`

### Workflow Layer Drift

Workflow should orchestrate and sequence, but must not absorb ops-like behavior.

Scan:

```bash
rg -n 'sign_|ecdsa_|hash|verify|stable_|env::|spawn' crates/canic-core/src/workflow -g '*.rs'
```

| Location | Pattern | Drift Risk | Why |
| --- | --- | --- | --- |
| `<path:line>` | `<pattern>` | `<Low/Medium/High>` | `<cross-layer behavior signal>` |

### Policy Layer Drift

Policy must remain pure and deterministic.

Scan:

```bash
rg -n '\\.await|spawn\\(|stable_|env::' crates/canic-core/src/{domain/policy,access} -g '*.rs'
```

| Location | Pattern | Drift Risk | Why |
| --- | --- | --- | --- |
| `<path:line>` | `<pattern>` | `<High>` | `<policy side-effect/async signal>` |

### Lifecycle Adapter Drift

Lifecycle adapters must stay thin and synchronous orchestration adapters.

Scans:

```bash
rg -n '\\.await|spawn\\(|workflow::|ops::' crates/canic-core/src/lifecycle -g '*.rs'
wc -l crates/canic-core/src/lifecycle/*.rs
```

Report lifecycle files exceeding `150` lines and any direct orchestration drift signals.

| File | Lines | Drift Signal | Risk |
| --- | ---: | --- | --- |
| `<path>` | `<count>` | `<signal>` | `<Low/Medium/High>` |

### DTO Responsibility Drift

DTO modules must remain passive transport shapes.

Scan:

```bash
rg -n 'impl .*fn|async fn' crates/canic-core/src/dto -g '*.rs'
```

| File | Signal | Risk |
| --- | --- | --- |
| `<path>` | `<behavior inside DTO>` | `<Low/Medium/High>` |

## Amplification Drivers (If Applicable)

When drift risk is tied to recent multi-file changes, record the largest amplifiers.

| Commit | Feature Slice | Files Touched | Subsystems | CAF | Risk |
| --- | --- | ---: | --- | ---: | --- |
| `<commit>` | `<feature>` | `<n>` | `<subsystems>` | `<caf>` | `<risk>` |

Detection commands (run and record output references):

```bash
git log --name-only -n 20 -- crates/
```

## Red Flags

- upward imports across core layers
- policy importing runtime side-effect surfaces
- workflow or API accumulating storage-mutation logic
- DTO leakage into domain/storage ownership

## Severity

- Critical: hard upward dependency violations or policy side-effect leakage
- High: DTO/domain/storage boundary breaks or business policy in wrong layer
- Medium: orchestration or policy leakage trends without hard break
- Low: ambiguity and placement drift with no current behavioral impact

## Early Warning Signals

Detect predictive architecture-decay patterns before they appear as friction or failures.

Detection scans (run and record output references):

```bash
rg 'enum ' crates/ -g '*.rs'
rg 'pub struct|pub fn' crates/ -g '*.rs'
rg '^use ' crates/ -g '*.rs'
git log --name-only -n 20 -- crates/
```

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| `<signal>` | `<path or module>` | `<scan evidence>` | `<Low/Medium/High>` |
| `dependency fan-in hub` | `<module path>` | `imported by <n> files across <subsystems>` | `<Low/Medium/High>` |

### Enum Shock Radius

| Enum | Defined In | Reference Files | Risk |
| --- | --- | ---: | --- |
| `<EnumName>` | `<path>` | `<count>` | `<Low/Medium/High>` |

Thresholds:

- `0-5` references = normal
- `6-10` = coupling forming
- `10+` = architectural shock radius

### Cross-Layer Struct Spread

| Struct | Defined In | Layers Referencing | Risk |
| --- | --- | --- | --- |
| `<StructName>` | `<path>` | `<api/workflow/ops/policy>` | `<Low/Medium/High>` |

### Growing Hub Modules

| Module | Subsystems Imported | Recent Commits | Risk |
| --- | --- | ---: | --- |
| `<path>` | `<subsystems>` | `<count>` | `<Low/Medium/High>` |

### Capability Surface Growth

| Module | Public Items | Risk |
| --- | ---: | --- |
| `<path>` | `<count pub fn + pub struct>` | `<Low/Medium/High>` |

Thresholds:

- `0-10` = normal
- `10-20` = growing surface
- `20+` = risk

If no predictive signals are detected, state: No predictive architectural signals detected in this run.

## Dependency Fan-In Pressure

Detect modules and structs becoming architectural gravity wells before friction increases.

Detection scans (run and record output references):

```bash
rg "use crate::" crates/ -g "*.rs"
rg "pub struct" crates/ -g "*.rs"
# then: rg "<StructName>" crates/ -g "*.rs"
```

### Module Fan-In

Count how many files import each module; flag modules imported by `6+` files.

| Module | Import Count | Layers Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `<module path>` | `<count>` | `<api/workflow/ops/policy/...>` | `<Low/Medium/High>` |

Pressure level rules:

- `0-3` imports = normal
- `4-6` imports = rising pressure
- `7-10` imports = hub forming
- `10+` imports = architectural gravity well

### DTO Fan-In (Expected)

DTO fan-in is expected for shared transport contracts.

Classify DTO fan-in rows explicitly as:

- `Expected DTO hub` (normal transport sharing), or
- `Escalating DTO hub` (if DTOs begin absorbing behavior/decision ownership)

### Struct Fan-In

Count references for public structs; flag structs referenced in `6+` files.

| Struct | Defined In | Reference Count | Risk |
| --- | --- | ---: | --- |
| `<StructName>` | `<path>` | `<count>` | `<Low/Medium/High>` |

Interpretation:

- `6-8` references = coupling forming
- `9-12` = hub abstraction
- `12+` = system dependency center

If no modules exceed the fan-in threshold, state: No fan-in pressure detected in this run.

## Risk Score

Risk Score: **X / 10**

Interpretation scale:

- 0-2 = negligible risk
- 3-4 = low risk
- 5-6 = moderate risk
- 7-8 = high risk
- 9-10 = critical architectural risk

Score must be justified using checklist findings and Structural Hotspots evidence.

Derivation guidance (deterministic):

- start at `0`
- add `+4` for any confirmed hard layering violation
- add `+2` per medium/high hotspot contribution (max `+4`)
- add `+2` if any hub module pressure score is `>= 7`
- add `+1` if enum shock radius is detected (`> 6` reference files)
- add `+1` if cross-layer struct spread is detected (`>= 3` architecture layers)
- add `+2` if growing hub module signal is detected
- add `+1` if capability public surface is `> 20` items
- add `+1` for fan-in `6-8` across multiple subsystems
- add `+2` for fan-in `9-12` across multiple subsystems
- add `+3` for fan-in `12+` across multiple subsystems
- add `+2` if workflow side-effect drift is detected
- add `+3` if policy async/side-effect drift is detected
- add `+1` if lifecycle adapter orchestration drift is detected
- add `+1` if DTO behavioral drift is detected
- clamp to `0..10`

If no confirmed findings and no hotspot/hub/amplification signals are present, score must remain `0-2`.
If no responsibility drift signals are detected, responsibility-drift additions must be `+0`.

## Architecture Health Interpretation

Summarize the run with a compact interpretation table.

| Dimension | Status |
| --- | --- |
| Layer invariants | `<Excellent/Good/At risk>` |
| Policy purity | `<Clean/Drifting>` |
| Lifecycle boundary | `<Stable/Drifting>` |
| Workflow orchestration | `<Healthy/Hub forming/Gravity well>` |
| DTO sharing | `<Expected/Escalating>` |

Add one-line interpretation, for example:

`healthy but centralizing runtime orchestration`

## Verification Readout

Use command outcomes with normalized statuses:

- `PASS`
- `FAIL`
- `BLOCKED`

## Follow-up Actions

If result is `FAIL`/`PARTIAL` or risk score is `>= 5`, include owner, action, and target report run.

If no action is needed, state: `No follow-up actions required.`

## Reporting Template

- Scope:
- Commit:
- Verdict:
- Violations:
- Ambiguous areas:
- Structural Hotspots:
- Hub Module Pressure:
- Architecture Watchpoint:
- Responsibility Drift Signals:
- Amplification Drivers:
- Early Warning Signals:
- Dependency Fan-In Pressure:
- Risk Score:
- Verification Readout:
- Follow-up actions:
