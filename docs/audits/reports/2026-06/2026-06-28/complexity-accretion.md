# Complexity Accretion Audit - 2026-06-28

## Report Preamble

- Scope: `crates/canic-core/src/**`
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-19/complexity-accretion.md`
- Code snapshot identifier: `37ac9f8e` plus dirty root-renewal cleanup and
  audit-report docs
- Method tag/version: `Method V4.5 / root-renewal scheduling/retrieval split refresh`
- Comparability status: `partially comparable`. File, logical LOC, enum,
  branch-density, large-file, and concept-spread counts are comparable with the
  June 19 method. Capability mention counts are noisy because the current code
  snapshot includes the 0.74.13 module split and retained auth-renewal tests,
  but the enum and switch-site surfaces remain comparable.
- Auditor: `codex`
- Run timestamp (UTC): `2026-06-28T13:17:08Z`
- Branch: `main`
- Worktree: `dirty`

## Audit Definition Maintenance

No recurring audit definition changes were required for this run.

## Executive Summary

- Risk Score: **3 / 10**.
- Delta summary: total runtime files grew `486 -> 507`, runtime logical LOC
  grew `56641 -> 63865`, and non-test files above `600` logical LOC grew
  `6 -> 8` after cleanup.
- Positive structural signal: root-managed renewal no longer has a production
  file above the audit's `600` logical LOC threshold. Template/provisioner
  facade, scheduler, retrieval/install-window validation, install outcome
  recording, identity, view conversion, and tests now have separate modules.
- Residual pressure: `api/blob_storage.rs` remains a `1005` logical LOC facade,
  while older delegated-token, runtime-auth prepare, nonroot-cycles, and
  support-seam modules remain branch-density or large-file watchpoints.
- Follow-up required: no immediate correctness blocker. To lower this audit
  below `3 / 10`, the next cleanup should target the blob-storage API facade or
  one of the older auth/cycles hubs.

## Baseline Capture

| Metric | Previous | Current | Delta |
| ---- | ----: | ----: | ----: |
| Total runtime files in scope | 486 | 507 | +21 |
| Runtime logical LOC | 56641 | 63865 | +7224 |
| Files >= 600 LOC | 12 | 14 | +2 |
| Non-test runtime files | 460 | 490 | +30 |
| Non-test runtime logical LOC | 48965 | 56295 | +7330 |
| Non-test files >= 600 LOC | 6 | 8 | +2 |
| Capability mentions | 25 files | 40 files | +15 |
| Capability decision owners | 14 `workflow` files, 4 `ops` files, 1 `api` file | 16 `workflow` files, 9 `ops` files, 1 `api` file | broader raw fan-in |
| Capability execution consumers | 4 `ops` files | 9 `ops` files | +5 raw mentions |
| Capability plumbing modules | 2 `dto` files, 4 `replay_policy` files | 10 `dto`/`replay_policy` files | +4 raw mentions |

## Subsystem Map

| Subsystem | Files | Logical LOC |
| --- | ---: | ---: |
| `access` | 10 | 1513 |
| `api` | 35 | 2549 |
| `bootstrap` | 2 | 807 |
| `cdk` | 16 | 714 |
| `config` | 12 | 2842 |
| `dispatch` | 2 | 105 |
| `domain` | 24 | 1985 |
| `dto` | 29 | 1666 |
| `format` | 1 | 87 |
| `ids` | 8 | 297 |
| `infra` | 17 | 1184 |
| `ingress` | 2 | 76 |
| `lifecycle` | 7 | 379 |
| `memory` | 6 | 599 |
| `model` | 4 | 573 |
| `ops` | 152 | 26384 |
| `replay-policy` | 12 | 1214 |
| `root` | 8 | 894 |
| `storage` | 29 | 3877 |
| `test-support` | 10 | 1200 |
| `view` | 8 | 129 |
| `workflow` | 113 | 14791 |

## Variant Surface Growth

| Enum | Variants | Previous | Delta | Variant Velocity | Switch Sites | Branch Multiplier | Enum Density | Mixed Domain? | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ----: | ----: | ---- | ---- |
| `dto::rpc::Request` | 4 | 4 | 0 | 0 | 7 production flow sites | 28 | 8 / 507 = 0.02 | Yes | Medium |
| `dto::rpc::Response` | 4 | 4 | 0 | 0 | 5 production flow sites | 20 | 20 / 507 = 0.04 | Yes | Medium |
| `dto::rpc::RequestFamily` | 4 | 4 | 0 | 0 | 2 production flow sites | 8 | 2 / 507 = 0.00 | Yes | Low |
| `dto::rpc::RootCapabilityCommand` | 4 | 4 | 0 | 0 | 1 conversion site plus replay manifest coverage | 4 | 2 / 507 = 0.00 | Yes | Low |
| `dto::capability::CapabilityProof` | 1 | 1 | 0 | 0 | 2 production flow sites | 2 | 5 / 507 = 0.01 | No | Low |
| `dto::capability::CapabilityService` | 1 | 1 | 0 | 0 | 2 guard sites | 2 | 4 / 507 = 0.01 | No | Low |
| `access::expr::BuiltinPredicate` | 4 top-level, 14 evaluator arms | 4 top-level, 14 evaluator arms | 0 | 0 | 1 central dispatch site | 14 evaluator arms | 2 / 507 = 0.00 | Yes | Medium |
| `workflow::rpc::request::handler::RootCapability` | 4 | 4 | 0 | 0 | 4 production flow sites | 16 | 5 / 507 = 0.01 | Yes | Medium |
| `ops::runtime::metrics::RootCapabilityMetricKey` | 4 | 4 | 0 | 0 | metrics routing | 4 | 6 / 507 = 0.01 | No | Low |
| `ops::runtime::metrics::RootCapabilityMetricEventType` | 5 | 5 | 0 | 0 | metrics routing | 5 | 1 / 507 = 0.00 | No | Low |
| `ops::runtime::metrics::RootCapabilityMetricOutcome` | 9 | 9 | 0 | 0 | metrics routing | low | 8 / 507 = 0.02 | Yes | Medium |
| `dto::auth::ActiveDelegationProofStatus` | 4 | 4 | 0 | 0 | 1 status builder | 4 | 3 / 507 = 0.01 | Yes | Medium |
| `dto::auth::RootDelegationProofInstallOutcome` | 6 | 6 | 0 | 0 | 1 install workflow plus recorders | 6 | 5 / 507 = 0.01 | Yes | Medium |
| `error::InternalErrorClass` | 6 | 6 | 0 | 0 | public error classifier sites | 36 | 14 / 507 = 0.03 | Yes | Medium |
| `infra::InfraError` | 1 | 1 | 0 | 0 | one conversion path | 1 | 9 / 507 = 0.02 | No | Low |

Enum surfaces did not grow. The complexity increase is therefore not
variant-driven; it is a hub-size and cross-layer concept-spread issue.

## Execution Branching Pressure

| Function / Area | Module | Branch Layers | Match Depth | Domains Mixed | Axis Coupling Index | Previous Branch Layers | Delta | Risk |
| ---- | ---- | ----: | ----: | ----: | ----: | ----: | ----: | ---- |
| renewal scheduler | `ops/auth/delegation/root_issuer_renewal/schedule.rs` | 4 | 2 | 3 | 12 | N/A | N/A | Medium after split |
| renewal retrieval/install gate | `ops/auth/delegation/root_issuer_renewal/retrieval.rs` | 4 | 2 | 3 | 12 | N/A | N/A | Medium after split |
| renewal install outcomes | `ops/auth/delegation/root_issuer_renewal/install.rs` | 4 | 2 | 3 | 12 | N/A | N/A | Medium after split |
| delegated-token prepare | `workflow/runtime/auth/prepare/mod.rs` | 5 | 2 | 4 | 20 | 5 | 0 | High |
| delegated-token verifier | `ops/auth/delegated/verify.rs` | 5 | 1 | 4 | 20 | 5 | 0 | Medium |
| blob-storage billing/status facade | `api/blob_storage.rs` | 4 | 2 | 4 | 16 | N/A | N/A | Medium |
| root request handler | `workflow/rpc/request/handler/*` | 4 | 2 | 4 | 16 | 4 | 0 | Medium |
| nonroot cycles handler | `workflow/rpc/request/handler/nonroot_cycles.rs` | 5 | 2 | 4 | 20 | 5 | 0 | High |
| root capability proof flow | `workflow/rpc/capability/*` | 3 | 1 | 2 | 6 | 3 | 0 | Low |

The follow-up split removed the remaining renewal parent hotspot. Scheduling
now owns due-template selection and prepare persistence; retrieval owns
proof-batch get validation, install-window validation, and batch pruning.

## Execution Path Multiplicity

| Operation | Axes Used | Axis Cardinalities | Theoretical Space | Removed Combinations | Effective Flows | Previous Effective Flows | Delta | Shared Core? | Risk |
| ---- | ---- | ---- | ----: | ----: | ----: | ----: | ----: | ---- | ---- |
| `response_capability_v1` | family, proof mode, replay, policy, caller topology | `4 x 1 x 4 x 2 x 2` | 64 | 54 illegal/short-circuit paths | 10 | 10 | 0 | Yes | Medium |
| `cycles` | family, replay, policy, funding state, caller topology | `1 x 4 x 2 x 3 x 2` | 48 | 39 illegal/short-circuit paths | 9 | 9 | 0 | Yes | Medium |
| `prepare_delegated_token` | auth lifecycle, active proof, issuer policy, replay metadata, caller binding | `3 x 3 x 2 x 3 x 2` | 108 | 96 invalid or terminal paths | 12 | 12 | 0 | Yes | High |
| `role_attestation_prepare/get` | attestation phase, key availability, replay state, caller topology | `2 x 3 x 3 x 2` | 36 | 29 terminal paths | 7 | 7 | 0 | Yes | Medium |
| `root_delegation_proof_batch_prepare/get/install` | renewal phase, issuer policy, replay metadata, proof availability, install outcome | `3 x 3 x 3 x 3 x 6` | 486 | 468 invalid/terminal combinations | 18 | N/A | N/A | Yes | Medium after split |
| `active_delegation_proof_install/status` | active proof lifecycle, issuer binding, expiry state, caller topology | `2 x 2 x 4 x 2` | 32 | 25 terminal paths | 7 | 7 | 0 | Yes | Medium |

No root capability flow count increased. Root-managed renewal remains the
largest active auth lifecycle, but prepare, get, install-window, and install
outcome ownership are now split across focused ops modules.

## Cross-Cutting Concern Spread

| Concept | Decision Owners | Execution Consumers | Plumbing Modules | Total Modules | Semantic Layers | Transport Layers | Decision Concentration | Concept Fragmentation | Risk |
| ---- | ---- | ---- | ---- | ----: | ----: | ----: | ---- | ---- | ---- |
| capability/replay | `workflow`, `ops` | `workflow`, `ops` | `dto`, `api`, `replay_policy` | 40 | 2 | 2 | Medium | Medium | Medium |
| root-managed renewal | `domain`, `ops` | `ops`, `workflow` | `dto`, `api`, `storage`, `access` | 36 | 3 | 2 | High inside focused `ops/auth/delegation/root_issuer_renewal/*` modules | Medium | Medium |
| delegated-token verification | `ops`, `access` | `access`, `workflow` | `dto`, `storage`, `api`, `config` | 43 | 3 | 2 | Medium | High | Medium |
| replay and idempotency | `replay_policy`, `ops`, `workflow` | `ops`, `workflow` | `dto`, `ids`, `model`, `storage` | 85 | 4 | 2 | Medium | High but expected | Medium |
| error mapping | `infra`, `ops`, `workflow` | all boundary layers | `dto`, `api`, root files | 174 | 4 | 2 | Low | High and noisy | Medium |

Root-managed renewal has acceptable decision concentration after the split:
the concept is spread across layers by design, but the decision-heavy code is
clustered under one ops directory instead of leaking into workflow or DTOs.

## Structural Hotspots

### Runtime Complexity Hotspots

| File / Module | Logical LOC | `match` | `if` | Branch Density / 100 LOC | Reason | Risk |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| `api/blob_storage.rs` | 1005 | 5 | 20 | 2.49 | billing/status/sync/fund facade still owns many boundary branches | High |
| `ops/storage/icp_refill.rs` | 783 | 5 | 3 | 1.02 | broad storage projection/update support but low branch density | Medium |
| `ops/auth/token.rs` | 750 | 11 | 13 | 3.20 | token prepare/get and issuer/root proof binding checks | High |
| `workflow/runtime/auth/prepare/mod.rs` | 647 | 16 | 11 | 4.17 | runtime auth bootstrap and prepare orchestration axes | High |
| `ops/auth/delegated/verify.rs` | 645 | 1 | 21 | 3.41 | verifier predicate stack remains narrow but branch-heavy | Medium |
| `workflow/rpc/request/handler/nonroot_cycles.rs` | 631 | 11 | 6 | 2.69 | non-root cycles authorization, replay, policy, and execution axes | High |
| `bootstrap/render.rs` | 616 | 7 | 4 | 1.79 | bootstrap rendering remains broad but mostly deterministic projection | Medium |

The non-test large-file scan also counts
`test/seams/registry_policy_seam.rs` at `629` logical LOC because it is shared
support rather than a `tests.rs` file; it is tracked below with test/support
complexity.

### Renewal Split Footprint

| File / Module | Logical LOC | Responsibility | Risk |
| --- | ---: | --- | --- |
| `ops/auth/delegation/root_issuer_renewal/mod.rs` | 225 | renewal template/provisioner facade and status delegation | Low |
| `ops/auth/delegation/root_issuer_renewal/schedule.rs` | 360 | due-template selection, prepare request construction, prepare persistence | Medium |
| `ops/auth/delegation/root_issuer_renewal/retrieval.rs` | 212 | scheduled proof-batch retrieval and install-window validation | Medium |
| `ops/auth/delegation/root_issuer_renewal/install.rs` | 506 | scheduled/manual install preflight and outcome recording | Medium |
| `ops/auth/delegation/root_issuer_renewal/view.rs` | 138 | DTO view projection | Low |
| `ops/auth/delegation/root_issuer_renewal/identity.rs` | 87 | deterministic fingerprints and ids | Low |
| `ops/auth/delegation/root_issuer_renewal/tests.rs` | 861 | retained renewal behavior matrix | Test-only |

No root-renewal production file remains above the large-file threshold.

### Test Complexity Hotspots

| Test File / Module | Logical LOC | Tracking Impact |
| --- | ---: | --- |
| `workflow/rpc/request/handler/tests.rs` | 1115 | Broad request-handler replay, auth, capability, and nonroot-cycle coverage. |
| `ops/runtime/metrics/tests.rs` | 1033 | All-family metrics coverage remains isolated from production projection code. |
| `config/schema/subnet/tests.rs` | 1014 | Config schema coverage remains broad but isolated from production schema code. |
| `ops/auth/delegation/root_issuer_renewal/tests.rs` | 861 | Renewal behavior matrix is broad but isolated from production logic. |
| `workflow/ic/icp_refill/tests.rs` | 825 | ICP refill workflow harness is broad but not on the current renewal path. |
| `ops/auth/delegation/tests.rs` | 665 | Root proof provisioning behavior matrix remains broad but production owners are split. |
| `test/seams/registry_policy_seam.rs` | 629 | Shared test seam remains broad by design. |

## Hub Module Pressure

| Module | Import / Surface Driver | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `api/blob_storage.rs` | billing status, gateway sync, funding, config, Cashier, root-owned install/status facade | 6 | 5 | 8 |
| `ops/auth/delegation/root_issuer_renewal/*` | renewal template/provisioner status, scheduler, retrieval, expiry, install state update | 5 | 3 | 5 |
| `workflow/runtime/auth/provisioning/mod.rs` | root batch install broadcast, per-signer outcomes, remote install call mapping | 4 | 3 | 7 |
| `workflow/runtime/auth/prepare/mod.rs` | token prepare orchestration, active proof dependency, caller/runtime checks | 4 | 3 | 7 |
| `ops/auth/token.rs` | active root proof, issuer proof creation, claims binding, canonical hashes | 4 | 3 | 7 |
| `workflow/rpc/request/handler/*` | capability, replay, authz, execution, non-root cycles | 5 | 4 | 8 |

## Control Surface Detection

| Control Surface | File | Responsibility | Risk |
| --- | --- | --- | --- |
| `eval_access` | `access/expr/mod.rs` | capability/auth evaluation engine | Medium |
| root renewal scheduler | `ops/auth/delegation/root_issuer_renewal/schedule.rs` | due-template selection and batch prepare persistence | Medium |
| root renewal retrieval | `ops/auth/delegation/root_issuer_renewal/retrieval.rs` | batch get validation, install-window validation, expiry recording | Medium |
| blob-storage operator facade | `api/blob_storage.rs` | status/sync/fund/billing boundary orchestration | High |
| runtime bootstrap | `workflow/runtime/mod.rs` | system initialization coordination | Medium |
| nonroot cycles handler | `workflow/rpc/request/handler/nonroot_cycles.rs` | replay, policy, authorization, and execution boundary | High |

## Drift Sensitivity

| Area | Decision Axes | Axis Count | Branch Multiplier | Drift Sensitivity | Risk |
| ---- | ---- | ----: | ----: | ---- | ---- |
| root-managed renewal | lifecycle phase, issuer policy, replay metadata, proof availability, install outcome, expiry state | 6 | high | each added renewal outcome touches scheduler/install/status/metrics | High |
| blob-storage billing/status | billing enabled, gateway set, Cashier balance, reserve policy, funding outcome | 5 | medium | facade grows when operator endpoints gain states | Medium |
| delegated-token prepare | active proof, issuer proof, subject binding, replay metadata, scope grants | 5 | high | adding auth material expands prepare and verifier paths | High |
| root capability | family, proof mode, replay state, policy outcome, caller topology | 5 | stable | variant surface is flat after hard cuts | Medium |
| nonroot cycles | caller topology, replay, funding policy, transfer outcome, error mapping | 5 | medium | policy and replay axes remain coupled | Medium |

## Complexity Risk Index

| Area | Score (1-10) | Weight | Weighted Score |
| ---- | ----: | ----: | ----: |
| Variant explosion risk | 2 | 2 | 4 |
| Branching pressure trend | 3 | 2 | 6 |
| Flow multiplicity | 3 | 2 | 6 |
| Cross-layer spread | 4 | 3 | 12 |
| Hub pressure + call depth | 4 | 2 | 8 |

Weighted aggregate: `36 / 11 = 3.27`, rounded to **3 / 10**.

Interpretation: **low-moderate, contained complexity risk**. The active enum
surface is stable, and the largest new renewal hub was split below the
large-file threshold. Remaining risk is concentrated in older facade and auth
orchestration files rather than new variant growth.

## Structural Entropy Drift

| Signal | Previous | Current | Delta | Risk |
| ---- | ----: | ----: | ----: | ---- |
| enum_density_avg | low | low | flat | Low |
| axis_coupling_avg | medium | medium | flat | Medium |
| concept_fragmentation_avg | medium | medium-high | slight increase | Medium |
| hub_modules | 6 non-test large files | 8 non-test large files | +2 | Medium |

Hub-module count is still elevated versus June 19, but the follow-up renewal
split removed the most recent high-pressure parent. Since enum density and
axis coupling stayed flat, this remains a size/locality warning rather than
variant explosion.

## Refactor Noise Filter

| Signal | Raw Trend | Noise Filter Result | Adjusted Interpretation |
| ---- | ---- | ---- | ---- |
| file count | `486 -> 507` | refactor transient | directory-module splits increase files while reducing single-file gravity |
| renewal mentions | increased around `root_issuer_renewal/*` | structural improvement | decision owners are more focused even though files increased |
| non-test large files | `6 -> 8` | true pressure, improved after cleanup | remaining large files are older facade/auth/cycles/support hubs |
| capability mentions | `25 -> 40` | mostly raw fan-in noise | capability variants and proof modes stayed flat |
| blob-storage API size | now `1005` logical LOC | true pressure | broad API facade remains a real complexity hotspot |

## Required Summary

1. Overall Complexity Risk Index: **3 / 10**.
2. Fastest Growing Concept Families: root-managed renewal, blob-storage
   billing/status, auth metrics/provisioner visibility.
3. Highest Branch Multipliers: `Request`, `Response`,
   `RootCapability`, `BuiltinPredicate`, `InternalErrorClass`.
4. Highest Axis Coupling Hotspots: delegated-token prepare, nonroot cycles,
   blob-storage API facade, and the split root renewal lifecycle.
5. Flow Multiplication Risks: root delegation proof batch prepare/get/install
   remains the highest effective-flow operation.
6. Cross-Layer Spread Risks: error mapping and replay/idempotency remain broad;
   root renewal is broad but locally concentrated under auth ops.
7. Concept Fragmentation Warnings: delegated-token verification and replay
   remain the noisiest long-lived concepts.
8. Hub Pressure + Call-Depth Warnings: `api/blob_storage.rs`,
   `ops/auth/token.rs`, `workflow/runtime/auth/prepare/mod.rs`, and
   `workflow/rpc/request/handler/nonroot_cycles.rs`.
9. Structural Entropy Drift Findings: enum surfaces flat; hub count up versus
   June 19 but down from the pre-cleanup 2026-06-28 snapshot.
10. Refactor-Transient vs True-Entropy Findings: renewal split file growth is
    beneficial; the blob-storage facade and older auth/cycles hubs are true
    residual pressure.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `git rev-parse --short HEAD` | PASS | Captured `37ac9f8e`. |
| `git branch --show-current` | PASS | Captured `main`. |
| `find crates/canic-core/src -name '*.rs'` | PASS | Captured `507` files. |
| logical LOC scan over `crates/canic-core/src` | PASS | Captured `63865` non-blank/non-comment logical lines. |
| non-test logical LOC scan | PASS | Captured `490` files and `56295` logical LOC. |
| large-file scan | PASS | Captured `14` total files and `8` non-test files above `600` logical LOC. |
| enum/reference scans | PASS | Root request/capability variants remain `4`; `CapabilityProof` remains `1`. |
| concept spread scans | PASS | Capability `40` files, root-managed renewal `36`, delegated-token verification `43`, replay/idempotency `85`, error mapping `174`. |
| branch-density sample | PASS | Current runtime hotspots sampled for `match`/`if` density. |
| `wc -l crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/{mod.rs,schedule.rs,retrieval.rs,install.rs,identity.rs,view.rs}` | PASS | Largest renewal production file is `install.rs` at `541` physical lines; all renewal production files are below `600` physical lines. |
| `cargo check --locked -p canic-core` | PASS | Passed after the scheduling/retrieval split. |
| `cargo clippy --locked -p canic-core --lib -- -D warnings` | PASS | Passed after the scheduling/retrieval split. |
| `cargo test --locked -p canic-core auth::delegation --lib` | PASS | Passed after the scheduling/retrieval split; `46` tests ran. |
| `cargo fmt --all -- --check` | PASS | Formatting check passed after final edits. |
| `git diff --check` | PASS | Whitespace diff check passed after final edits. |
