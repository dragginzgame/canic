# Subject-Caller Binding Invariant Audit - 2026-06-19

## Report Preamble

- Definition path: `docs/audits/recurring/invariants/subject-caller-binding.md`
- Scope: delegated-token endpoint subject binding, access-expression identity
  lanes, endpoint macro access expansion, delegated-session subject resolution,
  private token-material helper boundaries, and root-proof provisioning
  principal separation.
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-29/subject-caller-binding.md`
- Code snapshot identifier: `16894709`
- Method tag/version: `subject-caller-binding-current`
- Comparability status: `non-comparable`
- Auditor: `codex`
- Run timestamp: `2026-06-19`
- Branch: `main`
- Worktree: `dirty`

## Audit Selection

This audit was selected as the next stale recurring invariant after the
dependency-hygiene, expiry/replay, and module-structure refreshes. It is
non-comparable with the May baseline because the live definition now covers the
post-root-proof-provisioning auth surface and explicitly separates endpoint
auth subjects from provisioning principals such as `issuer_pid` and
`installed_by`.

## Audit Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Canonical subject binding exists | PASS | `crates/canic-core/src/access/auth/token.rs` verifies token material through `AuthOps::verify_token`, then calls `enforce_subject_binding(verified.subject, caller)` before `enforce_required_scope(...)`. |
| Mismatched subject rejects | PASS | `cargo test --locked -p canic-core --lib subject_binding -- --nocapture` passed matching and mismatched subject tests. |
| Authenticated predicate uses resolved subject | PASS | `crates/canic-core/src/access/expr/evaluators.rs` passes `ctx.authenticated_caller` into `access::auth::delegated_token_verified(...)`. |
| Caller/topology predicates use transport caller | PASS | `caller_predicates_use_transport_caller_not_authenticated_subject` passed. |
| Endpoint macro preserves both identity lanes | PASS | `crates/canic-macros/src/endpoint/expand/access.rs` builds `AccessContext` from `resolve_authenticated_identity(...)`; focused macro tests passed. |
| Delegated-session invalid subjects reject or fall back | PASS | `resolve_authenticated_identity` and `validate_delegated_session_subject` focused tests passed. |
| Private material helper is not a public endpoint-auth substitute | PASS | `AuthApi::verify_token_material(...)` remains private and documented as incomplete without endpoint caller binding. |
| Provisioning principals stay separate | PASS | Active proof install rejects wrong issuer canister; root-proof provisioning terms are installer/issuer authority, not delegated-token end-user subjects. |
| No bearer fallback path detected | PASS | Scans found positive-cache and material-verifier paths, but no production endpoint path that accepts delegated-token proof without binding the verified subject. |

## Binding Path Walkthrough

The authenticated endpoint path is:

1. Generated endpoint wrapper reads `msg_caller()`.
2. `resolve_authenticated_identity(...)` selects the authenticated subject.
3. The wrapper builds `AccessContext` with raw `caller` and
   `authenticated_caller`.
4. `eval_access(...)` dispatches `auth::authenticated(...)`.
5. The authenticated evaluator calls
   `access::auth::delegated_token_verified(...)` with `ctx.authenticated_caller`.
6. `access/auth/token.rs` verifies delegated-token material, enforces
   `verified.subject == authenticated_subject`, then enforces required scope.

Provisioning flow terms such as `issuer_pid`, root issuer policy, and
`installed_by` do not replace this endpoint-auth subject lane.

## Comparison To Previous Relevant Run

- Changed: the recurring definition now names root proof provisioning and
  active proof install/status as separation checks.
- Stable: canonical endpoint subject binding remains in
  `crates/canic-core/src/access/auth/token.rs`.
- Stable: endpoint macros still construct raw transport and authenticated
  subject lanes before access evaluation.
- Stable: delegated sessions affect only the authenticated subject lane;
  caller/topology predicates still use transport caller.
- Stable: `AuthApi::verify_token_material(...)` remains private and incomplete
  for endpoint authorization.
- No blocker or production cleanup was found.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/access/auth/token.rs` | `verify_token`, `enforce_subject_binding` | Canonical delegated-token subject binding and verifier ordering | High |
| `crates/canic-core/src/access/expr/evaluators.rs` | authenticated predicate branch | Passes authenticated subject into the canonical verifier | High |
| `crates/canic-core/src/access/expr/mod.rs` | `AccessContext` | Stores raw transport caller and authenticated subject lanes | High |
| `crates/canic-macros/src/endpoint/expand/access.rs` | access stage expansion | Generated endpoint wrappers construct the access context | High |
| `crates/canic-core/src/access/auth/identity.rs` | `resolve_authenticated_identity_at` | Delegated-session subject selection and fallback behavior | Medium |
| `crates/canic-core/src/api/auth/session/mod.rs` | `set_delegated_session_subject` | Session bootstrap checks requested subject against verified token subject | Medium |
| `crates/canic-core/src/api/auth/mod.rs` | `verify_token_material` | Private partial verifier that must not become endpoint auth | Medium |
| `crates/canic-core/src/ops/auth/delegated/active_proof.rs` | `install_active_delegation_proof` | Verifies active proof is for the local issuer canister, not an end-user subject | Medium |

