# Layer Violations Audit - 2026-03-10

## Report Preamble

- Scope: `crates/canic-core/src/{api,workflow,domain,ops,storage,lifecycle,access}`
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
| No upward `workflow/ops/storage/domain -> api` imports | PASS | scan on `use crate::api` in lower layers found no runtime violations |
| No upward `ops/storage/domain -> workflow` imports | PASS | scan on `use crate::workflow` in lower layers found no runtime violations |
| Policy purity (no async/side effects) | PASS | policy scans did not find runtime side-effect calls |
| DTO leakage into `domain/storage` | PASS | no direct DTO ownership leakage detected |
| Lifecycle adapter boundary discipline | PASS | lifecycle modules schedule bootstrap via timer and delegate orchestration |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/lifecycle/init.rs` | init adapters | lifecycle boundary contract is sensitive to layering drift | Medium |
| `crates/canic-core/src/lifecycle/upgrade.rs` | post-upgrade adapters | restore/schedule ordering must stay thin and deterministic | Medium |
| `crates/canic-core/src/workflow/rpc/request/handler/mod.rs` | request pipeline boundary | orchestration module near policy/ops boundary edges | Medium |
| `crates/canic-core/src/ops/replay/guard.rs` | replay guard | storage/workflow seam with strict boundary expectations | Low |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/workflow/runtime/mod.rs` | `workflow, api, ops` | 3 | 2 | 7 |
| `crates/canic-core/src/lifecycle/init.rs` | `workflow, lifecycle, dto` | 3 | 1 | 6 |
| `crates/canic-core/src/lifecycle/upgrade.rs` | `workflow, lifecycle, ops` | 3 | 1 | 6 |

## Responsibility Drift Signals

No behavioral layer drift detected.

### Workflow Layer Drift

| Location | Pattern | Drift Risk | Why |
| --- | --- | --- | --- |
| `crates/canic-core/src/workflow/rpc/request/handler/mod.rs` | `verify`/`hash` helpers | Low | request validation helpers inside workflow orchestration; no direct crypto side-effect APIs detected |

### Policy Layer Drift

| Location | Pattern | Drift Risk | Why |
| --- | --- | --- | --- |
| None | None | None | no `.await`, `spawn`, `stable_`, or `env::` behavioral drift signal in policy runtime paths |

### Lifecycle Adapter Drift

| File | Lines | Drift Signal | Risk |
| --- | ---: | --- | --- |
| `crates/canic-core/src/lifecycle/init.rs` | 94 | `workflow::` references for bootstrap scheduling | Low |
| `crates/canic-core/src/lifecycle/upgrade.rs` | 122 | `ops::`/`workflow::` references for restore + schedule | Low |

### DTO Responsibility Drift

| File | Signal | Risk |
| --- | --- | --- |
| None | no `async fn` or behavioral impl drift requiring escalation in DTO runtime paths | None |

## Architecture Watchpoint

`crates/canic-core/src/workflow/runtime/mod.rs`

Reasons:
- imports multiple subsystems (`workflow, api, ops`)
- central orchestration boundary for lifecycle/runtime initialization
- highest hub pressure score in this audit (`7`)
- this module should remain thin orchestration only

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| growing hub module | `crates/canic-core/src/lifecycle/init.rs` | touched in `20` recent commits | Medium |
| cross-layer struct spread | `RoleAttestation` | referenced across `api/ops/workflow` | Low |
| capability surface growth | `crates/canic-core/src/ops/runtime/env/mod.rs` | `20` public items | Medium |

## Dependency Fan-In Pressure

### Module Fan-In

| Module | Import Count | Layers Referencing | Pressure |
| --- | ---: | --- | --- |
| `dto/prelude/*` | 11 | `dto` | Expected DTO hub |
| `dto` | 8 | `api/dto` | Expected DTO hub |

## DTO Fan-In (Expected)

DTO sharing is expected across boundaries. DTO fan-in is treated as informational unless it starts driving policy/workflow decision ownership.

### Struct Fan-In

| Struct | Defined In | Reference Count | Layers Referencing | Risk |
| --- | --- | ---: | --- | --- |
| `RoleAttestation` | `crates/canic-core/src/dto/auth.rs` | 15 | `api/ops/workflow` | Medium |
| `DelegationProof` | `crates/canic-core/src/dto/auth.rs` | 9 | `api/ops/workflow` | Low |

## Risk Score

Risk Score: **3 / 10**

Score contributions:
- `+1` workflow runtime hub pressure (`7`)
- `+1` lifecycle boundary churn pressure (`init`/`upgrade` recent touches)
- `+1` capability surface growth signal (`ops/runtime/env/mod.rs`)
- `+0` responsibility drift signals (none detected)

No hard layering violation was found. Risk remains low and is driven by churn on lifecycle/workflow boundary files that are sensitive to future drift.

## Architecture Health Interpretation

| Dimension | Status |
| --- | --- |
| Layer invariants | Excellent |
| Policy purity | Clean |
| Lifecycle boundary | Stable |
| Workflow orchestration | Hub forming |
| DTO sharing | Expected |

Current interpretation: healthy architecture with centralizing runtime orchestration.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo tree -e features` | PASS | dependency graph resolved |
| `cargo test -p canic-core --lib --locked` | PASS | `230 passed; 0 failed` |
| `cargo test -p canic --test lifecycle_boundary --locked` | PASS | `3 passed; 0 failed` |

## Follow-up Actions

1. Re-check lifecycle boundary scans after any macro or lifecycle adapter edits.
2. Keep workflow test-only storage imports from leaking into runtime modules.
