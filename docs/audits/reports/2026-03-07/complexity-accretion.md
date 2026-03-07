# Complexity Accretion Audit — 2026-03-07

## Run Context

- Audit run: `complexity-accretion`
- Definition: `docs/audits/recurring/complexity-accretion.md`
- Auditor: `codex`
- Date (UTC): `2026-03-07 20:01:14Z`
- Branch: `eleven`
- Commit: `d0b8d415`
- Worktree: `dirty`
- Scope: `crates/canic-core/src/**`
- Baseline source: `v0.12.0` snapshot (no prior recurring run file for this audit)
- Measurement note: `switch_sites` uses a mechanical proxy (`EnumName::` decision-site references); this overcounts constructor-style references but is stable for trend comparison.

## STEP 0 — Baseline Capture

| Metric | Previous | Current | Delta |
| ---- | ----: | ----: | ----: |
| Total runtime files in scope | 289 | 290 | +1 |
| Runtime LOC | 30126 | 31504 | +1378 |
| Files >= 600 LOC | 7 | 9 | +2 |
| Capability mentions (`capability`) | 108 | 274 | +166 |
| Capability decision owners (proxy) | 1 | 2 | +1 |
| Capability execution consumers (proxy) | 1 | 8 | +7 |
| Capability plumbing modules (proxy) | 1 | 8 | +7 |

## STEP 1 — Variant Surface Growth + Branch Multiplier

| Enum | Variants | Switch Sites | Branch Multiplier | Domain Scope | Mixed Domains? | Growth Risk |
| ---- | ----: | ----: | ----: | ---- | ---- | ---- |
| `dto::rpc::Request` | 5 | 94 | 470 | RPC contract + root dispatch | Yes | High |
| `dto::rpc::Response` | 5 | 43 | 215 | RPC response contract | Yes | Medium |
| `dto::capability::CapabilityProof` | 3 | 13 | 39 | capability auth mode | Yes | High (new surface) |
| `dto::capability::CapabilityService` | 5 | 11 | 55 | service routing | Yes | Medium |
| `access::expr::BuiltinPredicate` | 14 | 30 | 420 | guard/auth/environment | Yes | High |
| `workflow::...::RootCapability` | 5 | 34 | 170 | root capability dispatch | Yes | High |
| `RootCapabilityMetricEvent` | 13 | 24 | 312 | replay/auth/exec observability | Yes | High |
| `DelegatedTokenOpsError` | 36 | 62 | 2232 | delegation/attestation failures | Yes | Critical |
| `InternalErrorClass` | 6 | 14 | 84 | internal error taxonomy | No | Low |
| `InfraError` | 1 | 26 | 26 | infra envelope | No | Low |

Trend notes (v0.12.0 -> v0.13.0/current):
- `CapabilityProof`: `0 -> 3` variants, `0 -> 13` decision-site references.
- `CapabilityService`: `0 -> 5` variants, `0 -> 11` decision-site references.
- `Request` decision-site references: `32 -> 94`.
- `RootCapabilityMetricEvent`: `9 -> 13` variants.

## STEP 2 — Execution Branching Pressure

| Function | Module | Branch Layers | Match Depth | Previous Branch Layers | Delta | Domains Mixed | Risk |
| ---- | ---- | ----: | ----: | ----: | ----: | ----: | ---- |
| `response_capability_v1` | `api/rpc.rs` | 5 | 3 | 1 | +4 | 4 | High |
| `verify_root_delegated_grant_claims` | `api/rpc.rs` | 9 | 1 | 0 | +9 | 4 | High |
| `response_with_pipeline` | `workflow/rpc/request/handler.rs` | 6 | 3 | 4 | +2 | 4 | High |
| `preflight` | `workflow/rpc/request/handler.rs` | 4 | 2 | 2 | +2 | 3 | Medium |
| `check_replay` | `workflow/rpc/request/handler.rs` | 6 | 2 | 5 | +1 | 4 | High |

Branch-axis coupling present in hotspots:
- capability family
- proof mode (`Structural` / `RoleAttestation` / `DelegatedGrant`)
- replay state (`accepted` / `duplicate-same` / `duplicate-conflict` / `expired` / `ttl-exceeded`)
- caller topology relation
- policy outcome (`allow` / `deny`)
- metadata validity (`request_id`, `ttl`, skew)

## STEP 3 — Execution Path Multiplicity (Effective Flows)

Effective flow counts are conservative lower-bound estimates from explicit reachable branch outcomes.

| Operation | Axes Used | Axis Cardinalities | Theoretical Space | Effective Flows | Previous Effective Flows | Delta | Shared Core? | Risk |
| ---- | ---- | ---- | ----: | ----: | ----: | ----: | ---- | ---- |
| `response_capability_v1` | family, proof mode, envelope validity, proof validity, metadata validity | `5x3x2x2x2` | 120 | 18 | 6 | +12 | Yes (`RootResponseWorkflow`) | High |
| `create_canister` | parent choice, replay status, policy, proof mode | `5x4x2x3` | 120 | 12 | 10 | +2 | Yes | High |
| `upgrade_canister` | target relation, replay status, policy, proof mode | `3x4x2x3` | 72 | 10 | 8 | +2 | Yes | Medium |
| `cycles` | balance check, replay status, policy, proof mode | `2x4x2x3` | 48 | 8 | 6 | +2 | Yes | Medium |
| `issue_delegation` | config gate, caller-shard relation, claim validity, proof mode | `2x2x4x3` | 48 | 11 | 9 | +2 | Yes | High |
| `issue_role_attestation` | subject/role/subnet/audience/ttl validity, proof mode | `2x2x2x2x2x3` | 96 | 12 | 10 | +2 | Yes | High |