## Hub Module Pressure

| Module | Import Tokens | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| access expression caller lanes | caller/auth subject lane scan found 11 direct files | 3 | 2 | 6 |
| access auth binding lane | access-auth binding scan found 9 direct files | 3 | 2 | 5 |
| delegated-token verifier shapes | verifier/claims scan found 19 direct files | 5 | 3 | 6 |
| explicit subject/caller checks | subject/caller mismatch scan found 23 direct files | 5 | 3 | 6 |
| root-proof provisioning identifiers | active proof/root issuer scan found 24 direct files | 5 | 3 | 6 |
| private material/cache helpers | material/cache scan found 6 direct files | 3 | 2 | 4 |

Pressure is moderate and expected for the auth surface. No scan showed a
bearer-auth shortcut or a provisioning principal reused as an endpoint subject.

## Risk Score

Risk Score: **3 / 10**

Score contributions:

- `+1` generated endpoint access context is security-sensitive.
- `+1` `AccessContext` intentionally carries two identity lanes.
- `+1` root proof provisioning adds similarly named principals that must remain
  separate from delegated-token end-user subjects.

Verdict: **Invariant holds with low residual identity-lane and terminology
pressure risk.**

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| caller-lane confusion | `AccessContext`, endpoint macros | raw caller and authenticated subject can intentionally differ during delegated sessions | Medium |
| verifier ordering drift | `access/auth/token.rs` | subject binding must stay after material verification and before scope success | Medium |
| private helper pressure | `api/auth/mod.rs` | `verify_token_material(...)` is useful for sessions but incomplete for endpoint auth | Low |
| provisioning-subject naming pressure | root proof provisioning DTOs/ops | `issuer_pid` and `installed_by` are not end-user auth subjects | Medium |

### Enum Shock Radius

| Enum | Defined In | Reference Files | Risk |
| --- | --- | ---: | --- |
| `AuthenticatedIdentitySource` | `crates/canic-core/src/access/auth/mod.rs` | 5 | Low |
| `BuiltinPredicate::Authenticated` | `crates/canic-core/src/access/expr/mod.rs` | 8 | Medium |

### Cross-Layer Struct Spread

| Struct | Defined In | Layers Referencing | Risk |
| --- | --- | --- | --- |
| `AccessContext` | `crates/canic-core/src/access/expr/mod.rs` | access, macro expansion, tests | Medium |
| `VerifyDelegatedTokenRuntimeInput` | `crates/canic-core/src/ops/auth/types.rs` | api, access, ops | Medium |
| `DelegatedTokenClaims` | `crates/canic-core/src/dto/auth.rs` | dto, ops, access tests, protocol tests | Medium |
| `ActiveDelegationProof` | `crates/canic-core/src/dto/auth.rs` | api, workflow, ops, storage, protocol tests | Medium |

### Growing Hub Modules

| Module | Subsystems Imported | Recent Commits | Risk |
| --- | --- | ---: | --- |
| `crates/canic-core/src/ops/auth/delegation/mod.rs` | root proof batch prepare/get/install, root issuer policy, active proof status | high recent auth-provisioning churn | Medium |
| `crates/canic-core/src/api/auth/mod.rs` | auth API, delegated-token issuer endpoints, root provisioning endpoints | high recent auth-provisioning churn | Medium |
| `crates/canic-core/src/access/auth/token.rs` | token decode, verification, binding, scope checks | focused auth verifier owner | Medium |
| `crates/canic-macros/src/endpoint/expand/access.rs` | endpoint access context emission | low recent churn but high blast radius | Medium |

### Capability Surface Growth

No subject-caller capability surface growth was detected in this run.

## Dependency Fan-In Pressure

