# Change Friction Audit - 2026-06-28

## Report Preamble

- Scope: `crates/canic-core`, `crates/canic`, `crates/canic-cli`,
  `crates/canic-host`, `crates/canic-tests`, test canisters, active audit
  definitions, active design/changelog docs, and the recent `0.71` through
  `0.74` feature slices.
- Definition path: `docs/audits/recurring/system/change-friction.md`
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-19/change-friction.md`
- Code snapshot identifier: `b140a86c` plus dirty root-renewal split,
  blob-storage conversion cleanup, and audit-report docs.
- Branch: `main`
- Method tag/version: `change-friction-current-root-renewal`
- Comparability status: partially comparable. CAF/locality, boundary leakage,
  enum shock radius, gravity-well pressure, and release-sweep filtering remain
  comparable with the June 19 method. The current sample shifts from the
  `0.68` root-proof provisioning line to `0.74` root-managed proof renewal,
  with `0.72` endpoint/cycles hardening and `0.71` blob-storage CLI automation
  retained as recent cross-layer feature references.
- Auditor: Codex
- Run timestamp: `2026-06-28T13:34:31Z`
- Worktree state: dirty before audit; pre-existing root-renewal cleanup and
  complexity-audit docs were preserved.

## Audit Definition Review

No recurring audit definition changes were required for this run.

## 1. Velocity Risk Index

Velocity Risk Index: **4 / 10** after cleanup.

Change friction is moderate. The broad `0.74.0` root-managed renewal feature
and `0.72.0` endpoint/cycles security hardening slice both crossed most core
layers, raising the sampled edit-blast radius versus June 19. Follow-up work
is healthier: `0.74.13` plus the dirty scheduling/retrieval split decomposed
the renewal owner, this audit removed a small production API-to-model
conversion leak from `api::blob_storage`, and the blob-storage API facade is
now split into hash, lifecycle, gateway, billing, and test child modules.

| Metric | Previous | Current | Delta |
| --- | ---: | ---: | ---: |
| Velocity Risk Index | 4 | 4 | 0 |
| Cross-layer leakage crossings | 0 confirmed | 0 after cleanup | 0 |
| Avg files touched per sampled routine feature slice | 17.00 | 20.50 | +3.50 |
| p95 sampled routine files touched | 23 | 49 | +26 |
| Top gravity-well fan-in | capability DTO/proof surface in 17 files | root-renewal terms in 41 files; capability DTO/proof surface in 17 files | shifted upward |

Current routine averages use sampled commits `fd98634e`, `110f9c52`,
`f281192b`, `ecdc1cc7`, `769a1cb8`, and `3ea404b7`. The `37ac9f8e` CLI/auth
module split and the dirty root-renewal scheduling/retrieval split are tracked
as structural split work rather than routine feature friction.

## 2. Revised CAF + Locality Summary

| Feature | Slice Type | Files Modified | Subsystems | Layers | Flow Axes | Revised CAF | Density | ELS | Feature Locality Index | Containment Score | Risk |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `fd98634e` root-managed delegation proof renewal MVP | feature_slice | 38 | 8 | 5 | 6 | 48 | 4.75 | 0.50 | 0.39 | 1.00 | High |
| `769a1cb8` endpoint/cycles security hardening | feature_slice | 49 | 8 | 5 | 5 | 40 | 6.12 | 0.43 | 0.27 | 1.00 | High |
| `ecdc1cc7` renewal failure/status/docs hardening | feature_slice | 15 | 5 | 4 | 4 | 20 | 3.00 | 0.40 | 0.33 | 0.62 | Medium |
| `3ea404b7` blob-storage status check mode | feature_slice | 10 | 3 | 2 | 2 | 6 | 3.33 | 0.40 | 0.40 | 0.38 | Low |
| `f281192b` renewal storage/status persistence | feature_slice | 6 | 3 | 3 | 3 | 9 | 2.00 | 0.50 | 0.50 | 0.38 | Medium |
| `110f9c52` renewal scheduler/install follow-up | feature_slice | 5 | 3 | 2 | 3 | 9 | 1.67 | 0.60 | 0.40 | 0.38 | Medium |
| `37ac9f8e` CLI auth and renewal module split | release_sweep | 13 | 3 | 2 | 2 | 6 | 4.33 | 0.85 | 0.46 | 0.38 | Low |
| dirty root-renewal scheduling/retrieval split | release_sweep | 8 tracked plus 2 new source files | 1 | 1 | 2 | 2 | 10.00 | 0.88 | 0.88 | 0.12 | Low |

Interpretation:

- The feature-friction spike is real for initial root-managed renewal and
  security hardening slices. They required API, macro, DTO/protocol, policy,
  ops, storage, workflow, tests, and docs to move together.
- Follow-up renewal changes are more contained after the module split.
- The dirty split is a structural improvement: production renewal logic is now
  distributed across `mod.rs`, `schedule.rs`, `retrieval.rs`, `install.rs`,
  `identity.rs`, and `view.rs`, with no production renewal file over the large
  file threshold recorded by the complexity audit.

## 3. Edit Blast Radius Summary

| Metric | Current | Previous | Delta |
| --- | ---: | ---: | ---: |
| average files touched per sampled routine feature slice | 20.50 | 17.00 | +3.50 |
| median files touched | 12.50 | 16 | -3.50 |
| p95 files touched | 49 | 23 | +26 |

Status: `slice-sampled`.

The p95 increase is driven by two high-consequence slices:
`fd98634e` root-managed renewal MVP and `769a1cb8` endpoint/cycles hardening.
Narrow follow-ups stayed in the 5-15 file range.

## 4. Boundary Leakage Trend Table

| Boundary | Import Crossings | Previous | Delta | Risk |
| --- | ---: | ---: | ---: | --- |
| endpoint macros -> model/storage direct references | 0 production | 0 | 0 | Low |
| workflow/access/API -> model/storage direct references | 0 production after cleanup | 0 confirmed | 0 | Low |
| ops/storage/access -> workflow references | 0 | 0 | 0 | Low |
| policy/access -> ops/runtime side effects | 0 confirmed beyond access-boundary auth helpers | 0 confirmed | 0 | Low |
| auth/capability DTOs leaking into model/storage ownership | 0 confirmed | 0 | 0 | Low |

Evidence:

- Initial scan found two production `api::blob_storage` references to
  `crate::model::blob_storage::BlobRootHash::into_string`. The audit cleanup
  moved canonical root-hash string conversion behind
  `BlobStorageConversionOps::canonical_root_hash_text` and
  `BlobStorageConversionOps::canonical_root_hash_bytes`.
- Rerun boundary scan found only `#[cfg(test)]` `BlobStorageStore::clear*`
  reset helpers in `api::blob_storage::tests`.
