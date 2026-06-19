# Canonical Auth Boundary Invariant Audit - 2026-06-19

## Report Preamble

- Definition path:
  `docs/audits/recurring/invariants/canonical-auth-boundary.md`
- Scope: macro-generated authenticated endpoint expansion, access-expression
  dispatch, delegated-token endpoint verification, private token-material
  helper boundaries, signed role-attestation verification, root proof
  provisioning endpoints, and issuer-local delegated-token issuance surfaces.
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-01/canonical-auth-boundary.md`
- Code snapshot identifier: `16894709`
- Method tag/version: `canonical-auth-boundary/current`
- Comparability status: `non-comparable`
- Auditor: `codex`
- Run timestamp: `2026-06-19`
- Branch: `main`
- Worktree: `dirty`

## Audit Selection

This audit was selected as the next stale recurring invariant after the
access-purity, auth-abstraction, bootstrap, capability, dependency, expiry,
module-structure, subject-caller, and token-trust-chain refreshes. It is
non-comparable with the 2026-06-01 report because the live auth model no
longer has active `caller::has_role(...)`, `caller::has_any_role(...)`, or
`verify_internal_invocation_proof` endpoint-auth paths, and delegated-token
audience DTOs now use `Canister`, `CanicSubnet`, and `Project`.

## Audit Definition Maintenance

The audit definition was reviewed before running the audit and updated to match
the current implementation. The refreshed definition now distinguishes:

- public delegated-token endpoint auth through `auth::authenticated(...)`;
- signed role-attestation verification through `SignedRoleAttestation`
  helpers/endpoints;
- root proof provisioning and issuer-local delegated-token issuance as
  explicit provisioning/issuer surfaces, not endpoint-auth alternatives.

The definition was also updated to scan for the current signed
role-attestation flow, root proof provisioning surfaces, and retired internal
role/proof names. Stale expectations around role/principal audience DTOs and
verifier-local token-use replay were removed. Delegated tokens are bearer
tokens until TTL/audience/grant/subject/scope checks fail; domain replay
belongs to command receipts, not the delegated-token verifier.

## Audit Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Macro authenticated endpoints require canonical token argument | PASS | `crates/canic-macros/src/endpoint/validate/mod.rs` requires the first authenticated endpoint argument to be `DelegatedToken`; focused macro tests passed. |
| Generated wrappers converge before handler execution | PASS | `crates/canic-macros/src/endpoint/expand/access.rs` resolves identity, builds `AccessContext`, and calls `eval_access(...)` before dispatching the handler. |
| Authenticated predicate routes to canonical delegated-token verifier | PASS | `AuthenticatedEvaluator` calls `access::auth::delegated_token_verified(...)`. |
| Endpoint verifier ordering is canonical | PASS | `access/auth/token.rs` verifies token material, then enforces subject binding, then required scope. |
| Partial token-material verifier is private | PASS | `AuthApi::verify_token_material(...)` remains private and is not exported as public endpoint authorization. |
| Current audience/grant semantics are used | PASS | Static scans and verifier tests cover `DelegationAudience::{Canister, CanicSubnet, Project}` and `DelegatedRoleGrant` checks. |
| Retired internal role/proof paths are absent | PASS | Active Rust scans found no `verify_internal_invocation_proof`, `InternalInvocationProof`, `caller::has_role`, or `caller::has_any_role`. |
| Signed role-attestation verifies proof before claims acceptance | PASS | `AuthApi::verify_role_attestation(...)` routes through root proof verification; unit and PocketIC role-attestation tests passed. |
| Root proof provisioning is not endpoint auth | PASS | Root proof batch endpoints are controller/root provisioning surfaces; issuer-local token prepare/get remains signer-local after active proof install. |
| Old role/principal audience compatibility shapes are absent | PASS | Static scans found no active `RolesOrPrincipals`, `DelegationAudience::Role`, `DelegationAudience::Roles`, or `DelegationAudience::Principal`. |

## Entrypoint Map

| Entrypoint Family | Boundary | Result |
| --- | --- | --- |
| `#[canic_query]` / `#[canic_update]` with `auth::authenticated(...)` | Generated macro wrapper -> `AccessContext` -> `eval_access(...)` -> delegated-token verifier | PASS |
| Delegated-session bootstrap | Private material verification plus explicit subject/session checks | PASS |
| Role-attestation prepare/get | Root endpoints with internal registered-to-subnet caller checks | PASS |
| Role-attestation verify | Explicit signed role-attestation helper using root proof verification | PASS |
| Root proof batch prepare/get/install | Root/controller provisioning endpoints, not user endpoint auth | PASS |
| Issuer-local delegated-token prepare/get | Issuer-enabled local endpoints after active proof install | PASS |
| Active proof install/status | Issuer/controller operational surface and non-secret status | PASS |

