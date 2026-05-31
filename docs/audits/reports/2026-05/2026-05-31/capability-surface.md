# Capability Surface Audit - 2026-05-31

## Report Preamble

- Scope: `crates/canic/src/macros/endpoints/**`,
  `crates/canic/src/macros/start.rs`, `crates/canic-core/src/protocol.rs`,
  `crates/canic/src/protocol.rs`, `crates/canic-core/src/dto/capability/**`,
  `crates/canic-core/src/dto/rpc.rs`, `crates/canic-core/src/api/rpc/**`,
  generated `.did` files under `.icp/local/canisters/**`
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-09/capability-surface.md`
- Code snapshot identifier: `f8c2149c`
- Method tag/version: `Current Method / endpoint-directory-module update`
- Comparability status: `partially comparable`. Endpoint and wire counts are
  comparable after adapting the template's old `endpoints.rs` path to the
  current `endpoints/**` directory module. DID totals are partially comparable:
  this run uses the current fleet roster from `fleets/test/canic.toml`
  (`app`, `user_hub`, `user_shard`, `scale_hub`, `scale_replica`, `root`),
  while the prior run included `minimal`, `scale`, and `wasm_store`.
- Generated artifact environment: `.icp/local`
- Retained public roster: `app`, `user_hub`, `user_shard`, `scale_hub`,
  `scale_replica`, `root`
- Filtered artifacts: `minimal`, `scale`, `test`, and `wasm_store` remained
  present under `.icp/local/canisters/**` but were not part of the current
  fleet roster scan.
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-31T11:15:38Z`
- Branch: `main`
- Worktree: `dirty`

## Artifact Refresh Commands

| Command | Status | Output |
| --- | --- | --- |
| `target/debug/canic build test app` | PASS | `.icp/local/canisters/app/app.wasm.gz` |
| `target/debug/canic build test user_hub` | PASS | `.icp/local/canisters/user_hub/user_hub.wasm.gz` |
| `target/debug/canic build test user_shard` | PASS | `.icp/local/canisters/user_shard/user_shard.wasm.gz` |
| `target/debug/canic build test scale_hub` | PASS | `.icp/local/canisters/scale_hub/scale_hub.wasm.gz` |
| `target/debug/canic build test scale_replica` | PASS | `.icp/local/canisters/scale_replica/scale_replica.wasm.gz` |
| `target/debug/canic build test root` | PASS | `.icp/local/canisters/root/root.wasm.gz` |

## Executive Summary

- Risk Score: `3 / 10`
- Delta summary: endpoint macro families stayed `26 -> 26`; generated endpoint
  definitions changed `56 -> 52`; `canic-core::protocol` constants changed
  `25 -> 30`; RPC request/response/family variants changed `5 -> 6`; capability
  proof/service variants stayed `3` and `1`.
- Largest growth contributor: public/root request-family expansion and the
  retained default `canic_memory_ledger` diagnostic surface in current refreshed
  DIDs.
- Over-bundled families: `none confirmed`. Shared runtime families are global
  by design, non-root cascade endpoints stay non-root-only, and root/operator
  families stay root-only in the retained roster.
- Follow-up required: `yes`: the recurring audit template macro path was
  updated after this run, its wire-scan scope now includes both the core and
  facade protocol tables, and it now requires roster-derived DID scans. Keep
  `canic_memory_ledger` documented and tested as the standing watchpoint.

## Hard Surface Violations

| Hard Check | Result | Evidence |
| --- | --- | --- |
| Root-only admin endpoints stay root-only | PASS | `rg '^  canic_.*_admin :' .icp/local/canisters -g '*.did'` found `6` entries, all in `root/root.did`. |
| Shared parent/cycles receiver exists where expected | PASS | `canic_response_capability_v1` appears on all retained public roles. |
| Root-only auth and WasmStore operator read surface stays root-only | PASS | `canic_request_delegation`, `canic_request_role_attestation`, `canic_request_internal_invocation_proof`, and `canic_wasm_store_*` root/operator methods appear only on `root`. |
| Non-root cascade endpoints stay non-root-only | PASS | `canic_sync_state` and `canic_sync_topology` appear on `app`, `user_hub`, `user_shard`, `scale_hub`, and `scale_replica`, not `root`. |
| Removed env/memory registry surface stays absent from retained roster | PASS | `canic_env` and `canic_memory_registry` did not appear in retained public DID files. |
| Default memory ledger diagnostic is intentional | PASS | `canic_memory_ledger` appears in retained public DID files and is guarded by `crates/canic/tests/protocol_surface.rs`. |
| Retired delegation proof-install endpoints are absent | PASS | No `canic_delegation_set_*` methods were found in current code or retained DID files. |

## Baseline Delta Summary

| Category | Previous | Current | Delta | % Change |
| --- | ---: | ---: | ---: | ---: |
| Endpoint macro families | 26 | 26 | 0 | 0.00% |
| `fn canic_*` endpoint definitions in macro source | 56 | 52 | -4 | -7.14% |
| `canic-core::protocol` constants | 25 | 30 | +5 | +20.00% |
| `canic::protocol` facade-only constants | N/A | 24 | N/A | N/A |
| RPC request variants | 5 | 6 | +1 | +20.00% |
| RPC response variants | 5 | 6 | +1 | +20.00% |
| RPC request-family variants | 5 | 6 | +1 | +20.00% |
| Capability proof variants | 3 | 3 | 0 | 0.00% |
| Capability service variants | 1 | 1 | 0 | 0.00% |
| Default retained leaf `canic_*` baseline (`app`) | 12 | 13 | +1 | +8.33% |

## Endpoint Bundle Inventory

| Metric | Current Count |
| --- | ---: |
| Endpoint macro families (`emit` + `bundle`) | 26 |
| `fn canic_*` definitions in `crates/canic/src/macros/endpoints/**` | 52 |
| Internal endpoint attributes in macro source | 24 |
| `*_admin` methods in retained DID output | 6 |
| Root-only exported `canic_*` endpoint families in retained DID output | 32 |
| Non-root shared `canic_*` endpoint families in retained DID output | 13 |
| Non-root hub-specific `canic_*` endpoint families in retained DID output | 3 |

## Wire Surface Inventory

| Surface | Current Count |
| --- | ---: |
| `canic-core/src/protocol.rs` constants | 30 |
| `canic/src/protocol.rs` facade-only constants | 24 |
| `dto::rpc::Request` variants | 6 |
| `dto::rpc::Response` variants | 6 |
| `dto::rpc::RequestFamily` variants | 6 |
| `dto::capability::CapabilityProof` variants | 3 |
| `dto::capability::CapabilityService` variants | 1 |

## Bundling vs Usage Alignment

| Endpoint Family | Roles Exposing It | Roles Requiring It | Bundling Mode | Assessment |
| --- | --- | --- | --- | --- |
| `canic_response_capability_v1` | all retained public roles | root + non-root capability/cycles receiver path | `global` | aligned |
| `canic_memory_ledger` | all retained public roles | controller diagnostic/recovery path | `global` | aligned, guarded |
| `canic_metrics` | all retained public roles | default facade metrics policy | `global` | aligned |
| `canic_sync_state` / `canic_sync_topology` | retained non-root roles | parent-owned topology/state cascade targets | `non-root-only` | aligned |
| root auth request family | `root` | delegated-token and internal proof issuance | `root-only` | aligned |
| root WasmStore publication/operator family | `root` | root publication and bootstrap recovery | `root-only` | aligned |
| local `wasm_store` family | filtered from current fleet roster | canonical store canister only | `role-scoped` | not assessed in retained roster |
| `canic_delegation_set_*` proof-install family | none | none | `retired` | aligned |

## Surface Utilization

| Endpoint Family | Defined | Exposed | Used | Class | Evidence |
| --- | --- | --- | --- | --- | --- |
| `canic_response_capability_v1` | yes | yes | yes | active | `shared.rs`, `ops/rpc`, retained DID output |
| `canic_memory_ledger` | yes | yes | yes | active | `shared.rs`, `protocol_surface.rs`, retained DID output |
| `canic_metrics` | yes | yes | yes | active | `shared.rs`, metrics API, retained DID output |
| `canic_sync_state` / `canic_sync_topology` | yes | yes | yes | active | `nonroot.rs`, `ops/cascade.rs`, retained DID output |
| root auth request family | yes | yes | yes | active | `root.rs`, `api/auth/root_client.rs`, root DID output |
| root WasmStore publication/operator family | yes | yes | yes | active | `root.rs`, `canic-host::release_set`, root DID output |
| `canic_delegation_set_*` proof-install family | no | no | no | retired | no current code or DID matches |

No dead globally exposed endpoint families were detected in the retained roster.

## DID Surface Growth

| Canister | Total Methods | `canic_*` | Non-`canic` | Notes |
| --- | ---: | ---: | ---: | --- |
| `app` | 14 | 13 | 1 | default retained leaf baseline |
| `scale_replica` | 15 | 13 | 2 | replica role, same Canic surface as `app` |
| `user_shard` | 16 | 13 | 3 | shard role, same Canic surface as `app` |
| `scale_hub` | 17 | 14 | 3 | adds `canic_scaling_registry` |
| `user_hub` | 18 | 15 | 3 | adds sharding registry and partition-key queries |
| `root` | 33 | 32 | 1 | root control-plane outlier |

Shared `canic_*` methods present on every retained public role:

- `canic_bootstrap_status`
- `canic_canister_children`
- `canic_cycle_balance`
- `canic_cycle_topups`
- `canic_cycle_tracker`
- `canic_log`
- `canic_memory_ledger`
- `canic_metadata`
- `canic_metrics`
- `canic_ready`
- `canic_response_capability_v1`

Shared `canic_*` methods present on every retained non-root role:

- `canic_sync_state`
- `canic_sync_topology`

Outliers:

- `root` exceeds the retained leaf total-method baseline by more than 20% and
  exceeds the leaf `canic_*` baseline by more than `5`.
- `user_hub` exceeds the retained leaf total-method baseline by more than 20%,
  but its extra surface is role-specific sharding registry read surface.

## Surface Growth Attribution

| Surface Family | Current Count | Previous | Delta | Bundling Mode | Status | Risk |
| --- | ---: | ---: | ---: | --- | --- | --- |
| default retained leaf `canic_*` baseline (`app`) | 13 | 12 | +1 | `global` | GROWING | Low |
| default memory ledger diagnostic | 6 retained roles | N/A | N/A | `global` guarded query | INTENTIONAL | Low |
| root auth request family | 3 | 2 | +1 | `root-only` | GROWING | Medium |
| retained root WasmStore operator/read family | 4 | 4 | 0 | `root-only` | STABLE | Low |
| retired delegation proof-install endpoints | 0 | 0 | 0 | `retired` | STABLE | Positive |
| local wasm-store surface | filtered | 18 | N/A | `role-scoped` | NOT ASSESSED | N/A |

## Structural Hotspots

| File / Module | Surface Driver | Evidence | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic/src/macros/endpoints/**` | shared macro fan-out | `26` macro families, `52` endpoint definitions across `8` files | High |
| `crates/canic/src/macros/endpoints/bundles.rs` | default bundle composition | shared bundle emits lifecycle, memory ledger, discovery, observability, metrics, cycle tracker, auth attestation, and topology views | Medium |
| `crates/canic-core/src/protocol.rs` | orchestration endpoint constants | `30` constants after auth/internal proof expansion | Medium |
| `crates/canic/src/protocol.rs` | public facade endpoint constants | `24` facade-only constants plus core re-exports | Medium |
| `.icp/local/canisters/root/root.did` | root control-plane concentration | `32` exported `canic_*` methods | Medium |

## Hub Module Pressure

| Module | Import / Surface Driver | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic/src/macros/endpoints/bundles.rs` | global/root/non-root/wasm-store endpoint composition | 4 | 3 | 7 |
| `crates/canic/src/macros/endpoints/shared.rs` | shared lifecycle, diagnostics, observability, metrics, auth receiver | 5 | 3 | 7 |
| `crates/canic/src/macros/endpoints/root.rs` | root admin, auth, and WasmStore operator surface | 4 | 3 | 7 |
| `crates/canic-core/src/protocol.rs` | protocol constants and protected WasmStore helpers | 3 | 2 | 6 |
| `crates/canic/src/protocol.rs` | public facade endpoint name table | 4 | 3 | 6 |

## Global Amplification Factor

| Surface Change | Affected Canisters | GAF | Risk |
| --- | ---: | ---: | --- |
| shared lifecycle/diagnostic/metrics/runtime baseline | 6 | 6 | Medium |
| `canic_response_capability_v1` global receiver family | 6 | 6 | Medium |
| `canic_memory_ledger` default diagnostic | 6 | 6 | Medium, mitigated by controller gate |
| `canic_metrics` default feature surface | 6 | 6 | Medium |
| `canic_sync_state` / `canic_sync_topology` non-root cascade family | 5 | 5 | Medium |
| root auth request family | 1 | 1 | Low |
| root WasmStore publication/operator family | 1 | 1 | Low |

## Compatibility Signals

| Signal | Status | Evidence | Risk |
| --- | --- | --- | --- |
| RPC request/response family grew by one variant | compatible but growing | `Request`, `Response`, and `RequestFamily` are `6` variants | Medium |
| `canic-core::protocol` constants grew | compatible but growing | `30` constants versus prior `25` | Medium |
| Public facade protocol table is wider than core table | compatible but review-sensitive | `crates/canic/src/protocol.rs` owns `24` facade-only endpoint constants | Medium |
| Endpoint macro source moved to directory module | methodology change | `endpoints.rs` no longer exists; current source is `endpoints/**` | Low |
| Retired proof-install endpoints remain absent | stable | no `canic_delegation_set_*` matches | Positive |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| global diagnostic surface | `shared.rs` + retained DIDs | `canic_memory_ledger` appears on all `6` retained roles | Medium |
| root auth endpoint growth | `root.rs`, `dto/rpc.rs`, root DID | internal invocation proof adds a third root auth request endpoint and sixth RPC family | Medium |
| public/core protocol split | `canic-core/src/protocol.rs`, `canic/src/protocol.rs` | core constants plus `24` facade-only constants require both scans | Medium |
| stale local DID artifacts | `.icp/local/canisters/**` | `minimal`, `scale`, `test`, and `wasm_store` remain present outside the retained roster; template now requires roster-derived counts | Low |
| template path drift | recurring audit definition | template referenced `crates/canic/src/macros/endpoints.rs` for macro scans; corrected after this run | Low |

## Endpoint / RPC Alignment

| Check | Result | Evidence |
| --- | --- | --- |
| RPC family growth has endpoint coverage | PASS | root DID exposes `canic_request_internal_invocation_proof`; `dto::rpc` has matching request/response/family variants. |
| Endpoint growth without RPC mapping is intentional | PASS | `canic_memory_ledger`, metrics, topology, and diagnostic endpoints are direct query/update surfaces rather than RPC capability requests. |
| Retired RPC/endpoint proof-install surface absent | PASS | no `canic_delegation_set_*` code or DID matches. |
| WasmStore protocol helpers align with root/operator endpoints | PASS | root DID exposes operator methods; filtered `wasm_store` artifact still contains local store methods but is outside retained fleet roster. |

## Dependency Fan-In Pressure

| Module / Type | Referencing Files | Referencing Subsystems | Pressure | Notes |
| --- | ---: | --- | --- | --- |
| `dto::capability` | 6+ | api, ops, workflow, tests, macros | Medium | Core envelope/proof shape remains cross-cutting. |
| `dto::rpc` | 8+ | api, ops, workflow, tests, macros | Medium | Request/response variants grew to `6`; call sites remain concentrated in RPC seams. |
| `canic_core::protocol` | 20+ | core, canic facade, control-plane, host, CLI, tests | High | Endpoint name table is intentionally shared across operator/runtime callers. |
| `crates/canic/src/macros/endpoints/**` | 3 production composition sites plus tests | macros, start, tests | Medium | Directory split improves scanability, but bundle composition remains high fan-out. |
| `canic::protocol` facade constants | 10+ | tests, testkit, downstream facade consumers | Medium | Wider facade table must be included in future capability-surface scans. |

## Deterministic Risk Score

Risk Score: `3 / 10`.

Score contributions:

- `+1` because retained DID outliers exist (`root`, and role-specific
  `user_hub` by total method count).
- `+1` because DTO/protocol fan-out spans three or more subsystems.
- `+1` because global endpoint families have `GAF >= 5`, even though the
  confirmed global families are intentional and active.

No score was added for dead global endpoints, capability proof growth, or
endpoint count growth above 10%; none were confirmed in this run.

## Verification Readout

| Check | Result | Notes |
| --- | --- | --- |
| Current fleet role list | PASS | `scripts/ci/list-config-canisters.sh --config fleets/test/canic.toml --ci-order` returned `app`, `user_hub`, `user_shard`, `scale_hub`, `scale_replica`, `root`. |
| Artifact refresh | PASS | `target/debug/canic build test <role>` succeeded for all retained public roles. |
| Endpoint macro inventory scans | PASS | Adapted to `crates/canic/src/macros/endpoints/**`; captured `26` macro families and `52` endpoint definitions. |
| DID roster scans | PASS | Retained current fleet roster only; stale local artifacts were filtered explicitly. |
| Admin/root-only scans | PASS | `6` `*_admin` methods found, all on `root`. |
| Env/memory registry and retired proof-install scans | PASS | `canic_env`, `canic_memory_registry`, and `canic_delegation_set_*` absent from retained roster. |
| Wire/DTO scans | PASS | `Request`, `Response`, and `RequestFamily` have `6` variants; `CapabilityProof` remains `3`; `CapabilityService` remains `1`. |
| Workspace clippy | PASS | `cargo clippy --workspace --all-targets --all-features -- -D warnings` completed in `1m 32s`. |

## Follow-up Actions

1. Completed after this run: update
   `docs/audits/recurring/system/capability-surface.md` so all macro
   scan examples use `crates/canic/src/macros/endpoints/**` instead of the
   removed `crates/canic/src/macros/endpoints.rs` file.
2. Completed after this run: require future capability-surface DID scans to be
   driven by the selected fleet role list, with stale local `.icp` artifacts
   listed as filtered evidence.
3. Keep `canic_memory_ledger` documented and tested as a controller-gated
   default diagnostic whenever shared runtime bundles change.
4. Completed after this run: include both `canic-core/src/protocol.rs` and
   `canic/src/protocol.rs` in future wire-surface scans so the core/facade
   split does not hide public endpoint constants.

## Conclusion

The current retained public fleet surface is stable enough for a low-moderate
`3 / 10` risk readout. The main improvement from running the audit is not a code
fix; it is audit-method hygiene: future runs need to follow the current endpoint
directory module, current fleet roster, and core/facade protocol split.
