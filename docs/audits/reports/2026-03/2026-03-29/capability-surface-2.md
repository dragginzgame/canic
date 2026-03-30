# Capability Surface Audit - 2026-03-29 (Rerun 2)

## Report Preamble

- Scope: `crates/canic/src/macros/endpoints.rs`, `crates/canic/src/macros/start.rs`, `crates/canic-core/src/protocol.rs`, `crates/canic-core/src/dto/capability/**`, `crates/canic-core/src/dto/rpc.rs`, `crates/canic-core/src/api/rpc/**`, generated `.did` files under `crates/canisters/**`
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-29/capability-surface.md`
- Code snapshot identifier: `f26eccd6`
- Method tag/version: `Method V2.0`
- Comparability status: `non-comparable` (audit definition now includes hard/drift split, utilization, GAF, deterministic scoring, and this rerun refreshed all generated `.did` files before scanning)
- Auditor: `codex`
- Run timestamp (UTC): `2026-03-29T18:19:47Z`
- Branch: `main`
- Worktree: `dirty`

## Method Changes

- Added the required hard-vs-drift split.
- Added normalized capability surface units and mandatory baseline delta reporting.
- Added surface utilization classification (`active` / `latent` / `dead`).
- Added Global Amplification Factor (`GAF`) and deterministic risk scoring.
- Refreshed all generated `.did` files before scanning, which removed stale interface drift from the earlier same-day baseline.

Anchor metrics retained for continuity:

- protocol constants
- RPC request variants
- RPC response variants
- capability proof variants

Metrics whose interpretation changed due refreshed `.did` artifacts or new required sections are discussed qualitatively below rather than treated as directly comparable.

## Executive Summary

- Risk Score: `2 / 10`
- Delta summary: shared wire counts stayed flat except endpoint macro inventory, which increased by `+1` generated method (`47 -> 48`) after restoring the explicit non-root `canic_response_capability_v1` branch.
- Largest growth contributor: [crates/canic/src/macros/endpoints.rs](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs) remains the fan-out hotspot, but the more important structural change in this rerun is proof payload compaction to `CapabilityProofBlob` across generated `.did` files.
- Over-bundled families: `none` confirmed in the current tree under the stricter utilization rules.
- Follow-up required: `yes` for compatibility documentation of the `CapabilityProofBlob` wire-shape change.

## Hard Surface Violations

| Hard Check | Result | Evidence |
| --- | --- | --- |
| Root-only admin endpoints stay root-only | PASS | `11` `*_admin` methods, all under [root.did](/home/adam/projects/canic/crates/canisters/root/root.did) |
| Shared parent/cycles receiver exists where expected | PASS | `canic_response_capability_v1` present on all `11` generated `.did` files |
| Root-only wasm-store operator read surface stays root-only | PASS | `canic_wasm_store_overview` appears only on `root` |
| No protocol constant removals or renames detected in this run | PASS | `protocol.rs` constant count remains `23`; canonical names still present |

## Baseline Delta Summary

| Category | Previous | Current | Delta | % Change |
| --- | ---: | ---: | ---: | ---: |
| Endpoint methods | 47 | 48 | 1 | 2.13% |
| Protocol constants | 23 | 23 | 0 | 0.00% |
| RPC request variants | 5 | 5 | 0 | 0.00% |
| RPC response variants | 5 | 5 | 0 | 0.00% |
| Capability proof variants | 3 | 3 | 0 | 0.00% |

## Endpoint Bundle Inventory

| Metric | Current Count |
| --- | ---: |
| Endpoint bundle macros | 23 |
| Generated methods | 48 |
| Admin methods | 11 |
| Controller-only endpoints | 19 |
| Internal endpoints | 15 |
| Root cfg markers | 3 |
| Non-root cfg markers | 1 |

## Wire Surface Inventory

| Surface | Current Count |
| --- | ---: |
| `protocol.rs` constants | 23 |
| `dto::rpc::Request` variants | 5 |
| `dto::rpc::Response` variants | 5 |
| `dto::rpc::RequestFamily` variants | 5 |
| `dto::capability::CapabilityProof` variants | 3 |
| `dto::capability::CapabilityService` variants | 5 |

## Bundling vs Usage Alignment

| Endpoint Family | Roles Exposing It | Roles Requiring It | Bundling Mode | Assessment |
| --- | --- | --- | --- | --- |
| `canic_response_capability_v1` | all `11` canisters | all canisters may need parent/cycles receiver semantics over time | `global` | aligned |
| `canic_sync_state` | all `10` non-root canisters | non-root topology/state cascade targets | `non-root-only` | aligned |
| `canic_sync_topology` | all `10` non-root canisters | non-root topology/state cascade targets | `non-root-only` | aligned |
| `canic_delegation_set_signer_proof` | `user_shard` | signer proof targets only | `cfg-gated` | aligned |
| `canic_delegation_set_verifier_proof` | `test`, `user_shard` | verifier proof targets only | `cfg-gated` | aligned |
| `canic_wasm_store_overview` | `root` | operator read surface on root only | `root-only` | aligned |

## Surface Utilization

| Endpoint Family | Defined | Exposed | Used | Class | Evidence |
| --- | --- | --- | --- | --- | --- |
| `canic_response_capability_v1` | yes | yes | yes | active | [ops/rpc/mod.rs](/home/adam/projects/canic/crates/canic-core/src/ops/rpc/mod.rs), [root_replay.rs](/home/adam/projects/canic/crates/canic/tests/root_replay.rs) |
| `canic_sync_state` | yes | yes | yes | active | [ops/cascade.rs](/home/adam/projects/canic/crates/canic-core/src/ops/cascade.rs), [api/cascade.rs](/home/adam/projects/canic/crates/canic-core/src/api/cascade.rs) |
| `canic_sync_topology` | yes | yes | yes | active | [ops/cascade.rs](/home/adam/projects/canic/crates/canic-core/src/ops/cascade.rs), [api/cascade.rs](/home/adam/projects/canic/crates/canic-core/src/api/cascade.rs) |
| `canic_delegation_set_signer_proof` | yes | yes | yes | active | [workflow/auth.rs](/home/adam/projects/canic/crates/canic-core/src/workflow/auth.rs) |
| `canic_delegation_set_verifier_proof` | yes | yes | yes | active | [workflow/auth.rs](/home/adam/projects/canic/crates/canic-core/src/workflow/auth.rs) |
| `canic_wasm_store_overview` | yes | yes | no in-repo caller | latent | operator endpoint only; no in-repo call site beyond macro/protocol declaration |

No `dead` endpoint families were detected in the audited set.

## DID Surface Growth

### Per-Canister Surface Table

| Canister | Total Methods | `canic_*` | Non-`canic` | Notes |
| --- | ---: | ---: | ---: | --- |
| `app` | 137 | 17 | 120 | baseline-aligned |
| `minimal` | 137 | 17 | 120 | baseline |
| `root` | 296 | 40 | 256 | outlier |
| `scale` | 137 | 17 | 120 | baseline-aligned |
| `scale_hub` | 143 | 18 | 125 | minor hub delta |
| `shard` | 137 | 17 | 120 | baseline-aligned |
| `shard_hub` | 147 | 19 | 128 | minor hub delta |
| `test` | 151 | 18 | 133 | verifier-only delta |
| `user_hub` | 146 | 19 | 127 | hub delta |
| `user_shard` | 152 | 19 | 133 | signer/verifier delta |
| `wasm_store` | 185 | 26 | 159 | outlier |

### Outliers

Outlier rule:

- total method count > `minimal + 20%` (`137 -> 164.4` threshold), or
- `canic_*` methods exceed `minimal` by more than `5`

Detected outliers:

- `root`
- `wasm_store`

Shared `canic_*` methods present on all canisters:

- `canic_response_capability_v1`

Shared `canic_*` methods present on all non-root canisters:

- `canic_sync_state`
- `canic_sync_topology`

Large shared type families present in every generated `.did`:

- `CapabilityProofBlob`
- `CapabilityService`

Notable reduction versus the earlier same-day baseline:

- `RoleAttestationProof`
- `DelegatedGrantProof`
- `DelegatedGrantScope`
- `DelegatedGrant`

These no longer fan out into every generated interface; the public wire form now uses `CapabilityProofBlob`.

## Surface Growth Attribution

| Surface Family | Current Count | Previous | Delta | Bundling Mode | Status | Risk |
| --- | ---: | ---: | ---: | --- | --- | --- |
| shared `canic_*` methods on `minimal` | 17 | 17 | 0 | `global` | STABLE | Low |
| root-only admin methods | 11 | 11 | 0 | `root-only` | STABLE | Medium |
| delegated auth proof-install methods | 2 families | 2 families | 0 | `cfg-gated` | STABLE | Low |
| topology/sync methods | 2 | 2 | 0 | `non-root-only` | STABLE | Low |
| generated endpoint inventory | 48 | 47 | 1 | `mixed` | GROWING | Low |
| shared capability proof payload shape | 1 compact blob family | concrete proof-tree fan-out in baseline | `N/A (method and artifact refresh change)` | `global` | STABLE | Low |

## Structural Hotspots

| File / Module | Surface Driver | Evidence | Risk Contribution |
| --- | --- | --- | --- |
| [endpoints.rs](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs) | shared endpoint bundles | `48` generated methods; `13` touches in last `20` commits | High |
| [protocol.rs](/home/adam/projects/canic/crates/canic-core/src/protocol.rs) | wire constant authority | `23` protocol constants; `9` recent touches | Medium |
| [dto/capability/mod.rs](/home/adam/projects/canic/crates/canic-core/src/dto/capability/mod.rs) | shared wire proof family | referenced across `23` Rust files spanning `src`, `test-canisters`, and `tests` | Medium |
| [api/rpc/capability/mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/rpc/capability/mod.rs) | capability routing / verification seam | `6` recent touches in last `20` commits; root/non-root semantics still meet here | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic/src/macros/endpoints.rs` | macro fan-out, no local import hub | 10 | 0 | 8 |
| `crates/canic-core/src/dto/capability/mod.rs` | `dto::capability` references across `src,test-canisters,tests` | 3 | 1 | 7 |
| `crates/canic-core/src/protocol.rs` | `protocol::*` references across `src,tests` | 2 | 0 | 6 |
| `crates/canic-core/src/api/rpc/capability/mod.rs` | `dto`, `ops`, `cdk` | 3 | 2 | 7 |

