# Auth Abstraction Equivalence Invariant Audit - 2026-06-01

## Report Preamble

- Definition path: `docs/audits/recurring/invariants/auth-abstraction-equivalence.md`
- Scope: macro-generated authenticated endpoint expansion, access-expression
  dispatch, delegated-token verifier parity, singular delegated-token audience
  binding, role-attestation caller predicates, delegated-session identity
  resolution, transport-caller lane separation, and trust-chain guard
  integration
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-14/auth-abstraction-equivalence.md`
- Code snapshot identifier: `5f525efc`
- Method tag/version: `Method V5`
- Comparability status: `comparable with expanded singular-audience checks`
- Auditor: `codex`
- Run timestamp: `2026-06-01`
- Branch: `main`
- Worktree: `dirty before report write`

## Audit Selection

This was selected as the next oldest unrefreshed recurring audit after the
`0.57.3` instruction-footprint refresh. `auth-abstraction-equivalence` and
`dry-consolidation` were both last run on `2026-05-14`; auth abstraction was
selected first because the delegated-token audience hard cut and auth workflow
reshuffling make it the higher-risk audit.

## Template Improvement Readout

The audit definition was improved before this run:

- Added the current hard-cut delegated-token model: singular
  `DelegationAudience::Role(CanisterRole)` or
  `DelegationAudience::Principal(Principal)`.
- Added explicit checks that no plural `Roles`, `Principals`, or
  `RolesOrPrincipals` public audience DTO remains.
- Clarified that endpoint role policy (`caller::has_role(...)` /
  `caller::has_any_role(...)`) is separate from delegated-token audience.
- Added `verifier_role_hash` / singular-audience verifier binding to the
  required checklist.
- Added wasm-store protected role-attestation endpoints and
  `ops/auth/delegated/audience.rs` to the structural hotspot list.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Auth abstractions identified | PASS | Current abstractions are `#[canic_query]`, `#[canic_update]`, `requires(auth::authenticated(...))`, `caller::has_role(...)`, `caller::has_any_role(...)`, access-expression helpers, and delegated-session identity resolution. |
| Macro generated path preserves identity lanes | PASS | `crates/canic-macros/src/endpoint/expand.rs` tests confirm generated `AccessContext` uses `transport_caller`, `authenticated_subject`, and `identity_source` from `resolve_authenticated_identity(...)`. |
| Default app guard stays raw caller | PASS | `access_stage_default_guard_marks_identity_source_raw_caller` passed; non-auth default guards do not resolve delegated sessions. |
| Authenticated endpoint shape is compile-time guarded | PASS | `crates/canic-macros/src/endpoint/validate.rs` authenticated tests passed, requiring first argument type `DelegatedToken`. |
| Authenticated predicate routes to canonical verifier | PASS | `AuthenticatedEvaluator` calls `access::auth::delegated_token_verified(...)`; `access::auth` tests passed for subject binding, required scope, replay, stateless query handling, and guard ordering. |
| Singular delegated-token audience shape is current | PASS | `crates/canic-core/src/dto/auth.rs` defines `DelegationAudience::Role(CanisterRole)` and `DelegationAudience::Principal(Principal)` only. |
| Plural/mixed public delegated-token audience is absent from active code | PASS | Targeted scan found no active Rust `Roles`, `Principals`, `RolesOrPrincipals`, `DelegatedTokenAudience`, or `RoleAudienceMustBeSingular` DTO. Historical hits remain in old changelogs, archived designs, and old audit reports only. |
| Role audience verifier binding is exact | PASS | `delegated::audience` tests passed, including `cert_role_hash_requires_exact_single_role_hash`, matching roles, different roles, and role/principal non-cross-match. |
| Principal audience verifier binding is exact | PASS | `verify_delegated_token_accepts_exact_principal_audience_without_local_role` and non-matching principal rejection tests passed. |
| Endpoint multi-role policy stays role-attestation based | PASS | `caller::has_role(...)` / `caller::has_any_role(...)` scan shows protected internal endpoint policy paths; no token multi-role audience model is used. |
| Public material-only verifier remains blocked | PASS | Guard script passed, and the direct `rg` scan found no public `AuthApi::verify_token` / public `verify_token_material` exposure. |
| Delegated-session convenience path keeps endpoint auth semantics | PASS | `access::auth` delegated-session resolution tests passed; convenience resolution remains separate from endpoint token verification. |
| Transport-caller predicates remain separate | PASS | `caller_predicates_use_transport_caller_not_authenticated_subject` passed. |
| Token material verifier still rejects trust failures | PASS | `verify_delegated_token` test filter passed signature, cert-hash drift, noncanonical vector, audience, scope, expiry, local-role, and principal-audience cases. |

## Equivalence Matrix

