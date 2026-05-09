# Capability Surface Audit - 2026-05-09

## Report Preamble

- Scope: `crates/canic/src/macros/endpoints.rs`,
  `crates/canic/src/macros/start.rs`, `crates/canic-core/src/protocol.rs`,
  `crates/canic-core/src/dto/capability/**`,
  `crates/canic-core/src/dto/rpc.rs`, `crates/canic-core/src/api/rpc/**`,
  generated `.did` files under `.icp/local/canisters/**`
- Compared baseline report path:
  `docs/audits/reports/2026-04/2026-04-05/capability-surface.md`
- Code snapshot identifier: `c0722e74`
- Method tag/version: `Current Method`
- Comparability status: `partially comparable`. Macro and wire counts are
  directly comparable. Generated DID output moved from `.dfx` to `.icp`, and
  this run refreshed the public local roster before counting.
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-09T14:52:00Z`
- Branch: `main`
- Worktree: `dirty`

## Method Notes

- Refreshed generated local DID artifacts with:
  - `target/debug/canic build minimal`
  - `target/debug/canic build app`
  - `target/debug/canic build scale`
  - `target/debug/canic build scale_hub`
  - `target/debug/canic build user_hub`
  - `target/debug/canic build user_shard`
  - `target/debug/canic build root`
  - `target/debug/canic build wasm_store`
- Excluded the internal `test` DID file from consumer-facing surface counts.
- Counted public roster roles: `app`, `minimal`, `scale`, `scale_hub`,
  `user_hub`, `user_shard`, `root`, and `wasm_store`.
- Treated `canic_metrics` as intentional default surface because
  `crates/canic/README.md` documents the default `metrics` feature as exporting
  `canic_metrics` unless the facade dependency opts out.

## Executive Summary

- Risk Score: `3 / 10`
- Delta summary: total exported `canic_*` methods across the refreshed public
  local roster changed `125 -> 129` (`+4`, `+3.20%`). The default leaf baseline
  changed `11 -> 12` because `canic_metrics` is now explicitly default-on.
- Largest growth contributor: default-on `canic_metrics` across ordinary
  canisters.
- Over-bundled families: `none`. `canic_metrics` is globally available through
  an explicit default feature, while env/memory observability remains absent
  from refreshed ordinary builds.
- Follow-up required: `no`.

## Hard Surface Violations

| Hard Check | Result | Evidence |
| --- | --- | --- |
| Root-only admin endpoints stay root-only | PASS | `10` `*_admin` methods appear only in `.icp/local/canisters/root/root.did`. |
| Shared parent/cycles receiver exists where expected | PASS | `canic_response_capability_v1` appears on all refreshed public roster DID files. |
| Root-only wasm-store operator read surface stays root-only | PASS | `canic_wasm_store_overview`, publication status, bootstrap debug, and admin methods appear only on root. |
| Local wasm-store operations stay wasm-store scoped | PASS | `canic_wasm_store_catalog`, chunk, prepare, publish, status, and GC methods appear only on `wasm_store`. |
| Internal env/memory observability is absent from ordinary default builds | PASS | Refreshed public roster DID files do not expose `canic_env` or `canic_memory_registry`. |
| Default metrics surface is intentional | PASS | `canic_metrics` appears on ordinary canisters and root; `crates/canic/README.md` documents the default `metrics` feature. |
| Retired delegation proof-install endpoints are absent | PASS | No `canic_delegation_set_*` methods or protocol constants remain in current code or refreshed DID files. |
| Protocol constant removals are accounted for | PASS | `CANIC_DELEGATION_SET_SIGNER_PROOF` and `CANIC_DELEGATION_SET_VERIFIER_PROOF` are gone, matching the self-contained delegated-token hard cut documented in `docs/changelog/0.29.md`. |

## Baseline Delta Summary

| Category | Previous | Current | Delta | % Change |
| --- | ---: | ---: | ---: | ---: |
| Endpoint macro families | 25 | 26 | 1 | 4.00% |
| `fn canic_*` endpoint definitions in macro source | 60 | 56 | -4 | -6.67% |
| Protocol constants | 27 | 25 | -2 | -7.41% |
| RPC request variants | 5 | 5 | 0 | 0.00% |
| RPC response variants | 5 | 5 | 0 | 0.00% |
| Capability proof variants | 3 | 3 | 0 | 0.00% |
| Capability service variants | 1 | 1 | 0 | 0.00% |
| Default leaf `canic_*` baseline (`minimal`) | 11 | 12 | 1 | 9.09% |

## Endpoint Bundle Inventory

| Metric | Current Count |
| --- | ---: |
| Endpoint macro families (`emit` + `bundle`) | 26 |
| `fn canic_*` definitions in `crates/canic/src/macros/endpoints.rs` | 56 |
| Internal endpoint definitions in macro source | 23 |
| Controller-gated endpoint definitions in macro source | 20 |
| Root-gated endpoint definitions in macro source | 10 |
| Parent-gated endpoint definitions in macro source | 2 |
| Registered-subnet-gated endpoint definitions in macro source | 4 |
| `*_admin` methods in refreshed DID output | 10 |
| Root-only exported `canic_*` endpoint families in refreshed DID output | 24 |
| Non-root-only exported `canic_*` endpoint families in refreshed DID output | 18 |
| Globally bundled `canic_*` endpoint families in refreshed DID output | 7 |

## Wire Surface Inventory

| Surface | Current Count |
| --- | ---: |
| `canic-core/src/protocol.rs` constants | 25 |
| `dto::rpc::Request` variants | 5 |
| `dto::rpc::Response` variants | 5 |
| `dto::rpc::RequestFamily` variants | 5 |
| `dto::capability::CapabilityProof` variants | 3 |
| `dto::capability::CapabilityService` variants | 1 |

## Bundling vs Usage Alignment

| Endpoint Family | Roles Exposing It | Roles Requiring It | Bundling Mode | Assessment |
| --- | --- | --- | --- | --- |
| `canic_response_capability_v1` | all refreshed public roster roles | root + non-root capability/cycles receiver path | `global` | aligned |
| `canic_metrics` | `app`, `minimal`, `scale`, `scale_hub`, `user_hub`, `user_shard`, `root` | ordinary facade users by default-feature policy | `global` | aligned |
| `canic_sync_state` / `canic_sync_topology` | all non-root public roster roles | parent-owned topology/state cascade targets | `non-root-only` | aligned |
| `canic_wasm_store_overview` / publication status family | `root` | operator + reconcile path on root only | `root-only` | aligned |
| local `wasm_store` query/update family | `wasm_store` | subnet-local template/store operations only | `role-scoped` | aligned |
| `canic_delegation_set_*` proof-install family | none | none in current self-contained delegated-token flow | `retired` | aligned |

## Surface Utilization

| Endpoint Family | Defined | Exposed | Used | Class | Evidence |
| --- | --- | --- | --- | --- | --- |
| `canic_response_capability_v1` | yes | yes | yes | active | `protocol.rs`, `api/rpc`, refreshed DID output |
| `canic_metrics` | yes | yes | yes | active | `crates/canic/README.md`, `workflow/metrics/query.rs`, instruction audit support |
| `canic_sync_state` / `canic_sync_topology` | yes | yes | yes | active | `api/cascade.rs`, `ops/cascade.rs`, refreshed DID output |
| root wasm-store publication/operator family | yes | yes | yes | active | root DID output, root wasm-store reconcile/install flows |
| local wasm-store family | yes | yes | yes | active | wasm_store DID output, template/store APIs |
| `canic_delegation_set_*` proof-install family | no | no | no | retired | no current code or DID matches |

No dead endpoint families were detected in the audited set.

## DID Surface Growth

| Canister | Total Methods | `canic_*` | Non-`canic` | Notes |
| --- | ---: | ---: | ---: | --- |
| `app` | 74 | 12 | 62 | default leaf with `canic_metrics` |
| `minimal` | 74 | 12 | 62 | default leaf baseline with `canic_metrics` |
| `scale` | 75 | 12 | 63 | default replica with `canic_metrics` |
| `scale_hub` | 80 | 13 | 67 | scaling registry delta |
| `user_hub` | 84 | 14 | 70 | sharding registry and partition-key delta |
| `user_shard` | 116 | 12 | 104 | no retired delegation proof-install endpoints |
| `root` | 329 | 36 | 293 | root control-plane outlier |
| `wasm_store` | 108 | 18 | 90 | local store contract, no `canic_metrics` |

Shared `canic_*` methods present on all refreshed public roster DID files:

- `canic_bootstrap_status`
- `canic_canister_cycle_balance`
- `canic_canister_version`
- `canic_ready`
- `canic_response_capability_v1`
- `canic_standards`

Shared `canic_*` methods present on ordinary non-store canisters:

- `canic_metrics`

Shared `canic_*` methods present on all non-root refreshed public roster DID
files:

- `canic_sync_state`
- `canic_sync_topology`

## Surface Growth Attribution

| Surface Family | Current Count | Previous | Delta | Bundling Mode | Status | Risk |
| --- | ---: | ---: | ---: | --- | --- | --- |
| default shared leaf `canic_*` baseline (`minimal`) | 12 | 11 | +1 | `global` | GROWING | Low |
| default metrics query surface on ordinary/root canisters | 7 | 0 | +7 | `global` default feature | INTENTIONAL | Low |
| retired delegation proof-install endpoints | 0 | 2 | -2 | `retired` | REMOVED | Positive |
| core protocol constants | 25 | 27 | -2 | wire authority | SHRINKING | Positive |
| root-only admin methods | 10 | 11 | -1 | `root-only` | SHRINKING | Low |
| local wasm-store surface | 18 | 18 | 0 | `role-scoped` | STABLE | Low |

## Structural Hotspots

| File / Module | Surface Driver | Evidence | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic/src/macros/endpoints.rs` | shared macro fan-out | `26` endpoint macro families, `56` `fn canic_*` definitions | High |
| `crates/canic-core/src/protocol.rs` | wire constant authority | `25` constants after proof-install constant removal | Medium |
| `.icp/local/canisters/root/root.did` | root control-plane concentration | `36` exported `canic_*` methods, `329` total methods | Medium |
| `.icp/local/canisters/wasm_store/wasm_store.did` | dedicated local store contract | `18` role-scoped `canic_*` methods | Low |
| `crates/canic/README.md` | default surface policy | documents default-on metrics feature | Low |

