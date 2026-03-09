# Complexity Accretion Audit — 2026-03-08

## Run Context

- Audit run: `complexity-accretion`
- Definition: `docs/audits/recurring/complexity-accretion.md`
- Auditor: `codex`
- Date (UTC): `2026-03-08 19:22:56Z`
- Branch: `eleven`
- Commit: `c98bb574`
- Worktree: `dirty`
- Scope: `crates/canic-core/src/**`
- Previous baseline: `docs/audits/reports/2026-03-07/complexity-accretion.md` (latest rerun `02ac3107`)

Rerun note:
- High-risk capability/replay/auth files were re-scanned in this pass after enum-surface decomposition.
- `BuiltinPredicate` and root-capability outcome multipliers were refreshed from current code.

## STEP 0 — Baseline Capture

| Metric | Previous | Current | Delta |
| ---- | ----: | ----: | ----: |
| Total runtime files in scope | 319 | 325 | +6 |
| Runtime LOC | 31870 | 32374 | +504 |
| Files >= 600 LOC | 7 | 7 | 0 |
| Capability mentions (`capability`) | 323 | 336 | +13 |
| Capability decision owners (proxy) | 2 | 2 | 0 |
| Capability execution consumers (proxy) | 8 | 8 | 0 |
| Capability plumbing modules (proxy) | 8 | 9 | +1 |

## STEP 1 — Variant Surface Growth + Branch Multiplier

| Enum | Variants | Switch Sites | Branch Multiplier | Domain Scope | Mixed Domains? | Growth Risk |
| ---- | ----: | ----: | ----: | ---- | ---- | ---- |
| `dto::rpc::Request` | 5 | 18 | 90 | RPC contract + root dispatch | Yes | Medium |
| `dto::rpc::Response` | 5 | 41 | 205 | RPC response contract | Yes | Medium |
| `dto::capability::CapabilityProof` | 3 | 15 | 45 | capability auth mode | Yes | High |
| `dto::capability::CapabilityService` | 5 | 11 | 55 | service routing | Yes | Medium |
| `access::expr::BuiltinPredicate` | 4 | 16 | 64 | guard/auth/environment | Yes | Low |
| `workflow::...::RootCapability` | 5 | 34 | 170 | root capability dispatch | Yes | High |
| `RootCapabilityMetricEventType` | 5 | 26 | 130 | replay/auth/exec observability axis (type) | Yes | Medium |
| `RootCapabilityMetricOutcome` | 9 | 24 | 216 | replay/auth/exec observability axis (outcome) | Yes | Medium |
| `RootCapabilityMetricProofMode` | 4 | 11 | 44 | replay/auth/exec observability axis (proof mode) | Yes | Low |
| `DelegatedTokenOpsError` | 4 | 22 | 88 | top-level auth envelope | No | Low |
| `DelegationValidationError` | 10 | 22 | 220 | validation/auth correctness | Yes | Medium |
| `DelegationSignatureError` | 8 | 11 | 88 | signature verification | No | Low |
| `DelegationScopeError` | 7 | 11 | 77 | scope/audience/subject | Yes | Medium |
| `DelegationExpiryError` | 11 | 18 | 198 | time/epoch windows | Yes | Medium |
| `InternalErrorClass` | 6 | 14 | 84 | internal error taxonomy | No | Low |
| `InfraError` | 1 | 26 | 26 | infra envelope | No | Low |

Trend note:
- The prior monolithic auth multiplier (`36 x 62 = 2232`) is no longer present at the top-level enum surface.

## STEP 2 — Execution Branching Pressure (Trend-Based)

| Function | Module | Branch Layers | Match Depth | Previous Branch Layers | Delta | Domains Mixed | Risk |
| ---- | ---- | ----: | ----: | ----: | ----: | ----: | ---- |
| `response_capability_v1` | `api/rpc/capability/mod.rs` | 5 | 3 | 5 | 0 | 4 | High |
| `verify_root_capability_proof` | `api/rpc/capability/verifier.rs` | 4 | 2 | N/A | N/A | 3 | Medium |
| `response_with_pipeline` | `workflow/rpc/request/handler/mod.rs` | 5 | 2 | 6 | -1 | 4 | Medium-high |
| `preflight` | `workflow/rpc/request/handler/mod.rs` | 3 | 2 | 4 | -1 | 3 | Medium |
| `check_replay` | `workflow/rpc/request/handler/replay.rs` | 4 | 1 | 6 | -2 | 3 | Medium |
| `evaluate_root_replay` | `ops/replay/guard.rs` | 4 | 2 | N/A | N/A | 2 | Medium |

## STEP 3 — Execution Path Multiplicity (Effective Flows)

| Operation | Axes Used | Axis Cardinalities | Theoretical Space | Effective Flows | Previous Effective Flows | Delta | Shared Core? | Risk |
| ---- | ---- | ---- | ----: | ----: | ----: | ----: | ---- | ---- |
| `response_capability_v1` | family, proof mode, envelope validity, proof validity, metadata validity | `5x3x2x2x2` | 120 | 16 | 18 | -2 | Yes (`RootResponseWorkflow`) | High |
| `create_canister` | parent choice, replay status, policy, proof mode | `5x4x2x3` | 120 | 11 | 12 | -1 | Yes | High |
| `upgrade_canister` | target relation, replay status, policy, proof mode | `3x4x2x3` | 72 | 9 | 10 | -1 | Yes | Medium |
| `cycles` | balance check, replay status, policy, proof mode | `2x4x2x3` | 48 | 7 | 8 | -1 | Yes | Medium |
| `issue_delegation` | config gate, caller-shard relation, claim validity, proof mode | `2x2x4x3` | 48 | 10 | 11 | -1 | Yes | High |
| `issue_role_attestation` | subject/role/subnet/audience/ttl validity, proof mode | `2x2x2x2x2x3` | 96 | 11 | 12 | -1 | Yes | High |

