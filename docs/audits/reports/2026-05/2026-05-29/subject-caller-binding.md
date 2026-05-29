# Subject-Caller Binding Invariant Audit - 2026-05-29

## Report Preamble

- Definition path: `docs/audits/recurring/invariants/subject-caller-binding.md`
- Scope: delegated-token subject binding, generated endpoint auth expansion,
  access-expression caller lanes, delegated-session subject resolution, and
  private/public token-verifier helper boundaries
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-09/subject-caller-binding.md`
- Code snapshot identifier: `9b435cac`
- Method tag/version: `Method V4.3`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp: `2026-05-29`
- Branch: `main`
- Worktree: `dirty`

## Audit Selection

This audit was selected for the next 0.48 cleanup patch because it was tied
for the oldest recurring audit still in the active recurring set. The previous
dedicated `subject-caller-binding` report was from 2026-05-09.

The run is comparable with the 2026-05-09 baseline. The macro path,
access-context shape, delegated-session subject lane, and canonical
delegated-token verifier ordering are unchanged in the audited areas.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Canonical subject binding exists | PASS | `access/auth/token.rs` verifies token material, then calls `enforce_subject_binding(verified.subject, caller)` before required-scope enforcement and update-token consumption. |
| Mismatched token subject rejects | PASS | `cargo +1.96.0 test -p canic-core --lib subject_binding --locked -- --nocapture` passed the matching and mismatched subject-binding tests. |
| Authenticated predicate routes through canonical verifier | PASS | `access/expr/evaluators.rs` passes `ctx.authenticated_caller`, the required scope, and call kind into `access::auth::delegated_token_verified(...)`. |
| Generated endpoint access context preserves both caller lanes | PASS | `canic-macros/src/endpoint/expand.rs` still builds `AccessContext` from `transport_caller` and `authenticated_subject`; the focused macro test passed. |
| Caller/topology predicates use transport caller | PASS | `caller_predicates_use_transport_caller_not_authenticated_subject` passed. |
| Delegated sessions reject invalid subjects | PASS | `delegated_session_subject` focused tests passed for anonymous and management-canister subjects. |
| Partial token-material verifier remains private | PASS | `AuthApi::verify_token_material(...)` remains private and its comment still states endpoint auth must also bind subject to caller and consume update tokens once. |
| No bearer fallback path detected | PASS | Scans found capability/internal-call envelopes, but no production delegated-token path that accepts token proof without subject binding. |

## Binding Path Walkthrough

The authenticated endpoint path remains:

1. Generated endpoint wrapper reads the transport caller.
2. `resolve_authenticated_identity(...)` resolves either raw caller identity or
   a validated delegated-session subject.
3. The wrapper builds `AccessContext` with `caller` and
   `authenticated_caller`.
4. `eval_access(...)` dispatches `auth::authenticated(...)` to the
   authenticated predicate evaluator.
5. The evaluator calls `access::auth::delegated_token_verified(...)` with the
   authenticated subject.
6. `access/auth/token.rs` decodes the token, verifies self-contained token
   material, enforces `verified.subject == authenticated_subject`, enforces
   required scope, and consumes update tokens once.

No handler-local substitute for this binding path was found.

## Comparison To Previous Relevant Run

- Stable: canonical subject-caller binding remains in
  `crates/canic-core/src/access/auth/token.rs`.
- Stable: generated authenticated endpoint wrappers still construct both raw
  transport caller and authenticated subject lanes before access evaluation.
- Stable: delegated sessions intentionally affect only the authenticated
  subject lane; caller/topology predicates still use the transport caller.
- Stable: `AuthApi::verify_token_material(...)` remains private and is still
  documented as incomplete for endpoint authorization.
- No new blocker or remediation was found.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/access/auth/token.rs` | `verify_token`, `enforce_subject_binding` | Canonical delegated-token subject binding and verifier ordering | High |
| `crates/canic-core/src/access/expr/evaluators.rs` | authenticated predicate branch | Passes authenticated subject into the canonical verifier | High |
| `crates/canic-core/src/access/expr/mod.rs` | `AccessContext` | Stores both raw transport caller and authenticated subject | High |
| `crates/canic-macros/src/endpoint/expand.rs` | access stage expansion | Generates endpoint wrappers that construct `AccessContext` | High |
| `crates/canic-core/src/access/auth/identity.rs` | `resolve_authenticated_identity_at` | Delegated-session subject selection and fallback behavior | Medium |
| `crates/canic-core/src/api/auth/session/mod.rs` | `set_delegated_session_subject` | Session bootstrap binds requested subject to verified token subject | Medium |
| `crates/canic-core/src/api/auth/mod.rs` | `verify_token_material` | Private partial verifier that must not become an endpoint-auth substitute | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| access expression caller lanes | caller/auth subject lane scan found 8 direct files | 3 | 2 | 6 |
| access auth binding lane | access-auth binding scan found 7 direct files | 3 | 2 | 6 |
| delegated-token verifier shapes | verifier/claims scan found 13 direct files | 5 | 3 | 6 |
| explicit subject/caller checks | subject/caller mismatch scan found 21 direct files | 5 | 3 | 6 |

