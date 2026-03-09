# Audit: Lifecycle Symmetry and Bootstrap Safety

## Purpose

Ensure IC lifecycle entrypoints remain thin, synchronous adapters with
consistent `init` and `post_upgrade` behavior and non-blocking bootstrap
scheduling.

Reliability invariant:

> Lifecycle hooks restore minimum environment state synchronously and schedule
> async bootstrap work; they never await orchestration directly.

## Canonical Contract

Primary references:
- `AGENTS.md` lifecycle semantics
- `docs/contracts/ARCHITECTURE.md` lifecycle section

Required lifecycle guarantees:
1. macros are thin and contain no business logic
2. lifecycle adapters do not await async work
3. bootstrap is scheduled via timer (`Duration::ZERO`)
4. `init` and `post_upgrade` keep equivalent execution structure
5. environment restoration happens before async bootstrap

## Scope

Audit these modules first:
- `crates/canic/src/macros/start.rs`
- `crates/canic-core/src/api/lifecycle.rs`
- `crates/canic-core/src/lifecycle/init.rs`
- `crates/canic-core/src/lifecycle/upgrade.rs`
- `crates/canic-core/src/workflow/runtime/mod.rs`
- `crates/canic-core/src/workflow/bootstrap/*`

## Run Context

Record in the result file:
- date
- auditor
- branch
- commit (`git rev-parse --short HEAD`)
- workspace state (`clean` or `dirty`)
- audited paths

## Checklist

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

### 4. No Await in Synchronous Lifecycle Adapters

Verify lifecycle adapters never await directly.

Suggested scans:

```bash
rg -n '\\.await' crates/canic-core/src/lifecycle -g '*.rs'
```

- [ ] No `.await` in synchronous lifecycle adapter code
- [ ] Async work only appears inside scheduled closures

Findings:
- (file, line, behavior)

### 5. Environment Restoration Happens Before Bootstrap

Verify environment restoration (`EnvOps::restore_*` and runtime init) happens
before timer-scheduled bootstrap tasks.

Suggested scans:

```bash
rg -n 'EnvOps::restore_|init_memory_registry_post_upgrade|workflow::runtime::init_|TimerOps::set|TimerWorkflow::set' \
  crates/canic-core/src/lifecycle/{init.rs,upgrade.rs} crates/canic-core/src/workflow/runtime/mod.rs -g '*.rs'
```

- [ ] Root env restoration precedes bootstrap scheduling
- [ ] Non-root role restoration precedes bootstrap scheduling
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
rg -n 'lifecycle|post_upgrade|init|bootstrap|Timer' crates/canic/tests crates/canic-core/tests -g '*.rs'
```

- [ ] Lifecycle boundary tests exist
- [ ] Post-upgrade path is exercised in integration tests
- [ ] Gaps are documented explicitly

Findings:
- (test file, missing case)

## Severity Guide

- Critical: bootstrap can run before env restoration or lifecycle awaits async work
- High: init/post-upgrade structural drift breaks deterministic startup
- Medium: timer scheduling inconsistency or trap path drift
- Low: observability/test coverage gaps without direct startup break

## Audit Frequency

Run this audit:
- after lifecycle, startup, or macro changes
- after environment restore/import changes
- after timer/bootstrap workflow changes
- before each release cut