## Global Amplification Factor

| Surface Change | Affected Canisters | GAF | Risk |
| --- | ---: | ---: | --- |
| `canic_response_capability_v1` global receiver family | 11 | 11 | Medium |
| `canic_sync_state` / `canic_sync_topology` family | 10 | 10 | Medium |
| `CapabilityProof` payload compaction to `CapabilityProofBlob` | 11 | 11 | High |
| `canic_wasm_store_overview` root operator query | 1 | 1 | Low |

## Compatibility Signals

| Surface | Signal | Evidence | Compatibility |
| --- | --- | --- | --- |
| protocol constants | no rename/removal in this run | count remains `23` | additive |
| `dto::rpc::{Request,Response}` | no variant growth or removal in this run | `5` / `5` variants | additive |
| `CapabilityProof` wire payload shape | generated `.did` payload changed from concrete proof records to `CapabilityProofBlob` | [minimal.did](/home/adam/projects/canic/crates/canisters/minimal/minimal.did), [root.did](/home/adam/projects/canic/crates/canisters/root/root.did) | breaking risk |
| endpoint families | no family rename/removal detected | same method names remain present | additive |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| endpoint inventory drift | [endpoints.rs](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs) | generated method count increased `47 -> 48` | Low |
| root admin clustering | [root.did](/home/adam/projects/canic/crates/canisters/root/root.did) | all `11` `*_admin` methods remain root-only | Medium |
| shared DTO fan-out, compacted but still global | [dto/capability/mod.rs](/home/adam/projects/canic/crates/canic-core/src/dto/capability/mod.rs) | `CapabilityProofBlob` and `CapabilityService` still appear in all `11` `.did` files | Low |
| latent root-only operator query | [endpoints.rs](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs) | `canic_wasm_store_overview` has no in-repo caller beyond declaration | Low |

