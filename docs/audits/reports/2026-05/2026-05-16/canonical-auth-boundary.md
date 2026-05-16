# Canonical Auth Boundary Invariant Audit - 2026-05-16

## Report Preamble

- Definition path: `docs/audits/recurring/invariants/canonical-auth-boundary.md`
- Scope: generated endpoint wrappers, access-expression evaluation, delegated-session identity resolution, endpoint delegated-token boundary, token-material verifier surfaces, root/internal auth-adjacent endpoints, and auth boundary tests
- Compared baseline report path: `docs/audits/reports/2026-05/2026-05-09/canonical-auth-boundary.md`
- Code snapshot identifier: `f5b88fe7`
- Method tag/version: `canonical-auth-boundary/current`
- Comparability status: `partially comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-16T13:18:21Z`
- Branch: `main`
- Worktree: dirty; active `0.37.0` cleanup changes and audit report updates were present during the run

## Audit Selection

This was selected as the next oldest latest-run recurring audit after
`bootstrap-lifecycle-symmetry` was refreshed and rerun. Several recurring
audits were tied at `2026-05-09`; `canonical-auth-boundary` was first in the
remaining sorted set.

The run is partially comparable with the May 9 baseline because the recurring
definition was refreshed before this run. The updated method now names current
`crates/canic-macros` and `crates/canic-core/src/access/auth/*` paths, treats
required scope and update-token replay as explicit endpoint-boundary stages,
and records that token-material verification is not by itself endpoint
authorization.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Authenticated endpoint entrypoints mapped | PASS | `crates/canic-macros/src/endpoint/validate.rs:58-60` detects authenticated predicates and requires a token argument; `validate.rs:133-160` requires first arg type `DelegatedToken`. |
| Generated authenticated path converges on canonical boundary | PASS | `crates/canic-macros/src/endpoint/expand.rs:270-283` resolves authenticated identity, builds `AccessContext`, and calls `eval_access(...)` before handler execution. |
| Authenticated predicate reaches endpoint boundary | PASS | `crates/canic-core/src/access/expr/evaluators.rs:384-400` dispatches authenticated predicates to `access::auth::delegated_token_verified(...)` and records the delegated authority after success. |
| Endpoint ordering preserved | PASS | `crates/canic-core/src/access/auth/token.rs:52-63` performs `AuthOps::verify_token(...)`, then subject binding, required-scope validation, and update-token consumption before returning success. |
| Trust-chain and freshness verification centralized | PASS | `crates/canic-core/src/ops/auth/token.rs:73-135` remains the runtime token-material verifier and delegates canonical token validation to `ops/auth/delegated/verify.rs`. |
| Public partial verifier surface absent | PASS | `rg -n 'AuthApi::verify_token\|pub fn verify_token\(\|fn verify_token_material\|pub fn verify_token_material' crates/canic-core/src crates/canic/src -g '*.rs'` found only private `AuthApi::verify_token_material(...)`, public `AuthOps::verify_token(...)`, and endpoint-local private `verify_token(...)`. |
| Delegated-session bootstrap remains narrower than endpoint auth | PASS | `crates/canic-core/src/api/auth/session/mod.rs:60` calls private `verify_token_material(...)`; the public helper remains private and documented in `api/auth/mod.rs:63-83` as token-material verification only. |
| Raw caller and authenticated subject lanes stay separate | PASS | `crates/canic-macros/src/endpoint/expand.rs:271-279` stores transport caller and authenticated subject separately; caller predicate tests passed. |
| Root/internal auth-adjacent endpoints have explicit gates or public intent | PASS | Root, wasm-store, non-root, topology, and shared generated endpoints in `crates/canic/src/macros/endpoints/**` either carry caller predicates, are marked internal, or are intentionally public query/status surfaces. |

## Finding Details

No confirmed canonical auth boundary bypass was found.

The most important watchpoint remains the split between token-material
verification and endpoint authorization. The current code keeps
`AuthApi::verify_token_material(...)` private and explicitly documents that
endpoint authorization must also bind subject-to-caller and consume update
tokens. The endpoint path still performs those stages in
`access/auth/token.rs`.

## Entrypoint Map

