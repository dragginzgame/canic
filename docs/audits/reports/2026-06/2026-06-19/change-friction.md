# Change Friction Audit - 2026-06-19

## Report Preamble

- Scope: `crates/canic-core`, `crates/canic`, `crates/canic-cli`,
  `crates/canic-host`, `crates/canic-tests`, test canisters, active audit
  definitions, active design/changelog docs, and recent `0.68.x` root proof
  provisioning plus hygiene/release-sweep slices.
- Definition path: `docs/audits/recurring/system/change-friction.md`
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-13/change-friction.md`
- Code snapshot identifier: `ef55e53c`
- Branch: `main`
- Method tag/version: `change-friction-current-root-proof-provisioning`
- Comparability status: partially comparable. CAF/locality, boundary leakage,
  enum shock radius, gravity-well pressure, and release-sweep filtering remain
  comparable; the current sample is the `0.68` root-proof provisioning and
  hygiene line rather than the earlier post-`0.65` auth cleanup plus host
  decomposition line.
- Auditor: Codex
- Run timestamp: `2026-06-19T14:07:55Z`
- Worktree state: dirty before audit; unrelated dirty files were preserved.

## Audit Definition Review

The audit definition was reviewed before running the audit.

Changes made:

- Added method tag `change-friction-current-root-proof-provisioning`.
- Replaced the stale flat `access/expr.rs` hotspot with `access/expr/*`.
- Added current root proof provisioning hotspots:
  `workflow/runtime/auth/provisioning/*`, `ops/auth/delegation/*`, and
  `ops/rpc/capability.rs`.
- Added current source scans and targeted test commands.

The CAF method itself was not changed.

## 1. Velocity Risk Index

Velocity Risk Index: **4 / 10**.

Risk remains moderate and manageable. No cross-layer leakage regression was
found, and the root proof provisioning ops owner has been split into focused
modules. The remaining friction is real but expected: root proof provisioning
and root capability changes still cross API, DTO/protocol, ops, storage,
workflow, endpoint/macro, tests, and docs.

| Metric | Previous | Current | Delta |
| --- | ---: | ---: | ---: |
| Velocity Risk Index | 4 | 4 | 0 |
| Cross-layer leakage crossings | 0 after fix | 0 confirmed | 0 |
| Avg files touched per sampled routine feature slice | 36.20 | 17.00 | -19.20 |
| p95 sampled routine files touched | 64 | 23 | -41 |
| Top gravity-well fan-in | 17 | capability DTO/proof surface in 17 files | stable |

Current routine averages use sampled commits `24d45756`, `165d123b`,
`e65d579a`, `c19b9767`, and `88db851f`. Broad hygiene/release-sweep commits
`0844ddb7` and `2fb69455` are tracked separately and excluded from the routine
feature-friction average.

## 2. Revised CAF + Locality Summary

| Feature | Slice Type | Files Modified | Subsystems | Layers | Flow Axes | Revised CAF | Density | ELS | Feature Locality Index | Containment Score | Risk |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `24d45756` root proof broadcast/provisioning follow-up | feature_slice | 16 | 4 | 3 | 3 | 12 | 4.00 | 0.62 | 0.56 | 0.50 | Medium |
| `165d123b` proof retrieval/direct-query and capability proof follow-up | feature_slice | 14 | 4 | 3 | 3 | 12 | 3.50 | 0.71 | 0.64 | 0.50 | Medium |
| `e65d579a` issuer install endpoint/protocol surface | feature_slice | 18 | 6 | 4 | 4 | 24 | 3.00 | 0.39 | 0.39 | 0.75 | Medium |
| `c19b9767` root issuer policy/API cleanup | feature_slice | 23 | 6 | 4 | 4 | 24 | 3.83 | 0.35 | 0.35 | 0.75 | Medium |
| `88db851f` layer-guard/runtime cleanup | feature_slice | 14 | 4 | 3 | 3 | 12 | 3.50 | 0.57 | 0.50 | 0.50 | Medium |
| `2fb69455` audit/code-hygiene guard sweep | release_sweep | 73 | 5 | 2 | 2 | 10 | 14.60 | 0.81 | 0.74 | 0.62 | Low |
| `0844ddb7` broad hygiene/module split and audit sweep | release_sweep | 200 | 8 | 5 | 3 | 24 | 25.00 | 0.58 | 0.47 | 1.00 | Low |

Interpretation:

- Root proof provisioning remains the main routine friction axis because even
  small behavior changes span auth API/DTO/protocol, ops, storage, workflow,
  tests, and docs.
- The largest current file counts are release/code-hygiene sweeps, not routine
  product-feature friction.
- The split from `ops/auth/delegation.rs` to `ops/auth/delegation/*` reduces
  future locality pressure even though the release-sweep diff is large.

## 3. Edit Blast Radius Summary

| Metric | Current | Previous | Delta |
| --- | ---: | ---: | ---: |
| average files touched per sampled routine feature slice | 17.00 | 36.20 | -19.20 |
| median files touched | 16 | 27 | -11 |
| p95 files touched | 23 | 64 | -41 |

Status: `slice-sampled`.

The current routine sample is smaller than the 2026-06-13 sample. This is
partly because the remaining `0.68` work is guard hardening and cleanup around
an already-shaped root proof provisioning model. It does not mean the auth
slice is cheap; the subsystem count remains high when behavior changes.

## 4. Boundary Leakage Trend Table

| Boundary | Import Crossings | Previous | Delta | Risk |
| --- | ---: | ---: | ---: | --- |
| endpoint macros -> model/storage direct references | 0 | 0 | 0 | Low |
| workflow/access/API -> model/storage direct references | 0 | 0 after fix | 0 | Low |
| ops/storage/access -> workflow references | 0 | 0 | 0 | Low |
| policy/access -> ops/runtime side effects | 0 confirmed beyond access boundary | 0 confirmed | 0 | Low |
| auth/capability DTOs leaking into model/storage ownership | 0 confirmed | 0 | 0 | Low |

Evidence:

- `rg 'crate::storage|crate::model|canic_core::storage|canic_core::model'`
  against endpoint/API/access/workflow paths returned no matches.
- `rg 'crate::workflow|canic_core::workflow'` against ops/storage/access
  returned no matches.
- Current root proof provisioning storage access is mediated through
  `ops/auth/delegation/*` and `ops/storage/auth/*`.

## 5. Change Multiplier Matrix

| Feature Axis | Endpoints | Workflow | Policy | Ops | Model/Storage | Subsystem Count |
| --- | --- | --- | --- | --- | --- | ---: |
| root proof batch prepare/get/install | yes | yes | yes | yes | yes | 5 |
| issuer active proof install/status | yes | no | yes | yes | yes | 4 |
| root capability request family | yes | yes | yes | yes | yes | 5 |
| delegated-token verifier rule | yes | yes | yes | yes | yes | 5 |
| root runtime metrics/hygiene guards | no | no | no | yes | yes | 2 |
| CLI evidence/output hygiene | endpoints/CLI | no | no | host/CLI | no | 2 |

| Candidate Feature | Axes Involved | Subsystem Count | Friction |
| --- | --- | ---: | --- |
| new root proof provisioning endpoint | API/protocol, DTO, ops, storage, workflow, tests/docs | 5 | High |
| new issuer proof install rule | policy, ops verifier/storage, API status, tests/docs | 4 | Medium |
| new delegated auth verifier rule | endpoint auth, verifier ops, config/DTO, storage, workflow runtime | 5 | High |
| new root capability request variant | DTO request/response, proof routing, replay, authorization, execution, metrics | 5 | High |
| new CLI evidence formatter | CLI parse/render, host evidence shape, tests/docs | 2 | Medium |

## 6. Enum Shock Radius Hotspots

| Enum | Variants | Switch Sites | Modules Using Enum | Switch Density | Subsystems | Shock Radius | Risk |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `dto::rpc::Request` | 4 active variants | 8 files with `Request::` | 8 | 1.00 | 4 | 16 | Medium |
| `dto::rpc::Response` | 4 active variants | scan-observed across ops/workflow/tests | 6+ | moderate | 4 | medium | Medium |
| `dto::capability::CapabilityProof` | 1 active runtime mode | 17 files in capability/proof fan-in scan | 17 | 1.00 | 5 | 5 | Medium |
| `access::expr::BuiltinPredicate` | 4 top-level families | evaluator/constructor sites | 2 | high local density | 1 | low | Low |
| `workflow::rpc::request::handler::RootCapability` | 4 variants | 5 files with `RootCapability::` | 5 | 1.00 | 2 | 8 | Medium |

No enum is Critical. The highest future friction remains `Request`/`Response`
plus `RootCapability`, because new root capability variants require DTO,
workflow mapping, authorization, execution, replay, metrics, and tests.

## 7. Gravity-Well Growth + Edit Frequency

| Module | LOC | LOC Delta | Fan-In | Fan-In Delta | Domains | Edit Frequency | Risk |
| --- | ---: | ---: | --- | --- | --- | --- | --- |
| `crates/canic-core/src/workflow/rpc/request/handler/*` | 2,994 | stable/high | workflow/API/tests | stable | capability, replay, authorization, execution | high | Medium |
| `crates/canic-core/src/ops/auth/delegation/*` | 1,664 | split from single large file | auth API/storage/tests | improved locality | root proof prepare/get/install/status | high | Medium |
| `crates/canic-core/src/workflow/runtime/auth/prepare/mod.rs` | 718 | stable | workflow/runtime auth | stable | delegated-token prepare | high | Medium |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | 754 | stable | auth verifier/tests | stable | token, audience, grant, scope, proof | high | Medium |
| `crates/canic-core/src/access/expr/mod.rs` | 668 | stable | endpoint access macros/tests | stable | access expression construction/eval | medium | Medium |
| `crates/canic-host/src/deployment_truth/*` | broad, top production files below 505 LOC | improved from old monoliths | host-local public support | improved | authority, lifecycle, promotion, reports | medium | Medium |
| `crates/canic-host/src/install_root/*` | broad, parent `mod.rs` 294 LOC | improved | host/CLI install flow | improved | readiness, truth gate, activation, receipts | medium | Medium |

The previous report's largest host files have been decomposed substantially.
Current large-file pressure is more distributed, with test modules and focused
core auth/runtime owners at the top.

## 8. Subsystem Independence Scores

| Subsystem | Internal Imports | External Imports | LOC Signal | Independence | Adjusted Independence | Risk |
| --- | ---: | ---: | --- | ---: | ---: | --- |
| `canic-core::ops` | high | storage/runtime/infra | large | medium-high | medium | Low |
| `canic-core::workflow` | high | ops/DTO/runtime metrics | large | medium | medium | Medium |
| `canic-core::access` | access-local plus auth/metrics | ops/config/runtime where boundary-owned | moderate | medium | medium | Low |
| `canic-core::dto` | passive data only | referenced broadly | moderate | low by design | medium risk from fan-in | Medium |
| `canic-host::deployment_truth` | high host-local | CLI/install consumers | broad | medium | medium | Medium |
| `canic-cli` command modules | CLI-local parse/render | host/backup/core package helpers | broad | medium | medium | Medium |

## 9. Independent-Axis Growth Warnings

| Operation | Axes | Axis Count | Independent Axes | Previous Independent Axes | Delta | Risk |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| root proof provisioning | issuer policy, request id, batch id, cert hash, direct query, proof install, active proof freshness | 7 | 5 | N/A | N/A | Medium |
| delegated auth verification | audience, expiry, root proof, issuer proof, role grants, local verifier config, scope | 7 | 5 | 4 | +1 | Medium |
| root capability execution | request family, proof mode, replay state, role/subnet context, metrics outcome | 5 | 4 | 4 | 0 | Medium |
| deployment-truth promotion/install | artifact source, materialization, policy, provenance, receipt state | 5 | 4 | 4 | 0 | Medium |
| code-hygiene release sweep | module ownership, lint posture, test layout, docs/audit summaries | 4 | 2 | N/A | N/A | Low |

## 10. Decision Surface Size Trends

| Enum | Decision Sites | Previous | Delta | Risk |
| --- | ---: | ---: | ---: | --- |
| `dto::rpc::Request` | 8 files with `Request::` | 8 files | stable | Medium |
| `dto::rpc::Response` | ops/workflow/replay/test references | broad | stable | Medium |
| `dto::capability::CapabilityProof` | 1 active runtime mode, 17 file fan-in | 10 references / lower scan scope | not method-comparable | Medium |
| `access::expr::BuiltinPredicate` | evaluator/constructor-local | central local sites | stable | Low |
| `workflow::rpc::request::handler::RootCapability` | 5 files with `RootCapability::` | 5 files | stable | Medium |

## 11. Refactor-Transient vs True-Drag Findings

| Signal | Raw Trend | Noise Classification | Adjusted Interpretation |
| --- | --- | --- | --- |
| `0844ddb7` touched 200 files | very broad | release/code-hygiene sweep | Not routine feature friction; includes module split, audit reports, docs, and broad hygiene. |
| `2fb69455` touched 73 files | broad | audit/code-hygiene guard sweep | Mostly lint/audit/ops-runtime guard work; tracked separately from routine feature CAF. |
| `ops/auth/delegation.rs` split | file count increased | structural improvement | Parent moved to directory-module owner; active/batch/pending/policy/error/test responsibilities are easier to locate. |
| root proof provisioning feature slices | multi-subsystem | true drag, expected | The capability needs API/protocol, DTO, policy, ops, storage, workflow, tests, and docs. |
| host deployment-truth decomposition | persistent broad tree | improved true drag | Parent modules are smaller; remaining breadth is distributed across focused host owners. |

## 12. Structural Drift Table

| Signal | Previous | Current | Delta | Risk |
| --- | ---: | ---: | ---: | --- |
| subsystem fan-in concentration | host/deployment-truth plus auth/helper-heavy | root proof provisioning plus distributed CLI/core hygiene | shifted | Medium |
| top production module LOC pressure | host deployment-truth/install-root heavy | core auth/runtime/replay plus CLI command/test pressure | shifted | Medium |
| cross-subsystem imports | no direct workflow storage after fix | no direct workflow/access/API storage references | 0 | Low |
| policy-layer decision ownership | no confirmed drift | no confirmed drift in scanned paths | 0 | Low |
| release/cleanup sweeps | visible | very visible | up, but classified | Low |

## 13. Synthetic Feature Simulation

| Synthetic Feature | Files Touched | Subsystems | Layers | Risk |
| --- | ---: | ---: | ---: | --- |
| new root proof provisioning policy | 8-16 | API, DTO/protocol, policy, ops, storage, workflow, tests/docs | 5 | High |
| new capability proof mode | 8-14 | DTO, ops hash/proof, workflow capability, metrics, tests/docs | 4 | High |
| new RPC request variant | 10-18 | DTO, API, workflow handler, replay, metrics, tests/docs | 5 | High |
| new delegated auth verifier rule | 8-16 | config/DTO, endpoint access, workflow runtime, ops verifier, storage, tests/docs | 4-5 | High |
| new CLI evidence/output cleanup | 3-8 | CLI, host evidence helper, tests/docs | 2 | Medium |
| new host deployment-truth report | 6-12 | host model/report/text/tests, optional CLI wrapper | 2-3 | Medium |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/workflow/rpc/request/handler/*` | `RootCapability`, replay, authorize, execute modules | root capability changes cross request type, replay, authorization, execution, metrics, and tests | Medium |
| `crates/canic-core/src/ops/auth/delegation/*` | active, batch, pending, root issuer policy, errors | root proof provisioning state/proof operations are split but remain central | Medium |
| `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs` | root proof broadcast install workflow | root proof installation crosses workflow, ops, calls, and issuer endpoints | Medium |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | verifier-local token proof checks | delegated-token verifier rule changes have broad auth impact | Medium |
| `crates/canic-core/src/dto/auth.rs` | root proof and delegated auth DTOs | passive but broad protocol surface | Medium |
| `crates/canic-core/src/dto/capability/mod.rs` | capability proof/envelope DTOs | passive but broad capability fan-in | Medium |
| `crates/canic-host/src/deployment_truth/*` | deployment-truth families | broad host-owned support tree, now decomposed | Medium |
| `crates/canic-host/src/install_root/*` | install-root families | root install flow touches readiness, truth gate, activation, receipts, and output | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/ops/auth/delegation/*` | active, batch, pending, root_issuer_policy, storage auth, DTO auth | 4 | 3 | 6 |
| `crates/canic-core/src/workflow/rpc/request/handler/*` | DTO request/response, replay, metrics, authorization, execution | 4 | 3 | 6 |
| `crates/canic-core/src/dto/auth.rs` | root proof, delegated-token, protocol surface | 5 | 3 | 6 |
| `crates/canic-host/src/deployment_truth/*` | model, report, text, lifecycle, promotion, authority, receipt | 1 host subsystem, many domains | 2 | 6 |
| `crates/canic-host/src/install_root/*` | readiness, truth gate, activation, receipts, config selection | 3 | 3 | 6 |

## Verification Readout

Commands passed:

- Stale audit-definition scan for old flat paths and retired capability grant
  wording: PASS.
- Boundary scan:
  `rg 'crate::storage|crate::model|canic_core::storage|canic_core::model'`
  against endpoint/API/access/workflow paths: PASS, no matches.
- Reverse dependency scan:
  `rg 'crate::workflow|canic_core::workflow'` against ops/storage/access:
  PASS, no matches.
- Enum/fan-in scans for `Request`, `Response`, `CapabilityProof`,
  `BuiltinPredicate`, `RootCapability`, `ReplayPreflight`, and capability DTO
  fan-in: PASS.
- LOC/gravity scan over `crates/canic-core/src`, `crates/canic-cli/src`, and
  `crates/canic-host/src`: PASS.
- `cargo test --locked -p canic-core --lib workflow::rpc::request::handler -- --nocapture`
  - PASS, 32 passed.
- `cargo test --locked -p canic-core --lib workflow::rpc::capability -- --nocapture`
  - PASS, 15 passed.
- `cargo test --locked -p canic-core --lib ops::auth::delegation -- --nocapture`
  - PASS, 26 passed.

## Follow-Up Actions

1. Treat future root proof provisioning behavior changes as coordinated
   cross-layer slices. Plan DTO/protocol, policy, ops, storage, workflow,
   tests, and docs together.
2. Keep the `ops/auth/delegation/*` split intact; avoid re-growing the parent
   module into the pre-split gravity well.
3. Keep capability DTOs passive and root capability request changes covered by
   request handler, capability proof, replay, metrics, and endpoint tests.
4. Continue separating broad code-hygiene/release sweeps from routine feature
   friction in future change-friction reports.
