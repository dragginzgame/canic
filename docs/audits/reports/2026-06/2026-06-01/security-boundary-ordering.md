# Security Boundary Ordering Audit - 2026-06-01

## Run Context

- Definition:
  `docs/audits/recurring/system/security-boundary-ordering.md`
- Previous report:
  `docs/audits/reports/2026-05/2026-05-16/security-boundary-ordering.md`
- Related same-day reports:
  - `docs/audits/reports/2026-06/2026-06-01/access-purity.md`
  - `docs/audits/reports/2026-06/2026-06-01/canonical-auth-boundary.md`
  - `docs/audits/reports/2026-06/2026-06-01/bootstrap-lifecycle-symmetry.md`
- Snapshot: `bf32cce1`
- Branch: `main`
- Worktree: dirty during audit; active `.57.10` audit-definition/report
  changes were present
- Method: `security-boundary-ordering/current`

## Executive Summary

Risk: **3 / 10**.

No ordering bypass was found.

Public delegated-token endpoints still follow:

```text
decode token boundary material -> verify token material/trust chain -> bind
subject to caller -> check scope -> consume update replay marker -> dispatch
```

Protected internal role endpoints now have an explicit root-signed internal
invocation proof boundary. Generated wrappers validate the internal call
envelope, verify the proof for accepted roles, and only then decode handler
arguments or dispatch the endpoint.

Root RPC capability replay keeps the intended two-mode ordering:

- default root handling authorizes before replay reservation;
- replay-first capability-envelope handling may reserve before authorization,
  but aborts the reservation on authorization or execution failure and commits
  only after successful execution.

No source remediation was needed. The recurring audit definition was refreshed
to track the current hard-cut singular delegated-token audience and protected
internal endpoint proof split.

## Verification Ordering Map

| Boundary | Ordering | Verdict |
| --- | --- | --- |
| Public delegated-token endpoint auth | `access/auth/token.rs` verifies through `AuthOps`, binds subject/caller, enforces required scope, then consumes update-token replay before returning success | Pass |
| Delegated-token material verifier | `ops/auth/token.rs` checks config, shard key binding, root trust anchor, local role, then delegates to the pure verifier before recording completed metrics | Pass |
| Delegated-token claims verifier | `ops/auth/delegated/verify.rs` verifies root signature before claims, audience, scope, and shard signature acceptance | Pass |
| Generated authenticated endpoint wrappers | `canic-macros/src/endpoint/expand.rs` evaluates access before dispatch and has a regression guard | Pass |
| Protected internal endpoint wrappers | `protected_internal_stage` validates envelope, verifies internal invocation proof, then decodes handler args | Pass |
| Root RPC default | authorization happens before replay preflight and execution | Pass |
| Root RPC replay-first | replay reservation happens first, then authorization; denial and execution failure abort the reservation; commit happens after success | Pass |
| Capability proof verifier | capability hash binding is verified before role-attestation or delegated-grant proof acceptance | Pass |
| Attestation caches | cached root response attestations and internal invocation proofs remain cache-only reuse paths with field, epoch, and expiry checks | Pass |

## Trust-Boundary Table

| Trust Boundary | Source Of Truth | Cache/Metric Status | Notes |
| --- | --- | --- | --- |
| Public delegated token | `AuthOps::verify_token(...)` and delegated verifier modules | No verifier cache | Endpoint success requires trust-chain, singular audience, scope, subject binding, and update replay consumption. |
| Protected internal role caller | Root-signed `CanicInternalCallEnvelopeV1` proof | Proof cache is caller-side only | Handler args are not decoded until proof verification succeeds. |
| Endpoint update replay marker | Auth stable replay marker | State mutation | Consumed only after endpoint authorization succeeds and before handler dispatch. |
| Root replay marker | Root replay store | State mutation | Fresh reservations are pending; response commits happen only after successful execution. |
| Capability envelope hash | Canonical request plus target canister and version | No authorization cache | Rebuilt per request, including when a role attestation was cached. |
| Role attestation cache | Root-issued signed attestation | Reuse-only cache | Cache reuse checks root, audience, subject, role, epoch floor, payload bindings, and expiry. |
| Metrics | Runtime metrics stores | Not a trust source | Metrics record bounded outcomes and are not consulted by authorization decisions. |