## Endpoint / RPC Alignment

- `canic_response_capability_v1` remains aligned with RPC usage:
  - endpoint emitted in `.did`
  - protocol constant present
  - request path used by [ops/rpc/mod.rs](/home/adam/projects/canic/crates/canic-core/src/ops/rpc/mod.rs)
- `canic_sync_state` and `canic_sync_topology` remain aligned with cascade RPC usage:
  - endpoint emitted on all non-root canisters
  - protocol constants present
  - outbound calls used by [ops/cascade.rs](/home/adam/projects/canic/crates/canic-core/src/ops/cascade.rs)
- `canic_wasm_store_overview` is direct-call operator surface:
  - endpoint emitted on `root`
  - no in-repo RPC caller
  - coupling risk remains low because it is `root-only`

## Dependency Fan-In Pressure

| Module / Type | Referencing Files | Referencing Subsystems | Pressure | Notes |
| --- | ---: | --- | --- | --- |
| `dto::capability` | 23 | `src`, `test-canisters`, `tests` | Medium | broadest capability DTO gravity well |
| `dto::rpc` | 14 | `root`, `src`, `test-canisters` | Medium | shared orchestration input/output family |
| `macros/endpoints.rs` | 2 explicit macro-use references | `src` | High | low import count but highest fan-out |
| `protocol.rs` | 13 | `src`, `tests` | Medium | stable but central wire authority |

