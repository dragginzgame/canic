# Capability Scope Enforcement Invariant Audit - 2026-06-13

## Report Preamble

- Scope: delegated-token verify/bind/scope ordering, endpoint subject binding,
  delegated-token local-role scope enforcement, root capability envelope
  validation, structural proof routing, root capability authorization/replay
  ordering, and endpoint-boundary rejection for unauthorized root capability
  callers.
- Definition path:
  `docs/audits/recurring/invariants/capability-scope-enforcement.md`
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-29/capability-scope-enforcement.md`
- Code snapshot identifier: `ea21d8a0`
- Branch: `main`
- Method tag/version: `Method V4.2-current-surface`
- Comparability status: `partially comparable`
- Auditor: `codex`
- Run timestamp: `2026-06-13`
- Worktree: `dirty`

Comparability note: endpoint delegated-token scope ordering, root capability
workflow authorization/replay ordering, and structural capability endpoint
checks remain comparable with the prior run. The prior report's standalone
delegated-grant and role-attestation capability proof-mode surfaces were not
found as active runtime proof modes in current scans; current root capability
proof routing accepts structural proof mode only.

## Executive Summary

Verdict: **PASS**.

The active capability-scope invariant still holds. Endpoint delegated-token
auth keeps the source-enforced order: delegated-token verifier, subject-caller
binding, required-scope check, then endpoint acceptance. The delegated-token
verifier rejects required scopes outside the local-role grant. Root capability
dispatch validates service/version/proof before projecting replay metadata and
entering workflow handlers. Current runtime root capability proof routing is
structural-only, and unauthorized callers are rejected at the endpoint boundary
before root capability dispatch metrics change.

Risk score: **3 / 10**.

No blocker was found. The main residual risk is method drift in the recurring
audit definition and older report wording around removed proof-mode surfaces.

## Findings

| ID | Status | Severity | Area | Finding | Resolution |
| --- | --- | --- | --- | --- | --- |
| CSE-2026-06-13-1 | PASS | High | Endpoint auth ordering | `access/auth/token.rs` still verifies delegated-token material, then enforces subject binding, then required scope. | `access::auth` unit tests passed, including the source-order guard. |
| CSE-2026-06-13-2 | PASS | High | Delegated-token verifier | Required scopes are checked against scopes for the verified local role; missing local-role scope rejects. | `ops::auth::delegated::verify` unit tests passed. |
| CSE-2026-06-13-3 | PASS | Medium | Root capability envelope/proof | Root and non-root capability envelopes validate service/version/proof before workflow dispatch; current proof mode is structural. | `workflow::rpc::capability` unit tests passed. |
| CSE-2026-06-13-4 | PASS | Medium | Root capability authorization/replay | Request-handler preflight preserves explicit authorization/replay ordering and aborts reserved replay slots on policy denial. | `workflow::rpc::request::handler` unit tests passed. |
| CSE-2026-06-13-5 | PASS | Medium | Endpoint boundary | Unauthorized root capability callers are rejected before dispatch metrics change. | Targeted `root_suite` PocketIC test passed after local `icq` was updated to the pinned version. |
| CSE-2026-06-13-6 | INFO | Medium | Audit surface drift | Standalone delegated-grant and role-attestation capability proof paths named in the prior report are no longer active current runtime proof modes. | Recorded as method drift; no active scope-as-identity bypass found. |

## Scenario Matrix

| Scenario | Expected Behavior | Current Evidence | Result |
| --- | --- | --- | --- |
| Valid scope plus mismatched caller | Reject before scope can authorize the wrong actor. | `subject_binding_rejects_mismatched_subject_and_caller` passed via `access::auth`. | PASS |
| Valid caller plus missing scope | Reject as a scope denial after delegated-token verification and subject binding. | `required_scope_rejects_when_scope_missing` passed via `access::auth`. | PASS |
| Valid caller plus valid scope | Accept verified token claims. | `verify_delegated_token_accepts_self_validating_token_without_proof_lookup` passed via delegated verifier module tests. | PASS |
| Required scope outside local-role grant | Reject before endpoint acceptance. | `verify_delegated_token_rejects_required_scope_outside_local_role_grant` passed. | PASS |
| Unauthorized root capability caller | Reject at endpoint boundary before capability dispatch metrics change. | `unauthorized_caller_is_denied_for_each_root_capability_variant` passed. | PASS |
| Supported structural root capability proof | Execute request-cycles path only after envelope/proof validation. | `capability_endpoint_policy_and_structural_paths` passed. | PASS |

## Current Capability Path

Delegated-token endpoint path:

1. `access/auth/token.rs` decodes the delegated token from ingress arguments.
2. `AuthOps::verify_token` performs verifier-side token checks with the
   endpoint-required scopes supplied.
3. `enforce_subject_binding` compares verified subject to caller.
4. `enforce_required_scope` checks endpoint scope against verified token
   scopes.
5. Endpoint execution receives the verified issuer principal only after those
   checks pass.

Root capability path:

1. The root `canic_response_capability_v1` endpoint boundary requires a caller
   registered to the subnet.
2. `workflow/rpc/capability/root.rs` validates service, capability version, and
   proof mode before proof verification.
3. `workflow/rpc/capability/verifier.rs` routes current root proof validation
   through `RootCapabilityProof::Structural`.
4. `project_replay_metadata` projects replay metadata only after proof
   acceptance.
5. `RootResponseWorkflow::response_replay_first` performs replay preflight,
   authorization, execution, and replay commit.

Non-root cycles capability path:

1. `workflow/rpc/capability/nonroot.rs` accepts only structural cycles
   envelopes.
2. `verify_nonroot_structural_cycles_proof` requires the caller to be a cached
   direct child before workflow execution.
3. `NonrootCyclesCapabilityWorkflow::response_replay_first` runs replay and
   cycles authorization before execution.

No current path was found that treats a requested scope, capability payload
field, or structural proof as identity.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `crates/canic-core/src/access/auth/token.rs` | `verify_token`, `enforce_subject_binding`, `enforce_required_scope` | Endpoint subject/scope ordering owner. | High |
| `crates/canic-core/src/access/auth/mod.rs` | access predicate facade and tests | Public access boundary around delegated-token checks and topology predicates. | Medium |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | `verify_delegated_token`, `verify_audience_and_grants`, `verify_scopes` | Token claim, audience, role-grant, freshness, and scope verifier. | High |
| `crates/canic-core/src/workflow/rpc/capability/envelope.rs` | `validate_root_capability_envelope` | Service/version/proof validation before capability dispatch. | Medium |
| `crates/canic-core/src/workflow/rpc/capability/verifier.rs` | `verify_root_capability_proof` | Runtime proof-mode routing; currently structural-only. | Medium |
| `crates/canic-core/src/workflow/rpc/capability/proof.rs` | `verify_root_structural_proof`, `verify_nonroot_structural_cycles_proof` | Structural proof constraints that must not become identity substitutes. | Medium |
| `crates/canic-core/src/workflow/rpc/request/handler/authorize.rs` | `authorize`, request-specific authorization helpers | Capability policy decision surface after endpoint/proof checks. | Medium |
| `crates/canic-core/src/workflow/rpc/request/handler/mod.rs` | `preflight`, replay/authorization pipeline | Replay reservation, policy denial, and execution ordering. | Medium |
| `crates/canic-core/src/dto/capability/mod.rs` | capability DTOs and proof/service enums | Broad cross-layer capability wire surface. | Medium |

## Hub And Fan-In Pressure

| Module / Symbol Group | Direct Files | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| `CapabilityProof` | 13 | `dto`, `ops`, `workflow`, `tests` | Hub forming |
| `CapabilityService` | 11 | `dto`, `ops`, `workflow`, `tests` | Hub forming |
| Capability envelope/response DTO group | 13 | `api`, `dto`, `ops`, `macros`, `tests`, `test canisters` | Architectural gravity well |
| `workflow/rpc/capability/*` | broad internal fan-in, mostly private verifier surface | `api`, `ops`, `workflow`, `tests` | Hub forming |
| `workflow/rpc/request/handler/*` | broad authorization/replay tests and recent edits | `workflow`, `ops`, `metrics`, `tests` | Hub forming |

Pressure remains moderate. The DTOs and workflow handler are expected hubs for
this surface, but capability changes still need coordinated API/workflow/test
updates.

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| stale audit terminology | recurring definition and 2026-05 baseline | Prior report names standalone delegated-grant and role-attestation capability proof modes; current runtime routing is structural-only. | Medium |
| capability DTO fan-in | `dto/capability/mod.rs` | Capability proof/service/envelope symbols appear across API, workflow, ops, macros, tests, and test canisters. | Medium |
| authorization/replay hub pressure | `workflow/rpc/request/handler/*` | Authorization, replay preflight, execution, and metrics meet in the same workflow family. | Medium |

## Enum Shock Radius

| Enum | Defined In | Reference Files | Risk |
| --- | --- | ---: | --- |
| `CapabilityProof` | `crates/canic-core/src/dto/capability/mod.rs` | 13 | Medium |
| `CapabilityService` | `crates/canic-core/src/dto/capability/mod.rs` | 11 | Medium |
| `RootCapability` | `crates/canic-core/src/workflow/rpc/request/handler/capability.rs` | handler-local plus tests | Low |

## Cross-Layer Struct Spread

| Struct | Defined In | Layers Referencing | Risk |
| --- | --- | --- | --- |
| `RootCapabilityEnvelopeV1` | `dto/capability/mod.rs` | api, workflow, ops, macros, tests | Medium |
| `RootCapabilityResponseV1` | `dto/capability/mod.rs` | api, workflow, tests | Medium |
| `NonrootCyclesCapabilityEnvelopeV1` | `dto/capability/mod.rs` | api, workflow, ops, tests | Medium |

## Capability Surface Growth

| Module | Public Items | Risk |
| --- | ---: | --- |
| `access/auth/mod.rs` | moderate facade surface | Medium |
| `workflow/rpc/capability/mod.rs` | moderate facade surface | Medium |
| `workflow/rpc/request/handler/mod.rs` | small public workflow surface, broad private tests | Medium |
| `dto/capability/mod.rs` | DTO/public wire surface | Medium |

No critical predictive architectural signal was detected in this run.

## Verification Readout

Selection evidence:

- Recurring report scan showed several audits tied at `2026-05-29`; after the
  `audience-target-binding` run, the next alphabetical tied audit was
  `capability-scope-enforcement`.
- Baseline compared:
  `docs/audits/reports/2026-05/2026-05-29/capability-scope-enforcement.md`.

Structural scans run:

- `rg -n 'capability|scope|authorize|AuthScope|required_scope|required_scopes|CapabilityProof|RootCapability|RequestFamily|endpoint_scope|canic_response_capability|canic_prepare|canic_get' crates/canic-core/src crates/canic-tests/tests -g '*.rs'`
- `rg -n 'subject_binding_rejects|required_scope_rejects|verify_delegated_token_rejects_required_scope|verify_delegated_token_accepts|authorize_rejects_role_attestation|authorize_request_cycles_records|api::rpc::capability|unauthorized_caller_is_denied|preflight_authorize|capability_endpoint_policy' crates/canic-core/src crates/canic-tests/tests -g '*.rs'`
- `rg -l 'CapabilityProof' crates canisters fleets -g '*.rs'`
- `rg -l 'CapabilityService' crates canisters fleets -g '*.rs'`
- `rg -l 'RootCapabilityEnvelopeV1|NonrootCyclesCapabilityEnvelopeV1|RootCapabilityResponseV1|NonrootCyclesCapabilityResponseV1' crates canisters fleets -g '*.rs'`
- `git log --name-only -n 20 -- crates/canic-core/src crates/canic-tests/tests`

Commands passed:

- `cargo test --locked -p canic-core --lib access::auth -- --nocapture`
  - 18 passed; includes subject binding, required scope, delegated-token
    decode limits, and source-order guard for verify/bind/scope ordering.
- `cargo test --locked -p canic-core --lib ops::auth::delegated::verify -- --nocapture`
  - 17 passed; includes accepted valid scope and rejected required scope
    outside the local-role grant.
- `cargo test --locked -p canic-core --lib workflow::rpc::capability -- --nocapture`
  - 15 passed; includes envelope validation, structural proof mode, target
    hash binding, and replay metadata projection.
- `cargo test --locked -p canic-core --lib workflow::rpc::request::handler -- --nocapture`
  - 32 passed; includes authorization/replay ordering, replay abort on policy
    denial, and request-cycles authorization metrics.
- `cargo test --locked -p canic-tests --test root_suite unauthorized_caller_is_denied_for_each_root_capability_variant -- --test-threads=1 --nocapture`
  - 1 passed after local `icq` was updated to the pinned `0.0.5`.
- `cargo test --locked -p canic-tests --test pic_role_attestation capability_endpoint_policy_and_structural_paths -- --test-threads=1 --nocapture`
  - 1 passed; current structural capability request-cycles path succeeds.
- `icq --version`
  - `icq 0.0.5`.

Tooling note:

- The first `root_suite` attempt failed before running the test because local
  `icq` was `0.0.4` while Canic requires `0.0.5`.
- `bash scripts/ci/install-ic-query.sh` installed the pinned local tool, after
  which the same test passed.

## Risk Score

Risk score remains **3 / 10**:

- `+1` endpoint auth/scope ordering is intentionally concentrated in
  `access/auth/token.rs`.
- `+1` capability DTO/proof/envelope types have broad fan-in.
- `+1` root capability authorization/replay code remains a sensitive workflow
  edit center.

No authorization-before-authentication violation was found.

## Residual Risk

No blocker remains. Keep changes to endpoint delegated-token auth ordering,
`CapabilityProof`, `CapabilityService`, capability envelope DTOs, and root
capability authorization/replay coordinated across API, workflow, metrics, and
tests. The recurring audit definition should eventually be refreshed to match
the current structural-only runtime proof surface.
