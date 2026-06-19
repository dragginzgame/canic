# Capability Scope Enforcement Invariant Audit - 2026-06-19

## Report Preamble

- Scope: endpoint delegated-token verify/bind/scope ordering, delegated-token
  local-role scope checks, root capability envelope/proof validation,
  structural root capability proof routing, root capability replay and
  authorization ordering, and endpoint rejection for unauthorized root
  capability callers.
- Definition path:
  `docs/audits/recurring/invariants/capability-scope-enforcement.md`
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-13/capability-scope-enforcement.md`
- Code snapshot identifier: `ef55e53c`
- Branch: `main`
- Method tag/version: `capability-scope-enforcement-current`
- Comparability status: partially comparable. Endpoint auth ordering and root
  capability authorization behavior remain comparable; the audit definition was
  refreshed to target the current structural-only capability proof path instead
  of retired delegated-grant capability proof names.
- Auditor: Codex
- Run timestamp: `2026-06-19T14:00:49Z`
- Worktree state: dirty before audit; unrelated dirty files were preserved.

## Method Changes

- Refreshed the recurring audit definition before execution.
- Removed the stale `api/rpc/capability/grant.rs` delegated-grant capability
  hotspot from the definition.
- Added current structural hotspots for delegated-token verification, root
  capability envelope/proof validation, structural proof routing, replay/
  authorization ordering, and passive capability DTOs.
- Added current targeted unit and PocketIC verification commands.

## Executive Summary

Status: PASS.

Risk score: 3 / 10.

No authorization-before-authentication or scope-as-identity break was found.
Endpoint delegated-token auth still runs token verification, subject-caller
binding, and required-scope enforcement before handler dispatch. Delegated
token verification rejects required scopes outside accepted local-role grants,
and root capability calls still validate the envelope/proof before replay,
authorization, execution, and replay commit.

## Findings

| ID | Status | Severity | Finding | Evidence |
| --- | --- | --- | --- | --- |
| CSE-2026-06-19-1 | PASS | High | Endpoint auth preserves `verify token -> bind subject -> enforce scope -> handler` ordering. | `crates/canic-core/src/access/auth/token.rs`; source-order test `delegated_auth_guard_preserves_verify_bind_scope_order`; `cargo test --locked -p canic-core --lib access::auth -- --nocapture` passed. |
| CSE-2026-06-19-2 | PASS | High | Delegated-token scope checks are derived from verified claims and accepted local-role grants, not request-only payload fields. | `crates/canic-core/src/ops/auth/delegated/verify.rs`; `verify_scopes`; `ScopeRejected` coverage; `cargo test --locked -p canic-core --lib ops::auth::delegated::verify -- --nocapture` passed. |
| CSE-2026-06-19-3 | PASS | Medium | Root capability proof validation is structural-only and does not create identity. | `workflow/rpc/capability/envelope.rs`, `verifier.rs`, and `proof.rs`; `cargo test --locked -p canic-core --lib workflow::rpc::capability -- --nocapture` passed. |
| CSE-2026-06-19-4 | PASS | Medium | Root capability replay reservation, authorization denial, execution, and replay commit ordering remain explicit. | `workflow/rpc/request/handler/mod.rs`; `preflight_replay_then_authorize_*` coverage; `cargo test --locked -p canic-core --lib workflow::rpc::request::handler -- --nocapture` passed. |
| CSE-2026-06-19-5 | PASS | Medium | Unauthorized root capability endpoint callers are rejected at the endpoint boundary. | `crates/canic-tests/tests/root_cases/replay.rs`; `cargo test --locked -p canic-tests --test root_suite unauthorized_caller_is_denied_for_each_root_capability_variant -- --test-threads=1 --nocapture` passed unsandboxed because PocketIC binds a local server. |
| CSE-2026-06-19-6 | INFO | Medium | The audit definition had drifted toward retired delegated-grant capability proof wording and was refreshed before execution. | Definition now uses `capability-scope-enforcement-current` and current verification commands. |

## Scenario Matrix

| Scenario | Expected Outcome | Result |
| --- | --- | --- |
| Valid scope with mismatched caller | Reject during subject-caller binding before scope can authorize | PASS |
| Valid caller with missing required scope | Reject as scope denial after token and subject binding | PASS |
| Valid caller with accepted local-role scope | Accept through delegated-token verifier and endpoint access guard | PASS |
| Unauthorized root capability caller | Reject before protected capability execution | PASS |
| Root capability replay reservation followed by authorization failure | Abort the fresh replay reservation instead of committing success | PASS |

## Current Capability Path

- Endpoint delegated-token auth:
  `AuthOps::verify_token -> enforce_subject_binding -> enforce_required_scope`.
- Delegated-token local verification:
  canonical token/cert/claims checks, audience/grant checks, local-role grant
  acceptance, scope check, freshness, and subject binding.
- Root capability path:
  endpoint guard and envelope validation, structural proof validation, request
  preflight, replay reservation, authorization, execution, replay commit.

## Structural Hotspots

| File / Module | Struct / Function | Reason | Risk Contribution |
| --- | --- | --- | --- |
| `access/auth/token.rs` | `verify_token`, `enforce_subject_binding`, `enforce_required_scope` | canonical endpoint token, subject, and scope ordering | High |
| `ops/auth/delegated/verify.rs` | `verify_delegated_token`, `verify_audience_and_grants`, `verify_scopes` | verifier-local audience, local-role grant, scope, and freshness decisions | High |
| `workflow/rpc/capability/envelope.rs` | `validate_root_capability_envelope` | root capability service/version/proof validation | Medium |
| `workflow/rpc/capability/verifier.rs` | `verify_root_capability_proof` | active capability proof-mode routing | Medium |
| `workflow/rpc/capability/proof.rs` | `verify_root_structural_proof` | structural proof constraints | Medium |
| `workflow/rpc/request/handler/authorize.rs` | `authorize` | root RPC authorization decision surface | Medium |
| `workflow/rpc/request/handler/mod.rs` | `preflight`, replay/authorization pipeline | replay, authorization, execution, and commit ordering | Medium |
| `dto/capability/mod.rs` | `CapabilityProof`, `CapabilityService`, capability envelopes | broad passive wire surface shared by API, workflow, ops, tests, and stubs | Medium |

## Hub / Fan-In Pressure

| Surface | Evidence | Risk |
| --- | --- | --- |
| Capability DTO/proof/envelope surface | Capability names appeared in 16 Rust files across `api`, `dto`, `ops`, `workflow`, tests, and test canisters. | Medium |
| `workflow/rpc/capability/*` | Owns envelope validation, structural proof routing, and compatibility wrappers. | Medium |
| `workflow/rpc/request/handler/*` | Owns replay reservation, authorization, execution, and commit sequencing. | Medium |
| `access/auth/token.rs` | Small but high-impact endpoint auth ordering choke point. | Medium |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| Capability DTO fan-in | `dto/capability/mod.rs` | Referenced by API, ops, workflow, tests, macros, and test canisters. | Medium |
| Replay/authorization edit center | `workflow/rpc/request/handler/*` | Combines replay reservation, policy denial, execution, and commit ordering. | Medium |
| Endpoint auth source-order guard | `access/auth/token.rs` | Correctness depends on stable verify/bind/scope ordering. | Low |

## Risk Score

Risk: 3 / 10.

- +0: no capability/scope enforcement violation found.
- +1: endpoint auth and scope ordering are concentrated in one small but
  high-impact access module.
- +1: capability DTO/proof/envelope surface has medium cross-subsystem fan-in.
- +1: root capability replay and authorization sequencing remains a sensitive
  workflow center.

## Verification Readout

| Check | Result |
| --- | --- |
| Stale definition reference scan for retired delegated-grant capability paths | PASS |
| Structural scan for endpoint token, delegated verifier, capability proof, and replay authorization symbols | PASS |
| Capability DTO/proof/envelope fan-in scan | PASS |
| `cargo test --locked -p canic-core --lib access::auth -- --nocapture` | PASS, 18 passed |
| `cargo test --locked -p canic-core --lib ops::auth::delegated::verify -- --nocapture` | PASS, 17 passed |
| `cargo test --locked -p canic-core --lib workflow::rpc::capability -- --nocapture` | PASS, 15 passed |
| `cargo test --locked -p canic-core --lib workflow::rpc::request::handler -- --nocapture` | PASS, 32 passed |
| `cargo test --locked -p canic-tests --test root_suite unauthorized_caller_is_denied_for_each_root_capability_variant -- --test-threads=1 --nocapture` | PASS, unsandboxed due PocketIC local server bind requirement |
| `cargo test --locked -p canic-tests --test pic_role_attestation capability_endpoint_policy_and_structural_paths -- --test-threads=1 --nocapture` | PASS, 1 passed |

## Follow-Up

No follow-up actions are required from this audit. Continue to keep capability
DTOs passive, endpoint macros thin, and replay/authorization sequencing covered
when the root capability surface changes.