## Deterministic Risk Score

Risk Score: **2 / 10**

Score contributions:

- `+0` endpoint count delta > `10%` (`2.13%`)
- `+0` DTO enum growth > `3` variants
- `+0` new global endpoint family added
- `+0` latent/dead endpoints in a global bundle
- `+1` `.did` outliers detected (`root`, `wasm_store`)
- `+1` DTO fan-out spans `>= 3` subsystems (`src`, `test-canisters`, `tests`)

Low structural risk under the stricter method. The main remaining pressure is concentrated, intentional control-plane surface on `root`, plus one explicit compatibility-risk signal from the proof wire-shape change.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| endpoint inventory `python3` scan over `crates/canic/src/macros/endpoints.rs` | PASS | captured `23` bundle macros, `48` generated methods, admin/controller/internal counts |
| wire surface `python3` scan over `crates/canic-core/src/protocol.rs`, `dto/rpc.rs`, and `dto/capability/mod.rs` | PASS | constant and enum-variant baselines captured |
| per-canister `.did` scans over `crates/canisters/*/*.did` | PASS | refreshed per-canister surface counts and family spread captured |
| `rg -n '^  canic_.*_admin :' crates/canisters -g '*.did'` | PASS | root-only admin clustering confirmed |
| `rg -n 'canic_response_capability_v1|canic_delegation_|canic_wasm_store_|canic_sync_' crates/canic-core/src crates/canic/src crates/canic/tests -g '*.rs'` | PASS | usage alignment captured for active versus latent families |
| `CARGO_TARGET_DIR=/tmp/canic-capability-audit-ws-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS | workspace lint-clean on warmed target dir |

## Follow-up Actions

1. Owner boundary: `dto/capability` + release governance
   Action: document the `CapabilityProofBlob` wire-shape change as a compatibility-risk item in the next relevant changelog or migration note.
   Target report date/run: `docs/audits/reports/2026-04/2026-04-05/capability-surface.md`
