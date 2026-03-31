# Audit: Bootstrap Lifecycle Symmetry

## Purpose

Ensure IC lifecycle entrypoints remain thin synchronous adapters that
schedule orchestration through the bootstrap workflow.

## Risk Model / Invariant

This is a drift audit, not a security invariant audit.

Risk model:

> Lifecycle hooks restore the minimum runtime environment synchronously and
> schedule orchestration asynchronously.

## Why This Matters

Startup-order drift can create nondeterministic initialization and upgrade regressions.

## Run This Audit After

- lifecycle/startup changes
- macro hook changes
- runtime restore/import changes
- timer/bootstrap workflow changes

## Audit Checklist

### Canonical Contract

Primary references:
- `AGENTS.md` lifecycle semantics
- `docs/contracts/ARCHITECTURE.md` lifecycle section

Required lifecycle guarantees:
1. macros are thin and contain no business logic
2. lifecycle adapters remain synchronous adapters
3. bootstrap orchestration is scheduled via the lifecycle timer mechanism, not executed directly in lifecycle hooks
4. `init` and `post_upgrade` keep equivalent execution structure
5. environment and runtime state are restored before bootstrap scheduling

### Canonical Lifecycle Pipeline

`restore environment -> initialize runtime state -> schedule bootstrap via timer -> exit lifecycle hook`

Lifecycle adapters must not perform orchestration directly.

### Scope

Audit these modules first:
- `crates/canic/src/macros/start.rs`
- `crates/canic-core/src/api/lifecycle.rs`
- `crates/canic-core/src/lifecycle/init.rs`
- `crates/canic-core/src/lifecycle/upgrade.rs`
- `crates/canic-core/src/workflow/runtime/mod.rs`
- `crates/canic-core/src/workflow/bootstrap/*`

### Run Context

Record in the result file:
- date
- auditor
- branch
- commit (`git rev-parse --short HEAD`)
- workspace state (`clean` or `dirty`)
- audited paths

Report preamble (required):
- scope
- compared baseline report path
- code snapshot identifier
- method tag/version
- comparability status

Mark each item:
- `[x]` Pass
- `[ ]` Fail
- `[~]` Ambiguous or follow-up needed

### 1. Macro Hooks Stay Thin

Verify lifecycle macros only wire IC hooks and delegate to lifecycle API/timer
helpers.

Suggested scans:

```bash
rg -n 'init\\(|post_upgrade\\(|LifecycleApi::|TimerApi::set_lifecycle_timer|Duration::ZERO' \
  crates/canic/src/macros/start.rs
```

- [ ] Macros do not embed policy/ops/model logic
- [ ] Macros do not run async orchestration directly
- [ ] User hooks are scheduled, not awaited

Findings:
- (file, line, behavior)

### 2. API Lifecycle Boundary Is Pure Delegation

Verify lifecycle API is direct delegation to `lifecycle::*` with no orchestration.

Suggested scans:

```bash
rg -n 'pub fn init_|pub fn post_upgrade_|lifecycle::' \
  crates/canic-core/src/api/lifecycle.rs
```

- [ ] API layer is glue only
- [ ] No direct workflow orchestration in API layer

Findings:
- (file, line, behavior)

### 3. Init and Post-Upgrade Structure Is Symmetric

Verify both lifecycle phases follow the same high-level phases:
- config/bootstrap prep
- runtime/environment restoration
- schedule async bootstrap via timer

Structural symmetry means both phases perform the same high-level steps in
the same order, except where upgrade state restoration requires documented
differences.

Suggested scans:

```bash
rg -n 'init_root_canister|init_nonroot_canister|post_upgrade_root_canister|post_upgrade_nonroot_canister|TimerWorkflow::set|TimerOps::set|Duration::ZERO' \
  crates/canic-core/src/lifecycle/{init.rs,upgrade.rs}
```

- [ ] Root init and root post-upgrade are structurally aligned
- [ ] Non-root init and non-root post-upgrade are structurally aligned
- [ ] Explicit, documented differences are limited to trusted-state restoration differences

Findings:
- (file, line, behavior)

### 4. Lifecycle Adapters Remain Synchronous

Verify lifecycle adapters do not implement async orchestration behavior directly.

Suggested scans:

```bash
rg -n '\\.await|async fn|spawn\\(' crates/canic-core/src/lifecycle -g '*.rs'
```

- [ ] No `.await` in synchronous lifecycle adapter code
- [ ] No async lifecycle adapter functions performing orchestration directly
- [ ] No direct spawn/orchestration paths in lifecycle adapters
- [ ] Async work is triggered only through timer bootstrap workflow

Findings:
- (file, line, behavior)

