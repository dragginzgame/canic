# Auth Abstraction Equivalence Invariant Audit - 2026-05-14

## Report Preamble

- Definition path: `docs/audits/recurring/invariants/auth-abstraction-equivalence.md`
- Scope: macro-generated authenticated endpoint expansion, access-expression
  runtime dispatch, delegated-token verifier parity, delegated-session identity
  resolution, transport-caller lane separation, and trust-chain guard
  integration
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-09/auth-abstraction-equivalence.md`
- Code snapshot identifier: `48213853`
- Method tag/version: `Method V4.3`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp: `2026-05-14`
- Branch: `main`
- Worktree: `dirty before report write`

## Audit Selection

This was selected as the next oldest latest-run recurring audit after the
same-day `token-trust-chain` refresh. Several recurring reports are tied at
`2026-05-09`; `auth-abstraction-equivalence` was selected first from that
day's recurring invariant run list.

## Template Improvement Readout

The audit definition was improved before writing this report:

- Replaced stale `canic-dsl-macros` references with current
  `crates/canic-macros` paths.
- Replaced generic whole-crate scans with targeted auth-abstraction fan-in
  scans.
- Added mandatory `scripts/ci/run-auth-trust-chain-guards.sh` evidence.
- Added explicit checks for public material-only verifier exposure, passive auth
  DTOs, caller-lane separation, default guard identity source, delegated
  endpoint guard ordering, and role-attestation refresh narrowing.
- Updated the focused test bundle to the current macro/core tests.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Auth abstractions identified | PASS | Current abstractions are `#[canic_query]`, `#[canic_update]`, `requires(auth::authenticated(...))`, access-expression helpers, and delegated-session identity resolution. |
| Macro generated path preserves identity lanes | PASS | `crates/canic-macros/src/endpoint/expand.rs` tests confirm generated `AccessContext` uses `transport_caller`, `authenticated_subject`, and `identity_source` from `resolve_authenticated_identity(...)`. |
| Default app guard stays raw caller | PASS | `access_stage_default_guard_marks_identity_source_raw_caller` passed; non-auth default guards do not resolve delegated sessions. |
| Authenticated endpoint shape is compile-time guarded | PASS | `crates/canic-macros/src/endpoint/validate.rs` authenticated tests passed, requiring first argument type `DelegatedToken`. |
| Authenticated predicate routes to canonical verifier | PASS | `AuthenticatedEvaluator` calls `access::auth::delegated_token_verified(...)`; `access::auth` tests passed for subject binding, required scope, replay, stateless query handling, and guard ordering. |
| Public material-only verifier remains blocked | PASS | Guard script passed, and the direct `rg` scan found no public `AuthApi::verify_token` / public `verify_token_material` exposure. |
| Delegated-session convenience path keeps endpoint auth semantics | PASS | `access::auth` delegated-session resolution tests passed; convenience resolution remains separate from endpoint token verification. |
| Transport-caller predicates remain separate | PASS | `caller_predicates_use_transport_caller_not_authenticated_subject` passed. |
| Token material verifier still rejects trust failures | PASS | `verify_delegated_token` test filter passed root signature, shard signature, cert-hash drift, noncanonical vector, audience, scope, and expiry cases. |

## Equivalence Matrix

| Scenario | Expected Behavior | Current Evidence | Result |
| --- | --- | --- | --- |
| Valid credential | Generated/helper path accepts after canonical verifier succeeds | `verify_delegated_token_accepts_self_validating_token_without_proof_lookup` passed | PASS |
| Invalid signature | Generated/helper path rejects through canonical verifier failure | `verify_delegated_token_rejects_root_signature_failure` and shard-signature failure tests passed | PASS |
| Mismatched subject/caller | Token rejects after material verification | `subject_binding_rejects_mismatched_subject_and_caller` passed | PASS |
| Expired credential | Token rejects at verifier boundary | `verify_delegated_token_rejects_expired_token_at_boundary` passed | PASS |
| Missing required scope | Endpoint scope rejects through shared path | `required_scope_rejects_when_scope_missing` passed | PASS |
| Update replay | Update token consumption rejects active replay | `update_token_consume_rejects_active_replay` passed | PASS |
| Query replay | Query verification remains stateless | `query_token_consume_is_stateless` passed | PASS |
| Delegated session | Session subject is a convenience lane, not a raw-caller replacement | delegated-session resolution tests and caller-lane predicate test passed | PASS |

