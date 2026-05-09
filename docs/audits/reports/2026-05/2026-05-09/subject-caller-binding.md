# Subject-Caller Binding Invariant Audit - 2026-05-09

## Report Preamble

- Definition path: `docs/audits/recurring/invariants/subject-caller-binding.md`
- Scope: delegated-token subject binding, generated endpoint auth expansion,
  access-expression caller lanes, delegated-session subject resolution, role
  attestation caller checks, and public/private verifier helper surfaces
- Compared baseline report path: `docs/audits/reports/2026-04/2026-04-05/subject-caller-binding.md`
- Code snapshot identifier: `518f57dd`
- Method tag/version: `Method V4.2`
- Comparability status: `partially comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-09T13:43:27Z`
- Branch: `main`
- Worktree: `dirty`

## Audit Selection

This was selected as the next oldest recurring invariant audit after
`expiry-replay-single-use`. Its latest previous dedicated report was
`docs/audits/reports/2026-04/2026-04-05/subject-caller-binding.md`.

The run is partially comparable with the April baseline because the endpoint
macro path has moved from the old `canic-dsl-macros` layout to
`crates/canic-macros`, and the auth surface now includes explicit
delegated-session subject resolution plus update-token single-use consumption.
The core invariant remains directly comparable: a delegated token must not grant
authority unless the verified token subject is the authenticated caller subject
used by the endpoint.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Canonical subject binding exists | PASS | `crates/canic-core/src/access/auth/token.rs:52-63` verifies token material, then calls `enforce_subject_binding(verified.subject, caller)` before scope enforcement and update-token consumption. |
| Mismatched token subject rejects | PASS | `subject_binding_rejects_mismatched_subject_and_caller` passed and asserts the canonical mismatch error path. |
| Macro-generated authenticated endpoints preserve binding | PASS | `crates/canic-macros/src/endpoint/expand.rs:268-284` resolves identity, stores transport caller and authenticated subject separately, and evaluates the generated `AccessExpr`; `access_stage_expr_builds_context_from_resolved_identity` passed. |
| Authenticated predicate routes through canonical verifier | PASS | `crates/canic-core/src/access/expr/evaluators.rs:384-399` passes `ctx.authenticated_caller`, required scope, and call kind into `access::auth::delegated_token_verified(...)`. |
| Raw caller and authenticated subject remain separate | PASS | `crates/canic-core/src/access/expr/mod.rs:22-41` keeps both lanes in `AccessContext`; `caller_predicates_use_transport_caller_not_authenticated_subject` passed. |
| Delegated sessions do not accept invalid subjects | PASS | `crates/canic-core/src/access/auth/identity.rs:25-48` validates stored delegated-session subjects before using them, otherwise clears the session and falls back to raw caller. |
| Session bootstrap binds requested subject to token subject | PASS | `crates/canic-core/src/api/auth/session/mod.rs:60-76` verifies bootstrap token material and rejects when the verified subject differs from the requested delegated subject. |
| Partial token-material verifier is private | PASS | `crates/canic-core/src/api/auth/mod.rs:63-82` keeps `verify_token_material(...)` private and documents that endpoint authorization must still bind subject to caller and consume update tokens once. |
| Role-attestation caller binding remains independent | PASS | `crates/canic-core/src/workflow/rpc/request/handler/authorize.rs` still rejects role-attestation requests whose subject does not match the workflow caller; existing mismatch tests remain in `crates/canic-core` and PocketIC role-attestation suites. |

## Binding Path Walkthrough

The authenticated endpoint path currently runs:

1. The generated wrapper reads `msg_caller()` as the transport caller.
2. `resolve_authenticated_identity(...)` maps that transport caller to either
   the raw caller or a validated delegated-session subject.
3. The wrapper builds `AccessContext` with both `caller` and
   `authenticated_caller`.
4. `eval_access(...)` dispatches `auth::authenticated(...)` to
   `AuthenticatedEvaluator`.
5. `AuthenticatedEvaluator` calls `access::auth::delegated_token_verified(...)`
   with the authenticated subject.
6. `access/auth/token.rs` decodes the token, verifies self-contained token
   material, enforces `verified.subject == authenticated_subject`, enforces the
   required scope, and consumes update tokens once.

This means delegated sessions can intentionally choose the subject lane used by
authenticated endpoint tokens, while caller/topology predicates continue to use
the raw transport caller lane.

## Comparison to Previous Relevant Run

- Stable: the canonical subject-caller check still lives below macro/helper
  abstractions in the access auth layer.
- Changed: the macro implementation path is now `crates/canic-macros` instead
  of the April report's `crates/canic-dsl-macros` path.
- Improved: the public partial token verifier identified during the
  `canonical-auth-boundary` audit has been reduced to a private
  `verify_token_material(...)` helper.
- Stable: caller/topology predicates continue to use transport caller semantics
  even when a delegated session resolves a different authenticated subject.
