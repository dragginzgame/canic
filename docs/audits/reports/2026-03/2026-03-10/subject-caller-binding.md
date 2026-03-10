# Subject-Caller Binding Invariant Audit - 2026-03-10

## Report Preamble

- Scope: delegated-token verification path and authenticated endpoint dispatch
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
| Canonical subject binding in verifier | PASS | `access/auth.rs` enforces `enforce_subject_binding(verified.claims.sub, caller)` before scope check |
| No bearer fallback path | PASS | scans for relay/bypass patterns found no production bypass route |
| Caller input not ignored | PASS | `access/expr.rs` threads `ctx.caller` into `delegated_token_verified(...)` |
| Macro/DSL path preserves binding | PASS | `canic-dsl-macros/src/endpoint/expand.rs` routes through `eval_access(...)` |
| Canonical mismatch test coverage | PASS | `crates/canic/tests/delegation_flow.rs` checks mismatched caller rejects with `does not match caller` |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/access/auth.rs` | `verify_token`, `enforce_subject_binding` | canonical invariant enforcement point | High |
| `crates/canic-core/src/access/expr.rs` | `eval_access` | dispatcher path into auth verifier | Medium |
| `crates/canic-dsl-macros/src/endpoint/expand.rs` | endpoint expansion | abstraction wiring to canonical auth path | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/access/auth.rs` | `access, ops, dto, config, ids` | 4 | 2 | 6 |
| `crates/canic-core/src/access/expr.rs` | `access, ids, log` | 3 | 1 | 7 |
| `crates/canic-dsl-macros/src/endpoint/expand.rs` | `endpoint parse/expand` | 2 | 1 | 4 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| enum shock radius | `crates/canic-core/src/access/mod.rs` | `AccessError` referenced in `8` files | Low |
| cross-layer struct spread | `DelegatedTokenClaims` | referenced in `7` files across `api/ops` | Low |
| growing hub module | `crates/canic-core/src/access/auth.rs` | touched in `19` recent commits | Medium |

## Dependency Fan-In Pressure

### Module Fan-In

| Module | Import Count | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `dto` | 8 | `api/dto` | Hub forming |

### Struct Fan-In

| Struct | Defined In | Reference Count | Risk |
| --- | --- | ---: | --- |
| `DelegatedTokenClaims` | `crates/canic-core/src/dto/auth.rs` | 7 | Low |
| `DelegationProof` | `crates/canic-core/src/dto/auth.rs` | 9 | Medium |

## Risk Score

Risk Score: **2 / 10**

No invariant failure found. Score remains negligible-low with only routine churn pressure around the canonical auth boundary.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-core --lib --locked` | PASS | includes subject-binding unit tests |
| `cargo test -p canic --test delegation_flow --locked` | PASS | includes macro-path mismatched-caller rejection |

## Follow-up Actions

No immediate follow-up required for this run.