## Code Path Walkthrough

1. `#[canic_update]` / `#[canic_query]` enters
   `crates/canic-macros/src/lib.rs` and calls endpoint expansion.
2. `crates/canic-macros/src/endpoint/validate.rs` rejects authenticated
   endpoints whose first argument is not `DelegatedToken`.
3. `crates/canic-macros/src/endpoint/expand.rs` resolves authenticated identity
   only for explicit access expressions and builds `AccessContext` with
   separate transport and authenticated-subject lanes.
4. `eval_access(...)` dispatches `BuiltinPredicate::Authenticated` to
   `AuthenticatedEvaluator`.
5. `AuthenticatedEvaluator` calls
   `access::auth::delegated_token_verified(...)`.
6. `access/auth/token.rs` decodes token arg zero, calls
   `AuthOps::verify_token(...)`, then enforces subject binding, required scope,
   and update-token single-use consumption.

No generated or helper-specific bypass was found.

## Comparison to Previous Relevant Run

- Stable: generated authenticated endpoints still converge through
  `access::expr` and `access::auth`.
- Improved: the auth trust-chain CI guard now mechanically blocks public
  material-only verifier helpers, auth DTO behavior, guard-order drift, and
  broad role-attestation refresh behavior.
- Improved: `access/auth/token.rs` now has an explicit ordering test for
  verify -> subject binding -> scope -> update-token consumption.
- Stable: delegated sessions remain identity-resolution convenience and do not
  replace raw transport caller semantics for caller/topology predicates.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-macros/src/endpoint/expand.rs` | `access_stage`, `build_access_plan` | Generates endpoint access wrappers and chooses identity-lane setup | High |
| `crates/canic-macros/src/endpoint/validate.rs` | `validate_authenticated_args` | Compile-time guard that keeps authenticated endpoint signature shape aligned with ingress token decoding | Medium |
| `crates/canic-core/src/access/expr/mod.rs` | `AccessContext`, `eval_access` | Shared dispatch surface for generated and handwritten access expressions | High |
| `crates/canic-core/src/access/expr/evaluators.rs` | `AuthenticatedEvaluator` | Boundary between access evaluation and canonical auth verification | High |
| `crates/canic-core/src/access/auth/token.rs` | `delegated_token_verified`, `verify_token` | Canonical endpoint verifier ordering | High |
| `crates/canic-core/src/access/auth/identity.rs` | `resolve_authenticated_identity_at` | Delegated-session convenience lane | Medium |
| `crates/canic-core/src/api/auth/session/mod.rs` | `set_delegated_session_subject` | Session bootstrap uses token-material verification before storing convenience state | Medium |

## Hub Module Pressure

| Module | Fan-In Evidence | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/access/expr/*` | 5 direct files mention access expression/evaluator symbols | 2 | 1 | 6 |
| `crates/canic-core/src/access/auth/*` | 7 direct files mention access auth, identity resolution, or delegated verifier symbols | 4 | 2 | 6 |
| `crates/canic-core/src/dto/auth.rs` token shapes | `DelegationProof` appears in 11 files; token claims/verifier symbols appear in 9 files | 5 | 2 | 5 |
| `crates/canic-macros/src/endpoint/expand.rs` | Single macro endpoint-auth wrapper source | 2 | 1 | 5 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| macro/auth drift | `crates/canic-macros/src/endpoint/expand.rs` | Generated context setup and guard behavior live in one codegen module | Medium |
| subject-lane confusion | `crates/canic-core/src/access/expr/mod.rs` | `AccessContext` intentionally carries raw caller and authenticated subject | Medium |
| verifier ordering drift | `crates/canic-core/src/access/auth/token.rs` | Security behavior depends on verify -> bind -> scope -> consume ordering | Medium |
| DTO spread | `crates/canic-core/src/dto/auth.rs` | Auth DTOs are referenced across API, access, ops, tests, and support crates | Low |

