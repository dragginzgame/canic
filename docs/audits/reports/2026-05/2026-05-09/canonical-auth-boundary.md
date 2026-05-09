# Canonical Auth Boundary Invariant Audit - 2026-05-09

## Report Preamble

- Definition path: `docs/audits/recurring/invariants/canonical-auth-boundary.md`
- Scope: generated endpoint auth wrappers, authenticated access-expression evaluation, delegated-session identity resolution, canonical delegated-token verification, public auth helper surfaces, and root/internal auth-adjacent endpoints
- Compared baseline report path: `docs/audits/reports/2026-04/2026-04-05/canonical-auth-boundary.md`
- Code snapshot identifier: `518f57dd`
- Method tag/version: `Method V4.2`
- Comparability status: `partially comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-09T12:47:12Z`
- Branch: `main`
- Worktree: `dirty`

## Audit Selection

This was selected as the next oldest recurring audit after
`auth-abstraction-equivalence`. The latest previous report was
`docs/audits/reports/2026-04/2026-04-05/canonical-auth-boundary.md`, tied with
several other recurring audits from `2026-04-05`. It is first in the remaining
tied invariant set.

The run is partially comparable with the April baseline because the macro
implementation path moved from `canic-dsl-macros` to `crates/canic-macros`,
and delegated-token verification is now split more explicitly across
`access/auth/token.rs` and `ops/auth/delegated/verify.rs`. The canonical
boundary invariant remains comparable.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Authenticated endpoint entrypoints mapped | PASS | Generated endpoint access enters through `#[canic_query]` / `#[canic_update]` in `crates/canic-macros/src/lib.rs` and endpoint emitters in `crates/canic/src/macros/endpoints.rs`. Authenticated predicates are parsed only as `requires(auth::authenticated(...))`. |
| Generated authenticated path converges on canonical boundary | PASS | `crates/canic-macros/src/endpoint/expand.rs:268-284` resolves authenticated identity, builds `AccessContext`, and calls `access::expr::eval_access(...)`; `crates/canic-core/src/access/expr/evaluators.rs:384-399` routes authenticated predicates to `access::auth::delegated_token_verified(...)`. |
| Canonical endpoint ordering preserved | PASS | Endpoint path is `AuthOps::verify_token(...)` in `access/auth/token.rs:52-59`, then subject binding at `access/auth/token.rs:61`, required scope at `access/auth/token.rs:62`, update-token consumption at `access/auth/token.rs:63`, then handler execution after access evaluation returns. |
| Trust-chain and freshness verification remain centralized | PASS | `crates/canic-core/src/ops/auth/token.rs:73-135` performs config/key/root-trust setup before calling `verify_delegated_token(...)`; `crates/canic-core/src/ops/auth/delegated/verify.rs:84-145` validates cert policy/time, hashes, root signature, claims, audience, scopes, and shard signature. |
| Delegated-session ingress verifies before storing identity | PASS | `crates/canic-core/src/api/auth/session/mod.rs:29-123` validates wallet/subject principals, calls private `verify_token_material(...)`, checks verified subject equality, clamps TTL, fingerprints the bootstrap token, enforces replay policy, and only then upserts session state. |
| Raw-caller predicates are not replaced by delegated subject | PASS | `AccessContext` carries `caller` and `authenticated_caller` separately in `crates/canic-core/src/access/expr/mod.rs:22-31`; caller predicates read `ctx.caller` in `crates/canic-core/src/access/expr/evaluators.rs:258-373`. |
| Root/internal auth-adjacent endpoints have explicit caller gates | PASS | Root delegation and attestation endpoints are emitted with `internal, requires(caller::is_registered_to_subnet())` in `crates/canic/src/macros/endpoints.rs:360-379`; root child/wasm-store surfaces are gated by controller, parent, root, or registered-subnet predicates in the same emitter file. |
| Public helper surfaces checked | REMEDIATED | Initial scan found public `canic::api::auth::AuthApi::verify_token(...)`, which verified token material and required scopes but could not bind subject or consume update-token replay state. Follow-up removed the public helper and replaced delegated-session bootstrap usage with private `verify_token_material(...)`. |

