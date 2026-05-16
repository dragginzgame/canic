# Bootstrap Lifecycle Symmetry Audit

## Audit ID

`bootstrap-lifecycle-symmetry/current`

## Objective

Verify that IC lifecycle entrypoints remain thin synchronous adapters that restore the minimum runtime environment and schedule bootstrap orchestration asynchronously through the lifecycle timer boundary.

## Audit Type

Architecture drift audit.

This is **not** a security invariant audit.

## Non-Goals

This audit does not verify:

* correctness of bootstrap business behavior
* security properties of bootstrap tasks
* downstream workflow internals after scheduling
* host/operator backup or restore workflows in `canic-backup`, `canic-cli`, or
  `canic-host`
* general architectural quality outside lifecycle-boundary relevance

## Core Invariant

> Lifecycle hooks synchronously restore environment/runtime state and schedule bootstrap orchestration asynchronously. Lifecycle hooks do not execute orchestration directly.

## Why This Matters

Drift in lifecycle startup structure can introduce:

* nondeterministic initialization order
* init vs post-upgrade behavior skew
* restore-before-bootstrap regressions
* hidden business logic in lifecycle hooks or macros
* upgrade-only failures that escape normal startup testing

## Run After

* lifecycle/startup changes
* macro hook changes
* runtime restore/import changes
* timer/bootstrap workflow changes
* init/post-upgrade refactors
* workflow boundary changes affecting bootstrap scheduling

---

## Canonical References

Primary references:

* `AGENTS.md` lifecycle semantics
* `docs/contracts/ARCHITECTURE.md` lifecycle section

If either reference changes, record that in the report and treat comparability as potentially reduced.

Terminology note: this audit uses `restore` only for IC canister lifecycle
runtime/environment restoration. Snapshot backup/restore, restore apply
journals, and host-side operator restore commands are out of scope unless they
change canister lifecycle hooks or runtime restoration code directly.

---

## Canonical Contract

The following must remain true:

1. lifecycle macros are thin hook wiring only
2. lifecycle API functions are glue/delegation only
3. lifecycle adapters are synchronous
4. orchestration is scheduled through the lifecycle timer mechanism
5. `init` and `post_upgrade` remain structurally symmetric
6. environment/runtime restoration occurs before bootstrap scheduling
7. lifecycle code does not bypass layering by mutating storage or embedding policy/orchestration logic directly

## Canonical Lifecycle Pipeline

`restore environment -> initialize runtime state -> schedule bootstrap via lifecycle timer -> exit lifecycle hook`

Lifecycle code must not perform bootstrap orchestration directly.

---

## Scope

Audit these modules first:

* `crates/canic/src/macros/start.rs`
* `crates/canic-core/src/api/lifecycle/{mod.rs,root.rs,nonroot.rs}`
* `crates/canic-core/src/lifecycle/{init,upgrade}/{mod.rs,root.rs,nonroot.rs}`
* `crates/canic-control-plane/src/api/lifecycle.rs`
* `crates/canic-core/src/workflow/runtime/mod.rs`
* `crates/canic-core/src/workflow/runtime/{root.rs,nonroot.rs}`
* `crates/canic-core/src/workflow/bootstrap/*`

Optional supporting scope:

* lifecycle-related tests in `crates/canic-tests/tests`
* lifecycle-related tests in `crates/canic-core/tests`
* root fixture uses of `start_root!(init = { ... })`

---

## Required Run Context

Record all of the following in the result file:

* date
* auditor
* branch
* commit (`git rev-parse --short HEAD`)
* workspace state (`clean` or `dirty`)
* audited paths
* baseline report path
* code snapshot identifier
* method tag: `bootstrap-lifecycle-symmetry/current`
* comparability status: `comparable` | `partially comparable` | `not comparable`

### Comparability Rules

* `comparable`: same audit method version and same canonical references still apply
* `partially comparable`: minor scope or reference drift
* `not comparable`: audit method, architecture contract, or scope changed materially

---

## Result States

For each check, use exactly one:

* `[x]` Pass
* `[ ]` Fail
* `[~]` Ambiguous / follow-up required

Use normalized command outcomes in the verification readout:

* `PASS`
* `FAIL`
* `BLOCKED`

