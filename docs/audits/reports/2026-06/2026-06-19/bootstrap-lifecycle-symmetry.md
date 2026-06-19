# Bootstrap Lifecycle Symmetry Audit - 2026-06-19

## Report Preamble

- Scope: `canic::start!`, `start_local!`, `start_wasm_store!`, lifecycle API
  wrappers, core init/post-upgrade adapters, root control-plane lifecycle
  scheduling, runtime continuation helpers, bootstrap timer boundaries, and
  lifecycle boundary tests.
- Compared baseline report path: `N/A`
- Code snapshot identifier: `16894709`
- Method tag/version: `bootstrap-lifecycle-symmetry/current`
- Comparability status: `partially comparable` - the contract is unchanged,
  but the live audit definition now explicitly scans `TimerApi::set_lifecycle_timer`
  in restore-order evidence and includes `crates/canic/tests` in lifecycle
  coverage discovery.

## Run Context

- Date: `2026-06-19`
- Auditor: `codex`
- Branch: `main`
- Worktree: dirty during audit; unrelated user/session changes were preserved.
- Previous retained report:
  `docs/audits/reports/2026-06/2026-06-01/bootstrap-lifecycle-symmetry.md`

## Audit Definition Review

The audit definition was reviewed before execution. Two small maintenance
updates were made:

- restore-before-bootstrap scans now include
  `TimerApi::set_lifecycle_timer`, which is the root control-plane scheduling
  API;
- lifecycle test discovery now includes `crates/canic/tests`;
- layering scans now catch grouped Rust imports such as
  `ops::runtime::env::EnvOps`, while documenting that runtime environment
  restoration through `EnvOps` is allowed.

## Executive Summary

Risk score: **2 / 10**.

The lifecycle symmetry invariant holds. Generated IC lifecycle hooks remain
thin adapters: they restore/configure synchronously, schedule bootstrap and
user hooks through lifecycle timers, and exit. Root scheduling remains split
between `canic-core` runtime restoration and `canic-control-plane` root
bootstrap timers. Non-root init and post-upgrade continue to schedule async
bootstrap work through `TimerWorkflow` / `TimerOps`.

No lifecycle contract break was found.

## Findings

### PASS - Macro Hooks Stay Thin

- `crates/canic/src/macros/start.rs:22-79` wires non-root `init` and
  `post_upgrade`, delegates pre-bootstrap restoration, then schedules bootstrap
  and user hooks through timers.
- `crates/canic/src/macros/start.rs:219-278` does the same for root lifecycle
  hooks through the control-plane lifecycle API.
- `crates/canic/src/macros/start.rs:282-298` runs optional user `init = { ... }`
  blocks from a lifecycle timer before continuation scheduling.
- `crates/canic/src/macros/start.rs:364-385` selects root/non-root dispatch by
  build metadata cfgs, not runtime branching.

### PASS - Startup Surface Dispatch Remains Metadata-Driven

- `crates/canic/src/macros/build.rs:86`, `186`, and `257-262` emit
  `canic_is_root` and `CANIC_CANISTER_ROLE` metadata.
- `crates/canic/src/macros/start.rs:403-415` keeps `start_local!` as a
  non-root local-dev mode and rejects root use at compile time.
- `crates/canic/src/macros/start.rs:429-447` keeps `start_wasm_store!` fixed to
  the canonical `WASM_STORE` role and endpoint bundle.

### PASS - Lifecycle API Boundary Is Delegation-Oriented

- `crates/canic-core/src/api/lifecycle/root.rs:10-29` delegates root
  pre-bootstrap lifecycle work to core lifecycle modules.
- `crates/canic-core/src/api/lifecycle/nonroot.rs:12-51` delegates non-root
  pre-bootstrap work and schedule calls to lifecycle modules.
- `crates/canic-control-plane/src/api/lifecycle.rs:18-38` registers root
  bootstrap module sources, delegates runtime restoration to `canic-core`, and
  logs embedded bootstrap state.
- `crates/canic-control-plane/src/api/lifecycle.rs:41-93` schedules root
  bootstrap through zero-delay lifecycle timers rather than running it inline.

### PASS - Init/Post-Upgrade Structure Is Symmetric

- `crates/canic-core/src/lifecycle/init/root.rs:24-50` initializes config and
  root runtime before completing the root init adapter.
- `crates/canic-core/src/lifecycle/upgrade/root.rs:23-72` initializes config,
  memory registry, root environment, and root runtime continuation before
  completion.
- `crates/canic-core/src/lifecycle/init/nonroot.rs:30-57` initializes config
  and non-root runtime before completion.
- `crates/canic-core/src/lifecycle/upgrade/nonroot.rs:29-80` initializes
  config, memory registry, non-root environment, and non-root runtime
  continuation before completion.

The init/post-upgrade difference is limited to documented post-upgrade
restoration work.

### PASS - Async Work Enters Through Timer Boundaries

- `crates/canic-core/src/lifecycle/init/nonroot.rs:67-97` schedules non-root
  init bootstrap through `TimerWorkflow::set`.
- `crates/canic-core/src/lifecycle/upgrade/nonroot.rs:90-120` schedules
  non-root post-upgrade bootstrap through `TimerOps::set`.