## Finding Details

### Low / Medium - public `AuthApi::verify_token` was a partial verifier surface

The canonical endpoint boundary was clean, but the public runtime API exposed:

- `crates/canic/src/api/mod.rs:9-11`: re-exports `AuthApi`
- `crates/canic-core/src/api/auth/mod.rs:104-121`: former
  `AuthApi::verify_token(...)`

That helper called `AuthOps::verify_token(...)`, which covers delegated-token
config, trust-chain, cert/token freshness, audience, signature, and required
scope checks. It did not receive a caller or endpoint call kind, so it could not
perform the endpoint-only stages:

- subject-caller binding
- update-call single-use consumption

Generated endpoints did not use this helper, and the generated
authenticated path still performs the full endpoint boundary through
`access/auth/token.rs`. This was therefore not a current bypass in Canic's
generated endpoint flow. The risk was that downstream/manual handler code could
mistake `AuthApi::verify_token(...)` for the canonical endpoint-auth boundary.

Follow-up remediation:

1. Removed public `AuthApi::verify_token(...)`.
2. Added private `AuthApi::verify_token_material(...)` for delegated-session
   bootstrap, returning only the verified token subject.
3. Kept endpoint authorization on the generated access path that also binds
   subject/caller and consumes update-token replay state.

## Entrypoint Map

| Entrypoint Class | Current Path | Boundary Status |
| --- | --- | --- |
| Macro-generated authenticated handlers | `canic-macros/src/endpoint/expand.rs` -> `access::expr::eval_access` -> `access::auth::delegated_token_verified` | Full canonical endpoint boundary |
| Delegated-session bootstrap | `api/auth/session.rs` -> private `verify_token_material` -> subject equality -> replay fingerprint/session policy | Verifies before state change; not handler auth by itself |
| Root delegation proof endpoint | `canic_request_delegation` in `crates/canic/src/macros/endpoints.rs:360-365` | Registered-subnet internal gate plus root-only API requirement |
| Root role-attestation endpoint | `canic_request_role_attestation` in `crates/canic/src/macros/endpoints.rs:367-373` | Registered-subnet internal gate plus root flow validation |
| Attestation key-set endpoint | `canic_attestation_key_set` in `crates/canic/src/macros/endpoints.rs:375-379` | Registered-subnet internal gate |
| Private token-material helper | `AuthApi::verify_token_material` | Private delegated-session bootstrap helper; not exposed as endpoint authorization |

## Canonical Path Walkthrough

The generated authenticated endpoint path is:

1. Endpoint macro expansion validates access syntax and fallible return shape.
2. The wrapper resolves `msg_caller()` and delegated-session identity.
3. The wrapper builds `AccessContext` with both raw transport caller and
   authenticated subject.
4. `eval_access(...)` dispatches authenticated predicates to
   `AuthenticatedEvaluator`.
5. `AuthenticatedEvaluator` calls `access::auth::delegated_token_verified(...)`.
6. `access/auth/token.rs` decodes the delegated token from ingress arg zero.
7. `AuthOps::verify_token(...)` verifies delegated token trust chain, freshness,
   audience, signatures, and required scopes.
8. `access/auth/token.rs` enforces subject binding against the resolved subject.
9. `access/auth/token.rs` consumes update tokens once.
10. Handler logic runs only after access evaluation succeeds.

The audit found no generated endpoint path that performs authorization or
handler execution before this boundary completes.

## Comparison to Previous Relevant Run

- Stable: generated authenticated endpoint flows still converge on the same
  canonical verifier stack.
- Changed: macro path is now `crates/canic-macros/src/endpoint/expand.rs`
  instead of `crates/canic-dsl-macros/src/endpoint/expand.rs`.
- Improved: update-token single-use consumption is now explicit in the endpoint
  boundary after subject binding and scope checks.
- Stable: delegated-session bootstrap still verifies token material and subject
  before storing a local session mapping.
