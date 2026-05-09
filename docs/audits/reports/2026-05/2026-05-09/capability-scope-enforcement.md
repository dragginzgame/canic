# Capability Scope Enforcement Invariant Audit - 2026-05-09

## Report Preamble

- Definition path: `docs/audits/recurring/invariants/capability-scope-enforcement.md`
- Scope: delegated-token scope ordering, endpoint subject binding, root/non-root capability proof verification, delegated grant claims, role-attestation capability proofs, and root capability workflow authorization
- Compared baseline report path: `docs/audits/reports/2026-04/2026-04-05/capability-scope-enforcement.md`
- Code snapshot identifier: `518f57dd`
- Method tag/version: `Method V4.2`
- Comparability status: `partially comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-05-09T13:00:18Z`
- Branch: `main`
- Worktree: `dirty`

## Audit Selection

This was selected as the next oldest recurring audit after
`auth-abstraction-equivalence` and `canonical-auth-boundary`. Its latest
previous report was
`docs/audits/reports/2026-04/2026-04-05/capability-scope-enforcement.md`.

The run is partially comparable with the April baseline because capability
tests now live under `crates/canic-tests/tests/pic_role_attestation_cases/`,
the public partial `AuthApi::verify_token(...)` helper was removed during the
same audit day, and capability endpoint code has been split into smaller files
under `api/rpc/capability/`. The invariant itself remains directly comparable.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Authentication and binding precede endpoint scope enforcement | PASS | `crates/canic-core/src/access/auth/token.rs:52-63` calls `AuthOps::verify_token(...)`, then `enforce_subject_binding(...)`, then `enforce_required_scope(...)`, then update-token replay consumption. |
| Token verifier scope checks use verified claims | PASS | `crates/canic-core/src/ops/auth/delegated/verify.rs:84-145` verifies cert policy/time, hashes, signatures, claims, audience, and scopes before returning `VerifiedDelegatedToken`; `verify_claims(...)` checks claims scopes against cert scopes and required scopes at `161-204`. |
| Scope cannot substitute identity | PASS | `access/auth/token.rs:61-63` binds subject before endpoint scope/replay work; delegated grants bind `grant.subject` to `caller` at `api/rpc/capability/grant.rs:56-60`; role-attestation capability authorization binds request subject to workflow caller at `workflow/rpc/request/handler/authorize.rs:115-125`. |
| Capability inputs use canonical context | PASS | `api/rpc/capability/verifier.rs:118-125` builds proof verification input from `IcOps::msg_caller()`, `IcOps::canister_self()`, `IcOps::now_secs()`, the canonical capability request, and the proof mode. |
| Capability hash binding precedes authorization | PASS | Role-attestation and delegated-grant verifiers call `verify_capability_hash_binding(...)` before attestation/grant validation in `api/rpc/capability/verifier.rs:63-71` and `91-104`; root dispatch calls `verify_root_capability_proof(...)` before `RootResponseWorkflow::response_replay_first(...)` in `api/rpc/capability/root.rs:61-89`. |
| Structural proof modes do not create identity | PASS | Root structural proofs require a registered caller and only support root cycles or direct-child upgrade paths in `api/rpc/capability/proof.rs:21-49`; non-root structural cycles proofs require a cached direct child at `proof.rs:51-64`. |
| Failure semantics stay ordered | PASS | Tests prove subject mismatch, missing scope, delegated grant scope mismatch, unsupported structural proof, signature failure, hash mismatch, audience mismatch, and expiry fail before successful capability execution. |

## Scenario Matrix

| Scenario | Expected Behavior | Current Evidence | Result |
| --- | --- | --- | --- |
| Valid scope + mismatched caller | Reject before authorization succeeds | `subject_binding_rejects_mismatched_subject_and_caller` and `authorize_rejects_role_attestation_when_subject_mismatches_caller` passed | PASS |
| Valid caller + missing scope | Reject as scope denial after verification | `required_scope_rejects_when_scope_missing` and `verify_delegated_token_rejects_required_scope_outside_claims` passed | PASS |
| Valid caller + valid scope | Accept verified token claims | `verify_delegated_token_accepts_self_validating_token_without_proof_lookup` passed | PASS |
| Valid capability proof | Execute request-cycles capability path | `capability_endpoint_role_attestation_proof_paths` passed valid cycles proof case | PASS |
| Bad capability proof material | Reject before workflow authorization/execution | PocketIC capability proof path rejected tampered signature, hash mismatch, audience mismatch, and expiry | PASS |
| Delegated grant wrong scope | Reject before capability execution | `capability_endpoint_policy_and_structural_paths` passed delegated grant scope rejection | PASS |
| Structural proof unsupported for requested capability | Reject before workflow authorization/execution | `capability_endpoint_policy_and_structural_paths` passed unsupported structural rejection | PASS |