- Reverse dependency scan from ops/storage/access to workflow returned no
  matches.

## 5. Change Multiplier Matrix

| Feature Axis | Endpoints | Workflow | Policy | Ops | Model/Storage | Subsystem Count |
| --- | --- | --- | --- | --- | --- | ---: |
| root-managed renewal template/schedule/get/install/status | yes | yes | yes | yes | yes | 5 |
| endpoint implicit-open and access hardening | yes | no | yes | yes | yes | 4 |
| child cycles funding ledger/cooldown policy | yes | yes | yes | yes | yes | 5 |
| blob-storage status check mode | CLI endpoint surface | no | no | host/CLI ops | no | 2 |
| canonical blob root hash conversion | API facade | no | no | yes | model owned behind ops | 2 |

| Candidate Feature | Axes Involved | Subsystem Count | Friction |
| --- | --- | ---: | --- |
| new root-renewal lifecycle outcome | DTO/protocol, policy, ops scheduler/retrieval/install/status, storage, workflow timer, tests/docs | 5 | High |
| new endpoint access predicate rule | macro parse/validate/expand, access eval, policy semantics, endpoint tests | 4 | Medium |
| new child funding role policy | config, policy, ops funding ledger, workflow execution, endpoint tests | 5 | High |
| new blob-storage CLI check flag | CLI parse/render/model/tests, host call seam, docs | 2 | Medium |
| new blob root hash input shape | API facade, ops conversion, model validator, endpoint tests | 2 | Low |

## 6. Enum Shock Radius Hotspots

