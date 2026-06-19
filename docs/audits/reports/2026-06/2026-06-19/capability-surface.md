# Capability Surface Audit - 2026-06-19

## Report Preamble

- Definition path: `docs/audits/recurring/system/capability-surface.md`
- Scope: endpoint macro bundles, generated retained-fleet DID surface, core and
  facade protocol constants, RPC/capability DTO variants, root proof
  provisioning endpoints, issuer-local delegated-token endpoints, role
  attestation endpoints, and protocol-surface guard tests.
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-31/capability-surface.md`
- Code snapshot identifier: `16894709`
- Method tag/version: `capability-surface-current`
- Comparability status: `partially comparable`
- Generated artifact environment: `.icp/local`
- Retained public roster: `app`, `user_hub`, `user_shard`, `scale_hub`,
  `scale_replica`, `root`
- Filtered artifacts: `minimal`, `scale`, `test`, and `wasm_store` remained
  present under `.icp/local/canisters/**` but were not part of the current
  retained fleet roster scan.
- Auditor: `codex`
- Run timestamp: `2026-06-19`
- Branch: `main`
- Worktree: `dirty`

## Audit Definition Maintenance

The audit definition was updated before this run so its hard-check and
alignment scans include the current root proof provisioning endpoints,
issuer-local delegated-token endpoints, role-attestation endpoints, and the
retired `canic_delegation_set_*` compatibility guard. The previous definition
still emphasized older request/delegation search terms that would have missed
part of the current auth surface.

## Artifact Refresh Commands

| Command | Status | Output |
| --- | --- | --- |
| `scripts/ci/list-config-canisters.sh --config fleets/test/canic.toml --ci-order` | PASS | `app`, `user_hub`, `user_shard`, `scale_hub`, `scale_replica`, `root` |
| `target/debug/canic build test app` | PASS | `.icp/local/canisters/app/app.wasm.gz` |
| `target/debug/canic build test user_hub` | PASS | `.icp/local/canisters/user_hub/user_hub.wasm.gz` |
| `target/debug/canic build test user_shard` | PASS | `.icp/local/canisters/user_shard/user_shard.wasm.gz` |
| `target/debug/canic build test scale_hub` | PASS | `.icp/local/canisters/scale_hub/scale_hub.wasm.gz` |
| `target/debug/canic build test scale_replica` | PASS | `.icp/local/canisters/scale_replica/scale_replica.wasm.gz` |
| `target/debug/canic build test root` | PASS | `.icp/local/canisters/root/root.wasm.gz` |

## Executive Summary

- Risk Score: `4 / 10`
- Delta summary: endpoint macro families changed `26 -> 27`; generated
  endpoint definitions changed `52 -> 58`; core protocol constants changed
  `30 -> 37`; facade-only protocol constants stayed `24`; RPC
  request/response/family variants changed `6 -> 4`; capability proof variants
  changed `3 -> 1`; capability service variants stayed `1`; default retained
  leaf `canic_*` baseline changed `13 -> 12`.
- Largest growth contributor: root proof provisioning plus issuer-local
  delegated-token endpoint surfacing.
- Over-bundled families: `none confirmed`.
- Follow-up required: `no`.

## Hard Surface Violations

| Hard Check | Result | Evidence |
| --- | --- | --- |
| Admin/controller-only endpoints stay on root | PASS | `6` `_admin` methods were found, all in `root/root.did`. |
| Shared parent/cycles receiver exists where expected | PASS | `canic_response_capability_v1` appears on all retained public roles. |
| Root proof provisioning endpoints stay root-only | PASS | `canic_upsert_root_issuer_policy`, `canic_prepare_delegation_proof_batch`, `canic_get_delegation_proof_batch`, and `canic_install_delegation_proof_batch` appeared only in `root/root.did`; root macros are controller-gated. |
| Issuer-local delegated-token endpoints stay off root | PASS | `canic_prepare_delegated_token`, `canic_get_delegated_token`, `canic_install_active_delegation_proof`, and `canic_active_delegation_proof_status` appeared only in `user_shard/user_shard.did`. |
| Role-attestation prepare/get stays root-only | PASS | `canic_prepare_role_attestation` and `canic_get_role_attestation` appeared only in `root/root.did`. |
| Non-root cascade endpoints stay non-root-only | PASS | `canic_sync_state` and `canic_sync_topology` appeared on the retained non-root roles and not on root. |
| Default memory-ledger diagnostic stays gated | PASS | No retained DID exposed `canic_memory_ledger`; protocol-surface tests pin the cfg gate. |
| Retired single-proof/request endpoints stay absent | PASS | Source/DID scan found no production `canic_delegation_set_*`, `canic_request_delegation`, `canic_request_role_attestation`, or `canic_request_internal_invocation_proof` endpoints. |
| Protocol constant changes are covered | PASS | Root proof, issuer-local auth, role-attestation, and WasmStore protocol constants have protocol-surface guard coverage. |

## Baseline Delta Summary

| Category | Previous | Current | Delta | % Change |
| --- | ---: | ---: | ---: | ---: |
| Endpoint macro families | 26 | 27 | +1 | +3.85% |
| `fn canic_*` endpoint definitions in macro source | 52 | 58 | +6 | +11.54% |
| Internal endpoint attributes in macro source | N/A | 22 | N/A | N/A |
| `canic-core::protocol` constants | 30 | 37 | +7 | +23.33% |
| `canic::protocol` facade-only constants | 24 | 24 | 0 | 0.00% |
| RPC request variants | 6 | 4 | -2 | -33.33% |
| RPC response variants | 6 | 4 | -2 | -33.33% |
| RPC request-family variants | 6 | 4 | -2 | -33.33% |
| Capability proof variants | 3 | 1 | -2 | -66.67% |
| Capability service variants | 1 | 1 | 0 | 0.00% |
| Default retained leaf `canic_*` baseline (`app`) | 13 | 12 | -1 | -7.69% |
| Retained root `canic_*` methods | 32 | 33 | +1 | +3.13% |

## Endpoint Bundle Inventory

| Metric | Current Count |
| --- | ---: |
| Endpoint macro families (`emit` + `bundle`) | 27 |
| `fn canic_*` definitions in `crates/canic/src/macros/endpoints/**` | 58 |
| Internal endpoint attributes in macro source | 22 |
| `*_admin` methods in retained DID output | 6 |
| Globally retained `canic_*` methods | 10 |
| Non-root shared `canic_*` methods | 12 |
| Root-only retained `canic_*` methods | 23 |
| Issuer-local retained `canic_*` methods | 4 |

## Wire Surface Inventory

| Surface | Current Count |
| --- | ---: |
| `canic-core/src/protocol.rs` constants | 37 |
| `canic/src/protocol.rs` facade-only constants | 24 |
| `dto::rpc::Request` variants | 4 |
| `dto::rpc::Response` variants | 4 |
| `dto::rpc::RequestFamily` variants | 4 |
| `dto::rpc::RootCapabilityCommand` variants | 4 |
| `dto::capability::CapabilityProof` variants | 1 |
| `dto::capability::CapabilityService` variants | 1 |

## Bundling vs Usage Alignment

| Endpoint Family | Roles Exposing It | Roles Requiring It | Bundling Mode | Assessment |
| --- | --- | --- | --- | --- |
| `canic_response_capability_v1` | all retained roles | root + non-root capability/cycles receiver path | `global` | aligned |
| shared runtime/metrics/log/cycles/discovery | all retained roles | all retained Canic roles | `global` | aligned |
| `canic_sync_state` / `canic_sync_topology` | retained non-root roles | parent-owned topology/state cascade targets | `non-root-only` | aligned |
| root proof provisioning batch family | `root` | root controllers/provisioners | `root-only` | aligned, growing |
| role-attestation prepare/get | `root` | registered subnet canisters through root | `root-only` | aligned |
| issuer-local delegated-token prepare/get/install/status | `user_shard` | delegated-token issuer canisters | `cfg-gated` / `role-scoped` | aligned |
| root control-plane and WasmStore operator family | `root` | root controller/operator paths | `root-only` | aligned |
| local `wasm_store` runtime family | filtered artifact only | canonical store canister | `role-scoped` | not assessed in retained roster |
| `canic_memory_ledger` | no retained roles | controller diagnostic when cfg-enabled | `cfg-gated` | aligned, reduced from retained default |
| retired request/delegation-set family | none | none | `retired` | aligned |

## Surface Utilization

| Endpoint Family | Defined | Exposed | Used | Class | Evidence |
| --- | --- | --- | --- | --- | --- |
| `canic_response_capability_v1` | yes | yes | yes | active | shared macro, RPC ops, retained DID output |
| shared runtime/metrics/log/cycles/discovery | yes | yes | yes | active | shared macros, retained DID output, protocol tests |
| non-root sync | yes | yes | yes | active | `nonroot.rs`, `ops/cascade.rs`, retained DID output |
| root proof provisioning batch | yes | yes | yes | active | `root.rs`, auth API/ops, root-suite tests, root DID output |
| issuer-local delegated-token family | yes | yes | yes | active | `nonroot.rs`, testing PIC delegation helpers, user shard DID output |
| role-attestation prepare/get | yes | yes | yes | active | `root.rs`, role-attestation tests, root DID output |
| root WasmStore operator/publication family | yes | yes | yes | active | root macros, host/control-plane clients, root DID output |
| `canic_memory_ledger` | yes | no retained default | yes | cfg-gated | protocol-surface tests pin cfg behavior |
| retired request/delegation-set family | no | no | no | retired | source/DID scan found no production matches |

No dead globally exposed endpoint families were detected in the retained roster.

## DID Surface Growth

| Canister | Total Methods | `canic_*` | Non-`canic` | Notes |
| --- | ---: | ---: | ---: | --- |
| `app` | 13 | 12 | 1 | default retained leaf baseline |
| `scale_replica` | 14 | 12 | 2 | replica role, same Canic surface as `app` |
| `scale_hub` | 16 | 13 | 3 | adds `canic_scaling_registry` |
| `user_hub` | 17 | 14 | 3 | adds sharding registry and partition-key queries |
| `user_shard` | 18 | 16 | 2 | adds issuer-local delegated-token surface |
| `root` | 34 | 33 | 1 | root control-plane/provisioning outlier |

Shared `canic_*` methods present on every retained public role:

- `canic_bootstrap_status`
- `canic_canister_children`
- `canic_cycle_balance`
- `canic_cycle_topups`
- `canic_cycle_tracker`
- `canic_log`
- `canic_metadata`
- `canic_metrics`
- `canic_ready`
- `canic_response_capability_v1`

Shared `canic_*` methods present on every retained non-root role:

- `canic_sync_state`
- `canic_sync_topology`

Outliers:

- `root` exceeds the retained leaf method baseline and exceeds the leaf
  `canic_*` baseline by more than `5`.
- `user_hub`, `user_shard`, and `scale_hub` exceed the total-method baseline by
  more than `20%`, but the extra surface is role-specific product or issuer
  surface rather than accidental global bundling.

## Surface Growth Attribution

| Surface Family | Current Count | Previous | Delta | Bundling Mode | Status | Risk |
| --- | ---: | ---: | ---: | --- | --- | --- |
| default retained leaf `canic_*` baseline (`app`) | 12 | 13 | -1 | `global` / `non-root-only` | REDUCED | Low |
| default memory-ledger diagnostic | 0 retained roles | 6 retained roles | -6 | `cfg-gated` | REDUCED | Low |
| root proof provisioning batch | 4 root methods | N/A | N/A | `root-only` | GROWING | Medium |
| issuer-local delegated-token family | 4 `user_shard` methods | N/A | N/A | `cfg-gated` / `role-scoped` | GROWING | Medium |
| role-attestation prepare/get | 2 root methods | N/A | N/A | `root-only` | STABLE | Low |
| root control-plane/WasmStore operator surface | retained root only | retained root only | stable | `root-only` | STABLE | Low |
| RPC request/response/family variants | 4 each | 6 each | -2 each | RPC DTO | REDUCED | Positive |
| capability proof variants | 1 | 3 | -2 | RPC DTO | REDUCED | Positive |
| retired request/delegation-set endpoints | 0 | 0 | 0 | `retired` | STABLE | Positive |

## Structural Hotspots

| File / Module | Surface Driver | Evidence | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic/src/macros/endpoints/**` | shared endpoint fan-out | `27` macro families and `58` `fn canic_*` definitions | High |
| `crates/canic/src/macros/endpoints/root.rs` | root control, proof provisioning, role attestation, WasmStore operator surface | root DID exposes `33` `canic_*` methods | High |
| `crates/canic/src/macros/endpoints/nonroot.rs` | cascade and issuer-local delegated-token surface | user shard DID exposes `4` issuer-local auth methods | Medium |
| `crates/canic/src/macros/endpoints/bundles.rs` | shared/root/non-root/wasm-store bundle composition | controls global and role-scoped endpoint emission | High |
| `crates/canic-core/src/protocol.rs` | endpoint constant table | `37` constants, including root proof and issuer-local auth families | High |
| `crates/canic-core/src/dto/auth.rs` | auth/provisioning DTO boundary | root proof and delegated-token DTO scan found `40` files | Medium |
| `crates/canic/tests/protocol_surface.rs` | protocol-surface guard tests | 11 protocol-surface tests passed | Medium |

## Hub Module Pressure

| Module | Import / Surface Driver | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic/src/macros/endpoints/bundles.rs` | global/root/non-root/wasm-store endpoint composition | 4 | 3 | 7 |
| `crates/canic/src/macros/endpoints/root.rs` | root admin/provisioning/attestation/WasmStore surface | 5 | 3 | 8 |
| `crates/canic/src/macros/endpoints/nonroot.rs` | cascade and issuer-local delegated-token surface | 3 | 3 | 6 |
| `crates/canic-core/src/protocol.rs` | endpoint constants used across runtime, tests, host, CLI, control-plane | 6 | 4 | 8 |
| `crates/canic-core/src/dto/auth.rs` | root proof, delegated-token, active proof, role-attestation DTOs | 5 | 4 | 7 |
| `crates/canic-core/src/dto/rpc.rs` | root capability request/response DTOs | 4 | 4 | 6 |

## Global Amplification Factor

| Surface Change | Affected Canisters | GAF | Risk |
| --- | ---: | ---: | --- |
| shared runtime/metrics/log/cycles/discovery baseline | 6 | 6 | Medium |
| `canic_response_capability_v1` global receiver family | 6 | 6 | Medium |
| `canic_metrics` default feature surface | 6 | 6 | Medium |
| `canic_sync_state` / `canic_sync_topology` non-root cascade family | 5 | 5 | Medium |
| root proof provisioning batch family | 1 | 1 | Low |
| issuer-local delegated-token family in current retained roster | 1 | 1 | Low |
| root role-attestation prepare/get | 1 | 1 | Low |
| root WasmStore publication/operator family | 1 | 1 | Low |
| `canic_memory_ledger` retained default surface | 0 | 0 | Low |

## Compatibility Signals

| Signal | Status | Evidence | Risk |
| --- | --- | --- | --- |
| Endpoint definitions grew by more than 10% | additive but growing | `52 -> 58`, mostly root proof and issuer-local auth surface | Medium |
| Core protocol constants grew | additive but growing | `30 -> 37` constants | Medium |
| Public facade protocol table is stable | stable | facade-only constants stayed `24` | Low |
| Root capability RPC variants contracted | contractive, intentional | request/response/family variants changed `6 -> 4`; protocol tests and replay-policy guards cover current shape | Medium |
| Capability proof variants contracted | contractive, intentional | `CapabilityProof` changed `3 -> 1` | Medium |
| Default memory-ledger DID surface removed | compatible reduction | retained DIDs expose no `canic_memory_ledger`; protocol tests pin cfg gate | Low |
| Retired request/single-proof endpoints remain absent | stable | no production source/DID matches except protocol-surface absence assertions | Positive |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| root proof provisioning surface growth | `root.rs`, root DID, protocol constants | 4 root-only endpoints | Medium |
| issuer-local auth surface growth | `nonroot.rs`, user shard DID | 4 issuer-local endpoints on current issuer role | Medium |
| shared DTO fan-out | `dto/auth.rs` | root proof/delegated-token/role-attestation DTO scan found 40 files | Medium |
| protocol table fan-out | `canic-core/src/protocol.rs` | protocol constant scan found 82 files | High |
| global endpoint family amplification | shared endpoint bundle | 10 `canic_*` methods on all retained roles | Medium |
| latent global surface | retained DID roster | none detected | Low |

## Endpoint / RPC Alignment

| Check | Result | Evidence |
| --- | --- | --- |
| Root capability RPC surface has endpoint coverage | PASS | `canic_response_capability_v1` exists on all retained roles; RPC request/response variants are still exercised by root capability tests. |
| Root proof provisioning direct endpoints are intentional | PASS | Root proof batch prepare/get/install and issuer-policy upsert are root controller/provisioner endpoints, not RPC capability variants. |
| Issuer-local delegated-token direct endpoints are intentional | PASS | User shard exposes prepare/get/install/status; root does not expose issuer-local delegated-token endpoints. |
| Role-attestation endpoints align with root authority | PASS | Prepare/get role-attestation endpoints are root-only and protocol-surface tests pin them. |
| Endpoint growth without RPC mapping is documented | PASS | Auth provisioning and delegated-token issuance are direct API surfaces by design. |
| WasmStore protocol helpers align with root/store endpoints | PASS | Root operator methods and filtered local store methods remain separated by role. |
| Retired proof-install/request surface absent | PASS | No production source/DID matches for retired request/single-proof names. |

## Dependency Fan-In Pressure

| Module / Type | Referencing Files | Referencing Subsystems | Pressure | Notes |
| --- | ---: | --- | --- | --- |
| `dto::capability` and capability envelopes | 16 | api, ops, workflow, tests, macros | Medium | Core envelope shape remains cross-cutting but smaller. |
| `dto::rpc` request/response family | 33 | api, ops, workflow, replay policy, tests, macros | High | Variant count contracted but fan-in remains broad. |
| protocol constants | 82 | core, facade, host, CLI, control-plane, tests | High | Endpoint name table is intentionally shared. |
| endpoint macro emit/bundle surface | 13 | macro emitters, start composition, tests | Medium | Directory split remains scanable. |
| auth/provisioning DTO family | 40 | api, ops, workflow, storage, tests | Medium | Root proof provisioning adds broad DTO fan-in but DTOs remain passive. |

## Deterministic Risk Score

Risk Score: `4 / 10`.

Score contributions:

- `+2` because endpoint definition count grew by more than 10%
  (`52 -> 58`).
- `+1` because retained DID outliers exist (`root`, plus role-specific hub and
  issuer canisters by total method count).
- `+1` because DTO/protocol fan-out spans three or more subsystems.

No score was added for global bundle endpoint-family growth, unused global
endpoints, or DTO enum growth; none were confirmed in this run.

## Verification Readout

| Check | Result | Notes |
| --- | --- | --- |
| Current fleet role list | PASS | Roster command returned `app`, `user_hub`, `user_shard`, `scale_hub`, `scale_replica`, `root`. |
| Artifact refresh | PASS | `target/debug/canic build test <role>` succeeded for all retained public roles. |
| Endpoint macro inventory scans | PASS | `27` macro families and `58` `fn canic_*` definitions found. |
| DID roster scans | PASS | Retained current fleet roster only; stale local artifacts were filtered explicitly. |
| Admin/root-only scans | PASS | `6` `_admin` methods found, all on root. |
| Root proof provisioning scans | PASS | Batch prepare/get/install and issuer-policy upsert appeared only on root. |
| Issuer-local auth scans | PASS | Delegated-token prepare/get/install/status appeared only on user shard. |
| Memory ledger and retired endpoint scans | PASS | Retained DIDs expose no `canic_memory_ledger`; retired request/single-proof names were absent from production source/DIDs. |
| Wire/DTO scans | PASS | RPC variants contracted to `4`; capability proof/service variants are `1` each. |
| `cargo test --locked -p canic --test protocol_surface -- --nocapture` | PASS | 11 protocol-surface tests passed. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS | Completed successfully. |

The full workspace clippy command emitted root bootstrap artifact warnings from
build scripts; no lint failure was reported.

## Follow-up Actions

No follow-up actions required.
