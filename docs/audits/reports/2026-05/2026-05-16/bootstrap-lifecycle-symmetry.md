# Bootstrap Lifecycle Symmetry Audit - 2026-05-16

## Report Preamble

- Definition path: `docs/audits/recurring/system/bootstrap-lifecycle-symmetry.md`
- Scope: `crates/canic/src/macros/start.rs`, `crates/canic-core/src/api/lifecycle/**`, `crates/canic-core/src/lifecycle/**`, `crates/canic-control-plane/src/api/lifecycle.rs`, `crates/canic-core/src/workflow/runtime/{mod,root,nonroot}.rs`, `crates/canic-core/src/workflow/bootstrap/**`, lifecycle tests, and root `start_root!(init = { ... })` fixtures
- Compared baseline report path: `docs/audits/reports/2026-05/2026-05-09/bootstrap-lifecycle-symmetry.md`
- Code snapshot identifier: `f5b88fe7`
- Method tag/version: `bootstrap-lifecycle-symmetry/current`
- Comparability status: `partially comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-16T13:12:25Z`
- Branch: `main`
- Worktree: started clean; ended dirty because this audit applied remediation and added this report

## Audit Selection

This was selected from the oldest latest-run recurring audits. Several audits
were tied at `2026-05-09`; `bootstrap-lifecycle-symmetry` was first in the
sorted recurring audit set and had just been refreshed to current module paths.

The run is partially comparable with the May 9 baseline because the recurring
definition now explicitly scopes the root control-plane lifecycle split and
excludes host-side backup/restore flows from lifecycle "restore" terminology.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Macro hooks stay thin | PASS | `crates/canic/src/macros/start.rs:22-65`, `111-156`, and `191-236` generate IC hooks that delegate to lifecycle APIs, then call the optional-init helper and timer bundles. |
| User hooks run through lifecycle timers | PASS | `crates/canic/src/macros/start.rs:240-256` runs optional `init = { ... }` blocks inside `TimerApi::set_lifecycle_timer(Duration::ZERO, ...)`; `start.rs:262-320` schedules user lifecycle hooks through the same timer boundary. |
| Lifecycle API remains delegation-only | PASS | `crates/canic-core/src/api/lifecycle/nonroot.rs:11-52` and `root.rs:9-35` only delegate to lifecycle modules; `crates/canic-control-plane/src/api/lifecycle.rs:18-93` performs root template registration plus lifecycle delegation and timer scheduling. |
| Init/post-upgrade execution model symmetry | FIXED | `crates/canic-core/src/workflow/runtime/nonroot.rs:87-115` previously panicked on post-upgrade runtime config/auth failures while the root path returned `Result`; it now returns `Result<(), InternalError>` and the adapter handles failure through the lifecycle path. |
| No direct await in lifecycle adapters | PASS | The async scan found `.await` only inside lifecycle timer closures at `crates/canic-core/src/lifecycle/init/nonroot.rs:70-96` and `upgrade/nonroot.rs:87-114`. |
| Restore-before-bootstrap ordering | PASS | `crates/canic-core/src/lifecycle/init/nonroot.rs:30-57` and `init/root.rs:24-50` complete config/runtime init before scheduling; `upgrade/nonroot.rs:29-75` and `upgrade/root.rs:23-75` complete config, memory, env restore, and runtime continuation before scheduling. |
| Root control-plane split | PASS | `crates/canic-control-plane/src/api/lifecycle.rs:25-37` delegates root runtime restore to `canic-core` before `schedule_init_root_bootstrap`; `lifecycle.rs:64-75` does the same before `schedule_post_upgrade_root_bootstrap`. |
| Lifecycle modules avoid storage/policy shortcuts | PASS | `rg -n 'crate::ops::\|crate::domain::policy\|crate::storage::stable::' crates/canic-core/src/lifecycle crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'` returned no matches. |
| Runtime coverage exercises lifecycle boundary | PASS | `crates/canic-tests/tests/lifecycle_boundary.rs:14-159` covers init trap phases, repeated non-root post-upgrade readiness, and non-root post-upgrade failure reporting; `crates/canic-core/tests/trap_guard.rs:9-22` keeps direct trap usage in lifecycle code. |

## Remediation Applied

| Change | Files | Result |
| --- | --- | --- |
| Returned typed errors from non-root post-upgrade runtime continuation instead of panicking on config/auth failures | `crates/canic-core/src/workflow/runtime/nonroot.rs` | Non-root post-upgrade now matches the root continuation shape and lets the lifecycle adapter own phase-aware trapping. |
| Recorded failed lifecycle runtime metrics before trapping non-root post-upgrade continuation errors | `crates/canic-core/src/lifecycle/upgrade/nonroot.rs` | Failed post-upgrade continuation now uses the same lifecycle failure path as config, memory, and env restore failures. |

## Comparison to Previous Relevant Run

- Stable: lifecycle macros still restore or delegate synchronously, then
  schedule bootstrap/user work through lifecycle timers.
- Stable: root bootstrap orchestration remains split between `canic-core`
  restoration and `canic-control-plane` scheduling.
- Improved: non-root post-upgrade runtime continuation no longer panics inside
  workflow runtime when config/auth continuation checks fail.
- Stable: no direct policy or storage mutation imports were found in lifecycle
  modules.
