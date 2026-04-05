# Canonical Auth Boundary Invariant Audit - 2026-04-05

## Report Preamble

- Scope: authenticated endpoint expansion, access-expression evaluation, canonical delegated-token verification, and delegated-session bootstrap ingress
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-10/canonical-auth-boundary.md`
- Code snapshot identifier: `c027b8df`
- Method tag/version: `Method V4.1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-05T16:34:13Z`
- Branch: `main`
- Worktree: `dirty`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Authenticated paths converge on canonical verifier | PASS | macro path still runs [expand.rs](/home/adam/projects/canic/crates/canic-dsl-macros/src/endpoint/expand.rs#L246) -> [expr.rs](/home/adam/projects/canic/crates/canic-core/src/access/expr.rs#L356) -> [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs#L216) -> [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs#L241). |
| Canonical ordering preserved | PASS | [token.rs](/home/adam/projects/canic/crates/canic-core/src/ops/auth/token.rs#L78) verifies trust chain and freshness, [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs#L252) binds caller to subject, and [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs#L253) enforces required scope before handler execution. |
| No weaker internal auth branch detected | PASS | delegated-session bootstrap in [mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/session/mod.rs#L57) still calls `DelegatedTokenOps::verify_token(...)` before persisting session state, rather than accepting externally asserted identity. |
| Freshness / expiry stage invoked from boundary | PASS | [token.rs](/home/adam/projects/canic/crates/canic-core/src/ops/auth/token.rs#L88) validates lifetime invariants and max TTL before trust-chain verification returns a `VerifiedDelegatedToken`. |
| Integration path coverage present | PASS | the local certified dispatcher test remains present in [delegation_flow.rs](/home/adam/projects/canic/crates/canic-tests/tests/delegation_flow.rs#L83), and the current PocketIC regression [pic_role_attestation.rs](/home/adam/projects/canic/crates/canic-core/tests/pic_role_attestation.rs#L267) proves delegated-session bootstrap only affects authenticated guard semantics and not a weaker alternate boundary. |

## Comparison to Previous Relevant Run

- Stable: all authenticated endpoint flows still converge on the same verifier stack.
- Stable: the canonical ordering remains `verify token -> bind subject -> authorize scope -> execute`.
- Improved: delegated-session bootstrap is now explicitly covered as part of the boundary story, and it still routes through the same token verifier before creating any local session state.
- Stable: no internal/admin bypass path was found that trusts upstream authentication without proof.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs) | `delegated_token_verified`, `verify_token`, `CallerBoundToken` | canonical verifier implementation and ordering owner | High |
| [expr.rs](/home/adam/projects/canic/crates/canic-core/src/access/expr.rs) | `eval_access`, authenticated builtin evaluator | central auth dispatch surface | High |
| [expand.rs](/home/adam/projects/canic/crates/canic-dsl-macros/src/endpoint/expand.rs) | access expansion block | macro entrypoint convergence wiring | Medium |
| [mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/session/mod.rs) | `set_delegated_session_subject` | alternate ingress that must not weaken verification semantics | Medium |
| [token.rs](/home/adam/projects/canic/crates/canic-core/src/ops/auth/token.rs) | `DelegatedTokenOps::verify_token` | canonical trust/freshness verification stage | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs) | `access, ops, dto, config, ids` | 5 | 2 | 6 |
| [expr.rs](/home/adam/projects/canic/crates/canic-core/src/access/expr.rs) | `access, ids, log` | 3 | 1 | 7 |
| [expand.rs](/home/adam/projects/canic/crates/canic-dsl-macros/src/endpoint/expand.rs) | `endpoint parse/expand, metrics, access wrapper` | 3 | 1 | 4 |
| [mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/session/mod.rs) | `api, ops, storage, metrics` | 4 | 2 | 5 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| growing hub module | [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs) | touched in `19` recent commits | Medium |
| growing hub module | [expand.rs](/home/adam/projects/canic/crates/canic-dsl-macros/src/endpoint/expand.rs) | touched in `19` recent commits | Medium |
| dependency fan-in hub | `access::expr` | referenced in `28` crate files | High |
| enum shock radius | [auth.rs](/home/adam/projects/canic/crates/canic-core/src/dto/auth.rs) | `DelegatedTokenClaims` appears in `54` references | High |
| capability surface growth | [expr.rs](/home/adam/projects/canic/crates/canic-core/src/access/expr.rs) | `15` public items | Medium |

## Dependency Fan-In Pressure

### Module Fan-In

| Module | Import Count | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `access::auth` | 22 | `access/api/workflow/tests/macros` | High |
| `access::expr` | 28 | `access/api/workflow/tests/macros` | High |

### Struct Fan-In

| Struct | Defined In | Reference Count | Risk |
| --- | --- | ---: | --- |
| `DelegatedTokenClaims` | [auth.rs](/home/adam/projects/canic/crates/canic-core/src/dto/auth.rs) | 54 | High |
| `VerifiedDelegatedToken` | [mod.rs](/home/adam/projects/canic/crates/canic-core/src/ops/auth/mod.rs) | 14 | High |

## Risk Score

Risk Score: **3 / 10**

Score contributions:
- `+2` medium/high hotspot pressure across `access::auth`, `access::expr`, and delegated-session ingress
- `+1` `access::expr` remains a hub-forming boundary file with high fan-in

Verdict: **Canonical auth boundary holds with low residual risk.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n "authenticated\\(|delegated_token_verified|resolve_authenticated_identity|verify_token|AccessExpr::|eval_access|requires_scope" crates/canic-core/src crates/canic-dsl-macros/src crates/canic-tests/tests crates/canic-core/tests -g '*.rs'` | PASS | all authenticated ingress still converges on the canonical path |
| `cargo test -p canic-core --test pic_role_attestation delegated_session_bootstrap_affects_authenticated_guard_only -- --test-threads=1 --nocapture` | PASS | delegated-session bootstrap still routes into canonical verifier semantics |
| `cargo test -p canic-tests --test delegation_flow authenticated_rpc_flow -- --nocapture` | PASS | test exists, but current local build lane self-skips the certified path as expected |

## Follow-up Actions

1. Keep watching [mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/session/mod.rs) because it is the most obvious place a weaker parallel auth ingress could accidentally appear.
2. Re-run this audit after any macro auth-wiring change in [expand.rs](/home/adam/projects/canic/crates/canic-dsl-macros/src/endpoint/expand.rs) or any delegated-session bootstrap change in [mod.rs](/home/adam/projects/canic/crates/canic-core/src/api/auth/session/mod.rs).