## Evidence Standard

Every finding must use this shape:

* `(path:line-range) observed behavior -> implication`

Example:

* `crates/canic-core/src/lifecycle/upgrade/nonroot.rs:42-56 bootstrap timer set before runtime restore -> violates restore-before-bootstrap ordering`

---

## Audit Procedure

### Phase 1: Evidence Collection

Run and record outputs or output references for these scans.

#### Lifecycle wiring and timer scheduling

```bash
rg -n 'init\(|post_upgrade\(|LifecycleApi::|TimerApi::set_lifecycle_timer|Duration::ZERO' \
  crates/canic/src/macros/start.rs
```

#### Lifecycle API delegation

```bash
rg -n 'pub fn init_|pub fn post_upgrade_|lifecycle::' \
  crates/canic-core/src/api/lifecycle crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'
```

#### Init/post-upgrade structure

```bash
rg -n 'init_root_canister|init_nonroot_canister|post_upgrade_root_canister|post_upgrade_nonroot_canister|TimerWorkflow::set|TimerOps::set|Duration::ZERO' \
  crates/canic-core/src/lifecycle crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'
```

#### Async behavior in lifecycle adapters

```bash
rg -n '\.await|async fn|spawn\(' crates/canic-core/src/lifecycle -g '*.rs'
```

#### Restore-before-bootstrap ordering

```bash
rg -n 'EnvOps::restore_|init_memory_registry_post_upgrade|workflow::runtime::init_|TimerOps::set|TimerWorkflow::set' \
  crates/canic-core/src/lifecycle crates/canic-core/src/workflow/runtime \
  crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'
```

#### Layering discipline

```bash
rg -n 'crate::ops::|crate::domain::policy|crate::storage::stable::' \
  crates/canic-core/src/lifecycle crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'
```

#### Test coverage

```bash
rg -n 'lifecycle|post_upgrade|init|bootstrap|Timer' \
  crates/canic-tests/tests crates/canic-core/tests -g '*.rs'
```

#### Root fixture coverage

```bash
rg -n 'start_root!\(|init = \{' canisters crates/canic-tests -g '*.rs'
```

---

### Phase 2: Contract Checks

## 1. Macro Hooks Stay Thin

Verify lifecycle macros only wire IC hooks and delegate to lifecycle/timer helpers.

Pass criteria:

* macros contain hook wiring only
* no policy, ops, model, or storage behavior is embedded
* no async orchestration is run directly from macros
* user hooks are scheduled or delegated, not awaited inline

Fail conditions:

* macro expands business logic beyond lifecycle/timer wiring
* macro directly runs orchestration logic
* macro introduces role/policy/storage branching unrelated to hook wiring

Checklist:

* [ ] Macros do not embed policy/ops/model/storage logic
* [ ] Macros do not run async orchestration directly
* [ ] User hooks are scheduled/delegated, not awaited

Findings:

* `(path:line-range) observed behavior -> implication`

## 2. Lifecycle API Boundary Is Pure Delegation

Verify lifecycle API is a glue layer that delegates to `lifecycle::*`.

Pass criteria:

* API functions are thin wrappers or direct delegation
* no workflow/bootstrap/runtime orchestration logic is introduced here

Fail conditions:

* API layer adds sequencing/orchestration logic
* API layer mutates lifecycle state directly
* API layer duplicates lifecycle adapter behavior

Checklist:

* [ ] API layer is glue only
* [ ] No direct workflow orchestration in API layer

Findings:

* `(path:line-range) observed behavior -> implication`

## 3. Init and Post-Upgrade Structure Is Symmetric

Verify root and non-root flows are structurally aligned.

Symmetry means same high-level phases in same order:

1. config/bootstrap prep
2. environment/runtime restoration
3. bootstrap scheduling through timer
4. exit

Allowed differences:

* trusted-state restoration specific to post-upgrade
* explicitly documented restore-path differences
* root bootstrap scheduling split between `canic-core` runtime restoration and
  `canic-control-plane` root orchestration, if the report records both halves

Fail conditions:

* init and post-upgrade diverge in high-level sequencing
* one path restores runtime before scheduling while the other does not
* one role performs direct orchestration while its counterpart only schedules bootstrap

Checklist:

* [ ] Root init and root post-upgrade are structurally aligned
* [ ] Non-root init and non-root post-upgrade are structurally aligned
* [ ] Differences are limited to documented restoration differences
* [ ] Root control-plane scheduling split is reviewed explicitly

Findings:

* `(path:line-range) observed behavior -> implication`

## 4. Lifecycle Adapters Remain Synchronous

Verify lifecycle adapters do not implement async orchestration directly.

Pass criteria:

* no `.await` in lifecycle adapter code
* no async lifecycle adapter entrypoints performing orchestration
* no direct spawn/orchestration in lifecycle layer
* async work enters only via timer/bootstrap workflow

Fail conditions:

* lifecycle adapter awaits or spawns orchestration directly
* lifecycle adapter contains embedded async execution path
* lifecycle path bypasses timer boundary

Checklist:

* [ ] No `.await` in lifecycle adapter code
* [ ] No async lifecycle adapter functions performing orchestration directly
* [ ] No direct spawn/orchestration paths in lifecycle adapters
* [ ] Async work is triggered only through timer bootstrap workflow

Findings:

* `(path:line-range) observed behavior -> implication`

## 5. Environment Restoration Happens Before Bootstrap Scheduling

Verify restore/init happens before scheduling continuation.

Pass criteria:

* root environment restoration precedes bootstrap scheduling
* non-root role restoration precedes bootstrap scheduling
* runtime init/restoration completes before timer scheduling
* trap/failure paths prevent continuation scheduling where required

Fail conditions:

* bootstrap scheduled before required restoration
* continuation scheduled after partial restore failure
* runtime init ordering differs materially between symmetric paths without documentation

Checklist:

* [ ] Root env restoration precedes bootstrap scheduling
* [ ] Non-root role restoration precedes bootstrap scheduling
* [ ] Runtime state init/restoration completes before scheduling bootstrap
* [ ] Failure paths trap before scheduling continuation where required

Findings:

* `(path:line-range) observed behavior -> implication`

## 6. Lifecycle-to-Workflow Boundary Discipline

Verify lifecycle code does not bypass layering.

Pass criteria:

* lifecycle layer does not mutate storage directly
* lifecycle layer does not embed domain policy
* workflow/bootstrap remains orchestration owner

Fail conditions:

* lifecycle code imports stable storage mutation paths directly
* lifecycle code contains domain policy logic
* lifecycle code owns bootstrap orchestration sequencing

Checklist:

* [ ] Lifecycle adapters do not mutate storage directly
* [ ] Lifecycle adapters do not embed policy logic
* [ ] Workflow/bootstrap remains orchestration owner

Findings:

* `(path:line-range) observed behavior -> implication`

Additional check for root control-plane lifecycle:

* root control-plane lifecycle may register bootstrap modules and schedule
  timers, but must not perform the bootstrap work inline
* root control-plane lifecycle must delegate runtime restoration to
  `canic-core` before scheduling root bootstrap timers

## 7. Timer and Bootstrap Coverage

Verify tests exercise lifecycle boundary behavior.

Pass criteria:

* lifecycle boundary tests exist
* init and post-upgrade paths are both covered
* timer/bootstrap boundary semantics are exercised
* known gaps are documented explicitly

Checklist:

* [ ] Lifecycle boundary tests exist
* [ ] Post-upgrade path is exercised in tests
* [ ] Timer/bootstrap boundary is exercised in tests
* [ ] Gaps are documented explicitly

Findings:

* `(test file) coverage or gap`

---

## Primary Result

Set one overall result:

* `PASS`: all contract checks pass
* `PARTIAL`: no confirmed contract break, but one or more ambiguous checks or significant coverage gaps
* `FAIL`: one or more confirmed contract breaks
* `BLOCKED`: audit could not be completed due to missing code, broken tooling, or incompatible workspace state

Decision rule:

* any confirmed failure in checks 1-6 => `FAIL`
* only check 7 failing => `PARTIAL`, unless missing coverage conceals a confirmed contract break
* any `[~]` without confirmed break => `PARTIAL`

---

## Structural Hotspots

Identify only concrete lifecycle-drift hotspots.

Detection scans:

