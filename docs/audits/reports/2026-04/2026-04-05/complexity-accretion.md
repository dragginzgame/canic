# Complexity Accretion Audit - 2026-04-05

## Report Preamble

- Scope: `crates/canic-core/src/**`
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-10/complexity-accretion.md`
- Code snapshot identifier: `c027b8df`
- Method tag/version: `Method V4.1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-05T16:48:44Z`
- Branch: `main`
- Worktree: `dirty`

## Baseline Capture

| Metric | Previous | Current | Delta |
| ---- | ----: | ----: | ----: |
| Total runtime files in scope | 326 | 331 | +5 |
| Runtime logical LOC | 25372 | 28019 | +2647 |
| Runtime files >= 600 LOC | 7 | 4 | -3 |
| `Request` reference files | 14 | 14 | 0 |
| `Response` reference files | 13 | 14 | +1 |
| `CapabilityProof` reference files | 7 | 7 | 0 |
| `CapabilityService` reference files | 7 | 8 | +1 |
| `DelegationValidationError` reference files | 11 | 13 | +2 |

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Runtime size baseline captured | PASS | current runtime scope is `331` files and `28019` logical LOC, up `+5` files and `+2647` LOC versus the March baseline |
| Variant surface growth remains controlled in core RPC/capability enums | PASS | `Request` and `Response` are still `5` variants each, `CapabilityProof` is still `3`, and `CapabilityService` is still `1` |
| Complexity pressure moved rather than disappeared | PASS | the biggest runtime hub files are now `ops/runtime/metrics/auth.rs`, `access/expr.rs`, `config/schema/subnet.rs`, and `storage/stable/auth.rs`, while the old `workflow/runtime/mod.rs` hotspot dropped out of the top tier |
| Auth/state surfaces are now the main cognitive stack | PASS | `storage/stable/auth.rs`, `access/auth.rs`, and `ops/runtime/metrics/auth.rs` now carry more branch/state pressure than the March runtime/bootstrap seams |
| Test complexity still exceeds runtime complexity in one request lane | PASS | `workflow/rpc/request/handler/tests.rs` is still the largest file in scope at `1028` logical LOC, but it remains test-only and is tracked separately from runtime risk |

## Structural Hotspots

### Runtime Complexity Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/access/expr.rs` | `eval_access` and builtin predicate dispatch | still the largest runtime control-surface file at `798` logical LOC and very high fan-in | High |
| `crates/canic-core/src/storage/stable/auth.rs` | delegated session / proof / key state storage | `709` logical LOC and dense auth-state branching | High |
| `crates/canic-core/src/ops/runtime/metrics/auth.rs` | auth metric event plumbing | `936` logical LOC and growing auth event taxonomy | Medium |
| `crates/canic-core/src/access/auth.rs` | authenticated identity resolution and canonical verifier wrapper | highest sampled branch density in this run | Medium |
| `crates/canic-core/src/ops/storage/intent.rs` | intent aggregation/storage ops | still a stable medium-high state transition hotspot | Medium |

### Test Complexity Hotspots

| Test File / Module | Reason | Tracking Impact |
| --- | --- | --- |
| `crates/canic-core/src/workflow/rpc/request/handler/tests.rs` | still the single largest file in scope (`1028` logical LOC) | Medium |
| `crates/canic-core/src/ops/auth/tests.rs` | auth trust/audience/replay test surface is now `646` logical LOC | Medium |
| `crates/canic-core/src/api/auth/tests.rs` | auth API / audience / proof-store tests are now `578` logical LOC and near the top tier | Low |

## Control Surface Detection

| Control Surface | File | Responsibility | Risk |
| --- | --- | --- | --- |
| `eval_access` | `crates/canic-core/src/access/expr.rs` | unified auth predicate dispatch | High |
| canonical auth wrapper | `crates/canic-core/src/access/auth.rs` | caller binding, delegated-session resolution, scope enforcement | Medium |
| auth-state storage | `crates/canic-core/src/storage/stable/auth.rs` | delegated session / proof / key persistence rules | Medium |
| auth metrics lane | `crates/canic-core/src/ops/runtime/metrics/auth.rs` | event taxonomy and auth/replay telemetry surface | Medium |

## Branching Density

| File | Logical LOC | `match` | `if` | `else if` | Branch Density (/100 LOC) | Runtime/Test | Risk |
| --- | ---: | ---: | ---: | ---: | ---: | --- | --- |
| `crates/canic-core/src/access/auth.rs` | 531 | 3 | 25 | 0 | 5.27 | Runtime | High |
| `crates/canic-core/src/storage/stable/auth.rs` | 709 | 1 | 23 | 0 | 3.39 | Runtime | High |
| `crates/canic-core/src/ops/storage/intent.rs` | 581 | 6 | 13 | 0 | 3.27 | Runtime | High |
| `crates/canic-core/src/access/expr.rs` | 798 | 13 | 3 | 0 | 2.01 | Runtime | Medium |
| `crates/canic-core/src/ops/runtime/metrics/auth.rs` | 936 | 12 | 2 | 0 | 1.50 | Runtime | Medium |
| `crates/canic-core/src/workflow/rpc/request/handler/tests.rs` | 1028 | mixed | mixed | mixed | large harness | Test | Low |

## Variant Surface Growth

