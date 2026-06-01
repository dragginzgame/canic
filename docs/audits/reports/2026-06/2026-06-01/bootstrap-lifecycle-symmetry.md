# Bootstrap Lifecycle Symmetry Audit - 2026-06-01

## Report Preamble

- Definition path:
  `docs/audits/recurring/system/bootstrap-lifecycle-symmetry.md`
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-16/bootstrap-lifecycle-symmetry.md`
- Code snapshot identifier: `5487d6ff`
- Method tag/version: `bootstrap-lifecycle-symmetry/current`
- Comparability status: `partially comparable`
- Auditor: `codex`
- Run timestamp: `2026-06-01`
- Branch: `main`
- Worktree: started clean; ended dirty because this audit refreshed the
  recurring definition and added this report
- Scope:
  - `crates/canic/src/macros/start.rs`
  - `crates/canic/src/macros/timer.rs`
  - `crates/canic-core/src/api/lifecycle/**`
  - `crates/canic-core/src/lifecycle/**`
  - `crates/canic-control-plane/src/api/lifecycle.rs`
  - `crates/canic-core/src/workflow/runtime/**`
  - `crates/canic-core/src/workflow/bootstrap/**`
  - lifecycle tests and startup macro fixtures

## Executive Summary

Initial risk: **2 / 10**.

Post-audit risk: **2 / 10**.

The lifecycle symmetry invariant holds. Ordinary Canic-managed canisters still
use `canic::start!()`, with root vs non-root behavior selected by
`canic::build!`-emitted package metadata cfgs. `start_local!` and
`start_wasm_store!` remain separate special-purpose runtime modes, but they
still enter the same synchronous restore and lifecycle timer boundary.

No remediation was needed. The only definition update was to make the recurring
audit explicitly track the current startup surface instead of relying on the
old root-macro-era framing.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Macro hooks stay thin | PASS | `crates/canic/src/macros/start.rs:23-75`, `126-181`, and `220-275` wire IC hooks, delegate lifecycle restore, then schedule bootstrap/user timers. |
| `start!()` dispatch is metadata-driven | PASS | `crates/canic/src/macros/build.rs:85`, `177`, and `240-246` emit `canic_is_root`, `CANIC_CANISTER_ROLE`, and role state; `start.rs:364-385` dispatches root/non-root from those cfgs. |
| Special startup modes remain special | PASS | `start_local!` rejects root roles and synthesizes local non-root init payloads at `start.rs:403-418`; `start_wasm_store!` fixes role to `WASM_STORE` at `start.rs:429-447`. |
| User hooks run through lifecycle timers | PASS | Optional `init = { ... }` blocks and user setup/install/upgrade hooks are scheduled through `TimerApi::set_lifecycle_timer` at `start.rs:287-300` and hook continuation sites. |
| Lifecycle API remains delegation-only | PASS | `crates/canic-core/src/api/lifecycle/{root,nonroot}.rs` delegate to lifecycle modules; root control-plane API registers bootstrap module sources before delegating core restore and later schedules root bootstrap timers. |
| Init/post-upgrade execution model symmetry | PASS | Root and non-root init paths initialize compiled config and runtime before scheduling; post-upgrade paths initialize config, memory registry, environment restoration, runtime continuation, and only then schedule bootstrap. |
| No direct await in lifecycle adapters | PASS | `.await` appears only inside timer closures in non-root lifecycle scheduling, not in synchronous restore adapters. |
| Restore-before-bootstrap ordering | PASS | `init/root.rs`, `init/nonroot.rs`, `upgrade/root.rs`, and `upgrade/nonroot.rs` trap on restore/init failures before any scheduling call is reached. |
| Lifecycle modules avoid storage/policy shortcuts | PASS | Direct storage stable-type and policy scans returned no matches in lifecycle modules. |
| Runtime coverage exercises lifecycle boundary | PASS | `crates/canic-tests/tests/lifecycle_boundary.rs` covers phase-specific init/post-upgrade traps and repeated non-root post-upgrade readiness; `crates/canic-core/tests/trap_guard.rs` guards trap usage. |

## Definition Refresh

The recurring audit definition now explicitly covers:

- `canic::start!()` as the only ordinary startup macro;
- compile-time root/non-root dispatch from `canic::build!` metadata;
- `start_local!` and `start_wasm_store!` as separate special-purpose runtime
  modes;
- `crates/canic/src/macros/timer.rs` as supporting timer macro scope;
- current fixture searches for `start!`, `start_local!`, and
  `start_wasm_store!`.

This makes the audit partially comparable with the May 16 baseline because the
contract is the same but the startup-surface wording is now post-0.48.

## Comparison to Previous Relevant Run

- Stable: lifecycle macros still restore or delegate synchronously, then
  schedule bootstrap/user work through lifecycle timers.
- Stable: non-root post-upgrade continuation still returns typed errors
  through the lifecycle trap path instead of panicking inside workflow runtime.
- Stable: root bootstrap orchestration remains intentionally split between
  `canic-core` restoration and `canic-control-plane` scheduling.
- Updated: root fixture coverage now uses `canic::start!(init = { ... })`;
  `start_root!` remains absent from active code.
- Updated: special `start_local!` and `start_wasm_store!` modes are now
  explicitly audited as lifecycle-boundary participants.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic/src/macros/start.rs` | lifecycle core macros and optional-init helper | IC hook generation and user-hook/bootstrap timer ordering | High |
| `crates/canic-core/src/lifecycle/init/*` | root and non-root init adapters | startup ordering and scheduling boundary | High |
| `crates/canic-core/src/lifecycle/upgrade/*` | root and non-root post-upgrade adapters | restore-before-bootstrap sequencing and trap path consistency | High |
| `crates/canic-control-plane/src/api/lifecycle.rs` | root schedule functions | root bootstrap orchestration timer boundary | High |
| `crates/canic-core/src/workflow/runtime/{root,nonroot}.rs` | post-restore runtime continuation | role-specific runtime continuation after lifecycle restoration | Medium |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| Root lifecycle split | `canic-core` and `canic-control-plane` | Root restore and root bootstrap scheduling intentionally live in different crates. | Medium |
| Optional init-block ordering | `crates/canic/src/macros/start.rs:287-300` | Optional init block runs in a lifecycle timer before continuation timers are scheduled. | Medium |
| Special startup modes | `start_local!`, `start_wasm_store!` | These remain deliberately separate runtime modes and must not become alternate ordinary managed startup surfaces. | Low |

## Dependency Fan-In Pressure

No fan-in pressure materially affected lifecycle drift risk in this run. The
recurring hotspot remains intentional fan-in around lifecycle macros and runtime
continuation helpers.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n 'init\(\|post_upgrade\(\|LifecycleApi::\|TimerApi::set_lifecycle_timer\|Duration::ZERO' crates/canic/src/macros/start.rs crates/canic/src/macros/timer.rs` | PASS | Macro hook and timer wiring scan recorded expected lifecycle delegation and timer boundaries. |
| `rg -n 'macro_rules! start\|macro_rules! start_local\|macro_rules! start_wasm_store\|canic_is_root\|CANIC_CANISTER_ROLE\|WASM_STORE\|compile_error!' crates/canic/src/macros/start.rs crates/canic/src/macros/build.rs` | PASS | Startup dispatch and special-mode scan recorded metadata cfg dispatch, root rejection in `start_local!`, and fixed `WASM_STORE` role. |
| `rg -n 'pub fn init_\|pub fn post_upgrade_\|lifecycle::' crates/canic-core/src/api/lifecycle crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'` | PASS | API delegation scan recorded thin core lifecycle wrappers and root control-plane delegation. |
| `rg -n 'init_root_canister\|init_nonroot_canister\|post_upgrade_root_canister\|post_upgrade_nonroot_canister\|TimerWorkflow::set\|TimerOps::set\|Duration::ZERO' crates/canic-core/src/lifecycle crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'` | PASS | Init/upgrade and timer scheduling scan recorded expected lifecycle boundaries. |
| `rg -n '\.await\|async fn\|spawn\(' crates/canic-core/src/lifecycle -g '*.rs'` | PASS | Only timer-closure awaits were found. |
| `rg -n 'EnvOps::restore_\|init_memory_registry_post_upgrade\|workflow::runtime::init_\|TimerOps::set\|TimerWorkflow::set\|TimerApi::set_lifecycle_timer' crates/canic-core/src/lifecycle crates/canic-core/src/workflow/runtime crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'` | PASS | Restore/init-before-schedule ordering scan recorded expected sequence. |
| `rg -n 'crate::ops::\|crate::domain::policy\|crate::storage::stable::' crates/canic-core/src/lifecycle crates/canic-control-plane/src/api/lifecycle.rs -g '*.rs'` | PASS | No direct storage/policy shortcuts in lifecycle modules. |
| `rg -n 'lifecycle\|post_upgrade\|init\|bootstrap\|Timer' crates/canic-tests/tests crates/canic-core/tests -g '*.rs'` | PASS | Lifecycle boundary and trap-guard coverage found. |
| `rg -n 'start!\(\|start_local!\(\|start_wasm_store!\(\|init = \{' canisters crates/canic-tests crates/canic-wasm-store -g '*.rs'` | PASS | Current fixture/startup macro uses found. |
| `rg -n 'start_root!\|start_root' crates canisters docs -g '*.rs' -g '*.md' -g '*.toml'` | PASS | Active code has no `start_root!`; matches are historical docs/audits/changelogs or helper names unrelated to the public macro. |
| `cargo fmt --all --check` | PASS | Formatting check passed. |
| `cargo check -p canic-core -p canic -p canic-control-plane --locked` | PASS | Lifecycle and macro owning crates checked. |
| `cargo test -p canic-core --test trap_guard --locked` | PASS | Trap boundary guard passed. |
| `cargo test -p canic-tests --test lifecycle_boundary --locked` | PASS | 3 PocketIC lifecycle boundary tests passed. |
| `cargo test -p canic --test changelog_governance --locked` | PASS | Changelog governance check passed. |
| `cargo test -p canic --test workspace_manifest --locked` | PASS | Workspace manifest governance check passed. |
| `git diff --check` | PASS | Whitespace check passed. |

## Final Verdict

PASS.

Lifecycle hooks remain thin synchronous adapters that restore the runtime
environment and schedule bootstrap/user work through lifecycle timers.
`canic::start!()` root/non-root selection is metadata-driven, and the separate
`start_local!` / `start_wasm_store!` modes continue to honor the same lifecycle
timer boundary.