- New warning, remediated: this run explicitly included public runtime helper
  surfaces and found that `AuthApi::verify_token(...)` was public but partial
  relative to the endpoint boundary; follow-up removed that public helper.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-macros/src/endpoint/expand.rs` | `access_stage`, `build_access_plan` | Generates the endpoint access wrapper and decides whether handler execution is reachable | High |
| `crates/canic-core/src/access/expr/mod.rs` | `AccessContext`, `eval_access` | Central dispatch surface before endpoint handler execution | High |
| `crates/canic-core/src/access/expr/evaluators.rs` | `AuthenticatedEvaluator` | Boundary between access expression predicates and delegated-token verification | High |
| `crates/canic-core/src/access/auth/token.rs` | `delegated_token_verified`, `verify_token` | Endpoint-auth ordering owner for token verification, subject binding, scope, and replay consumption | High |
| `crates/canic-core/src/ops/auth/token.rs` | `AuthOps::verify_token` | Canonical token-material verification stage | High |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | `verify_delegated_token` | Trust-chain/freshness/audience/signature/scope verifier | High |
| `crates/canic-core/src/api/auth/mod.rs` | `AuthApi::verify_token_material` | Private delegated-session bootstrap helper that intentionally verifies token material only | Low |
| `crates/canic-core/src/api/auth/session/mod.rs` | `set_delegated_session_subject` | Alternate auth-adjacent ingress that persists identity convenience state | Medium |

## Hub Module Pressure

| Module | Fan-In Evidence | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/access/auth/*` | 7 direct files mention `access::auth`, identity resolution, or delegated verifier symbols | 4 | 2 | 6 |
| `crates/canic-core/src/access/expr/*` | 5 direct files mention access expression/evaluator symbols | 2 | 1 | 6 |
| `crates/canic-core/src/ops/auth/*` | Auth ops exports public API, access boundary, delegated verifier, attestation, storage, config, and IC crypto interactions | 6 | 3 | 7 |
| `crates/canic-macros/src/endpoint/expand.rs` | Recent edit frequency and codegen ownership make it the only generated endpoint-auth wrapper path | 2 | 1 | 5 |
| `crates/canic-core/src/api/auth/mod.rs` | Public runtime API, root RPC helpers, verifier refresh, delegated sessions, and token helpers meet here | 5 | 3 | 6 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| partial boundary cleanup | `AuthApi::verify_token_material` | Partial token-material verifier is now private and used only by delegated-session bootstrap | Low |
| ordering-sensitive endpoint boundary | `crates/canic-core/src/access/auth/token.rs` | Endpoint safety depends on preserving verify -> bind subject -> scope -> update consumption order | Medium |
| generated wrapper gravity | `crates/canic-macros/src/endpoint/expand.rs` | Wrapper code owns call context, access metrics, default guards, and handler reachability | Medium |
| caller-lane confusion | `crates/canic-core/src/access/expr/mod.rs` | `AccessContext` intentionally carries raw caller and authenticated subject separately | Medium |
| auth ops hub | `crates/canic-core/src/ops/auth` | Recent git scan shows repeated auth edits across token, boundary, delegated verifier, and API modules | Medium |

## Dependency Fan-In Pressure

### Module Fan-In

| Module / Symbol Group | Direct Files | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `access::auth` / `delegated_token_verified` / identity resolution | 7 | `access`, `api`, `core`, `macros` | Hub forming |
| `access::expr` / `eval_access` / `AccessExpr` / `BuiltinPredicate` | 5 | `access`, `macros` | Rising pressure |
| `DelegatedTokenClaims` / `VerifiedDelegatedToken` / `VerifyDelegatedToken*` | 10 | `access`, `api`, `dto`, `ops` | Hub abstraction |

### Struct / DTO Fan-In

| Struct / Symbol Group | Defined In | Direct Files | Risk |
| --- | --- | ---: | --- |
| `VerifiedDelegatedToken` / `VerifyDelegatedToken*` | `crates/canic-core/src/ops/auth/delegated/verify.rs`, `ops/auth/types.rs` | 10 | Medium |
| `DelegatedTokenClaims` | `crates/canic-core/src/dto/auth.rs` | 10-symbol group scan | Medium |
| `AccessContext` | `crates/canic-core/src/access/expr/mod.rs` | Endpoint macro and access evaluator tests directly depend on two-lane caller semantics | Medium |

