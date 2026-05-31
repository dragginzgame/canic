# Complexity Accretion Audit - 2026-05-31

## Report Preamble

- Scope: `crates/canic-core/src/**`
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-09/complexity-accretion.md`
- Code snapshot identifier: `af7f960e`
- Method tag/version: `Method V4.2 / current subsystem-map update`
- Comparability status: `partially comparable`. Runtime file/LOC, enum, and
  large-file counts are comparable. Subsystem-map rows are more complete than
  the prior recurring template because this run assigns all current
  `canic-core/src` top-level scopes, including root files and current
  support-only scopes.
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-31T12:39:15Z`
- Branch: `main`
- Worktree: `dirty`

## Executive Summary

- Risk Score: **3 / 10 after cleanup**.
- Delta summary: total runtime files grew `377 -> 448` and logical LOC grew
  `40156 -> 45126`; runtime enum variants grew in the root request/capability
  family from `5 -> 6` after internal invocation proof issuance was added.
- Largest cleanup from this run: `api/ic/canic.rs` was the only current
  non-test file above `600` logical LOC before remediation (`908` LOC). It is
  now a directory module with focused production files:
  `api/ic/canic/mod.rs` at `375` LOC, `proof_cache.rs` at `158` LOC,
  `endpoint.rs` at `63` LOC, and `envelope.rs` at `30` LOC. Focused tests live
  in `api/ic/canic/tests.rs` at `311` LOC.
- Current large-file pressure is test-only: `workflow/rpc/request/handler/tests.rs`
  (`1232` LOC), `ops/runtime/metrics/tests.rs` (`771` LOC), and
  `config/schema/subnet/tests.rs` (`675` LOC).
- Second cleanup from this run: `workflow/placement/sharding/mod.rs` moved from
  `560` LOC to `291` LOC after allocation, bootstrap, and registry helper
  responsibilities moved to focused sibling modules.
- Third cleanup from this run: `workflow/placement/directory/mod.rs` moved from
  `529` LOC to `210` LOC after classification, creation/finalization,
  cleanup/recovery, and config resolution moved to focused sibling modules.
- Follow-up required: **no immediate runtime split required**. Next routine
  cleanup pressure is in request-handler/nonroot-cycles modules rather than the
  protected internal-call or placement facades.

## Baseline Capture

| Metric | Previous | Current | Delta |
| ---- | ----: | ----: | ----: |
| Total runtime files in scope | 377 | 448 | +71 |
| Runtime logical LOC | 40156 | 45126 | +4970 |
| Non-test runtime files | 365 | 429 | +64 |
| Non-test runtime logical LOC | 37883 | 39655 | +1772 |
| Files >= 600 LOC | 6 | 3 | -3 |
| Non-test files >= 600 LOC | 5 | 0 | -5 |
| Capability mentions | 30 files | 33 files | +3 |
| Capability decision owners | 12 `api` files, 8 `workflow` files | 11 `api` files, 8 `workflow` files | -1 `api` file |
| Capability execution consumers | 5 `ops` files | 6 `ops` files | +1 |
| Capability plumbing modules | 5 `dto` files | 4 `dto` files | -1 |

## Subsystem Map

| Subsystem | Files | Logical LOC |
| --- | ---: | ---: |
| `access` | 10 | 1485 |
| `api` | 50 | 4397 |
| `bootstrap` | 2 | 590 |
| `cdk` | 16 | 714 |
| `config` | 12 | 2204 |
| `dispatch` | 2 | 105 |
| `domain` | 20 | 1018 |
| `dto` | 28 | 1307 |
| `format` | 1 | 87 |
| `ids` | 8 | 280 |
| `infra` | 16 | 913 |
| `ingress` | 2 | 76 |
| `lifecycle` | 7 | 379 |
| `memory` | 6 | 603 |
| `ops` | 125 | 16460 |
| `root` | 8 | 788 |
| `storage` | 29 | 2877 |
| `test` | 10 | 861 |
| `view` | 6 | 67 |
| `workflow` | 90 | 9915 |

## Variant Surface Growth

| Enum | Variants | Previous | Delta | Variant Velocity | Switch Sites | Branch Multiplier | Enum Density | Mixed Domain? | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ----: | ----: | ---- | ---- |
| `dto::rpc::Request` | 6 | 5 | +1 | +1 | 3 production flow sites | 18 | 17 / 444 = 0.04 | Yes | Medium |
| `dto::rpc::Response` | 6 | 5 | +1 | +1 | 4 production flow sites | 24 | 13 / 444 = 0.03 | Yes | Medium |
| `dto::rpc::RequestFamily` | 6 | 5 | +1 | +1 | 2 production flow sites | 12 | 10 / 444 = 0.02 | Yes | Medium |
| `dto::capability::CapabilityProof` | 3 | 3 | 0 | 0 | 2 production flow sites | 6 | 6 / 444 = 0.01 | Yes | Medium |
| `dto::capability::CapabilityService` | 1 | 1 | 0 | 0 | 2 guard sites | 2 | 8 / 444 = 0.02 | No | Low |
| `access::expr::BuiltinPredicate` | 4 top-level, 14 evaluator arms | 4 | 0 | 0 | 1 central dispatch site | 14 evaluator arms | 2 / 444 = 0.00 | Yes | Medium |
| `workflow::rpc::request::handler::RootCapability` | 6 | 5 | +1 | +1 | 5 production flow sites | 30 | 8 / 444 = 0.02 | Yes | Medium |
| `ops::runtime::metrics::RootCapabilityMetricKey` | 6 | 5 | +1 | +1 | metrics routing | 6 | 1 / 444 = 0.00 | No | Low |
| `ops::runtime::metrics::RootCapabilityMetricEventType` | 5 | 5 | 0 | 0 | metrics routing | 5 | 1 / 444 = 0.00 | No | Low |
| `ops::runtime::metrics::RootCapabilityMetricOutcome` | 9 | 9 | 0 | 0 | metrics routing | low | 1 / 444 = 0.00 | Yes | Medium |
| `error::InternalErrorClass` | 6 | 6 | 0 | 0 | 8 production classifier sites | 48 | 11 / 444 = 0.03 | Yes | Medium |
| `infra::InfraError` | 1 | 1 | 0 | 0 | one conversion path | 1 | 13 / 444 = 0.03 | No | Low |

## Execution Branching Pressure

| Function / Area | Module | Branch Layers | Match Depth | Domains Mixed | Axis Coupling Index | Previous Branch Layers | Delta | Risk |
| ---- | ---- | ----: | ----: | ----: | ----: | ----: | ----: | ---- |
| protected internal-call facade | `api/ic/canic/*` | 4 | 2 | 4 | 16 | N/A | N/A | Low after split |
| sharding placement workflow | `workflow/placement/sharding/*` | 5 | 2 | 4 | 20 | 5 | 0 | Medium after split |
| directory placement workflow | `workflow/placement/directory/*` | 5 | 2 | 4 | 20 | 5 | 0 | Medium after split |
| root request handler | `workflow/rpc/request/handler/mod.rs` | 5 | 2 | 5 | 25 | 5 | 0 | High |
| nonroot cycles handler | `workflow/rpc/request/handler/nonroot_cycles.rs` | 5 | 2 | 4 | 20 | 5 | 0 | High |
| root replay classification | `workflow/rpc/request/handler/replay.rs` | 4 | 2 | 3 | 12 | 4 | 0 | Medium |
| access evaluator dispatch | `access/expr/mod.rs` | 4 | 2 | 3 | 12 | 4 | 0 | Medium |

The new root request variant increases DTO/workflow/metrics branch multiplier,
but it is one coordinated feature family (`IssueInternalInvocationProof`) rather
than unowned enum drift.

## Structural Hotspots

### Runtime Complexity Hotspots

