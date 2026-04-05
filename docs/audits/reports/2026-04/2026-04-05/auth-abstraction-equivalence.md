# Auth Abstraction Equivalence Invariant Audit - 2026-04-05

## Report Preamble

- Scope: macro-generated authenticated endpoint expansion, access-expression runtime dispatch, canonical verifier parity, and delegated-session interaction with raw-caller predicates
- Compared baseline report path: `docs/audits/reports/2026-03/2026-03-10/auth-abstraction-equivalence.md`
- Code snapshot identifier: `c027b8df`
- Method tag/version: `Method V4.1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-05T16:48:44Z`
- Branch: `main`
- Worktree: `dirty`

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Macro path still converges on canonical verifier | PASS | authenticated macro expansion still routes through `crates/canic-dsl-macros/src/endpoint/expand.rs`, then `crates/canic-core/src/access/expr.rs`, then `crates/canic-core/src/access/auth.rs`. |
| Generated/auth-helper path preserves verifier ordering | PASS | `eval_access(...)` still resolves authenticated predicates through `delegated_token_verified(...)`, which calls the canonical `verify_token(...)` path in `crates/canic-core/src/access/auth.rs`. |
| Convenience delegated-session path does not weaken raw-caller predicates | PASS | current PocketIC coverage proves delegated-session bootstrap affects only the authenticated guard lane while raw caller checks for role attestation, capability routing, and subnet predicates continue to use transport caller semantics. |
| Failure semantics remain equivalent for missing scope | PASS | missing-scope denial still comes from the canonical auth path in `crates/canic-core/src/access/auth.rs`, and the local unit test still rejects it without a helper-specific failure branch. |
| Valid / expired / missing-scope abstraction coverage still exists | PASS | `crates/canic-tests/tests/delegation_flow.rs` still carries the certified-path parity tests, though they self-skip on the non-`ic` local lane; current local PocketIC evidence covers the abstraction/raw-caller split directly. |

## Comparison to Previous Relevant Run

- Stable: macro and DSL auth paths still converge on the canonical verifier.
- Stable: no abstraction-specific bypass branch was found.
- Improved: today’s PocketIC evidence is stronger than the March baseline because it explicitly proves delegated-session convenience affects only the authenticated guard abstraction and does not leak into handwritten raw-caller capability or attestation checks.
- Stable: missing-scope denial still routes through the same canonical auth path as before.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-dsl-macros/src/endpoint/expand.rs` | access expansion block | abstraction wiring into auth runtime | High |
| `crates/canic-core/src/access/expr.rs` | `eval_access` | canonical predicate dispatch surface used by generated endpoints | High |
| `crates/canic-core/src/access/auth.rs` | `delegated_token_verified`, `verify_token` | canonical verifier behavior baseline | High |
| `crates/canic-core/src/api/auth/session/mod.rs` | delegated-session bootstrap helpers | convenience/auth abstraction seam that must not alter raw-caller semantics | Medium |
| `crates/canic-core/tests/pic_role_attestation.rs` | delegated-session parity regressions | current strongest end-to-end equivalence evidence | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/access/expr.rs` | `access, ids, log` | 3 | 1 | 7 |
| `crates/canic-core/src/access/auth.rs` | `access, ops, dto, config, ids` | 5 | 2 | 6 |
| `crates/canic-dsl-macros/src/endpoint/expand.rs` | `endpoint parse/expand, metrics, access wrapper` | 3 | 1 | 4 |
| `crates/canic-core/src/api/auth/session/mod.rs` | `api, ops, storage, metrics` | 4 | 2 | 5 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| growing hub module | `crates/canic-core/src/access/expr.rs` | still a high-fan-in auth dispatch file | Medium |
| growing hub module | `crates/canic-core/tests/pic_role_attestation.rs` | repeated edits across the auth hardening line and now carries much of the parity/runtime story | Medium |
| dependency fan-in hub | `access::expr` | referenced in `28` crate files across access/api/workflow/tests/macros | High |
| cross-layer struct spread | `VerifiedDelegatedToken` | shared across access/ops/tests | Medium |

## Dependency Fan-In Pressure

### Module Fan-In

| Module | Import Count | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `access::expr` | 28 | `access/api/workflow/tests/macros` | High |
| `access::auth` | 22 | `access/api/workflow/tests/macros` | High |

### Struct Fan-In

| Struct | Defined In | Reference Count | Risk |
| --- | --- | ---: | --- |
| `DelegationProof` | `crates/canic-core/src/dto/auth.rs` | 25 | High |
| `DelegatedTokenClaims` | `crates/canic-core/src/dto/auth.rs` | 10 | Medium |

## Risk Score

Risk Score: **3 / 10**

Score contributions:
- `+1` auth abstraction dispatch is still concentrated in `access::expr`
- `+1` the runtime parity story is still heavily concentrated in one PocketIC suite
- `+1` auth DTO surfaces remain broadly referenced

Verdict: **Invariant holds with low residual coupling risk.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-core --test pic_role_attestation delegated_session_bootstrap_affects_authenticated_guard_only -- --test-threads=1 --nocapture` | PASS | proves delegated-session bootstrap changes authenticated guard behavior only where intended |
| `cargo test -p canic-core --test pic_role_attestation delegated_session_does_not_affect_role_attestation_or_capability_raw_caller_checks -- --test-threads=1 --nocapture` | PASS | proves raw-caller attestation/capability predicates remain transport-caller based |
| `cargo test -p canic-core --lib required_scope_rejects_when_scope_missing -- --nocapture` | PASS | missing-scope failure still comes from canonical auth path |
| `cargo test -p canic-tests --test delegation_flow authenticated_rpc_flow -- --nocapture` | PASS | certified parity tests still exist, but current local lane self-skips them outside `BuildNetwork::Ic` |

## Follow-up Actions

1. Keep watching `crates/canic-core/src/access/expr.rs`; it remains the largest abstraction-equivalence pressure point.
2. Re-run this audit after any macro auth-wiring change in `crates/canic-dsl-macros/src/endpoint/expand.rs` or delegated-session semantics change in `crates/canic-core/src/api/auth/session/mod.rs`.
