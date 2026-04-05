# Subject-Caller Binding Invariant Audit - 2026-04-05

## Report Preamble

- Scope: delegated-token verification path, authenticated endpoint dispatch, delegated-session subject resolution, and current end-to-end subject/caller tests
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-10/subject-caller-binding.md`
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
| Canonical subject binding in verifier | PASS | [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs#L108) binds verified tokens with `CallerBoundToken::bind_to_caller(...)`, and [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs#L258) rejects `sub != caller` before scope authorization or handler execution. |
| No bearer fallback path | PASS | no production auth path verifies a delegated token without the caller-binding step; [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs#L216) routes delegated token auth only through `verify_token(...)`, which always caller-binds before returning. |
| Caller input not ignored | PASS | [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs#L226) passes the authenticated caller into `verify_token(...)`, and [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs#L252) consumes it in `CallerBoundToken::bind_to_caller(...)`. |
| Macro / DSL path preserves binding | PASS | [expand.rs](/home/adam/projects/canic/crates/canic-dsl-macros/src/endpoint/expand.rs#L246) resolves authenticated identity once, writes both transport caller and authenticated subject into the access context, and then calls `eval_access(...)` without bypassing the canonical auth path. |
| Canonical mismatch coverage exists | PASS | [delegation_flow.rs](/home/adam/projects/canic/crates/canic-tests/tests/delegation_flow.rs#L84) proves `token for A + caller B => Unauthorized`, and [pic_role_attestation.rs](/home/adam/projects/canic/crates/canic-core/tests/pic_role_attestation.rs#L912) proves delegated-session state does not rewrite raw caller checks for attestation or capability paths. |

## Comparison to Previous Relevant Run

- Stable: canonical caller-subject binding still lives in the auth verifier, not in endpoint-local logic.
- Stable: the macro/DSL expansion path still routes through `resolve_authenticated_identity(...)` and `eval_access(...)`.
- Improved: the current test surface is stronger than the March baseline because it now includes delegated-session end-to-end coverage proving that raw caller semantics are preserved for role attestation and capability checks.
- Stable: no bearer fallback or relay-style acceptance path was found in production auth flow.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs) | `CallerBoundToken::bind_to_caller`, `verify_token`, `enforce_subject_binding` | canonical subject-caller check location | High |
| [expr.rs](/home/adam/projects/canic/crates/canic-core/src/access/expr.rs) | `AccessContext`, authenticated builtin path | carries both transport caller and authenticated subject into predicate evaluation | Medium |
| [expand.rs](/home/adam/projects/canic/crates/canic-dsl-macros/src/endpoint/expand.rs) | access wrapper expansion | abstraction path into canonical verifier | Medium |
| [pic_role_attestation.rs](/home/adam/projects/canic/crates/canic-core/tests/pic_role_attestation.rs) | delegated-session raw-caller regression path | strongest current end-to-end proof that session state cannot act like a bearer caller override | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs) | `access, ops, dto, config, ids` | 5 | 2 | 6 |
| [expr.rs](/home/adam/projects/canic/crates/canic-core/src/access/expr.rs) | `access, ids, log` | 3 | 1 | 7 |
| [expand.rs](/home/adam/projects/canic/crates/canic-dsl-macros/src/endpoint/expand.rs) | `endpoint parse/expand, access wrapper, metrics` | 3 | 1 | 4 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| cross-layer struct spread | [auth.rs](/home/adam/projects/canic/crates/canic-core/src/dto/auth.rs) | `DelegatedTokenClaims` now has `54` references across the crate tree | Medium |
| growing hub module | [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs) | touched in `19` recent commits | Medium |
| growing hub module | [expr.rs](/home/adam/projects/canic/crates/canic-core/src/access/expr.rs) | touched in `16` recent commits | Medium |
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
| `ResolvedAuthenticatedIdentity` | [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs) | 6 | Low |

## Risk Score

Risk Score: **3 / 10**

Score contributions:
- `+1` `access::auth` remains the canonical high-value enforcement point
- `+1` `access::expr` has become a dense hub for transport-caller vs authenticated-subject semantics
- `+1` `DelegatedTokenClaims` has a large structural reference radius

Verdict: **Invariant holds with low residual risk.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n 'enforce_subject_binding|CallerBoundToken::bind_to_caller|resolve_authenticated_identity\(|delegated_token_verified\(' crates/canic-core/src crates/canic-dsl-macros/src -g '*.rs'` | PASS | canonical binding path remains intact |
| `cargo test -p canic-core --lib caller_predicates_use_transport_caller_not_authenticated_subject -- --nocapture` | PASS | proves caller predicates keep using transport caller semantics |
| `cargo test -p canic-core --test pic_role_attestation delegated_session_does_not_affect_role_attestation_or_capability_raw_caller_checks -- --test-threads=1 --nocapture` | PASS | proves delegated session state does not become a bearer caller override |

## Follow-up Actions

1. Keep `DelegatedTokenClaims` under watch in future invariant runs because its reference radius is now large enough to make auth DTO drift expensive.
2. Re-run this audit immediately after any change to [auth.rs](/home/adam/projects/canic/crates/canic-core/src/access/auth.rs), [expr.rs](/home/adam/projects/canic/crates/canic-core/src/access/expr.rs), or [expand.rs](/home/adam/projects/canic/crates/canic-dsl-macros/src/endpoint/expand.rs).
