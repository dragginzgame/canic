# Bootstrap Lifecycle Symmetry Audit - 2026-06-22

## Report Preamble

- Scope: `canic::start!`, `start_local!`, `start_wasm_store!`, lifecycle API
  wrappers, core init/post-upgrade adapters, root control-plane lifecycle
  scheduling, lifecycle metrics, embedded root wasm-store bootstrap source
  registration, runtime continuation helpers, bootstrap timer boundaries, and
  lifecycle boundary tests.
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-19/bootstrap-lifecycle-symmetry.md`
- Code snapshot identifier: `5bc5a458`
- Method tag/version: `bootstrap-lifecycle-symmetry/current`
- Comparability status: `partially comparable` - the lifecycle contract is
  unchanged, but the live audit definition now explicitly covers synchronous
  lifecycle metrics, embedded root wasm-store bootstrap release-set
  registration/logging, and post-upgrade memory registry restore ordering.

## Run Context

- Date: `2026-06-22`
- Auditor: `codex`
- Branch: `main`
- Worktree: dirty during audit; unrelated user/session changes were preserved.
- Audited paths:
  `crates/canic/src/macros/start.rs`,
  `crates/canic/src/macros/timer.rs`,
  `crates/canic-core/src/api/lifecycle/**`,
  `crates/canic-core/src/lifecycle/**`,
  `crates/canic-core/src/workflow/runtime/**`,
  `crates/canic-core/src/workflow/bootstrap/**`,
  `crates/canic-control-plane/src/api/lifecycle.rs`,
  `crates/canic-control-plane/src/api/template/mod.rs`,
  `crates/canic-control-plane/src/workflow/bootstrap/root.rs`,
  lifecycle-related tests.

## Audit Definition Review

The audit definition was reviewed before execution and refreshed to match the
current lifecycle surface:

- lifecycle adapters may record bounded lifecycle metrics synchronously;
- root lifecycle may register/log embedded wasm-store bootstrap module sources
  before scheduling bootstrap;
- post-upgrade memory registry restoration is an explicit restore-order check;
- storage-backed template admin helpers in the same `api/template` module are
  outside lifecycle scope unless lifecycle starts calling them.

No production code changes were made.

## Executive Summary

Risk score: **3 / 10**.

The lifecycle symmetry invariant holds. Generated IC hooks still synchronously
delegate restore/init work, schedule bootstrap and user hooks through zero-delay
timers, and exit. Root lifecycle has more adapter pressure than the previous
run because it now explicitly carries lifecycle metrics plus embedded
wasm-store module source registration/logging, but those duties remain bounded:
bootstrap execution still occurs only inside timer closures.

## Findings

### PASS - Macro Hooks Stay Thin

- `crates/canic/src/macros/start.rs:22-49` non-root `init` delegates
  pre-bootstrap work, then schedules bootstrap and user hooks through timers.
- `crates/canic/src/macros/start.rs:52-79` non-root `post_upgrade` follows the
  same shape for upgrade.
- `crates/canic/src/macros/start.rs:219-278` root `init` and `post_upgrade`
  delegate through the control-plane lifecycle API, then schedule root
  bootstrap and user hooks through timers.
- `crates/canic/src/macros/start.rs:282-298` optional `init = { ... }` blocks
  run from a lifecycle timer before continuation scheduling.
- `crates/canic/src/macros/start.rs:364-385` root/non-root dispatch remains
  compile-time metadata cfg dispatch.
- `crates/canic/src/macros/start.rs:403-447` keeps `start_local!` and
  `start_wasm_store!` as explicit special runtime modes.

### PASS - Lifecycle API Boundary Is Delegation-Oriented

- `crates/canic-core/src/api/lifecycle/root.rs:10-29` delegates root lifecycle
  work to core lifecycle modules.
- `crates/canic-core/src/api/lifecycle/nonroot.rs:12-51` delegates non-root
  pre-bootstrap and schedule calls to lifecycle modules.
- `crates/canic-control-plane/src/api/lifecycle.rs:18-38` registers embedded
  wasm-store module sources, registers the template resolver, delegates runtime
  restoration to `canic-core`, then logs embedded source provenance.
- `crates/canic-control-plane/src/api/lifecycle.rs:41-93` records lifecycle
  bootstrap metrics and schedules root bootstrap through
  `TimerApi::set_lifecycle_timer`; root bootstrap calls are inside timer
  closures.

### PASS - Embedded Root Bootstrap Registration Is Bounded

- `crates/canic-control-plane/src/api/template/mod.rs:31-46` registers only the
  embedded wasm-store module bytes for the module source runtime.
- `crates/canic-control-plane/src/api/template/mod.rs:49-68` logs embedded
  bootstrap artifact provenance.
- `crates/canic-control-plane/src/api/template/mod.rs:73-413` also contains
  storage-backed template admin helpers, but lifecycle does not call those
  functions.

### PASS - Init/Post-Upgrade Structure Is Symmetric

- `crates/canic-core/src/lifecycle/init/root.rs:12-51` records runtime metrics,
  initializes compiled config, initializes root runtime, and traps before
  completion on failure.
- `crates/canic-core/src/lifecycle/init/nonroot.rs:16-57` follows the same
  runtime init shape for non-root canisters.
- `crates/canic-core/src/lifecycle/upgrade/root.rs:12-72` initializes config,
  restores the memory registry, restores root environment, runs root runtime
  continuation, and traps before bootstrap scheduling on failure.
- `crates/canic-core/src/lifecycle/upgrade/nonroot.rs:16-80` follows the same
  post-upgrade phase order for non-root canisters.

### PASS - Async Work Enters Through Timer Boundaries

- `crates/canic-core/src/lifecycle/init/nonroot.rs:60-97` schedules non-root
  init bootstrap with `TimerWorkflow::set`.
- `crates/canic-core/src/lifecycle/upgrade/nonroot.rs:83-120` schedules
  non-root post-upgrade bootstrap with `TimerOps::set`.
- `crates/canic-control-plane/src/api/lifecycle.rs:48-54` and `86-92` schedule
  root bootstrap through lifecycle timers.

The await scan found awaits only inside timer closures.

### PASS - Restore Before Bootstrap Ordering

- `crates/canic-core/src/lifecycle/upgrade/root.rs:36-66` restores the memory
  registry and root environment before root runtime continuation.
- `crates/canic-core/src/lifecycle/upgrade/nonroot.rs:41-74` restores the
  memory registry and role environment before non-root runtime continuation.
- Init adapters initialize config and runtime state before any schedule calls
  can be reached by macro continuation.

### PASS - Layering Discipline

Expected lifecycle imports were found:

- `crates/canic-core/src/lifecycle/upgrade/root.rs:8` uses
  `ops::runtime::env::EnvOps`.
- `crates/canic-core/src/lifecycle/upgrade/nonroot.rs:11` uses `EnvOps` and
  `TimerOps`.
- `crates/canic-core/src/lifecycle/mod.rs:12` uses `ops::ic::IcOps` for trap
  timestamping.

No lifecycle adapter imports stable storage schemas or domain policy. The
`workflow::bootstrap` hits are inside scheduled timer closures.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk |
| --- | --- | --- | --- |
| `crates/canic/src/macros/start.rs` | lifecycle macro cores | IC hook generation and timer ordering | Medium |
| `crates/canic-core/src/lifecycle/init/*` | init adapters | restore/init sequencing before bootstrap scheduling | Medium |
| `crates/canic-core/src/lifecycle/upgrade/*` | post-upgrade adapters | memory/env restore and trap-before-schedule ordering | Medium |
| `crates/canic-control-plane/src/api/lifecycle.rs` | root lifecycle adapter | metrics, embedded source registration, and root bootstrap timer boundary | Medium |
| `crates/canic-control-plane/src/api/template/mod.rs` | embedded root wasm-store registration helpers | lifecycle-called helpers share a file with storage-backed template admin helpers | Medium |
| `crates/canic-core/src/workflow/runtime/*` | runtime continuation | role-specific runtime restoration after lifecycle hooks | Low |

## Hub Module Pressure

Pressure score: **3 / 10**.

Lifecycle fan-in is still bounded, but root lifecycle now spans core lifecycle
restoration, control-plane source registration, lifecycle metrics, and root
timer scheduling. The pressure is acceptable because orchestration remains in
workflow/timer closures and the template storage helpers are not lifecycle
calls.

## Early Warning Signals

- Keep root control-plane lifecycle limited to source registration, metrics,
  core runtime restoration delegation, provenance logging, and timer
  scheduling.
- Keep embedded module source registration separate from template staging,
  chunk publication, and storage-backed admin helpers.
- Keep lifecycle metrics inline but bounded; metrics must not become lifecycle
  state orchestration.
- Keep optional `start!(init = { ... })` blocks timer-based.
- Active code still has no public `start_root!`; matches remain historical
  docs/changelog/audit history.

## Dependency Fan-In Pressure

No lifecycle-specific fan-in pressure exceeds the audit threshold. The main
watchpoint is `api/template/mod.rs`, where lifecycle-called embedded source
helpers live beside broader template administration APIs.

## Risk Score

Risk Score: **3 / 10**.

Derivation:

- `+1` for concentrated lifecycle hook generation in `start.rs`.
- `+1` for root scheduling split across `canic-core` and
  `canic-control-plane`.
- `+1` for bounded root lifecycle adapter pressure from metrics plus embedded
  wasm-store source registration/logging.
- `0` confirmed contract breaks.

## Verification Readout

| Check | Result | Notes |
| --- | --- | --- |
| Lifecycle wiring and timer scheduling scan | PASS | Found expected `LifecycleApi` and `TimerApi::set_lifecycle_timer` calls. |
| Startup surface dispatch scan | PASS | Found metadata cfg dispatch, `start_local!`, and `start_wasm_store!`. |
| Lifecycle API delegation scan | PASS | API wrappers remain delegation/scheduling oriented. |
| Init/post-upgrade structure scan | PASS | Found expected root/non-root init and post-upgrade adapters plus timer scheduling. |
| Async behavior scan | PASS | Await points are inside timer closures. |
| Restore-before-bootstrap ordering scan | PASS | Memory registry and env restore precede post-upgrade continuation. |
| Lifecycle metrics/source registration scan | PASS | Metrics and embedded source registration are bounded adapter duties. |
| Layering scan | PASS | Only expected runtime ops imports; no lifecycle stable storage or domain policy imports. |
| Lifecycle test coverage scan | PASS | Found lifecycle boundary, trap guard, root post-upgrade reconcile, and bootstrap tests. |
| Startup fixture scan | PASS | Found `start!`, `start_local!`, `start_wasm_store!`, and optional root init fixture usage. |
| `start_root!` active-surface scan | PASS | Active code has no public `start_root!`; matches are historical docs/changelog/audit notes. |
| `cargo check --locked -p canic-core -p canic -p canic-control-plane` | PASS | Lifecycle-owning crates checked. |
| `cargo test --locked -p canic-core --test trap_guard -- --nocapture` | PASS | 1 test. |
| `cargo test --locked -p canic --test protocol_surface finish -- --nocapture` | PASS | 1 filtered test. |
| `cargo test --locked -p canic --test workspace_manifest -- --nocapture` | PASS | 6 tests. |
| `cargo test --locked -p canic --test metrics_facade -- --nocapture` | PASS | 3 tests. |
| `POCKET_IC_BIN=... cargo test --locked -p canic-tests --test lifecycle_boundary -- --nocapture` | PASS | Sandboxed run failed on local bind; unsandboxed retry passed 3 tests. |
| `POCKET_IC_BIN=... cargo test --locked -p canic-tests --test root_wasm_store_reconcile root_post_upgrade_preserves_multi_store_current_release_binding -- --nocapture` | PASS | Unsandboxed PocketIC run passed 1 focused root post-upgrade test. |

## Follow-up Actions

- Keep `api/lifecycle.rs` from calling storage-backed template admin helpers.
- Keep lifecycle metrics bounded to runtime/bootstrap phase recording.
- If embedded root wasm-store bootstrap registration expands, split the
  lifecycle-called helper into a smaller module before it starts sharing more
  of `api/template`'s admin surface.

## Final Verdict

PASS with watchpoints.

Lifecycle hooks remain thin synchronous adapters that restore runtime state and
schedule bootstrap/user work through lifecycle timers.
