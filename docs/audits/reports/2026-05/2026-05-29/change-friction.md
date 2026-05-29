# Change Friction Audit - 2026-05-29

## Report Preamble

- Scope: `crates/canic`, `crates/canic-core`, `crates/canic-control-plane`,
  `crates/canic-host`, `crates/canic-cli`, `crates/canic-backup`,
  `crates/canic-wasm-store`, `crates/canic-testing-internal`,
  `crates/canic-tests`, `fleets/**`, `canisters/**`, active docs, and recent
  `0.48.x` feature slices.
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-10/change-friction.md`
- Code snapshot identifier: `613b4a76`
- Method tag/version: `change-friction-current`
- Comparability status: partially comparable: the method is the same, but the
  baseline sampled the broad `0.33.x` ICP CLI hard cut while this run samples
  the narrower `0.48.x` setup simplification, demo sharding, auth freshness,
  role-artifact resolution, and audit cleanup line.
- Exclusions applied: generated `target/**`, `.icp/**`, release-version-only
  bookkeeping, and generated audit report artifacts outside this report.
- Notable methodology changes vs baseline: sibling `ic-testkit` is evaluated
  as an external generic testkit seam; release-version commits are visible but
  not counted as routine feature slices.

## 1. Velocity Risk Index

Velocity Risk Index: **4 / 10**.

The score improved from the `5 / 10` 2026-05-10 baseline. Routine 0.48 slices
are materially narrower than the 0.33 hard-cut samples, and no cross-layer
leakage was confirmed. The remaining pressure is concentrated in host/CLI
deployment coordination files and setup/build role resolution.

| Metric | Previous | Current | Delta |
| --- | ---: | ---: | ---: |
| Velocity Risk Index | 5 | 4 | -1 |
| Cross-layer leakage crossings | 0 | 0 | 0 |
| Avg files touched per sampled routine feature slice | 63.17 | 28.43 | -34.74 |
| p95 sampled routine files touched | 88 | 65 | -23 |
| Top gravity-well fan-in | 5 | 4 | -1 |

Current averages use sampled commits `f39bbf16`, `0dc7f946`, `48a0f5e6`,
`6329a8f2`, `77520afa`, `2c4aa218`, and `a43e27f8`. The Rust `1.96.0`
sweep (`17f1fb36`) is tracked separately as a release/toolchain sweep.

## 2. Revised CAF + Locality Summary

| Feature | Slice Type | Files Modified | Subsystems | Layers | Flow Axes | Revised CAF | Density | ELS | Feature Locality Index | Containment Score | Risk |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `f39bbf16` derived singleton topology and subnet config cleanup | feature_slice | 24 | 6 | 4 | 3 | 18 | 4.00 | 0.42 | 0.17 | 0.75 | Medium |
| `0dc7f946` package-metadata role source and startup/build hard cut | feature_slice | 65 | 7 | 4 | 3 | 21 | 9.29 | 0.38 | 0.12 | 0.88 | Medium |
| `48a0f5e6` setup/docs pass plus startup scaffold cleanup | feature_slice | 33 | 5 | 3 | 2 | 10 | 6.60 | 0.52 | 0.18 | 0.62 | Medium |
| `6329a8f2` demo sharding and `ic-testkit` adoption | feature_slice | 30 | 4 | 2 | 2 | 8 | 7.50 | 0.70 | 0.33 | 0.50 | Low |
| `77520afa` artifact boundary and auth audience docs | feature_slice | 12 | 4 | 3 | 3 | 12 | 3.00 | 0.42 | 0.17 | 0.50 | Medium |
| `2c4aa218` auth freshness expiry alignment | feature_slice | 17 | 3 | 3 | 2 | 6 | 5.67 | 0.18 overall / 1.00 code-only | 0.18 overall / 0.33 code-only | 0.38 | Low |
| `a43e27f8` scoped role artifact resolution | feature_slice | 18 | 4 | 3 | 3 | 12 | 4.50 | 0.39 | 0.22 | 0.50 | Medium |
| `17f1fb36` Rust `1.96.0` and MSRV/tooling sweep | release_sweep | 56 | 7 | 3 | 1 | 7 | 8.00 | 0.34 | 0.23 | 0.88 | Low |

Interpretation:

- The largest routine slice is the `0.48.1` hard cut that removed package-name
  inference and old root/build macro variants. That breadth is expected for a
  public setup-surface break.
- The narrowest functional code slice is auth freshness: only three runtime
  files changed, with docs/audit/status accounting for most of the file count.
- Demo sharding adoption stayed mostly in test harness and fleet/demo surfaces;
  it did not pull deployment-truth or control-plane runtime into the demo.

## 3. Edit Blast Radius Summary

| Metric | Current | Previous | Delta |
| --- | ---: | ---: | ---: |
| average files touched per sampled routine feature slice | 28.43 | 63.17 | -34.74 |
| median files touched | 24 | 54 | -30 |
| p95 files touched | 65 | 88 | -23 |

Status: slice-sampled.

0.48 routine work is less broad than the 0.33 hard cut, but setup changes still
touch facade macros, build support, scaffold output, canister manifests, docs,
and workspace governance together.

## 4. Boundary Leakage Trend Table

| Boundary | Import Crossings | Previous | Delta | Risk |
| --- | ---: | ---: | ---: | --- |
| endpoint macros -> model/storage direct references | 0 | 0 | 0 | Low |
| workflow -> model/storage direct references | 0 confirmed | 0 | 0 | Low |
| workflow -> ops-mediated storage access | 1 notable site | N/A | N/A | Low |
| policy/domain -> ops/runtime side effects | 0 confirmed | 0 | 0 | Low |
| access/auth endpoint boundary -> ops/storage cleanup | existing intentional boundary | existing | 0 | Medium pressure |
| auth/capability DTOs leaking into model/storage ownership | 0 confirmed | 0 | 0 | Low |
| sibling `ic-testkit` -> Canic runtime semantics | 0 | N/A | N/A | Low |

Evidence:

- `crates/canic/src/macros/start.rs` and `crates/canic/src/macros/mod.rs` route
  through facade API paths, not storage/model internals.
- `crates/canic-core/src/workflow/rpc/request/handler/nonroot_cycles.rs`
  reads `AppStateOps::cycles_funding_enabled()`, which is ops-mediated state
  access, not a storage bypass.
- `crates/canic-core/src/access/auth/mod.rs` still performs delegated-session
  cleanup through `AuthStateOps`; this is endpoint auth-boundary pressure and
  should not spread to general policy/domain code.
- `rg "canic|canic_testing_internal"` against sibling `../ic-testkit` found no
  Canic dependency in the prior module-structure run; this change-friction run
  found Canic-specific fixtures only in `canic-testing-internal` and tests.

## 5. Change Multiplier Matrix

| Feature Axis | Endpoints | Workflow | Policy | Ops | Model/Storage | Subsystem Count |
| --- | --- | --- | --- | --- | --- | ---: |
| package metadata role source | yes | no | no | yes | no | 2 |
| root-vs-non-root startup dispatch | yes | yes | no | yes | no | 3 |
| derived singleton topology | no | yes | yes | yes | yes | 4 |
| scoped role artifact resolution | no | no | no | yes | no | 1 |
| delegated expiry boundary | no | yes | yes | yes | no | 3 |
| demo sharding walkthrough | yes | no | no | no | no | 1 |

| Candidate Feature | Axes Involved | Subsystem Count | Friction |
| --- | --- | ---: | --- |
| new package metadata build/start contract | metadata role source, startup dispatch | 3 | Medium |
| new topology derivation rule | topology, config validation, provisioning, storage index | 4 | Medium |
| new root capability request variant | request type, capability proof mode, replay state, metrics | 5 | High |
| new demo-only canister role | canister manifest, fleet config, docs/test harness | 2 | Low |

## 6. Enum Shock Radius Hotspots

| Enum | Variants | Switch Sites | Modules Using Enum | Switch Density | Subsystems | Shock Radius | Risk |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `dto::rpc::Request` | 6 | ~12 | ~7 | ~1.7 | 4 | ~41 | Medium |
| `dto::rpc::Response` | 6 | ~8 | ~5 | ~1.6 | 4 | ~38 | Medium |
| `dto::capability::CapabilityProof` | 3 | ~8 | ~6 | ~1.3 | 3 | ~12 | Medium |
| `access::expr::BuiltinPredicate` | 4 top-level families | 3 central evaluator/constructor sites | 2 | ~1.5 | 1 | ~6 | Low |
| `workflow::rpc::request::handler::RootCapability` | 6 | ~7 | ~5 | ~1.4 | 2 | ~17 | Medium |

No enum met the structural-hotspot rule of `switch_density > 3` with
`subsystems >= 4`. The root capability request family remains the highest
future-friction surface because a new variant tends to require DTO, workflow
authorization, workflow execution, replay/capability proof handling, metrics,
tests, and Candid/docs follow-up.

## 7. Gravity-Well Growth + Edit Frequency

| Module | LOC | LOC Delta | Fan-In | Fan-In Delta | Domains | Edit Frequency (30d) | Risk |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `crates/canic-cli/src/deploy/mod.rs` | 6594 | N/A | 4 ownership roots | N/A | deploy UX, root verify, promotion, lifecycle | high | Medium |
| `crates/canic-host/src/deployment_truth/promotion.rs` | 5279 | N/A | public through `deployment_truth` | N/A | promotion, artifacts, policy, provenance | high | Medium |
| `crates/canic-host/src/deployment_truth/lifecycle.rs` | 4017 | N/A | public through `deployment_truth` | N/A | external lifecycle, consent, verification | high | Medium |
| `crates/canic-host/src/install_root/mod.rs` | 3009 | N/A | CLI/install/deploy consumers | N/A | install, verification, local state | high | Medium |
| `crates/canic-core/src/api/ic/canic.rs` | 1085 | N/A | facade/API consumers | N/A | internal calls, tokens, protected endpoints | medium | Medium |
| `crates/canic-core/src/workflow/rpc/request/handler/*` | split files plus large tests | N/A | API/ops/workflow/metrics | N/A | capability, replay, authorization, execution | medium | Medium |

The current run cannot compute path-comparable LOC deltas because the baseline
was pre-0.48 and pre-current deployment-truth pressure. The pressure is real,
but not a confirmed regression inside this `.10` slice.

## 8. Subsystem Independence Scores

| Subsystem | Internal Imports | External Imports | LOC Signal | Independence | Adjusted Independence | Risk |
| --- | ---: | ---: | --- | ---: | ---: | --- |
| `canic-core::domain/policy` | high local value/view use | no ops/runtime side effects confirmed | moderate | high | high | Low |
| `canic-core::workflow` | workflow + ops/API references | ops/runtime metrics and IC/provision helpers | large | medium | medium | Medium |
| `canic-core::ops` | storage/infra/support references | no workflow dependency confirmed | large | medium-high | medium-high | Low |
| `canic-cli` command modules | CLI-local parse/render plus host/backup support | host/backup/core package APIs | large | medium | medium | Medium |
| `canic-host::deployment_truth` | host-local model/validation/text modules | `canic-core::bootstrap::parse_config_model` and filesystem/process support | very large | medium | medium | Medium |
| `canic-testing-internal` | harness-local modules | `ic-testkit`, `canic`, `canic-core` | moderate | medium | medium | Low |

## 9. Independent-Axis Growth Warnings

| Operation | Axes | Axis Count | Independent Axes | Previous Independent Axes | Delta | Risk |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| `canic::start!()` dispatch | package role, root/non-root lifecycle, optional init hook | 3 | 2 | N/A | N/A | Low |
| root capability execution | request family, proof mode, replay state, role/subnet context, metrics outcome | 5 | 4 | N/A | N/A | Medium |
| scoped role artifact resolution | selected canister root, exact package metadata role, build profile/network | 3 | 2 | N/A | N/A | Medium |
| delegated auth verification | audience, expiry, replay/use, root key, signer proof | 5 | 4 | N/A | N/A | Medium |
| deployment-truth promotion | artifact source, materialization, policy, provenance, receipt state | 5 | 4 | N/A | N/A | Medium |

## 10. Decision Surface Size Trends

| Enum | Decision Sites | Previous | Delta | Risk |
| --- | ---: | ---: | ---: | --- |
| `dto::rpc::Request` | ~12 | N/A | N/A | Medium |
| `dto::rpc::Response` | ~8 | N/A | N/A | Medium |
| `dto::capability::CapabilityProof` | ~8 | N/A | N/A | Medium |
| `access::expr::BuiltinPredicate` | 3 central sites | N/A | N/A | Low |
| `workflow::rpc::request::handler::RootCapability` | ~7 | N/A | N/A | Medium |

The practical decision-surface warning is root capability growth. Adding a
request family is not isolated to one enum: it also touches root capability
mapping, authorization, execution, replay handling, metrics, tests, and public
Candid/API docs.

## 11. Refactor-Transient vs True-Drag Findings

| Signal | Raw Trend | Noise Classification | Adjusted Interpretation |
| --- | --- | --- | --- |
| `0.48.1` setup hard cut touched 65 files | broad | intentional setup-surface break | not routine feature friction, but future startup/build changes should be narrower now that metadata is the role source. |
| `17f1fb36` touched 56 files | broad | release/toolchain sweep | not routine architecture friction; mostly tests/assertion/toolchain cleanup. |
| `0.48.6` expiry alignment touched 17 files but only 3 runtime files | medium | docs/audit/status amplification | code slice was contained; audit/reporting overhead dominates total count. |
| demo sharding touched 30 files | medium | test/fleet example slice | contained to demo/test harness and not used in main test flow. |
| host deployment-truth files remain large | persistent | true drag | keep as 0.49 continuation target, not an incidental audit artifact. |

## 12. Structural Drift Table

| Signal | Previous | Current | Delta | Risk |
| --- | ---: | ---: | ---: | --- |
| subsystem fan-in concentration | operator-heavy | operator-heavy plus setup/build-heavy | stable | Medium |
| top 3 production modules LOC share | N/A | dominated by `deploy`, `deployment_truth/promotion`, and `deployment_truth/lifecycle` | N/A | Medium |
| cross-subsystem imports | no breach | no breach | 0 | Low |
| policy-layer decision ownership | no confirmed drift | no confirmed drift; domain policy remains side-effect free | 0 | Low |

## 13. Synthetic Feature Simulation

| Synthetic Feature | Files Touched | Subsystems | Layers | Risk |
| --- | ---: | ---: | ---: | --- |
| new capability proof mode | 8-14 | dto, api, workflow, ops/metrics, tests/docs | 4 | High |
| new RPC request variant | 10-18 | dto, api, workflow, ops, metrics, Candid/tests/docs | 5 | High |
| new policy rule | 3-7 | domain/access plus tests, sometimes workflow call site | 2-3 | Medium |
| new lifecycle timer workflow | 4-8 | lifecycle/workflow/runtime ops/macros/docs | 3 | Medium |
| new ordinary demo canister endpoint | 2-5 | fleet canister, docs/tests if exposed | 1-2 | Low |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/workflow/rpc/request/handler/*` | `RootResponseWorkflow`, `RootCapability`, replay/authorize/execute modules | root capability changes cross request type, proof, replay, auth, execution, metrics, and tests | Medium |
| `crates/canic-core/src/access/expr/mod.rs` | `AccessExpr`, `BuiltinPredicate`, `AsyncAccessPredicate` | central endpoint access expression surface; currently contained to access/value/log imports | Low |
| `crates/canic-core/src/api/rpc/capability/*` | `CapabilityProof`, `RootCapabilityProof`, verifiers | capability proof mode changes touch DTO, API validation, metrics, and tests | Medium |
| `crates/canic-host/src/deployment_truth/*` | promotion/lifecycle/report/text/model families | host-owned deployment-truth changes remain broad and public through one support module | Medium |
| `crates/canic-cli/src/deploy/mod.rs` | deploy command dispatch/rendering | private CLI module coordinates many deployment-truth command families | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-cli/src/deploy/mod.rs` | `cli`, `canic_host::deployment_truth`, `canic_host::install_root`, host config/build | 4 | 2 | 6 |
| `crates/canic-host/src/deployment_truth/mod.rs` | authority, executor, lifecycle, model, multi, observe, plan, promotion, receipt, report, root, text | 12 | 1 | 6 |
| `crates/canic-core/src/workflow/rpc/request/handler/mod.rs` | workflow handler modules, DTO request/response, metrics, replay, authorization | 5 | 3 | 6 |
| `crates/canic-core/src/access/expr/mod.rs` | `access`, `ids`, `log`, `cdk` | 4 | 1 | 3 |
| `crates/canic-testing-internal/src/pic/mod.rs` | artifacts, attestation, audit, canic, delegation, lifecycle, root | 7 | 1 | 3 |

## Amplification Drivers

| Commit | Feature Slice | Slice Type | Files Touched | Subsystems | Density | CAF | Risk |
| --- | --- | --- | ---: | --- | ---: | ---: | --- |
| `0dc7f946` | package metadata role source and startup/build hard cut | feature_slice | 65 | facade/build, scaffold, fleets, test canisters, docs | 9.29 | 21 | Medium |
| `48a0f5e6` | setup/docs pass plus startup scaffold cleanup | feature_slice | 33 | facade/build, docs, scaffold, fleet roots | 6.60 | 10 | Medium |
| `6329a8f2` | demo sharding and `ic-testkit` adoption | feature_slice | 30 | fleets, tests, internal test harness, docs | 7.50 | 8 | Low |
| `f39bbf16` | derived singleton topology | feature_slice | 24 | config, provisioning, host release/install tests, fleets, docs | 4.00 | 18 | Medium |
| `a43e27f8` | scoped role artifact resolution | feature_slice | 18 | host, test harness, fleet test canister, docs/audits | 4.50 | 12 | Medium |

Most impacted files:

- `crates/canic/src/macros/start.rs`
- `crates/canic/src/macros/build.rs`
- `crates/canic/src/build_support/config.rs`
- `crates/canic-cli/src/scaffold/mod.rs`
- `crates/canic-host/src/canister_build.rs`
- `crates/canic-host/src/release_set/mod.rs`
- `crates/canic-core/src/config/schema/subnet/mod.rs`
- `crates/canic-core/src/workflow/ic/provision/indexes.rs`
- `crates/canic-core/src/api/rpc/capability/grant.rs`
- `crates/canic-core/src/ops/auth/verify/attestation.rs`

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| setup-surface breadth | `canic` macros/build support plus scaffold/fleets | `0dc7f946` touched 65 files across facade, build support, canisters, fleets, docs, and tests | Medium |
| root capability shock radius | `dto/rpc.rs`, `api/rpc/capability/*`, `workflow/rpc/request/handler/*` | enum/reference scan shows `Request`, `Response`, `CapabilityProof`, and `RootCapability` decisions across DTO/API/workflow/ops/tests | Medium |
| deployment-truth gravity well | `crates/canic-host/src/deployment_truth/*` | largest production files include `promotion.rs = 5279`, `lifecycle.rs = 4017`, `text.rs = 2613`, `report.rs = 2416` | Medium |
| CLI deploy gravity well | `crates/canic-cli/src/deploy/mod.rs` | `6594` LOC private module importing broad `canic_host::deployment_truth` surface | Medium |
| generic testkit containment | `../ic-testkit` vs `canic-testing-internal` | Canic-specific setup remains in `canic-testing-internal`; `ic-testkit` remains generic | Low |

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `git rev-parse --short HEAD` | PASS | captured snapshot `613b4a76`. |
| `git log --oneline -n 30` | PASS | selected recent 0.48 implementation samples. |
| `git show --stat --name-only --format=fuller <commit>` | PASS | sampled `613b4a76`, `89cccc85`, `a43e27f8`, `3e96578d`, `2c4aa218`, `77520afa`, `17f1fb36`, `6329a8f2`, `48a0f5e6`, `0dc7f946`, `2935e40f`, `2beca8d1`, `a600541f`, `f39bbf16`, and `940c95ab`. |
| `git show --name-only --format= <commit> \| wc -l` | PASS | counted touched files for sampled feature slices. |
| `rg '^use ' crates/ -g '*.rs'` | PASS | import scan used for hotspot and boundary review. |
| `rg 'crate::workflow|crate::ops|crate::api|crate::policy|crate::storage|canic_core::workflow|canic_core::ops|canic_core::storage' crates/ -g '*.rs' -n` | PASS | boundary crossing scan; no High/Critical breach confirmed. |
| `rg 'enum \|pub struct\|pub fn\|impl ' crates/canic-core/src crates/canic/src crates/canic-host/src crates/canic-cli/src crates/canic-backup/src -g '*.rs' -n` | PASS | enum/public surface scan. |
| `git log --name-only -n 20 -- crates/` | PASS | repeat-touch and slice-selection support. |
| `find crates canisters fleets -type f -name '*.rs' -exec wc -l {} + \| sort -nr \| sed -n '1,40p'` | PASS | gravity-well LOC scan. |
| `rg "enum Request|enum Response|enum CapabilityProof|enum BuiltinPredicate|enum RootCapability|enum ReplayPreflight|enum RootPreflight" crates/canic-core/src -n` | PASS | enum definition scan. |
| `rg "Request::|Response::|CapabilityProof::|BuiltinPredicate::|RootCapability::|ReplayPreflight::|RootPreflight::" crates/canic-core/src crates/canic/src -g '*.rs' -n` | PASS | decision-site scan. |

## Follow-up Actions

1. Setup maintainers: treat metadata-driven startup/build changes as public
   setup-surface changes and keep future edits behind `build_support`,
   scaffold, and macro tests instead of scattering follow-ups.
2. Root capability maintainers: before adding a new root request or proof mode,
   budget DTO, API validation, workflow authorization/execution, replay,
   metrics, Candid/docs, and tests as one coordinated slice.
3. Host/CLI maintainers: split `canic-cli/src/deploy/mod.rs` and
   `canic-host/src/deployment_truth/*` before adding more deployment-truth
   command families.
4. Testkit maintainers: keep generic `ic-testkit` helpers free of Canic
   topology/bootstrap/readiness semantics; keep those in
   `canic-testing-internal`.
