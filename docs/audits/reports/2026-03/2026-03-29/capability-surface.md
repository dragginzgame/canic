# Capability Surface Audit - 2026-03-29

## Report Preamble

- Scope: `crates/canic/src/macros/endpoints.rs`, `crates/canic/src/macros/start.rs`, `crates/canic-core/src/protocol.rs`, `crates/canic-core/src/dto/capability.rs`, `crates/canic-core/src/dto/rpc.rs`, `crates/canic-core/src/api/rpc/**`, generated `.did` files under `canisters/**`
- Compared baseline report path: `N/A` (first run for this scope on 2026-03-29)
- Code snapshot identifier: `f26eccd6`
- Method tag/version: `Method V1.0`
- Comparability status: `non-comparable` (first recorded baseline for this recurring audit)
- Auditor: `codex`
- Run timestamp (UTC): `2026-03-29T17:35:57Z`
- Branch: `main`
- Worktree: `dirty`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Endpoint bundle inventory captured | PASS | `23` bundle macros, `47` generated methods, `11` admin methods, `19` controller-only endpoints, `14` internal endpoints in `crates/canic/src/macros/endpoints.rs` |
| Protocol and RPC surface inventory captured | PASS | `23` protocol constants; `dto::rpc::{Request,Response,RequestFamily}` each = `5` variants; `CapabilityProof` = `3`; `CapabilityService` = `5` |
| Global vs role-specific spread recorded | PASS | `canic_response_capability_v1` present on all `11` canisters; proof-install endpoints remain tightly scoped (`user_shard` signer, `test`/`user_shard` verifier) |
| DID surface growth captured | PASS | `minimal` baseline = `17` `canic_*` methods; `root` = `40` (`+23`), `wasm_store` = `26` (`+9`) |
| Structural hotspots and fan-in pressure recorded | PASS | recent churn concentrates in `crates/canic/src/macros/endpoints.rs` (`13` hits in last `20` commits) and `crates/canic-core/src/protocol.rs` (`9`) |

## Findings

### Medium

1. The capability receiver and proof DTO surface is still globally bundled across every canister, which keeps the minimum `.did` and review floor high even after the delegated-attestation timer trim.
   - Evidence:
     - `canic_response_capability_v1` appears in all `11` generated `.did` files.
     - `CapabilityProof`, `RoleAttestationProof`, and `DelegatedGrantProof` type families appear across all generated `.did` files, including `app`, `minimal`, `scale`, `shard`, and `wasm_store`.
     - `minimal` still carries `17` shared `canic_*` methods before any role-specific surface is added.
   - Why this matters: the user requirement that any canister may later become a parent is valid, but it means control-plane growth now propagates to the entire fleet unless DTO and envelope shapes are split more carefully.

2. `root` remains the clear surface-growth hotspot, with operator/admin control concentrated in one canister and one endpoint macro file.
   - Evidence:
     - `root.did` exposes `40` `canic_*` methods versus the `minimal` baseline of `17`.
     - all `11` `*_admin` methods are root-only.
     - recent churn counts: `crates/canic/src/macros/endpoints.rs` = `13` recent touches, `crates/canic-core/src/protocol.rs` = `9`.
   - Why this matters: the centralization is directionally correct, but `root` is now the primary place where small control-plane additions turn into permanent public-surface growth.

### Low

3. Delegated proof installation endpoints are already well-scoped, so the next capability-surface reductions should focus on shared DTO/protocol fan-out rather than further endpoint gating.
   - Evidence:
     - `canic_delegation_set_signer_proof` appears only on `user_shard`.
     - `canic_delegation_set_verifier_proof` appears only on `test` and `user_shard`.
     - `canic_wasm_store_overview` appears only on `root`.
   - Why this matters: the current problem is not broad endpoint leakage for delegated proof installation; it is global type and protocol coupling.

## Bundling Pressure Assessment

| Surface Family | Roles Exposing It | Bundling Mode | Pressure Source | Status | Risk |
| --- | --- | --- | --- | --- | --- |
| `canic_response_capability_v1` | all `11` canisters | `global` | capability / RPC / parent-child control plane | over-bundled | Medium |
| `canic_sync_state`, `canic_sync_topology` | all `10` non-root canisters | `non-root-only` | topology / state sync | stable | Low |
| delegated proof install endpoints | `user_shard`, `test` | `cfg-gated` | auth / delegation / attestation | stable | Low |
| `canic_wasm_store_*` operator queries/admin | `root` only | `root-only` | wasm / template control plane | growing | Medium |

## DID Surface Growth

| Canister | `canic_*` Method Count | Delta vs `minimal` |
| --- | ---: | ---: |
| `app` | 17 | 0 |
| `minimal` | 17 | 0 |
| `root` | 40 | 23 |
| `scale` | 17 | 0 |
| `scale_hub` | 18 | 1 |
| `shard` | 17 | 0 |
| `shard_hub` | 19 | 2 |
| `test` | 18 | 1 |
| `user_hub` | 19 | 2 |
| `user_shard` | 19 | 2 |
| `wasm_store` | 26 | 9 |

Shared `canic_*` methods present on all canisters:
- `canic_response_capability_v1`

Shared `canic_*` methods present on all non-root canisters:
- `canic_sync_state`
- `canic_sync_topology`

Large DTO/proof type families present in every generated `.did`:
- `CapabilityProof`
- `CapabilityService`
- `RoleAttestationProof`
- `DelegatedGrantProof`

