# Velocity Preservation Audit — 2026-03-07

## Run Context

- Audit run: `velocity-preservation`
- Definition: `docs/audits/recurring/velocity-preservation.md`
- Auditor: `codex`
- Date (UTC): `2026-03-07 20:01:14Z`
- Branch: `eleven`
- Commit: `d0b8d415`
- Worktree: `dirty`
- Scope: `crates/canic-core/src/**`
- Slice window used: `v0.10.7..v0.11.1`, `v0.11.1..v0.12.0`, `v0.12.0..v0.13.0`
- Measurement note: shock/decision-site metrics use mechanical `EnumName::` reference counting for stable trend comparisons.

## Rerun Context (Post-Decomposition)

- Date (UTC): `2026-03-07 22:09:26Z`
- Branch: `eleven`
- Commit: `bca4da37`
- Worktree: `dirty`
- Trigger: extraction-only 0.13.1 hub decomposition to reduce control-plane gravity wells.
- Scope: unchanged (`crates/canic-core/src/**`)
- Note: this rerun supersedes the initial same-day velocity index for release tracking.

## Rerun Delta (Initial Run -> Post-Decomposition)

| Metric | Initial Run | Post-Decomposition | Delta |
| ---- | ----: | ----: | ----: |
| Files >= 600 LOC | 9 | 7 | -2 |
| Control-plane hub (`workflow/rpc/request/handler`) | 1581 LOC | 218 LOC (`mod.rs`) | -1363 |
| Control-plane hub (`ops/auth`) | 1253 LOC | 76 LOC (`mod.rs`) | -1177 |
| Control-plane hub (`api/rpc`) | 900 LOC | 62 LOC (`mod.rs`) | -838 |
| Gravity-well escalation condition on control plane | Triggered | Cleared | Improved |

## Rerun Step — Gravity Well Recheck

| Module | LOC | LOC Delta vs Initial | Fan-In Proxy | Domains | Risk |
| ---- | ----: | ----: | ----: | ----: | ---- |
| `workflow/rpc/request/handler/mod.rs` | 218 | -1363 | 3 (`RootResponseWorkflow`) | 2 | Low |
| `api/rpc/mod.rs` | 62 | -838 | 3 (`RpcApi`) | 2 | Low |
| `api/rpc/capability/mod.rs` | 200 | N/A (new split target) | 4 (`RootCapabilityMetrics`) | 3 | Medium |
| `ops/auth/mod.rs` | 76 | -1177 | 6 (`DelegatedTokenOps`) | 1 | Low |
| `ops/rpc/mod.rs` | 328 | N/A | 4 (`RequestOps`) | 3 | Medium |

## Rerun Step — Velocity Risk Index

| Area | Score | Weight | Weighted Score |
| ---- | ----: | ----: | ----: |
| enum shock radius | 7 | 3 | 21 |
| CAF trend | 4 | 2 | 8 |
| cross-layer leakage | 3 | 2 | 6 |
| gravity-well growth | 3 | 2 | 6 |
| edit blast radius | 4 | 1 | 4 |

`overall_index = 45 / 10 = 4.50`

Interpretation: **Moderate-low risk**, improved from `5.80` due hub flattening.

## STEP 0 — Baseline Capture

| Metric | Previous | Current | Delta |
| ---- | ----: | ----: | ----: |
| Velocity Risk Index | N/A | 5.80 | N/A |
| Cross-layer leakage crossings | N/A | 2 | N/A |
| Avg files touched per feature slice | N/A | 18.7 | N/A |
| p95 files touched | N/A | 27 | N/A |
| Top gravity-well fan-in (proxy) | N/A | 6 | N/A |

## STEP 1 — Change Surface Mapping (Revised CAF)

Assumed total subsystem universe for containment: 15 (`src/*` top-level domains + root-level module files).

| Feature | Files Modified | Subsystems | Layers | Flow Axes | Revised CAF | ELS | Containment Score | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ----: | ----: | ---- |
| `v0.10.7 -> v0.11.1` root replay + auth hardening | 27 | 9 | 8 | 5 | 45 | 0.30 | 0.60 | High |
| `v0.11.1 -> v0.12.0` role attestation + token expansion | 20 | 7 | 7 | 5 | 35 | 0.40 | 0.47 | Medium-high |
| `v0.12.0 -> v0.13.0` capability envelope/proof rollout | 9 | 5 | 5 | 6 | 30 | 0.33 | 0.33 | Medium |

