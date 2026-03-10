# Change Friction Audit - 2026-03-10

## Report Preamble

- Scope: `crates/canic-core/src/**` and recent feature slices
- Compared baseline report path: `N/A` (first run for this scope on 2026-03-10)
- Code snapshot identifier: `fa06bfef`
- Method tag/version: `Method V4.0`
- Comparability status: `non-comparable` (method expanded with hotspots, amplification, predictive signals, fan-in pressure)
- Auditor: `codex`
- Run timestamp (UTC): `2026-03-10T14:30:36Z`
- Branch: `main`
- Worktree: `dirty`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Feature slices sampled from recent commits | PASS | last-20 commit spread scan with subsystem/layer counts |
| Revised CAF computed per slice | PASS | top slices include CAF `20`, `16`, `24` |
| Release sweeps separated from routine slices | PASS | `7d43cd7e` classified as `release_sweep` and excluded from routine friction trend interpretation |
| Slice density computed | PASS | density measured as `files/subsystems` across sampled slices |
| Blast radius measured | PASS | largest sampled slice touched `49` files |
| Boundary leakage reviewed | PASS | no new hard layering bypass found in sampled slices |
| Friction amplification drivers identified | PASS | highest spread commits mapped to impacted files |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/workflow/rpc/request/handler/mod.rs` | `RootResponseWorkflow` | repeated multi-slice orchestration edits | Medium |
| `crates/canic-core/src/config/schema/subnet.rs` | subnet schema parsing/validation | appears in high-file-count slices | Medium |
| `crates/canic-core/src/ops/replay/guard.rs` | replay decision gate | replay/auth slices repeatedly touch this boundary | Medium |
| `crates/canic-core/src/access/auth.rs` | canonical auth boundary | high churn (`19` recent touches) | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/workflow/runtime/mod.rs` | `workflow, api, ops` | 3 | 2 | 7 |
| `crates/canic-core/src/workflow/rpc/request/handler/mod.rs` | `workflow, dto, ops` | 3 | 2 | 6 |
| `crates/canic-core/src/access/auth.rs` | `access, ops, dto, config` | 4 | 2 | 6 |

## Primary Architectural Pressure

`crates/canic-core/src/workflow/runtime/mod.rs`

Reasons:
- imports multiple subsystems (`workflow, api, ops`)
- central runtime/orchestration boundary that many slices traverse
- highest hub pressure score in this run (`7`)

## Amplification Drivers

| Commit | Feature Slice | Slice Type | Files Touched | Subsystems | Density (Files/Subsystem) | CAF | Risk |
| --- | --- | --- | ---: | --- | ---: | ---: | --- |
| `7d43cd7e` | 0.13.8 prep + replay/workflow/test sweep | `release_sweep` | 49 | `ops/workflow` | 24.50 | 8 | Medium |
| `eeda9dbd` | capability + config + funding changes | `feature_slice` | 25 | `api/ops/workflow/policy/dto` | 5.00 | 20 | High |
| `ce7c843d` | access + rpc handler + storage/env update | `feature_slice` | 18 | `ops/storage/workflow/policy/endpoints` | 3.60 | 24 | High |
| `8bceeb05` | metrics + handler + endpoint consolidation | `feature_slice` | 18 | `ops/workflow/endpoints/dto` | 4.50 | 16 | Medium |

Density interpretation:

- high density with low subsystem count indicates deeper focused edits
- low density with high subsystem count indicates broader cross-system friction
- routine friction signal in this run comes from `feature_slice` rows (`eeda9dbd`, `ce7c843d`, `8bceeb05`)

Most impacted files in high-CAF slices:

- `crates/canic-core/src/workflow/rpc/request/handler/mod.rs`
- `crates/canic-core/src/api/rpc/capability/mod.rs`
- `crates/canic-core/src/config/schema/subnet.rs`
- `crates/canic-core/src/ops/replay/guard.rs`

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| growing hub module | `crates/canic-core/src/workflow/runtime/mod.rs` | touched in `20` recent commits | Medium |
| enum shock radius | `crates/canic-core/src/dto/rpc.rs` | `Request`/`Response` referenced in `14`/`13` files | Medium |
| cross-layer struct spread | `RoleAttestation` | references across `api/ops/workflow` | Medium |
| capability surface growth | `crates/canic-core/src/ops/runtime/env/mod.rs` | `20` public items | Medium |

## Dependency Fan-In Pressure

### Module Fan-In

| Module | Import Count | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `dto/prelude/*` | 11 | `dto` | Architectural gravity well (single-subsystem hub) |
| `dto` | 8 | `api/dto` | Hub forming |

## DTO Fan-In (Expected)

DTO fan-in is expected for boundary transport types and is separated from runtime gravity-well scoring.

### Struct Fan-In

| Struct | Defined In | Reference Count | Layers Referencing | Risk |
| --- | --- | ---: | --- | --- |
| `RoleAttestation` | `crates/canic-core/src/dto/auth.rs` | 15 | `api/ops/workflow` | High |
| `DelegationProof` | `crates/canic-core/src/dto/auth.rs` | 9 | `api/ops/workflow` | Medium |
| `ReplaySlotKey` | `crates/canic-core/src/ops/replay/key.rs` | 7 | `ops/workflow` | Low |

## Risk Score

Risk Score: **5 / 10**

Score contributions:
- `+2` high amplification drivers (CAF `20` and `24`)
- `+1` workflow runtime hub pressure (`7`)
- `+1` cross-layer struct spread (`RoleAttestation`)
- `+1` capability surface growth (`ops/runtime/env/mod.rs`)
- `+0` release-sweep inflation (`7d43cd7e` treated separately)

Moderate risk from high-CAF multi-subsystem slices and repeated edits to workflow/auth/replay coordination files. No immediate correctness failure was detected.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo tree -e features` | PASS | dependency graph resolved |
| `cargo test -p canic-core --lib --locked` | PASS | `230 passed; 0 failed` |
| `cargo test -p canic --test delegation_flow --locked` | PASS | `7 passed; 0 failed` |
| `cargo test -p canic --test lifecycle_boundary --locked` | PASS | `3 passed; 0 failed` |

## Follow-up Actions

1. Keep commit-level CAF tracking on `workflow/rpc/request/handler/*` through the next patch cycle.
2. Split high-churn replay + workflow adjustments into smaller slices when possible to keep file blast radius below `25`.