| File / Module | Logical LOC | `match` | `if` | Branch Density / 100 LOC | Reason | Risk |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| `bootstrap/render.rs` | 529 | 7 | 4 | 2.08 | bootstrap rendering is broad but mostly deterministic projection | Medium |
| `workflow/rpc/request/handler/nonroot_cycles.rs` | 513 | 11 | 5 | 3.12 | non-root cycles authorization, replay, policy, and execution axes | High |
| `ops/rpc/mod.rs` | 508 | 1 | 4 | 0.98 | RPC operation facade and error conversion surface | Medium |
| `ops/runtime/metrics/mod.rs` | 498 | 3 | 0 | 0.60 | metric-family projection hub remains decomposed below threshold | Low |
| `ops/auth/delegated/verify.rs` | 494 | 0 | 21 | 4.25 | delegated-token verifier has many validation predicates but narrow ownership | Medium |
| `workflow/placement/sharding/mod.rs` | 291 | 5 | 5 | 3.44 | sharding facade remains branchy, but allocation/bootstrap/registry support moved out | Low after split |
| `workflow/placement/directory/mod.rs` | 210 | 6 | 5 | 5.24 | directory facade remains branchy, but classification/create/cleanup/config support moved out | Low after split |

### Test Complexity Hotspots

| Test File / Module | Logical LOC | Tracking Impact |
| --- | ---: | --- |
| `workflow/rpc/request/handler/tests.rs` | 1232 | Largest request-handler test harness; tracks root capability, replay, and auth axes together. |
| `ops/runtime/metrics/tests.rs` | 771 | All-family metrics coverage remains broad but isolated from production projection code. |
| `config/schema/subnet/tests.rs` | 675 | Config schema coverage remains broad but isolated from production schema code. |
| `api/ic/canic/tests.rs` | 311 | New split keeps internal-call proof-cache and endpoint descriptor tests out of production LOC. |

## Hub Module Pressure

| Module | Import / Surface Driver | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `api/ic/canic/*` | public client facade, endpoint descriptors, envelope encoding, proof cache, raw-call transport | 4 | 3 | 5 |
| `workflow/placement/sharding/*` | assignment planning, bootstrap, allocation, registry filtering | 4 | 2 | 5 |
| `workflow/placement/directory/*` | resolve/recover facade, classification, creation, cleanup, config lookup | 4 | 2 | 5 |
| `workflow/rpc/request/handler/*` | capability, replay, authz, execution, non-root cycles | 5 | 4 | 8 |
| `ops/runtime/metrics/mod.rs` | all metric-family snapshots and DTO projection | 4 | 2 | 6 |

## Primary Architectural Pressure

`crates/canic-core/src/api/ic/canic.rs` at audit capture; remediated by moving
inline tests to `crates/canic-core/src/api/ic/canic/tests.rs`, then splitting
the remaining production facade into endpoint, envelope, and proof-cache
sibling modules.

Reasons:

- only current non-test file above the `600` logical LOC threshold at audit
  capture: `908` logical LOC
- mixed generated-client API, endpoint descriptors, proof caching, envelope
  construction, and 24 focused tests in one production-named file
- after cleanup, the largest protected internal-call production file is
  `api/ic/canic/mod.rs` at `375` logical LOC and the large non-test file count
  is `0`

This is not a correctness issue. It is change-friction pressure.

## Secondary Cleanup

`crates/canic-core/src/workflow/placement/sharding/mod.rs` at `560` logical LOC
after the protected internal-call cleanup; remediated by moving focused
responsibilities into sibling modules:

- `allocation.rs`: shard creation and lifecycle admission, `81` logical LOC
- `bootstrap.rs`: startup shard creation and empty-pool assignment, `194`
  logical LOC
- `registry.rs`: routable shard and slot helper queries, `47` logical LOC
- `mod.rs`: public assignment/plan facade, now `291` logical LOC

The branch-axis pressure still exists, but it is no longer concentrated in one
near-threshold production file.

## Tertiary Cleanup

`crates/canic-core/src/workflow/placement/directory/mod.rs` at `529` logical
LOC after the sharding cleanup; remediated by moving focused responsibilities
into sibling modules:

- `classification.rs`: entry classification and metric-reason mapping, `77`
  logical LOC
- `create.rs`: claim, create, and finalize flow, `132` logical LOC
- `cleanup.rs`: stale cleanup, recovery mapping, abandoned child recycling, and
  repair, `151` logical LOC
- `config.rs`: pool config resolution, `21` logical LOC
- `mod.rs`: public resolve/recover/bind facade, now `210` logical LOC

