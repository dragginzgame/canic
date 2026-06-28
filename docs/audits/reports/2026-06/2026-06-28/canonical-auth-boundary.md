# Canonical Auth Boundary Invariant Audit - 2026-06-28

## Report Preamble

- Definition path:
  `docs/audits/recurring/invariants/canonical-auth-boundary.md`
- Scope: macro-generated authenticated endpoint expansion, access-expression
  dispatch, delegated-token endpoint verification, private token-material
  helper boundaries, signed role-attestation verification, root proof
  provisioning endpoints, and issuer-local delegated-token issuance surfaces.
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-19/canonical-auth-boundary.md`
- Code snapshot identifier: `4a158a32`
- Method tag/version: `canonical-auth-boundary/current`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp: `2026-06-28T11:24:26Z`
- Branch: `main`
- Worktree: clean before report write; dirty after report write.

## Audit Selection

This audit was selected from the least-recent recurring audit set. The latest
retained reports for current recurring definitions showed a tie at
`2026-06-19`; `canonical-auth-boundary` was selected as the first audit in
that tied set by audit name.

## Audit Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Macro authenticated endpoints require canonical token argument | PASS | `crates/canic-macros/src/endpoint/validate/mod.rs` still requires the first authenticated endpoint argument to be `DelegatedToken`; focused macro tests passed. |
| Generated wrappers converge before handler execution | PASS | `crates/canic-macros/src/endpoint/expand/access.rs` resolves authenticated identity, builds `AccessContext`, and calls `eval_access(...)` before dispatch. |
| Authenticated predicate routes to canonical delegated-token verifier | PASS | `AuthenticatedEvaluator` still calls `access::auth::delegated_token_verified(...)`. |
| Endpoint verifier ordering is canonical | PASS | `crates/canic-core/src/access/auth/token.rs` verifies token material, then enforces subject binding, then enforces required scope. |
| Partial token-material verifier is private | PASS | `AuthApi::verify_token_material(...)` remains private and documents that endpoint authorization must also bind the verified subject to the caller. |
| Current audience/grant semantics are used | PASS | Static scans found active `DelegationAudience::{Canister, CanicSubnet, Project}` and `DelegatedRoleGrant` checks in DTO, ops, workflow, storage, and tests. |
| Retired internal role/proof paths are absent | PASS | Active Rust scans found no `verify_internal_invocation_proof`, `InternalInvocationProof`, `caller::has_role`, or `caller::has_any_role`. |
| Signed role-attestation verifies proof before claims acceptance | PASS | `AuthApi::verify_role_attestation(...)` still routes through runtime auth workflow verification; role-attestation unit tests passed. |
| Root proof provisioning is not endpoint auth | PASS | Root proof batch endpoints remain controller/provisioner scoped; issuer-local token prepare/get/status/install surfaces remain separate from delegated-token endpoint verification. |
| Old role/principal audience compatibility shapes are absent | PASS | Static scans found no active `RolesOrPrincipals`, `DelegationAudience::Role`, `DelegationAudience::Roles`, or `DelegationAudience::Principal`. |

## Entrypoint Map

| Entrypoint Family | Boundary | Result |
| --- | --- | --- |
| `#[canic_query]` / `#[canic_update]` with `auth::authenticated(...)` | Generated macro wrapper -> `AccessContext` -> `eval_access(...)` -> delegated-token verifier | PASS |
| Delegated-session bootstrap | Private material verification plus explicit session/subject handling | PASS |
| Role-attestation prepare/get | Root internal endpoints gated by registered-to-subnet caller checks | PASS |
| Role-attestation verify | Explicit signed role-attestation helper using root proof verification | PASS |
| Root proof batch prepare/get/install | Root/controller or renewal-provisioner provisioning endpoints, not delegated-token endpoint auth | PASS |
| Issuer-local delegated-token prepare/get | Issuer-enabled local endpoints after active proof install | PASS |
| Active proof install/status | Controller-gated install plus public non-secret status surface | PASS |

## Ordering Evidence

The delegated-token endpoint path remains ordered as follows:

1. generated endpoint wrapper reads the transport caller;
2. `resolve_authenticated_identity(...)` resolves the endpoint auth subject;
3. `AccessContext` stores transport caller and authenticated-subject lanes;
4. `eval_access(...)` evaluates the access expression;
5. `AuthenticatedEvaluator` calls `delegated_token_verified(...)`;
6. `AuthOps::verify_token(...)` verifies token material;
7. `enforce_subject_binding(...)` binds verified subject to caller;
8. `enforce_required_scope(...)` checks endpoint scope;
9. handler logic runs only after access evaluation succeeds.

No inspected path authorized from token material alone.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-macros/src/endpoint/expand/access.rs` | `access_stage`, `build_access_plan` | Macro entrypoint convergence wiring | High |
| `crates/canic-macros/src/endpoint/validate/mod.rs` | authenticated argument validation | Compile-time authenticated endpoint argument gate | High |
| `crates/canic-core/src/access/expr/mod.rs` | `AccessContext`, `eval_access` | Central access dispatch boundary | High |
| `crates/canic-core/src/access/expr/evaluators.rs` | `AuthenticatedEvaluator` | Routes authenticated predicates into delegated-token verification | High |
| `crates/canic-core/src/access/auth/token.rs` | `delegated_token_verified`, `verify_token` | Endpoint verifier ordering owner | High |
| `crates/canic-core/src/ops/auth/token.rs` | `AuthOps::verify_token` | Token-material verification and cache integration | High |
| `crates/canic-core/src/api/auth/mod.rs` | `verify_token_material` | Private partial verifier for delegated-session bootstrap only | Medium |
| `crates/canic-core/src/api/auth/mod.rs` | `verify_role_attestation` | Explicit signed role-attestation verification helper | Medium |
| `crates/canic-core/src/ops/auth/delegation/mod.rs` | root proof batch and active proof helpers | Provisioning surface adjacent to endpoint auth | Medium |

## Hub Module Pressure

| Module | Import / Reference Evidence | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| macro endpoint access lowering | access-stage scan found macro parse/validate/expand/test hits | 3 | 2 | 6 |
| access auth verifier lane | access-auth fan-in scan found 9 files | 3 | 2 | 5 |
| access expression dispatch | access-expression scan found 8 files | 3 | 2 | 5 |
| delegated auth DTO/proof family | token/proof/attestation scan found 54 files | 6 | 4 | 6 |
| root proof provisioning and renewal | proof/renewal scan found 39 files | 5 | 3 | 6 |
| role-attestation surfaces | role-attestation scan found 30 files | 5 | 3 | 6 |

Pressure remains moderate and expected for the auth surface. The audit did not
find weaker duplicated endpoint auth logic or provisioning APIs being used as
public endpoint-auth substitutes.

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| partial verifier pressure | `crates/canic-core/src/api/auth/mod.rs` | `verify_token_material(...)` is useful for session bootstrap but incomplete for endpoint auth | Medium |
| macro ordering drift | `crates/canic-macros/src/endpoint/expand/access.rs` | handler dispatch must remain after `eval_access(...)` | Medium |
| role-attestation confusion | `AuthApi::verify_role_attestation` | attestation proof verification must remain separate from delegated-token endpoint auth | Medium |
| provisioning-boundary confusion | root proof batch and active proof APIs | root/issuer provisioning authority names sit near endpoint auth DTOs | Medium |
| auth API churn | `crates/canic-core/src/api/auth/mod.rs` | recent commits continue to touch root proof and issuer surfaces | Medium |
| supplemental PocketIC runner health | local PocketIC role-attestation run | supplemental check hung with a defunct PocketIC child and was stopped | Medium |

### Enum Shock Radius

| Enum | Defined In | Reference Files | Risk |
| --- | --- | ---: | --- |
| `DelegationAudience` | `crates/canic-core/src/dto/auth.rs` | 23 | Medium |
| `DelegatedRoleGrant` | `crates/canic-core/src/dto/auth.rs` | 23 | Medium |
| `BuiltinPredicate::Authenticated` | `crates/canic-core/src/access/expr/mod.rs` | 8 | Medium |

### Cross-Layer Struct Spread

| Struct | Defined In | Reference Files | Risk |
| --- | --- | ---: | --- |
| `AccessContext` | `crates/canic-core/src/access/expr/mod.rs` | 3 | Low |
| `VerifyDelegatedTokenRuntimeInput` | `crates/canic-core/src/ops/auth/types.rs` | 5 | Low |
| `DelegatedTokenClaims` | `crates/canic-core/src/dto/auth.rs` | 10 | Medium |
| `SignedRoleAttestation` | `crates/canic-core/src/dto/auth.rs` | 8 | Medium |
| `ActiveDelegationProof` | `crates/canic-core/src/dto/auth.rs` | 14 | Medium |

### Growing Hub Modules

| Module | Subsystems Imported | Recent Commits | Risk |
| --- | --- | ---: | --- |
| `crates/canic-core/src/api/auth/mod.rs` | auth API, delegated-session helper, root provisioning, role attestation | high recent auth-provisioning churn | Medium |
| `crates/canic-core/src/ops/auth/delegation/mod.rs` | root proof batch, active proof, issuer policy, renewal | high recent root-provisioning churn | Medium |
| `crates/canic-core/src/access/auth/token.rs` | token decode, verification, subject binding, scope checks | focused verifier churn | Medium |
| `crates/canic-macros/src/endpoint/expand/access.rs` | endpoint access context emission | touched in current auth cycle | Medium |

### Capability Surface Growth

| Module | Public Items | Risk |
| --- | ---: | --- |
| `crates/canic-core/src/api/auth/mod.rs` | 18 | Medium |

The auth API public surface remains below the `> 20` risk threshold, but it is
close enough to keep under recurring review.

## Dependency Fan-In Pressure

| Module / Struct | Evidence | Risk |
| --- | --- | --- |
| top-level `dto` imports | `use crate::` aggregate found 25 top-level `dto` import lines | Medium |
| top-level `cdk` imports | `use crate::` aggregate found 23 top-level `cdk` import lines | Medium |
| top-level `ops` imports | `use crate::` aggregate found 7 top-level `ops` import lines | Low |
| top-level `config` imports | `use crate::` aggregate found 6 top-level `config` import lines | Low |
| access auth verifier lane | Direct scan found 9 files | Low |
| access expression dispatch | Direct scan found 8 files | Low |
| delegated token/proof/attestation DTOs | Direct scan found 54 files | Medium |
| role-attestation endpoint/helper terms | Static scan found root endpoints and `AuthApi::verify_role_attestation` | Medium |
| root proof batch and active proof terms | Static scan found API/ops/workflow/protocol usage | Medium |

No fan-in evidence indicated an alternate endpoint-auth verifier path.

## Risk Score

Risk Score: **4 / 10**

Score contributions:

- `+1` generated endpoint access lowering remains security-sensitive.
- `+1` the auth surface intentionally has separate delegated-token,
  role-attestation, root proof provisioning, and issuer-local boundaries.
- `+1` recent auth API/root proof churn increases boundary drift risk.
- `+1` passive auth DTO/proof fan-in remains broad.

Verdict: **Invariant holds with low residual boundary-convergence risk.**

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n 'authenticated\(|delegated_token_verified|resolve_authenticated_identity|verify_token|verify_token_material|require_auth|admin_|internal_|system_' crates/canic-core/src crates/canic-macros/src crates/canic/src -g '*.rs'` | PASS | Broad auth-boundary scan found expected macro, API, access, ops, and endpoint hits. |
| `rg -n 'access_stage|resolve_authenticated_identity|eval_access|auth::authenticated' crates/canic-macros/src/endpoint -g '*.rs'` | PASS | Macro wrapper path remains access-context based. |
| `rg -n 'SignedRoleAttestation|verify_role_attestation|canic_prepare_role_attestation|canic_get_role_attestation|RoleAttestation' crates/canic-macros/src/endpoint crates/canic-core/src/api crates/canic/src/macros/endpoints -g '*.rs'` | PASS | Role-attestation surfaces are explicit root/helper paths. |
| `rg -n 'delegated_token_verified|AuthOps::verify_token|enforce_subject_binding|enforce_required_scope' crates/canic-core/src/access/auth crates/canic-core/src/ops/auth -g '*.rs'` | PASS | Endpoint verification order remains verify, bind, scope. |
| `rg -n 'verify_internal_invocation_proof|InternalInvocationProof|caller::has_role|caller::has_any_role' crates/canic-core/src crates/canic-macros/src crates/canic/src canisters -g '*.rs'` | PASS | No active Rust matches for retired internal role/proof endpoint-auth paths. |
| `rg -n 'AuthApi::verify_token|pub fn verify_token\(|pub\([^)]*\) fn verify_token\(|fn verify_token_material|pub fn verify_token_material|RolesOrPrincipals|DelegationAudience::Roles|DelegationAudience::Role|DelegationAudience::Principal|Roles\(' crates/canic-core/src crates/canic/src crates/canic-macros/src -g '*.rs'` | PASS | Found private `verify_token_material` and public ops verifier only; no public `AuthApi::verify_token` or old audience variants. |
| `rg -n 'DelegationAudience::Canister|DelegationAudience::CanicSubnet|DelegationAudience::Project|DelegatedRoleGrant|claims\.grants|cert\.grants|role_grants_subset|scopes_for_role|audience_accepted' crates/canic-core/src crates/canic/src canisters -g '*.rs'` | PASS | Current audience/grant semantics are present in DTO, ops, workflow, storage, and tests. |
| `rg -n 'RolesOrPrincipals|DelegationAudience::Roles|DelegationAudience::Role|DelegationAudience::Principal|RoleAudienceMustBeSingular|DelegatedTokenAudience|Roles\(|Principals\(' crates/canic-core/src crates/canic/src crates/canic-macros/src canisters -g '*.rs'` | PASS | No active old role/principal audience compatibility shapes found. |
| `rg -n 'canic_update|canic_query|requires\(|internal|EnvOps::require_root|require_delegated_token_issuer_enabled|active_delegation_proof_status|install_active_delegation_proof|prepare_delegation_proof_batch_root|get_delegation_proof_batch_root|install_delegation_proof_batch_root' crates/canic/src/macros/endpoints crates/canic-core/src/api/auth -g '*.rs'` | PASS | Root proof provisioning and issuer-local surfaces are explicitly gated and not endpoint-auth shortcuts. |
| `cargo test --locked -p canic-macros authenticated --lib -- --nocapture` | PASS | 9 authenticated parse/validate/expand tests passed. |
| `cargo test --locked -p canic-core --lib verify_delegated_token -- --nocapture` | PASS | 17 delegated-token verifier tests passed. |
| `cargo test --locked -p canic-core --lib resolve_authenticated_identity -- --nocapture` | PASS | 6 delegated-session identity resolution tests passed. |
| `cargo test --locked -p canic-core --lib caller_predicates_use_transport_caller_not_authenticated_subject -- --nocapture` | PASS | Caller predicates still use transport-caller semantics. |
| `cargo test --locked -p canic-core --lib required_scope_rejects_when_scope_missing -- --nocapture` | PASS | Required-scope rejection remains in endpoint auth. |
| `cargo test --locked -p canic-core --lib subject_binding_rejects_mismatched_subject_and_caller -- --nocapture` | PASS | Mismatched subject/caller binding rejects. |
| `cargo test --locked -p canic-core --lib role_attestation -- --nocapture` | PASS | 13 role-attestation and verifier-gate tests passed. |
| `cargo test --locked -p canic --test protocol_surface -- --nocapture` | PASS | 17 protocol-surface tests passed, including root proof batch, active proof installer, and root role-attestation surfaces. |
| `POCKET_IC_BIN=/home/adam/projects/canic/.tmp/test-runtime/pocket-ic-server-14.0.0/pocket-ic cargo test --locked -p canic-tests --test pic_role_attestation role_attestation_verification_paths -- --nocapture` | BLOCKED | Supplemental parity check hung after starting the serialized PocketIC instance with a defunct `pocket-ic` child; the current command was stopped with Ctrl-C. A stale older `root_suite sharding` PocketIC process from a previous run was also terminated. |
| `ps -eo pid,ppid,stat,etime,comm \| rg 'cargo\|pic_role\|root_suite\|pocket-ic'` | PASS | Confirmed no remaining cargo/PocketIC test processes after cleanup. |

## Final Verdict

PASS with supplemental validation blocked. Every required inspected
authenticated endpoint path still converges on the canonical delegated-token
verifier before handler execution, signed role-attestation remains an explicit
proof-verification helper path, and root proof provisioning remains a
provisioning surface rather than a public endpoint-auth bypass.

## Follow-up Actions

- No auth-boundary remediation is required from this audit.
- Rerun the supplemental PocketIC role-attestation verification path after the
  local PocketIC runner is known healthy.
- Keep watch on `AuthApi::verify_token_material(...)`, endpoint macro access
  lowering, and root proof provisioning surfaces during continued auth work.