| Module / Struct | Evidence | Risk |
| --- | --- | --- |
| `AccessContext` | Direct scan found 3 files; usage is concentrated in access/macro tests | Low |
| `AuthenticatedIdentitySource` | Direct scan found 5 files | Low |
| `VerifyDelegatedTokenRuntimeInput` | Direct scan found 5 files across API/access/ops | Medium |
| `DelegatedTokenClaims` | Direct scan found 11 files | Medium |
| `ActiveDelegationProof` | Direct scan found 13 files | Medium |

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test --locked -p canic-core --lib subject_binding -- --nocapture` | PASS | 2 tests passed for matching and mismatched subject binding. |
| `cargo test --locked -p canic-core --lib caller_predicates_use_transport_caller_not_authenticated_subject -- --nocapture` | PASS | Proves caller/topology predicates use raw transport caller semantics. |
| `cargo test --locked -p canic-core --lib resolve_authenticated_identity -- --nocapture` | PASS | 6 delegated-session identity resolution tests passed. |
| `cargo test --locked -p canic-macros authenticated -- --nocapture` | PASS | 9 authenticated parse/validate/expand tests passed. |
| `cargo test --locked -p canic-macros access_stage_expr_builds_context_from_resolved_identity -- --nocapture` | PASS | Generated access stage preserves both caller lanes. |
| `cargo test --locked -p canic-core --lib validate_delegated_session_subject -- --nocapture` | PASS | 2 invalid delegated-session subject tests passed. |
| `cargo test --locked -p canic-core --lib verify_delegated_token_accepts_self_validating_token_without_proof_lookup -- --nocapture` | PASS | Lower-level self-contained delegated-token material verifier still succeeds. |
| `cargo test --locked -p canic-core --lib install_active_delegation_proof_rejects_wrong_issuer -- --nocapture` | PASS | Active proof install rejects a proof for another issuer canister. |
| `cargo test --locked -p canic-core --lib delegated_token_public_prepare_rejects_subject_mismatch_before_replay -- --nocapture` | PASS | Issuer-local public prepare rejects subject mismatch before replay admission. |
| `cargo test --locked -p canic --test protocol_surface active_delegation_proof_installer_surface_is_issuer_gated -- --nocapture` | PASS | Protocol surface keeps active-proof installer issuer-gated. |
| `rg -n 'enforce_subject_binding\|delegated_token_verified\|VerifyDelegatedTokenRuntimeInput' crates/canic-core/src crates/canic-macros/src -g '*.rs'` | PASS | Canonical verifier and runtime input paths are concentrated in access/API/ops. |
| `rg -n 'AccessContext\|authenticated_caller\|authenticated_subject\|transport_caller\|resolve_authenticated_identity' crates/canic-core/src crates/canic-macros/src -g '*.rs'` | PASS | Macro/access context lanes are explicit. |
| `rg -n 'verify_token_material\|AuthOps::verify_token\|verify_delegated_token_cached_proof_identity' crates/canic-core/src -g '*.rs'` | PASS | Private material helper and cache paths remain owned by API/ops. |
| `rg -n 'issuer_pid\|installed_by\|ActiveDelegationProof\|install_active_delegation_proof' crates/canic-core/src crates/canic/tests -g '*.rs'` | PASS | Provisioning principal terms are present but separate from endpoint auth subject binding. |
| `git log --name-only -n 20 -- crates/canic-core/src/access crates/canic-core/src/api/auth crates/canic-core/src/ops/auth crates/canic-macros/src/endpoint/expand` | PASS | Recent churn is concentrated in root-proof provisioning and auth API/ops. |
| `rg -l 'access::expr\|eval_access\|AccessContext\|BuiltinPredicate::Authenticated\|authenticated_caller\|transport_caller\|authenticated_subject' crates canisters fleets -g '*.rs' \| wc -l` | PASS | 11 direct files. |
| `rg -l 'access::auth\|delegated_token_verified\|resolve_authenticated_identity\|enforce_subject_binding\|AuthenticatedIdentitySource\|ResolvedAuthenticatedIdentity' crates canisters fleets -g '*.rs' \| wc -l` | PASS | 9 direct files. |
| `rg -l 'VerifyDelegatedTokenRuntimeInput\|VerifiedDelegatedToken\|verify_delegated_token\|DelegatedTokenClaims\|claims\.subject' crates canisters fleets -g '*.rs' \| wc -l` | PASS | 19 direct files. |
| `rg -l 'subject mismatch\|subject.*caller\|caller.*subject\|delegated token subject\|subject.*must match caller\|does not match caller' crates/canic-core/src crates/canic-tests/tests canisters/test fleets/test -g '*.rs' \| wc -l` | PASS | 23 direct files. |
| `rg -l 'ActiveDelegationProof\|InstallActiveDelegationProof\|RootDelegationProofBatch\|RootIssuerPolicy\|issuer_pid\|installed_by' crates/canic-core/src crates/canic/tests -g '*.rs' \| wc -l` | PASS | 24 direct files. |
| `rg -l 'verify_token_material\|AuthOps::verify_token\|verify_delegated_token_cached_proof_identity\|positive_cache' crates/canic-core/src -g '*.rs' \| wc -l` | PASS | 6 direct files. |

The `canic-core` test commands emitted the known
`unfulfilled_lint_expectations` warnings in
`crates/canic-core/src/ops/runtime/metrics/delegated_auth.rs`; they did not
affect the focused test outcomes.

## Follow-up Actions

No follow-up actions required.