| Enum | Variants | Previous | Delta | Switch Sites | Branch Multiplier | Enum Density | Mixed Domain? | Risk |
| ---- | ----: | ----: | ----: | ----: | ----: | ----: | ---- | ---- |
| `dto::rpc::Request` | 5 | 5 | 0 | 14 refs / several flow matches | stable | 14 / 331 = 0.04 | Yes | Medium |
| `dto::rpc::Response` | 5 | 5 | 0 | 14 refs / several flow matches | stable | 14 / 331 = 0.04 | Yes | Medium |
| `dto::capability::CapabilityProof` | 3 | 3 | 0 | 7 refs | stable | 7 / 331 = 0.02 | Yes | Medium |
| `dto::capability::CapabilityService` | 1 | 1 | 0 | 8 refs | low | 8 / 331 = 0.02 | No | Low |
| `access::expr::BuiltinPredicate` | 4 | 4 | 0 | central dispatch in `access/expr.rs` | stable | high fan-in via `access::expr` | Yes | Medium |
| `ops::runtime::metrics::RootCapabilityMetricEventType` | 5 | 5 | 0 | metrics-only routing | low | narrow | No | Low |
| `ops::auth::error::DelegationValidationError` | 10 | 10 | 0 | 13 refs | steady upward reference radius | 13 / 331 = 0.04 | Yes | Medium |
| `error::InternalErrorClass` | 6 | 6 | 0 | 5 refs | low | 5 / 331 = 0.02 | Yes | Low |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/access/expr.rs` | `access, dto, ids, log` | 4 | 2 | 7 |
| `crates/canic-core/src/access/auth.rs` | `access, ops, dto, config, ids` | 5 | 2 | 7 |
| `crates/canic-core/src/storage/stable/auth.rs` | `storage, dto, ids` | 3 | 1 | 6 |
| `crates/canic-core/src/ops/runtime/metrics/auth.rs` | `ops, runtime, metrics, dto` | 4 | 1 | 5 |

## Primary Architectural Pressure

`crates/canic-core/src/access/expr.rs`

Reasons:
- still the highest-fan-in runtime auth dispatch file
- still large at `798` logical LOC
- sits on the abstraction/auth boundary, so every new predicate or semantic split pays rent here

Secondary pressure has shifted into `storage/stable/auth.rs` and `access/auth.rs`, which means the complexity center of gravity is moving from generic workflow/runtime coordination into auth state and auth semantics.

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| growing hub module | `crates/canic-core/src/access/expr.rs` | remains high-fan-in and still one of the largest runtime files | Medium |
| growing hub module | `crates/canic-core/src/storage/stable/auth.rs` | grew into the runtime top tier and is now `709` logical LOC | High |
| growing hub module | `crates/canic-core/src/ops/runtime/metrics/auth.rs` | now the single largest runtime file at `936` logical LOC | High |
| enum shock radius | `crates/canic-core/src/dto/auth.rs` | `RoleAttestation` and `DelegationProof` remain widely referenced across auth/api/tests | High |
| capability surface growth | auth metrics lane | auth metrics surface is now large enough to be a separate conceptual subsystem | Medium |

## Dependency Fan-In Pressure

### Module Fan-In

| Module | Import Count | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `access::expr` | 28 | `access/api/workflow/tests/macros` | Architectural gravity well |
| `access::auth` | 22 | `access/api/workflow/tests/macros` | Architectural gravity well |
| auth verification / proof surfaces | broad | `ops/api/tests` | Hub forming |

### Struct Fan-In

| Struct | Defined In | Reference Count | Layers Referencing | Risk |
| --- | --- | ---: | --- | --- |
| `RoleAttestation` | `crates/canic-core/src/dto/auth.rs` | 35 | `api/ops/workflow/tests` | High |
| `DelegationProof` | `crates/canic-core/src/dto/auth.rs` | 25 | `api/ops/workflow/tests` | High |
| `DelegatedTokenClaims` | `crates/canic-core/src/dto/auth.rs` | 10 | `api/ops/tests` | Medium |

## Risk Score

Risk Score: **6 / 10**

Score contributions:
- `+2` auth complexity center moved into larger, denser runtime files (`storage/stable/auth.rs`, `access/auth.rs`, `ops/runtime/metrics/auth.rs`)
- `+2` `access::expr` and `access::auth` remain architectural gravity wells
- `+1` auth DTO cross-layer spread is still high
- `+1` the runtime large-file count improved, but overall runtime LOC still rose materially

Interpretation: **moderate complexity drift**, now concentrated more in auth state + auth semantics than in generic workflow/runtime coordination.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `find crates/canic-core/src -name '*.rs' | wc -l` | PASS | runtime file count captured |
| logical LOC / large-file scans over `crates/canic-core/src` | PASS | runtime LOC and `>=600` file set captured with comments/blanks filtered |
| enum/reference scans over `Request`, `Response`, `CapabilityProof`, `CapabilityService`, `DelegationValidationError`, `InternalErrorClass` | PASS | current variant/reference radius captured |
| recent-touch scan over `access/expr.rs`, `storage/stable/auth.rs`, `ops/runtime/metrics/auth.rs`, `ops/storage/intent.rs` | PASS | current pressure trend captured |

## Follow-up Actions

1. Keep `0.25` fixes away from expanding `access::expr.rs` further unless they reduce another higher-cost seam at the same time.
2. Consider splitting `storage/stable/auth.rs` by concern if the auth state model grows again; it is now a more important complexity hotspot than the older runtime/bootstrap files.
3. Re-run this audit after any new auth metric taxonomy or delegated-session/proof-state expansion.
