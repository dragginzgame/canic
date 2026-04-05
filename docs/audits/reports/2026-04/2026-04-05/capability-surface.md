# Capability Surface Audit - 2026-04-05

## Report Preamble

- Scope: `crates/canic/src/macros/endpoints.rs`, `crates/canic/src/macros/start.rs`, `crates/canic-core/src/protocol.rs`, `crates/canic-core/src/dto/capability/**`, `crates/canic-core/src/dto/rpc.rs`, `crates/canic-core/src/api/rpc/**`, generated `.did` files under `.dfx/local/canisters/**`
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-29/capability-surface-2.md`
- Code snapshot identifier: `07500bcd`
- Method tag/version: `Method V2.1`
- Comparability status: `partially comparable` (wire/protocol counts are directly comparable; generated `.did` outputs were freshly rebuilt for the current canonical nine-canister roster, so the old eleven-canister per-role table is only directionally comparable)
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-05T00:00:00Z`
- Branch: `main`
- Worktree: `dirty`

## Method Changes

- Refreshed the generated `.did` surface from the default build path before scanning.
- Treated the current canonical generated roster as `app`, `minimal`, `root`, `scale`, `scale_hub`, `test`, `user_hub`, `user_shard`, and `wasm_store`.
- Added an explicit default-build check for the internal test-only query removal (`canic_env`, `canic_memory_registry`, `canic_app_directory`, `canic_subnet_directory`).

## Executive Summary

- Risk Score: `3 / 10`
- Delta summary: the default leaf surface shrank again (`minimal` `canic_*` methods `17 -> 12`), the demo/reference canisters no longer ship audit-only `*_perf_test` helpers, and the total exported `canic_*` methods across the refreshed main `<role>.did` outputs is now `149`.
- Largest growth contributor: root-only WasmStore publication/operator APIs.
- Over-bundled families: `none` in the refreshed default build; the recent internal-test query removal worked as intended.
- Follow-up required: `no` for the demo surface; the remaining work is test-harness cleanup, not shipped capability pruning.

## Hard Surface Violations

| Hard Check | Result | Evidence |
| --- | --- | --- |
| Root-only admin endpoints stay root-only | PASS | `11` `*_admin` methods remain root-only in [root.did](/home/adam/projects/canic/.dfx/local/canisters/root/root.did) |
| Shared parent/cycles receiver exists where expected | PASS | `canic_response_capability_v1` is present on all refreshed service `.did` files for `app`, `minimal`, `root`, `scale`, `scale_hub`, `test`, `user_hub`, `user_shard`, and `wasm_store` |
| Root-only wasm-store operator read surface stays root-only | PASS | `canic_wasm_store_overview` appears only on [root.did](/home/adam/projects/canic/.dfx/local/canisters/root/root.did) |
| Internal test observability/directory queries are absent from the default leaf build | PASS | after a fresh default rebuild, [minimal.did](/home/adam/projects/canic/.dfx/local/canisters/minimal/minimal.did) no longer exposes `canic_env`, `canic_memory_registry`, `canic_app_directory`, or `canic_subnet_directory` |
| No protocol constant removals or renames detected in this run | PASS | [protocol.rs](/home/adam/projects/canic/crates/canic-core/src/protocol.rs) grew additively from `23` to `27` constants |

## Baseline Delta Summary

| Category | Previous | Current | Delta | % Change |
| --- | ---: | ---: | ---: | ---: |
| Endpoint macro families | 23 | 25 | 2 | 8.70% |
| `fn canic_*` endpoint definitions in macro source | 48 | 60 | 12 | 25.00% |
| Protocol constants | 23 | 27 | 4 | 17.39% |
| RPC request variants | 5 | 5 | 0 | 0.00% |
| RPC response variants | 5 | 5 | 0 | 0.00% |
| Capability proof variants | 3 | 3 | 0 | 0.00% |
| Capability service variants | 5 | 1 | -4 | -80.00% |
| Default leaf `canic_*` baseline (`minimal`) | 17 | 12 | -5 | -29.41% |

## Endpoint Bundle Inventory

| Metric | Current Count |
| --- | ---: |
| Endpoint macro families (`emit` + `bundle`) | 25 |
| `fn canic_*` definitions in [endpoints.rs](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs) | 60 |
| Internal endpoints in macro source | 25 |
| Controller-gated endpoints in macro source | 21 |
| `*_admin` methods | 11 |
| Root-only exported endpoint families in refreshed `.did` set | 29 |
| Non-root-only exported endpoint families in refreshed `.did` set | 20 |
| Globally bundled endpoint families in refreshed `.did` set | 11 |

## Wire Surface Inventory

| Surface | Current Count |
| --- | ---: |
| `protocol.rs` constants | 27 |
| `dto::rpc::Request` variants | 5 |
| `dto::rpc::Response` variants | 5 |
| `dto::rpc::RequestFamily` variants | 5 |
| `dto::capability::CapabilityProof` variants | 3 |
| `dto::capability::CapabilityService` variants | 1 |

## Bundling vs Usage Alignment