- `crates/canic-control-plane/src/api/lifecycle.rs:48-54` and `86-92` schedule
  root bootstrap through `TimerApi::set_lifecycle_timer`.

The `.await` scan found awaits only inside timer closures.

### PASS - Restore Before Bootstrap Ordering

- `crates/canic-core/src/lifecycle/upgrade/root.rs:36-66` traps on memory,
  environment, or runtime restoration failures before root bootstrap scheduling
  can be reached by the macro continuation.
- `crates/canic-core/src/lifecycle/upgrade/nonroot.rs:41-74` does the same for
  non-root post-upgrade.
- Init adapters trap on config/runtime init failure before schedule calls are
  reached.

### PASS - Layering Discipline

The updated grouped-import layering scan found expected runtime ops imports:

- `crates/canic-core/src/lifecycle/upgrade/root.rs:8` uses
  `ops::runtime::env::EnvOps`.
- `crates/canic-core/src/lifecycle/upgrade/nonroot.rs:11` uses
  `EnvOps` and `TimerOps`.
- `crates/canic-core/src/lifecycle/mod.rs:12` uses `ops::ic::IcOps` for trap
  timestamping.

No direct stable-storage mutation, stable schema import, or domain policy import
was found in lifecycle adapters.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk |
| --- | --- | --- | --- |
| `crates/canic/src/macros/start.rs` | lifecycle macro cores | IC hook generation and timer ordering | Medium |
| `crates/canic-core/src/lifecycle/init/*` | init adapters | restore/init sequencing before bootstrap scheduling | Medium |
| `crates/canic-core/src/lifecycle/upgrade/*` | post-upgrade adapters | memory/env restore and trap-before-schedule ordering | Medium |
| `crates/canic-control-plane/src/api/lifecycle.rs` | root schedule functions | root bootstrap timer boundary | Medium |
| `crates/canic-core/src/workflow/runtime/*` | runtime continuation | role-specific runtime restoration after lifecycle hooks | Low |

## Hub Module Pressure

Pressure score: **2 / 10**.

Lifecycle macro and adapter fan-in is expected and bounded. Recent edit history
shows `start.rs` touched in `0.68.24`; most other nearby churn is in runtime
auth/provisioning modules rather than lifecycle adapter sequencing.

## Early Warning Signals

- Keep root control-plane lifecycle as registration plus timer scheduling only.
- Keep optional `start!(init = { ... })` blocks timer-based.
- Keep `start_local!` and `start_wasm_store!` as explicit special runtime modes.
- Keep grouped-import scans in the audit definition so expected runtime ops
  imports remain visible.

## Dependency Fan-In Pressure

No lifecycle-specific fan-in pressure materially changed this run. Runtime auth
and provisioning modules are active 0.68 hotspots, but they do not alter
lifecycle hook symmetry.

## Risk Score

Risk Score: **2 / 10**.

Derivation:

- `+1` for concentrated lifecycle hook generation in `start.rs`.
- `+1` for root scheduling split across `canic-core` and
  `canic-control-plane`.
- `0` confirmed contract breaks.

## Verification Readout

| Check | Result | Notes |
| --- | --- | --- |
| Lifecycle wiring and timer scheduling scan | PASS | Found expected `LifecycleApi` and `TimerApi::set_lifecycle_timer` calls. |
| Startup surface dispatch scan | PASS | Found metadata cfg dispatch, `start_local!`, and `start_wasm_store!`. |
| Lifecycle API delegation scan | PASS | API wrappers remain delegation/scheduling oriented. |
| Init/post-upgrade structure scan | PASS | Found expected root/non-root init and post-upgrade adapters plus timer scheduling. |
| Async behavior scan | PASS | Await points are inside timer closures. |
| Restore-before-bootstrap ordering scan | PASS | Found restore/init before timer scheduling surfaces. |
| Layering scan | PASS | Only expected runtime ops imports; no stable storage or domain policy imports. |
| Lifecycle test coverage scan | PASS | Found lifecycle boundary, trap guard, sharding bootstrap, and root post-upgrade tests. |
| Startup fixture scan | PASS | Found `start!`, `start_local!`, `start_wasm_store!`, and root optional init fixture usage. |
| `start_root!` active-surface scan | PASS | Active code has no public `start_root!`; matches are historical docs/changelog or helper names. |
| `cargo check --locked -p canic-core -p canic -p canic-control-plane` | PASS | Lifecycle-owning crates checked. |
| `cargo test --locked -p canic-core --test trap_guard -- --nocapture` | PASS | 1 test. |
| `cargo test --locked -p canic-tests --test lifecycle_boundary -- --nocapture` | PASS | 3 PocketIC tests. |
| `cargo test --locked -p canic --test changelog_governance -- --nocapture` | PASS | 1 test. |
| `cargo test --locked -p canic --test workspace_manifest -- --nocapture` | PASS | 5 tests. |

`cargo test --locked -p canic --test protocol_surface lifecycle -- --nocapture`
was also run, but the filter matched zero tests, so it is not counted as
lifecycle evidence.

## Follow-up Actions

No required remediation.

## Final Verdict

PASS.

Lifecycle hooks remain thin synchronous adapters that restore runtime state and
schedule bootstrap/user work through lifecycle timers.
