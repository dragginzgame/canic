# Bootstrap Lifecycle Symmetry Audit - 2026-03-10

## Report Preamble

- Scope: lifecycle macros/adapters/bootstrap pipeline
- Compared baseline report path: `N/A` (first run for this scope on 2026-03-10)
- Code snapshot identifier: `fa06bfef`
- Method tag/version: `Method V4.0`
- Comparability status: `non-comparable` (method expanded with hotspots, predictive signals, fan-in pressure)
- Auditor: `codex`
- Run timestamp (UTC): `2026-03-10T14:30:36Z`
- Branch: `main`
- Worktree: `dirty`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Macro hooks stay thin and timer-driven | PASS | `start.rs` uses `TimerApi::set_lifecycle_timer(Duration::ZERO, ...)` |
| Lifecycle API remains delegation-only | PASS | `crates/canic-core/src/api/lifecycle.rs` delegates to lifecycle adapters |
| Init/post-upgrade execution model symmetry | PASS | both paths restore/init synchronously and schedule bootstrap async |
| No direct await in synchronous adapter flow | PASS | `.await` appears inside timer closures in `init.rs`/`upgrade.rs` |
| Restore-before-bootstrap ordering | PASS | `EnvOps::restore_*` occurs before `TimerOps::set(Duration::ZERO, ...)` |
| Lifecycle integration tests | PASS | `cargo test -p canic --test lifecycle_boundary --locked` passed |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic/src/macros/start.rs` | lifecycle macro hooks | primary lifecycle wiring boundary | High |
| `crates/canic-core/src/lifecycle/init.rs` | `init_root_canister`, `init_nonroot_canister` | synchronous/async phase split contract | High |
| `crates/canic-core/src/lifecycle/upgrade.rs` | `post_upgrade_*` adapters | restore-before-bootstrap ordering contract | High |
| `crates/canic-core/src/workflow/runtime/mod.rs` | runtime init/restore functions | lifecycle state initialization anchor | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/lifecycle/init.rs` | `workflow, dto, lifecycle` | 3 | 1 | 6 |
| `crates/canic-core/src/lifecycle/upgrade.rs` | `workflow, ops, lifecycle` | 3 | 1 | 6 |
| `crates/canic-core/src/workflow/runtime/mod.rs` | `workflow, api, ops` | 3 | 2 | 7 |

## Primary Architectural Pressure

`crates/canic-core/src/workflow/runtime/mod.rs`

Reasons:
- imports multiple subsystems (`workflow, api, ops`)
- central runtime boundary for init/post-upgrade paths
- highest hub pressure score in this run (`7`)

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| growing hub module | `crates/canic-core/src/lifecycle/init.rs` | touched in `20` recent commits | Medium |
| growing hub module | `crates/canic-core/src/workflow/runtime/mod.rs` | touched in `20` recent commits | Medium |
| capability surface growth | `crates/canic-core/src/ops/runtime/env/mod.rs` | `20` public items | Low |

## Dependency Fan-In Pressure

No fan-in pressure detected in this run.

## DTO Fan-In (Expected)

DTO sharing across lifecycle edges is expected for bootstrap payload transport and is not treated as a structural violation by itself.

## Risk Score

Risk Score: **3 / 10**

Score contributions:
- `+1` lifecycle hub pressure (`init.rs`/`upgrade.rs`)
- `+1` workflow runtime hub pressure (`workflow/runtime/mod.rs`)
- `+1` lifecycle churn signal (`20` recent touches on init/runtime files)

Lifecycle invariants currently hold. Residual low risk is from high-change lifecycle boundary files where ordering regressions can be introduced during future refactors.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic --test lifecycle_boundary --locked` | PASS | `3 passed; 0 failed` |
| `cargo test -p canic-core --lib --locked` | PASS | includes lifecycle-related unit coverage |

## Follow-up Actions

1. Re-run this audit immediately after any lifecycle macro changes.
2. Keep init/post-upgrade structure diff-visible in code review to catch ordering drift.
