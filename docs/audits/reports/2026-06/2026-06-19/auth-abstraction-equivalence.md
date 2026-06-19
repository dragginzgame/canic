# Auth Abstraction Equivalence Invariant Audit - 2026-06-19

## Report Preamble

- Scope: macro-generated authenticated endpoint expansion, access-expression
  dispatch, delegated-token verifier parity, delegated-session identity
  resolution, transport-caller lane separation, canister/subnet/project
  audience binding, role grants, and active root-proof provisioning integration.
- Compared baseline report path: `N/A`
- Code snapshot identifier: `16894709`
- Method tag/version: `auth-abstraction-equivalence-current`
- Comparability status: `non-comparable` - the live audit definition now
  targets the current canister/subnet/project audience model, split endpoint
  macro modules, and direct scan/test evidence after the old trust-chain guard
  script was intentionally removed in the 0.65 line.

## Method Changes

- Updated the recurring definition's audience expectation from stale
  `Canic`/project wording to the live `Canister`, `CanicSubnet`, and
  `Project` DTO variants.
- Updated structural hotspot paths for the split endpoint macro files:
  `endpoint/expand/access.rs` and `endpoint/validate/mod.rs`.
- Kept mechanical trust-chain checks as direct scans and focused tests rather
  than reviving the removed `scripts/ci/run-auth-trust-chain-guards.sh`.

## Executive Summary

Risk score: **3 / 10**.

The invariant holds. Generated authenticated endpoints still route through the
canonical access evaluator and endpoint verifier path:

`macro expansion -> AccessContext -> eval_access -> AuthenticatedEvaluator -> access::auth::delegated_token_verified -> AuthOps::verify_token`.

No generated/helper bypass, subject-lane collapse, public material-only verifier,
plural delegated-token audience shape, or role/principal token-audience model
was found. The main residual risk is normal security hotspot pressure: endpoint
macro expansion, access identity lanes, private `verify_token_material(...)`,
and the active root-proof provisioning/token verifier cluster have all changed
recently in the 0.68 slice.

## Checklist Results

| Check | Result | Evidence |
| --- | --- | --- |
| Auth abstractions identified | PASS | Current abstractions are `#[canic_query]`, `#[canic_update]`, `requires(auth::authenticated(...))`, `caller::has_role(...)`, `caller::has_any_role(...)`, access-expression helpers, and delegated-session identity resolution. |
| Macro generated path preserves identity lanes | PASS | `access_stage_expr_builds_context_from_resolved_identity` and `caller_predicates_use_transport_caller_not_authenticated_subject` passed. |
| Default app guard stays raw caller | PASS | `access_stage_default_guard_marks_identity_source_raw_caller` passed. |
| Authenticated endpoint shape is compile-time guarded | PASS | Macro authenticated parse/validate tests require first argument type `DelegatedToken`. |
| Authenticated predicate routes to canonical verifier | PASS | `AuthenticatedEvaluator` calls `access::auth::delegated_token_verified(...)`; access-auth tests passed for subject binding, scope, decode bounds, session resolution, and verifier ordering. |
| Current delegated-token audience shape | PASS | `DelegationAudience` is `Canister(Principal)`, `CanicSubnet(Principal)`, or `Project(String)`. |
| Removed plural/mixed audience shapes absent from active code | PASS | Active code/current-doc scan found no `Roles`, `Principals`, `RolesOrPrincipals`, or role/principal audience variants; one match remains in historical `CHANGELOG.md` only. |
| Audience/grant binding | PASS | Audience tests cover matching canister/subnet/project context, subset behavior, canonical role grants, and per-role scope subsets. |
| Endpoint multi-role policy stays role-attestation based | PASS | `caller::has_role(...)` / `caller::has_any_role(...)` remain protected endpoint policy terms, separate from delegated-token audience. |
| Public material-only verifier remains blocked | PASS | No public `AuthApi::verify_token` or public `verify_token_material(...)` surface found. |
| Delegated-session convenience path stays narrower than endpoint auth | PASS | `api/auth/session/mod.rs` calls private `verify_token_material(...)`; endpoint auth still binds subject/caller and required scope in access. |
| Integration path exercises generated authenticated endpoint | PASS | `canic-tests` sharding suite passed issuer-local delegated-token verification against `test_verify_delegated_token` after active root proof install. |