## Structural Hotspots

| File / Module | Surface Driver | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic/src/macros/endpoints.rs` | shared endpoint bundles | single macro fan-out point for `47` generated methods and all root admin surface | High |
| `crates/canic-core/src/protocol.rs` | protocol method constants | wire-level authority for `23` named methods/constants used across endpoint and RPC seams | Medium |
| `crates/canic-core/src/dto/capability.rs` | capability DTO family | proof and envelope types fan out into every canister interface | High |
| `crates/canic-core/src/api/rpc/capability/mod.rs` | capability verification / routing hub | centralizes root/non-root capability semantics, proof validation, and structural cycles handling | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic/src/macros/endpoints.rs` | macro body uses fully qualified `core::api::*` paths; no local `use` fan-in | 10 | 0 | 8 |
| `crates/canic-core/src/dto/capability.rs` | `dto` | 1 | 1 | 6 |
| `crates/canic-core/src/protocol.rs` | none | 0 | 0 | 7 |
| `crates/canic-core/src/api/rpc/capability/mod.rs` | `dto`, `ops`, `cdk` | 3 | 2 | 7 |

Pressure rationale:
- high scores here come from fan-out gravity and recent churn concentration, not only raw import count

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| global endpoint family growth | `crates/canic/src/macros/endpoints.rs` + generated `.did` files | `canic_response_capability_v1` appears on all `11` canisters, including `minimal` | Medium |
| shared DTO fan-out | `crates/canic-core/src/dto/capability.rs` | proof families and capability enums appear in every generated `.did` | Medium |
| admin surface clustering | `canisters/root/root.did` | all `11` `*_admin` methods are root-only, with `root` at `40` total `canic_*` methods | Medium |

## Capability Surface Growth Table

| Surface Family | Current Count | Previous | Delta | Bundling Mode | Risk |
| --- | ---: | ---: | ---: | --- | --- |
| shared `canic_*` methods on `minimal` | 17 | `N/A` | `N/A` | `global` | Medium |
| root-only admin methods | 11 | `N/A` | `N/A` | `root-only` | Medium |
| delegated auth proof-install methods | 2 families (`signer`, `verifier`) | `N/A` | `N/A` | `cfg-gated` | Low |
| topology / sync methods | 2 | `N/A` | `N/A` | `non-root-only` | Low |

## Dependency Fan-In Pressure

Surface-defining fan-in snapshot:
- `dto::rpc` referenced by `17` Rust files in `crates/`
- `protocol::*` referenced by `13` Rust files in `crates/`
- `dto::capability` referenced by `6` Rust files in `crates/`
- `canic_endpoints_*` macro bundle family defined centrally in `crates/canic/src/macros/endpoints.rs`

Interpretation:
- `dto::rpc` and `protocol.rs` currently provide the widest shared gravity wells in the audited slice
- endpoint growth still fans out primarily through one macro file rather than many role-local entrypoints

## Risk Score

Risk Score: **6 / 10**

Score contributions:
- `+3` globally bundled capability receiver and proof DTO spread across all canisters
- `+2` root-only admin/control plane concentrated in one public surface hotspot
- `+1` recent churn concentrated in `endpoints.rs` and `protocol.rs`
- `+0` delegated proof install endpoints, which are already well scoped

Moderate structural risk. No correctness failure was identified in this pass, but future control-plane additions will continue to widen the global `.did` floor unless DTO and envelope shapes are split more deliberately.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `python3` inventory scan over `crates/canic/src/macros/endpoints.rs` | PASS | captured bundle macro count, generated method count, admin/controller/internal counts |
| `python3` variant-count scan over `crates/canic-core/src/dto/{rpc,capability}.rs` | PASS | captured request/response/proof/service variant counts |
| `python3` `.did` surface scans over `canisters/*/*.did` | PASS | captured per-canister `canic_*` counts and endpoint-family spread |
| `rg -n '^  canic_.*_admin :' canisters -g '*.did'` | PASS | confirmed all admin endpoints are root-only |
| `rg -n 'RoleAttestationProof|DelegatedGrantProof|CapabilityProof|CapabilityService' canisters -g '*.did'` | PASS | confirmed proof/type fan-out across all generated interfaces |
| `git log --format='' --name-only -n 20 -- crates/canic/src/macros/endpoints.rs crates/canic-core/src/protocol.rs crates/canic-core/src/api/rpc` | PASS | captured recent churn concentration for hotspot scoring |

## Follow-up Actions

1. Owner boundary: `dto/capability` + `api/rpc`
   Action: split root/operator proof and envelope shapes from the globally shared capability surface where possible, without removing the user-required parent-evolution endpoint surface.
   Target report date/run: `docs/audits/reports/2026-04/2026-04-05/capability-surface.md`

2. Owner boundary: `canic` macros + `protocol`
   Action: add a lightweight recurring metric for `minimal.did` shared-method count and `root.did` admin count so future control-plane additions visibly move a tracked baseline.
   Target report date/run: `docs/audits/reports/2026-04/2026-04-05/capability-surface.md`

3. Owner boundary: `root` control plane
   Action: keep new wasm/template/bootstrap/admin entrypoints centralized on `root`, but require each new addition to justify whether it must change shared DTO/protocol families or can stay root-local.
   Target report date/run: `docs/audits/reports/2026-04/2026-04-05/capability-surface.md`