Pressure remains moderate and unchanged from the previous comparable run.

## Risk Score

Risk Score: **3 / 10**

Score contributions:

- `+1` generated endpoint access context is security-sensitive.
- `+1` `AccessContext` intentionally carries two identity lanes.
- `+1` private token-material verification remains useful but incomplete for
  endpoint authorization.

Verdict: **Invariant holds with low residual identity-lane coupling risk.**

## Amplification Drivers

- Authenticated endpoint macro changes can affect every endpoint using
  `auth::authenticated(...)`.
- Delegated-session changes can alter which subject is passed to the
  delegated-token verifier.
- Moving or exposing token-material verification could create a tempting
  partial-auth helper unless it also binds subject, checks scopes, and consumes
  update tokens.

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| caller-lane confusion | `AccessContext` and generated endpoint wrappers | `caller` and `authenticated_caller` intentionally differ during delegated sessions | Medium |
| verifier ordering drift | `access/auth/token.rs` | subject binding must stay after cryptographic verification and before scope/handler success | Medium |
| private helper pressure | `api/auth/mod.rs` | `verify_token_material(...)` is private and intentionally omits caller binding and update replay consumption | Low |
| macro generation pressure | `canic-macros/src/endpoint/expand.rs` | generated wrappers must continue constructing both caller lanes before `eval_access(...)` | Low |

### Enum Shock Radius

| Enum | Defined In | Reference Files | Risk |
| --- | --- | ---: | --- |
| `AuthenticatedIdentitySource` | `crates/canic-core/src/access/auth/mod.rs` | low local reference count in auth tests and identity resolution | Low |
| `BuiltinPredicate::Authenticated` | `crates/canic-core/src/access/expr/mod.rs` | referenced by access evaluator and endpoint macro parse/validate/expand code | Medium |

### Cross-Layer Struct Spread

| Struct | Defined In | Layers Referencing | Risk |
| --- | --- | --- | --- |
| `AccessContext` | `crates/canic-core/src/access/expr/mod.rs` | macro endpoint expansion, access evaluation, access tests | Medium |
| `VerifyDelegatedTokenRuntimeInput` | `crates/canic-core/src/ops/auth/types.rs` | api/auth, access/auth, ops/auth | Medium |

### Growing Hub Modules

| Module | Subsystems Imported | Recent Commits | Risk |
| --- | --- | ---: | --- |
| `crates/canic-macros/src/endpoint/expand.rs` | endpoint macro generation, access context, wrapper dispatch | 11 path hits in recent scoped history | Medium |
| `crates/canic-core/src/api/auth/mod.rs` | auth API, token material, sessions, delegated proof issuance | 6 path hits in recent scoped history | Medium |
| `crates/canic-core/src/access/auth/token.rs` | token decode, verification, subject binding, replay consumption | 3 path hits in recent scoped history | Medium |