Trend:
- Revised CAF decreased (`45 -> 35 -> 30`), but ELS stayed low (`0.30..0.40`) and indicates weak extension locality.

## STEP 2 — Boundary Leakage (Mechanical + Triaged)

| Boundary | Import Crossings | Previous | Delta | Risk |
| ---- | ----: | ----: | ----: | ---- |
| endpoints/api -> model/storage direct references | 0 (triaged) | 0 | 0 | Low |
| workflow -> model/storage direct references | 1 | 1 | 0 | Medium |
| policy -> dto/ops/runtime references | 1 (`dto::error` usage) | 1 | 0 | Medium |
| ops -> workflow references | 0 (code; only comment hit) | 0 | 0 | Low |
| auth/capability DTO leakage into storage ownership | 0 | 0 | 0 | Low |

## STEP 3 — Gravity Well Growth Rate

| Module | LOC | LOC Delta | Fan-In | Fan-In Delta | Domains | Edit Frequency (30d) | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ----: | ---- |
| `workflow/rpc/request/handler.rs` | 1581 | +94 | 3 (`RootResponseWorkflow` refs) | 0 | 6 | 5 | High |
| `api/rpc.rs` | 900 | +825 | 4 (`RootCapabilityMetrics` refs) | +1 | 5 | 3 | High |
| `ops/auth.rs` | 1253 | 0 | 6 (`DelegatedTokenOps` refs) | 0 | 4 | 2 | Medium-high |
| `ops/runtime/metrics/root_capability.rs` | 204 | +8 | 4 (`RootCapabilityMetrics` refs) | +1 | 3 | 3 | Medium |

Escalation condition hit:
- `api/rpc.rs` has high growth and sits on capability proof/replay gatekeeping.

## STEP 4 — Change Multiplier Matrix

| Feature Axis | Endpoints | Workflow | Policy | Ops | Model/Storage | Subsystem Count |
| ---- | ---- | ---- | ---- | ---- | ---- | ----: |
| proof mode selection | X | X |  | X |  | 3 |
| replay state semantics | X | X |  | X | X | 4 |
| caller topology relation |  | X | X | X | X | 4 |
| role/subnet context | X | X | X | X | X | 5 |
| lifecycle phase (`init`/runtime timers) |  | X |  | X | X | 3 |

| Candidate Feature | Axes Involved | Subsystem Count | Friction |
| ---- | ---- | ----: | ---- |
| Add new proof mode | proof + replay + role/subnet | 5 | High |
| Add new root capability family | proof + replay + topology + role/subnet | 5 | High |
| Replay policy tweak | replay + lifecycle | 4 | Medium-high |

## STEP 5 — Enum Shock Radius (Density-Adjusted)

| Enum | Variants | Switch Sites | Modules Using Enum | Switch Density | Subsystems | Shock Radius | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ----: | ---- |
| `dto::rpc::Request` | 5 | 94 | 6 | 15.67 | 4 | 313.33 | High |
| `dto::rpc::Response` | 5 | 43 | 8 | 5.38 | 5 | 134.38 | Medium |
| `dto::capability::CapabilityProof` | 3 | 13 | 2 | 6.50 | 2 | 39.00 | Medium-high |
| `dto::capability::CapabilityService` | 5 | 11 | 2 | 5.50 | 2 | 55.00 | Medium |
| `access::expr::BuiltinPredicate` | 14 | 30 | 1 | 30.00 | 1 | 420.00 | High |
| `workflow::...::RootCapability` | 5 | 34 | 1 | 34.00 | 1 | 170.00 | High |
| `RootCapabilityMetricEvent` | 13 | 24 | 3 | 8.00 | 3 | 312.00 | High |
| `DelegatedTokenOpsError` | 36 | 62 | 2 | 31.00 | 2 | 2232.00 | Critical |

## STEP 6 — Edit Blast Radius (Empirical, slice-sampled)

| Metric | Current | Previous | Delta |
| ---- | ----: | ----: | ----: |
| average files touched per feature slice | 18.7 | N/A | N/A |
| median files touched | 20 | N/A | N/A |
| p95 files touched | 27 | N/A | N/A |

## STEP 7 — Subsystem Independence Score (Size-Adjusted)

`independence = internal/(internal+external)` from `use crate::...` imports.
`adjusted_independence = independence * log(LOC)`.