| Enum | Variants | Switch Sites | Modules Using Enum | Switch Density | Subsystems | Shock Radius | Risk |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `dto::rpc::Request` | 4 active variants | 8 files with `Request::` | 8 | 1.00 | 4 | 16 | Medium |
| `dto::rpc::Response` | 4 active variants | 20 files with `Response::` | 20 | 1.00 | 4 | 16 | Medium |
| `dto::capability::CapabilityProof` | 1 active runtime mode | capability/proof fan-in scan found 17 files | 17 | 1.00 | 5 | 5 | Medium |
| `access::expr::BuiltinPredicate` | 4 top-level families | evaluator/constructor local sites | 2 | high local density | 1 | low | Low |
| `workflow::rpc::request::handler::RootCapability` | 4 variants | 5 files with `RootCapability::` | 5 | 1.00 | 2 | 8 | Medium |

No enum variant count grew in the audited sample. Root-renewal friction is
lifecycle/state-surface driven rather than enum-shock driven.

## 7. Gravity-Well Growth + Edit Frequency

| Module | LOC | LOC Delta | Fan-In | Fan-In Delta | Domains | Edit Frequency | Risk |
| --- | ---: | ---: | --- | --- | --- | --- | --- |
| `crates/canic-cli/src/auth/mod.rs` | 1406 | split from old `auth.rs` | CLI auth command/tests | new hub after split | renewal, delegated auth, medic rendering | high in 0.74 | Medium |
| `crates/canic-core/src/api/blob_storage/*` | parent 41 LOC; children: billing 460, lifecycle 122, gateway 67, hash 22, tests 517 | split from 1183-line facade | macro endpoints and tests | improved locality | blob lifecycle, gateway, billing status, funding | high in 0.70/0.71 | Medium |
| `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/*` | 2623 total including 948 test LOC | split from 2452-line parent | auth API/storage/workflow/tests | improved locality | schedule, retrieval, install, status, identity, view | high in 0.74 | Medium |
| `crates/canic-core/src/workflow/rpc/request/handler/*` | test file 1252, nonroot cycles 720 | stable/high | workflow/API/tests | stable | capability, replay, authorization, cycles | recurring edits | Medium |
| `crates/canic-core/src/ops/auth/token.rs` | 864 | stable/high | auth verifier/tests | stable | active proof, claims, issuer/root binding | recurring edits | Medium |
| `crates/canic-core/src/workflow/runtime/auth/prepare/mod.rs` | 718 | stable | workflow/runtime auth | stable | delegated-token prepare | recurring edits | Medium |

The strongest positive signal is the root-renewal split: the parent no longer
concentrates scheduling, retrieval, install, identity, view projection, and
tests in one production file.

## 8. Subsystem Independence Scores

| Subsystem | Internal Imports | External Imports | LOC Signal | Independence | Adjusted Independence | Risk |
| --- | ---: | ---: | --- | ---: | ---: | --- |
| `canic-core::ops` | high | storage/runtime/infra/model | large | medium-high | medium | Low |
| `canic-core::workflow` | high | ops/DTO/runtime metrics | large | medium | medium | Medium |
| `canic-core::access` | access-local plus auth/metrics | ops/config/runtime where boundary-owned | moderate | medium | medium | Low |
| `canic-core::dto` | passive data only | referenced broadly | large | low by design | medium risk from fan-in | Medium |
| `canic-core::api` | facade-local plus ops/DTO | ops conversion/lifecycle/funding | large | medium | medium | Medium |
| `canic-cli::auth` | CLI-local codec/render/tests | host/core call surfaces | large | medium | medium | Medium |

## 9. Independent-Axis Growth Warnings

| Operation | Axes | Axis Count | Independent Axes | Previous Independent Axes | Delta | Risk |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| root-managed renewal | template policy, scheduler due state, retrieval TTL, install deadline, install outcome, active proof freshness | 6 | 5 | N/A | N/A | High |
| delegated auth verification | audience, expiry, root proof, issuer proof, role grants, local verifier config, scope | 7 | 5 | 5 | 0 | Medium |
| root capability execution | request family, proof mode, replay state, role/subnet context, metrics outcome | 5 | 4 | 4 | 0 | Medium |
| child cycles funding | caller topology, role policy, cooldown, per-request cap, per-child cap, external deposit outcome | 6 | 5 | N/A | N/A | High |
| blob-storage status check mode | readiness, warning, command exit behavior, JSON/plain render | 4 | 2 | N/A | N/A | Low |