## STEP 4 — Cross-Cutting Concern Spread

| Concept | Decision Owners | Execution Consumers | Plumbing Modules | Total Modules | Semantic Layers | Transport Layers | Risk |
| ---- | ----: | ----: | ----: | ----: | ---- | ---- | ---- |
| Capability envelope validation | 1 | 2 | 2 | 3 | API + Workflow | DTO + Ops | Medium |
| Capability hash binding | 1 | 2 | 2 | 4 | API + Workflow | DTO + Ops | Medium |
| Replay key + payload hash semantics | 2 | 4 | 3 | 9 | API + Workflow + Ops | DTO + Storage | High |
| Role attestation verification + key refresh | 2 | 5 | 9 | 16 | API + Workflow + Ops + Config | DTO + Storage + Protocol | High |
| Delegated grant verification path | 1 | 1 | 1 | 2 | API | DTO | Medium |
| Error origin mapping (`InfraError`/`InternalError`/boundary `Error`) | 2 | 20 | 18 | 40 | API + Workflow + Ops + Infra + Domain | DTO + Protocol + Storage | High |

## STEP 5 — Cognitive Load Indicators

| Module/Operation | LOC or Call Depth | Domain Count | Previous | Delta | Risk |
| ---- | ----: | ----: | ----: | ----: | ---- |
| `workflow/rpc/request/handler.rs` | 1581 LOC | 6 | 1487 | +94 | High |
| `ops/auth.rs` | 1253 LOC | 4 | 1253 | 0 | High |
| `api/rpc.rs` | 900 LOC | 5 | 75 | +825 | High |
| `response_capability_v1` call chain | depth ~6 | 4 | ~2 | +4 | High |
| `create_canister` call chain | depth ~8 | 5 | ~7 | +1 | High |
| `issue_delegation` call chain | depth ~7 | 5 | ~6 | +1 | High |

Hub pressure condition (`LOC > 600` and `domain_count >= 3`) is met by the first three modules.

## STEP 6 — Drift Sensitivity (Axis Count)

| Area | Decision Axes | Axis Count | Branch Multiplier | Drift Sensitivity | Risk |
| ---- | ---- | ----: | ----: | ---- | ---- |
| Root capability dispatch | family + proof + replay + policy + topology | 5 | 679 | High | High |
| Replay resolution | ttl + request id + payload hash + existing-slot state | 4 | 312 | Medium-high | High |
| Delegated token/attestation verification | key id + signature + scope + audience + window + epoch | 6 | 2232 | Critical | Critical |
| Error envelope mapping | class + origin + infra variant | 3 | 110 | Medium | Medium |

## STEP 7 — Complexity Risk Index

| Area | Score (1-10) | Weight | Weighted Score |
| ---- | ----: | ----: | ----: |
| Variant explosion risk | 7 | 2 | 14 |
| Branching pressure trend | 7 | 2 | 14 |
| Flow multiplicity | 6 | 2 | 12 |
| Cross-layer spread | 6 | 3 | 18 |
| Hub pressure + call depth | 7 | 2 | 14 |

`overall_index = 72 / 11 = 6.55`

Interpretation: **High end of Moderate (near High)**.

## STEP 8 — Refactor Noise Filter

| Signal | Raw Trend | Noise Filter Result | Adjusted Interpretation |
| ---- | ---- | ---- | ---- |
| Capability mentions increased (`108 -> 274`) | Up sharply | Not transient (decision owners and consumers also increased) | True entropy growth |
| Large modules (`>=600 LOC`) `7 -> 9` | Up | Not structural split (hub pressure rose) | Complexity concentration |
| Feature slice file-touch trend (`27 -> 20 -> 9`) | Down | Partly improvement | Surface shrank, but branch density concentrated in fewer hubs |

## Required Summary

1. Overall Complexity Risk Index: **6.55/10**.
2. Fastest growing concept families: capability envelope/proof path and replay-coupled auth validation.
3. Highest branch multipliers: `DelegatedTokenOpsError` (2232), `Request` (470), `BuiltinPredicate` (420), `RootCapabilityMetricEvent` (312).
4. Flow multiplication risks: capability proof mode combined with replay and policy axes drives `response_capability_v1` theoretical space to 120.
5. Cross-layer spread risks: role-attestation and error-origin mapping both span `>=4` semantic layers.
6. Hub pressure warnings: `workflow/rpc/request/handler.rs`, `ops/auth.rs`, and `api/rpc.rs` are all high-pressure hubs.
7. Refactor-transient vs true-entropy: this run is primarily **true entropy growth**, not transient refactor noise.