## Current Capability Path

Generated root capability entrypoint:

1. `crates/canic/src/macros/endpoints.rs:160-165` emits
   `canic_response_capability_v1` for root with
   `internal, requires(caller::is_registered_to_subnet())`.
2. The generated endpoint gate authenticates the caller lane before the
   capability API receives the envelope.
3. `api/rpc/capability/root.rs:36-59` validates envelope service/version/proof
   mode.
4. `api/rpc/capability/root.rs:61-84` verifies the capability proof before root
   workflow dispatch.
5. `api/rpc/capability/root.rs:86-89` projects replay metadata and enters
   `RootResponseWorkflow::response_replay_first(...)`.
6. `workflow/rpc/request/handler/mod.rs:146-203` runs replay/authorization in
   the configured order, aborting fresh replay reservations on policy denial.
7. `workflow/rpc/request/handler/authorize.rs:19-82` performs capability-family
   authorization. Request-cycles uses the shared cycles authorization helper;
   other families require root context and role/parent/registry checks.

Generated non-root capability entrypoint:

1. `crates/canic/src/macros/endpoints.rs:168-175` emits the non-root
   `canic_response_capability_v1` as `internal`.
2. `api/rpc/capability/nonroot.rs:35-83` accepts only the structural
   request-cycles proof mode.
3. `api/rpc/capability/proof.rs:51-64` requires the caller to be a cached direct
   child before non-root cycles handling runs.

No capability path was found that treats a scope, grant, role-attestation proof,
or request payload field as identity.

## Comparison to Previous Relevant Run

- Stable: endpoint delegated-token scope enforcement still occurs after token
  verification and subject binding.
- Stable: capability proof checks still happen before workflow authorization and
  execution.
- Changed: capability code is now split across `api/rpc/capability/{root,
  nonroot, verifier, proof, grant}.rs`, making individual checks easier to
  inspect than the April report's broader module reference.
- Stable: role-attestation and delegated-grant capability proofs still bind the
  proof hash to the canonical capability payload.