## 10. Decision Surface Size Trends

| Enum | Decision Sites | Previous | Delta | Risk |
| --- | ---: | ---: | ---: | --- |
| `dto::rpc::Request` | 8 files with `Request::` | 8 files | 0 | Medium |
| `dto::rpc::Response` | 20 files with `Response::` | broad | stable/no variant growth | Medium |
| `dto::capability::CapabilityProof` | 17 capability/proof fan-in files | 17 files | 0 | Medium |
| `access::expr::BuiltinPredicate` | evaluator/constructor-local | central local sites | stable | Low |
| `workflow::rpc::request::handler::RootCapability` | 5 files with `RootCapability::` | 5 files | 0 | Medium |

## 11. Refactor-Transient vs True-Drag Findings

| Signal | Raw Trend | Noise Classification | Adjusted Interpretation |
| --- | --- | --- | --- |
| `fd98634e` touched 38 files | broad | true feature drag | Root-managed renewal introduced new endpoint, DTO, policy, ops, storage, workflow, CLI, test, and docs surfaces. |
| `769a1cb8` touched 49 files | broad | true security-hardening drag | Endpoint hardening and funding policy persistence required broad but justified coordinated changes. |
| `37ac9f8e` touched 13 files with large churn | structural split | CLI auth and renewal modules became easier to navigate despite high inserted/deleted line counts. |
| dirty root-renewal split | file count increased, parent shrank | structural improvement | Scheduling and retrieval now have separate owners; production files remain below the large-file threshold. |
| blob-storage conversion cleanup | two production API/model references removed | boundary cleanup | API now delegates canonical text conversion to ops, leaving model references behind the conversion owner. |

## 12. Structural Drift Table

| Signal | Previous | Current | Delta | Risk |
| --- | ---: | ---: | ---: | --- |
| subsystem fan-in concentration | root proof provisioning plus distributed CLI/core hygiene | root-managed renewal plus CLI auth pressure; blob-storage API pressure split into children | shifted | Medium |
| top production module LOC pressure | core auth/runtime/replay plus CLI command/test pressure | CLI auth module remains the top current production command hub | shifted | Medium |
| cross-subsystem imports | no direct workflow/access/API storage references | no production workflow/access/API storage/model references after cleanup | 0 | Low |
| policy-layer decision ownership | no confirmed drift | no confirmed drift in scanned paths | 0 | Low |
| release/cleanup sweeps | visible | visible and structurally positive | neutral | Low |

## 13. Synthetic Feature Simulation