## Ordering Evidence

The canonical delegated-token endpoint path remains:

1. generated endpoint wrapper reads the transport caller;
2. `resolve_authenticated_identity(...)` resolves the endpoint auth subject;
3. `AccessContext` stores raw caller and authenticated-subject lanes;
4. `eval_access(...)` evaluates the access expression;
5. `AuthenticatedEvaluator` calls `delegated_token_verified(...)`;
6. `AuthOps::verify_token(...)` verifies token material;
7. `enforce_subject_binding(...)` binds verified subject to caller;
8. `enforce_required_scope(...)` checks endpoint scope;
9. handler logic runs only after access evaluation succeeds.

No inspected endpoint path authorized from token material alone.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-macros/src/endpoint/expand/access.rs` | `access_stage`, `build_access_plan` | Macro entrypoint convergence wiring | High |
| `crates/canic-macros/src/endpoint/validate/mod.rs` | `validate_authenticated_args` | Compile-time authenticated endpoint argument gate | High |
| `crates/canic-core/src/access/expr/mod.rs` | `AccessContext`, `eval_access` | Central access dispatch boundary | High |
| `crates/canic-core/src/access/expr/evaluators.rs` | `AuthenticatedEvaluator` | Routes authenticated predicates into delegated-token verification | High |
| `crates/canic-core/src/access/auth/token.rs` | `delegated_token_verified`, `verify_token` | Endpoint verifier ordering owner | High |
| `crates/canic-core/src/ops/auth/token.rs` | `AuthOps::verify_token` | Token-material verification and cache integration | High |
| `crates/canic-core/src/api/auth/mod.rs` | `verify_token_material` | Private partial verifier for delegated-session bootstrap only | Medium |
| `crates/canic-core/src/api/auth/mod.rs` | `verify_role_attestation` | Explicit signed role-attestation verification helper | Medium |
| `crates/canic-core/src/ops/auth/delegation/mod.rs` | root proof batch prepare/get/install | Provisioning surface adjacent to endpoint auth | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| macro endpoint access lowering | access-stage scan found macro parse/validate/expand/test hits | 3 | 2 | 6 |
| access auth verifier lane | access auth scan found 9 direct files | 3 | 2 | 5 |
| access expression dispatch | access expression scan found 8 direct files | 3 | 2 | 5 |
| delegated auth proof DTO family | token/proof/attestation scan found broad DTO/API/ops/test fan-in | 6 | 4 | 6 |
| root proof provisioning | root proof batch and active proof scan found API/ops/workflow/protocol usage | 5 | 3 | 6 |

Pressure is moderate and expected for the auth surface. The audit did not find
weaker duplicated endpoint auth logic or provisioning APIs being used as public
endpoint-auth substitutes.

## Risk Score

Risk Score: **3 / 10**

Score contributions:

- `+1` generated endpoint access context remains security-sensitive.
- `+1` the auth surface intentionally has separate delegated-token,
  role-attestation, and provisioning boundaries.
- `+1` root proof provisioning is adjacent to endpoint auth and has recent
  churn.

Verdict: **Invariant holds with low residual boundary-convergence risk.**

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| partial verifier pressure | `api/auth/mod.rs` | `verify_token_material(...)` is useful for sessions but incomplete for endpoint auth | Medium |
| macro ordering drift | `endpoint/expand/access.rs` | handler dispatch must remain after `eval_access(...)` | Medium |
| role-attestation confusion | `AuthApi::verify_role_attestation` | explicit attestation proof verification must not be treated as delegated-token endpoint auth | Medium |
| provisioning-boundary confusion | root proof batch and active proof APIs | root/issuer provisioning authority names sit near endpoint auth DTOs | Medium |
| retired path reintroduction | old internal invocation and caller-role predicate names | negative scan must keep returning empty for active Rust surfaces | Medium |

### Enum Shock Radius

| Enum | Defined In | Reference Files | Risk |
| --- | --- | ---: | --- |
| `DelegationAudience` | `crates/canic-core/src/dto/auth.rs` | broad auth/protocol/test usage | Medium |
| `DelegatedRoleGrant` | `crates/canic-core/src/dto/auth.rs` | broad auth/protocol/test usage | Medium |
| `BuiltinPredicate::Authenticated` | `crates/canic-core/src/access/expr/mod.rs` | focused access/macro/test usage | Medium |

### Cross-Layer Struct Spread

| Struct | Defined In | Layers Referencing | Risk |
| --- | --- | --- | --- |
| `AccessContext` | `crates/canic-core/src/access/expr/mod.rs` | access, macro expansion, tests | Medium |
| `VerifyDelegatedTokenRuntimeInput` | `crates/canic-core/src/ops/auth/types.rs` | api, access, ops | Medium |
| `DelegatedTokenClaims` | `crates/canic-core/src/dto/auth.rs` | dto, ops, access tests, protocol tests | Medium |
| `SignedRoleAttestation` | `crates/canic-core/src/dto/auth.rs` | dto, api, ops, protocol/PocketIC tests | Medium |
| `ActiveDelegationProof` | `crates/canic-core/src/dto/auth.rs` | api, workflow, ops, storage, protocol tests | Medium |

### Growing Hub Modules

| Module | Subsystems Imported | Recent Commits | Risk |
| --- | --- | ---: | --- |
| `crates/canic-core/src/api/auth/mod.rs` | auth API, delegated-session helper, root provisioning, role attestation | high recent auth-provisioning churn | Medium |
| `crates/canic-core/src/ops/auth/delegation/mod.rs` | root proof batch prepare/get/install, issuer policy, active proof status | high recent root-provisioning churn | Medium |
| `crates/canic-core/src/access/auth/token.rs` | token decode, verification, subject binding, scope checks | focused auth verifier owner | Medium |
| `crates/canic-macros/src/endpoint/expand/access.rs` | endpoint access context emission | low recent churn but high blast radius | Medium |

### Capability Surface Growth

No canonical-auth-boundary capability surface growth was found. Newer root proof
and issuer-local delegated-token endpoints are provisioning/issuer surfaces and
are covered by protocol/gating tests, not alternate endpoint-auth verifier
paths.

## Dependency Fan-In Pressure

| Module / Struct | Evidence | Risk |
| --- | --- | --- |
| access auth verifier lane | Direct scan found 9 files | Low |
| access expression dispatch | Direct scan found 8 files | Low |
| delegated token/proof/attestation DTOs | Direct scan found broad DTO/API/ops/test fan-in | Medium |
| role-attestation endpoint/helper terms | Static scan found root endpoints and `AuthApi::verify_role_attestation` | Medium |
| root proof batch and active proof terms | Static scan found API/ops/workflow/protocol usage | Medium |

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n 'authenticated\(|delegated_token_verified|resolve_authenticated_identity|verify_token|verify_token_material|require_auth|admin_|internal_|system_' crates/canic-core/src crates/canic-macros/src crates/canic/src -g '*.rs'` | PASS | Broad auth-boundary scan found expected macro, API, access, and ops hits. |
| `rg -n 'access_stage|resolve_authenticated_identity|eval_access|auth::authenticated' crates/canic-macros/src/endpoint -g '*.rs'` | PASS | Macro wrapper path remains access-context based. |
| `rg -n 'SignedRoleAttestation|verify_role_attestation|canic_prepare_role_attestation|canic_get_role_attestation|RoleAttestation' crates/canic-macros/src/endpoint crates/canic-core/src/api crates/canic/src/macros/endpoints -g '*.rs'` | PASS | Role-attestation surfaces are explicit root/helper paths. |
| `rg -n 'delegated_token_verified|AuthOps::verify_token|enforce_subject_binding|enforce_required_scope' crates/canic-core/src/access/auth crates/canic-core/src/ops/auth -g '*.rs'` | PASS | Endpoint verification order remains verify, bind, scope. |
| `rg -n 'verify_internal_invocation_proof|InternalInvocationProof|caller::has_role|caller::has_any_role' crates/canic-core/src crates/canic-macros/src crates/canic/src canisters -g '*.rs'` | PASS | No active Rust matches for retired internal role/proof endpoint-auth paths. |
| `rg -n 'AuthApi::verify_token|pub fn verify_token\(|pub\([^)]*\) fn verify_token\(|fn verify_token_material|pub fn verify_token_material|RolesOrPrincipals|DelegationAudience::Roles|DelegationAudience::Role|DelegationAudience::Principal|Roles\(' crates/canic-core/src crates/canic/src crates/canic-macros/src -g '*.rs'` | PASS | Found private `verify_token_material` and public ops verifier only; no public `AuthApi::verify_token` or old audience variants. |
| `rg -n 'DelegationAudience::Canister|DelegationAudience::CanicSubnet|DelegationAudience::Project|DelegatedRoleGrant|claims\.grants|cert\.grants|role_grants_subset|scopes_for_role|audience_accepted' crates/canic-core/src crates/canic/src canisters -g '*.rs'` | PASS | Current audience/grant semantics are present in DTO, ops, workflow, and tests. |
| `rg -n 'RolesOrPrincipals|DelegationAudience::Roles|DelegationAudience::Role|DelegationAudience::Principal|RoleAudienceMustBeSingular|DelegatedTokenAudience|Roles\(|Principals\(' crates/canic-core/src crates/canic/src crates/canic-macros/src canisters -g '*.rs'` | PASS | No active old role/principal audience compatibility shapes found. |
| `rg -n 'canic_update|canic_query|requires\(|internal|EnvOps::require_root|require_delegated_token_issuer_enabled|active_delegation_proof_status|install_active_delegation_proof|prepare_delegation_proof_batch_root|get_delegation_proof_batch_root|install_delegation_proof_batch_root' crates/canic/src/macros/endpoints crates/canic-core/src/api/auth -g '*.rs'` | PASS | Root proof provisioning and issuer-local surfaces are explicitly gated and not endpoint-auth shortcuts. |
| `rg -l 'access::auth|delegated_token_verified|resolve_authenticated_identity|AuthenticatedIdentitySource|ResolvedAuthenticatedIdentity' crates canisters -g '*.rs'` | PASS | Access-auth fan-in is focused. |
| `rg -l 'access::expr|eval_access|AccessExpr|AccessPredicate|BuiltinPredicate' crates canisters -g '*.rs'` | PASS | Access-expression fan-in is focused. |
| `rg -l 'DelegatedTokenClaims|VerifiedDelegatedToken|VerifyDelegatedToken|DelegationProof|SignedRoleAttestation|RoleAttestation' crates canisters -g '*.rs'` | PASS | Delegated auth DTO fan-in is broad but passive. |
| `cargo test --locked -p canic-macros authenticated --lib -- --nocapture` | PASS | 9 authenticated parse/validate/expand tests passed. |
| `cargo test --locked -p canic-core --lib verify_delegated_token -- --nocapture` | PASS | 17 delegated-token verifier tests passed. |
| `cargo test --locked -p canic-core --lib resolve_authenticated_identity -- --nocapture` | PASS | 6 delegated-session identity resolution tests passed. |
| `cargo test --locked -p canic-core --lib caller_predicates_use_transport_caller_not_authenticated_subject -- --nocapture` | PASS | Proves caller predicates use transport caller semantics. |
| `cargo test --locked -p canic-core --lib required_scope_rejects_when_scope_missing -- --nocapture` | PASS | Required-scope rejection remains in endpoint auth. |
| `cargo test --locked -p canic-core --lib subject_binding_rejects_mismatched_subject_and_caller -- --nocapture` | PASS | Mismatched subject/caller binding rejects. |
| `cargo test --locked -p canic-core --lib role_attestation -- --nocapture` | PASS | 13 role-attestation and verifier-gate tests passed. |
| `cargo test --locked -p canic-tests --test pic_role_attestation role_attestation_verification_paths -- --nocapture` | PASS | PocketIC role-attestation proof and rejection paths passed. |
| `cargo test --locked -p canic --test protocol_surface -- --nocapture` | PASS | 11 protocol-surface tests passed, including root proof batch, active proof installer, and root role-attestation surfaces. |

The `canic-core` test commands emitted known
`unfulfilled_lint_expectations` warnings in
`crates/canic-core/src/ops/runtime/metrics/delegated_auth.rs`; they did not
affect the focused test outcomes.

## Final Verdict

PASS. Every inspected authenticated endpoint path still converges on the
canonical delegated-token verifier before handler execution, signed
role-attestation remains an explicit proof-verification helper path, and root
proof provisioning remains a provisioning surface rather than a public
endpoint-auth bypass.

## Follow-up Actions

No remediation is required from this audit. Keep watch on
`AuthApi::verify_token_material(...)`, endpoint macro access lowering, and root
proof provisioning surfaces during continued 0.68 work.