## STEP 4 — Cross-Cutting Concern Spread (Authority vs Plumbing)

| Concept | Decision Owners | Execution Consumers | Plumbing Modules | Total Modules | Semantic Layers | Transport Layers | Risk |
| ---- | ----: | ----: | ----: | ----: | ---- | ---- | ---- |
| Capability envelope validation | 1 | 2 | 2 | 4 | API + Workflow | DTO + Ops | Medium |
| Capability hash binding | 1 | 2 | 2 | 4 | API + Workflow | DTO + Ops | Medium |
| Replay key + payload hash semantics | 1 | 3 | 3 | 7 | Ops + Workflow + API | DTO + Storage | Medium |
| Role attestation verification + key refresh | 2 | 5 | 8 | 15 | API + Workflow + Ops + Config | DTO + Storage + Protocol | High |
| Delegated grant verification path | 2 | 2 | 2 | 6 | API + Workflow + Ops | DTO | Medium |
| Error origin mapping (`InfraError`/`InternalError`/boundary `Error`) | 2 | 18 | 16 | 36 | API + Workflow + Ops + Infra + Domain | DTO + Protocol + Storage | High |

## STEP 5 — Cognitive Load Indicators (Hub + Call Depth)

| Module/Operation | LOC or Call Depth | Domain Count | Previous | Delta | Risk |
| ---- | ----: | ----: | ----: | ----: | ---- |
| `workflow/rpc/request/handler/mod.rs` | 189 LOC | 2 | 218 | -29 | Low |
| `ops/auth/mod.rs` | 79 LOC | 1 | 76 | +3 | Low |
| `api/rpc/mod.rs` | 62 LOC | 2 | 62 | 0 | Low |
| `api/rpc/capability/mod.rs` | 203 LOC | 3 | 200 | +3 | Medium |
| `workflow/rpc/request/handler/replay.rs` | 197 LOC | 3 | 211 | -14 | Medium |
| `ops/replay/guard.rs` | 225 LOC | 2 | N/A | N/A | Medium |
| `response_capability_v1` call chain | depth ~6 | 4 | ~6 | 0 | Medium-high |

Hub-pressure condition (`LOC > 600` and `domain_count >= 3`):
- Not triggered by active control-plane modules.

## STEP 6 — Drift Sensitivity (Axis Count)

| Area | Decision Axes | Axis Count | Branch Multiplier | Drift Sensitivity | Risk |
| ---- | ---- | ----: | ----: | ---- | ---- |
| Root capability dispatch | family + proof + replay + policy + topology | 5 | 679 | High | High |
| Replay resolution | ttl + request id + payload hash + existing-slot state | 4 | 312 | Medium | Medium |
| Delegated token/attestation verification | validation + signature + scope + expiry | 4 | 583 (layered total proxy) | Medium-high | High |
| Error envelope mapping | class + origin + infra variant | 3 | 110 | Medium | Medium |

## STEP 7 — Complexity Risk Index

| Area | Score (1-10) | Weight | Weighted Score |
| ---- | ----: | ----: | ----: |
| Variant explosion risk | 2 | 2 | 4 |
| Branching pressure trend | 3 | 2 | 6 |
| Flow multiplicity | 5 | 2 | 10 |
| Cross-layer spread | 4 | 3 | 12 |
| Hub pressure + call depth | 4 | 2 | 8 |

`overall_index = 40 / 11 = 3.64`

Interpretation: **Low-moderate risk**, improved from the prior 5.36 baseline and earlier same-day reruns (`4.55`, `4.18`, `4.00`, `3.82`) after request, predicate, and metric-axis decomposition.

## STEP 8 — Refactor Noise Filter

| Signal | Raw Trend | Noise Filter Result | Adjusted Interpretation |
| ---- | ---- | ---- | ---- |
| Runtime LOC +504 | Up | Mostly structural extraction with targeted behavior tightening | Not entropy-only growth |
| Top-level auth multiplier collapse (`2232 -> 88`) | Down sharply | Non-transient | True complexity reduction |
| `Request::` decision sites (`94 -> 18`) | Down sharply | Non-transient | Material complexity reduction |
| `BuiltinPredicate::` top-level variant surface (`14 -> 4`) with grouped sub-enums | Down sharply | Non-transient | DSL hotspot pressure reduced |
| `RootCapabilityMetricEvent` split into type/outcome/proof-mode | Down in single-enum growth pressure | Non-transient | Observability axis decoupled from feature-linear enum growth |
| Workflow replay now pure-decision intake | Down in branching | Non-transient | Cross-layer coupling reduced |

## Required Summary

1. Overall Complexity Risk Index (2026-03-08 rerun): **3.64/10** (prior rerun: `5.36/10`; earlier same-day reruns: `4.55/10`, `4.18/10`, `4.00/10`, `3.82/10`).
2. Highest branch multipliers in this rerun are `DelegationValidationError` (220), `RootCapabilityMetricOutcome` (216), and `DelegationExpiryError` (198).
3. `BuiltinPredicate` pressure dropped materially after grouped-sub-enum decomposition (`14 -> 4` variants; multiplier `224 -> 64`).
4. Largest reduction remains delegated auth error surface moving from monolithic top-level multiplier to layered domain enums.
5. Request decision surface remains materially reduced (`94 -> 18`), with runtime non-dto variant branching for `Request` at `0`.
6. Prior workflow replay storage coupling remains resolved; replay record construction is owned by `ops/replay::commit_root_replay`.
