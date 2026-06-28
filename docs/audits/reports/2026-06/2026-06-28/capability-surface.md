# Capability Surface Audit - 2026-06-28

## Report Preamble

- Definition path: `docs/audits/recurring/system/capability-surface.md`
- Scope: endpoint macro bundles, generated retained-fleet DID service surface,
  core and facade protocol constants, RPC/capability DTO variants, root proof
  provisioning endpoints, root-managed delegation-renewal endpoints,
  issuer-local delegated-token endpoints, role-attestation endpoints, and
  protocol-surface guard tests.
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-19/capability-surface.md`
- Code snapshot identifier: `4a158a32`
- Method tag/version: `capability-surface-current`
- Comparability status: `comparable`
- Generated artifact environment: `.icp/local`
- Selected fleet config: `fleets/test/canic.toml`
- Retained public roster: `app`, `user_hub`, `user_shard`, `scale_hub`,
  `scale_replica`, `root`
- Filtered artifacts: `minimal`, `scale`, `test`, and `wasm_store` remained
  present under `.icp/local/canisters/**` but were not part of the retained
  fleet roster scan.
- Auditor: `codex`
- Run timestamp: `2026-06-28`
- Branch: `main`
- Worktree: `dirty` before report write with the in-progress 0.74.12
  changelog/visibility cleanup and prior 2026-06-28 audit reports.

## Audit Definition Maintenance

No audit definition change was made for this run. The report includes the
current root-managed delegation-renewal endpoint family as a concrete
capability-surface family in addition to the definition's minimum root proof
provisioning and issuer-local delegated-token families.

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
- Delta summary: endpoint macro families changed `27 -> 29`; generated
  `fn canic_*` endpoint definitions changed `58 -> 71`; core protocol export
  lines changed `37 -> 56`; facade-only protocol constants stayed `24`; RPC
  request/response/family/command variants stayed `4`; capability proof
  variants stayed `1`; retained app `canic_*` baseline stayed `12`; retained
  root `canic_*` methods changed `33 -> 39`.
- Largest growth contributor: root-managed delegation renewal in the retained
  public roster, adding six root-only `canic_*` service methods. Source and
  protocol inventory also grew from post-baseline blob-storage billing and
  refill surfaces, but those did not increase the retained shared DID surface.
- Over-bundled families: `none confirmed`.
- Follow-up required: `no`.

## Hard Surface Violations

| Hard Check | Result | Evidence |
| --- | --- | --- |
| Admin/controller-only endpoints stay on root | PASS | `6` `_admin` service methods were found, all in `root/root.did`. |
| Shared parent/cycles receiver exists where expected | PASS | `canic_response_capability_v1` appears on all six retained public roles. |
| Root proof provisioning endpoints stay root-only | PASS | `canic_upsert_root_issuer_policy`, `canic_prepare_delegation_proof_batch`, `canic_get_delegation_proof_batch`, and `canic_install_delegation_proof_batch` appeared only in `root/root.did`. |
| Root delegation-renewal endpoints stay root-only | PASS | The six renewal endpoints appeared only in `root/root.did`; protocol-surface tests pin their controller/provisioner gates. |
| Issuer-local delegated-token endpoints stay off root | PASS | `canic_prepare_delegated_token`, `canic_get_delegated_token`, `canic_install_active_delegation_proof`, and `canic_active_delegation_proof_status` appeared only in `user_shard/user_shard.did`. |
| Role-attestation prepare/get stays root-only | PASS | `canic_prepare_role_attestation` and `canic_get_role_attestation` appeared only in `root/root.did`. |
| Non-root cascade endpoints stay non-root-only | PASS | `canic_sync_state` and `canic_sync_topology` appeared on the retained non-root roles and not on root. |
| Default memory-ledger diagnostic stays gated | PASS | No retained DID exposed `canic_memory_ledger`; protocol-surface tests pin the cfg gate. |
| Retired single-proof/request endpoints stay absent | PASS | Source/DID scan found no production `canic_delegation_set_*`, `canic_request_delegation`, `canic_request_role_attestation`, or `canic_request_internal_invocation_proof` endpoints. |
| Protocol constant changes are covered | PASS | `cargo test --locked -p canic --test protocol_surface -- --nocapture` passed with 17 tests. |

## Baseline Delta Summary

| Category | Previous | Current | Delta | % Change |
| --- | ---: | ---: | ---: | ---: |
| Endpoint methods (`fn canic_*` in macro source) | 58 | 71 | +13 | +22.41% |
| Protocol constants / export lines (`canic-core::protocol`) | 37 | 56 | +19 | +51.35% |
| RPC request variants | 4 | 4 | 0 | 0.00% |
| RPC response variants | 4 | 4 | 0 | 0.00% |
| Capability proof variants | 1 | 1 | 0 | 0.00% |

Additional tracked inventory:

| Category | Previous | Current | Delta | % Change |
| --- | ---: | ---: | ---: | ---: |
| Endpoint macro families (`emit` + `bundle`) | 27 | 29 | +2 | +7.41% |
| Internal endpoint attributes in macro source | 22 | 25 | +3 | +13.64% |
| `canic::protocol` facade-only constants | 24 | 24 | 0 | 0.00% |
| RPC request-family variants | 4 | 4 | 0 | 0.00% |
| RPC root-command variants | 4 | 4 | 0 | 0.00% |
| Capability service variants | 1 | 1 | 0 | 0.00% |
| Default retained leaf `canic_*` baseline (`app`) | 12 | 12 | 0 | 0.00% |
| Retained root `canic_*` methods | 33 | 39 | +6 | +18.18% |

## Endpoint Bundle Inventory

| Metric | Current Count |
| --- | ---: |
| Endpoint macro families (`emit` + `bundle`) | 29 |
| `fn canic_*` definitions in `crates/canic/src/macros/endpoints/**` | 71 |
| Internal endpoint attributes in macro source | 25 |
| `*_admin` methods in retained DID output | 6 |
| Globally retained `canic_*` methods | 10 |
| Non-root shared `canic_*` methods | 12 |
| Root-only retained `canic_*` methods | 29 |
| Root-renewal retained `canic_*` methods | 6 |
| Issuer-local retained `canic_*` methods | 4 |

## Wire Surface Inventory

| Surface | Current Count |
| --- | ---: |
| `canic-core/src/protocol.rs` protocol export lines | 56 |
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
| root proof provisioning batch family | `root` | root controllers/provisioners | `root-only` | aligned |
| root-managed delegation renewal | `root` | root controllers and renewal provisioners | `root-only` | aligned, growing |
| role-attestation prepare/get | `root` | registered subnet canisters through root | `root-only` | aligned |
| issuer-local delegated-token prepare/get/install/status | `user_shard` | delegated-token issuer canisters | `cfg-gated` / `role-scoped` | aligned |
| root control-plane and WasmStore operator family | `root` | root controller/operator paths | `root-only` | aligned |
| blob-storage billing endpoints | filtered from retained roster | product/gateway billing canisters | `role-scoped` | not retained in this fleet scan |
| local `wasm_store` runtime family | filtered artifact only | canonical store canister | `role-scoped` | not assessed in retained roster |
| `canic_memory_ledger` | no retained roles | controller diagnostic when cfg-enabled | `cfg-gated` | aligned |
| retired request/delegation-set family | none | none | `retired` | aligned |

## Surface Utilization

| Endpoint Family | Defined | Exposed | Used | Class | Evidence |
| --- | --- | --- | --- | --- | --- |
| `canic_response_capability_v1` | yes | yes | yes | active | shared macro, RPC ops, retained DID output |
| shared runtime/metrics/log/cycles/discovery | yes | yes | yes | active | shared macros, retained DID output, protocol tests |
| non-root sync | yes | yes | yes | active | `nonroot.rs`, `ops/cascade.rs`, retained DID output |
| root proof provisioning batch | yes | yes | yes | active | `root.rs`, auth API/ops, root-suite tests, root DID output |
| root-managed delegation renewal | yes | yes | yes | active | `root.rs`, auth API/ops/workflow, protocol-surface tests, root-suite sharding renewal test |
| issuer-local delegated-token family | yes | yes | yes | active | `nonroot.rs`, testing PIC delegation helpers, user shard DID output |
| role-attestation prepare/get | yes | yes | yes | active | `root.rs`, role-attestation tests, root DID output |
| root WasmStore operator/publication family | yes | yes | yes | active | root macros, host/control-plane clients, root DID output |
| blob-storage billing endpoints | yes | no retained default | yes | role-scoped | billing endpoint macros and CLI tests; not in retained test fleet roster |
| `canic_memory_ledger` | yes | no retained default | yes | cfg-gated | protocol-surface tests pin cfg behavior |
| retired request/delegation-set family | no | no | no | retired | source/DID scan found no production matches |

No dead globally exposed endpoint families were detected in the retained
roster.

## DID Surface Growth

Service-method counts are scoped to each retained DID `service` block.

| Canister | Total Methods | `canic_*` | Non-`canic` | Notes |
| --- | ---: | ---: | ---: | --- |
| `app` | 13 | 12 | 1 | default retained leaf baseline |
| `scale_replica` | 14 | 12 | 2 | replica role, same Canic surface as `app` |
| `scale_hub` | 16 | 13 | 3 | adds `canic_scaling_registry` |
| `user_hub` | 17 | 14 | 3 | adds sharding registry and partition-key queries |
| `user_shard` | 18 | 16 | 2 | adds issuer-local delegated-token surface |
| `root` | 40 | 39 | 1 | root control-plane/provisioning/renewal outlier |

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
  more than `20%`, but the extra surface is role-specific product, sharding,
  scaling, or issuer surface rather than accidental global bundling.

Large DTO families:

- Capability envelope/proof DTOs appear in every retained DID through
  `canic_response_capability_v1`.
- Root delegation-renewal DTO families appear in the retained root DID only.
- Issuer delegated-token DTO families appear in the retained user shard DID
  only.

## Surface Growth Attribution

| Surface Family | Current Count | Previous | Delta | Bundling Mode | Status | Risk |
| --- | ---: | ---: | ---: | --- | --- | --- |
| default retained leaf `canic_*` baseline (`app`) | 12 | 12 | 0 | `global` / `non-root-only` | STABLE | Low |
| globally retained `canic_*` methods | 10 | 10 | 0 | `global` | STABLE | Medium |
| retained root `canic_*` methods | 39 | 33 | +6 | `root-only` | GROWING | Medium |
| root-managed delegation renewal | 6 root methods | 0 | +6 | `root-only` | GROWING | Medium |
| root proof provisioning batch | 4 root methods | 4 | 0 | `root-only` | STABLE | Low |
| issuer-local delegated-token family | 4 `user_shard` methods | 4 | 0 | `cfg-gated` / `role-scoped` | STABLE | Low |
| role-attestation prepare/get | 2 root methods | 2 | 0 | `root-only` | STABLE | Low |
| blob-storage billing endpoint macros | 3 source definitions | N/A | N/A | `role-scoped` | GROWING | Medium |
| RPC request/response/family variants | 4 each | 4 each | 0 | RPC DTO | STABLE | Low |
| capability proof variants | 1 | 1 | 0 | RPC DTO | STABLE | Low |
| retired request/delegation-set endpoints | 0 | 0 | 0 | `retired` | STABLE | Positive |

## Structural Hotspots

| File / Module | Surface Driver | Evidence | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic/src/macros/endpoints/**` | shared and role-scoped endpoint fan-out | `29` macro families and `71` `fn canic_*` definitions | High |
| `crates/canic/src/macros/endpoints/root.rs` | root admin, proof provisioning, renewal, role attestation, WasmStore operator surface | root DID exposes `39` `canic_*` service methods; root source defines `27` endpoint methods | High |
| `crates/canic/src/macros/endpoints/shared.rs` | global lifecycle, discovery, metrics, and capability receiver surface | `10` `canic_*` service methods are present on all retained roles | High |
| `crates/canic/src/macros/endpoints/nonroot.rs` | cascade and issuer-local delegated-token surface | non-root DIDs expose sync; user shard exposes `4` issuer-local auth methods | Medium |
| `crates/canic/src/macros/endpoints/blob_storage_billing.rs` | product/gateway billing endpoint source surface | `3` billing endpoint definitions, filtered out of retained test fleet | Medium |
| `crates/canic/src/macros/endpoints/bundles.rs` | shared/root/non-root/wasm-store bundle composition | controls global and role-scoped endpoint emission | High |
| `crates/canic-core/src/protocol.rs` | endpoint/protocol export table | `56` protocol export lines after root-renewal and billing/refill growth | High |
| `crates/canic-core/src/dto/auth.rs` | auth/provisioning DTO boundary | proof/renewal/delegated-token scan found `61` Rust files | Medium |
| `crates/canic/tests/protocol_surface.rs` | protocol-surface guard tests | `17` protocol-surface tests passed, including renewal, proof batch, active proof, role-attestation, blob-storage, and wasm-store guards | Medium |

## Hub Module Pressure

| Module | Import / Surface Driver | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic/src/macros/endpoints/bundles.rs` | global/root/non-root/wasm-store endpoint composition | 4 | 3 | 7 |
| `crates/canic/src/macros/endpoints/root.rs` | root admin/provisioning/renewal/attestation/WasmStore surface | 6 | 4 | 8 |
| `crates/canic/src/macros/endpoints/shared.rs` | lifecycle/discovery/observability/capability receiver fan-out | 4 | 3 | 7 |
| `crates/canic/src/macros/endpoints/nonroot.rs` | cascade and issuer-local delegated-token surface | 3 | 3 | 6 |
| `crates/canic-core/src/protocol.rs` | endpoint constants used across runtime, tests, host, CLI, control-plane | 6 | 4 | 8 |
| `crates/canic-core/src/dto/auth.rs` | root proof, renewal, delegated-token, active proof, role-attestation DTOs | 6 | 4 | 7 |
| `crates/canic-core/src/dto/rpc.rs` | root capability request/response DTOs | 4 | 4 | 6 |

## Global Amplification Factor

| Surface Change | Affected Canisters | GAF | Risk |
| --- | ---: | ---: | --- |
| shared runtime/metrics/log/cycles/discovery baseline | 6 | 6 | Medium |
| `canic_response_capability_v1` global receiver family | 6 | 6 | Medium |
| `canic_metrics` default feature surface | 6 | 6 | Medium |
| `canic_sync_state` / `canic_sync_topology` non-root cascade family | 5 | 5 | Medium |
| root-managed delegation-renewal family | 1 | 1 | Low |
| root proof provisioning batch family | 1 | 1 | Low |
| issuer-local delegated-token family in current retained roster | 1 | 1 | Low |
| root role-attestation prepare/get | 1 | 1 | Low |
| root WasmStore publication/operator family | 1 | 1 | Low |
| blob-storage billing endpoints in current retained roster | 0 | 0 | Low |
| `canic_memory_ledger` retained default surface | 0 | 0 | Low |

## Compatibility Signals

| Signal | Status | Evidence | Risk |
| --- | --- | --- | --- |
| Endpoint definitions grew by more than 10% | additive but growing | `58 -> 71`, with retained DID growth concentrated on root renewal | Medium |
| Retained root service surface grew | additive but growing | root DID `canic_*` methods changed `33 -> 39` | Medium |
| Core protocol export table grew | additive but growing | `37 -> 56` export lines | Medium |
| Public facade protocol table is stable | stable | facade-only constants stayed `24` | Low |
| Root capability RPC variants are stable | stable | request/response/family/root-command variants stayed `4` | Low |
| Capability proof variants are stable | stable | `CapabilityProof` stayed `1` | Low |
| Default shared DID surface is stable | stable | `app` remained `12` `canic_*` methods; global shared methods remained `10` | Low |
| Retired request/single-proof endpoints remain absent | stable | no production source/DID matches except protocol-surface absence assertions | Positive |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| root renewal surface growth | `root.rs`, root DID, protocol constants | `6` root-only renewal endpoints | Medium |
| admin surface clustering | `root.rs`, root DID | retained root now exposes `39` `canic_*` methods, including `6` `_admin` methods | High |
| protocol table fan-out | `canic-core/src/protocol.rs` | protocol reference scan found `32` Rust files | High |
| shared DTO fan-out | `dto/auth.rs` | proof/renewal/delegated-token scan found `61` Rust files | Medium |
| global endpoint family amplification | shared endpoint bundle | `10` `canic_*` methods on all retained roles | Medium |
| blob-storage billing source growth | `blob_storage_billing.rs`, protocol constants | `3` endpoint definitions and billing protocol constants, filtered from retained roster | Medium |
| latent global surface | retained DID roster | none detected | Low |

## Endpoint / RPC Alignment

| Alignment Check | Result | Evidence |
| --- | --- | --- |
| RPC capability growth without endpoint usage | PASS | RPC request/response/family/root-command variants stayed `4`; `canic_response_capability_v1` remains active on retained roles. |
| Endpoint growth without RPC mapping | ACCEPTED | Root-managed renewal added direct auth/provisioning endpoints rather than `dto::rpc` variants. This is intentional direct-call root surface, not unused RPC growth. |
| Root renewal direct-call coupling | WATCH | The renewal family is root-only and tested, but it expands direct endpoint/protocol surface outside the root capability RPC envelope. |
| Blob-storage billing direct-call surface | WATCH | Billing endpoint source exists but is filtered from the retained test fleet roster; keep role-scoped when enabled. |
| Retired endpoint/RPC mismatch | PASS | Retired request/delegation-set endpoints remain absent. |

## Dependency Fan-In Pressure

| Module / Type | Referencing Files | Referencing Subsystems | Pressure | Notes |
| --- | ---: | --- | --- | --- |
| `dto::capability` | 16 | API, ops, workflow, tests, test canisters | Medium | Passive capability envelopes/proof variants remain stable. |
| `dto::rpc` | 27 | API, ops, workflow, tests | Medium | Variant count stable; handler surface remains a sequencing center. |
| `macros/endpoints/**` | 16 | macro expansion, build, tests, canister endpoint source | Medium | Bundle composition controls global/root/non-root fan-out. |
| `canic-core/src/protocol.rs` | 32 | runtime, tests, CLI, host, control-plane, testkit | High | Protocol export growth is the broadest source-level fan-out. |
| `canic/src/protocol.rs` | 32 aggregate protocol refs | facade/tests/host-facing callers | Medium | Facade-only constants stayed stable at `24`. |
| `dto/auth` proof/renewal/delegated-token family | 61 | API, ops, workflow, storage, tests, endpoint macros | Medium | Broad but expected auth/provisioning DTO spread. |

## Deterministic Risk Score

Risk Score: 4 / 10.

Score contributions:

- `+2`: endpoint count delta exceeded `10%` (`58 -> 71`, `+22.41%`).
- `+0`: no DTO enum grew by more than `3` variants.
- `+0`: no global bundle added a new endpoint family; retained global surface
  stayed at `10` shared `canic_*` methods.
- `+0`: no unused latent or dead endpoint was found in a global bundle.
- `+1`: DID outliers were detected (`root`, plus role-specific
  `user_hub`, `user_shard`, and `scale_hub` total-method outliers).
- `+1`: DTO fan-out spans at least three subsystems.

The score remains moderate because the surface grew substantially, but the
growth is attributable and role-scoped. No hard placement violation or latent
global endpoint family was found.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `scripts/ci/list-config-canisters.sh --config fleets/test/canic.toml --ci-order` | PASS | Retained roster resolved to `app`, `user_hub`, `user_shard`, `scale_hub`, `scale_replica`, `root`. |
| `target/debug/canic build test app` | PASS | Refreshed retained app artifact. |
| `target/debug/canic build test user_hub` | PASS | Refreshed retained user hub artifact. |
| `target/debug/canic build test user_shard` | PASS | Refreshed retained user shard artifact. |
| `target/debug/canic build test scale_hub` | PASS | Refreshed retained scale hub artifact. |
| `target/debug/canic build test scale_replica` | PASS | Refreshed retained scale replica artifact. |
| `target/debug/canic build test root` | PASS | Refreshed retained root artifact. |
| `rg -n '^macro_rules!' crates/canic/src/macros/endpoints -g '*.rs'` | PASS | Found `29` endpoint macro families. |
| `rg -n 'fn canic_' crates/canic/src/macros/endpoints -g '*.rs'` | PASS | Found `71` endpoint definitions. |
| `rg -n '#\[.*internal' crates/canic/src/macros/endpoints -g '*.rs'` | PASS | Found `25` internal endpoint attributes. |
| `rg -n '^  canic_.*_admin :' .icp/local/canisters -g '*.did'` | PASS | Found `6` admin service methods, all under `root/root.did`. |
| `rg -n 'cfg\(canic_' crates/canic/src/macros/endpoints -g '*.rs'` | PASS | Found expected root, sharding/scaling, metrics, memory-ledger, and delegated-token issuer cfg gates. |
| `rg -n 'canic_response_capability_v1\|canic_upsert_root_issuer_policy\|canic_upsert_root_issuer_renewal_template\|canic_root_issuer_renewal_status\|canic_upsert_delegation_renewal_provisioner\|canic_delegation_renewal_provisioners\|canic_delegation_renewal_work\|canic_prepare_delegation_proof_batch\|canic_get_delegation_proof_batch\|canic_get_delegation_renewal_proof_batch\|canic_install_delegation_proof_batch\|canic_prepare_delegated_token\|canic_get_delegated_token\|canic_install_active_delegation_proof\|canic_active_delegation_proof_status\|canic_prepare_role_attestation\|canic_get_role_attestation' .icp/local/canisters/{app,user_hub,user_shard,scale_hub,scale_replica,root} -g '*.did'` | PASS | Placement matched expected global/root-only/user-shard-only families. |
| `rg -n 'canic_delegation_set_\|canic_request_delegation\|canic_request_role_attestation\|canic_request_internal_invocation_proof' crates/canic/src/macros/endpoints .icp/local/canisters/{app,user_hub,user_shard,scale_hub,scale_replica,root} -g '*.rs' -g '*.did'` | PASS | No matches, as expected. |
| `rg -n '^  canic_memory_ledger :' .icp/local/canisters/{app,user_hub,user_shard,scale_hub,scale_replica,root} -g '*.did'` | PASS | No retained DID exposes the cfg-gated diagnostic. |
| service-block DID method counts for retained roles | PASS | `app 13/12/1`, `user_hub 17/14/3`, `user_shard 18/16/2`, `scale_hub 16/13/3`, `scale_replica 14/12/2`, `root 40/39/1`. |
| `rg -l 'dto::capability\|RootCapabilityEnvelopeV1\|NonrootCyclesCapabilityEnvelopeV1\|CapabilityProof\|CapabilityService' crates/ -g '*.rs' \| wc -l` | PASS | Found `16` referencing files. |
| `rg -l 'dto::rpc\|CreateCanisterRequest\|UpgradeCanisterRequest\|RecycleCanisterRequest\|CyclesRequest\|RootCapabilityCommand\|RequestFamily' crates/ -g '*.rs' \| wc -l` | PASS | Found `27` referencing files. |
| `rg -l 'protocol::\|canic_core::protocol\|canic::protocol' crates/ -g '*.rs' \| wc -l` | PASS | Found `32` referencing files. |
| `rg -l 'RootDelegationProof\|DelegatedToken\|RoleAttestation\|RootIssuerRenewal\|DelegationRenewal' crates/ -g '*.rs' \| wc -l` | PASS | Found `61` referencing files. |
| `cargo test --locked -p canic --test protocol_surface -- --nocapture` | PASS | 17 protocol-surface tests passed. |

## Final Verdict

PASS. Capability surface grew materially since the 2026-06-19 baseline, but
the retained DID growth is attributable to root-only delegation renewal and the
global shared surface did not expand. No hard placement violation, retired
endpoint revival, or dead global endpoint family was found.

## Follow-up Actions

- No immediate remediation is required.
- Keep root-managed renewal direct-call endpoints root-only and covered by
  protocol-surface guards.
- Keep blob-storage billing endpoints role-scoped when enabled in future fleet
  rosters.
- Continue watching `canic-core/src/protocol.rs` fan-out before adding more
  protocol constants.