| Synthetic Feature | Files Touched | Subsystems | Layers | Risk |
| --- | ---: | ---: | ---: | --- |
| new root-renewal outcome state | 8-16 | DTO, policy, ops schedule/retrieval/install/status, storage, workflow, tests/docs | 5 | High |
| new root capability proof mode | 8-14 | DTO, ops hash/proof, workflow capability, metrics, tests/docs | 4 | High |
| new RPC request variant | 10-18 | DTO, API, workflow handler, replay, metrics, tests/docs | 5 | High |
| new child funding policy dimension | 8-18 | config, policy, ops ledger, workflow cycles, endpoint tests, docs | 5 | High |
| new blob-storage CLI automation flag | 3-8 | CLI model/options/render/tests, host call seam, docs | 2 | Medium |
| new blob root hash wire input | 3-6 | API facade, ops conversion, model validator, endpoint tests | 2 | Low |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-cli/src/auth/mod.rs` | `run`, `renewal_medic_summary`, auth command dispatch | new CLI auth command hub is large after 0.74 split | Medium |
| `crates/canic-core/src/api/blob_storage/*` | `BlobStorageApi` impl blocks | split into focused hash, lifecycle, gateway, billing, and tests modules; billing remains the largest child | Medium |
| `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/*` | scheduler/retrieval/install/status helpers | renewal lifecycle remains conceptually broad but is now split by responsibility | Medium |
| `crates/canic-core/src/workflow/rpc/request/handler/*` | `RootCapability`, replay, authorize, execute modules | root capability and nonroot cycles changes cross request type, replay, authorization, execution, metrics, and tests | Medium |
| `crates/canic-core/src/ops/auth/token.rs` | root/issuer proof and claims helpers | active proof and delegated-token trust-chain changes remain high consequence | Medium |
| `crates/canic-core/src/dto/auth.rs` | root renewal/proof and delegated auth DTOs | passive but broad protocol surface | Medium |
| `crates/canic-core/src/dto/capability/mod.rs` | capability proof/envelope DTOs | passive but broad capability fan-in | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/api/blob_storage/billing.rs` | DTO, lifecycle ops, funding guard, Cashier, management cycles | 4 | 3 | 6 |
| `crates/canic-core/src/api/blob_storage/{hash,lifecycle,gateway}.rs` | ops conversion/lifecycle and gateway principal helpers | 3 | 2 | 3 |
| `crates/canic-cli/src/auth/mod.rs` | CLI args, codec, render, host calls, medic summary | 3 | 2 | 7 |
| `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/*` | policy, DTO, storage ops, metrics, workflow callers | 5 | 3 | 6 |
| `crates/canic-core/src/workflow/rpc/request/handler/*` | DTO request/response, replay, metrics, authorization, execution | 4 | 3 | 6 |
| `crates/canic-core/src/dto/auth.rs` | root renewal, root proof, delegated-token protocol surface | 5 | 3 | 6 |
| `crates/canic-core/src/dto/capability/mod.rs` | capability envelopes and responses | 5 | 3 | 6 |

## Amplification Drivers

| Commit | Feature Slice | Slice Type | Files Touched | Subsystems | Density | CAF | Risk |
| --- | --- | --- | ---: | --- | ---: | ---: | --- |
| `fd98634e` | root-managed delegation proof renewal MVP | feature_slice | 38 | CLI, API, macros, access, policy, DTO, ops, storage, workflow, tests/docs | 4.75 | 48 | High |
| `769a1cb8` | endpoint/cycles security hardening | feature_slice | 49 | macros, access, config, policy, ops, storage, workflow, control-plane, test canisters, docs | 6.12 | 40 | High |
| `ecdc1cc7` | renewal failure/status/docs hardening | feature_slice | 15 | API, ops, workflow, docs/scripts | 3.00 | 20 | Medium |
| `3ea404b7` | blob-storage status check mode | feature_slice | 10 | CLI, docs, scripts | 3.33 | 6 | Low |
| `37ac9f8e` | CLI auth and renewal module split | release_sweep | 13 | CLI, ops, docs | 4.33 | 6 | Low |
| dirty | root-renewal scheduling/retrieval split | release_sweep | 10 source/report paths | ops, audit docs | 10.00 | 2 | Low |

Most impacted source paths:

- `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/*`
- `crates/canic-cli/src/auth/mod.rs`
- `crates/canic-core/src/api/blob_storage/billing.rs`
- `crates/canic-core/src/api/blob_storage/lifecycle.rs`
- `crates/canic-core/src/workflow/rpc/request/handler/nonroot_cycles.rs`
- `crates/canic-core/src/ops/auth/token.rs`
- `crates/canic-core/src/dto/auth.rs`

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| root-renewal concept fan-in | auth API, CLI, DTO, ops, storage, workflow, tests | renewal term scan found 41 Rust files | Medium |
| blob-storage API children | `crates/canic-core/src/api/blob_storage/*` | parent is 41 LOC after split; largest child is billing at 460 LOC | Medium |
| CLI auth hub after split | `crates/canic-cli/src/auth/mod.rs` | 1406 physical LOC; new codec/render/tests help but parent dispatch remains large | Medium |
| capability DTO fan-in | capability envelope/proof files | capability/proof fan-in scan found 17 files | Medium |
| enum shock | `dto::rpc::{Request, Response}` | variants stable; `Response::` appears in 20 files | Medium |

## Dependency Fan-In Pressure

| Module / Struct | Import or Reference Count | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| root-renewal terms | 41 files | CLI, access, API, policy, DTO, ops, storage, workflow, tests | High |
| capability DTO/proof terms | 17 files | canic macros, canic-core API/ops/workflow, tests, canisters | Medium |
| `BlobStorageApi` | 4 source files | canic facade/macros and canic-core API tests | Low |
| `BlobStorageConversionOps` | 2 source files | API and ops conversion owner | Low |
| `Request::` | 8 files | DTO, ops, workflow, tests | Medium |
| `Response::` | 20 files | DTO, ops, workflow, tests | Medium |
| `RootCapability::` | 5 files | workflow and tests | Low |

## Risk Score

Risk Score: **4 / 10** after cleanup.

| Area | Score | Weight | Weighted Score |
| ---- | ----: | ----: | ----: |
| enum shock radius | 3 | 3 | 9 |
| CAF trend | 5 | 2 | 10 |
| cross-layer leakage | 1 | 2 | 2 |
| gravity-well growth | 4 | 2 | 8 |
| edit blast radius | 5 | 1 | 5 |

Weighted aggregate: `34 / 10 = 3.40`, adjusted to **4 / 10** because two
routine feature slices still had CAF above `12` and the CLI auth command hub
still has a pressure score of `7`. The score does not honestly fall to `3`
until either the next sampled feature window is narrower or the CLI auth hub is
split further.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `git log --name-only -n 30 -- crates/canic-core/src crates/canic-cli/src crates/canic-host/src crates/canic-tests/tests docs/audits docs/design docs/changelog` | PASS | Feature-slice evidence captured for current audit window. |
| `rg 'crate::storage|crate::model|canic_core::storage|canic_core::model' crates/canic/src crates/canic-core/src/workflow crates/canic-core/src/access crates/canic-core/src/api -g '*.rs'` | PASS after cleanup | Initial production API/model hits were removed; remaining matches are test-only `BlobStorageStore::clear*` reset helpers. |
| `rg 'crate::workflow|canic_core::workflow' crates/canic-core/src/ops crates/canic-core/src/storage crates/canic-core/src/access -g '*.rs'` | PASS | No matches. |
| `rg -n 'enum Request|enum Response|enum CapabilityProof|enum BuiltinPredicate|enum RootCapability|enum ReplayPreflight|enum RootPreflight' crates/canic-core/src crates/canic/src -g '*.rs'` | PASS | Enum decision-surface evidence captured. |
| `find crates/canic-core/src crates/canic-cli/src crates/canic-host/src -type f -name '*.rs' -exec wc -l {} + | sort -nr | sed -n '1,40p'` | PASS | LOC/gravity scan captured. |
| `rg -l 'RootCapabilityEnvelopeV1|NonrootCyclesCapabilityEnvelopeV1|RootCapabilityResponseV1|NonrootCyclesCapabilityResponseV1|CapabilityProof|CapabilityService' crates canisters fleets -g '*.rs'` | PASS | Capability fan-in scan found 17 files. |
| `cargo fmt --all` | PASS | Formatting applied. |
| `cargo test --locked -p canic-core --features blob-storage canonical_text_helpers_return_boundary_strings --lib -- --nocapture` | PASS | 1 passed. |
| `cargo test --locked -p canic-core --lib workflow::rpc::request::handler -- --nocapture` | PASS | 33 passed. |
| `cargo test --locked -p canic-core --lib workflow::rpc::capability -- --nocapture` | PASS | 15 passed. |
| `cargo test --locked -p canic-core --lib ops::auth::delegation -- --nocapture` | PASS | 46 passed. |
| `cargo test --locked -p canic-core blob_storage --lib --features blob-storage-billing -- --nocapture` | PASS | 49 passed. |
| `cargo check --locked -p canic --features blob-storage-billing` | PASS | Public `canic` facade compiled with blob-storage billing enabled. |
| `cargo clippy --locked -p canic-core --lib --features blob-storage-billing -- -D warnings` | PASS | Affected core feature set linted cleanly. |
| `cargo fmt --all -- --check` | PASS | Final format check passed. |
| `git diff --check` | PASS | No whitespace errors. |

## Follow-Up Actions

1. Keep root-renewal scheduling, retrieval, install, identity, and view
   responsibilities split. New renewal outcomes should be planned as
   coordinated DTO/policy/ops/storage/workflow/test slices.
2. Consider a follow-up CLI auth module pass if `crates/canic-cli/src/auth/mod.rs`
   grows further after the 0.74 split.
3. Keep blob-storage API additions in the focused `hash`, `lifecycle`,
   `gateway`, and `billing` children rather than re-growing the parent facade.
