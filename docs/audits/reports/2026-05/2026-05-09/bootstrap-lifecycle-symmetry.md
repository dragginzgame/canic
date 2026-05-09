# Bootstrap Lifecycle Symmetry Audit - 2026-05-09

## Report Preamble

- Definition path: `docs/audits/recurring/system/bootstrap-lifecycle-symmetry.md`
- Scope: `crates/canic/src/macros/start.rs`,
  `crates/canic-core/src/api/lifecycle/**`,
  `crates/canic-core/src/lifecycle/{init,upgrade}/**`,
  `crates/canic-core/src/workflow/runtime/mod.rs`,
  `crates/canic-control-plane/src/api/lifecycle.rs`, and the only current
  `start_root!(init = ...)` fixture
- Compared baseline report path: `docs/audits/reports/2026-04/2026-04-05/bootstrap-lifecycle-symmetry.md`
- Code snapshot identifier: `518f57dd`
- Method tag/version: `bootstrap-lifecycle-symmetry/current`
- Comparability status: `partially comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-09T13:54:14Z`
- Branch: `main`
- Worktree: `dirty`

## Audit Selection

This was selected from the oldest tied recurring system reports last refreshed
on `2026-04-05`. It was prioritized from that tied set because lifecycle
semantics are explicitly named in `AGENTS.md`, and the current audit template
had already been updated during the 0.33 cleanup pass.

The run is partially comparable with the April baseline because the lifecycle
module split is stable, but the normative lifecycle contract is now stricter:
user hooks are expected to run through zero-delay timers after Canic invariants
are restored.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Macro hooks stay thin | FIXED | `crates/canic/src/macros/start.rs:35-42`, `57-64`, `205-212`, and `228-235` now delegate optional `init = { ... }` blocks through `__canic_after_optional_start_init_hook!(...)` instead of running them inline. |
| User hooks run through lifecycle timers | FIXED | `crates/canic/src/macros/start.rs:243-257` schedules optional init blocks with `TimerApi::set_lifecycle_timer(Duration::ZERO, ...)`, then schedules the bootstrap/user continuation timers. |
| Lifecycle API remains delegation-only | PASS | `crates/canic-core/src/api/lifecycle/nonroot.rs` and `root.rs` only delegate into lifecycle modules; `crates/canic-control-plane/src/api/lifecycle.rs:18-93` performs root bootstrap module registration plus timer scheduling. |
| Init/post-upgrade execution model symmetry | PASS | Non-root init at `crates/canic-core/src/lifecycle/init/nonroot.rs:16-58` and post-upgrade at `upgrade/nonroot.rs:16-75` run config/runtime restore before scheduling. Root init and upgrade follow the same split through `canic-core` plus `canic-control-plane`. |
| No direct await in lifecycle adapters | PASS | The async scan only found `.await` inside timer closures in `start.rs`, `canic-control-plane/src/api/lifecycle.rs`, and non-root bootstrap scheduling functions. |
| Restore-before-bootstrap ordering | PASS | `bootstrap::init_compiled_config`, memory registry post-upgrade init, and `EnvOps::restore_*` all happen before `TimerWorkflow::set`, `TimerOps::set`, or control-plane `TimerApi::set_lifecycle_timer` bootstrap scheduling. |
| Lifecycle modules avoid storage/policy shortcuts | PASS | `rg -n 'crate::ops::\|crate::domain::policy\|crate::storage::stable::' crates/canic-core/src/lifecycle crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'` returned no matches. |
| Runtime coverage exercises lifecycle boundary | PASS | `lifecycle_boundary`, root post-upgrade reconcile, and the role-attestation root fixture all passed after the macro remediation. |

## Remediation Applied

| Change | Files | Result |
| --- | --- | --- |
| Moved optional macro `init = { ... }` execution out of synchronous lifecycle hook bodies | `crates/canic/src/macros/start.rs` | User-provided init blocks now run from a zero-delay lifecycle timer. |
| Preserved pre-bootstrap ordering for optional init blocks | `crates/canic/src/macros/start.rs` | The optional init-block timer schedules the existing bootstrap and user lifecycle timers after the block finishes, so current root test fixtures can still seed staged releases before bootstrap. |
| Removed the old inline helper macro path | `crates/canic/src/macros/start.rs` | No `__canic_run_start_init_hook!` path remains. |

## Comparison to Previous Relevant Run

- Stable: core lifecycle adapters still restore config/env/runtime state
  synchronously and schedule bootstrap orchestration through timers.
- Stable: root scheduling remains split between `canic-core` runtime restore and
  `canic-control-plane` root bootstrap timers.
- Improved: optional macro `init = { ... }` blocks no longer run inline in
  lifecycle hook bodies.
- Stable: non-root and root bootstrap orchestration still happens only in timer
  closures.