```bash
rg '^use ' crates/ -g '*.rs'
rg 'crate::workflow|crate::ops|crate::api|crate::policy' crates/ -g '*.rs'
rg 'pub struct|impl ' crates/ -g '*.rs'
git log --name-only -n 20 -- crates/
```

Record only modules that materially affect lifecycle drift risk.

| File / Module                            | Struct / Function                | Reason                                    | Risk Contribution |
| ---------------------------------------- | -------------------------------- | ----------------------------------------- | ----------------- |
| `canic/src/macros/start.rs`              | lifecycle macro hooks            | hook wiring and timer scheduling boundary | High              |
| `canic-core/src/lifecycle/init/*`        | init adapters                    | startup ordering and scheduling boundary  | High              |
| `canic-core/src/lifecycle/upgrade/*`     | post-upgrade adapters            | restore-before-bootstrap sequencing       | High              |
| `canic-control-plane/src/api/lifecycle.rs` | root lifecycle scheduling      | root bootstrap timer boundary             | High              |
| `canic-core/src/workflow/runtime/mod.rs` | runtime init/restore entrypoints | lifecycle restore surface                 | Medium            |
| `canic-core/src/workflow/runtime/{root,nonroot}.rs` | post-restore runtime continuation | role-specific runtime restore continuation | Medium |

If none are identified, state: `No structural hotspots detected in this run.`

---

## Secondary Trend Signals

This section is advisory only. It must not by itself produce `FAIL`.

### Hub Module Pressure

| Module     | Import Tokens         | Unique Subsystems | Cross-Layer Count | Pressure Score |
| ---------- | --------------------- | ----------------: | ----------------: | -------------: |
| `<module>` | `<top import tokens>` |             `<n>` |             `<n>` |       `<1-10>` |

Pressure score guidance:

* `1-3` low
* `4-6` moderate
* `7-10` high

### Early Warning Signals

Detection scans:

```bash
rg 'enum ' crates/ -g '*.rs'
rg 'pub struct|pub fn' crates/ -g '*.rs'
rg '^use ' crates/ -g '*.rs'
git log --name-only -n 20 -- crates/
```

| Signal     | Location | Evidence     | Risk                |
| ---------- | -------- | ------------ | ------------------- |
| `<signal>` | `<path>` | `<evidence>` | `<Low/Medium/High>` |

If none are detected, state: `No predictive architectural signals detected in this run.`

### Enum Shock Radius

| Enum         | Defined In | Reference Files | Risk                |
| ------------ | ---------- | --------------: | ------------------- |
| `<EnumName>` | `<path>`   |       `<count>` | `<Low/Medium/High>` |

Thresholds:

* `0-5` normal
* `6-10` coupling forming
* `10+` architectural shock radius

### Cross-Layer Struct Spread

| Struct         | Defined In | Layers Referencing          | Risk                |
| -------------- | ---------- | --------------------------- | ------------------- |
| `<StructName>` | `<path>`   | `<api/workflow/ops/policy>` | `<Low/Medium/High>` |

### Growing Hub Modules

| Module   | Subsystems Imported | Recent Commits | Risk                |
| -------- | ------------------- | -------------: | ------------------- |
| `<path>` | `<subsystems>`      |      `<count>` | `<Low/Medium/High>` |

### Capability Surface Growth

| Module   | Public Items | Risk                |
| -------- | -----------: | ------------------- |
| `<path>` |    `<count>` | `<Low/Medium/High>` |

Thresholds:

* `0-10` normal
* `11-20` growing surface
* `21+` elevated risk

---

## Dependency Fan-In Pressure

This section is also advisory unless it clearly contributes to lifecycle drift risk.

Detection scans:

```bash
rg "use crate::" crates/ -g "*.rs"
rg "pub struct" crates/ -g "*.rs"
# then: rg "<StructName>" crates/ -g "*.rs"
```

### Module Fan-In

Flag modules imported by `6+` files.

| Module          | Import Count | Subsystems Referencing          | Pressure Level      |
| --------------- | -----------: | ------------------------------- | ------------------- |
| `<module path>` |    `<count>` | `<api/workflow/ops/policy/...>` | `<Low/Medium/High>` |

Pressure levels:

* `0-3` normal
* `4-6` rising pressure
* `7-10` hub forming
* `10+` gravity well

### Struct Fan-In

Flag public structs referenced in `6+` files.

| Struct         | Defined In | Reference Count | Risk                |
| -------------- | ---------- | --------------: | ------------------- |
| `<StructName>` | `<path>`   |       `<count>` | `<Low/Medium/High>` |

Interpretation:

* `6-8` coupling forming
* `9-12` hub abstraction
* `12+` system dependency center

If none exceed threshold, state: `No fan-in pressure detected in this run.`

---

## Red Flags

Any confirmed red flag must appear in findings and affect score:

* bootstrap scheduled before environment restoration
* lifecycle adapter awaiting orchestration directly
* init/post-upgrade structural drift between equivalent roles
* lifecycle layer importing policy or storage mutation paths
* lifecycle hook performing direct bootstrap orchestration
* init/post-upgrade using materially different runtime initialization paths without documented rationale
* `start_root!(init = { ... })` or `start!(init = { ... })` user code running
  synchronously inside generated IC lifecycle hooks
* root control-plane lifecycle scheduling bootstrap before `canic-core`
  restoration completes

---

## Severity Guidance

* `Critical`: bootstrap can run before restoration, or lifecycle awaits async work directly
* `High`: init/post-upgrade structural drift affects deterministic startup
* `Medium`: timer scheduling inconsistency, restore/trap path drift, or layering erosion
* `Low`: observability or coverage gaps without direct contract break

---

## Risk Score

Risk Score: **X / 10**

Deterministic derivation:

* start at `0`
* add `+4` for any confirmed lifecycle contract break from checks 1-6
* add `+2` per high-impact structural hotspot actively implicated in findings, max `+4`
* add `+2` if any hub module pressure score is `>= 7`
* add `+1` if enum shock radius is detected
* add `+1` if cross-layer struct spread is detected
* add `+2` if growing hub module signal is detected
* add `+1` if capability public surface is `> 20`
* add `+1` for fan-in `6-8` across multiple subsystems
* add `+2` for fan-in `9-12` across multiple subsystems
* add `+3` for fan-in `12+` across multiple subsystems
* clamp to `0..10`

Scoring rule:

* if no confirmed findings and no meaningful advisory signals, score must remain `0-2`
* advisory signals alone should not push score above `4` unless they are directly connected to audited lifecycle modules

---

## Verification Readout

Use this format:

| Check                          | Status    | Evidence                                          |
| ------------------------------ | --------- | ------------------------------------------------- |
| Macro hooks stay thin          | `PASS`    | `crates/canic/src/macros/start.rs:12-38`          |
| API boundary pure delegation   | `PASS`    | `crates/canic-core/src/api/lifecycle/nonroot.rs:5-22` |
| Root control-plane split       | `PASS`    | `crates/canic-control-plane/src/api/lifecycle.rs:18-93` |
| Init/post-upgrade symmetry     | `PASS`    | `lifecycle/init/nonroot.rs:10-48`, `lifecycle/upgrade/nonroot.rs:12-55` |
| Lifecycle adapters synchronous | `PASS`    | `rg async/await/spawn returned no violating hits` |
| Restore before bootstrap       | `PASS`    | `lifecycle/init/...`, `lifecycle/upgrade/...`     |
| Boundary discipline            | `PASS`    | `no forbidden imports in lifecycle/`              |
| Test coverage                  | `PARTIAL` | `post-upgrade coverage present, timer gap in ...` |

---

## Follow-up Actions

Include only if result is `FAIL` or `PARTIAL`, or risk score is `>= 5`.

| Owner     | Action               | Target Run            |
| --------- | -------------------- | --------------------- |
| `<owner>` | `<fix or follow-up>` | `<next audit/report>` |

If none: `No follow-up actions required.`

---

## Report Template

* Scope:
* Baseline report:
* Code snapshot identifier:
* Method tag/version:
* Comparability status:
* Commit:
* Workspace state:
* Lifecycle entrypoints reviewed:
* Result:
* Symmetry findings:
* Boundary findings:
* Structural Hotspots:
* Hub Module Pressure:
* Early Warning Signals:
* Dependency Fan-In Pressure:
* Risk Score:
* Verification Readout:
* Follow-up actions:
