# Velocity Preservation Audit — 2026-03-08

## Run Context

- Audit run: `velocity-preservation`
- Definition: `docs/audits/recurring/velocity-preservation.md`
- Auditor: `codex`
- Date (UTC): `2026-03-08 19:22:56Z`
- Branch: `eleven`
- Commit: `c98bb574`
- Worktree: `dirty`
- Scope: `crates/canic-core/src/**`
- Previous baseline: `docs/audits/reports/2026-03/2026-03-07/velocity-preservation.md` (latest rerun `02ac3107`)

Rerun note:
- Boundary and replay/auth hot paths were re-scanned in this pass after enum-surface decomposition.
- `BuiltinPredicate` and root-capability outcome shock rows were refreshed from current code.

## STEP 0 — Baseline Capture

| Metric | Previous | Current | Delta |
| ---- | ----: | ----: | ----: |
| Velocity Risk Index | 3.10 | 2.80 | -0.30 |
| Cross-layer leakage crossings | 1 | 0 | -1 |
| Avg files touched per feature slice (sampled) | 18.7 | 16.0 | -2.7 |
| p95 files touched (sampled) | 27 | 30 | +3 |
| Top gravity-well fan-in (proxy) | 6 | 5 | -1 |

## STEP 1 — Change Surface Mapping (Empirical, Revised CAF)

| Feature | Files Modified | Subsystems | Layers | Flow Axes | Revised CAF | ELS | Containment Score | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ----: | ----: | ---- |
| `0.13.4` verifier/replay/auth consolidation | 27 | 6 | 5 | 4 | 24 | 0.41 | 0.40 | Medium |
| `0.13.3` auth facade split + audit refresh | 9 | 4 | 4 | 3 | 12 | 0.44 | 0.27 | Medium-low |
| `0.13.2` control-plane decomposition continuation | 20 | 7 | 7 | 5 | 35 | 0.40 | 0.47 | Medium-high |

Trend:
- CAF remains below pre-0.13.1 hub pressure phases.
- ELS improved for the latest slice but is still below the `>0.70` strong-locality target.

## STEP 2 — Boundary Leakage (Mechanical + Triaged)

| Boundary | Import Crossings | Previous | Delta | Risk |
| ---- | ----: | ----: | ----: | ---- |
| endpoints/api -> model/storage direct references | 0 | 0 | 0 | Low |
| workflow -> model/storage direct references | 0 | 1 | -1 | Low |
| policy -> dto/ops/runtime references | 0 | 0 | 0 | Low |
| ops -> workflow references | 0 | 0 | 0 | Low |
| auth/capability DTO leakage into storage ownership | 0 | 0 | 0 | Low |

Triage note:
- No runtime workflow storage crossings remain; one storage import in `workflow/rpc/request/handler/mod.rs` is test-gated (`#[cfg(test)]`).

## STEP 3 — Gravity Well Growth Rate

| Module | LOC | LOC Delta | Fan-In | Fan-In Delta | Domains | Edit Frequency (30d) | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ----: | ---- |
| `workflow/rpc/request/handler/mod.rs` | 189 | -29 | 3 | 0 | 2 | 6 | Low |
| `api/rpc/mod.rs` | 62 | 0 | 3 | 0 | 2 | 4 | Low |
| `api/rpc/capability/mod.rs` | 203 | +3 | 4 | 0 | 3 | 5 | Medium |
| `ops/auth/mod.rs` | 79 | +3 | 5 | 0 | 1 | 5 | Low |
| `ops/replay/guard.rs` | 225 | +225 | 3 | +1 | 2 | 3 | Medium |

Escalation condition:
- No current module satisfies high fan-in + high growth gravity-well escalation.

## STEP 4 — Change Multiplier Matrix (Deterministic)

| Feature Axis | Endpoints | Workflow | Policy | Ops | Model/Storage | Subsystem Count |
| ---- | ---- | ---- | ---- | ---- | ---- | ----: |
| proof mode selection | X | X |  | X |  | 3 |
| replay state semantics | X | X |  | X | X | 4 |
| caller topology relation |  | X | X | X | X | 4 |
| role/subnet context | X | X | X | X | X | 5 |

| Candidate Feature | Axes Involved | Subsystem Count | Friction |
| ---- | ---- | ----: | ---- |
| Add new proof mode | proof + replay + role/subnet | 5 | High |
| Add new root capability family | proof + replay + topology + role/subnet | 5 | High |
| Replay policy tweak | replay + topology | 4 | Medium |

## STEP 5 — Enum Shock Radius (Density-Adjusted)

| Enum | Variants | Switch Sites | Modules Using Enum | Switch Density | Subsystems | Shock Radius | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ----: | ---- |
| `dto::rpc::Request` | 5 | 18 | 4 | 4.50 | 4 | 90.00 | Medium |
| `dto::rpc::Response` | 5 | 31 | 9 | 3.44 | 4 | 68.89 | Medium |
| `dto::capability::CapabilityProof` | 3 | 15 | 5 | 3.00 | 3 | 27.00 | Medium |
| `access::expr::BuiltinPredicate` | 4 | 16 | 1 | 16.00 | 1 | 64.00 | Low |
| `RootCapabilityMetricEventType` | 5 | 26 | 5 | 5.20 | 3 | 78.00 | Medium |
| `RootCapabilityMetricOutcome` | 9 | 24 | 1 | 24.00 | 1 | 216.00 | Medium |
| `RootCapabilityMetricProofMode` | 4 | 11 | 2 | 5.50 | 2 | 44.00 | Low |
| `DelegatedTokenOpsError` | 4 | 22 | 4 | 5.50 | 2 | 44.00 | Low |

Trend note:
- The previous critical shock source (`DelegatedTokenOpsError` monolith) is substantially reduced at top-level decision surfaces.

## STEP 6 — Edit Blast Radius (Empirical)

| Metric | Current | Previous | Delta |
| ---- | ----: | ----: | ----: |
| average files touched per feature slice | 16.0 | 18.7 | -2.7 |
| median files touched | 16 | 20 | -4 |
| p95 files touched | 30 | 27 | +3 |

Method note:
- `slice-sampled` from recent 0.13 patch slices.

## STEP 7 — Subsystem Independence Score (Size-Adjusted)

| Subsystem | Internal Imports | External Imports | LOC | Independence | Adjusted Independence | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ---- |
| `dto` | 17 | 5 | 1673 | 0.773 | 5.737 | Low |
| `ops` | 3 | 63 | 10171 | 0.045 | 0.414 | High |
| `domain` | 2 | 15 | 1289 | 0.118 | 0.846 | Medium |
| `storage` | 1 | 22 | 2654 | 0.043 | 0.339 | High |
| `api` | 1 | 25 | 2742 | 0.038 | 0.302 | High |
| `workflow` | 1 | 60 | 7982 | 0.016 | 0.146 | High |

## STEP 8 — Decision-Axis Growth (Independence-Aware)

| Operation | Axes | Axis Count | Independent Axes | Previous Independent Axes | Delta | Risk |
| ---- | ---- | ----: | ----: | ----: | ----: | ---- |
| `response_capability_v1` | family, proof mode, envelope validity, metadata validity | 4 | 4 | 4 | 0 | High |
| `response_with_pipeline` | pipeline order, replay status, policy outcome, capability family | 4 | 4 | 4 | 0 | High |
| `check_replay` | ttl, request id uniqueness, payload hash match, expiry | 4 | 3 | 4 | -1 | Medium |
| `issue_delegation` path | config gate, caller relation, claims validity, signature validity | 4 | 4 | 4 | 0 | Medium-high |

## STEP 9 — Decision Surface Size

| Enum | Decision Sites | Previous | Delta | Risk |
| ---- | ----: | ----: | ----: | ---- |
| `Request` | 18 | 94 | -76 | Medium |
| `Response` | 31 | 43 | -12 | Medium |
| `CapabilityProof` | 15 | 13 | +2 | Medium-high |
| `CapabilityService` | 11 | 11 | 0 | Medium |
| `RootCapabilityMetricEventType` | 26 | 24 | +2 | Medium |
| `RootCapabilityMetricOutcome` | 24 | 0 | +24 | Medium |
| `RootCapabilityMetricProofMode` | 11 | 0 | +11 | Low |
| `DelegatedTokenOpsError` | 22 | 62 | -40 | Low |

## STEP 10 — Refactor Noise Filter

| Signal | Raw Trend | Noise Classification | Adjusted Interpretation |
| ---- | ---- | ---- | ---- |
| New `ops/replay/*` files | Up file count | Structural improvement | Replay ownership moved down-stack |
| Top-level auth shock radius reduced | Down sharply | Non-transient | Sustainable velocity gain |
| `Request` decision surface reduced (`94 -> 18`) | Down sharply | Non-transient | Major velocity drag reduced |
| `BuiltinPredicate` top-level variant surface reduced (`14 -> 4`) | Down sharply | Non-transient | DSL hotspot drag reduced |
| Root capability metrics split into axis enums (`event_type`/`outcome`/`proof_mode`) | Mixed (`+` axis types, `-` single-enum pressure) | Non-transient | Future metric growth no longer requires monolithic event enum expansion |

## STEP 11 — Velocity Risk Index

| Area | Score | Weight | Weighted Score |
| ---- | ----: | ----: | ----: |
| enum shock radius | 2 | 3 | 6 |
| CAF trend | 4 | 2 | 8 |
| cross-layer leakage | 2 | 2 | 4 |
| gravity-well growth | 3 | 2 | 6 |
| edit blast radius | 4 | 1 | 4 |

`overall_index = 28 / 10 = 2.80`

Interpretation: **Low-moderate risk**, improved from `4.50` and from earlier same-day reruns (`3.90`, `3.60`, `3.40`, `3.10`) after request, replay, predicate, and metric-axis decomposition.

## Final Output

1. Velocity Risk Index (2026-03-08 rerun): **2.80/10**.
2. At run time, remaining top velocity drag was no longer `Request`; highest current shock multipliers are `RootCapabilityMetricOutcome` and delegated validation/expiry enums.
3. `BuiltinPredicate` now has materially lower shock pressure after grouped-sub-enum decomposition (`14 -> 4` variants; shock radius `224.00 -> 64.00`).
4. Cross-layer leakage count is now zero for runtime paths.
5. Auth error taxonomy split materially lowered top-level enum shock.
