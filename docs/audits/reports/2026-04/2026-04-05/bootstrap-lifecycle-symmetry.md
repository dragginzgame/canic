# Bootstrap Lifecycle Symmetry Audit - 2026-04-05

## Report Preamble

- Scope: `crates/canic/src/macros/start.rs`, `crates/canic-core/src/api/lifecycle/**`, `crates/canic-core/src/lifecycle/{init,upgrade}/**`, `crates/canic-core/src/workflow/runtime/mod.rs`, `crates/canic-control-plane/src/api/lifecycle.rs`
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-10/bootstrap-lifecycle-symmetry.md`
- Code snapshot identifier: `c027b8df`
- Method tag/version: `Method V4.1`
- Comparability status: `partially comparable` (the lifecycle contract is directly comparable, but the audited code moved from the old flat lifecycle files into `init/**`, `upgrade/**`, and control-plane root lifecycle adapters)
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-05T16:34:13Z`
- Branch: `main`
- Worktree: `dirty`

## Executive Summary

- Risk Score: `2 / 10`
- Delta summary: lifecycle invariants still hold. Macros remain thin, runtime/environment restoration still happens synchronously before bootstrap scheduling, and both non-root and root paths still schedule async bootstrap work through timers instead of awaiting it in lifecycle hooks.
- Largest structural drift since the March baseline: root lifecycle scheduling now visibly spans `canic-core` and `canic-control-plane`, and the old flat lifecycle file paths are obsolete.
- Follow-up required: `no` for correctness; `yes` for keeping future audit templates aligned with the current module layout.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Macro hooks stay thin and timer-driven | PASS | [start.rs](/home/adam/projects/canic/crates/canic/src/macros/start.rs) delegates init/post-upgrade to lifecycle APIs and uses `TimerApi::set_lifecycle_timer(Duration::ZERO, ...)` for user hooks. |
| Lifecycle API remains delegation-only | PASS | [nonroot.rs](/home/adam/projects/canic/crates/canic-core/src/api/lifecycle/nonroot.rs) and [root.rs](/home/adam/projects/canic/crates/canic-core/src/api/lifecycle/root.rs) are glue-only delegators; [lifecycle.rs](/home/adam/projects/canic/crates/canic-control-plane/src/api/lifecycle.rs) adds root control-plane registration plus timer scheduling only. |
| Init/post-upgrade execution model symmetry | PASS | [root.rs](/home/adam/projects/canic/crates/canic-core/src/lifecycle/init/root.rs), [root.rs](/home/adam/projects/canic/crates/canic-core/src/lifecycle/upgrade/root.rs), [nonroot.rs](/home/adam/projects/canic/crates/canic-core/src/lifecycle/init/nonroot.rs), and [nonroot.rs](/home/adam/projects/canic/crates/canic-core/src/lifecycle/upgrade/nonroot.rs) all follow `config -> runtime/env restore -> schedule bootstrap`. |
| No direct await in synchronous adapter flow | PASS | `rg -n '\.await|async fn|spawn\(' crates/canic-core/src/lifecycle -g '*.rs'` only matched awaits inside timer closures in non-root scheduling functions. |
| Restore-before-bootstrap ordering | PASS | upgrade paths restore memory/env before scheduling bootstrap, and init paths run `bootstrap::init_compiled_config(...)` plus `workflow::runtime::init_*` before any bootstrap timer is armed. |
| Lifecycle integration tests exist for current paths | PASS | `cargo test -p canic-tests --test lifecycle_boundary -- --nocapture` passed (`3 passed`), and `cargo test -p canic-tests --test root_wasm_store_reconcile root_post_upgrade_preserves_multi_store_current_release_binding -- --test-threads=1 --nocapture` passed. |

## Comparison to Previous Relevant Run

- Stable: macros are still thin lifecycle wiring with timer-driven async follow-through.
- Stable: synchronous adapters still trap on restore/init failures before continuation is scheduled.
- Stable: root and non-root post-upgrade paths still restore trusted state synchronously, then resume bootstrap asynchronously.
- Changed: the audited lifecycle boundary is now split across:
  - [start.rs](/home/adam/projects/canic/crates/canic/src/macros/start.rs)
  - [nonroot.rs](/home/adam/projects/canic/crates/canic-core/src/api/lifecycle/nonroot.rs)
  - [root.rs](/home/adam/projects/canic/crates/canic-core/src/api/lifecycle/root.rs)
  - [lifecycle.rs](/home/adam/projects/canic/crates/canic-control-plane/src/api/lifecycle.rs)
  - [root.rs](/home/adam/projects/canic/crates/canic-core/src/lifecycle/init/root.rs)
  - [root.rs](/home/adam/projects/canic/crates/canic-core/src/lifecycle/upgrade/root.rs)
  - [nonroot.rs](/home/adam/projects/canic/crates/canic-core/src/lifecycle/init/nonroot.rs)
  - [nonroot.rs](/home/adam/projects/canic/crates/canic-core/src/lifecycle/upgrade/nonroot.rs)

## Hard Violations Summary

| Hard Check | Result | Evidence |
| --- | --- | --- |
| Lifecycle hooks avoid direct orchestration | PASS | no `.await` in init/post-upgrade hook bodies in [start.rs](/home/adam/projects/canic/crates/canic/src/macros/start.rs) |
| Lifecycle API avoids policy/ops/model logic | PASS | API modules delegate only to lifecycle/control-plane adapters |
| Bootstrap remains timer-scheduled | PASS | non-root scheduling in [nonroot.rs](/home/adam/projects/canic/crates/canic-core/src/lifecycle/init/nonroot.rs) and [nonroot.rs](/home/adam/projects/canic/crates/canic-core/src/lifecycle/upgrade/nonroot.rs); root scheduling in [lifecycle.rs](/home/adam/projects/canic/crates/canic-control-plane/src/api/lifecycle.rs) |
| Lifecycle modules avoid direct storage/policy shortcuts | PASS | `rg -n 'crate::ops::|crate::domain::policy|crate::storage::stable::' crates/canic-core/src/lifecycle -g '*.rs'` returned no matches |

## Structural Hotspots

1. Root lifecycle now spans core and control-plane adapters.
   Evidence: [lifecycle.rs](/home/adam/projects/canic/crates/canic-control-plane/src/api/lifecycle.rs) owns embedded release-set registration and root bootstrap timer wiring, while [root.rs](/home/adam/projects/canic/crates/canic-core/src/lifecycle/init/root.rs) and [root.rs](/home/adam/projects/canic/crates/canic-core/src/lifecycle/upgrade/root.rs) own synchronous restore/init.

2. `workflow/runtime/mod.rs` remains the main lifecycle state-init hub.
   Evidence: [mod.rs](/home/adam/projects/canic/crates/canic-core/src/workflow/runtime/mod.rs) owns root/non-root runtime init, post-upgrade memory bootstrap, and timer family startup.

3. Non-root lifecycle duplication is still visible between attestation-aware and standard variants.
   Evidence: [nonroot.rs](/home/adam/projects/canic/crates/canic-core/src/lifecycle/init/nonroot.rs) and [nonroot.rs](/home/adam/projects/canic/crates/canic-core/src/lifecycle/upgrade/nonroot.rs) keep paired standard/attestation functions with only the runtime-start variant differing.

## Responsibility Drift Signals

- `PASS`: lifecycle adapters still trap on failed config/runtime restore before any async continuation is scheduled.
- `PASS`: user lifecycle hooks are still timer-scheduled from [start.rs](/home/adam/projects/canic/crates/canic/src/macros/start.rs), not awaited inline.
- `WARN`: the March audit template’s flat file list is now stale; future runs should keep using the split `init/**`, `upgrade/**`, and control-plane lifecycle paths to avoid false drift signals.

## Risk Score

Risk Score: **2 / 10**

Score contributions:
- `+1` root lifecycle ownership is split across `canic-core` and `canic-control-plane`
- `+1` duplicated non-root lifecycle variants (`with_attestation_cache` vs standard) remain a maintenance hotspot

Verdict: **Pass with low residual lifecycle drift risk.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n 'schedule_init_root_bootstrap|schedule_post_upgrade_root_bootstrap|schedule_init_nonroot_bootstrap|schedule_post_upgrade_nonroot_bootstrap|TimerApi::set_lifecycle_timer|Duration::ZERO' crates/canic-core crates/canic/src/macros -g '*.rs'` | PASS | timer scheduling remains explicit |
| `rg -n '\.await|async fn|spawn\(' crates/canic-core/src/lifecycle -g '*.rs'` | PASS | only timer closure awaits matched |
| `rg -n 'EnvOps::restore_|init_memory_registry_post_upgrade|workflow::runtime::init_|TimerOps::set|TimerWorkflow::set|bootstrap::init_compiled_config' crates/canic-core/src/lifecycle crates/canic-core/src/workflow/runtime -g '*.rs'` | PASS | restore/init precedes bootstrap scheduling |
| `rg -n 'crate::ops::|crate::domain::policy|crate::storage::stable::' crates/canic-core/src/lifecycle -g '*.rs'` | PASS | no direct storage/policy shortcuts |
| `cargo test -p canic-tests --test lifecycle_boundary -- --nocapture` | PASS | `3 passed; 0 failed` |
| `cargo test -p canic-tests --test root_wasm_store_reconcile root_post_upgrade_preserves_multi_store_current_release_binding -- --test-threads=1 --nocapture` | PASS | targeted root post-upgrade reconcile path still holds |

## Follow-up Actions

1. Keep the recurring audit template aligned with the current split lifecycle module layout so future runs do not keep pointing at removed flat files.
2. Re-run this audit immediately after any change to [start.rs](/home/adam/projects/canic/crates/canic/src/macros/start.rs), [lifecycle.rs](/home/adam/projects/canic/crates/canic-control-plane/src/api/lifecycle.rs), or [mod.rs](/home/adam/projects/canic/crates/canic-core/src/workflow/runtime/mod.rs).