- Stable: PocketIC tests still cover valid proof success, proof-material
  rejection, policy rejection, structural proof support, and delegated grant
  scope rejection.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/access/auth/token.rs` | `verify_token`, `enforce_subject_binding`, `enforce_required_scope` | Endpoint subject/scope ordering owner | High |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | `verify_delegated_token`, `verify_claims`, `verify_scopes` | Token claim, audience, freshness, and scope verifier | High |
| `crates/canic-core/src/api/rpc/capability/verifier.rs` | `RootCapabilityProof`, `RoleAttestationVerifier`, `DelegatedGrantVerifier` | Capability proof routing and proof-context construction | Medium |
| `crates/canic-core/src/api/rpc/capability/grant.rs` | `verify_root_delegated_grant_claims` | Delegated grant subject/audience/scope/freshness checks | Medium |
| `crates/canic-core/src/api/rpc/capability/proof.rs` | `verify_root_structural_proof`, `verify_nonroot_structural_cycles_proof` | Structural proof constraints that must not become identity substitutes | Medium |
| `crates/canic-core/src/workflow/rpc/request/handler/authorize.rs` | `authorize`, `authorize_issue_role_attestation` | Capability policy decision surface after proof/auth checks | Medium |
| `crates/canic-core/src/dto/capability/mod.rs` | `CapabilityProof`, `CapabilityService`, envelope DTOs | Broad cross-layer capability wire surface | Medium |

## Hub Module Pressure

| Module | Fan-In Evidence | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| `crates/canic-core/src/api/rpc/capability/*` | proof/envelope/grant/hash/root/nonroot/verifier split across API and tests | 4 | 2 | 6 |
| `crates/canic-core/src/workflow/rpc/request/handler/*` | recent edit scan shows repeated work in handler tests, execute, replay, capability, authorize | 4 | 2 | 7 |
| `crates/canic-core/src/dto/capability/*` | `CapabilityProof` appears in 14 direct files; envelope DTO group appears in 15 direct files | 5 | 3 | 7 |
| `crates/canic-core/src/access/auth/*` | subject/scope ordering and delegated-token replay remain centralized here | 4 | 2 | 6 |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| DTO shock radius | `crates/canic-core/src/dto/capability/mod.rs` | `CapabilityProof` appears in 14 direct files, `CapabilityService` in 15, envelope DTO group in 15 | High |
| workflow handler edit pressure | `crates/canic-core/src/workflow/rpc/request/handler/*` | recent git scan shows handler tests touched 8 times, execute 6, replay 5, capability/authorize 4 each | Medium |
| proof-mode split | `crates/canic-core/src/api/rpc/capability/verifier.rs` | mitigated by validated `RootCapabilityProof` and canonical `RootCapabilityProofMode` classification | Low |
| auth/scope ordering sensitivity | `crates/canic-core/src/access/auth/token.rs` | correctness depends on preserving verify -> subject binding -> required scope -> replay consumption order | Medium |

## Dependency Fan-In Pressure

### Module Fan-In

| Module / Symbol Group | Direct Files | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `CapabilityProof` | 14 | `api`, `dto`, `ops`, `tests`, `testkit` | Architectural gravity well |
| `CapabilityService` | 15 | `api`, `dto`, `ops`, `tests`, `testkit` | Architectural gravity well |
| Capability envelope/response DTO group | 15 | `api`, `dto`, `ops`, `macros`, `tests`, `testkit` | Architectural gravity well |
| Root capability workflow group | 35 | `api`, `dto`, `ops`, `workflow`, `metrics`, `tests`, `macros` | Architectural gravity well |

### Public Capability Surface

| Module | Public Items | Risk |
| --- | ---: | --- |
| `crates/canic-core/src/dto/capability/*` | 10 | Normal but broad DTO surface |
| `crates/canic-core/src/api/rpc/capability/*` | 1 public helper (`root_capability_hash`) plus private verifier surface | Low |
| `crates/canic-core/src/workflow/rpc/request/handler/*` | 2 public workflow structs in scoped modules | Low |

## Risk Score

Initial Risk Score: **4 / 10**

Post-remediation Risk Score: **3 / 10**

Score contributions:

- `+1` endpoint auth/scope ordering remains concentrated in
  `access/auth/token.rs`.
- `+1` capability DTO/proof/envelope surfaces have high direct file fan-in.
- `+1` workflow handler edit pressure remains active around replay,
  authorization, and execution.

Remediation removed the proof-mode split contribution by introducing
`RootCapabilityProof` as the validated proof view and `RootCapabilityProofMode`
as the canonical logging/metrics classification. Root and non-root dispatch now
derive proof metrics and verifier routing from that validated view instead of
re-matching raw DTOs in multiple places.

Verdict: **Invariant holds with reduced residual capability surface coupling
risk.**

## Remediation Applied

| Change | Files | Result |
| --- | --- | --- |
| Added validated root capability proof view and canonical proof mode | `api/rpc/capability/mod.rs`, `envelope.rs`, `root.rs`, `nonroot.rs`, `verifier.rs` | Proof header validation, verifier dispatch, logs, and metrics now share one proof classification. |
| Removed impossible verifier mismatch branches | `api/rpc/capability/verifier.rs` | Role-attestation and delegated-grant verifiers receive typed proof blobs from validated dispatch. |
| Added proof-mode regression tests | `api/rpc/capability/tests.rs` | Root and non-root envelope validation now asserts the returned canonical proof mode. |
| Cleaned delegated-session storage helpers found during verification | `ops/storage/auth/mod.rs`, `storage/stable/auth/mod.rs`, `storage/stable/auth/sessions.rs` | Dead separate bootstrap-binding upsert path removed; combined atomic upsert uses grouped capacity limits and passes `-D warnings`. |

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-core --lib subject_binding_rejects_mismatched_subject_and_caller -- --nocapture` | PASS | Valid token material with mismatched subject/caller rejects before scope can authorize. |
| `cargo test -p canic-core --lib required_scope_rejects_when_scope_missing -- --nocapture` | PASS | Missing endpoint scope rejects after verification/binding path. |
| `cargo test -p canic-core --lib verify_delegated_token_rejects_required_scope_outside_claims -- --nocapture` | PASS | Token verifier rejects required scope outside verified claims. |
| `cargo test -p canic-core --lib verify_delegated_token_accepts_self_validating_token_without_proof_lookup -- --nocapture` | PASS | Valid caller/scope token material succeeds. |
| `cargo test -p canic-core --lib authorize_rejects_role_attestation_when_subject_mismatches_caller -- --nocapture` | PASS | Capability authorization rejects request subject mismatch. |
| `cargo test -p canic-core --lib authorize_request_cycles_records_requested_and_child_not_found_denial_metrics -- --nocapture` | PASS | Request-cycles authorization denial remains explicit and metric-covered. |
| `cargo test -p canic-tests --test pic_role_attestation capability_endpoint_role_attestation_proof_paths -- --test-threads=1 --nocapture` | PASS | PocketIC capability endpoint accepts valid proof and rejects tampered signature, hash mismatch, audience mismatch, and expiry. |
| `cargo test -p canic-tests --test pic_role_attestation capability_endpoint_policy_and_structural_paths -- --test-threads=1 --nocapture` | PASS | PocketIC capability endpoint rejects policy mismatch, unsupported structural proof, and delegated grant scope mismatch while accepting supported structural cycles proof. |
| `rg -n "require_scope|required_scope|scopes|capability|authorize|allowed_scopes" crates/canic-core/src crates/canic/src/macros/endpoints.rs crates/canic-tests/tests -g '*.rs'` | PASS | Scope/capability map recorded. |
| `rg -l 'CapabilityProof' crates canisters fleets -g '*.rs'` | PASS | Fan-in scan recorded 14 direct files. |
| `rg -l 'CapabilityService' crates canisters fleets -g '*.rs'` | PASS | Fan-in scan recorded 15 direct files. |
| `rg -l 'RootCapabilityEnvelopeV1\|NonrootCyclesCapabilityEnvelopeV1\|RootCapabilityResponseV1\|NonrootCyclesCapabilityResponseV1' crates canisters fleets -g '*.rs'` | PASS | Envelope/response group fan-in scan recorded 15 direct files. |
| `rg -l 'RootCapability\|RootResponseWorkflow\|authorize_\|response_capability_v1\|verify_root_capability_proof\|verify_root_delegated_grant' crates canisters fleets -g '*.rs'` | PASS | Root capability workflow group scan recorded 35 direct files. |
| `git log --name-only -n 20 -- crates/canic-core/src/api/rpc/capability crates/canic-core/src/workflow/rpc/request/handler crates/canic-core/src/dto/capability crates/canic-core/src/access/auth crates/canic-core/src/ops/auth` | PASS | Recent edit-pressure scan recorded handler/auth/capability hotspots. |
| `cargo test -p canic-core --lib api::rpc::capability::tests -- --nocapture` | PASS | 33 capability unit tests passed after remediation. |
| `cargo test -p canic-core --lib resolve_authenticated_identity -- --nocapture` | PASS | Delegated-session helper boundary remains valid for auth identity resolution tests. |
| `cargo test -p canic-core --lib storage::stable::auth::sessions::tests -- --nocapture` | PASS | Atomic delegated-session storage helper tests passed after cleanup. |
| `cargo clippy -p canic-core --lib -- -D warnings` | PASS | Capability remediation and touched auth/session storage helpers pass clippy. |
| `cargo clippy -p canic-core --all-targets -- -D warnings` | PASS | Test targets for the touched capability and auth storage helpers pass clippy. |

## Follow-up Actions

1. Completed: route proof validation, metrics classification, and verifier
   dispatch through the validated `RootCapabilityProof` view.
2. Keep `CapabilityProof`, `CapabilityService`, and capability envelope DTOs
   stable unless a change also updates API, ops, workflow, metrics, and tests
   together.
3. Re-run this audit after changes to `access/auth/token.rs`,
   `api/rpc/capability/verifier.rs`, delegated grants, or root capability
   workflow authorization/replay ordering.