| Scenario | Expected Behavior | Current Evidence | Result |
| --- | --- | --- | --- |
| Valid credential | Generated/helper path accepts after canonical verifier succeeds | `verify_delegated_token_accepts_self_validating_token_without_proof_lookup` passed | PASS |
| Invalid signature | Generated/helper path rejects through canonical verifier failure | root-signature and shard-signature failure tests passed | PASS |
| Mismatched subject/caller | Token rejects after material verification | `subject_binding_rejects_mismatched_subject_and_caller` passed | PASS |
| Expired credential | Token rejects at verifier boundary | `verify_delegated_token_rejects_expired_token_at_boundary` passed | PASS |
| Missing required scope | Endpoint scope rejects through shared path | `required_scope_rejects_when_scope_missing` and `verify_delegated_token_rejects_required_scope_outside_claims` passed | PASS |
| Update replay | Update token consumption rejects active replay | `update_token_consume_rejects_active_replay` passed | PASS |
| Query replay | Query verification remains stateless | `query_token_consume_is_stateless` passed | PASS |
| Singular role audience | Role-bound cert requires exact verifier role hash and matching local role | `cert_role_hash_requires_exact_single_role_hash`, `verifier_membership_accepts_matching_role`, and missing-local-role verifier tests passed | PASS |
| Singular principal audience | Principal-bound cert requires exact verifier principal and no role hash | exact-principal and non-matching-principal verifier tests passed | PASS |
| Mixed role/principal token-vs-cert audience | Role/principal audience pairs do not cross-match | `role_and_principal_do_not_cross_match` and audience subset drift tests passed | PASS |
| Endpoint multi-role acceptance | Multi-role endpoint policy is implemented through role-attestation predicates, not token audience | `caller::has_any_role(...)` parser/runtime paths are separate from `DelegationAudience` | PASS |
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
7. Delegated-token audience validation remains in
   `ops/auth/delegated/audience.rs`, where role audiences are singular and bind
   to one `verifier_role_hash`, while principal audiences bind to the verifier
   principal with no role hash.
8. `caller::has_role(...)` and `caller::has_any_role(...)` protected internal
   endpoint policy remains an internal-invocation/role-attestation path and
   does not widen delegated-token audience semantics.

No generated or helper-specific bypass was found.

## Comparison to Previous Relevant Run

- Stable: generated authenticated endpoints still converge through
  `access::expr` and `access::auth`.
- Stable: delegated sessions remain identity-resolution convenience and do not
  replace raw transport caller semantics for caller/topology predicates.
- Improved audit scope: the audit now verifies the current singular-audience
  hard cut and distinguishes endpoint multi-role policy from token audience.
- Stable: the auth trust-chain CI guard still mechanically blocks public
  material-only verifier helpers, auth DTO behavior, guard-order drift, and
  broad role-attestation refresh behavior.

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
| `crates/canic-core/src/ops/auth/delegated/audience.rs` | `validate_audience_shape`, `audience_matches_local_verifier`, `audiences_match` | Singular role/principal audience validation and verifier binding | High |
| `crates/canic-core/src/dto/auth.rs` | `DelegationAudience`, `DelegatedToken`, `DelegationProof` | Passive DTO shape; must remain behavior-free and singular-audience | Medium |
| `crates/canic/src/macros/endpoints/wasm_store.rs` | `caller::has_role("root")` protected endpoints | Role-attestation endpoint policy that must not be confused with delegated-token audience | Medium |

## Hub Module Pressure