### 5. Environment Restoration Happens Before Bootstrap

Verify environment restoration (`EnvOps::restore_*` and runtime init) happens
before bootstrap tasks are scheduled.

Suggested scans:

```bash
rg -n 'EnvOps::restore_|init_memory_registry_post_upgrade|workflow::runtime::init_|TimerOps::set|TimerWorkflow::set' \
  crates/canic-core/src/lifecycle/{init.rs,upgrade.rs} crates/canic-core/src/workflow/runtime/mod.rs -g '*.rs'
```

- [ ] Root env restoration precedes bootstrap scheduling
- [ ] Non-root role restoration precedes bootstrap scheduling
- [ ] Runtime state init/restoration completes before scheduling bootstrap
- [ ] Failures trap before scheduling continuation where required

Findings:
- (file, line, behavior)

### 6. Lifecycle-to-Workflow Boundary Discipline

Verify lifecycle adapters do not bypass layering contracts.

Suggested scans:

```bash
rg -n 'crate::ops::|crate::domain::policy|crate::storage::stable::' \
  crates/canic-core/src/lifecycle -g '*.rs'
```

- [ ] Lifecycle adapters do not mutate storage directly
- [ ] Lifecycle adapters do not embed policy logic
- [ ] Workflow/bootstrap remains the orchestration owner

Findings:
- (file, line, behavior)

### 7. Timer and Bootstrap Coverage

Verify coverage exists for lifecycle boundary behavior and post-upgrade/init
phase semantics.

Suggested scans:

```bash
rg -n 'lifecycle|post_upgrade|init|bootstrap|Timer' crates/canic-tests/tests crates/canic-core/tests -g '*.rs'
```

- [ ] Lifecycle boundary tests exist
- [ ] Post-upgrade path is exercised in integration tests
- [ ] Gaps are documented explicitly

Findings:
- (test file, missing case)

## Structural Hotspots

List concrete files/modules/structs that carry lifecycle drift risk.

Detection commands (run and record output references):

```bash
rg '^use ' crates/ -g '*.rs'
rg 'crate::workflow|crate::ops|crate::api|crate::policy' crates/ -g '*.rs'
rg 'pub struct|impl ' crates/ -g '*.rs'
git log --name-only -n 20 -- crates/
```

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `canic/src/macros/start.rs` | lifecycle macro hooks | lifecycle hook wiring and timer scheduling | High |
| `canic-core/src/lifecycle/init.rs` | init adapters | init ordering and timer boundaries | High |
| `canic-core/src/lifecycle/upgrade.rs` | post-upgrade adapters | restore-before-bootstrap sequencing | High |
| `canic-core/src/workflow/runtime/mod.rs` | runtime init/restore entrypoints | lifecycle state restore surface | Medium |

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

## Red Flags

- bootstrap scheduled before environment restoration
- lifecycle adapter awaiting orchestration directly
- init/post-upgrade phase structure drifting between roles
- lifecycle layer importing policy/storage mutation paths
- lifecycle hook performing direct workflow orchestration
- init/post_upgrade performing different runtime initialization paths without documented rationale

## Severity

- Critical: bootstrap can run before env restoration or lifecycle awaits async work
- High: init/post-upgrade structural drift breaks deterministic startup
- Medium: timer scheduling inconsistency or trap path drift
- Low: observability/test coverage gaps without direct startup break

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

| Module | Import Count | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `<module path>` | `<count>` | `<api/workflow/ops/policy/...>` | `<Low/Medium/High>` |

Pressure level rules:

- `0-3` imports = normal
- `4-6` imports = rising pressure
- `7-10` imports = hub forming
- `10+` imports = architectural gravity well

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
- add `+4` for any confirmed lifecycle contract break
- add `+2` per medium/high hotspot contribution (max `+4`)
- add `+2` if any hub module pressure score is `>= 7`
- add `+1` if enum shock radius is detected (`> 6` reference files)
- add `+1` if cross-layer struct spread is detected (`>= 3` architecture layers)
- add `+2` if growing hub module signal is detected
- add `+1` if capability public surface is `> 20` items
- add `+1` for fan-in `6-8` across multiple subsystems
- add `+2` for fan-in `9-12` across multiple subsystems
- add `+3` for fan-in `12+` across multiple subsystems
- clamp to `0..10`

If no confirmed findings and no hotspot/hub signals are present, score must remain `0-2`.

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
- Lifecycle entrypoints reviewed:
- Result:
- Symmetry findings:
- Boundary findings:
- Structural Hotspots:
- Hub Module Pressure:
- Early Warning Signals:
- Dependency Fan-In Pressure:
- Risk Score:
- Verification Readout:
- Follow-up actions:
