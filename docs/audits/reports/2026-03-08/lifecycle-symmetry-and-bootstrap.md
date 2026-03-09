# Lifecycle Symmetry and Bootstrap Audit — 2026-03-08

## Run Context

- Audit run: `lifecycle-symmetry-and-bootstrap`
- Definition: `docs/audits/recurring/lifecycle-symmetry-and-bootstrap.md`
- Auditor: `codex`
- Date (UTC): `2026-03-08 17:08:39Z`
- Branch: `eleven`
- Commit: `c98bb574`
- Worktree: `dirty`
- Scope:
  - `crates/canic/src/macros/start.rs`
  - `crates/canic-core/src/api/lifecycle.rs`
  - `crates/canic-core/src/lifecycle/{init,upgrade}.rs`
  - `crates/canic-core/src/workflow/runtime/mod.rs`
  - `crates/canic-core/src/workflow/bootstrap/*`
  - lifecycle-related tests

## Checklist

### 1. Macro Hooks Stay Thin

- [x] Macros delegate to lifecycle API and timer API.
- [x] User lifecycle hooks are scheduled via timer with `Duration::ZERO`.
- [x] No business-policy/orchestration logic embedded in macro hooks.

Evidence:
- `crates/canic/src/macros/start.rs`

### 2. API Lifecycle Boundary Is Pure Delegation

- [x] Lifecycle API functions are direct delegates to lifecycle adapters.
- [x] No orchestration logic in API lifecycle boundary.

Evidence:
- `crates/canic-core/src/api/lifecycle.rs`

### 3. Init and Post-Upgrade Structure Is Symmetric

- [x] Both init and post-upgrade perform synchronous setup then schedule async bootstrap.
- [x] Root and non-root paths maintain equivalent execution model.
- [x] Differences remain explicit and phase-appropriate (trusted restore versus init payload).

Evidence:
- `crates/canic-core/src/lifecycle/init.rs`
- `crates/canic-core/src/lifecycle/upgrade.rs`

### 4. No Await in Synchronous Lifecycle Adapters

- [x] No top-level `.await` in lifecycle adapter call flow.
- [x] `.await` appears only inside scheduled async closures.

Evidence:
- `rg -n '\.await' crates/canic-core/src/lifecycle -g '*.rs'`

### 5. Environment Restoration Happens Before Bootstrap

- [x] Root env restoration occurs before bootstrap timer set in post-upgrade.
- [x] Non-root role restoration occurs before bootstrap timer set in post-upgrade.
- [x] Init runtime seeding happens before bootstrap timer set.

Evidence:
- `lifecycle/upgrade.rs` (`EnvOps::restore_root`, `EnvOps::restore_role`, then `TimerOps::set`)
- `lifecycle/init.rs` (`workflow::runtime::init_*`, then `TimerWorkflow::set`)

### 6. Lifecycle-to-Workflow Boundary Discipline

- [x] No direct policy embedding in lifecycle adapters.
- [x] No direct stable-storage mutation paths in lifecycle adapters.
- [x] Orchestration remains owned by workflow/bootstrap modules.

Evidence:
- `rg -n 'crate::ops::|crate::domain::policy|crate::storage::stable::' crates/canic-core/src/lifecycle -g '*.rs'` (no boundary violations found)

### 7. Timer and Bootstrap Coverage

- [x] Lifecycle boundary integration test executed and passed.
- [x] Lifecycle trap-phase coverage exists.
- [x] Additional repeated post-upgrade stress coverage exists for non-root readiness.

Executed test:
- `cargo test -p canic lifecycle_boundary_traps_are_phase_correct -- --nocapture` (pass)
- `cargo test -p canic --test lifecycle_boundary non_root_post_upgrade_remains_ready_across_repeated_upgrades -- --nocapture` (pass)
- `cargo test -p canic --test lifecycle_boundary non_root_post_upgrade_failure_reports_phase_error -- --nocapture` (pass)

## Findings

### High

- None.

### Medium

- None.

### Low

- None.

## Verdict

- Lifecycle symmetry: **Pass**
- Bootstrap scheduling safety: **Pass**
- Immediate follow-up: none