Directory placement remains a multi-axis workflow, but the public facade no
longer owns all stale classification, create/finalize, cleanup, and config
branches directly.

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| root request family growth | `dto/rpc.rs`, `workflow/rpc/request/handler/capability.rs`, `ops/runtime/metrics/root_capability.rs` | `Request`, `Response`, `RequestFamily`, `RootCapability`, and metric key all moved `5 -> 6` variants | Medium |
| protected internal-call split | `api/ic/canic/*` | production concerns are split into `mod.rs`, `proof_cache.rs`, `endpoint.rs`, and `envelope.rs`; largest file is `375` logical LOC | Low |
| sharding workflow split | `workflow/placement/sharding/*` | allocation, bootstrap, registry helpers, query, and facade are separate; largest file is `291` logical LOC | Low |
| directory workflow split | `workflow/placement/directory/*` | classification, create/finalize, cleanup/recovery, config lookup, state, query, and facade are separate; largest file is `210` logical LOC outside tests | Low |
| capability/replay spread | `api`, `dto`, `ops`, `workflow` | capability references in `33` files across API `11`, workflow `8`, ops `6`, DTO `4`, plus ids/protocol/domain | Medium |
| request-handler test gravity | `workflow/rpc/request/handler/tests.rs` | `1232` logical LOC test harness | Low for runtime, Medium for maintainability |

## Risk Score

Risk Score: **3 / 10 after cleanup**

Score contributions:

- `+1` root request/capability enum family increased by one coordinated variant.
- `+1` capability/replay concepts remain spread across four active layers.
- `+1` request-handler and nonroot-cycles workflow modules remain the highest
  branch-axis pressure centers after the protected internal-call and placement
  splits.
- `-1` applied noise filter: file count increased partly because production and
  test code were split, and non-test large-file pressure fell to zero.

Interpretation: **low residual complexity risk**. The current drift is
traceable to one root/internal-proof feature slice and existing high-axis
workflow modules, not broad unowned entropy.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `git rev-parse --short HEAD` | PASS | Captured `af7f960e`. |
| `find crates/canic-core/src -name '*.rs' | wc -l` | PASS | Captured `448` files. |
| logical LOC scan over `crates/canic-core/src` | PASS | Captured `45126` non-blank/non-comment lines. |
| non-test logical LOC scan | PASS | Captured `429` files and `39655` logical LOC. |
| large-file scan | PASS | Captured `3` total files and `0` non-test files above `600` logical LOC after cleanup. |
| enum/reference scans | PASS | Root request/capability variants are now `6`; capability references captured in `33` files. |
| branch-density sample | PASS | Current runtime hotspots sampled for `match`/`if` density. |
| `cargo fmt --all` | PASS | Rust formatting completed after the module split. |
| `cargo test -p canic-core --lib api::ic::canic -- --nocapture` | PASS | Focused internal-call facade tests passed: `24` passed, `442` filtered out. |
| `cargo test -p canic-core --lib workflow::placement::sharding -- --nocapture` | PASS | Focused sharding target compiled; no sharding-specific tests matched, `466` filtered out. |
| `cargo test -p canic-core --lib workflow::placement -- --nocapture` | PASS | Focused placement tests passed: `12` passed, `454` filtered out. |
| `git diff --check` | PASS | No whitespace errors detected in the working diff. |

## Follow-up Actions

1. Completed after the run: split `crates/canic-core/src/api/ic/canic.rs` into
   `api/ic/canic/mod.rs` and `api/ic/canic/tests.rs`.
2. Completed after the run: split protected internal-call endpoint descriptor,
   envelope encoding, and proof-cache state into focused sibling modules under
   `api/ic/canic/`.
3. Completed after the run: split sharding placement allocation, bootstrap, and
   registry helpers into focused sibling modules under
   `workflow/placement/sharding/`.
4. Completed after the run: split directory placement classification,
   create/finalize, cleanup/recovery, and config resolution into focused
   sibling modules under `workflow/placement/directory/`.
5. Completed after the run: update the recurring complexity template subsystem
   map to cover current `canic-core/src` top-level scopes and to treat non-test
   `>= 600 LOC` files as the primary runtime hub-pressure signal.
6. Carry forward: keep new request-handler branch axes in focused helper
   modules before `workflow/rpc/request/handler/*` crosses the production
   large-file threshold.