| Subsystem | Internal Imports | External Imports | LOC | Independence | Adjusted Independence | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ---- |
| `dto` | 17 | 5 | 1673 | 0.773 | 5.737 | Low |
| `ops` | 2 | 61 | 9582 | 0.032 | 0.293 | High |
| `domain` | 1 | 16 | 1268 | 0.059 | 0.422 | Medium-high |
| `storage` | 1 | 22 | 2654 | 0.043 | 0.339 | High |
| `api` | 0 | 24 | 2522 | 0.000 | 0.000 | High |
| `workflow` | 0 | 58 | 7778 | 0.000 | 0.000 | High |

Interpretation note: this method undercounts internal imports that use `super::`, but still highlights coupling pressure in large subsystems.

## STEP 8 — Decision-Axis Growth (Independence-Aware)

| Operation | Axes | Axis Count | Independent Axes | Previous Independent Axes | Delta | Risk |
| ---- | ---- | ----: | ----: | ----: | ----: | ---- |
| `response_capability_v1` | family, proof mode, envelope validity, metadata validity | 4 | 4 | 0 | +4 | High |
| `response_with_pipeline` | pipeline order, replay status, policy outcome, capability family | 4 | 4 | 3 | +1 | High |
| `check_replay` | ttl, request id uniqueness, payload hash match, expiry | 4 | 4 | 3 | +1 | High |
| `issue_delegation` path | config gate, caller relation, claims validity, signature validity | 4 | 4 | 4 | 0 | Medium-high |
| `issue_role_attestation` path | subject/role/subnet/audience/ttl validation | 5 | 5 | 5 | 0 | High |

## STEP 9 — Decision Surface Size

| Enum | Decision Sites | Previous | Delta | Risk |
| ---- | ----: | ----: | ----: | ---- |
| `Request` | 94 | 32 | +62 | High |
| `Response` | 43 | 33 | +10 | Medium |
| `CapabilityProof` | 13 | 0 | +13 | High |
| `CapabilityService` | 11 | 0 | +11 | Medium-high |
| `RootCapability` | 34 | 32 | +2 | Medium |
| `RootCapabilityMetricEvent` | 24 | 20 | +4 | Medium-high |
| `DelegatedTokenOpsError` | 62 | 62 | 0 | High (already large) |

## STEP 10 — Refactor Noise Filter

| Signal | Raw Trend | Noise Classification | Adjusted Interpretation |
| ---- | ---- | ---- | ---- |
| Files touched per slice dropped (`27 -> 20 -> 9`) | Down | Structural improvement | Better containment trend |
| CAF dropped (`45 -> 35 -> 30`) | Down | Structural improvement | Less cross-subsystem amplification |
| Enum shock remained high for auth/capability families | Flat-to-up | Not transient | Decision density is still concentrated |
| `api/rpc.rs` LOC grew by +825 in one slice | Up sharply | Not transient | New velocity drag hotspot |

## STEP 11 — Velocity Risk Index

| Area | Score | Weight | Weighted Score |
| ---- | ----: | ----: | ----: |
| enum shock radius | 7 | 3 | 21 |
| CAF trend | 5 | 2 | 10 |
| cross-layer leakage | 4 | 2 | 8 |
| gravity-well growth | 7 | 2 | 14 |
| edit blast radius | 5 | 1 | 5 |

`overall_index = 58 / 10 = 5.80`

Interpretation: **Moderate risk**; architecture still moves, but proof/replay/auth decision density is a velocity drag vector.

## Final Output

1. Velocity Risk Index (latest rerun): **4.50/10** (initial same-day run: `5.80/10`).
2. Revised CAF trend is down, but ELS remains low (`0.30..0.40`).
3. Boundary leakage is stable, with one persistent workflow direct-storage crossing.
4. Gravity growth concentration on control-plane hubs was reduced by decomposition (`api/rpc/mod.rs`, `workflow/rpc/request/handler/mod.rs`, `ops/auth/mod.rs`).
5. Highest shock radius remains `DelegatedTokenOpsError` (`2232`).
6. Blast radius is moderate (`avg 18.7`, `p95 27`, slice-sampled).
7. Independence pressure remains high outside `dto`.
8. Independent-axis growth is highest in the capability envelope path.
9. Decision surface expansion is strongest for `Request` and new capability enums.
10. Most remaining signal is decision-surface density (enum shock), not control-plane module accretion.
