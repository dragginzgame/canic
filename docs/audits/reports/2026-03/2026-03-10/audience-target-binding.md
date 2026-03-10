# Audience Target Binding Invariant Audit - 2026-03-10

## Report Preamble

- Scope: delegated token audience/target binding to runtime context
- Compared baseline report path: `N/A` (first run for this scope on 2026-03-10)
- Code snapshot identifier: `fa06bfef`
- Method tag/version: `Method V4.0`
- Comparability status: `non-comparable` (method expanded with hotspots, predictive signals, fan-in pressure)
- Auditor: `codex`
- Run timestamp (UTC): `2026-03-10T14:30:36Z`
- Branch: `main`
- Worktree: `dirty`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Audience claim validation is explicit | PASS | `ops/auth/verify.rs::verify_self_audience` |
| Runtime target context is used | PASS | verifier compares audience against `self_pid` runtime canister principal |
| Ordering before authorization/handler | PASS | audience check occurs in token verification stage before scope authorization |
| Wrong-audience rejection path present | PASS | capability/delegation claim tests cover audience mismatch rejection |
| No caller-vs-audience confusion found | PASS | audience checks bind to service target, not ingress caller identity |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/ops/auth/verify.rs` | `verify_self_audience` | canonical audience binding check | High |
| `crates/canic-core/src/ops/auth/token.rs` | `verify_token_structure` | claims verification orchestration | Medium |
| `crates/canic-core/src/api/rpc/capability/grant.rs` | delegated grant claim validation | audience/issuer claim enforcement in capability flow | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/ops/auth/verify.rs` | `ops, dto, ids, log` | 4 | 1 | 5 |
| `crates/canic-core/src/ops/auth/token.rs` | `ops, dto, cdk` | 3 | 1 | 5 |
| `crates/canic-core/src/api/rpc/capability/grant.rs` | `api, dto, ops` | 3 | 2 | 5 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| enum shock radius | `crates/canic-core/src/ops/auth/error.rs` | `DelegationValidationError` referenced in `11` files | Medium |
| cross-layer struct spread | `DelegationProof` | referenced across `api/ops/workflow` | Medium |
| capability surface growth | `crates/canic-core/src/dto/auth.rs` | high auth DTO usage concentration | Low |

## Dependency Fan-In Pressure

### Module Fan-In

| Module | Import Count | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `dto` | 8 | `api/dto` | Hub forming |

### Struct Fan-In

| Struct | Defined In | Reference Count | Risk |
| --- | --- | ---: | --- |
| `DelegationProof` | `crates/canic-core/src/dto/auth.rs` | 9 | Medium |
| `DelegatedTokenClaims` | `crates/canic-core/src/dto/auth.rs` | 7 | Low |

## Risk Score

Risk Score: **2 / 10**

Audience-target binding checks are present and correctly ordered. Residual risk is low and mainly from auth DTO coupling pressure.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-core --lib --locked` | PASS | includes audience mismatch rejection tests |
| `cargo test -p canic --test delegation_flow --locked` | PASS | delegated token flow passed |

## Follow-up Actions

No immediate follow-up required for this run.
