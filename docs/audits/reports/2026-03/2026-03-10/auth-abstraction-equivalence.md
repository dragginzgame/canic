# Auth Abstraction Equivalence Invariant Audit - 2026-03-10

## Report Preamble

- Scope: macro/DSL auth abstractions and canonical verifier equivalence
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
| Macro path converges on canonical verifier | PASS | `canic-dsl-macros/src/endpoint/expand.rs:248` calls `eval_access(...)` |
| Access expression runtime converges on auth verifier | PASS | `access/expr.rs` authenticated predicate path calls `delegated_token_verified(...)` |
| Equivalence failure modes covered | PASS | integration tests cover valid token, expired token, missing scope, mismatched caller |
| Canonical and abstraction failure semantics align | PASS | mismatched subject/caller rejection propagates through macro endpoint path |
| No helper-only bypass path detected | PASS | no alternate abstraction path with weaker checks observed |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-dsl-macros/src/endpoint/expand.rs` | macro expansion auth wiring | primary abstraction codegen path | High |
| `crates/canic-core/src/access/expr.rs` | `eval_access` | abstraction runtime dispatcher | High |
| `crates/canic-core/src/access/auth.rs` | `delegated_token_verified` | canonical verifier invoked by abstractions | Medium |
| `canisters/test/src/lib.rs` | macro-protected endpoint | integration parity surface | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/access/expr.rs` | `access, ids, log` | 3 | 1 | 7 |
| `crates/canic-dsl-macros/src/endpoint/expand.rs` | `endpoint parse/expand` | 2 | 1 | 4 |
| `crates/canic-core/src/access/auth.rs` | `access, ops, dto, config` | 4 | 2 | 6 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| growing hub module | `crates/canic-core/src/access/expr.rs` | `1023 LOC` + touched in `13` recent commits | Medium |
| enum shock radius | `crates/canic-core/src/access/mod.rs` | `AccessError` referenced in `8` files | Low |
| dependency fan-in hub | `dto` | imported in `8` files across `api/dto` | Low |

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

Abstraction-path equivalence checks passed with parity evidence in integration tests. Residual risk is low and tied to complexity concentration in auth expression dispatch.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic --test delegation_flow --locked` | PASS | macro-auth path tests passed (`7 passed`) |
| `cargo test -p canic-core --lib --locked` | PASS | canonical verifier unit tests passed |

## Follow-up Actions

1. Keep parity tests adjacent to macro-authenticated endpoints when new predicates are added.