- Stable: role-attestation and capability paths keep their own explicit
  subject/caller checks and do not substitute delegated-token subject binding
  for caller authorization.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/access/auth/token.rs` | `verify_token`, `enforce_subject_binding` | Canonical delegated-token subject binding and verifier ordering | High |
| `crates/canic-core/src/access/expr/evaluators.rs` | `AuthenticatedEvaluator` | Passes authenticated subject into the canonical verifier | High |
| `crates/canic-core/src/access/expr/mod.rs` | `AccessContext` | Stores both raw transport caller and authenticated subject | High |
| `crates/canic-macros/src/endpoint/expand.rs` | `access_stage` | Generates endpoint wrappers that construct `AccessContext` | High |
| `crates/canic-core/src/access/auth/identity.rs` | `resolve_authenticated_identity_at` | Delegated-session subject selection and fallback behavior | Medium |
| `crates/canic-core/src/api/auth/session/mod.rs` | `set_delegated_session_subject` | Session bootstrap binds requested subject to verified token subject | Medium |
| `crates/canic-core/src/api/auth/mod.rs` | `verify_token_material` | Private partial verifier that must not become an endpoint-auth substitute | Medium |

## Hub Module Pressure

| Module | Fan-In Evidence | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| access expression caller lanes | caller/auth subject scan found 8 direct files | 3 | 2 | 6 |
| access auth binding lane | access-auth scan found 7 direct files | 3 | 2 | 6 |
| delegated-token verifier shapes | verifier/claims scan found 13 direct files | 5 | 3 | 6 |
| explicit subject/caller checks | subject/caller mismatch scan found 21 direct files | 5 | 3 | 6 |

The fan-in is manageable, but the invariant depends on developers preserving the
meaning of two similarly named identity lanes: transport caller for topology
authorization, authenticated subject for delegated-token endpoint auth.

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| caller-lane confusion | `AccessContext` and evaluator code | `caller` and `authenticated_caller` intentionally differ during delegated sessions | Medium |
| verifier ordering drift | `access/auth/token.rs` | Binding must stay after cryptographic verification and before scope/handler success | Medium |
| public helper pressure | `api/auth/mod.rs` | `verify_token_material(...)` is useful internally but intentionally omits caller binding and update replay consumption | Low |
| macro generation pressure | `canic-macros/src/endpoint/expand.rs` | Generated code must keep constructing both caller lanes before `eval_access(...)` | Low |

## Risk Score

Risk Score: **3 / 10**

Score contributions:

- `+1` generated endpoint access context is security-sensitive.
- `+1` `AccessContext` carries two identity lanes with different authorization
  meanings.
- `+1` token-material verification has a private helper that must not become a
  public endpoint-auth substitute.

Verdict: **Invariant holds with low residual identity-lane coupling risk.**

No remediation is required from this dedicated run. The useful follow-up is to
keep the caller-lane distinction explicit whenever changing delegated sessions,
authenticated endpoint macros, or the delegated-token verifier.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-core --lib subject_binding -- --nocapture` | PASS | 2 tests passed for matching and mismatched subject binding. |
| `cargo test -p canic-core --lib caller_predicates_use_transport_caller_not_authenticated_subject -- --nocapture` | PASS | Proves topology caller predicates use raw transport caller semantics. |
| `cargo test -p canic-core --lib resolve_authenticated_identity -- --nocapture` | PASS | 6 tests passed for raw fallback, active delegated session, expiry boundary, clear behavior, and invalid-subject fallback. |
| `cargo test -p canic-macros authenticated -- --nocapture` | PASS | 8 authenticated parse/validate tests passed. |
| `cargo test -p canic-macros access_stage_expr_builds_context_from_resolved_identity -- --nocapture` | PASS | Generated access stage preserves both transport caller and authenticated subject fields. |
| `cargo test -p canic-core --lib delegated_session_subject -- --nocapture` | PASS | 2 delegated-session subject validation tests passed. |
| `cargo test -p canic-core --lib verify_delegated_token_accepts_self_validating_token_without_proof_lookup -- --nocapture` | PASS | Valid token verification still succeeds through the self-contained verifier. |
| `rg -l 'access::expr\|eval_access\|AccessContext\|BuiltinPredicate::Authenticated\|authenticated_caller\|transport_caller\|authenticated_subject' crates canisters fleets -g '*.rs'` | PASS | Caller/auth subject lane scan recorded 8 direct files. |
| `rg -l 'access::auth\|delegated_token_verified\|resolve_authenticated_identity\|enforce_subject_binding\|AuthenticatedIdentitySource\|ResolvedAuthenticatedIdentity' crates canisters fleets -g '*.rs'` | PASS | Access auth binding scan recorded 7 direct files. |
| `rg -l 'VerifyDelegatedTokenRuntimeInput\|VerifiedDelegatedToken\|verify_delegated_token\|DelegatedTokenClaims\|claims\.subject' crates canisters fleets -g '*.rs'` | PASS | Verifier/claims scan recorded 13 direct files. |
| `rg -l 'subject mismatch\|subject.*caller\|caller.*subject\|delegated token subject\|subject.*must match caller' crates/canic-core/src crates/canic-tests/tests canisters/test fleets/test -g '*.rs'` | PASS | Explicit subject/caller mismatch scan recorded 21 direct files. |

## Follow-up Actions

1. Keep `crates/canic-core/src/access/auth/token.rs`,
   `crates/canic-core/src/access/expr/mod.rs`, and
   `crates/canic-macros/src/endpoint/expand.rs` aligned whenever
   authenticated endpoint syntax or delegated-session identity resolution
   changes.
2. Keep `AuthApi::verify_token_material(...)` private unless a future public
   helper performs the full endpoint boundary: token verification,
   subject-caller binding, scope enforcement, and update replay consumption.
