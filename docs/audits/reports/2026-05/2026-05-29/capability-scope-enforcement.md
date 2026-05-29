# Capability Scope Enforcement Invariant Audit - 2026-05-29

## Report Preamble

- Scope: delegated-token scope ordering, endpoint subject binding, root and
  non-root capability proof verification, delegated grant claims,
  role-attestation capability proofs, structural capability proofs, and root
  capability workflow authorization/replay ordering.
- Definition path:
  `docs/audits/recurring/invariants/capability-scope-enforcement.md`
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-09/capability-scope-enforcement.md`
- Code snapshot identifier: `89cccc85`
- Method tag/version: `Method V4.2`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp: `2026-05-29`
- Worktree: `dirty`

## Executive Summary

Verdict: **PASS WITH TEMPLATE CLEANUP**.

The capability-scope invariant still holds. Endpoint delegated-token auth keeps
the canonical order: token verification, subject-caller binding, required-scope
enforcement, update-token replay consumption, and then handler execution.
Capability endpoint dispatch still validates the envelope and proof before
projecting replay metadata or entering root/non-root workflow handlers.

Delegated grants still bind issuer, subject, audience, capability hash, service,
capability family, quota, and validity window before signature acceptance. Role
attestation proofs still bind the canonical capability hash and then delegate to
the attestation verifier. Structural proofs still require topology evidence and
remain limited to supported capability families.

The only remediation was audit-definition cleanup: the recurring template still
referenced the pre-split `access/auth.rs` hotspot path. The current endpoint
auth ordering owner is `access/auth/token.rs`.

Risk score: **3 / 10**.

## Findings

| ID | Status | Severity | Area | Finding | Resolution |
| --- | --- | --- | --- | --- | --- |
| CSE-2026-05-29-1 | PASS | High | Endpoint auth ordering | Delegated-token endpoint auth still verifies token material before subject binding, required-scope checks, update replay consumption, and handler execution. | Existing source and tests still pass. |
| CSE-2026-05-29-2 | PASS | High | Capability proof dispatch | Root and non-root capability endpoints still validate envelope/proof before workflow dispatch. | Capability unit and PocketIC tests still pass. |
| CSE-2026-05-29-3 | PASS | Medium | Delegated grant claims | Delegated grants still bind subject to caller and scope to the requested capability family. | Unit and PocketIC tests still pass. |
| CSE-2026-05-29-4 | PASS | Medium | Role-attestation proof mode | Role-attestation capability proofs still bind the canonical capability hash before attestation verification. | PocketIC proof-path tests still pass. |
| CSE-2026-05-29-5 | FIXED | Low | Audit template | The recurring audit hotspot table referenced stale `access/auth.rs` ownership. | Updated the template to point at `access/auth/token.rs`. |

## Scenario Matrix

| Scenario | Expected Behavior | Current Evidence | Result |
| --- | --- | --- | --- |
| Valid scope plus mismatched caller | Reject before scope can authorize the wrong actor. | `subject_binding_rejects_mismatched_subject_and_caller` passed. | PASS |
| Valid caller plus missing scope | Reject as scope denial after token verification and subject binding. | `required_scope_rejects_when_scope_missing` and `verify_delegated_token_rejects_required_scope_outside_claims` passed. | PASS |
| Valid caller plus valid scope | Accept verified token claims. | `verify_delegated_token_accepts_self_validating_token_without_proof_lookup` passed. | PASS |
| Valid role-attestation proof | Execute request-cycles capability path. | `capability_endpoint_role_attestation_proof_paths` passed the valid cycles proof case. | PASS |
| Bad proof material | Reject before workflow authorization/execution. | PocketIC capability proof path rejected tampered signature, hash mismatch, audience mismatch, and expiry. | PASS |
| Delegated grant wrong scope | Reject before capability execution. | `capability_endpoint_policy_and_structural_paths` passed delegated grant scope rejection. | PASS |
| Unsupported structural proof | Reject before workflow authorization/execution. | `capability_endpoint_policy_and_structural_paths` passed unsupported structural rejection. | PASS |

## Current Capability Path

Root capability path:

1. `crates/canic/src/macros/endpoints/shared.rs` emits
   `canic_response_capability_v1` for root with
   `internal, requires(caller::is_registered_to_subnet())`.
2. `api/rpc/capability/root.rs` validates service, capability version, and
   proof mode before proof verification.
3. `api/rpc/capability/verifier.rs` builds verification context from the IC
   caller, local target canister, current time, capability version, and
   canonical request payload.
4. Role-attestation and delegated-grant verifiers bind the canonical
   capability hash before their proof-specific checks.
5. `api/rpc/capability/root.rs` projects replay metadata only after proof
   acceptance, then calls `RootResponseWorkflow::response_replay_first(...)`.
6. Workflow authorization enforces root-only, child/registry, subject, subnet,
   audience, TTL, and capability-family policy before execution.

Non-root cycles capability path:

1. `crates/canic/src/macros/endpoints/shared.rs` emits the non-root
   `canic_response_capability_v1` as `internal`.
2. `api/rpc/capability/nonroot.rs` accepts only the structural cycles proof
   mode.
3. `api/rpc/capability/proof.rs` requires the caller to be a cached direct
   child before non-root cycles workflow runs.
4. `workflow/rpc/request/handler/nonroot_cycles.rs` performs replay preflight,
   authorization, execution, and replay commit in order.

No capability path was found that treats a scope, grant, role-attestation proof,
structural proof, or request payload field as identity.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/access/auth/token.rs` | `verify_token`, `enforce_subject_binding`, `enforce_required_scope` | Endpoint subject/scope ordering owner. | High |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | `verify_delegated_token`, `verify_claims`, `verify_scopes` | Token claim, audience, freshness, and scope verifier. | High |
| `crates/canic-core/src/api/rpc/capability/verifier.rs` | `RootCapabilityProof`, `RoleAttestationVerifier`, `DelegatedGrantVerifier` | Capability proof routing and proof-context construction. | Medium |
| `crates/canic-core/src/api/rpc/capability/grant.rs` | `verify_root_delegated_grant_claims` | Delegated grant subject/audience/scope/freshness checks. | Medium |
| `crates/canic-core/src/api/rpc/capability/proof.rs` | `verify_root_structural_proof`, `verify_nonroot_structural_cycles_proof` | Structural proof constraints that must not become identity substitutes. | Medium |
| `crates/canic-core/src/workflow/rpc/request/handler/authorize.rs` | `authorize`, `authorize_issue_role_attestation` | Capability policy decision surface after proof/auth checks. | Medium |
| `crates/canic-core/src/workflow/rpc/request/handler/nonroot_cycles.rs` | `response_replay_first_with_planner`, `authorize_request_cycles_inner` | Replay/authorization/execution ordering for cycles capabilities. | Medium |
| `crates/canic-core/src/dto/capability/mod.rs` | `CapabilityProof`, `CapabilityService`, envelope DTOs | Broad cross-layer capability wire surface. | Medium |