| Module | Fan-In Evidence | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/access/expr/*` | 5 direct files mention access expression/evaluator symbols | 2 | 1 | 6 |
| `crates/canic-core/src/access/auth/*` | 7 direct files mention access auth, identity resolution, or delegated verifier symbols | 4 | 2 | 6 |
| `crates/canic-core/src/dto/auth.rs` token shapes | `DelegationProof` appears in 12 files; token claims/verifier symbols appear in 9 files | 5 | 2 | 5 |
| `crates/canic-core/src/ops/auth/delegated/audience.rs` | Singular-audience helper referenced by issue/cert/verify/canonical paths | 3 | 1 | 5 |
| `crates/canic-macros/src/endpoint/expand.rs` | Single macro endpoint-auth wrapper source | 2 | 1 | 5 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| macro/auth drift | `crates/canic-macros/src/endpoint/expand.rs` | Generated context setup and guard behavior live in one codegen module | Medium |
| subject-lane confusion | `crates/canic-core/src/access/expr/mod.rs` | `AccessContext` intentionally carries raw caller and authenticated subject | Medium |
| verifier ordering drift | `crates/canic-core/src/access/auth/token.rs` | Security behavior depends on verify -> bind -> scope -> consume ordering | Medium |
| endpoint policy vs token audience confusion | `caller::has_any_role(...)` vs `DelegationAudience::Role` | Endpoint multi-role acceptance is separate from token audience | Medium |
| DTO spread | `crates/canic-core/src/dto/auth.rs` | Auth DTOs are referenced across API, access, ops, tests, and support crates | Low |

## Dependency Fan-In Pressure

### Module Fan-In

| Module / Symbol Group | Direct Files | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `access::expr` / `eval_access` / `AccessExpr` / `BuiltinPredicate` | 5 | `access`, `macros` | Medium |
| `access::auth` / `delegated_token_verified` / identity resolution | 7 | `access`, `api`, `core`, `macros` | Medium |
| `ops/auth/delegated/audience` | 3 | `ops/auth/delegated` | Low |

### Struct / DTO Fan-In

| Struct / Symbol Group | Defined In | Direct Files | Risk |
| --- | --- | ---: | --- |
| `DelegationProof` | `crates/canic-core/src/dto/auth.rs` | 12 | Medium |
| `DelegatedTokenClaims` / `VerifiedDelegatedToken` / `VerifyDelegatedToken*` | `crates/canic-core/src/dto/auth.rs`, `ops/auth` | 9 | Medium |
| `DelegationAudience` | `crates/canic-core/src/dto/auth.rs` | active verifier/issue/canonical/docs paths | Medium |

## Risk Score

Risk Score: **3 / 10**

Score contributions:

- `+1` generated endpoint auth remains concentrated in one macro expansion
  module.
- `+1` `AccessContext` deliberately carries two caller identities.
- `+1` delegated-token endpoint behavior depends on ordering across access,
  ops, and DTO surfaces.
- `+1` endpoint role policy and delegated-token audience are adjacent enough to
  confuse if docs/tests drift.
- `-1` effective reduction from the trust-chain guard and singular-audience
  test coverage; floor remains `3` because the hotspot concentration is
  expected and security-sensitive.

Verdict: **Invariant holds with low residual coupling risk.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `bash scripts/ci/run-auth-trust-chain-guards.sh` | PASS | Public partial verifier, DTO behavior, endpoint ordering, and refresh-scope guards passed. |
| `rg -n 'pub(\([^)]*\))?\s+(async\s+)?fn\s+verify_token\b\|pub(\([^)]*\))?\s+(async\s+)?fn\s+verify_token_material\b\|AuthApi::verify_token\b' crates/canic-core/src/api/auth crates/canic/src -g '*.rs'` | PASS | No public material-only verifier matches. |
| `rg -n 'RolesOrPrincipals\|RoleAudienceMustBeSingular\|DelegatedTokenAudience\|DelegationAudience::Roles\|DelegationAudience::Principals\|Roles\\(\|Principals\\(' crates canisters fleets docs/architecture docs/contracts docs/getting-started docs/operations README* CHANGELOG.md -g '*.rs' -g '*.md'` | PASS | Only historical root changelog text matched; no active code/current docs expose the removed plural audience shape. |
| `cargo test -p canic-macros authenticated --locked -- --nocapture` | PASS | 9 parse/validate authenticated predicate tests passed. |
| `cargo test -p canic-macros access_stage_ --locked -- --nocapture` | PASS | 2 generated access-context tests passed. |
| `cargo test -p canic-core --lib access::auth --locked -- --nocapture` | PASS | 18 auth access tests passed, including scope, subject, replay, query, guard order, and session resolution. |
| `cargo test -p canic-core --lib verify_delegated_token --locked -- --nocapture` | PASS | 12 delegated-token verifier tests passed. |
| `cargo test -p canic-core --lib caller_predicates_use_transport_caller_not_authenticated_subject --locked -- --nocapture` | PASS | Caller-lane separation test passed. |
| `cargo test -p canic-core --lib delegated::audience --locked -- --nocapture` | PASS | 7 singular-audience and verifier-binding tests passed. |
| `rg -l 'access::expr\|eval_access\|AccessExpr\|AccessPredicate\|BuiltinPredicate' crates canisters fleets -g '*.rs'` | PASS | 5 direct files. |
| `rg -l 'access::auth\|delegated_token_verified\|resolve_authenticated_identity\|AuthenticatedIdentitySource\|ResolvedAuthenticatedIdentity' crates canisters fleets -g '*.rs'` | PASS | 7 direct files. |
| `rg -l 'DelegationProof' crates canisters fleets -g '*.rs'` | PASS | 12 direct files. |
| `rg -l 'DelegatedTokenClaims\|VerifiedDelegatedToken\|VerifyDelegatedToken' crates canisters fleets -g '*.rs'` | PASS | 9 direct files. |

## Follow-up Actions

No remediation required.

Watchpoints only:

1. Keep macro-generated authenticated endpoints aligned with
   `AccessContext`, `AuthenticatedEvaluator`, and `access/auth/token.rs`.
2. Keep `AuthApi::verify_token_material(...)` private unless a future public
   helper performs full endpoint subject binding and update replay consumption.
3. Keep singular delegated-token audience separate from endpoint multi-role
   policy helpers.
4. Keep `scripts/ci/run-auth-trust-chain-guards.sh` in the fast test lane.