## Equivalence Matrix

| Scenario | Expected Behavior | Current Evidence | Result |
| --- | --- | --- | --- |
| Valid delegated token | Generated endpoint accepts after canonical verifier succeeds | `root_suite sharding` issuer-local delegated-token verification passed. | PASS |
| Invalid proof/signature | Canonical verifier rejects before endpoint success | `verify_delegated_token_rejects_root_proof_failure` and issuer-proof failure tests passed. | PASS |
| Mismatched subject/caller | Access rejects after material verification | `subject_binding_rejects_mismatched_subject_and_caller` passed. | PASS |
| Expired token | Verifier rejects at expiry boundary | `verify_delegated_token_rejects_expired_token_at_boundary` passed. | PASS |
| Missing scope | Access/verifier rejects required-scope mismatch | `required_scope_rejects_when_scope_missing` and required-scope verifier tests passed. | PASS |
| Delegated-session subject resolution | Session lane remains separate from transport caller lane | Delegated-session fallback tests and caller-lane predicate test passed. | PASS |
| Canister/subnet/project audience | Accepted only when local verifier context matches | `delegated::audience` tests passed. | PASS |
| Token grants vs cert grants | Token cannot expand grants/scopes beyond cert | grant expansion and per-role scope tests passed. | PASS |

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk |
| --- | --- | --- | --- |
| `crates/canic-macros/src/endpoint/expand/access.rs` | `access_stage`, `build_access_plan` | Auth wrapper generation and identity-lane setup | Medium |
| `crates/canic-macros/src/endpoint/validate/mod.rs` | authenticated argument validation | Compile-time token-bearing endpoint shape guard | Medium |
| `crates/canic-core/src/access/expr/mod.rs` | `AccessContext`, `eval_access` | Shared generated/handwritten access dispatch | Medium |
| `crates/canic-core/src/access/expr/evaluators.rs` | `AuthenticatedEvaluator` | Access evaluator to canonical auth verifier edge | Medium |
| `crates/canic-core/src/access/auth/token.rs` | `delegated_token_verified`, `verify_token` | Endpoint verifier ordering and subject/scope binding | High |
| `crates/canic-core/src/api/auth/mod.rs` | `verify_token_material` | Private partial verifier for session bootstrap only | Medium |
| `crates/canic-core/src/ops/auth/delegated/audience.rs` | audience/grant helpers | Canister/subnet/project and role-grant binding | Medium |
| `crates/canic-core/src/ops/auth/delegation/mod.rs` | root proof provisioning helpers | Recent 0.68 active-proof provisioning pressure | Medium |
| `crates/canic-core/src/dto/auth.rs` | auth DTOs | Passive wire shapes with broad fan-in | Medium |

## Hub Module Pressure

| Module | Evidence | Pressure Score |
| --- | --- | ---: |
| `access::expr` and endpoint macro access expansion | 8 files mention access expression/evaluator symbols, concentrated in access and macro crates. | 5 / 10 |
| `access::auth` identity/verifier path | 9 files mention access auth, delegated verifier, or identity lane symbols. | 5 / 10 |
| Auth DTO/proof symbols | `DelegationProof` appears across runtime, storage, workflow, tests, and provisioning paths. | 6 / 10 |
| 0.68 auth/provisioning cluster | Recent `git log` shows repeated edits in `api/auth`, `ops/auth/delegation`, `dto/auth`, and root canister-signature helpers. | 6 / 10 |

## Early Warning Signals