## Hub And Fan-In Pressure

| Module / Symbol Group | Direct Files | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `CapabilityProof` | 15 | `api`, `dto`, `ops`, `tests`, `test canisters` | Architectural gravity well |
| `CapabilityService` | 14 | `api`, `dto`, `ops`, `tests` | Architectural gravity well |
| Capability envelope/response DTO group | 15 | `api`, `dto`, `ops`, `macros`, `tests`, `test canisters` | Architectural gravity well |
| `api/rpc/capability/*` | broad internal fan-in, mostly private verifier surface | `api`, `ops`, `workflow`, `tests` | Hub forming |
| `workflow/rpc/request/handler/*` | recent edits in 0.40, 0.48.5, and 0.48.6 | `workflow`, `ops`, `metrics`, `tests` | Hub forming |

## Verification Readout

Commands passed:

- `cargo +1.96.0 test -p canic-core --lib subject_binding_rejects_mismatched_subject_and_caller --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-core --lib required_scope_rejects_when_scope_missing --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-core --lib verify_delegated_token_rejects_required_scope_outside_claims --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-core --lib verify_delegated_token_accepts_self_validating_token_without_proof_lookup --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-core --lib authorize_rejects_role_attestation_when_subject_mismatches_caller --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-core --lib authorize_request_cycles_records_requested_and_child_not_found_denial_metrics --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-core --lib api::rpc::capability::tests --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-tests --test pic_role_attestation capability_endpoint_role_attestation_proof_paths --locked -- --test-threads=1 --nocapture`
- `cargo +1.96.0 test -p canic-tests --test pic_role_attestation capability_endpoint_policy_and_structural_paths --locked -- --test-threads=1 --nocapture`

Commands used as source scans:

- `rg -n "require_scope|required_scope|scopes|capability|authorize|allowed_scopes" crates/canic-core/src crates/canic/src crates/canic-tests/tests -g '*.rs'`
- `rg -n "pub (async )?fn verify_token|verify_token_material|verify_delegated_token|enforce_required_scope|enforce_subject_binding|consume_update_token_once" crates/canic-core/src crates/canic/src -g '*.rs'`
- `rg -l "CapabilityProof" crates canisters fleets -g '*.rs'`
- `rg -l "CapabilityService" crates canisters fleets -g '*.rs'`
- `rg -l "RootCapabilityEnvelopeV1|NonrootCyclesCapabilityEnvelopeV1|RootCapabilityResponseV1|NonrootCyclesCapabilityResponseV1" crates canisters fleets -g '*.rs'`
- `git log --name-only -n 20 -- crates/canic-core/src/api/rpc/capability crates/canic-core/src/workflow/rpc/request/handler crates/canic-core/src/dto/capability crates/canic-core/src/access/auth crates/canic-core/src/ops/auth`

## Residual Risk

Risk score remains **3 / 10**:

- `+1` endpoint auth/scope ordering is deliberately concentrated in
  `access/auth/token.rs`.
- `+1` capability DTO/proof/envelope types have broad fan-in.
- `+1` workflow handler authorization/replay code remains a sensitive edit
  center.

No blocker remains. Keep `CapabilityProof`, `CapabilityService`, capability
envelope DTOs, endpoint auth ordering, and root capability workflow
authorization/replay changes coordinated across API, ops, workflow, metrics,
and tests.