| Entrypoint Class | Current Path | Boundary Status |
| --- | --- | --- |
| Macro-generated authenticated handlers | `canic-macros/src/endpoint/expand.rs` -> `access::expr::eval_access` -> `access::auth::delegated_token_verified` | Full canonical endpoint boundary |
| Delegated-session bootstrap | `api/auth/session.rs` -> private `verify_token_material` -> subject equality/session policy | Verifies before state change; not endpoint authorization |
| Root delegation proof endpoint | `canic_request_delegation` in `crates/canic/src/macros/endpoints/root.rs:70-75` | Registered-subnet internal gate plus root-only API requirement |
| Root role-attestation endpoint | `canic_request_role_attestation` in `crates/canic/src/macros/endpoints/root.rs:77-83` | Registered-subnet internal gate plus root flow validation |
| Attestation key-set endpoint | `canic_attestation_key_set` in `crates/canic/src/macros/endpoints/root.rs:85-89` | Registered-subnet internal gate |
| Token-material helper | `AuthApi::verify_token_material` | Private delegated-session helper; not exposed as endpoint authorization |

## Canonical Path Walkthrough

The generated authenticated endpoint path is:

1. Endpoint macro parsing accepts only `requires(auth::authenticated(...))`.
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
   freshness, audience, signatures, and scopes.
9. `access/auth/token.rs` binds verified subject to authenticated caller.
10. `access/auth/token.rs` enforces the required endpoint scope.
11. `access/auth/token.rs` consumes update tokens once for update calls.
12. Handler logic runs only after access evaluation succeeds.

## Comparison to Previous Relevant Run

- Stable: generated authenticated endpoint flows still converge on
  `eval_access(...)` and `delegated_token_verified(...)`.
- Stable: token-material verification remains centralized under
  `AuthOps::verify_token(...)` and the delegated verifier modules.
- Stable: the public partial verifier removed in the May 9 run has not
  reappeared.
- Improved: the recurring audit method now explicitly checks required scope,
  update replay consumption, current macro/core paths, and partial verifier
  public-surface drift.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-macros/src/endpoint/expand.rs` | `access_stage`, `build_access_plan` | Generates the endpoint access wrapper and controls handler reachability | High |
| `crates/canic-macros/src/endpoint/validate.rs` | `validate_authenticated_args` | Ensures authenticated endpoints receive `DelegatedToken` as arg zero | High |
| `crates/canic-core/src/access/expr/mod.rs` | `AccessContext`, `eval_access` | Central dispatch surface before endpoint handler execution | High |
| `crates/canic-core/src/access/expr/evaluators.rs` | `AuthenticatedEvaluator` | Boundary between authenticated predicates and delegated-token verification | High |
| `crates/canic-core/src/access/auth/token.rs` | `delegated_token_verified`, `verify_token` | Endpoint-auth ordering owner for verification, binding, scope, and replay | High |
| `crates/canic-core/src/ops/auth/token.rs` | `AuthOps::verify_token` | Canonical token-material verification stage | High |
| `crates/canic-core/src/api/auth/mod.rs` | `verify_token_material` | Private delegated-session bootstrap helper intentionally short of endpoint auth | Medium |

## Hub Module Pressure

| Module | Fan-In Evidence | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/access/auth/*` | 7 direct files mention access auth or authenticated identity symbols | 4 | 2 | 6 |
| `crates/canic-core/src/access/expr/*` | 5 direct files mention access expression/evaluator symbols | 2 | 1 | 5 |
| `crates/canic-core/src/ops/auth/*` | Auth ops exports token verification, delegated verification, attestation, storage, config, and metrics interactions | 6 | 3 | 7 |
| `crates/canic-macros/src/endpoint/*` | Macro parse/validate/expand own endpoint auth syntax, token arg validation, and wrapper generation | 2 | 1 | 6 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| token-material helper drift | `crates/canic-core/src/api/auth/mod.rs:63-83` | Private helper verifies token material only; must stay private or gain full endpoint semantics before export | Medium |
| endpoint ordering pressure | `crates/canic-core/src/access/auth/token.rs:52-63` | Safety depends on preserving verify -> bind -> scope -> update consumption order | Medium |
| generated wrapper gravity | `crates/canic-macros/src/endpoint/expand.rs:241-287` | Wrapper owns caller lanes, access context, and handler reachability | Medium |
| caller-lane confusion | `crates/canic-core/src/access/expr/mod.rs` | Raw transport caller and authenticated subject are intentionally separate | Medium |

## Dependency Fan-In Pressure

### Module Fan-In

| Module / Symbol Group | Direct Files | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `access::auth` / delegated identity symbols | 7 | `access`, `api`, `core`, `macros` | Hub forming |
| `access::expr` / `eval_access` / access predicates | 5 | `access`, `macros` | Rising pressure |
| `DelegatedTokenClaims` / `VerifiedDelegatedToken` / verifier input types | 9 | `access`, `api`, `dto`, `ops` | Hub abstraction |

