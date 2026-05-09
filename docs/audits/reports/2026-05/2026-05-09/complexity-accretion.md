# Complexity Accretion Audit - 2026-05-09

## Report Preamble

- Scope: `crates/canic-core/src/**`
- Compared baseline report path:
  `docs/audits/reports/2026-04/2026-04-05/complexity-accretion.md`
- Code snapshot identifier: `ed6bfe9c`
- Method tag/version: `Method V4.1`
- Comparability status: `comparable`, with note that several prior large
  modules have since been split into directory modules
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-09T15:21:00Z`
- Branch: `main`
- Worktree: `dirty`

## Executive Summary

- Risk Score: **3 / 10 after remediation**; original audit readout was
  **7 / 10**.
- Delta summary: enum surfaces stayed stable, but core file count, logical LOC,
  and large-file pressure increased materially.
- Largest growth contributor: runtime metrics projection/tests and directory
  placement workflow/state machinery; follow-up remediation split those
  hotspots into focused production and test/support modules.
- Mixed-domain pressure: capability/replay remains spread across `api`, `dto`,
  `ops`, and `workflow`; directory placement now spans workflow + ops storage.
- Follow-up required: **completed for the identified high-risk hotspots**.

The prior audit's auth hotspots improved structurally: `access/auth`,
`access/expr`, `storage/stable/auth`, and auth metrics are now split into
smaller directory modules. The complexity did not disappear; it moved into
large aggregation modules, especially `ops/runtime/metrics/mod.rs`, and into
placement directory orchestration. Follow-up remediation reduced the identified
metrics and directory placement hotspots, then split additional config schema
and intent-storage test bulk so remaining pressure is mostly broader IC facade
size rather than immediate branch-axis concentration.

## Baseline Capture

| Metric | Previous | Current | Delta |
| ---- | ----: | ----: | ----: |
| Total runtime files in scope | 331 | 377 | +46 |
| Runtime logical LOC | 28019 | 40156 | +12137 |
| Non-test runtime files | N/A | 365 | N/A |
| Non-test runtime logical LOC | N/A | 37883 | N/A |
| Files >= 600 LOC | 4 | 6 | +2 |
| Non-test files >= 600 LOC | N/A | 5 | N/A |
| Capability mentions | N/A | 30 files | N/A |
| Capability decision owners | N/A | 12 `api` files, 8 `workflow` files | N/A |
| Capability execution consumers | N/A | 5 `ops` files | N/A |
| Capability plumbing modules | N/A | 5 `dto` files | N/A |

## Subsystem Map

| Subsystem | Files | Logical LOC |
| --- | ---: | ---: |
| `api` | 43 | 3162 |
| `workflow` | 76 | 9394 |
| `access` | 10 | 1712 |
| `domain` | 19 | 852 |
| `ops` | 115 | 15719 |
| `dto` | 28 | 1115 |
| `storage` | 29 | 2718 |
| `config` | 4 | 1676 |
| `infra` | 9 | 826 |
| `lifecycle` | 7 | 338 |
| `bootstrap` | 2 | 536 |
| `view` | 6 | 67 |
| `ids` | 8 | 278 |
| `dispatch` | 2 | 105 |
| `ingress` | 2 | 76 |
| `format` | 1 | 76 |
| `test` | 10 | 853 |

## Variant Surface Growth

| Enum | Variants | Previous | Delta | Variant Velocity | Switch Sites | Branch Multiplier | Enum Density | Mixed Domain? | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ----: | ----: | ---- | ---- |
| `dto::rpc::Request` | 5 | 5 | 0 | 0 | 3 production flow sites | 15 | 17 / 377 = 0.05 | Yes | Medium |
| `dto::rpc::Response` | 5 | 5 | 0 | 0 | 4 production flow sites | 20 | 13 / 377 = 0.03 | Yes | Medium |
| `dto::capability::CapabilityProof` | 3 | 3 | 0 | 0 | 2 production flow sites | 6 | 6 / 377 = 0.02 | Yes | Medium |
| `dto::capability::CapabilityService` | 1 | 1 | 0 | 0 | 2 guard sites | 2 | 8 / 377 = 0.02 | No | Low |
| `access::expr::BuiltinPredicate` | 4 top-level, 14 evaluator arms | 4 | 0 | 0 | 1 central dispatch site | 14 evaluator arms | 2 / 377 = 0.01 | Yes | Medium |
| `workflow::rpc::request::handler::RootCapability` | 5 | 5 | 0 | 0 | 5 production flow sites | 25 | 8 / 377 = 0.02 | Yes | Medium |
| `ops::runtime::metrics::RootCapabilityMetricEventType` | 5 | 5 | 0 | 0 | metrics-only routing | 5 | 1 / 377 = 0.00 | No | Low |
| `ops::runtime::metrics::RootCapabilityMetricOutcome` | 9 | N/A | N/A | N/A | metrics-only routing | low | 1 / 377 = 0.00 | Yes | Medium |
| `ops::auth::error::AuthValidationError` | 12 | N/A | N/A | N/A | converted at API/auth seams | moderate | 9 / 377 = 0.02 | Yes | Medium |
| `ops::auth::delegated::VerifyDelegatedTokenError` | 24 | N/A | N/A | N/A | delegated-token verification path | high | 3 / 377 = 0.01 | Yes | Medium |
| `error::InternalErrorClass` | 6 | 6 | 0 | 0 | 8 production classifier sites | 48 | 11 / 377 = 0.03 | Yes | Medium |
| `infra::InfraError` | 1 | N/A | N/A | N/A | one conversion path | 1 | 13 / 377 = 0.03 | No | Low |

## Execution Branching Pressure

| Function / Area | Module | Branch Layers | Match Depth | Domains Mixed | Axis Coupling Index | Previous Branch Layers | Delta | Risk |
| ---- | ---- | ----: | ----: | ----: | ----: | ----: | ----: | ---- |
| `entries` + projection helpers | `ops/runtime/metrics/mod.rs` | 2 | 1 | 3 | 6 | N/A | N/A | Low after test split |
| `workflow placement directory` | `workflow/placement/directory/mod.rs` | 5 | 2 | 4 | 20 | N/A | N/A | Medium after support/test split |
| `directory storage ops` | `ops/storage/placement/directory.rs` | 4 | 2 | 3 | 12 | N/A | N/A | Low after test split |
| `nonroot cycles handler` | `workflow/rpc/request/handler/nonroot_cycles.rs` | 5 | 2 | 4 | 20 | N/A | N/A | High |
| `root request handler` | `workflow/rpc/request/handler/mod.rs` | 5 | 2 | 5 | 25 | N/A | N/A | High |
| `root replay classification` | `workflow/rpc/request/handler/replay.rs` | 4 | 2 | 3 | 12 | N/A | N/A | Medium |
| `access evaluator dispatch` | `access/expr/evaluators.rs` | 4 | 1 | 3 | 12 | N/A | N/A | Medium |

Axis families observed:

- capability family
- proof mode
- replay state
- caller topology relation
- policy outcome
- metadata validity
- placement/directory claim state
- metric family/label projection

## Structural Hotspots

### Runtime Complexity Hotspots

| File / Module | Logical LOC | `match` | `if` | Branch Density / 100 LOC | Reason | Risk |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| `ops/runtime/metrics/mod.rs` | 573 after remediation; 1249 at audit capture | 6 | 1 | 1.22 | public metric-family projection; broad all-family tests moved to sibling `tests.rs` | Low |
| `config/schema/subnet.rs` | 672 after remediation; 1051 at audit capture | 2 | 24 | 3.87 | config schema validation/normalization concentration; validation tests moved out | Low |
| `workflow/placement/directory/mod.rs` | 589 after remediation; 990 at audit capture | 15 | 21 | 6.11 | directory placement orchestration; private state/validation support and tests moved out | Medium |
| `ops/ic/mgmt.rs` | 628 | 9 | 0 | 1.43 | management-canister side-effect facade | Medium |
| `ops/storage/placement/directory.rs` | 457 after remediation; 605 at audit capture | 7 | 10 | 3.72 | directory state transitions and projections; storage transition tests moved out | Low |
| `ops/storage/intent.rs` | 470 after remediation; 584 at audit capture | 6 | 13 | 4.04 | intent aggregation/storage state transitions; focused tests moved out | Low |

### Test Complexity Hotspots

| Test File / Module | Logical LOC | Tracking Impact |
| --- | ---: | --- |
| `workflow/rpc/request/handler/tests.rs` | 926 | Still the largest request-handler test harness; lower than April's `1028`, but still a cognitive load center. |
| `ops/runtime/metrics/tests.rs` | 787 | Split from production projection so all-family coverage no longer inflates `mod.rs`. |
| `config/schema/subnet_tests.rs` | 694 | Split from subnet config schema validation so schema types and tests do not share one oversized file. |
| `workflow/placement/directory/tests.rs` | 411 | Split from workflow orchestration so recovery/claim tests no longer inflate production flow. |
| `ops/storage/intent_tests.rs` | 263 | Split from intent storage ops so arithmetic/TTL transition tests no longer inflate production storage code. |
| `ops/storage/placement/directory_tests.rs` | 252 | Split with a path module to avoid `directory.rs` plus `directory/` module conflict. |

## Hub Module Pressure

| Module | Import / Surface Driver | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `ops/runtime/metrics/mod.rs` | all metric-family snapshots + DTO projection; docs/tests live in sibling module | 4 | 2 | 6 |
| `workflow/placement/directory/mod.rs` | workflow, storage ops, metrics, topology; private state/validation support lives in sibling module | 4 | 2 | 6 |
| `workflow/rpc/request/handler/*` | capability, replay, authz, execution, non-root cycles | 5 | 4 | 8 |
| `ops/storage/placement/directory.rs` | directory records, projections, stale-claim state | 4 | 2 | 7 |
| `config/schema/subnet.rs` | fleet/subnet/canister config shape normalization | 3 | 2 | 6 |

## Primary Architectural Pressure

`crates/canic-core/src/ops/runtime/metrics/mod.rs` at audit capture;
remediated by splitting broad tests into `ops/runtime/metrics/tests.rs`.

Reasons:

- largest file in `canic-core/src` at audit capture: `1249` logical LOC
- public `entries(kind)` switch spans every metric family
- test harness covers every metric family inline with production projection code
- adding a metric family touches production projection, reset coverage, docs
  coverage, and tests in one file

This is not a correctness issue. It is change-friction pressure.

## Secondary Architectural Pressure

`crates/canic-core/src/workflow/placement/directory/mod.rs` at audit capture;
remediated by splitting tests and private state/validation support.

Reasons:

- `990` logical LOC, `15` `match`, `21` `if`
- mixes placement workflow, claim lifecycle, stale repair, metrics, and storage
  coordination
- pairs with `ops/storage/placement/directory.rs` at `605` logical LOC, so the
  directory subsystem now has two large files on opposite sides of the
  workflow/ops boundary

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| metric projection gravity | `ops/runtime/metrics/mod.rs` | Remediated to `573` LOC plus sibling tests; every metric family still routes through one projection module | Low |
| directory workflow/state expansion | `workflow/placement/directory/mod.rs` + `ops/storage/placement/directory.rs` | Remediated to `589` and `457` LOC production files, with support/tests split out | Medium |
| capability/replay spread | `api`, `dto`, `ops`, `workflow` | 30 capability files across 4 active layers | Medium |
| root request-handler axis count | `workflow/rpc/request/handler/*` | capability family, proof mode, replay, authz, metadata, and execution axes remain separate but coordinated | Medium |
| auth error taxonomy breadth | `ops/auth/**` | `VerifyDelegatedTokenError` has 24 variants, but reference radius is narrow | Medium |

## Risk Score

Risk Score: **3 / 10 after remediation**

Score contributions:

- `+1` total file count and logical LOC remain higher than April.
- `+1` residual large-file pressure remains in broader config/IC facade modules,
  but the audited metrics/directory/config/intent hotspots are below the
  production large-file threshold.
- `+1` capability/replay remains spread across four active layers, although
  enum variants are stable and the reference radius is controlled.

Interpretation: **low-moderate residual complexity risk**. The code is not
showing variant explosion, and the most actionable aggregation hotspots from
this run have been decomposed. Future wins are targeted splits in config/IC
facade files if they become active edit centers.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `find crates/canic-core/src -name '*.rs' | wc -l` | PASS | Captured `377` files. |
| logical LOC scan over `crates/canic-core/src` | PASS | Captured `40156` non-blank/non-comment lines. |
| non-test logical LOC scan | PASS | Captured `365` files and `37883` logical LOC. |
| large-file scan | PASS | Captured `6` total files and `5` non-test files above `600` logical LOC. |
| enum/reference scans | PASS | RPC/capability enum variants stayed stable; auth error taxonomy and internal error references captured. |
| branch-density sample | PASS | Current hotspot files sampled for `match`/`if` density. |
| `cargo test -p canic-core --lib ops::runtime::metrics -- --nocapture` | PASS | Metrics projection/test split preserved all `53` focused metrics tests. |
| `cargo test -p canic-core --lib workflow::placement::directory -- --nocapture` | PASS | Directory workflow split preserved all `12` focused workflow tests. |
| `cargo test -p canic-core --lib ops::storage::placement::directory -- --nocapture` | PASS | Directory storage split preserved all `8` focused storage tests. |
| `cargo test -p canic-core --lib config::schema -- --nocapture` | PASS | Config schema splits preserved all `40` focused schema tests. |
| `cargo test -p canic-core --lib ops::storage::intent -- --nocapture` | PASS | Intent storage split preserved all `6` focused storage tests. |

## Follow-up Actions

1. Completed: split `ops/runtime/metrics/mod.rs` tests into a sibling
   `tests.rs` module so production projection and all-family coverage do not
   share one large file.
2. Completed: split directory placement workflow tests plus private
   state/validation support, and split directory storage tests into a sibling
   path module.
3. Completed: split config schema and intent storage tests out of production
   files to reduce residual large-file pressure.
4. Keep `VerifyDelegatedTokenError` narrow in reference radius; it has many
   variants but is currently contained to delegated-auth verification seams.
5. Re-run after adding any metric family, directory placement state, or root
   capability request family.