## Risk Score

Risk Score: **3 / 10**

Score contributions:

- `+1` endpoint auth remains concentrated in macro expansion and
  `access::expr`.
- `+1` canonical endpoint safety depends on verifier ordering across
  `access/auth/token.rs` and `ops/auth/token.rs`.
- `+1` `AccessContext` carries separate raw caller and authenticated subject
  lanes.

Verdict: **Canonical endpoint auth boundary holds; the public partial verifier
surface found during the audit was removed.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-macros access_stage_expr_builds_context_from_resolved_identity -- --nocapture` | PASS | Macro-generated expression access stage resolves authenticated identity before evaluating access. |
| `cargo test -p canic-macros authenticated -- --nocapture` | PASS | 8 authenticated parser/validator tests passed. |
| `cargo test -p canic-core --lib verify_delegated_token -- --nocapture` | PASS | 10 delegated-token verifier tests passed for trust, freshness, audience, signature, and scope rejection paths. |
| `cargo test -p canic-core --lib resolve_authenticated_identity -- --nocapture` | PASS | 6 delegated-session identity resolution tests passed. |
| `cargo test -p canic-core --lib caller_predicates_use_transport_caller_not_authenticated_subject -- --nocapture` | PASS | Raw-caller predicates remain transport-caller based. |
| `cargo test -p canic-core --lib required_scope_rejects_when_scope_missing -- --nocapture` | PASS | Required scope rejection remains canonical. |
| `cargo test -p canic-core --lib update_token_consume_rejects_active_replay -- --nocapture` | PASS | Update-token replay consumption rejects reuse. |
| `cargo test -p canic-core --lib subject_binding_rejects_mismatched_subject_and_caller -- --nocapture` | PASS | Subject binding rejects mismatched token subject/caller. |
| `rg -n "authenticated\\(|delegated_token_verified|resolve_authenticated_identity|verify_token|require_auth|admin_|internal_|system_" crates/canic-core/src crates/canic-macros/src crates/canic/src -g '*.rs'` | PASS | Entrypoint and auth-boundary search recorded. |
| `rg -n "canic_update|canic_query|requires\\(" crates/canic/src/macros/endpoints.rs crates/canic-macros/src crates/canic-core/src -g '*.rs'` | PASS | Endpoint gate map recorded. |
| `rg -l 'access::auth\|delegated_token_verified\|resolve_authenticated_identity\|AuthenticatedIdentitySource\|ResolvedAuthenticatedIdentity' crates canisters fleets -g '*.rs'` | PASS | Fan-in scan recorded 7 direct files. |
| `rg -l 'access::expr\|eval_access\|AccessExpr\|AccessPredicate\|BuiltinPredicate' crates canisters fleets -g '*.rs'` | PASS | Fan-in scan recorded 5 direct files. |
| `rg -l 'DelegatedTokenClaims\|VerifiedDelegatedToken\|VerifyDelegatedToken' crates canisters fleets -g '*.rs'` | PASS | Token DTO/verifier spread scan recorded 10 direct files. |
| `git log --name-only -n 20 -- crates/canic-core/src/access crates/canic-core/src/api/auth crates/canic-core/src/ops/auth crates/canic-macros/src/endpoint` | PASS | Recent edit-pressure scan recorded auth/macro hotspots. |
| `rg -n "AuthApi::verify_token\|pub fn verify_token\\(\|verify_token_material" crates/canic-core/src crates/canic/src -g '*.rs'` | PASS | Confirmed the public `AuthApi::verify_token` symbol is gone and only private `verify_token_material` plus `AuthOps::verify_token` remain. |
| `cargo check -p canic-core` | PASS | Post-remediation compile check passed. |
| `cargo clippy -p canic-core --lib -- -D warnings` | PASS | Post-remediation clippy gate passed. |

## Follow-up Actions

1. Keep `AuthApi::verify_token_material(...)` private unless a future public
   helper performs the full endpoint boundary, including subject binding and
   update replay.
2. Re-run this audit after changes to macro auth wiring, `AccessContext`,
   delegated-session bootstrap, or delegated-token verifier ordering.
