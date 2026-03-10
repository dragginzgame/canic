# Complexity Accretion Audit - 2026-03-10

## Report Preamble

- Scope: `crates/canic-core/src/**`
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
| Runtime size baseline captured | PASS | `runtime_files=326`, `runtime_loc=25372`, `files>=600=7` |
| Variant surface growth monitored | PASS | `Request` refs `14`, `Response` refs `13`, `InfraError` refs `12`, `DelegationValidationError` refs `11` |
| Branching pressure hotspots identified | PASS | runtime hubs include `access/expr.rs`, `ops/storage/intent.rs`, `workflow/rpc/request/handler/mod.rs` |
| Cross-cutting auth/replay spread checked | PASS | auth/replay enums and structs referenced across `api/ops/workflow` |
| Cognitive load hub files tracked | PASS | `ops/storage/intent.rs` `741 LOC`, `workflow/bootstrap/root.rs` `642 LOC` |
| Test complexity tracked separately | PASS | `workflow/rpc/request/handler/tests.rs` recorded as test hotspot, excluded from runtime risk contribution |

## Structural Hotspots

### Runtime Complexity Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/access/expr.rs` | `eval_access` and predicate model | highest control-surface file (`1023 LOC`) | Medium |
| `crates/canic-core/src/ops/storage/intent.rs` | intent aggregation/storage ops | high state-transition density (`741 LOC`) | Medium |
| `crates/canic-core/src/workflow/placement/sharding/mod.rs` | sharding placement workflow | branching topology decisions (`691 LOC`) | Medium |
| `crates/canic-core/src/workflow/rpc/request/handler/mod.rs` | `RootResponseWorkflow` | orchestration control hub with high branch density | Medium |

### Test Complexity Hotspots

| Test File / Module | Reason | Tracking Impact |
| --- | --- | --- |
| `crates/canic-core/src/workflow/rpc/request/handler/tests.rs` | large request-pipeline harness (`962 LOC`) | Medium |

## Control Surface Detection

| Control Surface | File | Responsibility | Risk |
| --- | --- | --- | --- |
| `eval_access` | `crates/canic-core/src/access/expr.rs` | capability/auth evaluation engine | Medium |
| `runtime bootstrap` | `crates/canic-core/src/workflow/runtime/mod.rs` | system initialization coordination | Medium |
| `intent aggregation` | `crates/canic-core/src/ops/storage/intent.rs` | state transition aggregation boundary | Medium |

## Branching Density

| File | Logical LOC | `match` | `if` | `else if` | Branch Density (/100 LOC) | Runtime/Test | Risk |
| --- | ---: | ---: | ---: | ---: | ---: | --- | --- |
| `crates/canic-core/src/workflow/rpc/request/handler/mod.rs` | 165 | 4 | 2 | 0 | 3.64 | Runtime | High |
| `crates/canic-core/src/ops/storage/intent.rs` | 581 | 6 | 13 | 0 | 3.27 | Runtime | High |
| `crates/canic-core/src/access/expr.rs` | 754 | 13 | 3 | 0 | 2.12 | Runtime | Medium |
| `crates/canic-core/src/workflow/placement/sharding/mod.rs` | 538 | 2 | 8 | 0 | 1.86 | Runtime | Medium |
| `crates/canic-core/src/workflow/runtime/mod.rs` | 224 | 3 | 1 | 0 | 1.79 | Runtime | Medium |
| `crates/canic-core/src/workflow/rpc/request/handler/tests.rs` | 860 | 6 | 1 | 0 | 0.81 | Test | Low |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/access/expr.rs` | `access, dto, ids, log` | 3 | 2 | 7 |
| `crates/canic-core/src/workflow/runtime/mod.rs` | `workflow, ops, api` | 3 | 2 | 7 |
| `crates/canic-core/src/ops/storage/intent.rs` | `ops, storage, ids` | 3 | 1 | 5 |

## Primary Architectural Pressure

`crates/canic-core/src/workflow/runtime/mod.rs`

Reasons:
- imports multiple subsystems (`workflow, ops, api`)
- central orchestration/runtime boundary
- highest hub pressure tier in this run (`7`)

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| enum shock radius | `crates/canic-core/src/dto/rpc.rs` | `Request` referenced in `14` files, `Response` in `13` files | Medium |
| growing hub module | `crates/canic-core/src/workflow/runtime/mod.rs` | touched in `20` recent commits | Medium |
| cross-layer struct spread | `RoleAttestation` (`crates/canic-core/src/dto/auth.rs`) | referenced across `api/ops/workflow` (`15` files) | Medium |
| capability surface growth | `crates/canic-core/src/ops/runtime/env/mod.rs` | `20` public items (`pub fn`/`pub struct`) | Medium |

## Dependency Fan-In Pressure

### Module Fan-In

| Module | Import Count | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `dto/prelude/*` | 11 | `dto` | Architectural gravity well (single subsystem) |
| `dto` | 8 | `api/dto` | Hub forming |

## DTO Fan-In (Expected)

DTO fan-in is expected at transport boundaries. It is tracked separately from runtime architecture hubs to avoid false-positive gravity-well alerts.

### Struct Fan-In

| Struct | Defined In | Reference Count | Layers Referencing | Risk |
| --- | --- | ---: | --- | --- |
| `RoleAttestation` | `crates/canic-core/src/dto/auth.rs` | 15 | `api/ops/workflow` | High |
| `DelegationProof` | `crates/canic-core/src/dto/auth.rs` | 9 | `api/ops/workflow` | Medium |
| `DelegatedTokenClaims` | `crates/canic-core/src/dto/auth.rs` | 7 | `api/ops` | Low |

## Risk Score

Risk Score: **5 / 10**

Score contributions:
- `+2` high hub pressure (`access/expr.rs`, `workflow/runtime/mod.rs`)
- `+1` enum shock radius (`Request`/`Response`)
- `+1` cross-layer struct spread (`RoleAttestation`)
- `+1` capability surface growth (`ops/runtime/env/mod.rs`)
- `+0` test hotspot contribution (tracked separately from runtime risk)

Moderate risk driven by high LOC control files, medium enum shock radius, and cross-layer auth DTO fan-in. No critical invariant break was observed in this complexity pass.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo tree -e features` | PASS | dependency graph resolved |
| `cargo test -p canic-core --lib --locked` | PASS | `230 passed; 0 failed` |
| `cargo test -p canic --test delegation_flow --locked` | PASS | `7 passed; 0 failed` |
| `cargo test -p canic --test lifecycle_boundary --locked` | PASS | `3 passed; 0 failed` |

## Follow-up Actions

1. Track `access/expr.rs` and `workflow/runtime/mod.rs` fan-in trend in the next recurring run.
2. If `Request`/`Response` enum reference counts rise above `16`, split handler dispatch seams before next release.