- `verify_token_material(...)` remains private and intentionally incomplete for
  endpoint auth. Keep it that way unless a future public helper also performs
  endpoint subject binding and replay-sensitive mutation checks.
- Keep generated access output structural: resolve identity, build context,
  evaluate predicates, delegate.
- Keep delegated-token audience as canister/subnet/project acceptance only;
  endpoint role policy belongs to role-attestation predicates.
- Keep root-proof provisioning lifecycle out of endpoint macro expansion.

## Dependency Fan-In Pressure

| Symbol Group | Direct Files | Pressure |
| --- | ---: | --- |
| `access::expr` / `eval_access` / `AccessExpr` / `BuiltinPredicate` | 8 | Medium |
| `access::auth` / `delegated_token_verified` / identity resolution | 9 | Medium |
| `DelegationProof` | 23 | High but expected for active proof provisioning and protocol tests |
| `DelegatedTokenClaims` / `VerifiedDelegatedToken` / `VerifyDelegatedToken*` | 14 | Medium-high, mostly verifier/protocol/test fan-in |

## Risk Score

Risk Score: **3 / 10**.

Derivation:

- `+2` for security-sensitive hotspots in macro expansion, access auth, and
  private material verifier helpers.
- `+1` for auth DTO/proof fan-in and recent 0.68 provisioning edit pressure.
- `0` for confirmed parity breaks; none found.

## Verification Readout

| Check | Result | Notes |
| --- | --- | --- |
| Auth abstraction symbol scan | PASS | Found expected macro/access surfaces. |
| Access auth/identity symbol scan | PASS | Found expected access and session helper surfaces. |
| `DelegationProof` fan-in scan | PASS | High but expected fan-in recorded. |
| Delegated-token verifier fan-in scan | PASS | Medium-high expected fan-in recorded. |
| Removed plural/mixed audience scan | PASS | Only historical `CHANGELOG.md` text matched. |
| Role-attestation vs token-audience scan | PASS | Only protected role-policy docs/history matched; no active role/principal token audience. |
| Canister/subnet/project audience scan | PASS | Live code and docs use current audience/grant model. |
| Public material-only verifier scan | PASS | No public `AuthApi::verify_token` or public `verify_token_material(...)`. |
| Private material verifier scan | PASS | Matches are private helper and session bootstrap caller only. |
| `cargo test --locked -p canic-macros authenticated -- --nocapture` | PASS | 9 tests. |
| `cargo test --locked -p canic-macros access_stage_ -- --nocapture` | PASS | 2 tests. |
| `cargo test --locked -p canic-core --lib access::auth -- --nocapture` | PASS | 18 tests; known delegated-auth metrics lint-expectation warnings emitted. |
| `cargo test --locked -p canic-core --lib verify_delegated_token -- --nocapture` | PASS | 17 tests; same known warnings. |
| `cargo test --locked -p canic-core --lib caller_predicates_use_transport_caller_not_authenticated_subject -- --nocapture` | PASS | 1 test; same known warnings. |
| `cargo test --locked -p canic-core --lib delegated::audience -- --nocapture` | PASS | 4 tests; same known warnings. |
| `cargo test --locked -p canic --test endpoint_macro -- --nocapture` | PASS | 3 tests. |
| `cargo test --locked -p canic --test protocol_surface -- --nocapture` | PASS | 11 tests. |
| `cargo test --locked -p canic-tests --test root_suite sharding -- --nocapture` | PASS | 4 PocketIC root/sharding/provisioning tests. |

## Follow-up Actions

No required remediation.

Watchpoints:

- Keep the delegated-auth metrics lint-expectation cleanup in the existing
  focused lint/hygiene queue.
- Keep auth abstraction reports using direct scans/tests now that the old
  trust-chain guard shell script is intentionally retired.

## Final Verdict

Pass with watchpoints - generated and helper auth abstractions remain equivalent
to the canonical verifier path.