### Capability Surface Growth

No subject-caller capability surface growth was detected in this run.

## Dependency Fan-In Pressure

| Module / Struct | Evidence | Risk |
| --- | --- | --- |
| `AccessContext` | Direct scan found uses in access expression code and endpoint macro expansion | Medium |
| `delegated_token_verified` | Direct scan found the canonical access-auth path and authenticated evaluator call site | Low |
| `VerifyDelegatedTokenRuntimeInput` | Direct scan found API/auth, access/auth, and ops/auth use | Medium |

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo +1.96.0 test -p canic-core --lib subject_binding --locked -- --nocapture` | PASS | 2 tests passed for matching and mismatched subject binding. |
| `cargo +1.96.0 test -p canic-core --lib caller_predicates_use_transport_caller_not_authenticated_subject --locked -- --nocapture` | PASS | Proves caller/topology predicates use raw transport caller semantics. |
| `cargo +1.96.0 test -p canic-core --lib resolve_authenticated_identity --locked -- --nocapture` | PASS | 6 delegated-session identity resolution tests passed. |
| `cargo +1.96.0 test -p canic-macros authenticated --locked -- --nocapture` | PASS | 9 authenticated parse/validate/expand tests passed. |
| `cargo +1.96.0 test -p canic-macros access_stage_expr_builds_context_from_resolved_identity --locked -- --nocapture` | PASS | Generated access stage preserves both caller lanes. |
| `cargo +1.96.0 test -p canic-core --lib delegated_session_subject --locked -- --nocapture` | PASS | 2 delegated-session subject validation tests passed. |
| `cargo +1.96.0 test -p canic-core --lib verify_delegated_token_accepts_self_validating_token_without_proof_lookup --locked -- --nocapture` | PASS | Self-contained delegated-token verification still succeeds. |
| `rg -l 'access::expr\|eval_access\|AccessContext\|BuiltinPredicate::Authenticated\|authenticated_caller\|transport_caller\|authenticated_subject' crates canisters fleets -g '*.rs' \| wc -l` | PASS | Caller/auth subject lane scan found 8 direct files. |
| `rg -l 'access::auth\|delegated_token_verified\|resolve_authenticated_identity\|enforce_subject_binding\|AuthenticatedIdentitySource\|ResolvedAuthenticatedIdentity' crates canisters fleets -g '*.rs' \| wc -l` | PASS | Access auth binding scan found 7 direct files. |
| `rg -l 'VerifyDelegatedTokenRuntimeInput\|VerifiedDelegatedToken\|verify_delegated_token\|DelegatedTokenClaims\|claims\.subject' crates canisters fleets -g '*.rs' \| wc -l` | PASS | Verifier/claims scan found 13 direct files. |
| `rg -l 'subject mismatch\|subject.*caller\|caller.*subject\|delegated token subject\|subject.*must match caller\|does not match caller' crates/canic-core/src crates/canic-tests/tests canisters/test fleets/test -g '*.rs' \| wc -l` | PASS | Explicit subject/caller mismatch scan found 21 direct files. |

## Follow-up Actions

1. Keep `crates/canic-core/src/access/auth/token.rs`,
   `crates/canic-core/src/access/expr/mod.rs`, and
   `crates/canic-macros/src/endpoint/expand.rs` aligned whenever
   authenticated endpoint syntax or delegated-session identity resolution
   changes.
2. Keep `AuthApi::verify_token_material(...)` private unless a future public
   helper performs the full endpoint boundary: token verification,
   subject-caller binding, scope enforcement, and update replay consumption.
3. Rerun this audit after delegated-session, authenticated endpoint macro, or
   token-verifier changes.