- Stable: no direct storage/policy shortcuts were found in lifecycle adapters.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic/src/macros/start.rs` | lifecycle core macros, `__canic_after_optional_start_init_hook!` | IC hook generation and user-hook/bootstrap timer ordering | High |
| `crates/canic-core/src/lifecycle/init/nonroot.rs` | `init_nonroot_canister_before_bootstrap`, `schedule_init_nonroot_bootstrap` | Non-root init restore and bootstrap timer split | High |
| `crates/canic-core/src/lifecycle/upgrade/nonroot.rs` | `post_upgrade_nonroot_canister_before_bootstrap`, `schedule_post_upgrade_nonroot_bootstrap` | Non-root post-upgrade restore and bootstrap timer split | High |
| `crates/canic-core/src/lifecycle/init/root.rs` | `init_root_canister_before_bootstrap` | Root init runtime restore before control-plane scheduling | Medium |
| `crates/canic-core/src/lifecycle/upgrade/root.rs` | `post_upgrade_root_canister_before_bootstrap` | Root post-upgrade memory/env restore before control-plane scheduling | Medium |
| `crates/canic-control-plane/src/api/lifecycle.rs` | root schedule functions | Root bootstrap orchestration timer boundary | Medium |
| `canisters/test/delegation_root_stub/src/lib.rs` | `start_root!(init = { ... })` | Current only fixture exercising optional init-block ordering | Medium |

## Hub Module Pressure

| Module | Fan-In Evidence | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| lifecycle macro core | start macro scan found 6 generated hook paths and 5 lifecycle timer bundles | 3 | 2 | 6 |
| lifecycle API adapters | API delegation scan found 13 direct lifecycle wrapper functions/calls | 3 | 2 | 5 |
| core lifecycle adapters | init/upgrade scan found 4 restore functions and 4 bootstrap scheduling functions | 3 | 2 | 5 |
| root control-plane lifecycle | root lifecycle spans template registration, metrics, core restore, and timer scheduling | 4 | 3 | 6 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| inline hook drift | `crates/canic/src/macros/start.rs` | Found and fixed optional `init = { ... }` blocks running synchronously in lifecycle hooks | Fixed |
| root lifecycle split | `canic-core` + `canic-control-plane` | Root restore and bootstrap scheduling intentionally live in different crates | Medium |
| timer ordering pressure | optional init-block support | Optional init blocks must complete before bootstrap continuation timers are scheduled | Medium |
| fixture dependency | `canisters/test/delegation_root_stub` | The role-attestation fixture relies on pre-bootstrap release staging | Low |

## Risk Score

Initial Risk Score: **4 / 10**

Post-remediation Risk Score: **2 / 10**

Initial score contributions:

- `+2` optional `init = { ... }` blocks ran synchronously in lifecycle hook
  bodies, conflicting with the current timer-based user-hook rule.
- `+1` root lifecycle remains split across `canic-core` and
  `canic-control-plane`.
- `+1` optional init-block ordering remains a specialized path used by a root
  test fixture.

Remediation removed the inline-hook contribution by moving optional init-block
execution behind `TimerApi::set_lifecycle_timer(Duration::ZERO, ...)` while
preserving pre-bootstrap ordering.

Verdict: **Invariant holds after remediation with low residual lifecycle
ordering risk.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n 'init\(\|post_upgrade\(\|LifecycleApi::\|TimerApi::set_lifecycle_timer\|Duration::ZERO\|__canic_after_optional_start_init_hook' crates/canic/src/macros/start.rs` | PASS | Macro hook and timer wiring scan recorded the remediated optional init-block path. |
| `rg -n 'pub fn init_\|pub fn post_upgrade_\|lifecycle::\|schedule_.*bootstrap' crates/canic-core/src/api/lifecycle crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'` | PASS | Lifecycle API delegation scan recorded core/control-plane wrappers. |
| `rg -n 'init_root_canister\|init_nonroot_canister\|post_upgrade_root_canister\|post_upgrade_nonroot_canister\|TimerWorkflow::set\|TimerOps::set\|Duration::ZERO\|schedule_.*bootstrap' crates/canic-core/src/lifecycle crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'` | PASS | Init/upgrade and timer scheduling scan recorded expected lifecycle boundaries. |
| `rg -n 'EnvOps::restore_\|init_memory_registry_post_upgrade\|workflow::runtime::init_\|TimerOps::set\|TimerWorkflow::set\|bootstrap::init_compiled_config' crates/canic-core/src/lifecycle crates/canic-core/src/workflow/runtime crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'` | PASS | Restore/init-before-schedule ordering scan recorded expected sequence. |
| `rg -n 'crate::ops::\|crate::domain::policy\|crate::storage::stable::' crates/canic-core/src/lifecycle crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'` | PASS | No direct storage/policy shortcuts in lifecycle modules. |
| `cargo fmt --all` | PASS | Formatting completed. |
| `cargo test -p canic --lib -- --nocapture` | PASS | Macro crate support tests passed. |
| `cargo test -p canic-tests --test lifecycle_boundary -- --nocapture` | PASS | 3 PocketIC lifecycle boundary tests passed. |
| `cargo test -p canic-tests --test root_wasm_store_reconcile root_post_upgrade_preserves_multi_store_current_release_binding -- --test-threads=1 --nocapture` | PASS | Root post-upgrade reconcile lifecycle path passed. |
| `cargo test -p canic-tests --test pic_role_attestation role_attestation_verification_paths -- --test-threads=1 --nocapture` | PASS | Current `start_root!(init = { ... })` fixture passed with timer-based init-block execution. |
| `cargo clippy -p canic --all-targets -- -D warnings` | PASS | Remediated macro crate passed clippy. |

## Follow-up Actions

1. Keep optional `init = { ... }` support behind lifecycle timers; do not
   reintroduce synchronous user code inside generated `init` or `post_upgrade`
   hook bodies.
2. Re-run this audit after changes to `start.rs`, lifecycle API adapters,
   control-plane root lifecycle scheduling, or role-attestation root fixture
   setup.
