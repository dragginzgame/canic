# Canonical Auth Boundary Invariant Audit - 2026-06-01

## Report Preamble

- Definition path:
  `docs/audits/recurring/invariants/canonical-auth-boundary.md`
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-16/canonical-auth-boundary.md`
- Code snapshot identifier: `5487d6ff`
- Method tag/version: `canonical-auth-boundary/current`
- Comparability status: `partially comparable`
- Auditor: `codex`
- Run timestamp: `2026-06-01`
- Branch: `main`
- Worktree: dirty during audit; active `.57.9` audit-definition/report
  changes were present
- Scope:
  - generated endpoint wrappers in `crates/canic-macros/src/endpoint/**`
  - access-expression evaluation in `crates/canic-core/src/access/**`
  - delegated-token verification in `crates/canic-core/src/ops/auth/**`
  - internal invocation proof APIs in `crates/canic-core/src/api/auth/**`
  - generated endpoint bundles in `crates/canic/src/macros/endpoints/**`

## Executive Summary

Initial risk: **3 / 10**.

Post-audit risk: **3 / 10**.

The canonical auth boundary holds. Public delegated-token endpoint auth still
enters through `auth::authenticated(...)`, requires the first handler argument
to be `DelegatedToken`, resolves the transport caller and authenticated subject
before access evaluation, and reaches the endpoint handler only after
`eval_access(...)` succeeds.

Protected internal caller-role predicates are separate from delegated-token
endpoint auth. They lower through the root-signed internal invocation proof
envelope path and call `AuthApi::verify_internal_invocation_proof(...)` before
handler execution.

No remediation was needed. The recurring audit definition was refreshed to
explicitly track the hard-cut singular delegated-token audience and the
protected internal caller-role proof boundary.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Authenticated endpoint entrypoints mapped | PASS | `crates/canic-macros/src/endpoint/validate.rs:58-60` detects authenticated predicates; `validate.rs:183-191` requires first argument type `DelegatedToken`. |
| Generated authenticated path converges on canonical boundary | PASS | `crates/canic-macros/src/endpoint/expand.rs:280-323` resolves authenticated identity, builds `AccessContext`, and calls `eval_access(...)` before handler execution. |
| Authenticated predicate reaches endpoint boundary | PASS | `crates/canic-core/src/access/expr/evaluators.rs:88` dispatches authenticated predicates to `access::auth::delegated_token_verified(...)`. |
| Endpoint ordering preserved | PASS | `crates/canic-core/src/access/auth/token.rs:52-63` performs `AuthOps::verify_token(...)`, subject binding, required-scope validation, and update-token consumption before returning success. |
| Singular audience model is active | PASS | `crates/canic-core/src/dto/auth.rs:17-20` exposes `DelegationAudience::Role(CanisterRole)` and `DelegationAudience::Principal(Principal)` only; scans found no `RolesOrPrincipals` or plural `DelegationAudience::Roles`. |
| Trust-chain and freshness verification centralized | PASS | `crates/canic-core/src/ops/auth/token.rs:73-135` remains the runtime token-material verifier and delegates canonical token validation to delegated verifier modules. |
| Public partial verifier surface absent | PASS | The partial `verify_token_material(...)` helper in `crates/canic-core/src/api/auth/mod.rs:85-103` remains private and documented as token-material verification only. |
| Delegated-session bootstrap remains narrower than endpoint auth | PASS | `crates/canic-core/src/api/auth/session/mod.rs:60` calls private `verify_token_material(...)`; endpoint authorization still requires subject binding and update replay consumption elsewhere. |
| Protected internal role predicates verify root-signed proof | PASS | `crates/canic-macros/src/endpoint/expand.rs:739-806` decodes the internal call envelope and calls `AuthApi::verify_internal_invocation_proof(...)` before handler args are decoded. |
| Raw caller and authenticated subject lanes stay separate | PASS | `crates/canic-macros/src/endpoint/expand.rs:308-318` stores transport caller and authenticated subject separately in `AccessContext`. |
| Root/internal auth-adjacent endpoints have explicit gates or public intent | PASS | Generated endpoint bundles show controller, registered-subnet, root, internal proof, or intentionally public query/status surfaces. |

## Entrypoint Map

| Entrypoint Class | Current Path | Boundary Status |
| --- | --- | --- |
| Macro-generated authenticated handlers | `canic-macros/src/endpoint/expand.rs` -> `access::expr::eval_access` -> `access::auth::delegated_token_verified` | Full canonical endpoint boundary |
| Protected internal role endpoints | `caller::has_role(...)` / `caller::has_any_role(...)` -> generated envelope wrapper -> `AuthApi::verify_internal_invocation_proof(...)` | Root-signed internal proof boundary |
| Delegated-session bootstrap | `api/auth/session.rs` -> private `verify_token_material` -> subject equality/session policy | Verifies before state change; not endpoint authorization |
| Root delegation proof endpoint | `canic_request_delegation` in `crates/canic/src/macros/endpoints/root.rs` | Registered-subnet internal gate plus root flow validation |
| Root role-attestation endpoint | `canic_request_role_attestation` in `crates/canic/src/macros/endpoints/root.rs` | Registered-subnet internal gate plus root flow validation |
| Internal invocation proof request endpoint | `canic_request_internal_invocation_proof` in `crates/canic/src/macros/endpoints/root.rs` | Root-issued proof workflow path |
| Token-material helper | `AuthApi::verify_token_material` | Private delegated-session helper; not exposed as endpoint authorization |

## Canonical Path Walkthrough

The generated authenticated endpoint path is:

1. Endpoint macro parsing accepts `requires(auth::authenticated(...))`.
2. Endpoint macro validation requires the first handler argument to be
   `DelegatedToken`.
3. The wrapper resolves `msg_caller()` and any active delegated-session
   identity.
4. The wrapper builds `AccessContext` with raw transport caller and
   authenticated subject in separate fields.
5. `eval_access(...)` dispatches authenticated predicates to
   `AuthenticatedEvaluator`.
6. `AuthenticatedEvaluator` calls `access::auth::delegated_token_verified(...)`.
7. `access/auth/token.rs` decodes the delegated token from ingress argument
   zero.
8. `AuthOps::verify_token(...)` verifies token material, trust chain,
   freshness, singular audience, signatures, and scopes.
9. `access/auth/token.rs` binds verified subject to authenticated caller.
10. `access/auth/token.rs` enforces the required endpoint scope.
11. `access/auth/token.rs` consumes update tokens once for update calls.
12. Handler logic runs only after access evaluation succeeds.

The generated protected internal role endpoint path is:

1. Endpoint macro parsing accepts `internal, requires(caller::has_role(...))`
   or `caller::has_any_role(...)`.
2. Endpoint macro validation requires update endpoints and `internal`.
3. Macro expansion emits an internal call envelope wrapper instead of normal
   public `eval_access(...)`.
4. The wrapper validates envelope version, target canister, and target method.
5. The wrapper calls `AuthApi::verify_internal_invocation_proof(...)` with the
   accepted roles.
6. Handler arguments are decoded only after proof verification succeeds.

## Comparison to Previous Relevant Run

- Stable: generated authenticated endpoint flows still converge on
  `eval_access(...)` and `delegated_token_verified(...)`.
- Stable: token-material verification remains centralized under
  `AuthOps::verify_token(...)`.
- Stable: partial token-material verification remains private for
  delegated-session bootstrap only.
- Updated: the recurring method now explicitly checks singular delegated-token
  audience and protected internal role proof lowering.
- Updated: protected internal `caller::has_any_role(...)` is included in the
  auth-boundary method because endpoint policy may accept multiple roles while
  token audience remains singular.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-macros/src/endpoint/expand.rs` | `access_stage`, `protected_internal_stage` | Generates public endpoint access wrappers and protected internal proof wrappers | High |
| `crates/canic-macros/src/endpoint/validate.rs` | `validate_authenticated_args` | Ensures authenticated endpoints receive `DelegatedToken` as arg zero | High |
| `crates/canic-core/src/access/expr/mod.rs` | `AccessContext`, `eval_access` | Central dispatch surface before endpoint handler execution | High |
| `crates/canic-core/src/access/expr/evaluators.rs` | `AuthenticatedEvaluator` | Boundary between authenticated predicates and delegated-token verification | High |
| `crates/canic-core/src/access/auth/token.rs` | `delegated_token_verified`, `verify_token` | Endpoint-auth ordering owner for verification, binding, scope, and replay | High |
| `crates/canic-core/src/ops/auth/token.rs` | `AuthOps::verify_token` | Canonical token-material verification stage | High |
| `crates/canic-core/src/api/auth/mod.rs` | `verify_token_material`, `verify_internal_invocation_proof` | Private partial delegated-token helper and public internal-proof boundary | Medium |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| token-material helper drift | `crates/canic-core/src/api/auth/mod.rs:85-103` | Private helper verifies token material only; must stay private or gain full endpoint semantics before export | Medium |
| endpoint ordering pressure | `crates/canic-core/src/access/auth/token.rs:52-63` | Safety depends on preserving verify -> bind -> scope -> update consumption order | Medium |
| generated wrapper gravity | `crates/canic-macros/src/endpoint/expand.rs:280-323` and `739-806` | Wrapper owns caller lanes, access context, internal proof envelope, and handler reachability | Medium |
| caller-lane confusion | `crates/canic-core/src/access/expr/mod.rs` | Raw transport caller and authenticated subject are intentionally separate | Medium |
| internal proof role policy | `caller::has_any_role(...)` | Endpoint policy may be multi-role, but delegated-token audience must remain singular | Medium |

## Dependency Fan-In Pressure

| Module / Symbol Group | Direct Files | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `access::auth` / delegated identity symbols | 7 | `access`, `api`, `macros` | Hub forming |
| `access::expr` / `eval_access` / access predicates | 5 | `access`, `macros` | Rising pressure |
| `DelegatedTokenClaims` / `VerifiedDelegatedToken` / verifier input types | 9 | `access`, `api`, `dto`, `ops` | Hub abstraction |
| protected internal endpoint descriptors/proofs | 8+ | `macros`, `api`, `ops`, `workflow`, generated clients | Hub forming |

## Risk Score

Risk Score: **3 / 10**.

Score contributions:

- `+1` endpoint auth remains concentrated in macro expansion and
  `access::expr`.
- `+1` canonical endpoint safety depends on ordering across
  `access/auth/token.rs` and `ops/auth/token.rs`.
- `+1` partial token-material verification remains present but private for
  delegated-session bootstrap.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n 'authenticated\(\|delegated_token_verified\|resolve_authenticated_identity\|verify_token\|verify_token_material\|require_auth\|admin_\|internal_\|system_' crates/canic-core/src crates/canic-macros/src crates/canic/src -g '*.rs'` | PASS | Entrypoint and auth-boundary search recorded. |
| `rg -n 'access_stage\|resolve_authenticated_identity\|eval_access\|auth::authenticated' crates/canic-macros/src/endpoint -g '*.rs'` | PASS | Macro auth wrapper path recorded. |
| `rg -n 'protected_internal\|verify_internal_invocation_proof\|caller::has_role\|caller::has_any_role' crates/canic-macros/src/endpoint crates/canic-core/src/api crates/canic/src/macros/endpoints -g '*.rs'` | PASS | Protected internal proof path recorded. |
| `rg -n 'delegated_token_verified\|AuthOps::verify_token\|enforce_subject_binding\|enforce_required_scope\|consume_update_token_once\|verify_internal_invocation_proof' crates/canic-core/src/access/auth crates/canic-core/src/ops/auth crates/canic-core/src/api -g '*.rs'` | PASS | Endpoint ordering and internal proof path recorded. |
| `rg -n 'AuthApi::verify_token\|pub fn verify_token\(\|fn verify_token_material\|pub fn verify_token_material\|RolesOrPrincipals\|DelegationAudience::Roles\|Roles\(' crates/canic-core/src crates/canic/src crates/canic-macros/src -g '*.rs'` | PASS | Confirmed no public partial verifier and no plural delegated-token audience shape. |
| `rg -n 'DelegationAudience\|Role\(\|Principal\(\|RolesOrPrincipals\|roles: Vec\|principals: Vec' crates/canic-core/src/dto crates/canic-core/src/ops/auth crates/canic-core/src/access -g '*.rs'` | PASS | Singular audience shape and verifier usage recorded. |
| `rg -n 'canic_update\|canic_query\|requires\(' crates/canic/src/macros/endpoints crates/canic-macros/src crates/canic-core/src -g '*.rs'` | PASS | Endpoint gate map recorded. |
| `rg -l 'access::auth\|delegated_token_verified\|resolve_authenticated_identity\|AuthenticatedIdentitySource\|ResolvedAuthenticatedIdentity' crates canisters -g '*.rs'` | PASS | Fan-in scan recorded 7 files. |
| `rg -l 'access::expr\|eval_access\|AccessExpr\|AccessPredicate\|BuiltinPredicate' crates canisters -g '*.rs'` | PASS | Fan-in scan recorded 5 files. |
| `rg -n 'verify_delegated_token\|resolve_authenticated_identity\|caller_predicates_use_transport_caller_not_authenticated_subject\|required_scope_rejects_when_scope_missing\|update_token_consume_rejects_active_replay\|subject_binding_rejects_mismatched_subject_and_caller\|protected_internal' crates/canic-core/src crates/canic-macros/src -g '*.rs'` | PASS | Relevant test coverage found. |
| `cargo test -p canic-macros authenticated --lib --locked` | PASS | 9 authenticated parser/validator/expansion tests passed. |
| `cargo test -p canic-core verify_delegated_token --lib --locked` | PASS | 12 delegated-token verifier tests passed. |
| `cargo test -p canic-core resolve_authenticated_identity --lib --locked` | PASS | 6 delegated-session identity resolution tests passed. |
| `cargo test -p canic-core caller_predicates_use_transport_caller_not_authenticated_subject --lib --locked` | PASS | Raw caller predicates remain transport-caller based. |
| `cargo test -p canic-core required_scope_rejects_when_scope_missing --lib --locked` | PASS | Required scope rejection remains canonical. |
| `cargo test -p canic-core update_token_consume_rejects_active_replay --lib --locked` | PASS | Update-token replay consumption rejects reuse. |
| `cargo test -p canic-core subject_binding_rejects_mismatched_subject_and_caller --lib --locked` | PASS | Subject binding rejects mismatched token subject/caller. |
| `cargo test -p canic-core protected_internal --lib --locked` | PASS | 9 protected internal descriptor tests passed. |

## Final Verdict

PASS.

All authenticated endpoint paths converge on the canonical boundary or the
separate protected internal root-proof boundary. No public weaker delegated
token verifier was found, no plural delegated-token audience shape remains, and
handler execution stays behind verification, subject binding, scope checks, and
update replay consumption.