## Global Amplification Factor

| Surface Change | Affected Canisters | GAF | Risk |
| --- | ---: | ---: | --- |
| shared lifecycle/standards/capability receiver baseline | 8 | 8 | Medium |
| `canic_response_capability_v1` global receiver family | 8 | 8 | Medium |
| `canic_metrics` default feature surface | 7 | 7 | Low |
| `canic_sync_state` / `canic_sync_topology` family | 7 | 7 | Medium |
| retired proof-install family removal | 1 previous role | -1 | Positive |
| root-only WasmStore publication/operator family | 1 | 1 | Low |

## Verification Readout

| Check | Result | Notes |
| --- | --- | --- |
| `target/debug/canic build <role>` for public roster | PASS | Refreshed `app`, `minimal`, `scale`, `scale_hub`, `user_hub`, `user_shard`, `root`, and `wasm_store`. |
| Endpoint macro inventory scans | PASS | `26` macro families, `56` endpoint definitions. |
| DID roster scans | PASS | Refreshed public roster counted; internal `test` excluded. |
| Admin/root-only scans | PASS | Admin methods appear only on root. |
| Wasm-store role-scope scans | PASS | Local store methods appear only on `wasm_store`; operator publication methods appear only on root. |
| Env/memory observability scans | PASS | `canic_env` and `canic_memory_registry` absent from refreshed public roster. |
| Retired proof-install scans | PASS | No current code or DID matches for `canic_delegation_set_*`. |

## Conclusion

The current capability surface is stable and intentional after the ICP CLI hard
cut. The local generated surface now lives under `.icp`, ordinary env/memory
diagnostics are absent from refreshed default builds, and `canic_metrics` is
present because the facade crate documents metrics as a default feature. The
main review pressure remains the expected macro fan-out and root control-plane
surface, not accidental over-bundling across ordinary roles.
