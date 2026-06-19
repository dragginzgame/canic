# Complexity Accretion Audit - 2026-06-19

## Report Preamble

- Scope: `crates/canic-core/src/**`
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-31/complexity-accretion.md`
- Code snapshot identifier: `ef55e53c`
- Method tag/version: `Method V4.3 / root-proof provisioning map refresh`
- Comparability status: `partially comparable`. File, LOC, enum, and
  large-file counts are comparable. Capability-surface rows are only partially
  comparable because the current method recognizes the hard-cut capability
  model, current `model/` and `replay_policy/` scopes, and root proof
  provisioning as a first-class auth lifecycle.
- Auditor: `codex`
- Run timestamp (UTC): `2026-06-19T10:40:12Z`
- Branch: `main`
- Worktree: `dirty`

## Audit Definition Maintenance

The recurring audit definition was stale before execution and was updated in
`docs/audits/recurring/system/complexity-accretion.md`.

Updates made:

- added `model/**` and `replay_policy/**` to the mandatory subsystem map
- updated the auth domain category to include root-proof provisioning
- replaced retired capability proof mode assumptions with the current
  hard-cut capability proof model plus the active auth proof lifecycle
- added root delegation proof batch prepare/get/install and active proof
  install/status to the current core operation set
- refreshed hotspot paths for directory modules and current auth delegation
  modules

## Executive Summary

- Risk Score: **3 / 10 after cleanup**.
- Delta summary: total runtime files grew `448 -> 486` and logical LOC grew
  `45126 -> 56641`; non-test files above `600` LOC grew `0 -> 6`.
- Positive contraction: the root capability RPC surface contracted from `6` to
  `4` variants, and `CapabilityProof` contracted from `3` to `1` after the
  hard cuts.
- Cleanup applied: root-proof provisioning no longer concentrates batch
  prepare/get/install, pending metadata, root issuer policy mapping, active
  proof status, and error mapping in one production file. The previous
  `810`-LOC `ops/auth/delegation/mod.rs` is now a `70`-LOC facade over focused
  sibling modules.
- Follow-up required: **no immediate root-proof split required**. Residual
  pressure is in older large modules such as delegated-token prepare/verify,
  non-root cycles, and bootstrap rendering.

## Baseline Capture

| Metric | Previous | Current | Delta |
| ---- | ----: | ----: | ----: |
| Total runtime files in scope | 448 | 486 | +38 |
| Runtime logical LOC | 45126 | 56641 | +11515 |
| Files >= 600 LOC | 3 | 12 | +9 |
| Non-test runtime files | 429 | 460 | +31 |
| Non-test runtime logical LOC | 39655 | 48965 | +9310 |
| Non-test files >= 600 LOC | 0 | 6 | +6 |
| Capability mentions | 33 files | 25 files | -8 |
| Capability decision owners | 11 `api` files, 8 `workflow` files | 14 `workflow` files, 4 `ops` files, 1 `api` file | shifted out of API |
| Capability execution consumers | 6 `ops` files | 4 `ops` files | -2 |
| Capability plumbing modules | 4 `dto` files | 2 `dto` files, 4 `replay_policy` files | method shift |

## Subsystem Map

| Subsystem | Files | Logical LOC |
| --- | ---: | ---: |
| `access` | 10 | 1483 |
| `api` | 34 | 1475 |
| `bootstrap` | 2 | 792 |
| `cdk` | 16 | 714 |
| `config` | 12 | 2666 |
| `dispatch` | 2 | 105 |
| `domain` | 24 | 1856 |
| `dto` | 28 | 1353 |
| `format` | 1 | 87 |
| `ids` | 8 | 297 |
| `infra` | 17 | 1184 |
| `ingress` | 2 | 76 |
| `lifecycle` | 7 | 379 |
| `memory` | 6 | 555 |
| `model` | 2 | 378 |
| `ops` | 137 | 22179 |
| `replay_policy` | 12 | 1177 |
| `root` | 8 | 839 |
| `storage` | 28 | 3088 |
| `test` | 10 | 1186 |
| `view` | 7 | 95 |
| `workflow` | 113 | 14677 |

## Variant Surface Growth

| Enum | Variants | Previous | Delta | Variant Velocity | Switch Sites | Branch Multiplier | Enum Density | Mixed Domain? | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ----: | ----: | ---- | ---- |
| `dto::rpc::Request` | 4 | 6 | -2 | -2 | 7 production flow sites | 28 | 17 / 486 = 0.03 | Yes | Medium |
| `dto::rpc::Response` | 4 | 6 | -2 | -2 | 5 production flow sites | 20 | 25 / 486 = 0.05 | Yes | Medium |
| `dto::rpc::RequestFamily` | 4 | 6 | -2 | -2 | 2 production flow sites | 8 | 2 / 486 = 0.00 | Yes | Low |
| `dto::rpc::RootCapabilityCommand` | 4 | 6 | -2 | -2 | 1 conversion site plus replay manifest coverage | 4 | 2 / 486 = 0.00 | Yes | Low |
| `dto::capability::CapabilityProof` | 1 | 3 | -2 | -2 | 2 production flow sites | 2 | 6 / 486 = 0.01 | No | Low |
| `dto::capability::CapabilityService` | 1 | 1 | 0 | 0 | 2 guard sites | 2 | 6 / 486 = 0.01 | No | Low |
| `access::expr::BuiltinPredicate` | 4 top-level, 14 evaluator arms | 4 top-level | 0 | 0 | 1 central dispatch site | 14 evaluator arms | 2 / 486 = 0.00 | Yes | Medium |
| `workflow::rpc::request::handler::RootCapability` | 4 | 6 | -2 | -2 | 4 production flow sites | 16 | 6 / 486 = 0.01 | Yes | Medium |
| `ops::runtime::metrics::RootCapabilityMetricKey` | 4 | 6 | -2 | -2 | metrics routing | 4 | 1 / 486 = 0.00 | No | Low |
| `ops::runtime::metrics::RootCapabilityMetricEventType` | 5 | 5 | 0 | 0 | metrics routing | 5 | 1 / 486 = 0.00 | No | Low |
| `ops::runtime::metrics::RootCapabilityMetricOutcome` | 9 | 9 | 0 | 0 | metrics routing | low | 1 / 486 = 0.00 | Yes | Medium |
| `dto::auth::ActiveDelegationProofStatus` | 4 | N/A | N/A | N/A | 1 status builder | 4 | 5 combined auth-status files / 486 = 0.01 | Yes | Medium |
| `dto::auth::RootDelegationProofInstallOutcome` | 6 | N/A | N/A | N/A | 1 install workflow | 6 | 5 combined auth-status files / 486 = 0.01 | Yes | Medium |
| `error::InternalErrorClass` | 6 | 6 | 0 | 0 | public error classifier sites | 36 | 11 / 486 = 0.02 | Yes | Medium |
| `infra::InfraError` | 1 | 1 | 0 | 0 | one conversion path | 1 | 13 / 486 = 0.03 | No | Low |

The root capability family is healthier than the prior run because the retired
role-attestation and delegated-grant capability proof variants are gone from
the active success model. The new enum growth is in auth provisioning DTOs,
where it is expected for the 0.68 root proof repair.

## Execution Branching Pressure

| Function / Area | Module | Branch Layers | Match Depth | Domains Mixed | Axis Coupling Index | Previous Branch Layers | Delta | Risk |
| ---- | ---- | ----: | ----: | ----: | ----: | ----: | ----: | ---- |
| root proof batch provisioning | `ops/auth/delegation/batch.rs`, `ops/auth/delegation/pending.rs` | 4 | 2 | 4 | 16 | N/A | N/A | Medium after split |
| active proof install/status | `ops/auth/delegation/active.rs` | 3 | 2 | 3 | 9 | N/A | N/A | Low after split |
| delegated-token prepare | `workflow/runtime/auth/prepare/mod.rs` | 5 | 2 | 4 | 20 | 5 | 0 | High |
| delegated-token verifier | `ops/auth/delegated/verify.rs` | 5 | 1 | 4 | 20 | 5 | 0 | Medium |
| root request handler | `workflow/rpc/request/handler/*` | 4 | 2 | 4 | 16 | 5 | -1 | Medium |
| nonroot cycles handler | `workflow/rpc/request/handler/nonroot_cycles.rs` | 5 | 2 | 4 | 20 | 5 | 0 | High |
| root capability proof flow | `workflow/rpc/capability/*` | 3 | 1 | 2 | 6 | 4 | -1 | Low |

Root request complexity improved because the capability proof surface
contracted. Root proof provisioning remains a real auth lifecycle, but no
longer concentrates its lifecycle axes in one production file.

## Concept Scattering

| Concept | Decision Owners | Execution Consumers | Plumbing Modules | Total Modules | Semantic Layers | Transport Layers | Decision Concentration | Concept Fragmentation | Risk |
| ---- | ---- | ---- | ---- | ----: | ----: | ----: | ---- | ---- | ---- |
| capability/replay | `workflow`, `ops` | `workflow`, `ops` | `dto`, `api`, `replay_policy` | 25 | 2 | 2 | Medium | Medium | Medium |
| root proof provisioning | `domain`, `ops` | `ops`, `workflow` | `dto`, `api`, `storage`, `access` | 29 | 3 | 2 | High within `ops/auth/delegation/*` | Medium after split | Medium |
| delegated-token verification | `ops`, `access` | `access`, `workflow` | `dto`, `storage`, `api`, `config` | 39 | 3 | 2 | Medium | High | Medium |
| role attestation | `ops`, `workflow` | `workflow`, `api` | `dto`, `config`, `lifecycle`, `bootstrap` | 32 | 3 | 2 | Medium | Medium | Medium |
| replay and idempotency | `replay_policy`, `ops`, `workflow` | `ops`, `workflow` | `dto`, `ids`, `model`, `storage` | 97 | 4 | 2 | Medium | High but expected | Medium |
| error mapping | `infra`, `ops`, `workflow` | all boundary layers | `dto`, `api`, root files | 169 | 4 | 2 | Low | High and noisy | Medium |

Root proof provisioning remains the newest auth concept, and its file count
increased because the large ops module was split. That is an acceptable
fragmentation tradeoff: policy, active proof state, pending metadata, batch
orchestration, and error mapping now have separate local owners under
`ops/auth/delegation/*`.

## Structural Hotspots

### Runtime Complexity Hotspots

| File / Module | Logical LOC | `match` | `if` | Branch Density / 100 LOC | Reason | Risk |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| `ops/storage/icp_refill.rs` | 783 | 5 | 3 | 1.02 | broad storage projection/update support but low branch density | Medium |
| `ops/auth/token.rs` | 700 | 9 | 13 | 3.14 | token prepare/get and issuer/root proof binding checks | High |
| `workflow/runtime/auth/prepare/mod.rs` | 647 | 16 | 11 | 4.17 | runtime auth bootstrap and prepare orchestration axes | High |
| `ops/auth/delegated/verify.rs` | 645 | 1 | 21 | 3.41 | verifier predicate stack remains narrow but branch-heavy | Medium |
| `workflow/rpc/request/handler/nonroot_cycles.rs` | 628 | 11 | 6 | 2.71 | non-root cycles authorization, replay, policy, and execution axes | High |
| `bootstrap/render.rs` | 602 | 7 | 4 | 1.83 | bootstrap rendering remains broad but mostly deterministic projection | Medium |

### Root Proof Split

| File / Module | Logical LOC | Responsibility | Risk |
| --- | ---: | --- | --- |
| `ops/auth/delegation/pending.rs` | 309 | pending batch metadata, replay cache, quotas, request fingerprinting, cleanup | Medium |
| `ops/auth/delegation/batch.rs` | 302 | batch preflight, prepare, direct-query get assembly, install preflight | Medium |
| `ops/auth/delegation/policy.rs` | 115 | root issuer policy request validation and DTO/domain mapping | Low |
| `ops/auth/delegation/active.rs` | 100 | issuer-local active proof install/status facade helpers | Low |
| `ops/auth/delegation/mod.rs` | 70 | `AuthOps` facade only | Low |
| `ops/auth/delegation/errors.rs` | 15 | local error mapping | Low |

### Test Complexity Hotspots

| Test File / Module | Logical LOC | Tracking Impact |
| --- | ---: | --- |
| `workflow/rpc/request/handler/tests.rs` | 1070 | Broad request-handler replay, auth, capability, and nonroot-cycle coverage. |
| `ops/runtime/metrics/tests.rs` | 999 | All-family metrics coverage remains isolated from production projection code. |
| `config/schema/subnet/tests.rs` | 912 | Config schema coverage remains broad but isolated from production schema code. |
| `workflow/ic/icp_refill/tests.rs` | 825 | ICP refill workflow test harness is broad but not in the current auth repair path. |
| `ops/auth/delegation/tests.rs` | 664 | Root proof provisioning behavior matrix remains broad, but production owners are split. |
| `test/seams/registry_policy_seam.rs` | 619 | Shared test seam remains broad by design. |

## Hub Module Pressure

| Module | Import / Surface Driver | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `ops/auth/delegation/*` | root issuer policy, pending metadata, batch proof callbacks, active proof state | 6 | 4 | 6 after split |
| `workflow/runtime/auth/provisioning/mod.rs` | root batch install broadcast, per-signer outcomes, remote install call mapping | 4 | 3 | 7 |
| `workflow/runtime/auth/prepare/mod.rs` | token prepare orchestration, active proof dependency, caller/runtime checks | 4 | 3 | 7 |
| `ops/auth/token.rs` | active root proof, issuer proof creation, claims binding, canonical hashes | 4 | 3 | 7 |
| `workflow/rpc/request/handler/*` | capability, replay, authz, execution, non-root cycles | 5 | 4 | 8 |

## Primary Architectural Pressure

The primary root-proof concentration was remediated in this run.

`crates/canic-core/src/ops/auth/delegation/mod.rs` moved from `810` logical LOC
to a `70`-LOC facade. The previous responsibilities now live in focused
siblings:

- `pending.rs`: pending metadata, request-id replay, quotas, cleanup, and
  fingerprinting
- `batch.rs`: batch preflight, prepare, direct root query retrieval assembly,
  and install preflight
- `policy.rs`: root issuer policy request validation and DTO/domain mapping
- `active.rs`: issuer-local active proof install/status
- `errors.rs`: local error conversion

The remaining primary architectural pressure is the older delegated-token and
runtime-auth stack, not root proof provisioning itself.

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| non-test large-file growth | `crates/canic-core/src/**` | non-test files >= `600` LOC moved `0 -> 6` | Medium |
| root proof provisioning split | `ops/auth/delegation/*` | previous `810`-LOC module is now `70`, with largest production sibling at `309` LOC | Low |
| auth lifecycle concept spread | `ops`, `workflow`, `domain`, `storage`, `dto`, `api`, `access` | root proof provisioning appears in `29` files after intentional split | Medium |
| capability hard-cut contraction | `dto/rpc.rs`, `dto/capability/mod.rs`, `workflow/rpc/*` | request/root capability variants moved `6 -> 4`; `CapabilityProof` moved `3 -> 1` | Low |
| replay/idempotency breadth | `replay_policy`, `ops`, `workflow`, `dto`, `storage` | replay/idempotency terms appear in `97` files | Medium |
| request-handler residual pressure | `workflow/rpc/request/handler/*` | nonroot cycles remains `628` LOC and high branch-axis pressure | Medium |

## Risk Score

Risk Score: **3 / 10 after cleanup**

Score contributions:

- `+1` non-test large-file count remains above the previous zero-file state at
  `6`, but root proof provisioning is no longer one of those files.
- `+1` delegated auth/root proof concepts span many layers by necessity.
- `+1` request-handler and nonroot-cycles workflow modules remain persistent
  high-axis centers.
- `-1` root capability and capability-proof surfaces contracted after the hard
  cuts.
- `-1` root proof provisioning was split into focused local owners.

Interpretation: **low residual complexity risk**. The 0.68 MVP is not blocked
by this audit. Additional cleanup should target older broad modules only after
the current dev feedback loop confirms there is no functional regression.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `git rev-parse --short HEAD` | PASS | Captured `ef55e53c`. |
| `git branch --show-current` | PASS | Captured `main`. |
| `find crates/canic-core/src -name '*.rs'` | PASS | Captured `486` files. |
| logical LOC scan over `crates/canic-core/src` | PASS | Captured `56641` non-blank/non-comment logical lines. |
| non-test logical LOC scan | PASS | Captured `460` files and `48965` logical LOC. |
| large-file scan | PASS | Captured `12` total files and `6` non-test files above `600` logical LOC. |
| enum/reference scans | PASS | Root request/capability variants are now `4`; `CapabilityProof` variants are now `1`. |
| concept spread scans | PASS | Root proof provisioning `29` files, delegated-token verification `39` files, replay/idempotency `97` files. |
| branch-density sample | PASS | Current runtime hotspots sampled for `match`/`if` density. |
| `cargo test --locked -p canic-core ops::auth::delegation --lib -- --nocapture` | PASS | Focused root proof provisioning tests passed: `26` passed. |
| `cargo clippy --locked -p canic-core --lib -- -D warnings` | PASS | Clippy passed after removing stale facade wrappers and tightening metric lint expectations. |

## Follow-up Actions

1. Completed before the run: refreshed the recurring complexity-accretion audit
   definition for the current root-proof provisioning model.
2. Completed after the run: split `ops/auth/delegation/mod.rs` into focused
   active, batch, pending, policy, and error modules.
3. Completed after the run: removed now-stale auth delegation facade wrappers
   and fixed stale delegated-auth metric lint expectations.
4. Keep the root capability family in contraction mode. Do not reintroduce
   retired delegated-grant or role-attestation capability proof variants.
