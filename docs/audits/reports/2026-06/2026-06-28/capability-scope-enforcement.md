# Capability Scope Enforcement Invariant Audit - 2026-06-28

## Report Preamble

- Scope: endpoint delegated-token verify/bind/scope ordering, delegated-token
  local-role scope checks, root capability envelope/proof validation,
  structural capability proof routing, root capability replay and
  authorization ordering, and endpoint rejection for unauthorized root
  capability callers.
- Definition path:
  `docs/audits/recurring/invariants/capability-scope-enforcement.md`
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-19/capability-scope-enforcement.md`
- Code snapshot identifier: `4a158a32`
- Branch: `main`
- Method tag/version: `capability-scope-enforcement-current`
- Comparability status: comparable. No audit definition change was required.
- Auditor: Codex
- Run timestamp: `2026-06-28T11:39:54Z`
- Worktree state: dirty before audit with prior 2026-06-28 audit report docs;
  this run also narrowed internal RPC capability workflow handler visibility.

## Audit Selection

This audit was selected as the next stale recurring audit after the
`2026-06-28` canonical-auth-boundary report. The retained recurring report
inventory showed `capability-scope-enforcement` in the remaining
`2026-06-19` tie set.

## Executive Summary

Status: PASS.

Risk score: 3 / 10.

No authorization-before-authentication or scope-as-identity violation was
found. Endpoint delegated-token auth still verifies token material, binds the
verified subject to the caller, and then enforces the required endpoint scope.
Delegated-token verification still derives accepted scopes from verified
claims and accepted local-role grants, and root capability calls still validate
their envelope/proof before replay, authorization, execution, and replay
commit. A follow-up visibility cleanup reduced the counted public
capability-facing surface from 20 to 18 items by narrowing workflow handler
entry points to RPC-internal visibility.

## Findings

| ID | Status | Severity | Finding | Evidence |
| --- | --- | --- | --- | --- |
| CSE-2026-06-28-1 | PASS | High | Endpoint auth preserves `verify token -> bind subject -> enforce scope -> handler` ordering. | `crates/canic-core/src/access/auth/token.rs`; source-order guard covered by `cargo test --locked -p canic-core --lib access::auth -- --nocapture`. |
| CSE-2026-06-28-2 | PASS | High | Delegated-token scope checks are derived from verified claims and local-role grant acceptance, not request-only payload fields. | `crates/canic-core/src/ops/auth/delegated/verify.rs`; `verify_audience_and_grants(...)` feeds `verify_scopes(...)`; delegated verifier tests passed. |
| CSE-2026-06-28-3 | PASS | Medium | Root capability proof validation remains structural-only and does not create identity. | `workflow/rpc/capability/envelope.rs`, `verifier.rs`, and `proof.rs`; capability tests passed. |
| CSE-2026-06-28-4 | PASS | Medium | Root capability replay reservation, authorization denial, execution, and replay commit ordering remain explicit. | `workflow/rpc/request/handler/mod.rs`; handler tests passed, including replay-then-authorize abort coverage. |
| CSE-2026-06-28-5 | PASS | Medium | Unauthorized root capability endpoint callers are rejected at the endpoint boundary. | `cargo test --locked -p canic-tests --test root_suite unauthorized_caller_is_denied_for_each_root_capability_variant -- --test-threads=1 --nocapture` passed outside the sandbox. |
| CSE-2026-06-28-6 | PASS | Medium | Capability endpoint policy and supported structural paths remain valid under PocketIC. | `cargo test --locked -p canic-tests --test pic_role_attestation capability_endpoint_policy_and_structural_paths -- --test-threads=1 --nocapture` passed outside the sandbox. |
| CSE-2026-06-28-7 | PASS | Low | RPC capability workflow handler entry points no longer use broader public visibility than needed. | `RootResponseWorkflow`, `response_replay_first(...)`, and `NonrootCyclesCapabilityWorkflow` were narrowed to `pub(in crate::workflow::rpc)`; clippy and focused RPC tests passed. |

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
- Delegated-token local verification: canonical token/cert/claims checks,
  audience/grant checks, local-role grant acceptance, scope checks, freshness,
  and subject binding.
- Root capability path: endpoint guard and envelope validation, structural
  proof validation, request preflight, replay reservation, authorization,
  execution, and replay commit.

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

## Hub Module Pressure

| Surface | Evidence | Unique Subsystems | Cross-Layer Count | Pressure Score |
| --- | --- | ---: | ---: | ---: |
| Capability DTO/proof/envelope surface | capability DTO/envelope scan found 19 Rust files | 6 | 4 | 6 |
| `workflow/rpc/capability/*` | capability validation/proof scan found 8 Rust files | 3 | 2 | 5 |
| `workflow/rpc/request/handler/*` | request handler/replay/authorization scan found 31 Rust files | 4 | 3 | 6 |
| root capability metrics | metrics scan found 23 Rust files | 3 | 2 | 5 |
| `access/auth/token.rs` | small endpoint auth ordering choke point | 2 | 2 | 5 |

Pressure remains moderate. No hub exceeded the `>= 7` high-pressure threshold
in this run.

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| Capability DTO fan-in | `dto/capability/mod.rs` | Capability envelope/proof names appeared in 19 Rust files across API, ops, workflow, tests, and test canisters. | Medium |
| Replay/authorization edit center | `workflow/rpc/request/handler/*` | Combines replay reservation, authorization, execution, commit, and recovery handling. | Medium |
| Endpoint auth source-order guard | `access/auth/token.rs` | Correctness depends on stable verify/bind/scope ordering; source-order unit guard passed. | Low |
| Capability public surface edge | `api/rpc`, `workflow/rpc/capability`, `workflow/rpc/request/handler`, `dto/capability` | Public capability-facing scan now finds 18 items after narrowing workflow handler visibility. | Low |

### Enum Shock Radius

| Enum | Defined In | Reference Files | Risk |
| --- | --- | ---: | --- |
| `CapabilityProof` | `crates/canic-core/src/dto/capability/mod.rs` | 13 | Medium |
| `CapabilityService` | `crates/canic-core/src/dto/capability/mod.rs` | 11 | Medium |
| `RootCapability` | `crates/canic-core/src/workflow/rpc/request/handler/capability.rs` | 31 handler-surface files in aggregate scan | Medium |

### Cross-Layer Struct Spread

| Struct | Defined In | Reference Evidence | Risk |
| --- | --- | --- | --- |
| `RootCapabilityEnvelopeV1` | `crates/canic-core/src/dto/capability/mod.rs` | 12 files across API, ops, workflow, tests, and stubs | Medium |
| `CapabilityRequestMetadata` | `crates/canic-core/src/dto/capability/mod.rs` | capability DTO/envelope aggregate found 19 files | Medium |
| `RootCapabilityResponseV1` | `crates/canic-core/src/dto/capability/mod.rs` | API, workflow, tests, and stubs | Medium |

### Growing Hub Modules

| Module | Subsystems Imported | Recent Commits | Risk |
| --- | --- | ---: | --- |
| `crates/canic-core/src/workflow/rpc/request/handler/` | replay, authorization, execution, metrics, recovery | recent 0.72 and 0.68 capability work | Medium |
| `crates/canic-core/src/workflow/rpc/capability/` | envelope validation, proof routing, hash/replay metadata | recent 0.183 and 0.68 capability work | Medium |
| `crates/canic-core/src/access/auth/token.rs` | endpoint delegated-token ordering | recent delegated-auth verifier work | Medium |

### Capability Surface Growth

| Module Set | Public Items | Risk |
| --- | ---: | --- |
| `dto/capability`, `api/rpc`, `workflow/rpc/capability`, `workflow/rpc/request/handler` | 18 | Low |

The public capability-facing surface is below the `20` item growing-surface
threshold after narrowing RPC-internal workflow handler visibility. The
remaining counted items are DTO/protocol shapes, the public `RpcApi` facade,
and crate-internal workflow capability dispatchers that clippy requires to
remain plain `pub` inside the private workflow module.

## Dependency Fan-In Pressure

| Surface | Evidence | Risk |
| --- | --- | --- |
| Top-level `dto` imports | `use crate::` aggregate found 25 top-level `dto` import lines | Medium |
| Top-level `cdk` imports | `use crate::` aggregate found 23 top-level `cdk` import lines | Medium |
| Capability DTO/proof/envelope surface | Direct scan found 19 Rust files | Medium |
| Request handler/replay/authorization surface | Direct scan found 31 Rust files | Medium |
| Root capability metrics | Direct scan found 23 Rust files | Medium |
| Workflow capability validation surface | Direct scan found 8 Rust files | Low |

No fan-in evidence indicated capability or scope checks creating identity or
authorizing without the existing caller/root context.

## Risk Score

Risk: 3 / 10.

- `+0`: no capability/scope enforcement violation found.
- `+1`: endpoint auth and scope ordering remain concentrated in one
  high-impact access module.
- `+1`: capability DTO/proof/envelope surface has medium cross-subsystem
  fan-in.
- `+1`: root capability replay and authorization sequencing remains a
  sensitive workflow center.
- `+0`: public capability-facing surface is now 18 items, below the audit
  definition's `> 20` scoring threshold.

Verdict: invariant holds with low residual risk.

## Verification Readout

| Check | Result |
| --- | --- |
| `rg -n 'require_scope\|capability\|authorize\|permission\|allowed_scopes' crates/canic-core/src crates/canic/src crates/canic-macros/src canisters -g '*.rs'` | PASS |
| `rg -n 'verify_token\(\|enforce_subject_binding\|enforce_required_scope\|verify_scopes\|required_scope\|ScopeRejected\|TokenGrantRejected' crates/canic-core/src/access/auth crates/canic-core/src/ops/auth/delegated -g '*.rs'` | PASS |
| `rg -n 'validate_root_capability_envelope\|verify_root_capability_proof\|verify_root_structural_proof\|verify_nonroot_structural_cycles_proof\|authorize\(\|preflight\|reserve\|commit' crates/canic-core/src/workflow/rpc -g '*.rs'` | PASS |
| `rg -n 'CapabilityProof\|CapabilityService\|RootCapability\|CapabilityEnvelope\|RootCapabilityEnvelope\|Capability' crates/canic-core/src crates/canic/src canisters crates/canic-tests/tests -g '*.rs'` | PASS |
| `cargo test --locked -p canic-core --lib access::auth -- --nocapture` | PASS, 18 passed |
| `cargo test --locked -p canic-core --lib ops::auth::delegated::verify -- --nocapture` | PASS, 17 passed |
| `cargo test --locked -p canic-core --lib workflow::rpc::capability -- --nocapture` | PASS, 15 passed |
| `cargo test --locked -p canic-core --lib workflow::rpc::request::handler -- --nocapture` | PASS, 33 passed |
| `POCKET_IC_BIN=/home/adam/projects/canic/.tmp/test-runtime/pocket-ic-server-14.0.0/pocket-ic cargo test --locked -p canic-tests --test root_suite unauthorized_caller_is_denied_for_each_root_capability_variant -- --test-threads=1 --nocapture` | PASS, 1 passed outside sandbox |
| `POCKET_IC_BIN=/home/adam/projects/canic/.tmp/test-runtime/pocket-ic-server-14.0.0/pocket-ic cargo test --locked -p canic-tests --test pic_role_attestation capability_endpoint_policy_and_structural_paths -- --test-threads=1 --nocapture` | PASS, 1 passed outside sandbox |
| `rg -n 'pub enum\|pub struct\|pub fn\|pub async fn' crates/canic-core/src/dto/capability/mod.rs crates/canic-core/src/api/rpc/mod.rs crates/canic-core/src/workflow/rpc/capability/mod.rs crates/canic-core/src/workflow/rpc/request/handler/mod.rs` | PASS, 18 counted items after visibility cleanup |
| `cargo clippy --locked -p canic-core --lib -- -D warnings` | PASS |

## Follow-Up

No follow-up actions required. Keep capability DTOs passive, endpoint macros
thin, and replay/authorization sequencing covered when the root capability
surface changes.