| Endpoint Family | Roles Exposing It | Roles Requiring It | Bundling Mode | Assessment |
| --- | --- | --- | --- | --- |
| `canic_response_capability_v1` | all refreshed service canisters | root + non-root capability/cycles receiver path | `global` | aligned |
| `canic_sync_state` / `canic_sync_topology` | all non-root service canisters | parent-owned topology/state cascade targets | `non-root-only` | aligned |
| `canic_delegation_set_signer_proof` | `user_shard` | signer proof installation target only | `cfg-gated` | aligned |
| `canic_delegation_set_verifier_proof` | `test`, `user_shard` | verifier proof installation targets only | `cfg-gated` | aligned |
| `canic_wasm_store_overview` / publication status family | `root` | operator + reconcile path on root only | `root-only` | aligned |
| internal audit probes | not present in refreshed demo `.did` outputs | `instruction_audit` only | `internal test canisters` | aligned |

## Surface Utilization

| Endpoint Family | Defined | Exposed | Used | Class | Evidence |
| --- | --- | --- | --- | --- | --- |
| `canic_response_capability_v1` | yes | yes | yes | active | [protocol.rs](/home/adam/projects/canic/crates/canic-core/src/protocol.rs), [ops/rpc/mod.rs](/home/adam/projects/canic/crates/canic-core/src/ops/rpc/mod.rs) |
| `canic_sync_state` / `canic_sync_topology` | yes | yes | yes | active | [api/cascade.rs](/home/adam/projects/canic/crates/canic-core/src/api/cascade.rs), [ops/cascade.rs](/home/adam/projects/canic/crates/canic-core/src/ops/cascade.rs) |
| `canic_delegation_set_signer_proof` / `canic_delegation_set_verifier_proof` | yes | yes | yes | active | [workflow/auth.rs](/home/adam/projects/canic/crates/canic-core/src/workflow/auth.rs), [pic_role_attestation.rs](/home/adam/projects/canic/crates/canic-core/tests/pic_role_attestation.rs) |
| `canic_wasm_store_overview` / publication status family | yes | yes | yes | active | [root_wasm_store_reconcile.rs](/home/adam/projects/canic/crates/canic-tests/tests/root_wasm_store_reconcile.rs), [install_root.rs](/home/adam/projects/canic/crates/canic-installer/src/install_root.rs) |
| internal audit probes (`audit_*_probe`) | no in demo surface | yes in internal audit canisters | yes | active | [audit_leaf_probe](/home/adam/projects/canic/crates/canic-core/audit-canisters/audit_leaf_probe/src/lib.rs), [audit_root_probe](/home/adam/projects/canic/crates/canic-core/audit-canisters/audit_root_probe/src/lib.rs), [audit_scaling_probe](/home/adam/projects/canic/crates/canic-core/audit-canisters/audit_scaling_probe/src/lib.rs), [instruction_audit.rs](/home/adam/projects/canic/crates/canic-tests/tests/instruction_audit.rs) |

No `dead` endpoint families were detected in the audited set.

## DID Surface Growth

### Per-Canister Surface Table

| Canister | Total Methods | `canic_*` | Non-`canic` | Notes |
| --- | ---: | ---: | ---: | --- |
| `app` | 69 | 12 | 57 | baseline-aligned after moving audit probes off the demo canister |
| `minimal` | 69 | 12 | 57 | default leaf baseline after test-query gating and `canic_time` removal |
| `root` | 324 | 37 | 287 | main outlier |
| `scale` | 70 | 12 | 58 | baseline-aligned |
| `scale_hub` | 75 | 13 | 62 | scaling registry delta |
| `test` | 89 | 13 | 76 | verifier-only delta |
| `user_hub` | 79 | 14 | 65 | sharding registry delta |
| `user_shard` | 90 | 14 | 76 | signer/verifier delta |
| `wasm_store` | 126 | 22 | 104 | control-plane outlier |

### Outliers

Outlier rule:

- total method count > `minimal + 20%` (`69 -> 82.8` threshold), or
- `canic_*` methods exceed `minimal` by more than `5`

Detected outliers:

- `root`
- `test`
- `user_shard`
- `wasm_store`

Shared `canic_*` methods present on all refreshed service canisters:

- `canic_bootstrap_status`
- `canic_canister_children`
- `canic_canister_cycle_balance`
- `canic_canister_version`
- `canic_cycle_tracker`
- `canic_log`
- `canic_metrics`
- `canic_ready`
- `canic_response_capability_v1`
- `canic_standards`

Shared `canic_*` methods present on all non-root refreshed service canisters:

- `canic_sync_state`
- `canic_sync_topology`

Notable reduction versus the stale local default-build surface before refresh:

- `canic_env`
- `canic_memory_registry`
- `canic_app_directory`
- `canic_subnet_directory`

These no longer appear in the refreshed default `.did` outputs for ordinary leaf canisters.

## Surface Growth Attribution