## Endpoint Delegated Token Analysis

`crates/canic-core/src/access/auth/token.rs` keeps the endpoint guard order
mechanical:

1. decode the first ingress argument as `DelegatedToken`;
2. call `AuthOps::verify_token(...)`;
3. enforce `verified.subject == caller`;
4. enforce required endpoint scope;
5. consume update-token replay marker for update calls;
6. return the issuer shard principal to the access layer.

The current singular audience hard cut also holds for this path. Scans found no
`DelegationAudience::Roles`, no `RolesOrPrincipals`, and no plural role/principal
audience compatibility shim in the delegated-token auth path.

Existing guard:

- `delegated_auth_guard_preserves_verify_bind_scope_consume_order`

## Endpoint Macro Sequencing Analysis

Generated public authenticated wrappers still evaluate access before dispatch.
The macro expansion test
`authenticated_endpoint_expansion_evaluates_access_before_dispatch` asserts the
source order from access evaluation to dispatch.

Generated protected internal wrappers now form a separate boundary for
`caller::has_role(...)` and `caller::has_any_role(...)`:

```text
decode envelope -> validate version/target/method -> verify internal
invocation proof -> decode handler args -> dispatch
```

Evidence:

- `crates/canic-macros/src/endpoint/expand.rs:739` owns
  `protected_internal_stage`;
- `expand.rs:797` calls `AuthApi::verify_internal_invocation_proof(...)`;
- handler argument decoding is emitted after that proof check;
- `protected_internal_role_endpoint_exports_envelope_wrapper` asserts
  `envelope_decode < verify < args_decode < dispatch`.

## Delegated Token Material Verification

`crates/canic-core/src/ops/auth/token.rs` remains the runtime material verifier.
The path checks delegated-token configuration and local shard/root binding
before pure token verification succeeds. Metrics are recorded on bounded
failure/completion paths, but they are not verifier inputs.

`crates/canic-core/src/ops/auth/delegated/verify.rs` verifies root signature
before claims acceptance. Claims verification then enforces token/certificate
windows, singular audience membership, cert/audience subset, and scope checks
before shard signature acceptance returns `VerifiedDelegatedToken`.

## Replay Sequencing Analysis

### Endpoint Update Tokens

Endpoint update-token replay consumption remains after token verification,
subject binding, and scope enforcement, and before handler dispatch. This
preserves the intended tradeoff: unauthorized calls do not consume replay
state, while authorized duplicate updates cannot reach user side effects.

### Root RPC Capabilities

`RootResponseWorkflow::response(...)` uses authorize-before-replay ordering.

`RootResponseWorkflow::response_replay_first(...)` is intentionally available
for capability-envelope execution. In replay-first mode:

1. `check_replay(...)` validates and reserves fresh replay state;
2. `authorize_with_hint(...)` runs policy;
3. policy denial calls `abort_replay(...)`;
4. execution failure calls `abort_replay(...)`;
5. successful execution calls `commit_replay(...)`.

Existing guards include:

- `preflight_authorize_then_replay_denies_before_replay_validation`;
- `preflight_replay_then_authorize_validates_replay_before_policy`;
- `preflight_replay_then_authorize_aborts_reserved_replay_on_policy_denial`;
- `check_replay_returns_cached_response_for_duplicate_same_payload`;
- `check_replay_rejects_conflicting_payload_for_same_request_id`.

The non-root cycles replay-first helper follows the same abort-on-denial and
abort-on-execution-failure pattern before committing cycles replay output.

## RPC Capability Handling Review

Root capability envelope handling validates the envelope, verifies the proof,
projects replay metadata, and only then calls the replay-first root response
workflow. Capability proof verification checks capability-hash binding before
role-attestation or delegated-grant proof acceptance.

The root response attestation cache remains a cache, not authority:

- cache lookup checks root, audience, subject, role, epoch floor, payload
  subject, payload role, payload audience, and expiry;
- invalid cache entries are cleared;
- capability hashes are still rebuilt for each request.

Existing guard:

- `cached_root_response_attestation_rejects_payload_subject_drift`

Internal invocation proof caching also rejects stale, future, invalid-window,
or epoch-below-floor entries through existing `api/ic/canic/tests.rs` guards.

## Residual Watchpoints

| Area | Risk | Note |
| --- | --- | --- |
| `crates/canic-core/src/access/auth/token.rs` | Medium | Keep verify/bind/scope/consume order source-obvious and tested. |
| `crates/canic-core/src/ops/auth/token.rs` | Medium | Metrics must remain outcome recording only. |
| `crates/canic-core/src/ops/auth/delegated/verify.rs` | Medium | Singular audience verification and local role hash binding are security-sensitive. |
| `crates/canic-macros/src/endpoint/expand.rs` | Medium | Wrapper generation owns both public access-before-dispatch and protected internal proof-before-args ordering. |
| `crates/canic-core/src/workflow/rpc/request/handler/mod.rs` | Medium | Replay-first mode must preserve abort-on-denial and commit-after-success. |
| `crates/canic-core/src/workflow/rpc/capability/verifier.rs` | Medium | Proof verifiers must keep capability-hash binding before proof acceptance. |
| `crates/canic-core/src/ops/rpc/mod.rs` | Medium | Attestation cache reuse must continue to re-check payload bindings and expiry. |

## Recommended Guard Additions

No immediate guard additions are required.

If these surfaces change again, useful extra guards would be:

- a source-order regression for `RootResponseWorkflow::response_with_pipeline`
  covering reserve, authorize, abort-on-denial, execute, abort-on-failure, and
  commit-after-success in one test;
- a protected internal `caller::has_any_role(...)` expansion test that asserts
  every accepted role is passed to proof verification before args decode.

## Commands Run

```text
rg -n 'consume|scope|subject|verify|bind|return Err|return Ok|DelegationAudience::Roles|RolesOrPrincipals|roles: Vec|principals: Vec' crates/canic-core/src/access/auth -g '*.rs'
rg -n 'eval_access|eval_default_app_guard|dispatch_|return Err' crates/canic-macros/src/endpoint -g '*.rs'
rg -n 'protected_internal|verify_internal_invocation_proof|decode_args|dispatch_update_async|caller::has_role|caller::has_any_role' crates/canic-macros/src/endpoint crates/canic/src/macros/endpoints -g '*.rs'
rg -n 'verify_delegated_token|verify_claims|verify_audience|verify_scopes|root_trust_anchor|verify_shard_key_binding|record_verify|DelegationAudience::Roles|RolesOrPrincipals|roles: Vec|principals: Vec' crates/canic-core/src/ops/auth -g '*.rs'
rg -n 'check_replay|reserve|authorize|execute|commit_replay|abort_replay|Cached|Duplicate' crates/canic-core/src/workflow/rpc crates/canic-core/src/ops/replay -g '*.rs'
rg -n 'capability_hash|RootCapabilityEnvelope|NonrootCyclesCapabilityEnvelope|attestation|cache|cached_root_response_attestation|RootResponseAttestation|payload_subject' crates/canic-core/src/ops/rpc crates/canic-core/src/workflow/rpc crates/canic-core/src/api/ic -g '*.rs'
rg -n 'delegated_auth_guard_preserves_verify_bind_scope_consume_order|authenticated_endpoint_expansion_evaluates_access_before_dispatch|protected_internal_role_endpoint_exports_envelope_wrapper|cached_root_response_attestation_rejects_payload_subject_drift|replay_first|abort.*replay|commit.*replay|decode.*trailing|payload_subject' crates/canic-core/src crates/canic-macros/src -g '*.rs'
```

## Final Verdict

Pass with watchpoints.

The enforcement ordering invariants hold. The residual risk is future drift in
the security hot paths, not a current bypass.