- Stable: lifecycle boundary tests still exercise init and post-upgrade failure
  behavior.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic/src/macros/start.rs` | lifecycle core macros and optional-init helper | IC hook generation and user-hook/bootstrap timer ordering | High |
| `crates/canic-core/src/lifecycle/init/*` | root and non-root init adapters | startup ordering and scheduling boundary | High |
| `crates/canic-core/src/lifecycle/upgrade/*` | root and non-root post-upgrade adapters | restore-before-bootstrap sequencing and trap path consistency | High |
| `crates/canic-control-plane/src/api/lifecycle.rs` | root schedule functions | root bootstrap orchestration timer boundary | High |
| `crates/canic-core/src/workflow/runtime/{root,nonroot}.rs` | post-restore runtime continuation | role-specific runtime continuation after lifecycle restoration | Medium |

## Hub Module Pressure

| Module | Fan-In Evidence | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| lifecycle macro core | start macro scan found generated non-root, local, and root lifecycle hook paths plus lifecycle timer bundles | 3 | 2 | 6 |
| core lifecycle adapters | init/upgrade scan found root and non-root runtime restore plus bootstrap scheduling functions | 3 | 2 | 5 |
| root control-plane lifecycle | root lifecycle spans template registration, core restore delegation, metrics, and timer scheduling | 4 | 3 | 6 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| root lifecycle split | `canic-core` and `canic-control-plane` | Root restore and root bootstrap scheduling intentionally live in different crates | Medium |
| optional init-block ordering | `crates/canic/src/macros/start.rs:240-256` | Optional init block must complete before continuation timers are scheduled | Medium |
| post-upgrade continuation drift | `crates/canic-core/src/workflow/runtime/nonroot.rs:87-115` | Found and fixed one panic-based non-root continuation path that differed from root typed-error handling | Fixed |

## Dependency Fan-In Pressure

No fan-in pressure materially affected lifecycle drift risk in this run. The
recurring hotspot remains intentional fan-in around lifecycle macros and runtime
continuation helpers.

## Risk Score

Initial Risk Score: **3 / 10**

Post-remediation Risk Score: **2 / 10**

Initial score contributions:

- `+1` non-root post-upgrade continuation used panic-based config/auth failure
  handling instead of returning typed errors through the lifecycle adapter.
- `+1` root lifecycle remains intentionally split across `canic-core` and
  `canic-control-plane`.
- `+1` optional init-block ordering remains a specialized lifecycle timer path.

Remediation removed the post-upgrade continuation drift contribution. Residual
risk is limited to intentional structural hotspots and timer-ordering pressure.

Verdict: **Invariant holds after remediation with low residual lifecycle
ordering risk.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n 'init\(\|post_upgrade\(\|LifecycleApi::\|TimerApi::set_lifecycle_timer\|Duration::ZERO' crates/canic/src/macros/start.rs` | PASS | Macro hook and timer wiring scan recorded expected lifecycle delegation and timer boundaries. |
| `rg -n 'pub fn init_\|pub fn post_upgrade_\|lifecycle::' crates/canic-core/src/api/lifecycle crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'` | PASS | API delegation scan recorded core lifecycle wrappers and root control-plane delegation. |
| `rg -n 'init_root_canister\|init_nonroot_canister\|post_upgrade_root_canister\|post_upgrade_nonroot_canister\|TimerWorkflow::set\|TimerOps::set\|Duration::ZERO' crates/canic-core/src/lifecycle crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'` | PASS | Init/upgrade and timer scheduling scan recorded expected lifecycle boundaries. |
| `rg -n '\.await\|async fn\|spawn\(' crates/canic-core/src/lifecycle -g '*.rs'` | PASS | Only timer-closure awaits were found. |
| `rg -n 'EnvOps::restore_\|init_memory_registry_post_upgrade\|workflow::runtime::init_\|TimerOps::set\|TimerWorkflow::set' crates/canic-core/src/lifecycle crates/canic-core/src/workflow/runtime crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'` | PASS | Restore/init-before-schedule ordering scan recorded expected sequence. |
| `rg -n 'crate::ops::\|crate::domain::policy\|crate::storage::stable::' crates/canic-core/src/lifecycle crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'` | PASS | No direct storage/policy shortcuts in lifecycle modules. |
| `rg -n 'lifecycle\|post_upgrade\|init\|bootstrap\|Timer' crates/canic-tests/tests crates/canic-core/tests -g '*.rs'` | PASS | Lifecycle boundary and trap-guard coverage found. |
| `rg -n 'start_root!\(\|init = \{' canisters crates/canic-tests -g '*.rs'` | PASS | Root optional init fixture remains visible at `canisters/test/delegation_root_stub/src/lib.rs:32-35`. |
| `cargo fmt --all --check` | PASS | Formatting check passed. |
| `cargo check -p canic-core -p canic -p canic-control-plane` | PASS | Touched lifecycle crates checked. |
| `cargo test -p canic-core --test trap_guard -- --nocapture` | PASS | 1 trap-guard test passed. |
| `cargo test -p canic-tests --test lifecycle_boundary -- --nocapture` | PASS | 3 PocketIC lifecycle boundary tests passed. |
| `cargo clippy -p canic-core --all-targets -- -D warnings` | PASS | Touched core crate passed clippy. |

## Follow-up Actions

No follow-up actions required.