| Surface Family | Current Count | Previous | Delta | Bundling Mode | Status | Risk |
| --- | ---: | ---: | ---: | --- | --- | --- |
| default shared leaf `canic_*` baseline (`minimal`) | 12 | 17 | -5 | `global` | SHRINKING | Low |
| newer root-only WasmStore publication/operator constants | 4 | 0 in prior retained summary | +4 | `root-only` | GROWING | Medium |
| root-only admin methods | 11 | 11 | 0 | `root-only` | STABLE | Medium |
| delegated auth proof-install methods | 2 families | 2 families | 0 | `cfg-gated` | STABLE | Low |
| non-root topology/sync methods | 2 | 2 | 0 | `non-root-only` | STABLE | Low |
| shipped audit probe endpoint families | 0 | 0 in prior retained summary | 0 | `internal-only` | ELIMINATED FROM DEMO SURFACE | Positive |

## Structural Hotspots

| File / Module | Surface Driver | Evidence | Risk Contribution |
| --- | --- | --- | --- |
| [endpoints.rs](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs) | shared macro fan-out | `25` endpoint macro families; `60` `fn canic_*` definitions | High |
| [protocol.rs](/home/adam/projects/canic/crates/canic-core/src/protocol.rs) | wire constant authority | `27` constants, including four newer WasmStore publication/operator names | Medium |
| [root.did](/home/adam/projects/canic/.dfx/local/canisters/root/root.did) | root control-plane concentration | `37` exported `canic_*` methods, `11` `*_admin` methods | Medium |
| [audit_leaf_probe](/home/adam/projects/canic/crates/canic-core/audit-canisters/audit_leaf_probe/src/lib.rs), [audit_root_probe](/home/adam/projects/canic/crates/canic-core/audit-canisters/audit_root_probe/src/lib.rs), [audit_scaling_probe](/home/adam/projects/canic/crates/canic-core/audit-canisters/audit_scaling_probe/src/lib.rs) | internal audit-only probe lane | audit measurement helpers now live outside the demo/reference canisters | Positive |

## Global Amplification Factor

| Surface Change | Affected Canisters | GAF | Risk |
| --- | ---: | ---: | --- |
| shared lifecycle/observability/query baseline | 9 | 9 | Medium |
| `canic_response_capability_v1` global receiver family | 9 | 9 | Medium |
| `canic_sync_state` / `canic_sync_topology` family | 8 | 8 | Medium |
| internal test-query removal from default leaf bundles | 8 | 8 | Positive |
| root-only WasmStore publication/operator family | 1 | 1 | Low |

## Compatibility Signals

| Surface | Signal | Evidence | Compatibility |
| --- | --- | --- | --- |
| protocol constants | additive growth only | [protocol.rs](/home/adam/projects/canic/crates/canic-core/src/protocol.rs) grew `23 -> 27` with no removals | additive |
| `dto::rpc::{Request,Response}` | no variant growth or removal in this run | [rpc.rs](/home/adam/projects/canic/crates/canic-core/src/dto/rpc.rs) remains `5` / `5` variants | additive |
| `CapabilityProof` | unchanged | [mod.rs](/home/adam/projects/canic/crates/canic-core/src/dto/capability/mod.rs) remains `3` variants | additive |
| `CapabilityService` | compressed | [mod.rs](/home/adam/projects/canic/crates/canic-core/src/dto/capability/mod.rs) is now `1` variant | narrowing, but not a fresh break in this run |
| default leaf `.did` surface | narrower after refresh | [minimal.did](/home/adam/projects/canic/.dfx/local/canisters/minimal/minimal.did) no longer carries the four internal test queries | narrowing |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| wire/protocol drift is now root-heavy | [protocol.rs](/home/adam/projects/canic/crates/canic-core/src/protocol.rs) | `+4` constants, all in the WasmStore publication/operator lane | Medium |
| root remains the main capability concentration point | [root.did](/home/adam/projects/canic/.dfx/local/canisters/root/root.did) | `37` exported `canic_*` methods and `324` total methods | Medium |
| default leaf shrink depends on fresh rebuilds, not stale local artifacts | [build.rs](/home/adam/projects/canic/crates/canic/src/macros/build.rs) | the intended gating is correct, but old `.dfx` outputs can mask it until rebuilt | Low |

## Endpoint / RPC Alignment

- `canic_response_capability_v1` remains aligned with RPC usage:
  - endpoint emitted in refreshed `.did` outputs
  - protocol constant present
  - request path used by [ops/rpc/mod.rs](/home/adam/projects/canic/crates/canic-core/src/ops/rpc/mod.rs)
- The newer WasmStore publication/operator constants remain aligned:
  - protocol constants present
  - root-only endpoints emitted in [root.did](/home/adam/projects/canic/.dfx/local/canisters/root/root.did)
  - exercised in [root_wasm_store_reconcile.rs](/home/adam/projects/canic/crates/canic-tests/tests/root_wasm_store_reconcile.rs)

## Conclusion

The current capability surface is still structurally controlled. The most important positive changes in this run are that the default leaf capability surface is genuinely smaller once the generated main `<role>.did` outputs are rebuilt from the normal path, and the audit-only probe endpoints no longer live on the demo/reference canisters at all. The main growth since the last retained report is now concentrated in root-only WasmStore publication/operator endpoints, not in accidental global bundling.
