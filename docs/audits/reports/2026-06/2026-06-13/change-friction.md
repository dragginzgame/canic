# Change Friction Audit - 2026-06-13

## Report Preamble

- Scope: `crates/canic-core`, `crates/canic`, `crates/canic-cli`,
  `crates/canic-host`, `crates/canic-tests`, test canisters, active docs, and
  recent `0.66.x` / `0.67.x` feature and cleanup slices.
- Definition path:
  `docs/audits/recurring/system/change-friction.md`
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-29/change-friction.md`
- Code snapshot identifier: `ea21d8a0`
- Branch: `main`
- Method tag/version: `change-friction-current-surface`
- Comparability status: `partially comparable`
- Auditor: `codex`
- Run timestamp: `2026-06-13`
- Worktree: `dirty`

Comparability note: the prior run sampled `0.48.x` setup, demo sharding,
freshness, and role-artifact slices. This run samples the current post-0.65
auth cleanup line, NNS/operator helper extraction, topology command cleanup,
and host test/module cleanup. The method is comparable, but the feature mix is
different and the `0.67.1` host slice is treated as a release/cleanup sweep,
not routine feature friction.

Same-slice remediation note: the initial scan found one direct
workflow-to-storage type reference in pool recycle metadata handling. That was
fixed by moving recycle metadata projection into `ops::storage::pool` and
rerunning the boundary scan before this report was finalized.

## 1. Velocity Risk Index

Velocity Risk Index: **4 / 10**.

Risk remains low-moderate after the same-slice cleanup. Routine feature-slice
average file count increased against the 2026-05-29 baseline, but the one
workflow-to-storage type reference found by the initial mechanical scan was
removed before finalization. The result is still bounded: the broadest current
slice is an operator/helper extraction that removes future Canic ownership, and
the largest `0.67.1` file count is a host test/module cleanup sweep rather than
new runtime coupling.

| Metric | Previous | Current | Delta |
| --- | ---: | ---: | ---: |
| Velocity Risk Index | 4 | 4 | 0 |
| Cross-layer leakage crossings | 0 confirmed | 0 after fix | 0 |
| Avg files touched per sampled routine feature slice | 28.43 | 36.20 | +7.77 |
| p95 sampled routine files touched | 65 | 64 | -1 |
| Top gravity-well fan-in | 4 | 17 | not method-comparable |

Current routine averages use sampled commits `5996340`, `28ad0e3`,
`0ec1fb8`, `2c1dd86`, and `4f7e76e`. Commit `d81096f` is tracked separately
as a host cleanup/test-structure sweep.

## 2. Revised CAF + Locality Summary

| Feature | Slice Type | Files Modified | Subsystems | Layers | Flow Axes | Revised CAF | Density | ELS | Feature Locality Index | Containment Score | Risk |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `5996340` NNS/operator helper extraction | feature_slice | 64 | 7 | 2 | 3 | 21 | 9.14 | 0.27 | 0.22 | 0.88 | Medium |
| `28ad0e3` topology command/report cleanup | feature_slice | 9 | 4 | 2 | 2 | 8 | 2.25 | 0.44 | 0.22 | 0.50 | Low |
| `0ec1fb8` topology output and auth-runtime follow-up | feature_slice | 26 | 5 | 3 | 3 | 15 | 5.20 | 0.35 | 0.19 | 0.62 | Medium |
| `2c1dd86` auth verifier terminology and runtime cleanup | feature_slice | 27 | 4 | 3 | 3 | 12 | 6.75 | 0.44 | 0.44 | 0.50 | Medium |
| `4f7e76e` auth config/schema/runtime cleanup | feature_slice | 55 | 6 | 4 | 4 | 24 | 9.17 | 0.55 | 0.45 | 0.75 | Medium |
| `d81096f` host tests/module cleanup | release_sweep | 61 | 3 | 1 | 1 | 3 | 20.33 | 0.98 | 0.80 | 0.38 | Low |

Interpretation:

- The broadest routine slice is the `0.67.0` NNS/operator helper extraction.
  Its file count is high because it crosses CLI docs/help, host support
  modules, tests, workspace manifests, scripts, and new helper crates.
- The highest CAF routine slice is `4f7e76e`, where auth config/schema/runtime
  wording and verifier cleanup touched API, config, ops, storage, workflow, and
  tests together.
- The `0.67.1` host cleanup sweep has a large file count but strong locality:
  nearly all edits stay inside `canic-host`, especially tests. Treat this as
  structural cleanup churn, not feature friction.

## 3. Edit Blast Radius Summary

| Metric | Current | Previous | Delta |
| --- | ---: | ---: | ---: |
| average files touched per sampled routine feature slice | 36.20 | 28.43 | +7.77 |
| median files touched | 27 | 24 | +3 |
| p95 files touched | 64 | 65 | -1 |

Status: `slice-sampled`.

The average rose because the current window contains broad helper extraction
and auth/config cleanup. The p95 did not rise; there is no evidence of runaway
routine file count growth beyond the prior setup hard-cut baseline.

## 4. Boundary Leakage Trend Table

| Boundary | Import Crossings | Previous | Delta | Risk |
| --- | ---: | ---: | ---: | --- |
| endpoint macros -> model/storage direct references | 0 | 0 | 0 | Low |
| workflow -> model/storage direct references | 0 after fix | 0 confirmed | 0 | Low |
| workflow -> ops-mediated storage access | many expected call sites | expected | stable | Low |
| policy/access -> ops/runtime side effects | 0 confirmed beyond access boundary | 0 confirmed | 0 | Low |
| ops/storage -> workflow references | 0 | 0 | 0 | Low |
| auth/capability DTOs leaking into model/storage ownership | 0 confirmed | 0 | 0 | Low |

Evidence:

- The initial `rg 'crate::storage|crate::model'` scan against
  workflow/access/api paths found one direct workflow reference in pool recycle
  metadata handling.
- The cleanup added `PoolRegistrationMetadata` in
  `crates/canic-core/src/ops/storage/pool/mod.rs` and made
  `crates/canic-core/src/workflow/pool/mod.rs` pass that projection instead of
  naming `CanisterRecord`.
- The rerun scan found no direct `crate::storage` or `crate::model` references
  in workflow/access/api paths.
- `rg 'crate::workflow|canic_core::workflow'` against ops/storage/access found
  no reverse dependency.
- A stricter data-shape pass should still look at workflow call sites that
  consume `CanisterRecord` values indirectly through `SubnetRegistryOps::get`.
  This report closes the explicit pool recycle crossing, not a full registry
  projection migration.

## 5. Change Multiplier Matrix

| Feature Axis | Endpoints | Workflow | Policy | Ops | Model/Storage | Subsystem Count |
| --- | --- | --- | --- | --- | --- | ---: |
| NNS/operator helper extraction | yes | no | no | yes | no | 2 |
| delegated auth verifier naming/config | yes | yes | yes | yes | yes | 5 |
| topology command output shape | yes | no | no | yes | no | 2 |
| root capability request family | yes | yes | yes | yes | yes | 5 |
| host deployment-truth evidence flow | no | no | yes | yes | no | 2 |
| host test/module decomposition | no | no | no | yes | no | 1 |

| Candidate Feature | Axes Involved | Subsystem Count | Friction |
| --- | --- | ---: | --- |
| new external helper-backed operator command | CLI surface, host adapter, scripts/install, docs/tests | 4 | Medium |
| new delegated auth verifier rule | DTO/config, endpoint auth, workflow runtime, ops verifier, storage records | 5 | High |
| new root capability request variant | request type, proof routing, replay, authorization, execution, metrics | 5 | High |
| new host deployment-truth report | host model/report/text/tests plus CLI parser/renderer if exposed | 3 | Medium |
| new host test module split | test module layout and `mod.rs` exports | 1 | Low |

## 6. Enum Shock Radius Hotspots

| Enum | Variants | Switch Sites | Modules Using Enum | Switch Density | Subsystems | Shock Radius | Risk |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `dto::rpc::Request` | 4 active variants | 44 `Request::` references | 8 files | 5.50 | 4 | 88 | Medium |
| `dto::rpc::Response` | 4 active variants | 80 `Response::` references | scan-observed across workflow/ops/tests | high | 4 | high | Medium |
| `dto::capability::CapabilityProof` | 1 active runtime mode | 10 `CapabilityProof::` references | scan-observed across DTO/workflow/tests | moderate | 3 | moderate | Medium |
| `access::expr::BuiltinPredicate` | 4 top-level families | central evaluator/constructor sites | access-local | low | 1 | low | Low |
| `workflow::rpc::request::handler::RootCapability` | 4 variants | 43 `RootCapability::` references | 5 files | 8.60 | 2 | 68.8 | Medium |

No enum crosses into Critical risk. `Request`, `Response`, and
`RootCapability` remain the main future-friction surfaces because adding a
new root capability request still touches DTO, workflow mapping,
authorization, execution, replay, metrics, and tests.

## 7. Gravity-Well Growth + Edit Frequency

| Module | LOC | LOC Delta | Fan-In | Fan-In Delta | Domains | Edit Frequency (30d) | Risk |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `crates/canic-host/src/deployment_truth/promotion.rs` | 5279 | N/A | public through `deployment_truth` | N/A | promotion, artifacts, policy, provenance | high | Medium |
| `crates/canic-host/src/deployment_truth/lifecycle.rs` | 4017 | N/A | public through `deployment_truth` | N/A | lifecycle, consent, verification | high | Medium |
| `crates/canic-host/src/install_root/mod.rs` | 3007 | stable | CLI/install/deploy consumers | stable | install, verification, local state | high | Medium |
| `crates/canic-cli/src/fleets/mod.rs` | 2310 | N/A | CLI command family | N/A | fleet config, role lifecycle, adoption | medium | Medium |
| `crates/canic-core/src/workflow/ic/icp_refill/mod.rs` | 1268 | N/A | workflow/API/tests | N/A | refill, ledger, replay, metrics | medium | Medium |
| `crates/canic-core/src/workflow/rpc/request/handler/tests.rs` | 1198 | stable | test-only | stable | capability, replay, authorization | medium | Low |
| `crates/canic-core/src/workflow/pool/mod.rs` | 1174 | stable | workflow/admin/tests | stable | pool, recycle, replay, codec | medium | Medium |

The host deployment-truth family remains the largest true drag. The `0.67.1`
test split improved scanability around it, but did not reduce the top
production LOC gravity wells.

## 8. Subsystem Independence Scores

| Subsystem | Internal Imports | External Imports | LOC Signal | Independence | Adjusted Independence | Risk |
| --- | ---: | ---: | --- | ---: | ---: | --- |
| `canic-core::ops` | high | storage/infra/runtime | large | medium-high | medium | Low |
| `canic-core::workflow` | high | ops/runtime metrics, DTO | large | medium | medium-high | Medium |
| `canic-core::access` | access-local plus auth/metrics | ops/config/runtime where boundary-owned | moderate | medium | medium | Low |
| `canic-host::deployment_truth` | high host-local fan-in | core config parsing, filesystem/process support | very large | medium | medium | Medium |
| `canic-host::install_root` | host modules plus core DTO/config helpers | CLI/install consumers | large | medium | medium | Medium |
| `canic-cli` command modules | CLI-local parse/render | host/backup/core packages | large | medium | medium | Medium |

## 9. Independent-Axis Growth Warnings

| Operation | Axes | Axis Count | Independent Axes | Previous Independent Axes | Delta | Risk |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| delegated auth verification | audience, expiry, root proof, issuer proof, role grants, local verifier config | 6 | 4 | 4 | 0 | Medium |
| root capability execution | request family, proof mode, replay state, role/subnet context, metrics outcome | 5 | 4 | 4 | 0 | Medium |
| operator helper-backed topology inspection | helper version/source, network, cache freshness, render shape | 4 | 3 | N/A | N/A | Medium |
| deployment-truth promotion | artifact source, materialization, policy, provenance, receipt state | 5 | 4 | 4 | 0 | Medium |
| host module/test decomposition | module ownership, test fixture location | 2 | 1 | N/A | N/A | Low |

## 10. Decision Surface Size Trends

| Enum | Decision Sites | Previous | Delta | Risk |
| --- | ---: | ---: | ---: | --- |
| `dto::rpc::Request` | 44 references / 8 files | ~12 sites | not method-comparable | Medium |
| `dto::rpc::Response` | 80 references | ~8 sites | not method-comparable | Medium |
| `dto::capability::CapabilityProof` | 10 references | ~8 sites | stable/down in active modes | Medium |
| `access::expr::BuiltinPredicate` | central evaluator/constructor sites | 3 central sites | stable | Low |
| `workflow::rpc::request::handler::RootCapability` | 43 references / 5 files | ~7 sites | not method-comparable | Medium |

The count method is stricter in this run because it counts references rather
than manually grouped match sites. The trend takeaway is unchanged: root
request/capability changes have the highest future amplification.

## 11. Refactor-Transient vs True-Drag Findings

| Signal | Raw Trend | Noise Classification | Adjusted Interpretation |
| --- | --- | --- | --- |
| `d81096f` touched 61 files | broad | structural cleanup sweep | Host tests moved into focused modules; high file count is not routine feature friction. |
| `5996340` touched 64 files | broad | ownership extraction | Broad now, likely lower future Canic ownership for helper-backed NNS/operator data. |
| host deployment-truth modules remain >4k LOC | persistent | true drag | Still the top host friction target despite test decomposition. |
| workflow direct storage type reference | one pre-fix crossing | remediated boundary pressure | Pool recycle metadata now goes through an ops-owned projection. |
| auth cleanup slices touched API/ops/workflow/storage/docs | broad | release stabilization | Expected for post-hard-cut auth cleanup, but future verifier changes should be narrower. |

## 12. Structural Drift Table

| Signal | Previous | Current | Delta | Risk |
| --- | ---: | ---: | ---: | --- |
| subsystem fan-in concentration | operator-heavy plus setup/build-heavy | host/deployment-truth plus auth/helper-heavy | shifted | Medium |
| top production module LOC pressure | deployment-truth/deploy heavy | deployment-truth/install-root heavy | stable | Medium |
| cross-subsystem imports | no confirmed breach | no direct workflow storage reference after fix | 0 | Low |
| policy-layer decision ownership | no confirmed drift | no confirmed drift in access/domain policy scans | 0 | Low |
| release/cleanup sweeps | visible | visible and test-heavy | stable | Low |

## 13. Synthetic Feature Simulation

| Synthetic Feature | Files Touched | Subsystems | Layers | Risk |
| --- | ---: | ---: | ---: | --- |
| new capability proof mode | 8-14 | dto, workflow capability, metrics, tests/docs | 4 | High |
| new RPC request variant | 10-18 | dto, API, workflow handler, replay, metrics, tests/docs | 5 | High |
| new delegated auth verifier rule | 8-16 | config/DTO, endpoint access, workflow runtime, ops verifier, tests/docs | 4-5 | High |
| new host deployment-truth report | 6-12 | host model/report/text/tests, optional CLI wrapper | 2-3 | Medium |
| new helper-backed operator command | 5-12 | CLI command/help/tests, host adapter/cache, scripts/docs | 2-3 | Medium |
| new host test module split | 2-8 | host tests/module exports | 1 | Low |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/workflow/rpc/request/handler/*` | `RootResponseWorkflow`, `RootCapability`, replay/authorize/execute modules | root capability changes cross request type, proof, replay, auth, execution, metrics, and tests | Medium |
| `crates/canic-core/src/workflow/pool/mod.rs` | recycle/replay helpers | recycle metadata now uses an ops-owned projection; module remains broad but boundary-clean after fix | Low |
| `crates/canic-core/src/access/expr/*` | `AccessExpr`, `BuiltinPredicate`, evaluator modules | central endpoint access expression surface; currently contained to access/value/log imports | Low |
| `crates/canic-core/src/workflow/rpc/capability/*` | capability envelope/proof/hash modules | capability proof and replay metadata changes touch DTO, workflow validation, metrics, and tests | Medium |
| `crates/canic-host/src/deployment_truth/*` | promotion/lifecycle/report/text/model families | host-owned deployment-truth changes remain broad and public through one support module | Medium |
| `crates/canic-host/src/install_root/mod.rs` | install orchestration/reporting | install, verification, local state, CLI-facing output, and deployment-truth receipts meet here | Medium |
| `crates/canic-cli/src/fleets/mod.rs` | fleet command parsing/rendering | large CLI command module with role lifecycle, adoption, config, and scaffold-adjacent concerns | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-host/src/deployment_truth/*` | model, report, text, lifecycle, promotion, authority, executor, receipt | 1 host subsystem, many domains | 2 | 7 |
| `crates/canic-host/src/install_root/mod.rs` | deployment_truth, release_set, icp, replica_query, canic_core DTO/config helpers | 3 | 3 | 6 |
| `crates/canic-core/src/workflow/rpc/request/handler/*` | DTO request/response, replay, metrics, authorization, execution | 4 | 3 | 6 |
| `crates/canic-cli/src/fleets/mod.rs` | CLI parse/render, host release/adoption/config helpers | 3 | 2 | 6 |
| `crates/canic-core/src/workflow/pool/mod.rs` | pool workflow, ops pool metadata, replay codecs, metrics | 4 | 2 | 5 |

## Amplification Drivers

| Commit | Feature Slice | Slice Type | Files Touched | Subsystems | Density | CAF | Risk |
| --- | --- | --- | ---: | --- | ---: | ---: | --- |
| `5996340` | NNS/operator helper extraction | feature_slice | 64 | CLI, host, helper crates, tests, scripts, docs, workspace | 9.14 | 21 | Medium |
| `4f7e76e` | auth config/schema/runtime cleanup | feature_slice | 55 | API, config, ops, storage, workflow, docs/tests | 9.17 | 24 | Medium |
| `2c1dd86` | auth verifier terminology/runtime cleanup | feature_slice | 27 | API, ops, workflow, tests/docs | 6.75 | 12 | Medium |
| `0ec1fb8` | topology output and auth-runtime follow-up | feature_slice | 26 | CLI, host, core workflow, docs/tests | 5.20 | 15 | Medium |
| `28ad0e3` | topology command/report cleanup | feature_slice | 9 | CLI, host, docs, tests | 2.25 | 8 | Low |
| `d81096f` | host tests/module cleanup | release_sweep | 61 | host, tests, docs | 20.33 | 3 | Low |

Most impacted files and modules:

- `crates/canic-host/src/deployment_truth/promotion.rs`
- `crates/canic-host/src/deployment_truth/lifecycle.rs`
- `crates/canic-host/src/install_root/mod.rs`
- `crates/canic-cli/src/fleets/mod.rs`
- `crates/canic-core/src/workflow/ic/icp_refill/mod.rs`
- `crates/canic-core/src/workflow/rpc/request/handler/*`
- `crates/canic-core/src/workflow/pool/mod.rs`
- `crates/canic-core/src/api/auth/mod.rs`
- `crates/canic-core/src/ops/auth/delegated/*`
- `crates/canic-core/src/workflow/runtime/auth/mod.rs`

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| workflow/storage type crossing | `workflow/pool/mod.rs`, `ops/storage/pool/mod.rs` | fixed by replacing the direct `CanisterRecord` helper parameter with `PoolRegistrationMetadata`; rerun scan found no direct workflow/access/api storage references | Low |
| host deployment-truth gravity well | `deployment_truth/promotion.rs`, `deployment_truth/lifecycle.rs` | 5279 and 4017 LOC production modules remain largest host files | Medium |
| helper extraction breadth | `5996340` | 64 files across CLI, host, helper crates, scripts, docs, tests, workspace manifests | Medium |
| auth cleanup breadth | `4f7e76e` and `2c1dd86` | auth config/verifier cleanup crossed API, config, ops, storage, workflow, docs/tests | Medium |
| host test decomposition | `d81096f` | 61-file sweep primarily under host test modules | Low |

## Verification Readout

Commands passed:

- `cargo test --locked -p canic-core --lib workflow::pool -- --nocapture`
  - 10 passed.
- `cargo clippy --locked -p canic-core --lib -- -D warnings`
  - passed.
- `cargo fmt --all -- --check`
  - passed.
- `git diff --check`
  - passed.
- `cargo test --locked -p canic-host --lib -- --nocapture`
  - 659 passed.
- `cargo test --locked -p canic-cli --lib -- --nocapture`
  - 444 passed.

Commands used as source scans:

- `git rev-parse --short HEAD`
- `git log --name-only -n 30 -- crates/canic-core/src crates/canic-cli/src crates/canic-host/src crates/canic-tests/tests docs/audits docs/design docs/changelog`
- `git show --name-only --format= <commit>`
- `git show --name-only --format= <commit> | sed '/^$/d' | wc -l`
- `rg 'crate::storage|crate::model|canic_core::storage|canic_core::model' crates/canic/src crates/canic-core/src/workflow crates/canic-core/src/access crates/canic-core/src/api -g '*.rs'`
- `rg 'crate::workflow|canic_core::workflow' crates/canic-core/src/ops crates/canic-core/src/storage crates/canic-core/src/access -g '*.rs'`
- `rg '^use ' crates/canic-core/src crates/canic-cli/src crates/canic-host/src -g '*.rs'`
- `find crates/canic-core/src crates/canic-cli/src crates/canic-host/src -type f -name '*.rs' -exec wc -l {} + | sort -nr | sed -n '1,40p'`
- `rg -n 'enum Request|enum Response|enum CapabilityProof|enum BuiltinPredicate|enum RootCapability|enum ReplayPreflight|enum RootPreflight' crates/canic-core/src crates/canic/src -g '*.rs'`
- `rg -l 'RootCapabilityEnvelopeV1|NonrootCyclesCapabilityEnvelopeV1|RootCapabilityResponseV1|NonrootCyclesCapabilityResponseV1|CapabilityProof|CapabilityService' crates canisters fleets -g '*.rs'`
- `rg -l 'RootCapability::' crates/canic-core/src crates/canic/src -g '*.rs'`
- `rg -l 'Request::' crates/canic-core/src crates/canic/src -g '*.rs'`

## Follow-up Actions

1. Keep host deployment-truth decomposition as the main friction target before
   adding more promotion/lifecycle report families. The first follow-up slices
   moved external lifecycle error/digest helpers plus promotion error/request,
   digest, identity, policy, and guard helpers out of the largest modules, with
   lifecycle and promotion internals under directory modules. The
   promotion/lifecycle gravity wells remain open.
2. Treat new root capability request variants and delegated auth verifier rules
   as coordinated cross-layer slices with DTO, workflow, ops, metrics, tests,
   and docs planned together.
3. Treat broad helper-backed operator command work as an integration slice:
   CLI surface, host adapter, install scripts, helper version checks, docs, and
   tests should move together rather than as follow-up fragments.
4. Keep the broader workflow-purity watchpoint for future record carriers and
   Candid codecs. The explicit `workflow/pool` storage-record crossing was
   closed before this report was finalized, and the workflow/API registry-record
   projection follow-up was closed by the next cleanup slice.