## Dependency Fan-In Pressure

### Module Fan-In

| Module / Symbol Group | Direct Files | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `access::expr` / `eval_access` / `AccessExpr` / `BuiltinPredicate` | 5 | `access`, `macros` | Medium |
| `access::auth` / `delegated_token_verified` / identity resolution | 7 | `access`, `api`, `core`, `macros` | Medium |

### Struct / DTO Fan-In

| Struct / Symbol Group | Defined In | Direct Files | Risk |
| --- | --- | ---: | --- |
| `DelegationProof` | `crates/canic-core/src/dto/auth.rs` | 11 | Medium |
| `DelegatedTokenClaims` / `VerifiedDelegatedToken` / `VerifyDelegatedToken*` | `crates/canic-core/src/dto/auth.rs`, `ops/auth` | 9 | Medium |

## Risk Score

Risk Score: **3 / 10**

Score contributions:

- `+1` generated endpoint auth remains concentrated in one macro expansion
  module.
- `+1` `AccessContext` deliberately carries two caller identities.
- `+1` delegated-token endpoint behavior depends on ordering across access,
  ops, and DTO surfaces.
- `-1` effective reduction from the new CI guard and focused ordering/passive
  DTO tests; floor remains `3` because the hotspot concentration is expected
  and security-sensitive.

Verdict: **Invariant holds with low residual coupling risk.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `bash scripts/ci/run-auth-trust-chain-guards.sh` | PASS | Public partial verifier, DTO behavior, endpoint ordering, and refresh-scope guards passed. |
| `cargo test -p canic-macros authenticated -- --nocapture` | PASS | 8 parse/validate authenticated predicate tests passed. |
| `cargo test -p canic-macros access_stage_ -- --nocapture` | PASS | 2 generated access-context tests passed. |
| `cargo test -p canic-core --lib access::auth -- --nocapture` | PASS | 18 auth access tests passed, including scope, subject, replay, query, guard order, and session resolution. |
| `cargo test -p canic-core --lib verify_delegated_token -- --nocapture` | PASS | 10 delegated-token verifier tests passed. |
| `cargo test -p canic-core --lib caller_predicates_use_transport_caller_not_authenticated_subject -- --nocapture` | PASS | Caller-lane separation test passed. |
| `rg -l 'access::expr\|eval_access\|AccessExpr\|AccessPredicate\|BuiltinPredicate' crates canisters fleets -g '*.rs'` | PASS | 5 direct files. |
| `rg -l 'access::auth\|delegated_token_verified\|resolve_authenticated_identity\|AuthenticatedIdentitySource\|ResolvedAuthenticatedIdentity' crates canisters fleets -g '*.rs'` | PASS | 7 direct files. |
| `rg -l 'DelegationProof' crates canisters fleets -g '*.rs'` | PASS | 11 direct files. |
| `rg -l 'DelegatedTokenClaims\|VerifiedDelegatedToken\|VerifyDelegatedToken' crates canisters fleets -g '*.rs'` | PASS | 9 direct files. |
| `rg -n 'pub(\([^)]*\))?\s+(async\s+)?fn\s+verify_token\b\|pub(\([^)]*\))?\s+(async\s+)?fn\s+verify_token_material\b\|AuthApi::verify_token\b' crates/canic-core/src/api/auth crates/canic/src -g '*.rs'` | PASS | No public material-only verifier matches. |

## Follow-up Actions

No remediation required.

Watchpoints only:

1. Keep macro-generated authenticated endpoints aligned with
   `AccessContext`, `AuthenticatedEvaluator`, and `access/auth/token.rs`.
2. Keep `AuthApi::verify_token_material(...)` private unless a future public
   helper performs full endpoint subject binding and update replay consumption.
3. Keep `scripts/ci/run-auth-trust-chain-guards.sh` in the fast test lane.