### Struct / DTO Fan-In

| Struct / Symbol Group | Defined In | Direct Files | Risk |
| --- | --- | ---: | --- |
| `VerifiedDelegatedToken` / `VerifyDelegatedToken*` | `crates/canic-core/src/ops/auth/delegated/verify.rs`, `ops/auth/types.rs` | 9 | Medium |
| `DelegatedTokenClaims` | `crates/canic-core/src/dto/auth.rs` | 9-symbol group scan | Medium |
| `AccessContext` | `crates/canic-core/src/access/expr/mod.rs` | Endpoint macro and access evaluator tests directly depend on two-lane caller semantics | Medium |

## Risk Score

Risk Score: **3 / 10**

Score contributions:

- `+1` endpoint auth remains concentrated in macro expansion and
  `access::expr`.
- `+1` canonical endpoint safety depends on ordering across
  `access/auth/token.rs` and `ops/auth/token.rs`.
- `+1` partial token-material verification remains present but private for
  delegated-session bootstrap.

Verdict: **Canonical endpoint auth boundary holds with low residual structural
risk.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n 'authenticated\(\|delegated_token_verified\|resolve_authenticated_identity\|verify_token\|verify_token_material\|require_auth\|admin_\|internal_\|system_' crates/canic-core/src crates/canic-macros/src crates/canic/src -g '*.rs'` | PASS | Entrypoint and auth-boundary search recorded. |
| `rg -n 'access_stage\|resolve_authenticated_identity\|eval_access\|auth::authenticated' crates/canic-macros/src/endpoint -g '*.rs'` | PASS | Macro auth wrapper path recorded. |
| `rg -n 'delegated_token_verified\|AuthOps::verify_token\|enforce_subject_binding\|enforce_required_scope\|consume_update_token_once' crates/canic-core/src/access/auth crates/canic-core/src/ops/auth -g '*.rs'` | PASS | Endpoint ordering path recorded. |
| `rg -n 'AuthApi::verify_token\|pub fn verify_token\(\|fn verify_token_material\|pub fn verify_token_material' crates/canic-core/src crates/canic/src -g '*.rs'` | PASS | Confirmed no public `AuthApi::verify_token` or public `verify_token_material` helper. |
| `rg -n 'canic_update\|canic_query\|requires\(' crates/canic/src/macros/endpoints crates/canic-macros/src crates/canic-core/src -g '*.rs'` | PASS | Endpoint gate map recorded. |
| `rg -l 'access::auth\|delegated_token_verified\|resolve_authenticated_identity\|AuthenticatedIdentitySource\|ResolvedAuthenticatedIdentity' crates canisters fleets -g '*.rs'` | PASS | Fan-in scan recorded 7 files. |
| `rg -l 'access::expr\|eval_access\|AccessExpr\|AccessPredicate\|BuiltinPredicate' crates canisters fleets -g '*.rs'` | PASS | Fan-in scan recorded 5 files. |
| `rg -l 'DelegatedTokenClaims\|VerifiedDelegatedToken\|VerifyDelegatedToken' crates canisters fleets -g '*.rs'` | PASS | Token DTO/verifier spread scan recorded 9 files. |
| `cargo test -p canic-macros authenticated -- --nocapture` | PASS | 8 authenticated parser/validator tests passed. |
| `cargo test -p canic-core --lib verify_delegated_token -- --nocapture` | PASS | 10 delegated-token verifier tests passed. |
| `cargo test -p canic-core --lib resolve_authenticated_identity -- --nocapture` | PASS | 6 delegated-session identity resolution tests passed. |
| `cargo test -p canic-core --lib caller_predicates_use_transport_caller_not_authenticated_subject -- --nocapture` | PASS | Raw-caller predicates remain transport-caller based. |
| `cargo test -p canic-core --lib required_scope_rejects_when_scope_missing -- --nocapture` | PASS | Required scope rejection remains canonical. |
| `cargo test -p canic-core --lib update_token_consume_rejects_active_replay -- --nocapture` | PASS | Update-token replay consumption rejects reuse. |
| `cargo test -p canic-core --lib subject_binding_rejects_mismatched_subject_and_caller -- --nocapture` | PASS | Subject binding rejects mismatched token subject/caller. |
| `cargo check -p canic-core -p canic-macros -p canic` | PASS | Auth-bearing crates checked under workspace version `0.37.0`. |
| `cargo clippy -p canic-core -p canic-macros --all-targets -- -D warnings` | PASS | Auth-bearing crates passed clippy. |

## Follow-up Actions

No follow-up actions required.
